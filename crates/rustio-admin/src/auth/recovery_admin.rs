//! Organisational recovery (R2).
//!
//! See `DESIGN_R2_ORGANISATIONAL.md` for the canonical contract this
//! module implements. R2 ships in 0.6.0; this module owns every
//! recovery-related runtime fn except the existing R1 self-recovery
//! flow (which lives in `auth::recovery`). The HTTP wrappers live in
//! `admin::admin_recovery_handlers`; routes are registered in
//! `admin::routes::register_admin_routes`. The testcontainers
//! integration suite under `tests/integration_*.rs` exercises the
//! DB-touching paths end-to-end against an ephemeral Postgres,
//! gated behind `--features integration-test` per `DESIGN_R2_
//! ORGANISATIONAL.md` §10.3.
//!
//! ## Visibility note
//!
//! Items here are `pub` (rather than `pub(crate)`) so the
//! `crate::__integration` re-export module can re-export them under
//! the `integration-test` feature. The MODULE itself is
//! `pub(crate)` (`auth::mod`), so the canonical path
//! `rustio_admin::auth::recovery_admin::*` remains closed to
//! external callers — `__integration` is the only door, and it is
//! itself feature-gated + `#[doc(hidden)]`.
//!
//! ## What lives here today
//!
//! - [`migrate_user_lockout_schema`] — adds the additive
//!   `failed_login_count`, `last_failed_login_at`, and `locked_until`
//!   columns on `rustio_users` plus a partial index on `locked_until`
//!   for the "list currently-locked accounts" admin view (§9 of the
//!   design doc). R2 commit #1.
//! - [`check_account_lockout`] / [`LockState`] — read the lockout
//!   state for a user. The login flow calls this BEFORE password
//!   verification (R2 commit #9 + §3.3 + §9.1).
//! - [`record_failed_login`] / [`ThrottleOutcome`] — bump the
//!   sliding-window counter and stamp `locked_until` if the
//!   threshold is reached. Caller emits the typed
//!   `AuditEvent::AccountLocked` row when the outcome is
//!   `JustLocked`.
//! - [`record_successful_login`] — zeroes the counter on success.
//! - [`promote_session_elevated`] / [`check_session_elevated`] —
//!   re-auth wall runtime over the existing `rustio_sessions
//!   .elevated_until` column. R2 commit #10 + §3.5 + §11.
//! - [`issue_admin_reset_token`] / [`admin_set_temp_password`] /
//!   [`AdminActor`] / [`AdminIssueOutcome`] / [`AdminTempPwOutcome`]
//!   — admin-driven password reset (email + temp_pw modes) per
//!   §3.1. Both modes emit `AuditEvent::PasswordResetByOther` with
//!   the unified §5.1 metadata schema; temp_pw additionally emits
//!   per-revoked-session `SessionsRevokedByOther` rows. R2 commit
//!   #14 + §3.1.
//! - [`lock_user_account`] / [`unlock_user_account`] /
//!   [`admin_revoke_sessions`] / [`LockDuration`] / [`LockOutcome`] /
//!   [`UnlockOutcome`] / [`AdminRevokeOutcome`] — manual lock /
//!   unlock / revoke admin actions per §3.2. Lock revokes every
//!   session via `AdministrativeRevoke`; unlock does NOT touch
//!   sessions; revoke-only is a sibling of lock without the
//!   `locked_until` write. R2 commit #16 + §3.2.
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

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::Row as _;

use crate::admin::audit::{record as audit_record, ActionType, AuditEvent, LogEntry};
use crate::admin::builtin::client_ip;
use crate::admin::redact::redact_token;
use crate::admin::Admin;
use crate::auth::recovery::{LoginThrottle, MailerEmailStatus};
use crate::auth::sessions::{hash_token_for_storage, random_token};
use crate::auth::{invalidate_sessions, set_password, SessionInvalidationReason, SessionTarget};
use crate::email::Mail;
use crate::error::Result;
use crate::http::Request;
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
pub async fn migrate_user_lockout_schema(db: &Db) -> Result<()> {
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

// ---- Login-throttle runtime (R2 commit #9) ---------------------------------

/// Whether an account is currently soft-locked. Returned by
/// [`check_account_lockout`]; the login flow refuses with a uniform
/// 401 on `Locked`, regardless of whether the next field would have
/// been a correct password.
///
/// `until` is the absolute UTC instant the lock expires
/// (`rustio_users.locked_until` from the row). The value is
/// informational: the lockout check itself uses `> NOW()` semantics,
/// so callers don't need to compare the timestamp themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    /// Account is logged-in-able as far as throttle state is
    /// concerned (`locked_until IS NULL` or `<= NOW()`).
    Unlocked,
    /// Account has `locked_until > NOW()`. The login flow returns
    /// the uniform 401; admin can manually unlock earlier via
    /// `/admin/users/:id/unlock` (R2 commit #16).
    Locked { until: DateTime<Utc> },
}

/// Outcome of [`record_failed_login`]. Lets the caller decide
/// whether to emit the typed [`crate::admin::audit::AuditEvent::AccountLocked`]
/// row — emission lives at the call site so it can attach the
/// request's `correlation_id` and `ip_address` without threading
/// them through the runtime layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleOutcome {
    /// Counter incremented, threshold not yet reached.
    Recorded { count: i32 },
    /// This failure tripped the threshold; account is now locked
    /// until `until`. Caller emits `AccountLocked` audit row with
    /// `via: "auto_throttle"` metadata.
    JustLocked { count: i32, until: DateTime<Utc> },
    /// `LoginThrottle::max_attempts == 0` — auto-throttle is
    /// disabled by policy. The counter is still incremented so
    /// admins inspecting `rustio_users.failed_login_count` see
    /// signal, but no lock is applied. Caller MUST NOT emit
    /// `AccountLocked` for this variant.
    Disabled { count: i32 },
}

