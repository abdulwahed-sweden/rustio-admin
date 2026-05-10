//! TOTP multi-factor authentication (R3).
//!
//! See `DESIGN_R3_MFA.md` for the canonical contract this module
//! implements. R3 ships in 0.7.0; this module owns the MFA
//! runtime — TOTP enrolment + verification, backup-code
//! generation + consumption + regeneration, MFA disable, and the
//! AES-256-GCM secret-encryption helpers. The HTTP wrappers will
//! live in `admin::mfa_handlers`; routes are registered in
//! `admin::routes::register_admin_routes` after R2's
//! admin-recovery routes. The testcontainers integration suite
//! under `tests/integration_*.rs` exercises the DB-touching paths
//! end-to-end against an ephemeral Postgres, gated behind
//! `--features integration-test` per `DESIGN_R3_MFA.md` §13.3.
//!
//! ## Visibility note
//!
//! Items here are `pub` (rather than `pub(crate)`) so the
//! `crate::__integration` re-export module can re-export them
//! under the `integration-test` feature. The MODULE itself is
//! `pub(crate)` (`auth::mod`), so the canonical path
//! `rustio_admin::auth::mfa::*` remains closed to external
//! callers — `__integration` is the only door, and it is
//! itself feature-gated + `#[doc(hidden)]`. Same pattern as
//! `auth::recovery_admin`.
//!
//! ## What lives here today
//!
//! - [`migrate_user_mfa_schema`] — adds the additive R3 columns
//!   on `rustio_users` (`mfa_enabled`, `mfa_secret_ciphertext`,
//!   `mfa_secret_key_id`, `mfa_last_used_step`) plus the new
//!   `rustio_mfa_backup_codes` table and a per-user partial
//!   index on `(user_id) WHERE used_at IS NULL` for the
//!   verification-path scan (§7 of the design doc). R3 commit #1.
//! - [`MfaPolicy`] — the four-variant enum that controls
//!   framework-wide MFA enforcement (§11.1 of the design doc).
//!   `Disabled` / `Optional` (default) / `Required` /
//!   `RequiredForRoles(&[Role])`. The variant is data-only at
//!   this commit; the `login_guard` routing that consults it
//!   lands in a later commit (§12.3). Wired onto `Admin` via
//!   [`crate::admin::types::Admin::require_mfa`].
//!
//! Subsequent commits will add: TOTP step generator + verifier
//! (RFC 6238, hand-rolled — §9.4), AES-256-GCM secret wrap /
//! unwrap helpers (§8.1), Argon2id backup-code hasher with
//! low-memory params (§8.1), enrolment / verification / disable
//! / regeneration runtime functions (§9), and `MfaPolicy`
//! routing into `login_guard` (§12.3).
//!
//! ## Doctrine 22 reminder
//!
//! Centralised invalidation remains the single writer of
//! `revoked_at` on `rustio_sessions`. R3 will pass `MfaEnabled`
//! and `MfaDisabled` reasons to
//! [`crate::auth::sessions::invalidate_sessions`] when the
//! enrolment / disable runtime lands; nothing in this module
//! writes to `revoked_at` directly. See `DESIGN_SESSIONS.md`
//! Doctrine 22 for the grep proof contract.
//!
//! ## At-rest secrecy reminder
//!
//! TOTP secrets are encrypted with AES-256-GCM keyed by
//! `RUSTIO_SECRET_KEY` before persisting (D1 of the R3 design
//! doc). Backup codes are Argon2id-hashed with low-memory params
//! (D2). Plaintext TOTP secrets and plaintext backup codes
//! exist only in process memory during enrolment + verification.
//! The schema column `mfa_secret_ciphertext BYTEA` carries the
//! AEAD output (`nonce || ciphertext || tag`); the
//! `code_hash TEXT` column carries the Argon2id hash. The
//! schema enforced here is the persistence contract for those
//! invariants.
//!
//! Idempotent. Safe to call on every boot. `auth::init_tables`
//! invokes [`migrate_user_mfa_schema`] after R2's
//! `recovery_admin::migrate_user_lockout_schema`.

