//! DB-backed sessions.
//!
//! See `DESIGN_SESSIONS.md` for the canonical lifecycle, trust-level
//! model, and invalidation reasons. Briefly:
//!
//! - A **session** is a device/browser context with a stable
//!   [`SessionId`], a current [`SessionTrust`], and an issuance chain
//!   tracked through `parent_session_id`.
//! - Cookie tokens are sha-256-hashed at rest in `token_hash`; the
//!   plaintext only exists in the user's cookie.
//! - Trust escalation rotates the cookie (mints a new row, sets the
//!   parent's `revoked_at` with reason `trust_escalation`).
//! - All revocations go through [`invalidate_sessions`] — no other
//!   code path writes `revoked_at`. Grep for `revoked_at` to verify.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::orm::{Db, Row};

use super::role::Role;
use super::users::Identity;

/// The cookie name we look for and set. Constant so middleware and
/// handlers stay in sync.
pub const SESSION_COOKIE: &str = "rustio_session";

const SESSION_LENGTH_DAYS: i64 = 14;

/// Trust level a session has acquired. The login flow mints
/// [`SessionTrust::Authenticated`]; the future re-auth wall promotes
/// to [`SessionTrust::Elevated`]; a successful TOTP step on this
/// session lifts to [`SessionTrust::MfaVerified`].
///
/// The variants are ordered: `Authenticated < Elevated <
/// MfaVerified`. Compare via [`SessionTrust::satisfies`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTrust {
    Authenticated,
    Elevated,
    MfaVerified,
}

impl SessionTrust {
    /// Stable lowercase identifier matching the SQL `trust_level`
    /// column's CHECK constraint.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authenticated => "authenticated",
            Self::Elevated => "elevated",
            Self::MfaVerified => "mfa_verified",
        }
    }

    /// Numeric ladder for partial-order comparisons.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Authenticated => 1,
            Self::Elevated => 2,
            Self::MfaVerified => 3,
        }
    }

    /// `self` is at least as trusted as `other`.
    pub const fn satisfies(self, other: SessionTrust) -> bool {
        self.rank() >= other.rank()
    }

    /// Parse from the SQL `trust_level` column. Defaults to
    /// `Authenticated` on unknown input so a malformed migration
    /// can't lock anyone out.
    pub fn parse(s: &str) -> Self {
        match s {
            "elevated" => Self::Elevated,
            "mfa_verified" => Self::MfaVerified,
            _ => Self::Authenticated,
        }
    }
}

/// Why a session is being invalidated. Drives both the audit
/// `action_type` and decisions about whether to clear remembered MFA
/// or mint a replacement session.
///
/// All [`invalidate_sessions`] callers pass one of these — the engine
/// is the single writer of `revoked_at`. Free-form reasons are not
/// allowed; doctrine 22 ("centralized invalidation") in
/// `DESIGN_SYSTEM.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionInvalidationReason {
    Logout,
    Expired,
    UserRequested,
    AdministrativeRevoke,
    PasswordReset,
    PasswordResetByOther,
    MfaEnabled,
    MfaDisabled,
    MfaDisabledByOther,
    AuthorityEscalation,
    EmergencyRecovery,
    /// Token rotation that accompanies a trust escalation
    /// (`Authenticated → Elevated`, etc.). The replacement session is
    /// minted as the parent's child; this revokes the old token.
    TrustEscalation,
}

impl SessionInvalidationReason {
    /// Stable lowercase identifier persisted in
    /// `rustio_sessions.revoked_reason` and used as the audit
    /// `action_type` suffix.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Logout => "logout",
            Self::Expired => "expired",
            Self::UserRequested => "user_requested",
            Self::AdministrativeRevoke => "administrative_revoke",
            Self::PasswordReset => "password_reset",
            Self::PasswordResetByOther => "password_reset_by_other",
            Self::MfaEnabled => "mfa_enabled",
            Self::MfaDisabled => "mfa_disabled",
            Self::MfaDisabledByOther => "mfa_disabled_by_other",
            Self::AuthorityEscalation => "authority_escalation",
            Self::EmergencyRecovery => "emergency_recovery",
            Self::TrustEscalation => "trust_escalation",
        }
    }
}