/// Read the lockout state for a user. Cheap — single indexed lookup
/// on `id`. The login flow calls this BEFORE password verification
/// (per `DESIGN_R2_ORGANISATIONAL.md` §3.3 + §9.1) so a locked
/// account's response time stays uniform with a wrong-password
/// response.
///
/// Returns `LockState::Unlocked` for non-existent users (the caller
/// should `find_user_by_email` first; this fn is keyed on a verified
/// id). Returns `LockState::Unlocked` when `locked_until IS NULL`
/// or has already elapsed.
pub async fn check_account_lockout(db: &Db, user_id: i64) -> Result<LockState> {
    let row = sqlx::query("SELECT locked_until FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db.pool())
        .await?;

    let Some(row) = row else {
        // User row missing — fail open. The caller's earlier
        // `find_user_by_email` should have caught this; this fn
        // doesn't fabricate authentication state.
        return Ok(LockState::Unlocked);
    };

    let locked_until: Option<DateTime<Utc>> = row.try_get("locked_until")?;
    match locked_until {
        Some(until) if until > Utc::now() => Ok(LockState::Locked { until }),
        _ => Ok(LockState::Unlocked),
    }
}

/// Record a failed login attempt. Increments
/// `rustio_users.failed_login_count`, stamps `last_failed_login_at`,
/// and — if the threshold trips — sets `locked_until` to NOW() +
/// `throttle.lock_minutes`.
///
/// Sliding-window semantics: when `last_failed_login_at` is older
/// than `throttle.window_minutes` (or NULL), the counter is reset to
/// 1 first. So an attacker can't accumulate failures over hours;
/// every burst gets its own window.
///
/// Auto-throttle does NOT revoke sessions (Doctrine 22 + §13
/// locked-decision). The caller emits `AuditEvent::AccountLocked`
/// when the returned outcome is [`ThrottleOutcome::JustLocked`].
///
/// Two-step SQL by design: the first UPDATE is the atomic counter
/// bump; the second writes `locked_until` only when needed. The
/// TOCTOU window is benign — concurrent failures may both pass the
/// threshold and both write the same `locked_until` value
/// (idempotent), and a concurrent successful login can't happen
/// because the password was wrong on this attempt.
pub async fn record_failed_login(
    db: &Db,
    user_id: i64,
    throttle: LoginThrottle,
) -> Result<ThrottleOutcome> {
    // Step 1: bump the counter, with sliding-window reset.
    let row = sqlx::query(
        "UPDATE rustio_users SET \
            failed_login_count = CASE \
                WHEN last_failed_login_at IS NULL \
                  OR last_failed_login_at < NOW() - (INTERVAL '1 minute' * $2::int) \
                    THEN 1 \
                    ELSE failed_login_count + 1 \
            END, \
            last_failed_login_at = NOW() \
          WHERE id = $1 \
          RETURNING failed_login_count",
    )
    .bind(user_id)
    .bind(throttle.window_minutes as i32)
    .fetch_one(db.pool())
    .await?;
    let new_count: i32 = row.try_get("failed_login_count")?;

    // Step 2: when threshold met (and auto-throttle is enabled),
    // stamp `locked_until`. `max_attempts == 0` disables the
    // auto-throttle entirely per the LoginThrottle docs.
    if throttle.max_attempts == 0 {
        return Ok(ThrottleOutcome::Disabled { count: new_count });
    }
    if (new_count as u32) < throttle.max_attempts {
        return Ok(ThrottleOutcome::Recorded { count: new_count });
    }

    let row = sqlx::query(
        "UPDATE rustio_users SET \
            locked_until = NOW() + (INTERVAL '1 minute' * $2::int) \
          WHERE id = $1 \
          RETURNING locked_until",
    )
    .bind(user_id)
    .bind(throttle.lock_minutes as i32)
    .fetch_one(db.pool())
    .await?;
    let until: DateTime<Utc> = row.try_get("locked_until")?;
    Ok(ThrottleOutcome::JustLocked {
        count: new_count,
        until,
    })
}

/// Record a successful login. Zeroes `failed_login_count` and clears
/// `last_failed_login_at`. `locked_until` is left alone — the row
/// reaches this fn only when `check_account_lockout` returned
/// `Unlocked`, so the column is either NULL or already in the past;
/// the partial-index hygiene path is acceptable.
///
/// Idempotent on its own column writes — repeating the call against
/// an already-zeroed row is a no-op at the database level.
pub async fn record_successful_login(db: &Db, user_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE rustio_users SET \
            failed_login_count = 0, \
            last_failed_login_at = NULL \
          WHERE id = $1",
    )
    .bind(user_id)
    .execute(db.pool())
    .await?;
    Ok(())
}

// ---- Re-auth wall runtime (R2 commit #10) ----------------------------------
//
// State lives entirely on the existing `rustio_sessions.elevated_until`
// column added in R0 (see `auth::sessions::migrate_session_lifecycle`).
// No schema change here — the column was scaffolded, this commit lands
// the reader and writer.
//
// Doctrine 22 reminder: neither fn touches `revoked_at`. Promotion only
// extends the elevation window; expiry is detected at read time
// (`elevated_until > NOW()`). When a session needs to be revoked
// (manual lock, password reset by other), that goes through
// `auth::sessions::invalidate_sessions`, not these helpers.

