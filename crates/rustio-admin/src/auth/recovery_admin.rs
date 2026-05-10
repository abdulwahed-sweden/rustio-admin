//! Organisational recovery (R2).
//!
//! See `DESIGN_R2_ORGANISATIONAL.md` for the canonical contract this
//! module implements. R2 ships in 0.6.0; the admin-driven flows
//! (`issue_admin_reset_token`, `admin_set_temp_password`,
//! `lock_user_account`, `unlock_user_account`,
//! `admin_revoke_sessions`), the re-auth wall runtime, the handlers,
//! the templates, and the testcontainers integration test harness
//! land in subsequent atomic commits per
//! `DESIGN_R2_ORGANISATIONAL.md` §11.
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

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sqlx::Row as _;

use crate::auth::recovery::LoginThrottle;
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
pub(crate) enum LockState {
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
pub(crate) enum ThrottleOutcome {
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
pub(crate) async fn check_account_lockout(db: &Db, user_id: i64) -> Result<LockState> {
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
pub(crate) async fn record_failed_login(
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
pub(crate) async fn record_successful_login(db: &Db, user_id: i64) -> Result<()> {
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
#[allow(dead_code)] // call site lands in R2 commit #11 (re-auth handler)
pub(crate) async fn promote_session_elevated(
    db: &Db,
    session_id: i64,
    ttl: ChronoDuration,
) -> Result<()> {
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
#[allow(dead_code)] // call sites land in R2 commits #15 / #16 (admin recovery handlers)
pub(crate) async fn check_session_elevated(db: &Db, session_id: i64) -> Result<bool> {
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
}
