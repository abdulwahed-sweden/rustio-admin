//! Authentication & authorization.
//!
//! Three pieces:
//! - `users.rs`       — user records, password hashing, login
//! - `sessions.rs`    — DB-backed sessions with expiry cleanup
//! - `permissions.rs` — granular permissions + groups
//!
//! A user belongs to zero or more groups. Permissions come from two
//! sources: (a) direct assignments on the user, (b) inherited from
//! the user's groups. The permission string is
//! `<app>.<action>_<model>` — e.g. `posts.change_post`.

pub mod emergency;
pub mod guards;
pub(crate) mod mfa;
mod permissions;
pub(crate) mod recovery;
pub(crate) mod recovery_admin;
mod role;
// `sessions` is `pub(crate)` so the `__integration` test door at
// `crate::__integration` can re-export individual `pub(crate)`
// helpers (specifically `hash_token_for_storage`, used by the R4
// emergency-access integration suite to verify token-hash format
// parity with R1's consume path). Public re-exports still flow
// through `pub use sessions::{...}` below — this visibility bump
// does not change the external API surface.
pub(crate) mod sessions;
mod users;

pub use mfa::MfaPolicy;
pub(crate) use permissions::invalidate_user_cache;
pub use permissions::{
    add_user_to_group, check_permission, create_group, grant_to_group, grant_to_user,
    init_permission_tables, permissions_for_user, register_model_permissions,
    remove_user_from_group, Permission, PermissionError, Superuser,
};
pub use recovery::{
    DefaultPasswordPolicy, DefaultRecoveryPolicy, LoginThrottle, PasswordPolicy,
    PasswordPolicyError, RecoveryPolicy, SharedPasswordPolicy, SharedRecoveryPolicy,
};
// `issue_reset_token` / `consume_reset_token` and the `IssueOutcome` /
// `ConsumeOutcome` / `MailerEmailStatus` types live in `recovery`
// (`pub(crate) mod recovery`) so the admin handlers in commit #8+
// reach them as `crate::auth::recovery::*`. They are intentionally
// NOT re-exported here — the framework owns the handler shape, and
// projects compose recovery via the trait surfaces re-exported above.
// `purge_expired_reset_tokens` (R1 commit #12) is reached the same
// way from `background::spawn_session_sweeper`.
pub use role::{protected_roles, Role};
pub use sessions::{
    create_session, current_session_id, delete_session, identity_from_session, init_session_tables,
    invalidate_sessions, list_active_for_user, logout_session, purge_expired_sessions,
    session_token_from_cookie, InvalidationOutcome, Session, SessionInvalidationReason,
    SessionTarget, SessionTrust, SESSION_COOKIE,
};
#[allow(deprecated)]
pub use users::would_orphan_developers;
pub use users::{
    create_user, find_user_by_email, hash_password, init_user_tables, load_user_profile, login,
    migrate_user_schema, set_password, update_user_role, verdict_for_orphan_role, verify_password,
    would_orphan_protected, would_orphan_role, Identity, StoredUser, UserProfile,
};

use crate::error::Result;
use crate::orm::Db;

/// Initialise every auth-related table. Safe to call on every boot.
pub async fn init_tables(db: &Db) -> Result<()> {
    init_user_tables(db).await?;
    migrate_user_schema(db).await?;
    init_session_tables(db).await?;
    sessions::migrate_session_schema(db).await?;
    sessions::migrate_session_lifecycle(db).await?;
    init_permission_tables(db).await?;
    // R1 (0.5.0) — self password recovery schema. See
    // DESIGN_RECOVERY.md §9 for the contract.
    recovery::migrate_user_recovery_schema(db).await?;
    recovery::init_recovery_tables(db).await?;
    // R2 (0.6.0) — organisational recovery schema (lockout columns +
    // partial index). See DESIGN_R2_ORGANISATIONAL.md §4 for the
    // contract. Schema is additive and orthogonal to R1's recovery
    // tables; the runtime that reads these columns lands in later
    // R2 commits.
    recovery_admin::migrate_user_lockout_schema(db).await?;
    // R3 (0.7.0) — TOTP MFA schema (4 additive columns on
    // rustio_users + new rustio_mfa_backup_codes table + partial
    // index). See DESIGN_R3_MFA.md §7 for the contract. Schema
    // is additive and orthogonal to R1 / R2; the encryption
    // helpers, TOTP RFC 6238 implementation, enrolment /
    // verification / disable runtime, and login-flow integration
    // land in later R3 commits.
    mfa::migrate_user_mfa_schema(db).await?;
    // R2 (0.6.0) — surfaced by the testcontainers integration suite:
    // `rustio_admin_actions` was previously created lazily on first
    // dashboard hit (`handlers::ensure_audit_ready`), so any audit-
    // emitting path that ran BEFORE someone visited `/admin` would
    // fail. The R2 audit-heavy paths (auto-throttle, admin reset,
    // admin lock/unlock/revoke, forced rotation) made this latent
    // issue routinely reachable. Creating the table eagerly during
    // boot closes the gap; `ensure_table` is already idempotent so
    // this is safe to call alongside the lazy path that still
    // exists in the dashboard handler.
    crate::admin::audit::ensure_table(db).await?;
    Ok(())
}