/// Promote a session into the *elevated* trust band, valid for `ttl`
/// from now. The login flow's re-auth wall (handler in R2 commit #11)
/// calls this after the actor re-verifies their password; the admin-
/// recovery handlers in commits #15 / #16 read the resulting
/// `elevated_until` via [`check_session_elevated`] before allowing a
/// destructive mutation.
///
/// Idempotent: re-promoting an already-elevated session simply
/// extends the window to a fresh `NOW() + ttl`. Promotion is a no-op
/// for revoked sessions (`AND revoked_at IS NULL` in the WHERE
/// clause); the caller's preceding identity / cookie checks should
/// already have rejected those.
///
/// `ttl` is read from `RecoveryPolicy::reauth_window()` (default 15
/// minutes per `DESIGN_R2_ORGANISATIONAL.md` §12). Negative or zero
/// durations are written as-is — the resulting `elevated_until` lands
/// at-or-before `NOW()`, so [`check_session_elevated`] returns
/// `false` and every admin-recovery action will require a fresh
/// re-auth. That's the documented escape hatch when a project sets
/// `reauth_window = ChronoDuration::zero()`.
pub async fn promote_session_elevated(db: &Db, session_id: i64, ttl: ChronoDuration) -> Result<()> {
    sqlx::query(
        "UPDATE rustio_sessions \
            SET elevated_until = NOW() + (INTERVAL '1 second' * $2::bigint), \
                trust_level = 'elevated' \
          WHERE session_id = $1 AND revoked_at IS NULL",
    )
    .bind(session_id)
    .bind(ttl.num_seconds())
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Whether the given session is currently elevated. Returns `true`
/// iff the session row exists, is not revoked, and
/// `elevated_until > NOW()`. Returns `false` for missing /
/// revoked / never-promoted / already-expired sessions.
///
/// The admin-recovery handlers in R2 commits #15 / #16 call this
/// before rendering the destructive action's form. On `false` the
/// handler redirects to `/admin/reauth?return_to=<encoded>` (R2
/// commit #11). The post-reauth handler then promotes the session
/// and redirects back to the original URL, which now passes the
/// check.
pub async fn check_session_elevated(db: &Db, session_id: i64) -> Result<bool> {
    let row = sqlx::query(
        "SELECT elevated_until FROM rustio_sessions \
          WHERE session_id = $1 AND revoked_at IS NULL",
    )
    .bind(session_id)
    .fetch_optional(db.pool())
    .await?;

    let Some(row) = row else {
        return Ok(false);
    };
    let elevated_until: Option<DateTime<Utc>> = row.try_get("elevated_until")?;
    Ok(matches!(elevated_until, Some(t) if t > Utc::now()))
}

// ---- Admin-driven password reset (R2 commit #14) ---------------------------
//
// Two modes per `DESIGN_R2_ORGANISATIONAL.md` §3.1:
//
// - email mode: insert a reset-token row (R1's table), dispatch a
//   reset-link email to the target. Target consumes via R1's
//   `consume_reset_token` (`/admin/reset-password/<token>`); that
//   path already revokes every session for the user via
//   `SessionInvalidationReason::PasswordReset`.
// - temp_pw mode: generate a 16-char URL-safe-base64 temp password,
//   call `set_password(target, temp)` directly, set
//   `must_change_password = TRUE`, revoke EVERY session via
//   `SessionInvalidationReason::PasswordResetByOther`. The temp
//   password is returned in the outcome and rendered ONCE on the
//   admin's success page (handler in commit #15) — never logged,
//   never persisted in plaintext.
//
// Both modes emit one `AuditEvent::PasswordResetByOther` row with
// the unified §5.1 metadata schema. temp_pw additionally emits one
// `AuditEvent::SessionsRevokedByOther` row per revoked session. The
// audit chain (§5.3) becomes complete once the user completes the
// forced rotation in the must-change-password handler (commit #12).
//
// Doctrine 22 invariant: every session revocation goes through
// `auth::invalidate_sessions(...)`. Neither runtime fn touches
// `revoked_at` directly. Doctrine grep returns the same 3 SET arms.

/// Generate a 16-char URL-safe-base64 temp password
/// (`DESIGN_R2_ORGANISATIONAL.md` §12 locked decision — not
/// project-tunable). 12 random bytes → 16 base64-no-pad chars,
/// ~96 bits of entropy.
fn random_temp_password() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// 8-char SHA-256 fingerprint of the actor's email, persisted in
/// audit metadata as `actor_email_hash` (`DESIGN_R2_ORGANISATIONAL.md`
/// §13 locked decision: never log/audit actor email plaintext).
/// One-way function; not reversible. Same b64 alphabet
/// `redact_token` uses.
fn actor_email_fingerprint(email: &str) -> String {
    let digest = Sha256::digest(email.as_bytes());
    let b64 = URL_SAFE_NO_PAD.encode(digest);
    b64.chars().take(8).collect()
}

/// Look up the target user by id. Returns `(email, is_active)` or
/// `None` for a missing row. Read-only; the caller maps the result
/// to the `UnknownTarget` / `InactiveTarget` outcome variants.
async fn load_admin_reset_target(db: &Db, target_user_id: i64) -> Result<Option<(String, bool)>> {
    let row = sqlx::query("SELECT email, is_active FROM rustio_users WHERE id = $1")
        .bind(target_user_id)
        .fetch_optional(db.pool())
        .await?;
    match row {
        Some(r) => {
            let email: String = r.try_get("email")?;
            let is_active: bool = r.try_get("is_active")?;
            Ok(Some((email, is_active)))
        }
        None => Ok(None),
    }
}

/// Update the `mail_status` column on a freshly-inserted reset-token
/// row. Same shape as R1's private `set_token_mail_status`; copy
/// rather than promote that helper's visibility to keep R1 / R2
/// coupling minimal.
async fn update_token_mail_status(db: &Db, token_id: i64, status: &str) -> Result<()> {
    sqlx::query("UPDATE rustio_password_reset_tokens SET mail_status = $1 WHERE id = $2")
        .bind(status)
        .bind(token_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// The acting administrator. Bundled because both admin-reset
/// runtime fns need the same two fields and the unbundled
/// signatures otherwise hit clippy's `too_many_arguments` floor.
/// `email` is hashed into `metadata.actor_email_hash`; the
/// plaintext NEVER lands in audit metadata
/// (`DESIGN_R2_ORGANISATIONAL.md` §13).
#[derive(Debug, Clone, Copy)]
pub struct AdminActor<'a> {
    pub user_id: i64,
    pub email: &'a str,
}

/// Outcome of [`issue_admin_reset_token`] (email mode). The handler
/// (R2 commit #15) maps `Issued` to a "reset email sent" success
/// page; `UnknownTarget` and `InactiveTarget` both render a
/// generic "could not reset" message — we intentionally do NOT
/// distinguish the two from inside the admin surface either,
/// because the admin already sees the user list and any divergence
/// here just adds UX complexity for no security benefit.
#[allow(dead_code)] // variant fields consumed by R2 commit #15 handler
#[derive(Debug, Clone)]
pub enum AdminIssueOutcome {
    /// Token row inserted, mail dispatch attempted (status in
    /// `email_status`), audit row written. The token plaintext is
    /// in the email body — never returned from this fn.
    Issued {
        target_user_id: i64,
        token_id: i64,
        email_status: MailerEmailStatus,
    },
    /// `target_user_id` matched no row in `rustio_users`. No
    /// token, no audit, no mail. The handler maps this to a 404.
    UnknownTarget,
    /// Target row exists but `is_active = FALSE`. No mutation.
    /// The handler can render "this account is disabled" since
    /// the admin already has visibility into the row.
    InactiveTarget,
}

/// Outcome of [`admin_set_temp_password`] (temp_pw mode). On
/// success the plaintext temp password is in `temp_password` —
/// the handler (commit #15) renders it ONCE on the success page
/// and discards it. The framework writes the temp password ONLY
/// to `rustio_users.password_hash` (Argon2id) via `set_password`;
/// the plaintext NEVER lands in any log line, audit row, or
/// other DB column.
#[allow(dead_code)] // variant fields consumed by R2 commit #15 handler
#[derive(Debug, Clone)]
pub enum AdminTempPwOutcome {
    Set {
        target_user_id: i64,
        temp_password: String,
        revoked_session_count: usize,
    },
    UnknownTarget,
    InactiveTarget,
}

/// Issue an admin-initiated password-reset email. The target
/// receives a link to R1's `/admin/reset-password/<token>` flow,
/// which already handles the password-change + session-revocation
/// path; R2's contribution here is the *origin* of the token row
/// (admin-initiated, with the actor pinned in audit metadata).
///
/// Inputs:
/// - `target_user_id`: the user the admin is resetting.
/// - `actor`: bundles the admin's `user_id` (pinned in
///   `metadata.actor_user_id` via `LogEntry::with_actor`) and
///   `email` (hashed into `metadata.actor_email_hash`; plaintext
///   NEVER lands in audit metadata).
/// - `reason`: free-text justification. Persisted in
///   `metadata.reason` verbatim.
///
/// Doctrine 22: this fn does NOT revoke sessions. Revocation happens
/// in R1's `consume_reset_token` when the target consumes the link,
/// via `SessionInvalidationReason::PasswordReset`. The path is
/// audit-distinguishable from a self-initiated reset by the
/// `PasswordResetByOther` event type on this issue row.
pub async fn issue_admin_reset_token(
    db: &Db,
    admin: &Admin,
    request: &Request,
    target_user_id: i64,
    actor: AdminActor<'_>,
    reason: &str,
    correlation_id: Option<&str>,
) -> Result<AdminIssueOutcome> {
    let (target_email, is_active) = match load_admin_reset_target(db, target_user_id).await? {
        Some(v) => v,
        None => return Ok(AdminIssueOutcome::UnknownTarget),
    };
    if !is_active {
        return Ok(AdminIssueOutcome::InactiveTarget);
    }

    // Generate token + insert row. Same shape as R1's
    // `issue_reset_token`; the consume path (R1's
    // `consume_reset_token`) doesn't distinguish admin-issued
    // tokens from self-issued ones — it can't, and shouldn't.
    let token = random_token();
    let token_hash = hash_token_for_storage(&token);
    let policy = admin.active_recovery_policy();
    let ttl = policy.reset_token_ttl();
    let expires_at = Utc::now() + ttl;
    let ip = client_ip(request);
    let user_agent = request.header("user-agent").map(|s| s.to_string());

    let token_id: i64 = sqlx::query_scalar(
        "INSERT INTO rustio_password_reset_tokens \
            (user_id, token_hash, requested_ip, requested_user_agent, \
             expires_at, mail_status, correlation_id) \
         VALUES ($1, $2, $3, $4, $5, 'pending', $6) \
       RETURNING id",
    )
    .bind(target_user_id)
    .bind(&token_hash)
    .bind(ip.as_deref())
    .bind(user_agent.as_deref())
    .bind(expires_at)
    .bind(correlation_id)
    .fetch_one(db.pool())
    .await?;

    // Compose + dispatch the reset-link email. If site-URL
    // derivation fails or the mailer errors, mark mail_status =
    // 'failed' and continue — the audit row records the failure
    // and the admin sees it on the success page.
    let mail_status = match policy.public_site_url(request) {
        Some(public_site_url) => {
            let reset_link = format!(
                "{}/admin/reset-password/{}",
                public_site_url.trim_end_matches('/'),
                token,
            );
            let when = Utc::now();
            let body = format!(
                "An administrator at {site_header} has reset your password.\n\n\
                 Click the link below to set a new password:\n\n\
                 {reset_link}\n\n\
                 The link expires shortly. If you have questions, contact your \
                 administrator.\n",
                site_header = admin.branding().site_header,
                reset_link = reset_link,
            );
            let mail = Mail::framework_envelope(
                target_email.clone(),
                format!("{} — password reset", admin.branding().site_header),
                body,
                &admin.branding().site_header,
                ip.as_deref(),
                user_agent.as_deref(),
                when,
            );
            match admin.active_mailer().send(mail).await {
                Ok(()) => {
                    update_token_mail_status(db, token_id, "sent").await?;
                    MailerEmailStatus::Sent
                }
                Err(e) => {
                    log::error!(
                        target: "rustio_admin::recovery_admin::issue",
                        "mailer send failed target_user_id={} actor_user_id={} \
                         fingerprint={} correlation_id={:?}: {}",
                        target_user_id,
                        actor.user_id,
                        redact_token(&token),
                        correlation_id,
                        e,
                    );
                    update_token_mail_status(db, token_id, "failed").await?;
                    MailerEmailStatus::Failed
                }
            }
        }
        None => {
            log::error!(
                target: "rustio_admin::recovery_admin::issue",
                "public_site_url derivation returned None — admin reset link \
                 cannot be built. target_user_id={} actor_user_id={} \
                 fingerprint={} correlation_id={:?}",
                target_user_id,
                actor.user_id,
                redact_token(&token),
                correlation_id,
            );
            update_token_mail_status(db, token_id, "failed").await?;
            MailerEmailStatus::Failed
        }
    };

    // Audit row — `PasswordResetByOther` with §5.1 unified
    // metadata schema. Token plaintext is NEVER in metadata; the
    // 8-char fingerprint via `redact_token` is the cross-row
    // pivot key.
    let metadata = serde_json::json!({
        "actor_email_hash": actor_email_fingerprint(actor.email),
        "reason": reason,
        "mode": "email",
        "email_send_status": match mail_status {
            MailerEmailStatus::Sent => "sent",
            MailerEmailStatus::Failed => "failed",
        },
        "must_change_password_set": false,
        "token_fingerprint": redact_token(&token),
    });
    let mut entry = LogEntry::new(target_user_id, ActionType::Update, "users", target_user_id)
        .with_event(AuditEvent::PasswordResetByOther)
        .with_actor(actor.user_id);
    entry.correlation_id = correlation_id;
    entry.ip_address = ip.as_deref();
    entry.metadata = Some(metadata);
    entry.summary = format!(
        "admin password reset (email mode); mail {}",
        match mail_status {
            MailerEmailStatus::Sent => "sent",
            MailerEmailStatus::Failed => "failed",
        }
    );
    audit_record(db, entry).await?;

    Ok(AdminIssueOutcome::Issued {
        target_user_id,
        token_id,
        email_status: mail_status,
    })
}

/// Set a temp password directly. The admin shares the temp value
/// with the target out-of-band (in person, secure chat, etc.).
/// Target signs in with the temp password → `must_change_password`
/// gate → forced rotation interstitial (commit #12) sets the real
/// value.
///
/// Inputs match [`issue_admin_reset_token`] minus `admin: &Admin`
/// (no mailer needed). On `AdminTempPwOutcome::Set` the caller
/// MUST render `temp_password` exactly once and discard it; the
/// plaintext is not persisted anywhere reachable by a later
/// request.
///
/// Doctrine 22: session revocation goes through
/// [`crate::auth::invalidate_sessions`] with
/// `SessionInvalidationReason::PasswordResetByOther`. No direct
/// `revoked_at` write.
pub async fn admin_set_temp_password(
    db: &Db,
    request: &Request,
    target_user_id: i64,
    actor: AdminActor<'_>,
    reason: &str,
    correlation_id: Option<&str>,
) -> Result<AdminTempPwOutcome> {
    let (_target_email, is_active) = match load_admin_reset_target(db, target_user_id).await? {
        Some(v) => v,
        None => return Ok(AdminTempPwOutcome::UnknownTarget),
    };
    if !is_active {
        return Ok(AdminTempPwOutcome::InactiveTarget);
    }

    let temp_password = random_temp_password();

    // Apply the password change. `set_password` writes the hash +
    // stamps `password_changed_at` (R1 commit #2 invariant).
    set_password(db, target_user_id, &temp_password).await?;

    // Force rotation on next sign-in.
    sqlx::query("UPDATE rustio_users SET must_change_password = TRUE WHERE id = $1")
        .bind(target_user_id)
        .execute(db.pool())
        .await?;

    // Revoke EVERY session for the target. Doctrine 22:
    // invalidate_sessions is the sole writer of `revoked_at`.
    let outcome = invalidate_sessions(
        db,
        SessionTarget::User {
            user_id: target_user_id,
        },
        SessionInvalidationReason::PasswordResetByOther,
    )
    .await?;
    let revoked_count = outcome.revoked_session_ids.len();

    let ip = client_ip(request);

    // Per-revoked-session SessionsRevokedByOther rows. The R1
    // `record_session_revocations` helper writes `SessionsRevokedSelf`;
    // that's the wrong event for an admin-driven revocation, so we
    // emit the rows inline here with the correct typed event +
    // actor pinned.
    for revoked_id in &outcome.revoked_session_ids {
        let metadata = serde_json::json!({
            "session_id": revoked_id,
            "reason": reason,
            "via": "admin_password_reset",
        });
        let mut entry = LogEntry::new(target_user_id, ActionType::Update, "users", target_user_id)
            .with_event(AuditEvent::SessionsRevokedByOther)
            .with_actor(actor.user_id);
        entry.correlation_id = correlation_id;
        entry.ip_address = ip.as_deref();
        entry.metadata = Some(metadata);
        entry.summary = format!("session {revoked_id} revoked by admin (temp_pw reset)");
        if let Err(e) = audit_record(db, entry).await {
            log::error!(
                target: "rustio_admin::recovery_admin::temp_pw",
                "audit::record (SessionsRevokedByOther) failed target_user_id={} \
                 session_id={}: {}",
                target_user_id, revoked_id, e,
            );
        }
    }

    // The headline `PasswordResetByOther` row.
    let metadata = serde_json::json!({
        "actor_email_hash": actor_email_fingerprint(actor.email),
        "reason": reason,
        "mode": "temp_pw",
        "must_change_password_set": true,
        "invalidated_session_count": revoked_count,
    });
    let mut entry = LogEntry::new(target_user_id, ActionType::Update, "users", target_user_id)
        .with_event(AuditEvent::PasswordResetByOther)
        .with_actor(actor.user_id);
    entry.correlation_id = correlation_id;
    entry.ip_address = ip.as_deref();
    entry.metadata = Some(metadata);
    entry.summary =
        format!("admin password reset (temp_pw mode); {revoked_count} session(s) revoked");
    audit_record(db, entry).await?;

    Ok(AdminTempPwOutcome::Set {
        target_user_id,
        temp_password,
        revoked_session_count: revoked_count,
    })
}

// ---- Lock / unlock / admin-revoke (R2 commit #16) --------------------------
//
// Three runtime fns wired to:
//   POST /admin/users/:id/lock            → lock_user_account
//   POST /admin/users/:id/unlock          → unlock_user_account
//   POST /admin/users/:id/revoke-sessions → admin_revoke_sessions
//
// All revocation goes through `auth::invalidate_sessions(...)` per
// Doctrine 22. Manual lock revokes every session
// (`AdministrativeRevoke`); unlock does NOT touch sessions (the
// lock-time revocation already cleared them); admin-revoke-sessions
// is a sibling of lock that only revokes (no `locked_until` write).

/// Lock-duration choice. Locked-decision presets per
/// `DESIGN_R2_ORGANISATIONAL.md` §12 (15min / 1h / 24h / 7d /
/// indefinite + freeform-minutes). The handler maps the form's
/// radio + minutes input to one of these variants.
///
/// `Indefinite` is encoded as a far-future timestamp (year 9999)
/// rather than NULL so the partial index
/// `rustio_users_locked_until_idx` continues to find it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockDuration {
    FifteenMinutes,
    OneHour,
    TwentyFourHours,
    SevenDays,
    Indefinite,
    /// Freeform minutes. Negative or zero collapses to a past
    /// timestamp at the runtime — effectively a no-op (the
    /// account is never locked); the handler validates `>= 1`
    /// before constructing this variant.
    Minutes(u32),
}

impl LockDuration {
    /// Compute the absolute `locked_until` timestamp for this
    /// duration. `Indefinite` returns a year-9999 instant per
    /// the schema doctrine.
    pub fn to_locked_until(self, now: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::FifteenMinutes => now + ChronoDuration::minutes(15),
            Self::OneHour => now + ChronoDuration::hours(1),
            Self::TwentyFourHours => now + ChronoDuration::hours(24),
            Self::SevenDays => now + ChronoDuration::days(7),
            Self::Indefinite => DateTime::parse_from_rfc3339("9999-12-31T23:59:59Z")
                .expect("year-9999 sentinel parses")
                .with_timezone(&Utc),
            Self::Minutes(m) => now + ChronoDuration::minutes(i64::from(m)),
        }
    }
}

