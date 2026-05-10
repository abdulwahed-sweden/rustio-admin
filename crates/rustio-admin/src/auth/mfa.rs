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
//!   [`crate::admin::types::Admin::require_mfa`]. R3 commit #2.
//! - [`MfaKey`] / [`wrap_secret`] / [`unwrap_secret`] —
//!   AES-256-GCM secret encryption helpers (§8.1 of the design
//!   doc, D1). The plaintext TOTP secret is encrypted before it
//!   reaches the database; storage layout is `nonce ||
//!   ciphertext || tag`. `MfaKey::from_env` reads
//!   `RUSTIO_SECRET_KEY` (32-byte URL-safe-base64); the boot
//!   refusal when `MfaPolicy != Disabled` and the env var is
//!   unset lands in a later commit. Round-trip + tamper +
//!   wrong-key detection are pinned by unit tests. R3 commit #3.
//!
//! Subsequent commits will add: TOTP step generator + verifier
//! (RFC 6238, hand-rolled — §9.4), Argon2id backup-code hasher
//! with low-memory params (§8.1), enrolment / verification /
//! disable / regeneration runtime functions (§9), and `MfaPolicy`
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

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key as GcmKey, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;

use crate::auth::Role;
use crate::error::{Error, Result};
use crate::orm::Db;

/// AES-256-GCM key material for TOTP secret encryption (D1).
///
/// 32 raw bytes — the AES-256 key. Constructed from the
/// `RUSTIO_SECRET_KEY` environment variable (32-byte
/// URL-safe-base64-encoded) via [`MfaKey::from_env`], or from
/// raw bytes via [`MfaKey::from_bytes`] (for tests / explicit
/// construction).
///
/// `Clone` is intentional — the key is held by the framework's
/// `MfaSecretKeyResolver` (future commit) and cloned cheaply
/// onto cipher instances per-encryption. `Copy` is intentionally
/// NOT derived: a `Copy` key would silently scatter copies into
/// every stack frame that touches it; an explicit `.clone()`
/// makes key usage auditable on review.
///
/// Plaintext key material lives only in process memory. The
/// `Drop` is a no-op intentionally — the operating system zeroes
/// freed pages on most production deployments, and the framework
/// does not promise constant-time secure-erase on Drop. Operators
/// who require zeroize-on-drop semantics can wrap this type in
/// the `zeroize` crate's `Zeroizing` shim at the construction
/// site.
#[derive(Clone)]
#[allow(dead_code)] // call sites land in R3 commit #6+ (enrol / verify runtime)
pub struct MfaKey([u8; 32]);

#[allow(dead_code)] // see MfaKey type comment — light up in R3 commit #6+
impl MfaKey {
    /// Read the framework-wide secret key from the
    /// `RUSTIO_SECRET_KEY` environment variable.
    ///
    /// The variable carries 32 raw key bytes, encoded as
    /// URL-safe-base64 without padding. After decoding the
    /// constructor verifies the byte length is exactly 32.
    ///
    /// **Failure modes** (all surface as `Error::Internal` —
    /// the failure happens at boot, not at request time):
    ///
    /// - Env var unset.
    /// - Decode failure (invalid URL-safe-base64 alphabet,
    ///   stray padding, etc.).
    /// - Wrong length after decode (≠ 32 bytes).
    ///
    /// The boot guard that ties this requirement to
    /// `MfaPolicy != Disabled` is wired in a later commit; this
    /// constructor reports the failure but does NOT enforce
    /// "policy says Disabled, so missing key is fine."
    pub fn from_env() -> Result<Self> {
        let raw = std::env::var("RUSTIO_SECRET_KEY").map_err(|_| {
            Error::Internal(
                "RUSTIO_SECRET_KEY env var is unset; required when MfaPolicy != Disabled".into(),
            )
        })?;
        let decoded = URL_SAFE_NO_PAD.decode(raw.trim()).map_err(|e| {
            Error::Internal(format!(
                "RUSTIO_SECRET_KEY is not valid URL-safe-base64 (no padding): {e}"
            ))
        })?;
        let bytes: [u8; 32] = decoded.as_slice().try_into().map_err(|_| {
            Error::Internal(format!(
                "RUSTIO_SECRET_KEY decodes to {} bytes; AES-256 requires exactly 32",
                decoded.len()
            ))
        })?;
        Ok(Self(bytes))
    }