use crate::auth::Role;
use crate::error::Result;
use crate::orm::Db;

/// Framework-wide MFA enforcement policy.
///
/// Plain `Copy` enum (no trait object) — operators wire it onto
/// `Admin` via [`crate::admin::types::Admin::require_mfa`]. The
/// `login_guard` consults the active policy AFTER successful
/// password verification and AFTER R2's `must_change_password`
/// check (commit #15 of the R3 plan).
///
/// **Forward-only enforcement (D6).** Switching to
/// [`MfaPolicy::Required`] does NOT retroactively revoke
/// existing sessions. Existing users without MFA enrolled are
/// redirected to `/admin/mfa/enroll` at the next request that
/// hits `login_guard`. The pattern mirrors R2's
/// `must_change_password` interstitial.
///
/// **Default is [`MfaPolicy::Optional`].** R1 page copy contains
/// zero MFA mention; the doctrine-9 floor in DESIGN_RECOVERY
/// (email is convenience, not root of trust) sets the baseline.
/// Operators who want MFA enforcement opt in explicitly.
///
/// Typical project wiring:
///
/// ```ignore
/// use rustio_admin::auth::{MfaPolicy, Role};
///
/// // Enforce for everyone:
/// let admin = Admin::new().require_mfa(MfaPolicy::Required);
///
/// // Enforce for privileged roles only:
/// const PRIVILEGED: &[Role] = &[Role::Administrator, Role::Supervisor];
/// let admin = Admin::new().require_mfa(MfaPolicy::RequiredForRoles(PRIVILEGED));
///
/// // Reject MFA enrolment outright (e.g. for a public-kiosk admin):
/// let admin = Admin::new().require_mfa(MfaPolicy::Disabled);
/// ```
#[derive(Debug, Clone, Copy)]
pub enum MfaPolicy {
    /// MFA enrolment is rejected outright. Existing enrolments
    /// remain readable on the `rustio_users` row but the verify
    /// flow refuses to honour them. Used by deployments that
    /// have decided MFA is operationally inappropriate (kiosks,
    /// shared-credential workflows, etc.).
    Disabled,
    /// Default. Users may enrol; users without MFA can sign in
    /// with password alone. The pre-R3 framework behaviour.
    Optional,
    /// Every user must enrol. Forward-only — existing sessions
    /// remain valid; the `login_guard` redirects users without
    /// MFA to `/admin/mfa/enroll` at the next request.
    Required,
    /// Required only for users whose [`Role`] appears in the
    /// slice. Forward-only with the same semantics as
    /// [`MfaPolicy::Required`]. Empty slice is equivalent to
    /// [`MfaPolicy::Optional`] — the policy reads "no role
    /// requires MFA" rather than "no users require MFA".
    RequiredForRoles(&'static [Role]),
}

impl Default for MfaPolicy {
    /// [`MfaPolicy::Optional`] is the framework default. R1 page
    /// copy contains zero MFA mention; operators opt into
    /// enforcement explicitly via
    /// [`crate::admin::types::Admin::require_mfa`].
    fn default() -> Self {
        Self::Optional
    }
}

/// Add the additive R3 MFA schema.
///
/// Adds four columns on `rustio_users`:
///
/// - `mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE` — the boolean
///   gate the login flow consults after password verification.
///   `FALSE` means MFA is not enrolled; the rest of the columns
///   are NULL. `TRUE` means the rest of the columns are
///   populated and `/admin/mfa/verify` is required to promote
///   the session to `mfa_verified`.
/// - `mfa_secret_ciphertext BYTEA` (nullable) — the AES-256-GCM
///   encrypted TOTP secret. Storage layout is
///   `nonce (12 bytes) || ciphertext || auth_tag (16 bytes)`.
///   Plaintext secret never reaches disk; decryption happens in
///   process memory during verification, scoped to the request
///   handler.
/// - `mfa_secret_key_id INT` (nullable) — which version of
///   `RUSTIO_SECRET_KEY` encrypted this row. Per-row stamp lets
///   key rotation proceed in stages: existing rows continue to
///   decrypt against their stamped key while new rows encrypt
///   against the active key. The retire-old-key sweep is a
///   future operational procedure (see §7 / Appendix E of the
///   design doc).
/// - `mfa_last_used_step BIGINT` (nullable) — the highest TOTP
///   step value previously accepted by `verify_totp`. Replay
///   protection (D4): a TOTP code from a step `≤
///   mfa_last_used_step` is rejected even if cryptographically
///   valid. Monotonic per user; never decrements.
///
/// Adds one new table for backup codes:
///
/// - `rustio_mfa_backup_codes` with `id BIGSERIAL`,
///   `user_id BIGINT NOT NULL REFERENCES rustio_users(id) ON
///   DELETE CASCADE`, `code_hash TEXT NOT NULL` (Argon2id,
///   low-memory params), `created_at TIMESTAMPTZ NOT NULL`,
///   `used_at TIMESTAMPTZ` (nullable; NULL = unused). The
///   `ON DELETE CASCADE` is the disable / account-deletion
///   contract — when the parent user disables MFA, the runtime
///   issues an explicit `DELETE` on these rows; when the user
///   row itself is deleted, cascade cleans up.
///
/// Plus a per-user partial index
/// `rustio_mfa_backup_codes_user_unused_idx ON (user_id) WHERE
/// used_at IS NULL` for the verification-path scan: at most 8
/// rows per user × the partial predicate makes the consume
/// scan an index seek to a tiny page.
///
/// **Backfill.** Existing `rustio_users` rows get the column
/// defaults: `mfa_enabled = FALSE`, all three NULL fields. No
/// pre-existing user is auto-enrolled. The new
/// `rustio_mfa_backup_codes` table is empty after the
/// migration.
///
/// **Rollback.** Rolling back to 0.6.0 (R2) is data-safe — the
/// columns and table become unreferenced; nothing hard-fails.
/// Forward migration is the supported direction; reverse is an
/// operator's snapshot-restore concern.
///
/// Idempotent. Safe to call on every boot. Depends on
/// `rustio_users` existing first (which `auth::init_tables`
/// guarantees by ordering this call after `init_user_tables`
/// and the R1 / R2 schema migrations).
pub async fn migrate_user_mfa_schema(db: &Db) -> Result<()> {
    sqlx::query(
        "ALTER TABLE rustio_users \
         ADD COLUMN IF NOT EXISTS mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS mfa_secret_ciphertext BYTEA")
        .execute(db.pool())
        .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS mfa_secret_key_id INT")
        .execute(db.pool())
        .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS mfa_last_used_step BIGINT")
        .execute(db.pool())
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_mfa_backup_codes ( \
            id          BIGSERIAL PRIMARY KEY, \
            user_id     BIGINT NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE, \
            code_hash   TEXT NOT NULL, \
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(), \
            used_at     TIMESTAMPTZ \
         )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_mfa_backup_codes_user_unused_idx \
         ON rustio_mfa_backup_codes (user_id) \
         WHERE used_at IS NULL",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_optional() {
        assert!(matches!(MfaPolicy::default(), MfaPolicy::Optional));
    }

    #[test]
    fn policy_is_copy() {
        // Copy ensures the policy can be carried by value without
        // Arc indirection. The compiler enforces this at the
        // declaration site (`#[derive(Copy)]`); this test pins
        // the contract so a future field addition that breaks
        // Copy fails the suite, not just the next caller.
        const ROLES: &[Role] = &[Role::Administrator];
        let original = MfaPolicy::RequiredForRoles(ROLES);
        let copy = original;
        // Both bindings are usable — Copy.
        assert!(matches!(original, MfaPolicy::RequiredForRoles(_)));
        assert!(matches!(copy, MfaPolicy::RequiredForRoles(_)));
    }
}