/// Outcome of [`lock_user_account`].
#[allow(dead_code)] // variant fields consumed by R2 commit #16 handlers
#[derive(Debug, Clone)]
pub enum LockOutcome {
    /// Lock applied. `until` is the absolute UTC instant the lock
    /// expires (year 9999 for indefinite). `revoked_session_count`
    /// is how many active sessions were revoked at lock time.
    Locked {
        target_user_id: i64,
        until: DateTime<Utc>,
        revoked_session_count: usize,
    },
    UnknownTarget,
}

/// Outcome of [`unlock_user_account`].
#[allow(dead_code)] // variant fields consumed by R2 commit #16 handlers
#[derive(Debug, Clone)]
pub enum UnlockOutcome {
    /// Lock cleared (or row was already unlocked — same UPDATE,
    /// same audit row, same response shape).
    Unlocked {
        target_user_id: i64,
    },
    UnknownTarget,
}

/// Outcome of [`admin_revoke_sessions`].
#[allow(dead_code)] // variant fields consumed by R2 commit #16 handlers
#[derive(Debug, Clone)]
pub enum AdminRevokeOutcome {
    Revoked {
        target_user_id: i64,
        revoked_session_count: usize,
    },
    UnknownTarget,
}

/// Apply a manual lock to a target user. Sets `locked_until`,
/// revokes every session via `invalidate_sessions(..,
/// AdministrativeRevoke)`, emits per-revoked-session
/// `SessionsRevokedByOther` rows, and the headline
/// `AuditEvent::AccountLocked` row.
///
/// Doctrine 22: revocation goes through
/// `auth::invalidate_sessions`. This fn never writes
/// `revoked_at` directly.
pub async fn lock_user_account(
    db: &Db,
    request: &Request,
    target_user_id: i64,
    actor: AdminActor<'_>,
    duration: LockDuration,
    reason: &str,
    correlation_id: Option<&str>,
) -> Result<LockOutcome> {
    if load_admin_reset_target(db, target_user_id).await?.is_none() {
        return Ok(LockOutcome::UnknownTarget);
    }

    let until = duration.to_locked_until(Utc::now());
    sqlx::query("UPDATE rustio_users SET locked_until = $1 WHERE id = $2")
        .bind(until)
        .bind(target_user_id)
        .execute(db.pool())
        .await?;

    let outcome = invalidate_sessions(
        db,
        SessionTarget::User {
            user_id: target_user_id,
        },
        SessionInvalidationReason::AdministrativeRevoke,
    )
    .await?;
    let revoked_count = outcome.revoked_session_ids.len();

    let ip = client_ip(request);

    // Per-revoked-session audit rows.
    for revoked_id in &outcome.revoked_session_ids {
        let metadata = serde_json::json!({
            "session_id": revoked_id,
            "reason": reason,
            "via": "manual",
        });
        let mut entry = LogEntry::new(target_user_id, ActionType::Update, "users", target_user_id)
            .with_event(AuditEvent::SessionsRevokedByOther)
            .with_actor(actor.user_id);
        entry.correlation_id = correlation_id;
        entry.ip_address = ip.as_deref();
        entry.metadata = Some(metadata);
        entry.summary = format!("session {revoked_id} revoked by admin (manual lock)");
        if let Err(e) = audit_record(db, entry).await {
            log::error!(
                target: "rustio_admin::recovery_admin::lock",
                "audit::record (SessionsRevokedByOther) failed target_user_id={} \
                 session_id={}: {}",
                target_user_id, revoked_id, e,
            );
        }
    }

    // Headline AccountLocked row.
    let metadata = serde_json::json!({
        "actor_email_hash": actor_email_fingerprint(actor.email),
        "reason": reason,
        "until": until,
        "via": "manual",
    });
    let mut entry = LogEntry::new(target_user_id, ActionType::Update, "users", target_user_id)
        .with_event(AuditEvent::AccountLocked)
        .with_actor(actor.user_id);
    entry.correlation_id = correlation_id;
    entry.ip_address = ip.as_deref();
    entry.metadata = Some(metadata);
    entry.summary = format!(
        "manual account lock until {}; {revoked_count} session(s) revoked",
        until.to_rfc3339()
    );
    audit_record(db, entry).await?;

    Ok(LockOutcome::Locked {
        target_user_id,
        until,
        revoked_session_count: revoked_count,
    })
}

