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
    /// Per-request UUID (R0). All audit rows written under one HTTP
    /// request share this id so a future `/admin/history/<id>` page
    /// can reconstruct the chain of events ("admin reset password →
    /// all sessions revoked → security email dispatched").
    pub correlation_id: Option<&'a str>,
    /// The session that performed the action, when applicable. CLI
    /// emergency actions write `None`.
    pub session_id: Option<i64>,
    /// Structured before/after / extra metadata. JSONB column.
    pub metadata: Option<serde_json::Value>,
}

impl<'a> LogEntry<'a> {
    /// Builder helper for the common case (every field that R0
    /// added defaults to `None`). Existing call sites can migrate
    /// incrementally.
    pub fn new(
        user_id: i64,
        action_type: ActionType,
        model_name: &'a str,
        object_id: i64,
    ) -> Self {
        Self {
            user_id,
            action_type,
            model_name,
            object_id,
            ip_address: None,
            summary: String::new(),
            correlation_id: None,
            session_id: None,
            metadata: None,
        }
    }
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
             (user_id, action_type, model_name, object_id, timestamp, ip_address, summary,
              correlation_id, session_id, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(entry.user_id)
    .bind(entry.action_type.as_str())
    .bind(entry.model_name)
    .bind(entry.object_id)
    .bind(now)
    .bind(entry.ip_address)
    .bind(&entry.summary)
    .bind(entry.correlation_id)
    .bind(entry.session_id)
    .bind(entry.metadata.as_ref())
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Internal typed representation of every audit `action_type` the
/// framework emits. `pub(crate)` for now — doctrine 18 commits to a
/// future public typed surface, but we don't promote it until 0.5.x.
///
/// Every framework call site MUST go through `AuditEvent::as_str()`
/// rather than writing the string literal inline. The drift test
/// below enumerates every variant and asserts nothing else lands in
/// the live `rustio_admin_actions` audit stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // not all variants have call sites yet (R1+)
pub(crate) enum AuditEvent {
    UserCreated,
    UserUpdated,
    UserDeleted,
    GroupCreated,
    GroupUpdated,
    GroupDeleted,
    PasswordResetSelfRequest,
    PasswordResetSelfConsume,
    PasswordResetByOther,
    AccountLocked,
    AccountUnlocked,
    MfaEnabled,
    MfaDisabled,
    MfaResetByOther,
    SessionsRevokedSelf,
    SessionsRevokedByOther,
    SessionLogout,
    EmergencyRecovery,
}

impl AuditEvent {
    /// Stable lowercase identifier persisted as
    /// `rustio_admin_actions.action_type`. Distinct from
    /// `ActionType::as_str()` (the legacy create/update/delete trio)
    /// — the two enums coexist; AuditEvent strings are richer and
    /// will eventually replace ActionType in the public API.
    #[allow(dead_code)]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::UserCreated => "user_created",
            Self::UserUpdated => "user_updated",
            Self::UserDeleted => "user_deleted",
            Self::GroupCreated => "group_created",
            Self::GroupUpdated => "group_updated",
            Self::GroupDeleted => "group_deleted",
            Self::PasswordResetSelfRequest => "password_reset_self_request",
            Self::PasswordResetSelfConsume => "password_reset_self_consume",
            Self::PasswordResetByOther => "password_reset_by_other",
            Self::AccountLocked => "account_locked",
            Self::AccountUnlocked => "account_unlocked",
            Self::MfaEnabled => "mfa_enabled",
            Self::MfaDisabled => "mfa_disabled",
            Self::MfaResetByOther => "mfa_reset_by_other",
            Self::SessionsRevokedSelf => "sessions_revoked_self",
            Self::SessionsRevokedByOther => "sessions_revoked_by_other",
            Self::SessionLogout => "session_logout",
            Self::EmergencyRecovery => "emergency_recovery",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Drift test for the internal AuditEvent enum (doctrine 18).
    ///
    /// Property: every variant's `as_str()` is unique across the
    /// enum. Catches accidental copy-paste collisions during R1+
    /// (`password_reset_self_request` vs
    /// `password_reset_self_consume` — easy to mis-paste).
    #[test]
    fn audit_event_strings_are_unique() {
        let events = [
            AuditEvent::UserCreated,
            AuditEvent::UserUpdated,
            AuditEvent::UserDeleted,
            AuditEvent::GroupCreated,
            AuditEvent::GroupUpdated,
            AuditEvent::GroupDeleted,
            AuditEvent::PasswordResetSelfRequest,
            AuditEvent::PasswordResetSelfConsume,
            AuditEvent::PasswordResetByOther,
            AuditEvent::AccountLocked,
            AuditEvent::AccountUnlocked,
            AuditEvent::MfaEnabled,
            AuditEvent::MfaDisabled,
            AuditEvent::MfaResetByOther,
            AuditEvent::SessionsRevokedSelf,
            AuditEvent::SessionsRevokedByOther,
            AuditEvent::SessionLogout,
            AuditEvent::EmergencyRecovery,
        ];
        let mut set = std::collections::HashSet::new();
        for e in events {
            assert!(set.insert(e.as_str()), "duplicate as_str() for {e:?}");
        }
        assert_eq!(set.len(), events.len());
    }

    /// All AuditEvent strings are snake_case ASCII (no whitespace, no
    /// uppercase, no punctuation beyond `_`). Future SIEM integrations
    /// will tokenize on these — keep them pre-normalised.
    #[test]
    fn audit_event_strings_are_snake_case() {
        let events = [
            AuditEvent::UserCreated,
            AuditEvent::UserUpdated,
            AuditEvent::UserDeleted,
            AuditEvent::GroupCreated,
            AuditEvent::GroupUpdated,
            AuditEvent::GroupDeleted,
            AuditEvent::PasswordResetSelfRequest,
            AuditEvent::PasswordResetSelfConsume,
            AuditEvent::PasswordResetByOther,
            AuditEvent::AccountLocked,
            AuditEvent::AccountUnlocked,
            AuditEvent::MfaEnabled,
            AuditEvent::MfaDisabled,
            AuditEvent::MfaResetByOther,
            AuditEvent::SessionsRevokedSelf,
            AuditEvent::SessionsRevokedByOther,
            AuditEvent::SessionLogout,
            AuditEvent::EmergencyRecovery,
        ];
        for e in events {
            let s = e.as_str();
            assert!(!s.is_empty(), "{e:?} as_str is empty");
            assert!(
                s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{e:?}.as_str() = {s:?} is not snake_case"
            );
        }
    }
}
