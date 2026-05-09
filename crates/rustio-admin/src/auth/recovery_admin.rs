//! Organisational recovery (R2).
//!
//! See `DESIGN_R2_ORGANISATIONAL.md` for the canonical contract this
//! module implements. R2 ships in 0.6.0; this commit lands the
//! schema only — the `LoginThrottle` struct, the `RecoveryPolicy`
//! extensions (login throttle, re-auth window, scope-for), the
//! runtime primitives (`record_failed_login`, `record_successful_login`,
//! `is_locked`, `apply_lock`, `clear_lock`, admin-issued reset, forced
//! rotation, re-auth wall), the handlers, the templates, and the
//! testcontainers integration test harness land in subsequent atomic
//! commits per `DESIGN_R2_ORGANISATIONAL.md` §11.
//!
//! ## What lives here today
//!
//! - [`migrate_user_lockout_schema`] — adds the additive
//!   `failed_login_count`, `last_failed_login_at`, and `locked_until`
//!   columns on `rustio_users` plus a partial index on `locked_until`
//!   for the "list currently-locked accounts" admin view (§9 of the
//!   design doc).
//!
//! ## Doctrine 22 reminder
//!
//! Centralised invalidation remains the single writer of `revoked_at`
//! on `rustio_sessions`. Auto-throttle (soft lock) does NOT revoke
//! sessions; manual lock DOES revoke sessions, via
//! [`crate::auth::sessions::invalidate_sessions`] with
//! `SessionInvalidationReason::AdminAction`. Nothing in this module
//! writes to `revoked_at` directly — see §13 of the design doc and
//! `DESIGN_SESSIONS.md` Doctrine 22 for the proof contract.
//!
//! Idempotent. Safe to call on every boot. `auth::init_tables`
//! invokes [`migrate_user_lockout_schema`] after R1's
//! `recovery::migrate_user_recovery_schema`.

use crate::error::Result;
use crate::orm::Db;

/// Add the additive R2 lockout columns on `rustio_users`.
///
/// - `failed_login_count INT NOT NULL DEFAULT 0` — incremented by
///   `record_failed_login` (R2 commit #9) and reset to zero by
///   `record_successful_login`. Pre-R2 rows default to 0; the auto-
///   throttle threshold is unaffected by historical state because the
///   counter is anchored to a sliding window via `last_failed_login_at`.
/// - `last_failed_login_at TIMESTAMPTZ` (nullable) — sliding-window
///   anchor for the auto-throttle threshold (§3.3 of the design
///   doc). NULL = never failed, or window has elapsed and the
///   counter has been logically reset.
/// - `locked_until TIMESTAMPTZ` (nullable) — when set and `> NOW()`,
///   the login flow refuses with the uniform "currently disabled"
///   page. NULL = unlocked. "Indefinite" manual locks are encoded
///   as a far-future timestamp (year 9999) so the column is never
///   NULL while locked — this lets the partial index find every
///   currently-locked account in a single seek.
///
/// Plus a partial index `rustio_users_locked_until_idx ON (locked_until)
/// WHERE locked_until IS NOT NULL` for the "list currently-locked
/// accounts" admin view (§9 — incident-triage surface). Storage cost
/// is small at admin-tier scale; the partial predicate keeps the
/// index a tiny fraction of the user table.
///
/// Idempotent. Safe to call on every boot. Depends on `rustio_users`
/// existing first.
pub(crate) async fn migrate_user_lockout_schema(db: &Db) -> Result<()> {
    sqlx::query(
        "ALTER TABLE rustio_users \
         ADD COLUMN IF NOT EXISTS failed_login_count INT NOT NULL DEFAULT 0",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS last_failed_login_at TIMESTAMPTZ",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ")
        .execute(db.pool())
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_users_locked_until_idx \
         ON rustio_users (locked_until) \
         WHERE locked_until IS NOT NULL",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}
