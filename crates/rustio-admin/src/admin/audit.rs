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
    /// R2 emissions populate this with an *object* (not a scalar
    /// or array) — the merge layer in [`record`] inserts
    /// `actor_user_id` and similar typed sidecars into the object
    /// before persistence, and a non-object value disables that
    /// merge (the row still writes; a warning is logged).
    pub metadata: Option<serde_json::Value>,
    /// The acting principal when this row records one user
    /// performing an action on another (admin password reset,
    /// admin lock/unlock, admin revoke-sessions, etc.). Persisted
    /// under `metadata.actor_user_id` — the column itself doesn't
    /// change.
    ///
    /// R2 emissions set this via [`LogEntry::with_actor`]; R0/R1
    /// emissions leave it `None`. The legacy [`Self::user_id`]
    /// field continues to carry the actor for backwards-compat
    /// with `/admin/history`'s "who did what" view; `actor_user_id`
    /// is the typed mirror that lets metadata consumers (SIEM,
    /// future per-user audit pivots) read the actor without
    /// relying on heuristics about which row-shape sets
    /// `user_id` to actor vs. target.
    ///
    /// When `Some(id)` is set and `metadata` is also `Some(obj)`,
    /// [`record`] inserts the key into the object. If the existing
    /// metadata already contains `actor_user_id`, the typed-field
    /// value wins — the struct field is the source of truth.
    pub actor_user_id: Option<i64>,
    /// When `Some`, supersedes `action_type.as_str()` as the
    /// persisted `rustio_admin_actions.action_type` string. Set via
    /// [`LogEntry::with_event`]; the `action_type` field becomes a
    /// placeholder in that case (the convention is to pass
    /// `ActionType::Update`). Used by R1+ recovery / authority /
    /// identity emissions that need the richer typed vocabulary —
    /// see `DESIGN_AUDIT.md` §3 + `DESIGN_RECOVERY.md` §6.
    pub event: Option<AuditEvent>,
}

impl<'a> LogEntry<'a> {
    /// Builder helper for the common case (every field that R0
    /// added defaults to `None`). Existing call sites can migrate
    /// incrementally.
    pub fn new(user_id: i64, action_type: ActionType, model_name: &'a str, object_id: i64) -> Self {
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
            actor_user_id: None,
            event: None,
        }
    }

    /// Mark this entry as an admin acting on another user. The id
    /// is persisted under `metadata.actor_user_id` by [`record`],
    /// not as a separate column. Pair with `.with_event(...)` for
    /// the canonical R2 admin-action shape, e.g.
    ///
    /// ```ignore
    /// LogEntry::new(target_id, ActionType::Update, "user", target_id)
    ///     .with_event(AuditEvent::PasswordResetByOther)
    ///     .with_actor(admin_identity.user_id)
    /// ```
    ///
    /// Auto-throttle (no actor) leaves this `None`; the row's
    /// `user_id` column carries the affected user as the
    /// closest-reasonable subject. See `DESIGN_R2_ORGANISATIONAL.md`
    /// §5.2.
    pub fn with_actor(mut self, actor_user_id: i64) -> Self {
        self.actor_user_id = Some(actor_user_id);
        self
    }

    /// Promote this entry's persisted `action_type` string from the
    /// legacy [`ActionType`] (create/update/delete) trio to the
    /// richer typed [`AuditEvent`]. The `action_type` field becomes
    /// a placeholder; the convention is to pass `ActionType::Update`
    /// to [`Self::new`] and chain `.with_event(...)`.
    ///
    /// ```ignore
    /// let entry = LogEntry::new(user_id, ActionType::Update, "user", user_id)
    ///     .with_event(AuditEvent::PasswordChangedSelf);
    /// ```
    ///
    /// Use this for framework-internal authority + identity +
    /// recovery audit rows per `DESIGN_AUDIT.md` §3 +
    /// `DESIGN_RECOVERY.md` §6. Project code that records generic
    /// CRUD on its own models continues to use [`Self::new`] alone
    /// with the legacy `ActionType` trio.
    pub fn with_event(mut self, event: AuditEvent) -> Self {
        self.event = Some(event);
        self
    }

    /// Resolve the persisted `action_type` string. The `event`
    /// override wins when set; otherwise the legacy `action_type`
    /// trio's lowercase string is used. Pulled out as a small helper
    /// so the `record()` insert and any future read-side rendering
    /// share one resolution rule.
    pub(crate) fn resolved_action_type(&self) -> &'static str {
        match self.event {
            Some(e) => e.as_str(),
            None => self.action_type.as_str(),
        }
    }
}