/// Which sessions an [`invalidate_sessions`] call targets.
#[derive(Debug, Clone, Copy)]
pub enum SessionTarget {
    /// Every active session belonging to `user_id`.
    User { user_id: i64 },
    /// Every active session belonging to `user_id` except the one
    /// identified by `current_session_id`. Used by "log me out
    /// everywhere else" and by post-password-reset flows that want to
    /// keep the current device alive.
    UserExceptCurrent {
        user_id: i64,
        current_session_id: i64,
    },
    /// One specific session row.
    Single { session_id: i64 },
}

/// One session row, reconstructed from `rustio_sessions`. Returned
/// by [`list_active_for_user`] for the active-sessions UI.
#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub session_id: i64,
    pub user_id: i64,
    pub trust_level: SessionTrust,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub elevated_until: Option<DateTime<Utc>>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

/// Outcome of an [`invalidate_sessions`] call. Used by the audit
/// pipeline to write one row per affected session and by the caller
/// to decide whether to clear the user's cookie.
#[derive(Debug, Clone, Default)]
pub struct InvalidationOutcome {
    /// `session_id`s that were transitioned from active to revoked.
    pub revoked_session_ids: Vec<i64>,
    /// Reason recorded for the audit pipeline.
    pub reason: Option<SessionInvalidationReason>,
}

