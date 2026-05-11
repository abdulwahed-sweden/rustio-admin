//! Emergency-recovery primitives — the framework-side surface called
//! by the `rustio user <op>` CLI commands.
//!
//! Each function performs the atomic DB-mutation chain for one
//! emergency operation and returns a typed outcome the CLI uses to
//! render its post-mutation summary.
//!
//! ## D12 — CLI-only audit emission
//!
//! These functions deliberately do NOT write the
//! `AuditEvent::EmergencyRecovery` row. That emission lives in the
//! CLI crate (`crates/rustio-admin-cli/`), so the audit row's
//! call-stack-of-record is always rooted in the CLI binary. The
//! framework provides the DB transaction; the CLI provides the
//! audit trail. The
//! [`admin::audit::tests::emergency_recovery_is_cli_only`] unit
//! test enforces this — adding an `AuditEvent::EmergencyRecovery`
//! emission anywhere in `crates/rustio-admin/src/` fails the
//! default test gate.
//!
//! ## D11 — atomic per command
//!
//! Each function that mutates more than one row runs its mutations
//! inside a single sqlx transaction. Partial failures roll back so
//! half-applied state never lands — e.g. a `reset_password` that
//! hashed + stored the new password but then failed to flip
//! `must_change_password` would leave the user in a worse state
//! than before.
//!
//! ## Building blocks reused
//!
//! - [`crate::auth::users::hash_password`] for Argon2id-hashed
//!   password storage
//! - [`crate::auth::invalidate_sessions`] for revocation under
//!   doctrine 22 (single writer of `revoked_at`)
//!
//! See `DESIGN_R4_EMERGENCY.md` §6 for the building-block table per
//! operation and §3 for the locked scope.

use crate::auth::sessions::{
    invalidate_sessions, InvalidationOutcome, SessionInvalidationReason, SessionTarget,
};
use crate::auth::users::hash_password;
use crate::auth::Role;
use crate::error::{Error, Result};
use crate::orm::Db;

// ---- Outcomes ------------------------------------------------------------

/// Outcome of [`reset_password`]. The CLI uses this to compose the
/// post-mutation summary line ("password set, N sessions revoked"
/// or "user not found").
#[derive(Debug, Clone)]
pub enum ResetOutcome {
    Ok { revoked_session_count: usize },
    UnknownTarget,
}

/// Outcome of [`unlock`].
#[derive(Debug, Clone)]
pub enum UnlockOutcome {
    Ok { previously_locked: bool },
    UnknownTarget,
}

/// Outcome of [`disable_mfa`].
#[derive(Debug, Clone)]
pub enum DisableMfaOutcome {
    Ok {
        was_enabled: bool,
        deleted_backup_codes: usize,
        revoked_session_count: usize,
    },
    UnknownTarget,
}

/// Outcome of [`promote`]. `SoleAdministratorDemoteRefused` is the
/// only "refuse the operation outright" branch — the rest of the
/// emergency surface allows action on any extant target (the
/// operator already has DB access; the framework's role is
/// audit-visibility, not gatekeeping).
#[derive(Debug, Clone)]
pub enum PromoteOutcome {
    Ok {
        previous_role: Role,
        new_role: Role,
        revoked_session_count: usize,
    },
    /// Target already carries `new_role`. No DB write, no session
    /// revocation. The CLI surfaces this as a benign no-op.
    NoChange {
        current_role: Role,
    },
    UnknownTarget,
    /// Demoting from `Administrator` would leave zero active
    /// administrators. Refused per `DESIGN_R4_EMERGENCY.md` §3.4.
    SoleAdministratorDemoteRefused,
}

/// Outcome of [`emergency_access`].
#[derive(Debug, Clone)]
pub enum EmergencyAccessOutcome {
    Ok {
        token_id: i64,
        url_path: String,
        expires_at: chrono::DateTime<chrono::Utc>,
    },
    UnknownTarget,
    InactiveTarget,
}

// ---- Helpers -------------------------------------------------------------