/// Merge `actor_user_id` into the persisted `metadata` JSONB column.
///
/// - When `actor_user_id` is `None`: returns `metadata` unchanged.
/// - When `actor_user_id` is `Some(id)` and `metadata` is `None`:
///   synthesizes `{"actor_user_id": id}`.
/// - When `actor_user_id` is `Some(id)` and `metadata` is
///   `Some(object)`: inserts the key, replacing any existing
///   `actor_user_id` (the typed field wins — it is the source of
///   truth for the actor association).
/// - When `actor_user_id` is `Some(id)` and `metadata` is
///   `Some(scalar | array)`: returns the original metadata
///   unchanged and emits a `log::warn!`. The merge requires an
///   object; non-object metadata is a programming error and the
///   audit row is preferable to silent loss.
///
/// Pulled out as a free function so the merge contract is
/// unit-testable without a database.
fn build_persisted_metadata(
    metadata: Option<serde_json::Value>,
    actor_user_id: Option<i64>,
) -> Option<serde_json::Value> {
    let actor = match actor_user_id {
        None => return metadata,
        Some(id) => id,
    };

    match metadata {
        None => Some(serde_json::json!({ "actor_user_id": actor })),
        Some(mut value) => {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("actor_user_id".to_string(), serde_json::json!(actor));
                Some(value)
            } else {
                log::warn!(
                    "audit::record: actor_user_id={} set but metadata is not a JSON object \
                     ({:?}); writing row without merging actor — fix the call site",
                    actor,
                    value
                );
                Some(value)
            }
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
    let action_type_str = entry.resolved_action_type();
    let metadata = build_persisted_metadata(entry.metadata, entry.actor_user_id);
    sqlx::query(
        "INSERT INTO rustio_admin_actions
             (user_id, action_type, model_name, object_id, timestamp, ip_address, summary,
              correlation_id, session_id, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(entry.user_id)
    .bind(action_type_str)
    .bind(entry.model_name)
    .bind(entry.object_id)
    .bind(now)
    .bind(entry.ip_address)
    .bind(&entry.summary)
    .bind(entry.correlation_id)
    .bind(entry.session_id)
    .bind(metadata.as_ref())
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Typed representation of every audit `action_type` the framework
/// emits for authority + identity + recovery actions.
///
/// **Public-API stability (0.5.0):** the enum is `pub` from R1
/// onwards (doctrine 18). External consumers — SIEM tooling, custom
/// dashboards, integration tests — can match on these variants
/// instead of (or in addition to) the persisted strings. The
/// `as_str()` mapping is the single canonical boundary between the
/// typed surface and the `rustio_admin_actions.action_type` TEXT
/// column. Every existing variant's string is locked-in by the
/// `audit_event_existing_variants_have_stable_strings` test below;
/// renaming a string is a breaking change requiring a major version
/// bump.
///
/// **Coexistence with `ActionType`:** the legacy
/// `ActionType::{Create, Update, Delete}` trio writes the strings
/// `"create" / "update" / "delete"`, used for generic CRUD on
/// project-registered models. `AuditEvent` strings are richer
/// (`"user_created"`, `"password_reset_self_consume"`, …) and used
/// for the framework's own authority + identity + recovery surfaces.
/// The two vocabularies are disjoint by design;
/// `action_type_and_audit_event_vocabularies_dont_collide` asserts
/// the disjointness.
///
/// **Future-extensibility:** `#[non_exhaustive]` lets future
/// R-phases (R2 / R3 / R4) add variants without breaking external
/// matchers. Variants whose call-sites haven't shipped yet are
/// listed here in anticipation — `as_str()` returns the canonical
/// string regardless of whether anything emits it. The roadmap
/// in `DESIGN_RECOVERY.md` §16 + `ROADMAP.md` covers when each
/// variant lights up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AuditEvent {
    // ---- User / Group authority CRUD (R0+) ----
    UserCreated,
    UserUpdated,
    UserDeleted,
    GroupCreated,
    GroupUpdated,
    GroupDeleted,
    // ---- Password lifecycle (R1+) ----
    /// Authenticated user changed their own password via
    /// `/admin/password_change`. R1 commit #11 wires emission.
    PasswordChangedSelf,
    /// Anonymous user requested a password-reset email via
    /// `/admin/forgot-password`. R1 commit #7 wires emission.
    PasswordResetSelfRequest,
    /// Anonymous user consumed a reset token + set a new password
    /// via `/admin/reset-password/<token>`. R1 commit #7 wires
    /// emission.
    PasswordResetSelfConsume,
    /// An administrator reset another user's password. R2 wires
    /// emission via the dedicated `/admin/users/<id>/reset-password`
    /// route.
    PasswordResetByOther,
    /// A user with `must_change_password = TRUE` completed the
    /// forced rotation via `POST /admin/must-change-password`,
    /// clearing the flag and (per `DESIGN_R2_ORGANISATIONAL.md`
    /// §3.4) revoking every other session for the same user.
    /// `metadata.triggered_by_audit_id` links back to the
    /// originating `PasswordResetByOther` row;
    /// `metadata.invalidated_session_count` records how many
    /// sessions were revoked. R2 commit #12 wires emission.
    ForcedPasswordChangeCompleted,
    // ---- Account state (R2+) ----
    AccountLocked,
    AccountUnlocked,
    // ---- MFA (R3+) ----
    MfaEnabled,
    MfaDisabled,
    MfaResetByOther,
    /// A user consumed a backup code as the second factor on
    /// the login or re-auth flow. Emitted by
    /// `auth::mfa::consume_backup_code` (R3 commit #8).
    /// `metadata.code_id` identifies which code was used;
    /// `metadata.remaining_codes` tracks the unused-code count
    /// post-consume so the user can be nudged toward
    /// regeneration before exhaustion;
    /// `metadata.via = "login" | "reauth"` distinguishes the
    /// caller context.
    MfaCodeConsumed,
    // ---- Session lifecycle (R0/R1+) ----
    SessionsRevokedSelf,
    SessionsRevokedByOther,
    SessionLogout,
    // ---- Layer-3 CLI (R4+) ----
    EmergencyRecovery,
}

impl AuditEvent {
    /// Stable lowercase identifier persisted as
    /// `rustio_admin_actions.action_type`.
    ///
    /// **Stability contract:** every string returned here is
    /// part of the public API from 0.5.0 onwards. Existing values
    /// are locked-in by
    /// `audit_event_existing_variants_have_stable_strings` and
    /// changing one is a breaking change requiring a major bump.
    /// New `AuditEvent` variants may be added in minor versions
    /// (the enum is `#[non_exhaustive]`); each new variant ships
    /// with its locked string from day one.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UserCreated => "user_created",
            Self::UserUpdated => "user_updated",
            Self::UserDeleted => "user_deleted",
            Self::GroupCreated => "group_created",
            Self::GroupUpdated => "group_updated",
            Self::GroupDeleted => "group_deleted",
            Self::PasswordChangedSelf => "password_changed_self",
            Self::PasswordResetSelfRequest => "password_reset_self_request",
            Self::PasswordResetSelfConsume => "password_reset_self_consume",
            Self::PasswordResetByOther => "password_reset_by_other",
            Self::ForcedPasswordChangeCompleted => "forced_password_change_completed",
            Self::AccountLocked => "account_locked",
            Self::AccountUnlocked => "account_unlocked",
            Self::MfaEnabled => "mfa_enabled",
            Self::MfaDisabled => "mfa_disabled",
            Self::MfaResetByOther => "mfa_reset_by_other",
            Self::MfaCodeConsumed => "mfa_code_consumed",
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

    /// Single source of truth for every `AuditEvent` variant the
    /// framework currently exposes. Drift tests below iterate over
    /// this constant; adding a new variant means adding it here.
    /// CHANGELOG / DESIGN_AUDIT.md call out variant additions.
    const ALL_AUDIT_EVENTS: &[AuditEvent] = &[
        AuditEvent::UserCreated,
        AuditEvent::UserUpdated,
        AuditEvent::UserDeleted,
        AuditEvent::GroupCreated,
        AuditEvent::GroupUpdated,
        AuditEvent::GroupDeleted,
        AuditEvent::PasswordChangedSelf,
        AuditEvent::PasswordResetSelfRequest,
        AuditEvent::PasswordResetSelfConsume,
        AuditEvent::PasswordResetByOther,
        AuditEvent::ForcedPasswordChangeCompleted,
        AuditEvent::AccountLocked,
        AuditEvent::AccountUnlocked,
        AuditEvent::MfaEnabled,
        AuditEvent::MfaDisabled,
        AuditEvent::MfaResetByOther,
        AuditEvent::MfaCodeConsumed,
        AuditEvent::SessionsRevokedSelf,
        AuditEvent::SessionsRevokedByOther,
        AuditEvent::SessionLogout,
        AuditEvent::EmergencyRecovery,
    ];

    /// Drift test (doctrine 18): every variant's `as_str()` is
    /// unique. Catches copy-paste collisions when adding variants
    /// — `password_reset_self_request` vs
    /// `password_reset_self_consume` are easy to mis-paste.
    #[test]
    fn audit_event_strings_are_unique() {
        let mut set = std::collections::HashSet::new();
        for &e in ALL_AUDIT_EVENTS {
            assert!(set.insert(e.as_str()), "duplicate as_str() for {e:?}");
        }
        assert_eq!(set.len(), ALL_AUDIT_EVENTS.len());
    }

    /// Every `AuditEvent` string is snake_case ASCII. Future SIEM
    /// integrations tokenise on these — keep them pre-normalised.
    #[test]
    fn audit_event_strings_are_snake_case() {
        for &e in ALL_AUDIT_EVENTS {
            let s = e.as_str();
            assert!(!s.is_empty(), "{e:?} as_str is empty");
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{e:?}.as_str() = {s:?} is not snake_case"
            );
        }
    }

    /// R1 commit #6: `PasswordChangedSelf` maps to the locked string
    /// `"password_changed_self"`. The string is part of the public
    /// API contract from 0.5.0; renaming requires a major bump.
    #[test]
    fn audit_event_password_changed_self_maps_correctly() {
        assert_eq!(
            AuditEvent::PasswordChangedSelf.as_str(),
            "password_changed_self"
        );
    }

    /// Stability contract for the public API: every existing
    /// variant's string value is locked-in here. A change to any of
    /// these strings is a breaking change requiring a major bump
    /// (the persisted `rustio_admin_actions.action_type` column
    /// would have rows referencing the old string from prior
    /// installations). New variants may extend this list; existing
    /// rows must keep their strings.
    #[test]
    fn audit_event_existing_variants_have_stable_strings() {
        assert_eq!(AuditEvent::UserCreated.as_str(), "user_created");
        assert_eq!(AuditEvent::UserUpdated.as_str(), "user_updated");
        assert_eq!(AuditEvent::UserDeleted.as_str(), "user_deleted");
        assert_eq!(AuditEvent::GroupCreated.as_str(), "group_created");
        assert_eq!(AuditEvent::GroupUpdated.as_str(), "group_updated");
        assert_eq!(AuditEvent::GroupDeleted.as_str(), "group_deleted");
        assert_eq!(
            AuditEvent::PasswordChangedSelf.as_str(),
            "password_changed_self"
        );
        assert_eq!(
            AuditEvent::PasswordResetSelfRequest.as_str(),
            "password_reset_self_request"
        );
        assert_eq!(
            AuditEvent::PasswordResetSelfConsume.as_str(),
            "password_reset_self_consume"
        );
        assert_eq!(
            AuditEvent::PasswordResetByOther.as_str(),
            "password_reset_by_other"
        );
        assert_eq!(
            AuditEvent::ForcedPasswordChangeCompleted.as_str(),
            "forced_password_change_completed"
        );
        assert_eq!(AuditEvent::AccountLocked.as_str(), "account_locked");
        assert_eq!(AuditEvent::AccountUnlocked.as_str(), "account_unlocked");
        assert_eq!(AuditEvent::MfaEnabled.as_str(), "mfa_enabled");
        assert_eq!(AuditEvent::MfaDisabled.as_str(), "mfa_disabled");
        assert_eq!(AuditEvent::MfaResetByOther.as_str(), "mfa_reset_by_other");
        assert_eq!(AuditEvent::MfaCodeConsumed.as_str(), "mfa_code_consumed");
        assert_eq!(
            AuditEvent::SessionsRevokedSelf.as_str(),
            "sessions_revoked_self"
        );
        assert_eq!(
            AuditEvent::SessionsRevokedByOther.as_str(),
            "sessions_revoked_by_other"
        );
        assert_eq!(AuditEvent::SessionLogout.as_str(), "session_logout");
        assert_eq!(AuditEvent::EmergencyRecovery.as_str(), "emergency_recovery");
    }

    /// `ActionType` and `AuditEvent` are intentionally separate
    /// vocabularies — `ActionType` writes generic CRUD strings
    /// (`"create" / "update" / "delete"`) for project-registered
    /// models; `AuditEvent` writes the framework's richer authority,
    /// identity, and recovery vocabulary. The two namespaces must
    /// stay disjoint so a SIEM consumer can route on the string
    /// alone without disambiguation.
    #[test]
    fn action_type_and_audit_event_vocabularies_dont_collide() {
        let action_type_strs = [
            ActionType::Create.as_str(),
            ActionType::Update.as_str(),
            ActionType::Delete.as_str(),
        ];
        let mut set = std::collections::HashSet::new();
        for s in action_type_strs {
            assert!(set.insert(s), "duplicate ActionType string {s:?}");
        }
        for &e in ALL_AUDIT_EVENTS {
            assert!(
                set.insert(e.as_str()),
                "AuditEvent::{:?} ({:?}) collides with ActionType",
                e,
                e.as_str()
            );
        }
        assert_eq!(set.len(), action_type_strs.len() + ALL_AUDIT_EVENTS.len());
    }

    // ---- LogEntry::with_event ----

    #[test]
    fn log_entry_with_event_overrides_action_type_persistence() {
        // Without with_event(), the legacy ActionType wins.
        let entry = LogEntry::new(1, ActionType::Update, "user", 1);
        assert_eq!(entry.resolved_action_type(), "update");

        // with_event() promotes to the richer AuditEvent string.
        let entry = LogEntry::new(1, ActionType::Update, "user", 1)
            .with_event(AuditEvent::PasswordChangedSelf);
        assert_eq!(entry.resolved_action_type(), "password_changed_self");

        // Different events resolve to their canonical string.
        let entry = LogEntry::new(1, ActionType::Update, "user", 1)
            .with_event(AuditEvent::PasswordResetSelfRequest);
        assert_eq!(entry.resolved_action_type(), "password_reset_self_request");

        let entry = LogEntry::new(1, ActionType::Update, "user", 1)
            .with_event(AuditEvent::PasswordResetSelfConsume);
        assert_eq!(entry.resolved_action_type(), "password_reset_self_consume");
    }

    #[test]
    fn log_entry_default_event_is_none() {
        // Backwards-compat: legacy callers continue to work.
        let entry = LogEntry::new(1, ActionType::Create, "post", 99);
        assert!(entry.event.is_none());
        assert_eq!(entry.resolved_action_type(), "create");
    }

    // ---- LogEntry::with_actor + build_persisted_metadata (R2 #7) -----------

    #[test]
    fn log_entry_with_actor_sets_field() {
        let entry = LogEntry::new(1, ActionType::Update, "user", 1).with_actor(7);
        assert_eq!(entry.actor_user_id, Some(7));
    }

    #[test]
    fn log_entry_default_actor_user_id_is_none() {
        // R0/R1 emissions leave actor_user_id None — only R2 admin
        // actions opt in via .with_actor(...).
        let entry = LogEntry::new(1, ActionType::Update, "user", 1);
        assert!(entry.actor_user_id.is_none());
    }

    #[test]
    fn merge_returns_metadata_unchanged_when_no_actor() {
        // None actor means no merge — even an existing
        // actor_user_id key in the input is preserved verbatim.
        let original = serde_json::json!({"reason": "x", "actor_user_id": 99});
        let out = build_persisted_metadata(Some(original.clone()), None);
        assert_eq!(out.unwrap(), original);

        // None metadata + None actor → None.
        assert!(build_persisted_metadata(None, None).is_none());
    }

    #[test]
    fn merge_synthesizes_object_when_metadata_is_none() {
        let out = build_persisted_metadata(None, Some(7)).unwrap();
        assert_eq!(out, serde_json::json!({"actor_user_id": 7}));
    }

    #[test]
    fn merge_inserts_into_existing_object() {
        let input = serde_json::json!({"reason": "support ticket", "mode": "email"});
        let out = build_persisted_metadata(Some(input), Some(7)).unwrap();
        assert_eq!(
            out,
            serde_json::json!({
                "reason": "support ticket",
                "mode": "email",
                "actor_user_id": 7
            })
        );
    }

    #[test]
    fn merge_typed_actor_wins_over_existing_metadata_key() {
        // Doctrine: the LogEntry struct field is the source of
        // truth for the actor association. A handler that puts
        // actor_user_id directly into metadata gets overridden
        // when it also sets the typed field — preventing
        // accidental inconsistency between the JSON key and the
        // struct.
        let input = serde_json::json!({"actor_user_id": 999, "extra": "x"});
        let out = build_persisted_metadata(Some(input), Some(7)).unwrap();
        assert_eq!(out, serde_json::json!({"actor_user_id": 7, "extra": "x"}));
    }

    #[test]
    fn merge_passes_through_non_object_metadata_with_warning() {
        // Non-object metadata is a programming bug. The merge
        // returns the original value unchanged so the row still
        // writes (audit-row-loss is worse than missing actor
        // metadata); a log::warn in the runtime path surfaces
        // the bug. Here we just assert the row preserves shape.
        let input = serde_json::json!(42);
        let out = build_persisted_metadata(Some(input.clone()), Some(7)).unwrap();
        assert_eq!(out, input);

        let input = serde_json::json!(["a", "b"]);
        let out = build_persisted_metadata(Some(input.clone()), Some(7)).unwrap();
        assert_eq!(out, input);

        let input = serde_json::json!("scalar");
        let out = build_persisted_metadata(Some(input.clone()), Some(7)).unwrap();
        assert_eq!(out, input);
    }

    /// The legacy `ActionType::parse` is a partial parser — it only
    /// recognises the original create/update/delete trio. Strings
    /// emitted by `AuditEvent` (and any free-form legacy strings
    /// already in older `rustio_admin_actions` rows) return `None`,
    /// which the render layer maps to a neutral pill class without
    /// panicking. This pins the property so a future change to
    /// `ActionType::parse` doesn't accidentally start matching
    /// AuditEvent strings.
    #[test]
    fn legacy_action_type_parser_returns_none_on_unknown_strings() {
        // Legacy trio still parses.
        assert_eq!(ActionType::parse("create"), Some(ActionType::Create));
        assert_eq!(ActionType::parse("update"), Some(ActionType::Update));
        assert_eq!(ActionType::parse("delete"), Some(ActionType::Delete));

        // Every AuditEvent string is unrecognised by the legacy
        // parser — the render layer falls through to "badge-neutral"
        // for these, which is the documented behaviour.
        for &e in ALL_AUDIT_EVENTS {
            assert!(
                ActionType::parse(e.as_str()).is_none(),
                "ActionType::parse should not recognise AuditEvent string {:?}",
                e.as_str()
            );
        }

        // Pure garbage and free-form legacy strings.
        assert!(ActionType::parse("garbage").is_none());
        assert!(ActionType::parse("").is_none());
        assert!(ActionType::parse("CREATE").is_none()); // case-sensitive
    }
}
