//! Admin action log — every create / update / delete driven through
//! the admin writes a row to `rustio_admin_actions`. The audit trail
//! powers two user-visible surfaces:
//!
//! - `GET /admin/history` — project-wide timeline.
//! - `GET /admin/<model>/<id>/history` — per-object history.
//!
//! ## Integrity
//!
//! [`record`] rejects entries that are missing any of `user_id`,
//! `model_name`, or `object_id`. The caller gets an
//! [`Error::Internal`] so the admin handler can fail loudly rather
//! than silently losing the audit trail.

use chrono::{DateTime, Utc};
use sqlx::Row as _;

use crate::error::{Error, Result};
use crate::orm::Db;

pub(crate) const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS rustio_admin_actions (
    id          BIGSERIAL   PRIMARY KEY,
    user_id     BIGINT      NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
    action_type TEXT        NOT NULL,
    model_name  TEXT        NOT NULL,
    object_id   BIGINT      NOT NULL,
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address  TEXT,
    summary     TEXT        NOT NULL DEFAULT ''
)";

pub(crate) const CREATE_MODEL_INDEX_SQL: &str =
    "CREATE INDEX IF NOT EXISTS rustio_admin_actions_model_idx \
     ON rustio_admin_actions(model_name, object_id)";

pub(crate) const CREATE_TIMESTAMP_INDEX_SQL: &str =
    "CREATE INDEX IF NOT EXISTS rustio_admin_actions_timestamp_idx \
     ON rustio_admin_actions(timestamp DESC)";

/// Ensure the `rustio_admin_actions` table and its indexes exist.
/// Idempotent. Depends on `rustio_users` existing first.
///
/// 0.4.0 lifecycle additions: `metadata` JSONB, `correlation_id`, and
/// `session_id`. The framework will populate these as recovery flows
/// land in R1+; existing audit rows from 0.3.x stay valid with NULLs.
pub async fn ensure_table(db: &Db) -> Result<()> {
    sqlx::query(CREATE_TABLE_SQL).execute(db.pool()).await?;
    sqlx::query(CREATE_MODEL_INDEX_SQL)
        .execute(db.pool())
        .await?;
    sqlx::query(CREATE_TIMESTAMP_INDEX_SQL)
        .execute(db.pool())
        .await?;

    // R0 (0.4.0) lifecycle additions — additive, idempotent.
    sqlx::query("ALTER TABLE rustio_admin_actions ADD COLUMN IF NOT EXISTS metadata JSONB")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_admin_actions ADD COLUMN IF NOT EXISTS correlation_id TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_admin_actions ADD COLUMN IF NOT EXISTS session_id BIGINT")
        .execute(db.pool())
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_admin_actions_correlation_idx \
         ON rustio_admin_actions (correlation_id) WHERE correlation_id IS NOT NULL",
    )
    .execute(db.pool())
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_admin_actions_session_idx \
         ON rustio_admin_actions (session_id) WHERE session_id IS NOT NULL",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Create,
    Update,
    Delete,
}

impl ActionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "create" => Some(Self::Create),
            "update" => Some(Self::Update),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Create => "Created",
            Self::Update => "Updated",
            Self::Delete => "Deleted",
        }
    }

    pub fn pill_class(self) -> &'static str {
        match self {
            Self::Create => "badge-success",
            Self::Update => "badge-neutral",
            Self::Delete => "badge-danger",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdminAction {
    pub id: i64,
    pub user_id: i64,
    pub user_email: Option<String>,
    pub action_type: String,
    pub model_name: String,
    pub object_id: i64,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub summary: String,
}

pub struct LogEntry<'a> {
    pub user_id: i64,
    pub action_type: ActionType,
    pub model_name: &'a str,
    pub object_id: i64,
    pub ip_address: Option<&'a str>,
    pub summary: String,
}

/// Write one row to the action log. Validates required fields before
/// touching the DB so a broken audit pipeline becomes visible.
pub async fn record(db: &Db, entry: LogEntry<'_>) -> Result<()> {
    if entry.user_id <= 0 {
        return Err(Error::Internal("admin audit: missing user_id".to_string()));
    }
    if entry.model_name.trim().is_empty() {
        return Err(Error::Internal(
            "admin audit: missing model_name".to_string(),
        ));
    }
    if entry.object_id <= 0 {
        return Err(Error::Internal(
            "admin audit: missing object_id".to_string(),
        ));
    }

    let now = Utc::now();
    sqlx::query(
        "INSERT INTO rustio_admin_actions
             (user_id, action_type, model_name, object_id, timestamp, ip_address, summary)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(entry.user_id)
    .bind(entry.action_type.as_str())
    .bind(entry.model_name)
    .bind(entry.object_id)
    .bind(now)
    .bind(entry.ip_address)
    .bind(&entry.summary)
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Fetch the most recent `limit` admin actions, newest first.
pub async fn recent(
    db: &Db,
    limit: i64,
    model_filter: Option<&str>,
    action_filter: Option<&str>,
) -> Result<Vec<AdminAction>> {
    let mut sql = String::from(
        "SELECT a.id, a.user_id, u.email AS user_email, a.action_type,
                a.model_name, a.object_id, a.timestamp, a.ip_address, a.summary
         FROM rustio_admin_actions a
         LEFT JOIN rustio_users u ON u.id = a.user_id",
    );
    let mut clauses: Vec<String> = Vec::new();
    let mut param_idx: usize = 1;
    if model_filter.is_some() {
        clauses.push(format!("a.model_name = ${param_idx}"));
        param_idx += 1;
    }
    if action_filter.is_some() {
        clauses.push(format!("a.action_type = ${param_idx}"));
        param_idx += 1;
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(&format!(
        " ORDER BY a.timestamp DESC, a.id DESC LIMIT ${param_idx}"
    ));

    let mut q = sqlx::query(&sql);
    if let Some(m) = model_filter {
        q = q.bind(m);
    }
    if let Some(a) = action_filter {
        q = q.bind(a);
    }
    q = q.bind(limit);

    let rows = q.fetch_all(db.pool()).await?;
    rows.iter().map(row_to_action).collect()
}

/// All actions for one `(model, object_id)`, newest first.
pub async fn for_object(db: &Db, model_name: &str, object_id: i64) -> Result<Vec<AdminAction>> {
    let rows = sqlx::query(
        "SELECT a.id, a.user_id, u.email AS user_email, a.action_type,
                a.model_name, a.object_id, a.timestamp, a.ip_address, a.summary
         FROM rustio_admin_actions a
         LEFT JOIN rustio_users u ON u.id = a.user_id
         WHERE a.model_name = $1 AND a.object_id = $2
         ORDER BY a.timestamp DESC, a.id DESC",
    )
    .bind(model_name)
    .bind(object_id)
    .fetch_all(db.pool())
    .await?;
    rows.iter().map(row_to_action).collect()
}

fn row_to_action(r: &sqlx::postgres::PgRow) -> Result<AdminAction> {
    Ok(AdminAction {
        id: r.try_get("id")?,
        user_id: r.try_get("user_id")?,
        user_email: r.try_get("user_email")?,
        action_type: r.try_get("action_type")?,
        model_name: r.try_get("model_name")?,
        object_id: r.try_get("object_id")?,
        timestamp: r.try_get("timestamp")?,
        ip_address: r.try_get("ip_address")?,
        summary: r.try_get("summary")?,
    })
}
