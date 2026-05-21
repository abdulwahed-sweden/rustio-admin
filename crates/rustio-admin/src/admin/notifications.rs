//! Per-operator notifications — a small `rustio_notifications`
//! table + a public [`send`] helper for project code.
//!
//! Schema:
//!
//! ```sql
//! CREATE TABLE rustio_notifications (
//!     id         BIGSERIAL    PRIMARY KEY,
//!     user_id    BIGINT       NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
//!     message    TEXT         NOT NULL,
//!     url        TEXT         NOT NULL DEFAULT '',
//!     read_at    TIMESTAMPTZ,
//!     created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
//! );
//! ```
//!
//! Projects emit notifications via
//! [`rustio_admin::send_notification`] — anywhere a `Db` is in
//! scope. Every authenticated admin page renders a bell in the
//! topbar with an unread-count badge; the operator clicks
//! through to `/admin/notifications`, scans the list, and hits
//! "Mark all read" to clear the badge.
//!
//! v1 scope: per-row dismissal is not yet shipped — the page
//! offers a single "mark every notification for this operator
//! as read" action. Operators with high notification volume can
//! filter by date in a future iteration.

use chrono::{DateTime, Utc};
use sqlx::Row as _;

use crate::error::Result;
use crate::orm::Db;

pub(crate) const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS rustio_notifications (
    id         BIGSERIAL    PRIMARY KEY,
    user_id    BIGINT       NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
    message    TEXT         NOT NULL,
    url        TEXT         NOT NULL DEFAULT '',
    read_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
)";

pub(crate) const CREATE_INDEX_SQL: &str =
    "CREATE INDEX IF NOT EXISTS rustio_notifications_user_unread_idx \
     ON rustio_notifications (user_id, read_at) WHERE read_at IS NULL";

// public:
/// Ensure the `rustio_notifications` table + its unread-lookup
/// index exist. Idempotent.
pub async fn ensure_table(db: &Db) -> Result<()> {
    sqlx::query(CREATE_TABLE_SQL).execute(db.pool()).await?;
    sqlx::query(CREATE_INDEX_SQL).execute(db.pool()).await?;
    Ok(())
}

// public:
/// One stored notification. Surfaced by [`list_for_user`] for
/// the admin UI.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: i64,
    pub user_id: i64,
    pub message: String,
    /// Optional click-through URL — empty string when the
    /// notification is informational only.
    pub url: String,
    /// `None` while the notification is unread; set to the
    /// dismissal timestamp once the operator hits "mark all read".
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// public:
/// Persist one notification targeted at `user_id`. `url` may be
/// empty when the message stands on its own (no click-through).
/// Returns the new row's id on success.
///
/// Project code calls this anywhere a `Db` is in scope — request
/// handlers, background jobs, periodic tasks. The framework's
/// own audit pipeline does not emit notifications today; this
/// is a project-facing surface.
pub async fn send(db: &Db, user_id: i64, message: &str, url: &str) -> Result<i64> {
    ensure_table(db).await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO rustio_notifications (user_id, message, url) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(user_id)
    .bind(message)
    .bind(url)
    .fetch_one(db.pool())
    .await?;
    Ok(id)
}

/// Unread count for `user_id`. Page handlers fetch this once
/// per render and pin it on `BaseContext` via
/// `with_unread_count`; the topbar badge in `_topbar.html`
/// branches on the result. Failure-soft — returns `0` on any
/// DB hiccup so the topbar stays mute rather than 500ing.
pub(crate) async fn unread_count(db: &Db, user_id: i64) -> i64 {
    let _ = ensure_table(db).await;
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM rustio_notifications \
         WHERE user_id = $1 AND read_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0)
}

/// List every notification for `user_id`, newest first. Empties
/// to an empty vec on error so the page renders.
pub(crate) async fn list_for_user(db: &Db, user_id: i64) -> Vec<Notification> {
    let _ = ensure_table(db).await;
    let rows = sqlx::query(
        "SELECT id, user_id, message, url, read_at, created_at \
         FROM rustio_notifications \
         WHERE user_id = $1 \
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();
    rows.iter()
        .map(|r| Notification {
            id: r.try_get("id").unwrap_or(0),
            user_id: r.try_get("user_id").unwrap_or(0),
            message: r.try_get("message").unwrap_or_default(),
            url: r.try_get("url").unwrap_or_default(),
            read_at: r.try_get("read_at").ok().flatten(),
            created_at: r.try_get("created_at").unwrap_or_else(|_| Utc::now()),
        })
        .collect()
}

/// Mark every unread notification for `user_id` as read. Stamps
/// `read_at = NOW()` on every affected row. Returns the count
/// of rows updated.
pub(crate) async fn mark_all_read(db: &Db, user_id: i64) -> i64 {
    let _ = ensure_table(db).await;
    let result = sqlx::query(
        "UPDATE rustio_notifications \
         SET read_at = NOW() \
         WHERE user_id = $1 AND read_at IS NULL",
    )
    .bind(user_id)
    .execute(db.pool())
    .await;
    match result {
        Ok(r) => r.rows_affected() as i64,
        Err(e) => {
            log::warn!("notifications::mark_all_read({user_id}): {e}");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_table_sql_is_idempotent_shape() {
        // The SQL is a stable contract for the framework's boot
        // path — anything that breaks `IF NOT EXISTS` re-runs is
        // a regression.
        assert!(CREATE_TABLE_SQL.contains("IF NOT EXISTS"));
        assert!(CREATE_INDEX_SQL.contains("IF NOT EXISTS"));
    }
}