/// Clear a manual lock. Sets `locked_until = NULL` and zeroes the
/// `failed_login_count` so the auto-throttle window resets. Does
/// NOT touch sessions — the lock-time revocation already cleared
/// them, and a fresh sign-in mints a new session anyway.
///
/// Idempotent on its UPDATE: re-unlocking an already-unlocked row
/// is a no-op at the row level. The audit row still emits, which
/// matters for forensic-trace completeness when an admin re-runs
/// unlock after losing the response.
pub async fn unlock_user_account(
    db: &Db,
    request: &Request,
    target_user_id: i64,
    actor: AdminActor<'_>,
    reason: &str,
    correlation_id: Option<&str>,
) -> Result<UnlockOutcome> {
    if load_admin_reset_target(db, target_user_id).await?.is_none() {
        return Ok(UnlockOutcome::UnknownTarget);
    }

    sqlx::query(
        "UPDATE rustio_users SET locked_until = NULL, failed_login_count = 0 WHERE id = $1",
    )
    .bind(target_user_id)
    .execute(db.pool())
    .await?;

    let metadata = serde_json::json!({
        "actor_email_hash": actor_email_fingerprint(actor.email),
        "reason": reason,
        "via": "manual",
    });
    let ip = client_ip(request);
    let mut entry = LogEntry::new(target_user_id, ActionType::Update, "users", target_user_id)
        .with_event(AuditEvent::AccountUnlocked)
        .with_actor(actor.user_id);
    entry.correlation_id = correlation_id;
    entry.ip_address = ip.as_deref();
    entry.metadata = Some(metadata);
    entry.summary = "manual account unlock; throttle counter cleared".to_string();
    audit_record(db, entry).await?;

    Ok(UnlockOutcome::Unlocked { target_user_id })
}