    /// Construct from raw 32 bytes. Used by tests and explicit
    /// project wiring (e.g. a project that derives the key from
    /// AWS KMS / HashiCorp Vault rather than an env var).
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the 32-byte key for the AES-256-GCM cipher's
    /// `KeyInit`. The reference is bounded to the borrow's
    /// lifetime; callers cannot retain it past the cipher
    /// construction.
    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Encrypt `plaintext` under `key` with AES-256-GCM.
///
/// Returns the on-disk byte layout: `nonce (12 bytes) ||
/// ciphertext || auth_tag (16 bytes)`. The nonce is generated
/// fresh per call from `rand::thread_rng()`.
///
/// **Output length** is `12 + plaintext.len() + 16`, exactly the
/// shape persisted in `rustio_users.mfa_secret_ciphertext` per
/// `DESIGN_R3_MFA.md` §8.1. Callers do not need to track the
/// nonce separately — it travels with the ciphertext.
///
/// **Infallible.** AEAD encryption with `aes-gcm` cannot fail
/// for in-memory plaintexts; the method that returns `Result` on
/// the underlying API exists for streaming-mode callers we do
/// not use. Returning `Vec<u8>` directly keeps the call sites
/// simple.
#[allow(dead_code)] // call site lands in R3 commit #6 (enrol_secret runtime)
pub fn wrap_secret(plaintext: &[u8], key: &MfaKey) -> Vec<u8> {
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher = Aes256Gcm::new(GcmKey::<Aes256Gcm>::from_slice(key.as_bytes()));
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("AES-256-GCM encrypt cannot fail for in-memory plaintext");

    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

/// Decrypt `input` (`nonce || ciphertext || tag`) under `key`.
///
/// **Failure modes** (all surface as `Error::Internal` — the
/// recovery is operator-side; the user surface treats this as
/// "session invalid" via the verify handler's outcome mapping):
///
/// - Input shorter than 28 bytes (no room for nonce + tag).
/// - AEAD verification failure: tampered ciphertext, wrong key,
///   nonce reuse on a different message, etc. The library does
///   not distinguish between these — they all reduce to "the
///   tag did not verify."
///
/// The function is constant-time at the AEAD primitive level;
/// the framework adds no timing-leak surface on top of it.
#[allow(dead_code)] // call site lands in R3 commit #7 (verify_totp runtime)
pub fn unwrap_secret(input: &[u8], key: &MfaKey) -> Result<Vec<u8>> {
    if input.len() < 12 + 16 {
        return Err(Error::Internal(format!(
            "MFA ciphertext too short ({} bytes); minimum is 28 (nonce + tag)",
            input.len()
        )));
    }
    let (nonce_bytes, ciphertext) = input.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(GcmKey::<Aes256Gcm>::from_slice(key.as_bytes()));
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| Error::Internal("MFA ciphertext failed AEAD verification".into()))
}

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

    fn fixed_test_key() -> MfaKey {
        // Deterministic 32-byte key for round-trip tests. The
        // value is arbitrary — we just need a stable key across
        // wrap and unwrap calls.
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7).wrapping_add(13);
        }
        MfaKey::from_bytes(bytes)
    }

    #[test]
    fn wrap_unwrap_round_trip_recovers_plaintext() {
        let key = fixed_test_key();
        let plaintext = b"hello-mfa-secret-20-bytes";
        let ciphertext = wrap_secret(plaintext, &key);

        // Storage layout: nonce (12) || ciphertext || tag (16).
        // Plaintext is 25 bytes ⇒ ciphertext_with_tag is 41 ⇒
        // total 53.
        assert_eq!(ciphertext.len(), 12 + plaintext.len() + 16);

        let recovered = unwrap_secret(&ciphertext, &key).expect("round-trip must decrypt");
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn wrap_uses_fresh_nonce_per_call() {
        // Two encryptions of the same plaintext under the same
        // key must NOT collide — fresh nonce per call. Without
        // this the AEAD's confidentiality breaks (same nonce +
        // same key + different plaintexts leaks XOR via known-
        // plaintext attacks).
        let key = fixed_test_key();
        let plaintext = b"identical-plaintext";
        let a = wrap_secret(plaintext, &key);
        let b = wrap_secret(plaintext, &key);
        assert_ne!(a, b, "fresh nonce per call must yield different ciphertext");
    }

    #[test]
    fn tampered_ciphertext_fails_aead_verification() {
        let key = fixed_test_key();
        let plaintext = b"sensitive-mfa-secret";
        let mut ciphertext = wrap_secret(plaintext, &key);

        // Flip a bit in the ciphertext body (post-nonce, pre-tag).
        ciphertext[20] ^= 0x01;
        let result = unwrap_secret(&ciphertext, &key);
        assert!(
            result.is_err(),
            "tampered ciphertext must fail AEAD verification"
        );
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key_enc = fixed_test_key();
        let key_dec = MfaKey::from_bytes([0xFFu8; 32]);
        let plaintext = b"wrong-key-test";
        let ciphertext = wrap_secret(plaintext, &key_enc);

        let result = unwrap_secret(&ciphertext, &key_dec);
        assert!(result.is_err(), "decrypt with wrong key must fail");
    }

    #[test]
    fn truncated_input_rejects_explicitly() {
        let key = fixed_test_key();
        // 27 bytes — one byte short of nonce + tag minimum.
        let too_short = [0u8; 27];
        let result = unwrap_secret(&too_short, &key);
        assert!(result.is_err(), "input below 28 bytes must reject");
    }
}