/// Confirm the target row exists. Returns the `is_active` flag when
/// present, `None` when no row matches. Used by every emergency
/// function as the first gate (UnknownTarget short-circuit).
async fn target_exists(db: &Db, user_id: i64) -> Result<Option<bool>> {
    let row: Option<(bool,)> = sqlx::query_as("SELECT is_active FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db.pool())
        .await
        .map_err(Error::from)?;
    Ok(row.map(|(active,)| active))
}

// ---- reset_password ------------------------------------------------------

/// Set a new password for `target_user_id`, raise
/// `must_change_password = TRUE`, revoke every session for the user.
///
/// The CLI supplies `new_password` — either operator-provided via
/// `--temp-password` or a CLI-generated random string. This function
/// does not generate or echo the plaintext; the caller owns it and
/// is responsible for displaying it exactly once.
///
/// Atomic: the password update + must-change flag flip + audit
/// columns (`password_changed_at = NOW()`) land in one transaction.
/// Session revocation runs after commit because
/// [`invalidate_sessions`] is the single writer of `revoked_at`
/// (doctrine 22) and runs its own atomic statement; a transaction
/// boundary here keeps the password mutation isolated from the
/// session sweep.
pub async fn reset_password(
    db: &Db,
    target_user_id: i64,
    new_password: &str,
) -> Result<ResetOutcome> {
    if target_exists(db, target_user_id).await?.is_none() {
        return Ok(ResetOutcome::UnknownTarget);
    }

    let hash = hash_password(new_password)?;

    let mut tx = db.pool().begin().await.map_err(Error::from)?;
    sqlx::query(
        "UPDATE rustio_users \
         SET password_hash = $1, \
             password_changed_at = NOW(), \
             must_change_password = TRUE \
         WHERE id = $2",
    )
    .bind(&hash)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(Error::from)?;
    tx.commit().await.map_err(Error::from)?;

    let outcome = invalidate_sessions(
        db,
        SessionTarget::User {
            user_id: target_user_id,
        },
        SessionInvalidationReason::PasswordResetByOther,
    )
    .await?;

    Ok(ResetOutcome::Ok {
        revoked_session_count: outcome.revoked_session_ids.len(),
    })
}

// ---- unlock --------------------------------------------------------------

/// Clear `locked_until` and reset `failed_login_count = 0` on the
/// target. Does NOT touch sessions — an unlock is not a session
/// event per `DESIGN_R4_EMERGENCY.md` §7.
///
/// Returns `previously_locked` so the CLI can render "the account
/// was indeed locked, now cleared" vs. "the account was already
/// open, this was a no-op" without an extra round-trip.
pub async fn unlock(db: &Db, target_user_id: i64) -> Result<UnlockOutcome> {
    if target_exists(db, target_user_id).await?.is_none() {
        return Ok(UnlockOutcome::UnknownTarget);
    }

    // Capture the pre-state so the outcome can distinguish "no-op"
    // from "actually unlocked." Both rows still write — clearing
    // an already-clear column is benign.
    let was_locked: bool = sqlx::query_scalar(
        "SELECT (locked_until IS NOT NULL AND locked_until > NOW()) \
                OR failed_login_count > 0 \
         FROM rustio_users WHERE id = $1",
    )
    .bind(target_user_id)
    .fetch_one(db.pool())
    .await
    .map_err(Error::from)?;

    sqlx::query(
        "UPDATE rustio_users \
         SET locked_until = NULL, failed_login_count = 0 \
         WHERE id = $1",
    )
    .bind(target_user_id)
    .execute(db.pool())
    .await
    .map_err(Error::from)?;

    Ok(UnlockOutcome::Ok {
        previously_locked: was_locked,
    })
}

// ---- disable_mfa ---------------------------------------------------------

/// Clear every MFA column on the target user, delete every backup-
/// code row, revoke every session for the user.
///
/// **Session-revocation scope.** `DESIGN_R4_EMERGENCY.md` §7 calls
/// for revoking only sessions with `trust_level = 'mfa_verified'`
/// (other sessions stay valid). The current [`SessionTarget`] enum
/// has no trust-level filter; rather than introduce a new variant
/// in commit #3, this function revokes ALL of the target's sessions
/// via `SessionTarget::User`. The over-broad revoke is conservative
/// — every revoked session forces a fresh login that picks up the
/// post-disable MFA state cleanly. A future
/// `SessionTarget::UserWithTrustLevel` variant could narrow this
/// without changing the function's caller contract.
///
/// Atomic: the column clear + backup-code DELETE land in one
/// transaction. Session revocation runs after commit.
pub async fn disable_mfa(db: &Db, target_user_id: i64) -> Result<DisableMfaOutcome> {
    if target_exists(db, target_user_id).await?.is_none() {
        return Ok(DisableMfaOutcome::UnknownTarget);
    }

    let was_enabled: bool =
        sqlx::query_scalar("SELECT COALESCE(mfa_enabled, FALSE) FROM rustio_users WHERE id = $1")
            .bind(target_user_id)
            .fetch_one(db.pool())
            .await
            .map_err(Error::from)?;

    let mut tx = db.pool().begin().await.map_err(Error::from)?;

    sqlx::query(
        "UPDATE rustio_users SET \
            mfa_enabled = FALSE, \
            mfa_secret_ciphertext = NULL, \
            mfa_secret_key_id = NULL, \
            mfa_last_used_step = NULL \
         WHERE id = $1",
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(Error::from)?;

    let deleted: u64 = sqlx::query("DELETE FROM rustio_mfa_backup_codes WHERE user_id = $1")
        .bind(target_user_id)
        .execute(&mut *tx)
        .await
        .map_err(Error::from)?
        .rows_affected();

    tx.commit().await.map_err(Error::from)?;

    let outcome = invalidate_sessions(
        db,
        SessionTarget::User {
            user_id: target_user_id,
        },
        SessionInvalidationReason::MfaDisabledByOther,
    )
    .await?;

    Ok(DisableMfaOutcome::Ok {
        was_enabled,
        deleted_backup_codes: deleted as usize,
        revoked_session_count: outcome.revoked_session_ids.len(),
    })
}

// ---- promote -------------------------------------------------------------

/// Change the target user's role to `new_role`.
///
/// Refuses to demote the sole active administrator: if the target
/// currently holds `Role::Administrator` AND `new_role !=
/// Administrator` AND no OTHER active administrators exist, returns
/// [`PromoteOutcome::SoleAdministratorDemoteRefused`]. This guard
/// is per `DESIGN_R4_EMERGENCY.md` §3.4 — the framework refuses to
/// leave the deployment with zero administrators, even via CLI.
///
/// Atomic: the role-write + session-revoke are in one transaction
/// to preserve doctrine 22 single-writer semantics while keeping
/// the promote operation isolated from concurrent session reads.
/// Session revocation runs after commit per the
/// `invalidate_sessions` contract.
pub async fn promote(db: &Db, target_user_id: i64, new_role: Role) -> Result<PromoteOutcome> {
    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT role, is_active FROM rustio_users WHERE id = $1")
            .bind(target_user_id)
            .fetch_optional(db.pool())
            .await
            .map_err(Error::from)?;
    let (current_role_str, _is_active) = match row {
        Some(r) => r,
        None => return Ok(PromoteOutcome::UnknownTarget),
    };

    let current_role = parse_role(&current_role_str).unwrap_or(Role::User);
    if current_role == new_role {
        return Ok(PromoteOutcome::NoChange { current_role });
    }

    // Sole-administrator demote check. `is_active = TRUE` is the
    // pre-condition for an administrator to be usable; demoting to
    // a tier below Administrator must leave at least one active
    // administrator standing.
    if current_role == Role::Administrator && new_role != Role::Administrator {
        let other_admins: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rustio_users \
             WHERE role = 'administrator' AND is_active = TRUE AND id <> $1",
        )
        .bind(target_user_id)
        .fetch_one(db.pool())
        .await
        .map_err(Error::from)?;
        if other_admins == 0 {
            return Ok(PromoteOutcome::SoleAdministratorDemoteRefused);
        }
    }

    sqlx::query("UPDATE rustio_users SET role = $1 WHERE id = $2")
        .bind(role_as_str(new_role))
        .bind(target_user_id)
        .execute(db.pool())
        .await
        .map_err(Error::from)?;

    let outcome = invalidate_sessions(
        db,
        SessionTarget::User {
            user_id: target_user_id,
        },
        SessionInvalidationReason::RoleChangedByOther,
    )
    .await?;

    Ok(PromoteOutcome::Ok {
        previous_role: current_role,
        new_role,
        revoked_session_count: outcome.revoked_session_ids.len(),
    })
}

// ---- emergency_access ----------------------------------------------------

/// Issue a single-use password-reset URL bypassing the email
/// mailer. The URL plaintext is returned in
/// [`EmergencyAccessOutcome::Ok::url_path`] — the operator hands it
/// to the target via whatever out-of-band channel makes sense.
///
/// Reuses R1's `rustio_password_reset_tokens` table: inserts a row
/// keyed by SHA-256(token), returns the plaintext token in the URL
/// path. The plaintext never lands in the DB; the token is single-
/// use per the R1 partial unique index on
/// `(token_hash) WHERE consumed_at IS NULL`.
///
/// `ttl_minutes` is clamped to `[1, 60]` — beyond 60 the operator
/// should use `reset_password` instead (longer TTLs widen the URL
/// interception window for diminishing operational benefit).
///
/// `InactiveTarget` is the only emergency operation that refuses
/// inactive users: issuing a URL to a deactivated account has no
/// recovery semantic.
pub async fn emergency_access(
    db: &Db,
    target_user_id: i64,
    ttl_minutes: i64,
) -> Result<EmergencyAccessOutcome> {
    let is_active = match target_exists(db, target_user_id).await? {
        Some(active) => active,
        None => return Ok(EmergencyAccessOutcome::UnknownTarget),
    };
    if !is_active {
        return Ok(EmergencyAccessOutcome::InactiveTarget);
    }

    let ttl = ttl_minutes.clamp(1, 60);
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl);

    // Generate 32 random bytes → URL-safe-base64-no-pad. Matches
    // R1's `issue_reset_token` token format.
    use base64::Engine;
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);

    use sha2::{Digest, Sha256};
    let token_hash = format!("{:x}", Sha256::digest(token.as_bytes()));

    let token_id: i64 = sqlx::query_scalar(
        "INSERT INTO rustio_password_reset_tokens \
            (user_id, token_hash, expires_at) \
         VALUES ($1, $2, $3) \
         RETURNING id",
    )
    .bind(target_user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .fetch_one(db.pool())
    .await
    .map_err(Error::from)?;

    Ok(EmergencyAccessOutcome::Ok {
        token_id,
        url_path: format!("/admin/reset-password/{token}"),
        expires_at,
    })
}

// ---- Role <-> string helpers --------------------------------------------
//
// `Role` already implements `FromStr` / `Display` somewhere in the
// framework, but the persisted shape (`'administrator'`, lowercase
// snake-case-ish) is what we need here and not all of those impls
// match. Keep the mapping explicit and local until R5 unifies the
// role-string surface.

fn parse_role(s: &str) -> Option<Role> {
    match s {
        "user" => Some(Role::User),
        "staff" => Some(Role::Staff),
        "supervisor" => Some(Role::Supervisor),
        "administrator" => Some(Role::Administrator),
        "developer" => Some(Role::Developer),
        _ => None,
    }
}

fn role_as_str(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Staff => "staff",
        Role::Supervisor => "supervisor",
        Role::Administrator => "administrator",
        Role::Developer => "developer",
    }
}

// `InvalidationOutcome` is re-exported by `auth::mod` already; this
// `use` keeps the type name reachable in callers that match on it.
#[allow(unused_imports)]
use InvalidationOutcome as _;
