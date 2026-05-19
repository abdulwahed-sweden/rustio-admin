//! `rustio_admin_saved_filters` — per-user bookmarkable filter presets
//! on each list page.
//!
//! Operators routinely build the same filter / search / sort combo over
//! and over ("active patients this week," "appointments scheduled but
//! past time," …). This module persists those combos as named entries
//! keyed by `(user_id, admin_name, name)` so they survive sessions and
//! follow the operator across browsers.
//!
//! ## Surface contract
//!
//! - **Per-user scope.** A saved filter belongs to one user; another
//!   operator never sees it. There is no "team-shared" concept in v1;
//!   if shared presets become a real ask the surface can grow a
//!   `shared_with_group_id` column.
//! - **Per-model scope.** Filters are keyed by `admin_name` (the
//!   slug), so `Patient` filters don't bleed into `Practitioner`
//!   filters.
//! - **Stored payload is the raw URL query string** —
//!   `q=anna&status=active&sort=name&dir=asc` etc. Applying a saved
//!   filter is `<a href="/admin/<name>?<query_string>">`, no
//!   per-widget parser. New filter widgets ride for free.
//! - **`(user_id, admin_name, name)` is UNIQUE** so re-saving with the
//!   same name overwrites the row (caller does the upsert).
//!
//! ## Permissions
//!
//! Saving / deleting is gated at the route level on `Role::Staff` —
//! any operator who can reach the list page can also bookmark its
//! state. The handler scope-checks `user_id` on delete so one
//! operator can't drop another's bookmark by id-typing.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::Result;
use crate::orm::Db;

/// Hard caps on user-supplied free text so a malicious POST can't
/// stuff arbitrary kilobytes into `rustio_admin_saved_filters`.
/// Bounded by what the existing form widgets and URL parsers
/// produce in practice; far above the realistic operator input.
pub(crate) const NAME_MAX_LEN: usize = 120;
pub(crate) const QUERY_MAX_LEN: usize = 2048;

/// One bookmarked filter / search / sort state for one user on one
/// admin model.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SavedFilter {
    pub id: i64,
    pub user_id: i64,
    pub admin_name: String,
    pub name: String,
    pub query_string: String,
    pub created_at: DateTime<Utc>,
}

/// Idempotent CREATE TABLE — called by the list handler on first hit
/// for the model. Mirrors the lazy-init posture of `audit::ensure_table`
/// so the framework's own boot path stays narrow.
pub(crate) async fn ensure_table(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_admin_saved_filters (
            id           BIGSERIAL   PRIMARY KEY,
            user_id      BIGINT      NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
            admin_name   TEXT        NOT NULL,
            name         TEXT        NOT NULL,
            query_string TEXT        NOT NULL,
            created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT rustio_admin_saved_filters_unique
                UNIQUE (user_id, admin_name, name)
        )",
    )
    .execute(db.pool())
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_admin_saved_filters_user_model_idx
         ON rustio_admin_saved_filters (user_id, admin_name)",
    )
    .execute(db.pool())
    .await?;
    Ok(())
}

/// All saved filters for `(user_id, admin_name)`, ordered
/// alphabetically by name. Alphabetical is the right default for a
/// "bookmarks" UX — operators name these things and expect to find
/// `Active` next to `Awaiting`, not shuffled by recency on every
/// save.
pub(crate) async fn list_for_user(
    db: &Db,
    user_id: i64,
    admin_name: &str,
) -> Result<Vec<SavedFilter>> {
    let rows: Vec<(i64, i64, String, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, user_id, admin_name, name, query_string, created_at
         FROM rustio_admin_saved_filters
         WHERE user_id = $1 AND admin_name = $2
         ORDER BY name ASC",
    )
    .bind(user_id)
    .bind(admin_name)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, user_id, admin_name, name, query_string, created_at)| SavedFilter {
            id,
            user_id,
            admin_name,
            name,
            query_string,
            created_at,
        })
        .collect())
}

/// Upsert one row. Re-saving with the same name for the same user +
/// model overwrites the previous `query_string`. Returns the row id.
pub(crate) async fn save(
    db: &Db,
    user_id: i64,
    admin_name: &str,
    name: &str,
    query_string: &str,
) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO rustio_admin_saved_filters (user_id, admin_name, name, query_string)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, admin_name, name) DO UPDATE
           SET query_string = EXCLUDED.query_string,
               created_at = NOW()
         RETURNING id",
    )
    .bind(user_id)
    .bind(admin_name)
    .bind(name)
    .bind(query_string)
    .fetch_one(db.pool())
    .await?;
    Ok(row.0)
}

/// Delete one saved filter, scoped to the requesting user. Returns
/// `true` when the row was found and removed, `false` when the id
/// didn't match anything for this user (404-equivalent — never reveal
/// whether someone else's row exists).
pub(crate) async fn delete(db: &Db, user_id: i64, id: i64) -> Result<bool> {
    let res = sqlx::query(
        "DELETE FROM rustio_admin_saved_filters
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .execute(db.pool())
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Trim + length-cap an operator-supplied saved-filter name. Returns
/// `None` for empty / whitespace-only input so the handler can reject
/// with a clear flash. The cap matches the column's effective storage
/// limit ([`NAME_MAX_LEN`]); over that, truncate silently rather than
/// refuse — operators paste long titles and don't want a 400.
pub(crate) fn sanitise_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let bounded: String = if chars.len() > NAME_MAX_LEN {
        chars.into_iter().take(NAME_MAX_LEN).collect()
    } else {
        trimmed.to_string()
    };
    Some(bounded)
}

/// Trim + length-cap an operator-supplied query string. Empty is
/// allowed (saves "the default view"). Truncates silently like
/// [`sanitise_name`].
pub(crate) fn sanitise_query(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() > QUERY_MAX_LEN {
        chars.into_iter().take(QUERY_MAX_LEN).collect()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitise_name_rejects_empty_and_whitespace() {
        assert!(sanitise_name("").is_none());
        assert!(sanitise_name("   ").is_none());
        assert!(sanitise_name("\t\n").is_none());
    }

    #[test]
    fn sanitise_name_trims_surrounding_whitespace() {
        assert_eq!(sanitise_name("  Active patients  "), Some("Active patients".into()));
    }

    #[test]
    fn sanitise_name_truncates_at_cap() {
        let long = "a".repeat(NAME_MAX_LEN + 50);
        let out = sanitise_name(&long).unwrap();
        assert_eq!(out.chars().count(), NAME_MAX_LEN);
    }

    #[test]
    fn sanitise_name_handles_unicode_chars_correctly() {
        // Truncate by character count, not byte count, so multi-byte
        // chars stay intact.
        let long = "ñ".repeat(NAME_MAX_LEN + 5);
        let out = sanitise_name(&long).unwrap();
        assert_eq!(out.chars().count(), NAME_MAX_LEN);
    }

    #[test]
    fn sanitise_query_allows_empty_for_default_view() {
        assert_eq!(sanitise_query(""), "");
        assert_eq!(sanitise_query("   "), "");
    }

    #[test]
    fn sanitise_query_caps_long_input() {
        let long = "q=".to_string() + &"x".repeat(QUERY_MAX_LEN + 100);
        let out = sanitise_query(&long);
        assert_eq!(out.chars().count(), QUERY_MAX_LEN);
    }
}
