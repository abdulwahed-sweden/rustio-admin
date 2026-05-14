//! `ConcreteOps<M>` — the manual runtime that drives every project
//! model registered via `Admin::model::<M>()`.
//!
//! Every framework-level read or write goes through one of the
//! [`AdminOps`](super::types::AdminOps) methods, which in turn calls
//! the matching free function in `crate::orm`. The trait stays
//! `pub(crate)` because handlers route through `AdminEntry::ops`
//! directly; consumers never name `ConcreteOps`.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::error::Result;
use crate::http::FormData;
use crate::orm::{Db, Row};

use super::bulk::{BulkActionContext, BulkActionResult};
use super::modeladmin::ModelAdmin;
use super::types::{
    AdminModel, AdminOps, CreateResult, EditRow, ListOpts, ListPage, ListRow, UpdateResult,
};

pub(crate) struct ConcreteOps<M> {
    _marker: std::marker::PhantomData<M>,
}

impl<M> ConcreteOps<M> {
    pub(crate) fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M> AdminOps for ConcreteOps<M>
where
    M: AdminModel + ModelAdmin + crate::orm::Model,
{
    fn list<'a>(
        &'a self,
        db: &'a Db,
        opts: ListOpts,
    ) -> Pin<Box<dyn Future<Output = Result<ListPage>> + Send + 'a>> {
        Box::pin(async move {
            // Defense-in-depth: every column name interpolated into
            // SQL must come from `M::COLUMNS`. The handler validates
            // sort/filter/search names against the model's
            // AdminField list, but reasserting here keeps the runtime
            // safe even if a future caller forgets.
            let valid: HashSet<&str> = M::COLUMNS.iter().copied().collect();

            // ---- WHERE: filters + search ----
            let mut where_clauses: Vec<String> = Vec::new();
            let mut where_bindings: Vec<String> = Vec::new();
            let mut placeholder: usize = 1;

            for (col, val) in &opts.filters {
                if !valid.contains(col.as_str()) {
                    continue;
                }
                // `::text` cast keeps the comparison string-shaped
                // — bool / int / timestamp columns then match the
                // form values produced by `display_values()`.
                where_clauses.push(format!("{col}::text = ${placeholder}"));
                where_bindings.push(val.clone());
                placeholder += 1;
            }

            if let Some((term, cols)) = &opts.search {
                let term = term.trim();
                if !term.is_empty() {
                    let valid_cols: Vec<&String> =
                        cols.iter().filter(|c| valid.contains(c.as_str())).collect();
                    if !valid_cols.is_empty() {
                        let p = placeholder;
                        let or_clauses: Vec<String> = valid_cols
                            .iter()
                            .map(|c| format!("{c}::text ILIKE ${p}"))
                            .collect();
                        where_clauses.push(format!("({})", or_clauses.join(" OR ")));
                        where_bindings.push(format!("%{term}%"));
                        placeholder += 1;
                    }
                }
            }

            let where_sql = if where_clauses.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", where_clauses.join(" AND "))
            };

            // ---- ORDER BY ----
            let mut order_parts: Vec<String> = Vec::with_capacity(opts.ordering.len());
            for (col, dir) in &opts.ordering {
                if valid.contains(col.as_str()) {
                    order_parts.push(format!("{} {}", col, dir.sql()));
                }
            }
            let order_clause = if order_parts.is_empty() {
                "id DESC".to_string()
            } else {
                order_parts.join(", ")
            };

            // ---- COUNT(*) over the same WHERE ----
            // Done first so a paginated render that lands beyond the
            // last page doesn't waste a SELECT on a row span that
            // doesn't exist (the handler can clamp `page` against
            // `total` if it wants to). For Tier 1 we just always run
            // both — the COUNT is fast even on millions of rows when
            // the WHERE is selective.
            let count_sql = format!("SELECT COUNT(*) FROM {}{}", M::TABLE, where_sql);
            let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
            for b in &where_bindings {
                count_q = count_q.bind(b);
            }
            let total: i64 = count_q.fetch_one(db.pool()).await?;

            // ---- SELECT page ----
            let mut sql = format!(
                "SELECT {} FROM {}{} ORDER BY {}",
                M::COLUMNS.join(", "),
                M::TABLE,
                where_sql,
                order_clause,
            );
            let limit_idx = opts.limit.map(|_| {
                let i = placeholder;
                placeholder += 1;
                i
            });
            let offset_idx = opts.offset.map(|_| {
                let i = placeholder;
                placeholder += 1;
                i
            });
            if let Some(i) = limit_idx {
                sql.push_str(&format!(" LIMIT ${i}"));
            }
            if let Some(i) = offset_idx {
                sql.push_str(&format!(" OFFSET ${i}"));
            }

            let mut q = sqlx::query(&sql);
            for b in &where_bindings {
                q = q.bind(b);
            }
            if let Some(limit) = opts.limit {
                q = q.bind(limit);
            }
            if let Some(offset) = opts.offset {
                q = q.bind(offset);
            }
            let rows = q.fetch_all(db.pool()).await?;
            let models: Result<Vec<M>> =
                rows.iter().map(|r| M::from_row(Row::from_pg(r))).collect();
            let models = models?;
            let list_rows = models
                .into_iter()
                .map(|r| {
                    let id = AdminModel::id(&r);
                    let cells: Vec<String> =
                        r.display_values().into_iter().map(|(_, v)| v).collect();
                    // FK hydration runs in the handler layer (where the
                    // `RelationRegistry` is reachable). The ops layer
                    // emits a parallel `cell_links` of all-None so the
                    // invariant `cells.len() == cell_links.len()` holds
                    // even for callers that bypass hydration.
                    let cell_links = vec![None; cells.len()];
                    ListRow {
                        id,
                        cells,
                        cell_links,
                    }
                })
                .collect();
            Ok(ListPage {
                rows: list_rows,
                total,
            })
        })
    }

    fn find_row<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EditRow>>> + Send + 'a>> {
        Box::pin(async move {
            let found = crate::orm::find::<M>(db, id).await?;
            Ok(found.map(|m| EditRow {
                id: AdminModel::id(&m),
                values: m.display_values(),
            }))
        })
    }

    fn create<'a>(&'a self, db: &'a Db, form: &'a FormData) -> CreateResult<'a> {
        Box::pin(async move {
            match M::from_form(form) {
                Ok(model) => match crate::orm::create(db, &model).await {
                    Ok(id) => Ok(Ok(id)),
                    // Postgres constraint violations route to
                    // `Error::Conflict` via `From<sqlx::Error>`. Catch
                    // them here so the user sees a re-rendered form
                    // with an inline error instead of a 500.
                    Err(crate::error::Error::Conflict(msg)) => {
                        log::warn!("create rejected by DB constraint: {msg}");
                        Ok(Err(vec!["Invalid value or constraint violation. \
                             Please check the highlighted fields and try again."
                            .into()]))
                    }
                    Err(other) => Err(other),
                },
                Err(errs) => Ok(Err(errs)),
            }
        })
    }

    fn update<'a>(&'a self, db: &'a Db, id: i64, form: &'a FormData) -> UpdateResult<'a> {
        Box::pin(async move {
            match M::from_form(form) {
                Ok(model) => match crate::orm::update(db, id, &model).await {
                    Ok(()) => Ok(Ok(())),
                    Err(crate::error::Error::Conflict(msg)) => {
                        log::warn!("update rejected by DB constraint: {msg}");
                        Ok(Err(vec!["Invalid value or constraint violation. \
                             Please check the highlighted fields and try again."
                            .into()]))
                    }
                    Err(other) => Err(other),
                },
                Err(errs) => Ok(Err(errs)),
            }
        })
    }

    fn delete<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { crate::orm::delete::<M>(db, id).await })
    }

    fn object_label<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'a>> {
        Box::pin(async move {
            let found = crate::orm::find::<M>(db, id).await?;
            Ok(found.map(|m| m.object_label()))
        })
    }

    fn execute_bulk_action<'a>(
        &'a self,
        db: &'a Db,
        name: &'a str,
        ids: &'a [i64],
        ctx: &'a BulkActionContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<BulkActionResult>> + Send + 'a>> {
        // Forward to the project's `ModelAdmin::execute_bulk_action`.
        // The model's override decides per-action semantics; the
        // framework's default impl (in modeladmin.rs) returns a
        // structured BadRequest when the action name has no project
        // handler.
        M::execute_bulk_action(name, ids, db, ctx)
    }
}