pub async fn init_session_tables(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_sessions (
            token      TEXT PRIMARY KEY,
            user_id    BIGINT NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
            expires_at TIMESTAMPTZ NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_seen  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS rustio_sessions_user_idx ON rustio_sessions (user_id)")
        .execute(db.pool())
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_sessions_expires_idx ON rustio_sessions (expires_at)",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

/// Additive schema upgrade for session-level metadata (ip, user_agent).
/// Idempotent; safe to call on every boot. Reads are consumed by the
/// built-in user profile page; the auth path itself never reads these.
pub(crate) async fn migrate_session_schema(db: &Db) -> Result<()> {
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS ip TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS user_agent TEXT")
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Additive lifecycle migration introduced in 0.4.0 (`feat/session-token-hashing`).
/// Adds:
///
/// - `session_id` — a stable BIGINT identifier separate from the
///   token, so trust escalation can rotate the cookie without losing
///   the session's identity. Backed by a sequence; existing rows are
///   assigned ids on the ALTER.
/// - `token_hash` — sha256 of the cookie token, URL-safe base64.
///   Reads will prefer this over the plaintext `token` PK during a
///   14-day transition window; new sessions populate it at insert.
/// - `device_id` — nullable, reserved for future device-recognition
///   work. R0 leaves it empty.
/// - `trust_level` — `authenticated | elevated | mfa_verified`.
///   Defaults to `authenticated` for existing rows.
/// - `elevated_until` — re-auth wall expiry; populated by the future
///   `/admin/reauth` endpoint.
/// - `parent_session_id` — lineage anchor for trust-escalation
///   rotation; future invalidations use it to revoke ancestor
///   sessions when a child elevates.
/// - `revoked_at` / `revoked_reason` — soft-delete with a typed
///   reason. Replaces the old DELETE-on-logout flow (the row stays
///   for audit retention until `purge_expired_sessions` reaps it).
///
/// Idempotent; safe to call on every boot.
pub(crate) async fn migrate_session_lifecycle(db: &Db) -> Result<()> {
    sqlx::query("CREATE SEQUENCE IF NOT EXISTS rustio_sessions_session_id_seq")
        .execute(db.pool())
        .await?;
    sqlx::query(
        "ALTER TABLE rustio_sessions \
         ADD COLUMN IF NOT EXISTS session_id BIGINT NOT NULL DEFAULT \
             nextval('rustio_sessions_session_id_seq')",
    )
    .execute(db.pool())
    .await?;
    sqlx::query(
        "ALTER SEQUENCE rustio_sessions_session_id_seq OWNED BY rustio_sessions.session_id",
    )
    .execute(db.pool())
    .await?;
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS token_hash TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS device_id TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query(
        "ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS trust_level TEXT \
         NOT NULL DEFAULT 'authenticated'",
    )
    .execute(db.pool())
    .await?;
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS elevated_until TIMESTAMPTZ")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS parent_session_id BIGINT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_sessions ADD COLUMN IF NOT EXISTS revoked_reason TEXT")
        .execute(db.pool())
        .await?;

    // CHECK constraint guarded via pg_constraint, since `IF NOT EXISTS`
    // doesn't apply to constraints.
    sqlx::query(
        "DO $$ BEGIN \
            IF NOT EXISTS ( \
                SELECT 1 FROM pg_constraint \
                WHERE conname = 'rustio_sessions_trust_level_check' \
            ) THEN \
                ALTER TABLE rustio_sessions \
                ADD CONSTRAINT rustio_sessions_trust_level_check \
                CHECK (trust_level IN ('authenticated', 'elevated', 'mfa_verified')); \
            END IF; \
        END $$",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS rustio_sessions_session_id_uq \
         ON rustio_sessions (session_id)",
    )
    .execute(db.pool())
    .await?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS rustio_sessions_token_hash_uq \
         ON rustio_sessions (token_hash) \
         WHERE revoked_at IS NULL AND token_hash IS NOT NULL",
    )
    .execute(db.pool())
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_sessions_user_active_idx \
         ON rustio_sessions (user_id) WHERE revoked_at IS NULL",
    )
    .execute(db.pool())
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_sessions_parent_idx \
         ON rustio_sessions (parent_session_id) WHERE parent_session_id IS NOT NULL",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

pub async fn create_session(db: &Db, user_id: i64) -> Result<String> {
    let token = random_token();
    let token_hash = hash_token_for_storage(&token);
    let expires = Utc::now() + Duration::days(SESSION_LENGTH_DAYS);
    // Both `token` (PRIMARY KEY) and `token_hash` are stored. The
    // plaintext column is preserved so the 0.3.x fallback read path
    // keeps working for sessions created before this commit; new
    // sessions write both values so a future migration can drop the
    // plaintext column without a data backfill.
    sqlx::query(
        "INSERT INTO rustio_sessions (token, token_hash, user_id, expires_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&token)
    .bind(&token_hash)
    .bind(user_id)
    .bind(expires)
    .execute(db.pool())
    .await?;
    Ok(token)
}

/// Hard-delete a session row by cookie token. Retained as a
/// pre-0.4.0 compatibility shim — internal callers are migrating to
/// [`invalidate_sessions`], which soft-revokes via `revoked_at` and
/// keeps the row available for the audit trail. New code MUST NOT
/// call this directly; only the expired-row sweeper and the read-path
/// stale-cleanup branch are allowed callers, both of which are
/// inside this module.
pub async fn delete_session(db: &Db, token: &str) -> Result<()> {
    sqlx::query("DELETE FROM rustio_sessions WHERE token = $1 OR token_hash = $2")
        .bind(token)
        .bind(hash_token_for_storage(token))
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Centralised session invalidation — the single legitimate writer of
/// `rustio_sessions.revoked_at`.
///
/// Doctrine 22 (centralized invalidation) makes every revoke decision
/// pass through here. Handlers MUST NOT issue raw `UPDATE … SET
/// revoked_at` statements; a grep for that string in the source tree
/// must return only this module. PR review enforces it.
///
/// What this function does:
///
/// - Resolves the [`SessionTarget`] into the set of session ids that
///   are currently active and match.
/// - Marks each row `revoked_at = NOW()` and `revoked_reason =
///   reason.as_str()`.
/// - Returns the affected ids in the [`InvalidationOutcome`] so the
///   caller can write one audit row per revoked session, all sharing
///   the supplied `correlation_id`.
///
/// Audit row writes are the caller's job (the audit module owns the
/// `rustio_admin_actions` table; sessions own `rustio_sessions`). The
/// reason is returned so the caller can render a typed `action_type`
/// without re-deriving it.
pub async fn invalidate_sessions(
    db: &Db,
    target: SessionTarget,
    reason: SessionInvalidationReason,
) -> Result<InvalidationOutcome> {
    let reason_str = reason.as_str();
    let revoked_ids: Vec<i64> = match target {
        SessionTarget::User { user_id } => {
            sqlx::query_scalar::<_, i64>(
                "UPDATE rustio_sessions \
                SET revoked_at = NOW(), revoked_reason = $2 \
              WHERE user_id = $1 AND revoked_at IS NULL \
            RETURNING session_id",
            )
            .bind(user_id)
            .bind(reason_str)
            .fetch_all(db.pool())
            .await?
        }
        SessionTarget::UserExceptCurrent {
            user_id,
            current_session_id,
        } => {
            sqlx::query_scalar::<_, i64>(
                "UPDATE rustio_sessions \
                SET revoked_at = NOW(), revoked_reason = $3 \
              WHERE user_id = $1 AND session_id <> $2 AND revoked_at IS NULL \
            RETURNING session_id",
            )
            .bind(user_id)
            .bind(current_session_id)
            .bind(reason_str)
            .fetch_all(db.pool())
            .await?
        }
        SessionTarget::Single { session_id } => {
            sqlx::query_scalar::<_, i64>(
                "UPDATE rustio_sessions \
                SET revoked_at = NOW(), revoked_reason = $2 \
              WHERE session_id = $1 AND revoked_at IS NULL \
            RETURNING session_id",
            )
            .bind(session_id)
            .bind(reason_str)
            .fetch_all(db.pool())
            .await?
        }
    };

    Ok(InvalidationOutcome {
        revoked_session_ids: revoked_ids,
        reason: Some(reason),
    })
}

/// Convenience wrapper for the existing logout flow. Routes through
/// [`invalidate_sessions`] with `SessionTarget::Single` and
/// `SessionInvalidationReason::Logout`.
///
/// Looks up the session by the cookie token (fast path: token_hash;
/// fallback: plaintext for legacy 0.3.x sessions). Returns `Ok(())`
/// even when no row matches — logout is idempotent.
pub async fn logout_session(db: &Db, token: &str) -> Result<()> {
    let token_hash = hash_token_for_storage(token);
    let session_id: Option<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT session_id FROM rustio_sessions \
          WHERE (token_hash = $1 OR (token_hash IS NULL AND token = $2)) \
            AND revoked_at IS NULL \
          LIMIT 1",
    )
    .bind(&token_hash)
    .bind(token)
    .fetch_optional(db.pool())
    .await?;

    if let Some(sid) = session_id {
        invalidate_sessions(
            db,
            SessionTarget::Single { session_id: sid },
            SessionInvalidationReason::Logout,
        )
        .await?;
    }
    Ok(())
}

/// List a user's currently-active sessions, ordered by `last_seen`
/// descending so the active-sessions UI surfaces the most recently
/// used row first. Excludes revoked + expired rows.
pub async fn list_active_for_user(db: &Db, user_id: i64) -> Result<Vec<Session>> {
    let rows = sqlx::query(
        "SELECT session_id, user_id, trust_level, created_at, last_seen, expires_at, \
                elevated_until, ip, user_agent \
           FROM rustio_sessions \
          WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW() \
          ORDER BY last_seen DESC",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;

    rows.iter()
        .map(|r| {
            let r = Row::from_pg(r);
            Ok(Session {
                session_id: r.get_i64("session_id")?,
                user_id: r.get_i64("user_id")?,
                trust_level: SessionTrust::parse(&r.get_string("trust_level")?),
                created_at: r.get_datetime("created_at")?,
                last_seen: r.get_datetime("last_seen")?,
                expires_at: r.get_datetime("expires_at")?,
                elevated_until: None, // optional column; reader lands when re-auth wall ships
                ip: r.get_optional_string("ip")?,
                user_agent: r.get_optional_string("user_agent")?,
            })
        })
        .collect()
}

/// Resolve the cookie token to its `session_id` (active sessions
/// only). Used by the active-sessions UI to mark which row is the
/// current device, and by `UserExceptCurrent` callers.
pub async fn current_session_id(db: &Db, token: &str) -> Result<Option<i64>> {
    let token_hash = hash_token_for_storage(token);
    let id: Option<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT session_id FROM rustio_sessions \
          WHERE (token_hash = $1 OR (token_hash IS NULL AND token = $2)) \
            AND revoked_at IS NULL AND expires_at > NOW() \
          LIMIT 1",
    )
    .bind(&token_hash)
    .bind(token)
    .fetch_optional(db.pool())
    .await?;
    Ok(id)
}

pub async fn identity_from_session(db: &Db, token: &str) -> Result<Option<Identity>> {
    // Fast path: lookup by sha-256 of the cookie token. Every session
    // created in 0.4.0+ has `token_hash` populated, and the unique
    // partial index `rustio_sessions_token_hash_uq` makes this an
    // index seek. Revoked sessions (`revoked_at IS NOT NULL`) are
    // excluded so a logged-out cookie never re-authenticates.
    let token_hash = hash_token_for_storage(token);
    let row = sqlx::query(
        "SELECT u.id, u.email, u.role, u.is_active, u.is_demo, u.demo_label, \
                u.must_change_password, \
                s.expires_at, s.token_hash IS NOT NULL AS hashed \
           FROM rustio_sessions s \
           JOIN rustio_users u ON u.id = s.user_id \
          WHERE s.token_hash = $1 AND s.revoked_at IS NULL",
    )
    .bind(&token_hash)
    .fetch_optional(db.pool())
    .await?;

    let row = match row {
        Some(r) => Some(r),
        // Slow path / transition fallback: pre-0.4.0 sessions have
        // NULL `token_hash` and were keyed by plaintext `token` PK.
        // Look those up so existing logged-in users aren't kicked out
        // when 0.4.0 deploys. The fallback can be removed in a follow-
        // up release once SESSION_LENGTH_DAYS (14d) has elapsed since
        // 0.4.0 publish — every legacy session will have expired by
        // then.
        None => {
            sqlx::query(
                "SELECT u.id, u.email, u.role, u.is_active, u.is_demo, u.demo_label, \
                    u.must_change_password, \
                    s.expires_at, FALSE AS hashed \
               FROM rustio_sessions s \
               JOIN rustio_users u ON u.id = s.user_id \
              WHERE s.token = $1 AND s.token_hash IS NULL AND s.revoked_at IS NULL",
            )
            .bind(token)
            .fetch_optional(db.pool())
            .await?
        }
    };
    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };
    let r = Row::from_pg(&row);
    let expires_at = r.get_datetime("expires_at")?;
    if expires_at < Utc::now() {
        // Don't bother keeping the stale row around. Fire-and-forget;
        // the central invalidate_sessions API lands in the next
        // commit and replaces this DELETE with a soft revoke. Until
        // then a hard delete is consistent with prior behavior for
        // expired rows (purge_expired_sessions also DELETEs).
        let _ = delete_session(db, token).await;
        return Ok(None);
    }

    // Touch last_seen without holding the request back. Updates by
    // token_hash on the fast path, falls back to token for legacy
    // sessions so the activity timestamp lands on the right row.
    let db_clone = db.clone();
    let token_owned = token.to_string();
    let token_hash_owned = token_hash.clone();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "UPDATE rustio_sessions SET last_seen = NOW() \
              WHERE (token_hash = $1 OR (token_hash IS NULL AND token = $2)) \
                AND revoked_at IS NULL",
        )
        .bind(&token_hash_owned)
        .bind(&token_owned)
        .execute(db_clone.pool())
        .await;
    });

    Ok(Some(Identity {
        user_id: r.get_i64("id")?,
        email: r.get_string("email")?,
        role: Role::parse(&r.get_string("role")?)?,
        is_active: r.get_bool("is_active")?,
        is_demo: r.get_bool("is_demo")?,
        demo_label: r.get_optional_string("demo_label")?,
        must_change_password: r.get_bool("must_change_password")?,
    }))
}