/// Revoke every active session for a target user. Sibling of
/// [`lock_user_account`] but without the `locked_until` write —
/// the user can still sign back in immediately on a fresh
/// session. Useful when a security incident calls for "kick
/// them out of every device" without rate-limiting future logins.
///
/// Doctrine 22: revocation goes through
/// `auth::invalidate_sessions`. No `AccountLocked` audit row
/// emits because no lock was applied; only the per-revoked-session
/// `SessionsRevokedByOther` rows.
pub async fn admin_revoke_sessions(
    db: &Db,
    request: &Request,
    target_user_id: i64,
    actor: AdminActor<'_>,
    reason: &str,
    correlation_id: Option<&str>,
) -> Result<AdminRevokeOutcome> {
    if load_admin_reset_target(db, target_user_id).await?.is_none() {
        return Ok(AdminRevokeOutcome::UnknownTarget);
    }

    let outcome = invalidate_sessions(
        db,
        SessionTarget::User {
            user_id: target_user_id,
        },
        SessionInvalidationReason::AdministrativeRevoke,
    )
    .await?;
    let revoked_count = outcome.revoked_session_ids.len();

    let ip = client_ip(request);
    for revoked_id in &outcome.revoked_session_ids {
        let metadata = serde_json::json!({
            "session_id": revoked_id,
            "reason": reason,
            "via": "manual",
        });
        let mut entry = LogEntry::new(target_user_id, ActionType::Update, "users", target_user_id)
            .with_event(AuditEvent::SessionsRevokedByOther)
            .with_actor(actor.user_id);
        entry.correlation_id = correlation_id;
        entry.ip_address = ip.as_deref();
        entry.metadata = Some(metadata);
        entry.summary = format!("session {revoked_id} revoked by admin (manual revoke)");
        if let Err(e) = audit_record(db, entry).await {
            log::error!(
                target: "rustio_admin::recovery_admin::revoke",
                "audit::record (SessionsRevokedByOther) failed target_user_id={} \
                 session_id={}: {}",
                target_user_id, revoked_id, e,
            );
        }
    }

    Ok(AdminRevokeOutcome::Revoked {
        target_user_id,
        revoked_session_count: revoked_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure-data property: ThrottleOutcome variants are
    /// distinguishable. The exhaustive match documents the contract
    /// the do_login caller is expected to follow.
    #[test]
    fn throttle_outcome_variants_are_distinct() {
        let now = Utc::now();
        let recorded = ThrottleOutcome::Recorded { count: 3 };
        let just_locked = ThrottleOutcome::JustLocked {
            count: 5,
            until: now,
        };
        let disabled = ThrottleOutcome::Disabled { count: 7 };
        assert_ne!(recorded, just_locked);
        assert_ne!(just_locked, disabled);
        assert_ne!(recorded, disabled);

        // Exhaustive match shape — caller's branching contract.
        for o in [recorded, just_locked, disabled] {
            match o {
                ThrottleOutcome::Recorded { count } => assert!(count >= 0),
                ThrottleOutcome::JustLocked { count, until: _ } => {
                    assert!(count > 0)
                }
                ThrottleOutcome::Disabled { count } => assert!(count >= 0),
            }
        }
    }

    #[test]
    fn lock_state_variants_round_trip() {
        let now = Utc::now();
        let unlocked = LockState::Unlocked;
        let locked = LockState::Locked { until: now };
        assert_ne!(unlocked, locked);
        // Copy + Eq compile-time guarantees.
        fn assert_traits<T: Copy + Eq + std::fmt::Debug>() {}
        assert_traits::<LockState>();
        assert_traits::<ThrottleOutcome>();
    }

    // ---- Admin reset helpers (R2 commit #14) ------------------------------

    #[test]
    fn random_temp_password_is_sixteen_chars() {
        // Locked-decision per DESIGN_R2_ORGANISATIONAL.md §12.
        for _ in 0..32 {
            let pw = random_temp_password();
            assert_eq!(pw.len(), 16, "temp pw {pw:?} is not 16 chars");
        }
    }

    #[test]
    fn random_temp_password_is_url_safe_base64() {
        // URL-safe-no-pad alphabet: A-Z, a-z, 0-9, -, _.
        for _ in 0..32 {
            let pw = random_temp_password();
            assert!(
                pw.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "temp pw {pw:?} has non-url-safe-base64 chars"
            );
        }
    }

    #[test]
    fn random_temp_password_distinct_across_calls() {
        // Not a strict guarantee (collisions possible at 96 bits)
        // but vanishingly unlikely; this catches a totally-broken
        // RNG / constant return.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..64 {
            let pw = random_temp_password();
            assert!(seen.insert(pw), "duplicate temp pw within a small batch");
        }
    }

    #[test]
    fn actor_email_fingerprint_is_eight_chars() {
        let h = actor_email_fingerprint("admin@example.com");
        assert_eq!(h.len(), 8);
    }

    #[test]
    fn actor_email_fingerprint_is_deterministic() {
        let a = actor_email_fingerprint("admin@example.com");
        let b = actor_email_fingerprint("admin@example.com");
        assert_eq!(a, b);
    }

    #[test]
    fn actor_email_fingerprint_differs_for_different_emails() {
        let a = actor_email_fingerprint("admin@example.com");
        let b = actor_email_fingerprint("user@example.com");
        let c = actor_email_fingerprint("admin@example.org");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
    }

    #[test]
    fn actor_email_fingerprint_does_not_leak_plaintext() {
        // The hash must not contain any substring of the input.
        // SHA-256 makes this overwhelmingly true; this test pins
        // the property so a future refactor (e.g. to a different
        // "hash" that's actually base64) can't quietly leak.
        for input in [
            "admin@example.com",
            "operator@bosphorus-sham.local",
            "ALICE@WORK.IO",
        ] {
            let h = actor_email_fingerprint(input);
            for window_size in 3..=input.len() {
                for window in input.as_bytes().windows(window_size) {
                    let needle = std::str::from_utf8(window).unwrap_or("");
                    if needle.is_empty() {
                        continue;
                    }
                    assert!(
                        !h.contains(needle),
                        "fingerprint {h:?} leaks substring {needle:?} of input {input:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn admin_outcome_types_are_send() {
        // Compile-time guarantee that the outcomes flow through
        // async handler boundaries without footguns.
        fn assert_send<T: Send>() {}
        assert_send::<AdminIssueOutcome>();
        assert_send::<AdminTempPwOutcome>();
        assert_send::<LockOutcome>();
        assert_send::<UnlockOutcome>();
        assert_send::<AdminRevokeOutcome>();
    }

    // ---- LockDuration (R2 commit #16) -------------------------------------

    #[test]
    fn lock_duration_presets_are_relative_to_now() {
        // Use a fixed `now` so the test is deterministic. Each
        // preset shifts `now` by its labelled offset.
        let now = DateTime::parse_from_rfc3339("2026-05-10T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            LockDuration::FifteenMinutes.to_locked_until(now),
            now + ChronoDuration::minutes(15)
        );
        assert_eq!(
            LockDuration::OneHour.to_locked_until(now),
            now + ChronoDuration::hours(1)
        );
        assert_eq!(
            LockDuration::TwentyFourHours.to_locked_until(now),
            now + ChronoDuration::hours(24)
        );
        assert_eq!(
            LockDuration::SevenDays.to_locked_until(now),
            now + ChronoDuration::days(7)
        );
        assert_eq!(
            LockDuration::Minutes(30).to_locked_until(now),
            now + ChronoDuration::minutes(30)
        );
    }

    #[test]
    fn lock_duration_indefinite_is_year_9999() {
        // Locked: indefinite encodes as a far-future timestamp,
        // not NULL, so `rustio_users_locked_until_idx` continues
        // to find it.
        let now = Utc::now();
        let until = LockDuration::Indefinite.to_locked_until(now);
        assert_eq!(until.format("%Y").to_string(), "9999");
        // Sanity: well after any plausible operational deadline.
        assert!(until > now + ChronoDuration::days(365 * 100));
    }

    #[test]
    fn lock_duration_minutes_zero_is_a_past_no_op() {
        // `Minutes(0)` is documented as a no-op (the resulting
        // `locked_until` lands at-or-before NOW). The handler
        // validates `>= 1` before constructing this variant; the
        // runtime contract is just that it doesn't panic.
        let now = Utc::now();
        let until = LockDuration::Minutes(0).to_locked_until(now);
        assert_eq!(until, now);
    }
}
