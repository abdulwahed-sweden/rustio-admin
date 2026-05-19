//! `/admin/db` — read-only Postgres schema explorer for the
//! Developer role.
//!
//! Three `information_schema` / `pg_catalog` queries pull every
//! table in the `public` schema, its columns, and its foreign-key
//! relationships, then the renderer composes a per-table card
//! summary. The page is diagnostic — useful during development to
//! eyeball the schema without leaving the admin chrome and without
//! reaching for `psql`. No mutations; no DDL; no row sampling
//! (operators query directly when they need data).
//!
//! Row counts are the planner's `pg_class.reltuples` estimate
//! (cheap; single batched query) rather than `COUNT(*)` per
//! table (slow on big tables, and the dashboard isn't the place
//! for ground truth). Surfaced as "~ N" so operators read it as
//! approximate.
//!
//! Permission: Developer role only. The route is registered behind
//! `role_guard(Role::Developer)` in `routes.rs`; this module
//! assumes the identity has already cleared that gate.

use serde::Serialize;
use sqlx::Row as SqlxRow;

use crate::auth::Identity;
use crate::error::{Error, Result};
use crate::http::{Request, Response};

use super::handlers::{csrf_token, AdminCtx};
use super::render::{BaseContext, SidebarEntry};

#[derive(Serialize)]
pub(crate) struct DbBrowserCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    /// Sorted alphabetically by table name. The `public` schema is
    /// the only one consulted — other schemas (the framework
    /// doesn't create any) are intentionally excluded so the page
    /// stays focused on the operator's working set.
    pub tables: Vec<TableSummary>,
    /// Sum of `tables[*].column_count` — surfaced at the top of
    /// the page so operators see schema-wide size at a glance.
    pub total_columns: i64,
    /// Sum of `tables[*].row_estimate`. Approximate by definition.
    pub total_row_estimate: i64,
}

#[derive(Serialize)]
pub(crate) struct TableSummary {
    pub name: String,
    /// `pg_class.reltuples` cast to bigint. Approximate — refreshed
    /// by `ANALYZE` / autovacuum, not by every INSERT.
    pub row_estimate: i64,
    pub columns: Vec<ColumnInfo>,
    /// Foreign keys *owned* by this table (`<col> → other.<col>`).
    pub outgoing_fks: Vec<ForeignKey>,
    /// Foreign keys *pointing at* this table from elsewhere.
    /// Useful for "who depends on this row."
    pub incoming_fks: Vec<ForeignKey>,
    /// Total column count, surfaced in the per-card header so the
    /// operator doesn't have to count rows in the columns table.
    pub column_count: i64,
}

#[derive(Serialize, Clone)]
pub(crate) struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    /// Default expression as Postgres reports it — `nextval(...)`,
    /// `'static'`, etc. Empty string when no default.
    pub default_expr: String,
}

#[derive(Serialize, Clone)]
pub(crate) struct ForeignKey {
    pub source_table: String,
    pub source_column: String,
    pub target_table: String,
    pub target_column: String,
}

pub(crate) async fn show_db_browser(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    // Fetch the three result sets in sequence — the table count is
    // small (handfuls in a typical project), so a concurrent fetch
    // would just add complexity without measurable wins.
    let tables_rows = sqlx::query(
        "SELECT
             c.relname AS name,
             c.reltuples::bigint AS row_estimate
         FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = 'public'
           AND c.relkind = 'r'
         ORDER BY c.relname",
    )
    .fetch_all(ctx.db.pool())
    .await
    .map_err(|e| Error::Internal(format!("db_browser tables: {e}")))?;

    let columns_rows = sqlx::query(
        "SELECT
             table_name,
             column_name,
             data_type,
             is_nullable,
             COALESCE(column_default, '') AS column_default,
             ordinal_position
         FROM information_schema.columns
         WHERE table_schema = 'public'
         ORDER BY table_name, ordinal_position",
    )
    .fetch_all(ctx.db.pool())
    .await
    .map_err(|e| Error::Internal(format!("db_browser columns: {e}")))?;

    let fks_rows = sqlx::query(
        "SELECT
             tc.table_name AS source_table,
             kcu.column_name AS source_col,
             ccu.table_name AS target_table,
             ccu.column_name AS target_col
         FROM information_schema.table_constraints tc
         JOIN information_schema.key_column_usage kcu
             ON tc.constraint_name = kcu.constraint_name
             AND tc.table_schema = kcu.table_schema
         JOIN information_schema.constraint_column_usage ccu
             ON ccu.constraint_name = tc.constraint_name
             AND ccu.table_schema = tc.table_schema
         WHERE tc.constraint_type = 'FOREIGN KEY'
           AND tc.table_schema = 'public'
         ORDER BY tc.table_name, kcu.column_name",
    )
    .fetch_all(ctx.db.pool())
    .await
    .map_err(|e| Error::Internal(format!("db_browser fks: {e}")))?;

    // Assemble.
    let mut tables: Vec<TableSummary> = tables_rows
        .into_iter()
        .map(|r| {
            let name: String = r.try_get("name").unwrap_or_default();
            let row_estimate: i64 = r.try_get("row_estimate").unwrap_or(0).max(0);
            TableSummary {
                name,
                row_estimate,
                columns: Vec::new(),
                outgoing_fks: Vec::new(),
                incoming_fks: Vec::new(),
                column_count: 0,
            }
        })
        .collect();

    // Index by table name for O(1) attachment of columns + FKs.
    // The vec ordering (alphabetic) is preserved on the final
    // serialised side; we just need to write into the right slot.
    let mut by_name: std::collections::HashMap<String, usize> =
        tables.iter().enumerate().map(|(i, t)| (t.name.clone(), i)).collect();

    for r in columns_rows {
        let table: String = r.try_get("table_name").unwrap_or_default();
        let Some(idx) = by_name.get(&table).copied() else {
            continue;
        };
        let is_nullable_str: String = r.try_get("is_nullable").unwrap_or_else(|_| "NO".to_string());
        tables[idx].columns.push(ColumnInfo {
            name: r.try_get("column_name").unwrap_or_default(),
            data_type: r.try_get("data_type").unwrap_or_default(),
            is_nullable: is_nullable_str.eq_ignore_ascii_case("YES"),
            default_expr: r.try_get("column_default").unwrap_or_default(),
        });
    }

    for r in fks_rows {
        let fk = ForeignKey {
            source_table: r.try_get("source_table").unwrap_or_default(),
            source_column: r.try_get("source_col").unwrap_or_default(),
            target_table: r.try_get("target_table").unwrap_or_default(),
            target_column: r.try_get("target_col").unwrap_or_default(),
        };
        if let Some(idx) = by_name.get(&fk.source_table).copied() {
            tables[idx].outgoing_fks.push(fk.clone());
        }
        if let Some(idx) = by_name.get(&fk.target_table).copied() {
            tables[idx].incoming_fks.push(fk);
        }
    }

    let mut total_columns: i64 = 0;
    let mut total_row_estimate: i64 = 0;
    for t in tables.iter_mut() {
        t.column_count = t.columns.len() as i64;
        total_columns += t.column_count;
        total_row_estimate += t.row_estimate;
    }
    // Discourage the unused-map warning while keeping the API tight.
    by_name.clear();

    let view = DbBrowserCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Database",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        tables,
        total_columns,
        total_row_estimate,
    };
    let body = ctx.templates.render("admin/db_browser.html", &view)?;
    Ok(Response::html(body))
}