/// Delete all expired sessions. Intended to be called periodically
/// from a background task (see `background::spawn_session_sweeper`).
pub async fn purge_expired_sessions(db: &Db) -> Result<u64> {
    let result = sqlx::query("DELETE FROM rustio_sessions WHERE expires_at < NOW()")
        .execute(db.pool())
        .await?;
    Ok(result.rows_affected())
}

pub fn session_token_from_cookie(cookie_header: &str) -> Option<String> {
    let prefix = format!("{SESSION_COOKIE}=");
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix(&prefix) {
            return Some(v.to_string());
        }
    }
    None
}

/// Generate a 256-bit cryptographically-random URL-safe-base64
/// token. Shared between session cookies and password-reset tokens
/// (R1) — both want the same "random enough that brute force is
/// infeasible regardless of any hash function's work factor"
/// shape. `pub(crate)` so `auth::recovery` can call it without
/// duplicating the helper; not in the public API.
pub(crate) fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash a session-cookie token for at-rest storage in
/// `rustio_sessions.token_hash`. SHA-256 of the URL-safe-base64
/// plaintext, re-encoded as URL-safe-base64 (no padding) so the
/// column accepts ASCII text.
///
/// SHA-256 is the right choice here (not Argon2): the input is a
/// 256-bit random token, so brute force is infeasible regardless of
/// the hash function's work factor; SHA-256 is fast enough to keep
/// the session-lookup path under 1ms even at high RPS. Argon2 would
/// add latency without security benefit for this input distribution.
pub(crate) fn hash_token_for_storage(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_token_from_cookie_header() {
        let h = "foo=bar; rustio_session=abc123; other=x";
        assert_eq!(session_token_from_cookie(h), Some("abc123".into()));
    }

    #[test]
    fn returns_none_when_cookie_missing() {
        let h = "foo=bar; other=x";
        assert!(session_token_from_cookie(h).is_none());
    }

    #[test]
    fn random_token_has_reasonable_entropy() {
        // Rough sanity check — two consecutive tokens should differ.
        assert_ne!(random_token(), random_token());
    }

    #[test]
    fn hash_token_is_deterministic() {
        // Same input → same hash, every call. Required for the
        // identity_from_session lookup to find the row.
        let token = random_token();
        assert_eq!(
            hash_token_for_storage(&token),
            hash_token_for_storage(&token)
        );
    }

    #[test]
    fn hash_token_differs_per_token() {
        // Different inputs → different hashes (collision-resistance is
        // the point).
        let a = hash_token_for_storage("aaaa");
        let b = hash_token_for_storage("aaab");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_token_output_is_url_safe_base64() {
        let h = hash_token_for_storage("anything");
        // 256 bits → 43 url-safe-no-pad base64 chars.
        assert_eq!(h.len(), 43);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn hash_token_does_not_leak_plaintext() {
        // Property check — the hash output should bear no obvious
        // resemblance to the plaintext, including substrings.
        let plaintext = "secret-cookie-value-12345";
        let h = hash_token_for_storage(plaintext);
        assert!(!h.contains("secret"));
        assert!(!h.contains("12345"));
    }

    // ---- typed session model ----

    #[test]
    fn session_trust_orders_correctly() {
        assert!(SessionTrust::Authenticated.rank() < SessionTrust::Elevated.rank());
        assert!(SessionTrust::Elevated.rank() < SessionTrust::MfaVerified.rank());
        assert!(SessionTrust::MfaVerified.satisfies(SessionTrust::Elevated));
        assert!(SessionTrust::MfaVerified.satisfies(SessionTrust::Authenticated));
        assert!(SessionTrust::Authenticated.satisfies(SessionTrust::Authenticated));
        assert!(!SessionTrust::Authenticated.satisfies(SessionTrust::Elevated));
        assert!(!SessionTrust::Elevated.satisfies(SessionTrust::MfaVerified));
    }

    #[test]
    fn session_trust_round_trips_through_sql() {
        for tier in [
            SessionTrust::Authenticated,
            SessionTrust::Elevated,
            SessionTrust::MfaVerified,
        ] {
            assert_eq!(SessionTrust::parse(tier.as_str()), tier);
        }
    }

    #[test]
    fn session_trust_parse_defaults_safely_on_unknown() {
        // Unknown / malformed trust_level column → fall back to the
        // weakest tier so a bad row can't accidentally elevate.
        assert_eq!(SessionTrust::parse("garbage"), SessionTrust::Authenticated);
        assert_eq!(SessionTrust::parse(""), SessionTrust::Authenticated);
    }

    #[test]
    fn invalidation_reason_strings_are_distinct() {
        // Property: as_str() values must be globally unique so audit
        // rows are unambiguous.
        let reasons = [
            SessionInvalidationReason::Logout,
            SessionInvalidationReason::Expired,
            SessionInvalidationReason::UserRequested,
            SessionInvalidationReason::AdministrativeRevoke,
            SessionInvalidationReason::PasswordReset,
            SessionInvalidationReason::PasswordResetByOther,
            SessionInvalidationReason::MfaEnabled,
            SessionInvalidationReason::MfaDisabled,
            SessionInvalidationReason::MfaDisabledByOther,
            SessionInvalidationReason::AuthorityEscalation,
            SessionInvalidationReason::EmergencyRecovery,
            SessionInvalidationReason::TrustEscalation,
        ];
        let mut set = std::collections::HashSet::new();
        for r in reasons {
            assert!(set.insert(r.as_str()), "duplicate as_str() for {r:?}");
        }
        assert_eq!(set.len(), reasons.len());
    }
}
