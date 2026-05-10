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
//! - [`generate_backup_codes`] / [`hash_backup_code`] /
//!   [`verify_backup_code`] / [`normalise_backup_code`] +
//!   [`BACKUP_CODE_COUNT`] / [`BACKUP_CODE_LEN`] — the
//!   backup-code surface (§8.1, D2 + D7). 8 codes per batch in
//!   the locked `XXXX-XXXX` format from the 31-char
//!   ambiguity-stripped alphabet (no `0/O/1/I/L`); Argon2id with
//!   low-memory params (`m = 16 MiB`, `t = 2`, `p = 1`); the
//!   normalise function uppercases and strips the hyphen so the
//!   user can copy with or without the separator. Every helper
//!   is marked `#[allow(dead_code)]` until the enrolment +
//!   verification runtime wires the call sites in R3 commits
//!   #6 and #7. R3 commit #4.
//! - [`current_step`] / [`generate_totp`] / [`verify_totp`] —
//!   hand-rolled RFC 6238 TOTP (§9.4). HMAC-SHA1 (the
//!   authenticator-app-default algorithm; `algorithm=SHA256`
//!   variants exist but interop is best with SHA-1), 30-second
//!   step interval, ±1 step skew tolerance (per Appendix B
//!   locked decisions). 6-digit codes per industry standard.
//!   `verify_totp` returns the step that matched on success so
//!   the caller can stamp `mfa_last_used_step` for replay
//!   protection (D4). Pinned by the canonical RFC 6238
//!   Appendix B test vectors (truncated from 8-digit to 6-digit
//!   per authenticator-app standard). R3 commit #5.
//! - [`provision_secret`] / [`ProvisionedSecret`] /
//!   [`confirm_enrolment`] / [`EnrolOutcome`] — the enrolment
//!   runtime (§4.1, §9). `provision_secret` is pure: 20 random
//!   bytes for RFC 6238 + base32 encoding for the QR / manual
//!   entry display. `confirm_enrolment` is the DB-touching
//!   path: verifies the user's first TOTP code, AES-GCM
//!   encrypts the secret, stores the row, generates +
//!   Argon2id-hashes 8 backup codes, INSERTs them, and emits
//!   `AuditEvent::MfaEnabled`. The first MFA function that
//!   touches `rustio_users.mfa_enabled` and writes
//!   `rustio_mfa_backup_codes`. The HTTP handler that calls it
//!   lands in a later commit. R3 commit #6.
//! - [`verify_totp_for_user`] / [`VerifyOutcome`] — the TOTP
//!   verification runtime (§4.2, D4). Reads the encrypted
//!   secret + `mfa_last_used_step` from the user row,
//!   decrypts via [`unwrap_secret`], runs [`verify_totp`]
//!   against the candidate, and rejects steps at or below
//!   the stored value (D4 replay protection). On success
//!   stamps the new step. No audit row — TOTP success is
//!   captured via the session-promotion `parent_session_id`
//!   lineage per §8.3, not a separate event. R3 commit #7.
//! - [`consume_backup_code`] / [`BackupConsumeOutcome`] — the
//!   backup-code consume runtime (§4.4, D7). Argon2id-verifies
//!   the candidate against every unused row for the user
//!   (constant-time iteration), atomically marks the matching
//!   row `used_at = NOW()`, emits
//!   `AuditEvent::MfaCodeConsumed` with metadata
//!   `{ code_id, remaining_codes, via }`. Single-use enforced
//!   at the index level + a conditional UPDATE that races
//!   safely. R3 commit #8.
//!
//! Subsequent commits will add: disable / regeneration
//! runtime functions (§9), and `MfaPolicy` routing into
//! `login_guard` (§12.3).
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
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::{Rng, RngCore};
use sha1::Sha1;

use crate::admin::audit::{record as audit_record, ActionType, AuditEvent, LogEntry};
use crate::admin::builtin::client_ip;
use crate::auth::Role;
use crate::error::{Error, Result};
use crate::http::Request;
use crate::orm::Db;

type HmacSha1 = Hmac<Sha1>;

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

// -----------------------------------------------------------------
// Backup codes (R3 commit #4)
// -----------------------------------------------------------------

/// Number of backup codes generated per batch. Locked at 8 per
/// `DESIGN_R3_MFA.md` Appendix B. Industry-standard range is
/// 8-16; 8 is enough for emergency use without bloating the
/// post-enrolment confirmation page.
#[allow(dead_code)] // call site lands in R3 commit #6 (enrolment runtime)
pub const BACKUP_CODE_COUNT: usize = 8;

/// Backup-code length in characters, excluding the visual
/// hyphen separator at position 4. Locked at 8 (rendered as
/// `XXXX-XXXX`) per `DESIGN_R3_MFA.md` Appendix B.
pub const BACKUP_CODE_LEN: usize = 8;

/// 31-character ambiguity-stripped alphabet for backup codes.
/// Excludes `0` / `O` (digit zero / letter O), `1` / `I` /
/// `L` (digit one / letter I / letter L). Per
/// `DESIGN_R3_MFA.md` Appendix B locked decision; the alphabet
/// is the persistence contract — changing it breaks any code
/// that was issued under a different alphabet.
///
/// Entropy per backup code: `8 chars × log2(31) ≈ 39.6 bits`
/// — adequate for single-use rate-limited verification. The
/// design doc rounds to "≈41 bits" approximately.
const BACKUP_CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

/// Argon2id parameters for backup-code hashing. Locked at
/// `m = 16 MiB / t = 2 / p = 1` per `DESIGN_R3_MFA.md` §8.1.
///
/// Lower than full password Argon2id (default `m ≈ 19 MiB`)
/// because backup codes have higher entropy per character than
/// passwords and verification runs on every login attempt that
/// tries a backup code (up to `BACKUP_CODE_COUNT` rows). Full
/// Argon2id would add latency without strengthening the
/// security model meaningfully.
fn backup_code_argon2() -> Result<Argon2<'static>> {
    let params = Params::new(16 * 1024, 2, 1, None)
        .map_err(|e| Error::Internal(format!("argon2 params: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Generate a fresh batch of backup codes.
///
/// Returns `count` strings of the form `XXXX-XXXX` where each
/// `X` is drawn unbiased from [`BACKUP_CODE_ALPHABET`] using
/// `rand::thread_rng().gen_range(...)`. The hyphen is purely
/// visual — at storage time the framework normalises away
/// hyphens via [`normalise_backup_code`].
///
/// **Plaintext lifecycle (D2).** The returned strings are the
/// only place the plaintext exists. Callers MUST hash via
/// [`hash_backup_code`] before persisting and MUST render the
/// plaintext to the user exactly once on the enrolment /
/// regeneration success page. After that response, the
/// plaintext is dropped from memory.
///
/// Typical caller pattern (R3 commit #6):
///
/// ```ignore
/// let codes = generate_backup_codes(BACKUP_CODE_COUNT);
/// let hashes: Vec<String> = codes
///     .iter()
///     .map(|c| hash_backup_code(c))
///     .collect::<Result<_>>()?;
/// // INSERT hashes into rustio_mfa_backup_codes
/// // RENDER `codes` to the user once, then drop
/// ```
#[allow(dead_code)] // call sites land in R3 commit #6 (enrolment) +
                    // commit when regenerate_backup_codes lands
pub fn generate_backup_codes(count: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let alphabet_len = BACKUP_CODE_ALPHABET.len();
    (0..count)
        .map(|_| {
            // 8 chars + 1 hyphen = 9 chars total.
            let mut out = String::with_capacity(BACKUP_CODE_LEN + 1);
            for i in 0..BACKUP_CODE_LEN {
                if i == 4 {
                    out.push('-');
                }
                let idx = rng.gen_range(0..alphabet_len);
                out.push(BACKUP_CODE_ALPHABET[idx] as char);
            }
            out
        })
        .collect()
}

/// Normalise a user-submitted backup code for hash comparison.
///
/// Strips every non-alphanumeric character (so `XXXX-XXXX`,
/// `XXXXXXXX`, `xxxx xxxx`, `xxxx-xxxx`, etc. all collapse to
/// the same canonical form) and uppercases. The hash compare
/// runs on the canonical form.
///
/// Idempotent: `normalise(normalise(x)) == normalise(x)`.
#[allow(dead_code)] // call site lands in R3 commit #8 (consume_backup_code runtime)
pub fn normalise_backup_code(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

/// Hash a backup code with Argon2id (low-memory params).
///
/// Generates a fresh 16-byte salt per call from the OS RNG.
/// Returns the PHC string (`$argon2id$v=19$m=16384,t=2,p=1$...`)
/// suitable for storage in `rustio_mfa_backup_codes.code_hash`.
/// The PHC string is self-describing — verification reads the
/// params from the hash itself.
///
/// The caller normalises the plaintext via
/// [`normalise_backup_code`] before passing to this function so
/// the user's hyphen / casing variation does not affect the
/// hash.
///
/// **Failure modes** (all `Error::Internal` — the failure is
/// operator-side at boot, not user-facing):
///
/// - Argon2id parameter construction fails (should not happen
///   with the locked `m / t / p` values).
/// - Hashing itself fails (rare; usually OOM under contrived
///   conditions).
#[allow(dead_code)] // call site lands in R3 commit #6 (enrolment runtime)
pub fn hash_backup_code(plaintext: &str) -> Result<String> {
    let argon2 = backup_code_argon2()?;
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash = argon2
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|e| Error::Internal(format!("argon2 hash: {e}")))?;
    Ok(hash.to_string())
}

/// Verify a normalised backup-code candidate against a stored
/// PHC hash.
///
/// Reads the Argon2 parameters from the PHC string itself, so
/// the verifier does not need to know the params used at hash
/// time. Constant-time at the Argon2id primitive level.
///
/// Returns `false` for any failure shape — invalid PHC string,
/// param mismatch, hash mismatch, etc. The caller does not
/// distinguish causes; the user-facing response is uniform per
/// `DESIGN_R3_MFA.md` §4.4.
#[allow(dead_code)] // call site lands in R3 commit #8 (consume_backup_code runtime)
pub fn verify_backup_code(plaintext: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(plaintext.as_bytes(), &parsed)
        .is_ok()
}

// -----------------------------------------------------------------
// TOTP — RFC 6238 (R3 commit #5)
// -----------------------------------------------------------------
//
// Hand-rolled HMAC-SHA1-based TOTP per RFC 6238, with the
// canonical 30-second step interval and 6-digit code format.
// Pinned by the RFC 6238 Appendix B test vectors (truncated
// from 8-digit to 6-digit). The framework's TOTP secret is the
// 20-byte default; longer secrets are accepted but not
// recommended (no security gain; reduces interop surface).
//
// Why hand-rolled rather than `totp-rs`: the framework's
// dependency-conservative character (one stylesheet, narrow
// surface). RFC 6238 is small enough to review at the source
// level; the canonical test vectors give a strong correctness
// signal. See DESIGN_R3_MFA.md §9.4 + Appendix B for the
// trade-off discussion.

/// TOTP step number for the given Unix time and step interval.
///
/// Pure function: `now_unix / step_seconds` (integer division).
/// At the canonical 30-second interval, the step value
/// increments every 30 seconds of wall-clock time. The
/// `mfa_last_used_step` column persists the highest step value
/// previously accepted by [`verify_totp`] for replay protection
/// (D4).
#[allow(dead_code)] // call sites land in R3 commit #6 (enrolment) + #7 (verify_totp)
pub fn current_step(now_unix: u64, step_seconds: u64) -> u64 {
    debug_assert!(step_seconds > 0, "step_seconds must be > 0");
    now_unix / step_seconds
}

/// Generate a 6-digit TOTP code for the given secret + step
/// per RFC 6238 (HMAC-SHA1 + dynamic truncation per RFC 4226
/// §5.3).
///
/// Steps:
///
/// 1. Compute `hmac = HMAC-SHA1(secret, step.to_be_bytes())`.
///    The 8-byte step value is encoded big-endian per RFC 4226.
/// 2. Read `offset = hmac[19] & 0x0F`. The low nibble of the
///    last HMAC byte selects a window into the 20-byte HMAC.
/// 3. Read 4 bytes starting at `offset`, masking the high bit
///    of the first byte (drops the sign bit per RFC 4226 §5.3).
/// 4. Modulo `1_000_000` to yield a 6-digit value.
///
/// Returns the integer TOTP value in `[0, 999_999]`. Callers
/// rendering for display should pad with leading zeros via
/// `format!("{:06}", code)`.
///
/// **Infallible.** `Hmac::new_from_slice` accepts any key
/// length per the HMAC construction; the framework never
/// produces an invalid secret length internally.
#[allow(dead_code)] // call sites land in R3 commit #6 (enrolment) + #7 (verify_totp)
pub fn generate_totp(secret: &[u8], step: u64) -> u32 {
    // UFCS to disambiguate from `aes_gcm::aead::KeyInit` —
    // both traits define a `new_from_slice` method.
    let mut mac = <HmacSha1 as Mac>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&step.to_be_bytes());
    let hash = mac.finalize().into_bytes();

    // Dynamic truncation per RFC 4226 §5.3.
    let offset = (hash[19] & 0x0F) as usize;
    let bin_code = u32::from_be_bytes([
        hash[offset] & 0x7F,
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]);

    bin_code % 1_000_000
}

/// Verify a TOTP candidate within the configured step skew.
///
/// Tries the current step ± `skew_steps` against `candidate`.
/// Returns `Some(step)` of the matching step on success so the
/// caller can stamp `rustio_users.mfa_last_used_step` for D4
/// replay protection; returns `None` if no step in the window
/// matches.
///
/// **Replay protection runs at the call site, not here.** This
/// function reports cryptographic match only. The verify
/// runtime (R3 commit #7) reads `mfa_last_used_step` from the
/// user row and rejects matches at or below the stored value
/// before calling this function.
///
/// **Skew window** is symmetric: `[current - skew_steps,
/// current + skew_steps]` inclusive. Default skew (per
/// `RecoveryPolicy::mfa_skew_steps`) is 1, giving a 90-second
/// total acceptance window at the canonical 30-second step.
#[allow(dead_code)] // call site lands in R3 commit #7 (verify_totp runtime)
pub fn verify_totp(
    secret: &[u8],
    candidate: u32,
    now_unix: u64,
    step_seconds: u64,
    skew_steps: u32,
) -> Option<u64> {
    let current = current_step(now_unix, step_seconds);
    let skew = i64::from(skew_steps);

    for delta in -skew..=skew {
        let step_to_try = (current as i64).saturating_add(delta).max(0) as u64;
        if generate_totp(secret, step_to_try) == candidate {
            return Some(step_to_try);
        }
    }
    None
}

// -----------------------------------------------------------------
// Enrolment runtime (R3 commit #6)
// -----------------------------------------------------------------

/// A freshly-provisioned TOTP secret + its base32 encoding for
/// QR / manual-entry display.
///
/// **Lifecycle.** The struct's two fields contain the same
/// secret in two encodings; both are plaintext. The handler
/// MUST hold this value for the duration of the GET → POST
/// enrolment hand-off (typically via in-memory session-state
/// or a short-lived encrypted form-token), then MUST discard
/// it after [`confirm_enrolment`] runs. Plaintext lives only
/// in process memory; the at-rest persistence contract (D1)
/// is enforced inside `confirm_enrolment` via [`wrap_secret`].
#[allow(dead_code)] // fields read by the enrolment GET handler in a later commit
pub struct ProvisionedSecret {
    /// 20 random bytes from the OS RNG. RFC 6238 recommends
    /// HMAC-SHA1's block size (64 bytes) or output size
    /// (20 bytes); 20 is the universal authenticator-app
    /// minimum and matches every standard QR-provisioning URL
    /// in the wild.
    pub secret_bytes: Vec<u8>,
    /// Base32 (RFC 4648) without padding — the form expected
    /// by `otpauth://totp/...?secret=<this>` URLs and by
    /// authenticator apps that accept manual entry.
    pub base32: String,
}

/// Generate a fresh TOTP secret + its base32 encoding.
///
/// Pure (apart from the OS RNG read). Returns 20 raw bytes
/// drawn from `rand::thread_rng().fill_bytes` plus the
/// matching base32 string. Callers compose the `otpauth://`
/// URL elsewhere — this function does not touch the project's
/// issuer name or the user's email; those concerns live at the
/// HTTP layer.
#[allow(dead_code)] // call site lands in the enrolment GET handler
pub fn provision_secret() -> ProvisionedSecret {
    let mut bytes = vec![0u8; 20];
    rand::thread_rng().fill_bytes(&mut bytes);
    let base32 = base32_encode_no_pad(&bytes);
    ProvisionedSecret {
        secret_bytes: bytes,
        base32,
    }
}

/// RFC 4648 base32 encoder (no padding). Hand-rolled rather
/// than added as a dependency to match the framework's
/// dependency-conservative character — base32 is ~30 lines and
/// the alphabet is the persistence contract for the
/// `otpauth://...?secret=...` URL format.
///
/// Pinned by the standard RFC 4648 §10 test vector
/// `"foobar" -> "MZXW6YTBOI"`.
fn base32_encode_no_pad(bytes: &[u8]) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut buffer: u32 = 0;
    let mut bits_in_buffer: u8 = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits_in_buffer += 8;
        while bits_in_buffer >= 5 {
            bits_in_buffer -= 5;
            let idx = (buffer >> bits_in_buffer) as usize & 0x1F;
            out.push(ALPHA[idx] as char);
        }
    }
    if bits_in_buffer > 0 {
        let idx = (buffer << (5 - bits_in_buffer)) as usize & 0x1F;
        out.push(ALPHA[idx] as char);
    }
    out
}

/// Outcome of [`confirm_enrolment`]. Lets the caller render the
/// right page without embedding HTTP concerns in the runtime
/// layer.
#[allow(dead_code)] // variants light up at the HTTP handler in a later commit
pub enum EnrolOutcome {
    /// The user's first TOTP code matched the just-provisioned
    /// secret. The secret has been encrypted and persisted on
    /// the user row; the 8 backup codes have been hashed and
    /// inserted into `rustio_mfa_backup_codes`. The plaintext
    /// backup codes ride in the variant for the one-time
    /// success-page render (D2).
    Enrolled { plain_backup_codes: Vec<String> },
    /// The candidate code did not match the secret within the
    /// configured skew window. No DB writes occurred; the
    /// caller can re-render the verify form.
    InvalidCode,
    /// The user already has `mfa_enabled = TRUE`. Defensive
    /// — should not happen if the enrolment handler checks
    /// state up-front, but the runtime refuses anyway to keep
    /// the contract honest.
    AlreadyEnrolled,
}

/// Confirm a TOTP enrolment by verifying the user's first code
/// against the provisioned secret, then persisting everything.
///
/// **Inputs.**
///
/// - `request` — for client-IP capture into the audit row.
/// - `user_id` — the enrolling user (self-action; actor == target).
/// - `secret_bytes` — the 20-byte TOTP secret returned by
///   [`provision_secret`]. The handler holds this across the
///   GET → POST round-trip and passes it back here.
/// - `candidate_code` — the 6-digit TOTP code the user typed.
/// - `step_seconds` — TOTP step interval (locked at 30s per
///   Appendix B).
/// - `skew_steps` — accepted skew window (locked at ±1 per
///   Appendix B).
/// - `key` — the AES-256-GCM key for at-rest encryption.
/// - `key_id` — the active `RUSTIO_SECRET_KEY` version, stamped
///   onto `mfa_secret_key_id` for staged-rotation decryption (D8).
/// - `correlation_id` — forensic-chain anchor.
///
/// **Steps.**
///
/// 1. SELECT `mfa_enabled`. If TRUE → `AlreadyEnrolled` (no DB
///    writes).
/// 2. `verify_totp`. If no step matches → `InvalidCode` (no DB
///    writes).
/// 3. AES-256-GCM encrypt the secret via [`wrap_secret`].
/// 4. UPDATE `rustio_users` setting `mfa_enabled = TRUE`,
///    `mfa_secret_ciphertext`, `mfa_secret_key_id`, and
///    `mfa_last_used_step` (the step that just verified, for D4
///    replay protection).
/// 5. [`generate_backup_codes`] (`BACKUP_CODE_COUNT`).
/// 6. Hash each via [`hash_backup_code`] and INSERT into
///    `rustio_mfa_backup_codes`.
/// 7. Emit `AuditEvent::MfaEnabled` with metadata
///    `{ "backup_codes_count", "key_id" }`.
///
/// Returns `EnrolOutcome::Enrolled { plain_backup_codes }`. The
/// caller renders the codes ONCE on the success page, then
/// drops the strings. After the response is sent, the only
/// place the codes exist is the Argon2id hashes in the DB.
///
/// **Doctrine 22.** This function does not write `revoked_at`.
/// The audit emission and DB updates do not pass through
/// `invalidate_sessions`; enrolment does not invalidate
/// existing sessions per `DESIGN_R3_MFA.md` §4.1.
#[allow(dead_code)] // call site lands at the enrolment POST handler in a later commit
#[allow(clippy::too_many_arguments)]
pub async fn confirm_enrolment(
    db: &Db,
    request: &Request,
    user_id: i64,
    secret_bytes: &[u8],
    candidate_code: u32,
    step_seconds: u64,
    skew_steps: u32,
    key: &MfaKey,
    key_id: u32,
    correlation_id: Option<&str>,
) -> Result<EnrolOutcome> {
    // 1. Refuse if already enrolled.
    let already: Option<bool> =
        sqlx::query_scalar("SELECT mfa_enabled FROM rustio_users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(db.pool())
            .await?;
    let Some(already) = already else {
        return Err(Error::NotFound(format!("user {user_id} not found")));
    };
    if already {
        return Ok(EnrolOutcome::AlreadyEnrolled);
    }

    // 2. Verify the candidate code against the freshly-provisioned
    //    secret.
    let now_unix = Utc::now().timestamp().max(0) as u64;
    let step = match verify_totp(
        secret_bytes,
        candidate_code,
        now_unix,
        step_seconds,
        skew_steps,
    ) {
        Some(step) => step,
        None => return Ok(EnrolOutcome::InvalidCode),
    };

    // 3. Encrypt the secret for at-rest storage (D1).
    let ciphertext = wrap_secret(secret_bytes, key);

    // 4. Update the user row. Stamps mfa_last_used_step with the
    //    step that just verified, so the very first verify_totp
    //    after enrolment cannot replay this same code (D4).
    sqlx::query(
        "UPDATE rustio_users \
         SET mfa_enabled = TRUE, \
             mfa_secret_ciphertext = $1, \
             mfa_secret_key_id = $2, \
             mfa_last_used_step = $3 \
         WHERE id = $4",
    )
    .bind(&ciphertext)
    .bind(key_id as i32)
    .bind(step as i64)
    .bind(user_id)
    .execute(db.pool())
    .await?;

    // 5. Generate the backup-code batch.
    let plain_codes = generate_backup_codes(BACKUP_CODE_COUNT);

    // 6. Hash + insert each. Normalisation runs at consume time
    //    (the user types XXXX-XXXX with the hyphen); the hashes
    //    persist the canonical form.
    for code in &plain_codes {
        let normalised = normalise_backup_code(code);
        let hash = hash_backup_code(&normalised)?;
        sqlx::query("INSERT INTO rustio_mfa_backup_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(&hash)
            .execute(db.pool())
            .await?;
    }

    // 7. Audit emit.
    let metadata = serde_json::json!({
        "backup_codes_count": BACKUP_CODE_COUNT,
        "key_id": key_id,
    });
    let ip = client_ip(request);
    let mut entry = LogEntry::new(user_id, ActionType::Update, "user", user_id)
        .with_event(AuditEvent::MfaEnabled)
        .with_actor(user_id);
    entry.correlation_id = correlation_id;
    entry.ip_address = ip.as_deref();
    entry.metadata = Some(metadata);
    entry.summary = "MFA enabled (TOTP + 8 backup codes)".to_string();
    audit_record(db, entry).await?;

    Ok(EnrolOutcome::Enrolled {
        plain_backup_codes: plain_codes,
    })
}

// -----------------------------------------------------------------
// Verification runtime — TOTP login second factor (R3 commit #7)
// -----------------------------------------------------------------

/// Outcome of [`verify_totp_for_user`]. Lets the verify
/// handler render the right page without embedding HTTP
/// concerns in the runtime layer.
///
/// All four variants collapse to a uniform user-facing
/// response per `DESIGN_R3_MFA.md` §3.1 disclosure rules —
/// the handler must NOT render different copy for `Replay`
/// vs `Invalid` vs `NotEnrolled`. The variant distinctions
/// exist for forensic logging, future audit emission, and
/// internal debugging only.
#[allow(dead_code)] // variants light up at the verify handler in a later commit
pub enum VerifyOutcome {
    /// The candidate code matched within the skew window AND
    /// the matched step is strictly greater than
    /// `mfa_last_used_step`. The runtime has stamped the new
    /// step; the caller proceeds with trust escalation
    /// (mint a fresh `mfa_verified` session row, revoke the
    /// pending row, swap the cookie).
    Verified { step_used: u64 },
    /// The candidate matched cryptographically but the matched
    /// step is at or below `mfa_last_used_step` — D4 replay
    /// protection. Cause: a network-captured code, a
    /// double-submit, or clock drift on the user's device.
    /// Caller MUST NOT trust-escalate; user-facing copy is
    /// uniform with `Invalid`.
    Replay { last_used_step: u64 },
    /// The candidate did not match within the configured skew
    /// window, or the candidate string was not parseable as a
    /// 6-digit number.
    Invalid,
    /// The user row exists but `mfa_enabled = FALSE`. Should
    /// not happen if the verify handler checks state up-front,
    /// but the runtime refuses anyway to keep the contract
    /// honest.
    NotEnrolled,
}

/// Verify a TOTP candidate for an enrolled user.
///
/// **Inputs.**
///
/// - `user_id` — the user being challenged.
/// - `candidate_code_str` — the 6-digit string the user typed.
///   Whitespace-trimmed and parsed to `u32`; invalid input
///   collapses to `VerifyOutcome::Invalid`.
/// - `step_seconds` — TOTP step interval (locked at 30s per
///   Appendix B).
/// - `skew_steps` — accepted skew window (locked at ±1 per
///   Appendix B).
/// - `key` — the AES-256-GCM key for at-rest decryption.
///   Future: when the `MfaSecretKeyResolver` trait lands,
///   this becomes a resolver lookup keyed by the row's
///   `mfa_secret_key_id`. For now (R3 commit #7) the
///   framework assumes a single active key (`key_id = 1`).
///
/// **Steps.**
///
/// 1. Parse `candidate_code_str` as a 6-digit `u32`. Failure
///    → `Invalid`.
/// 2. SELECT `mfa_enabled`, `mfa_secret_ciphertext`,
///    `mfa_last_used_step` from `rustio_users`. Missing row
///    → `Error::NotFound`.
/// 3. If `!mfa_enabled` → `NotEnrolled`.
/// 4. If ciphertext is `NULL` while `mfa_enabled = TRUE` →
///    `Error::Internal` (corrupted state — cannot happen
///    via the framework's own writes).
/// 5. [`unwrap_secret`] decrypts the ciphertext under `key`.
///    Decryption failure surfaces as `Error::Internal` (key
///    mismatch or tampering — operator-side recovery).
/// 6. [`verify_totp`] against the secret. No match within the
///    skew window → `Invalid`.
/// 7. **D4 replay check.** If the matched step is at or below
///    `mfa_last_used_step` → `Replay`. The previously-stored
///    step value rides in the variant for forensic logging.
/// 8. UPDATE `mfa_last_used_step` to the just-verified step.
/// 9. Return `Verified { step_used }`.
///
/// **No audit row.** TOTP success is captured via the
/// session-promotion `parent_session_id` lineage per §8.3.
/// Backup-code consume DOES emit `AuditEvent::MfaCodeConsumed`
/// (R3 commit #8).
///
/// **Doctrine 22.** This function does not write `revoked_at`.
/// The trust-escalation that follows (mint fresh
/// `mfa_verified` row + revoke pending row + swap cookie)
/// runs through `auth::sessions::invalidate_sessions` at the
/// handler level — not here.
#[allow(dead_code)] // call site lands at the verify POST handler in a later commit
pub async fn verify_totp_for_user(
    db: &Db,
    user_id: i64,
    candidate_code_str: &str,
    step_seconds: u64,
    skew_steps: u32,
    key: &MfaKey,
) -> Result<VerifyOutcome> {
    use sqlx::Row as _;

    // 1. Parse the candidate as a 6-digit u32.
    let candidate = match candidate_code_str.trim().parse::<u32>() {
        Ok(n) if n < 1_000_000 => n,
        _ => return Ok(VerifyOutcome::Invalid),
    };

    // 2. Read MFA state from the user row.
    let row = sqlx::query(
        "SELECT mfa_enabled, mfa_secret_ciphertext, mfa_last_used_step \
         FROM rustio_users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("user {user_id} not found")))?;

    let mfa_enabled: bool = row.try_get("mfa_enabled")?;
    if !mfa_enabled {
        return Ok(VerifyOutcome::NotEnrolled);
    }

    let ciphertext: Option<Vec<u8>> = row.try_get("mfa_secret_ciphertext")?;
    let last_used_step: Option<i64> = row.try_get("mfa_last_used_step")?;

    let ciphertext = ciphertext.ok_or_else(|| {
        Error::Internal(format!(
            "user {user_id} has mfa_enabled=TRUE but mfa_secret_ciphertext IS NULL"
        ))
    })?;

    // 3. Decrypt the secret. Failure here is operator-side:
    //    wrong key (rotation issue) or tampered ciphertext (DB
    //    attack). Both surface as Error::Internal; the user
    //    sees a uniform error response from the handler.
    let secret_bytes = unwrap_secret(&ciphertext, key)?;

    // 4. Verify the candidate against the secret + skew window.
    let now_unix = Utc::now().timestamp().max(0) as u64;
    let step = match verify_totp(&secret_bytes, candidate, now_unix, step_seconds, skew_steps) {
        Some(step) => step,
        None => return Ok(VerifyOutcome::Invalid),
    };

    // 5. D4 replay protection. mfa_last_used_step is monotonic
    //    per user; a TOTP code from a step ≤ the stored value
    //    is rejected even if it just verified cryptographically.
    //    A NULL last-used-step (theoretically unreachable after
    //    confirm_enrolment stamps it; included defensively) is
    //    treated as -1 so any non-negative step value passes
    //    the comparison.
    let last = last_used_step.unwrap_or(-1);
    if (step as i64) <= last {
        return Ok(VerifyOutcome::Replay {
            last_used_step: last.max(0) as u64,
        });
    }

    // 6. Stamp the new step. Subsequent verifications with the
    //    same step value (replay) will be rejected at step 5
    //    above.
    sqlx::query("UPDATE rustio_users SET mfa_last_used_step = $1 WHERE id = $2")
        .bind(step as i64)
        .bind(user_id)
        .execute(db.pool())
        .await?;

    Ok(VerifyOutcome::Verified { step_used: step })
}

// -----------------------------------------------------------------
// Backup-code consume runtime (R3 commit #8)
// -----------------------------------------------------------------

/// Outcome of [`consume_backup_code`]. Lets the verify handler
/// render the right page without embedding HTTP concerns in the
/// runtime layer.
///
/// All variants collapse to a uniform user-facing response per
/// `DESIGN_R3_MFA.md` §3.1 disclosure rules — the handler MUST
/// NOT distinguish `Invalid` from `AlreadyUsed` from
/// `NotEnrolled` in the rendered copy. The variant distinctions
/// exist for forensic logging and internal debugging only.
#[allow(dead_code)] // variants light up at the verify handler in a later commit
pub enum BackupConsumeOutcome {
    /// The candidate matched an unused backup code. The row has
    /// been atomically marked `used_at = NOW()`; the audit row
    /// has been emitted. The caller proceeds with trust
    /// escalation (mint fresh `mfa_verified` session row, revoke
    /// the pending row, swap the cookie).
    Consumed { code_id: i64, remaining: u32 },
    /// The candidate did not match any unused row, OR the input
    /// failed normalisation, OR a race against a parallel
    /// consume request lost. Uniform copy with `AlreadyUsed`
    /// per the disclosure rule.
    Invalid,
    /// The user row exists but `mfa_enabled = FALSE`. Should
    /// not happen if the verify handler checks state up-front,
    /// but the runtime refuses anyway to keep the contract
    /// honest.
    NotEnrolled,
    /// Reserved for the case where the SELECT widens beyond
    /// `WHERE used_at IS NULL`. The current SELECT filters at
    /// the index level so this variant is unreachable; it is
    /// retained for forward-compatibility per
    /// `DESIGN_R3_MFA.md` §9.2.
    #[allow(dead_code)]
    AlreadyUsed,
}

/// Consume a backup code as the second factor on the verify
/// flow.
///
/// **Inputs.**
///
/// - `request` — for client-IP capture into the audit row.
/// - `user_id` — the user being challenged.
/// - `candidate_str` — the raw input the user typed. Hyphen
///   and casing are normalised via
///   [`normalise_backup_code`] before hash compare.
/// - `via` — caller context (`"login"` or `"reauth"`)
///   recorded into `metadata.via` per §8.2.
/// - `correlation_id` — forensic-chain anchor.
///
/// **Steps.**
///
/// 1. Normalise the candidate. Empty after normalisation →
///    `Invalid`.
/// 2. SELECT `mfa_enabled` from `rustio_users`. Missing row →
///    `Error::NotFound`. `mfa_enabled = FALSE` → `NotEnrolled`.
/// 3. SELECT `id, code_hash` from `rustio_mfa_backup_codes`
///    WHERE `user_id = ? AND used_at IS NULL`. The partial
///    index makes this an index seek scoped to ≤
///    `BACKUP_CODE_COUNT` rows.
/// 4. **Constant-time iteration** over the rows. Argon2id
///    verify each candidate; do NOT break on first match. The
///    matched id is recorded once; subsequent matches (cannot
///    happen given fresh-salt-per-row) are ignored. Iterating
///    every row prevents a timing leak about the matching
///    index.
/// 5. No match → `Invalid`.
/// 6. **Atomic single-use UPDATE.** `UPDATE … SET used_at =
///    NOW() WHERE id = $1 AND used_at IS NULL`. If
///    `rows_affected = 0`, another concurrent request consumed
///    the same code first; treated as `Invalid` for uniform
///    user-facing response (D7 protected at the SQL level).
/// 7. Count remaining unused codes for the audit metadata +
///    caller's render decision (the handler may flash a
///    "regenerate" warning when `remaining ≤ 2`).
/// 8. Emit `AuditEvent::MfaCodeConsumed` with metadata
///    `{ code_id, remaining_codes, via }`.
///
/// **Doctrine 22.** This function does not write `revoked_at`.
/// The trust escalation that follows on `Consumed` runs
/// through `auth::sessions::invalidate_sessions` at the
/// handler level — not here.
///
/// **Audit row emits inside the function.** Unlike
/// [`verify_totp_for_user`] which is silent (TOTP success is
/// captured via session-promotion lineage), backup-code
/// consume is an out-of-band recovery event worth surfacing in
/// the forensic chain.
#[allow(dead_code)] // call site lands at the verify POST handler in a later commit
pub async fn consume_backup_code(
    db: &Db,
    request: &Request,
    user_id: i64,
    candidate_str: &str,
    via: &'static str,
    correlation_id: Option<&str>,
) -> Result<BackupConsumeOutcome> {
    use sqlx::Row as _;

    // 1. Normalise.
    let candidate = normalise_backup_code(candidate_str);
    if candidate.is_empty() {
        return Ok(BackupConsumeOutcome::Invalid);
    }

    // 2. Verify enrolment.
    let mfa_enabled: Option<bool> =
        sqlx::query_scalar("SELECT mfa_enabled FROM rustio_users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(db.pool())
            .await?;
    let mfa_enabled =
        mfa_enabled.ok_or_else(|| Error::NotFound(format!("user {user_id} not found")))?;
    if !mfa_enabled {
        return Ok(BackupConsumeOutcome::NotEnrolled);
    }

    // 3. SELECT all unused candidates.
    let rows = sqlx::query(
        "SELECT id, code_hash FROM rustio_mfa_backup_codes \
         WHERE user_id = $1 AND used_at IS NULL \
         ORDER BY id",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;

    // 4. Constant-time iteration. Verify against every row even
    //    after a match; record the first matched id only. Per
    //    §4.4: do not break on first match — leaks timing about
    //    candidate ordering otherwise.
    let mut matched_id: Option<i64> = None;
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        let hash: String = row.try_get("code_hash")?;
        if verify_backup_code(&candidate, &hash) && matched_id.is_none() {
            matched_id = Some(id);
        }
    }

    let Some(matched_id) = matched_id else {
        return Ok(BackupConsumeOutcome::Invalid);
    };

    // 5. Atomic single-use UPDATE. The `AND used_at IS NULL`
    //    clause guards against a parallel consume of the same
    //    code: only one of two concurrent requests sees
    //    rows_affected = 1; the loser collapses to Invalid for
    //    uniform user-facing response.
    let result = sqlx::query(
        "UPDATE rustio_mfa_backup_codes \
         SET used_at = NOW() \
         WHERE id = $1 AND used_at IS NULL",
    )
    .bind(matched_id)
    .execute(db.pool())
    .await?;

    if result.rows_affected() == 0 {
        return Ok(BackupConsumeOutcome::Invalid);
    }

    // 6. Count remaining unused codes for the metadata.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rustio_mfa_backup_codes \
         WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;
    let remaining = remaining.max(0) as u32;

    // 7. Emit AuditEvent::MfaCodeConsumed.
    let metadata = serde_json::json!({
        "code_id": matched_id,
        "remaining_codes": remaining,
        "via": via,
    });
    let ip = client_ip(request);
    let mut entry = LogEntry::new(user_id, ActionType::Update, "user", user_id)
        .with_event(AuditEvent::MfaCodeConsumed)
        .with_actor(user_id);
    entry.correlation_id = correlation_id;
    entry.ip_address = ip.as_deref();
    entry.metadata = Some(metadata);
    entry.summary = format!("backup code consumed via {via}; {remaining} remaining");
    audit_record(db, entry).await?;

    Ok(BackupConsumeOutcome::Consumed {
        code_id: matched_id,
        remaining,
    })
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

    // ---- backup codes (R3 commit #4) -----------------------------

    #[test]
    fn alphabet_is_31_chars_no_ambiguous() {
        // The 31-char alphabet excludes 0/O/1/I/L per
        // DESIGN_R3_MFA.md Appendix B. This test pins the
        // alphabet — if a future commit accidentally adds an
        // ambiguous character, the suite catches it.
        assert_eq!(BACKUP_CODE_ALPHABET.len(), 31);
        for &b in BACKUP_CODE_ALPHABET {
            let c = b as char;
            assert!(c.is_ascii_alphanumeric(), "non-alphanumeric: {c:?}");
            assert!(
                !matches!(c, '0' | 'O' | '1' | 'I' | 'L'),
                "ambiguous char in alphabet: {c:?}"
            );
        }
    }

    #[test]
    fn generate_returns_count_codes() {
        let codes = generate_backup_codes(BACKUP_CODE_COUNT);
        assert_eq!(codes.len(), BACKUP_CODE_COUNT);
    }

    #[test]
    fn each_code_is_xxxx_dash_xxxx_shape() {
        let codes = generate_backup_codes(8);
        for code in &codes {
            assert_eq!(code.len(), BACKUP_CODE_LEN + 1, "wrong length: {code:?}");
            assert_eq!(
                code.chars().nth(4),
                Some('-'),
                "hyphen missing at position 4: {code:?}"
            );
            // Every non-hyphen char is in the locked alphabet.
            for (i, c) in code.chars().enumerate() {
                if i == 4 {
                    continue;
                }
                assert!(
                    BACKUP_CODE_ALPHABET.contains(&(c as u8)),
                    "char {c:?} at position {i} not in alphabet"
                );
            }
        }
    }

    #[test]
    fn generated_codes_are_unique_within_batch() {
        // Birthday-bound for 8 codes from a 31^8 ≈ 9 × 10^11
        // space is well below the collision threshold. A repeated
        // code in a single batch would indicate the RNG is broken
        // or the alphabet is much smaller than expected.
        let codes = generate_backup_codes(64);
        let unique: std::collections::HashSet<_> = codes.iter().cloned().collect();
        assert_eq!(unique.len(), 64, "batch contained duplicates");
    }

    #[test]
    fn normalise_strips_hyphens_and_uppercases() {
        assert_eq!(normalise_backup_code("ABCD-EFGH"), "ABCDEFGH");
        assert_eq!(normalise_backup_code("abcd-efgh"), "ABCDEFGH");
        assert_eq!(normalise_backup_code("AbCdEfGh"), "ABCDEFGH");
        assert_eq!(normalise_backup_code(" abcd efgh "), "ABCDEFGH");
        assert_eq!(normalise_backup_code("abcdefgh"), "ABCDEFGH");
    }

    #[test]
    fn normalise_is_idempotent() {
        let once = normalise_backup_code("xxxx-yyyy");
        let twice = normalise_backup_code(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn hash_verify_round_trip() {
        let code = "ABCDEFGH";
        let hash = hash_backup_code(code).expect("hashing must succeed");
        assert!(verify_backup_code(code, &hash), "round-trip must verify");
    }

    #[test]
    fn hash_uses_argon2id_low_memory_params() {
        // Argon2's PHC string carries the params; this test pins
        // them so a future "let's tune Argon2" change either
        // updates the locked-decision table OR fails the suite
        // here.
        let hash = hash_backup_code("ABCDEFGH").expect("hash succeeds");
        assert!(hash.starts_with("$argon2id$"), "wrong algorithm: {hash}");
        assert!(
            hash.contains("m=16384,t=2,p=1"),
            "params drifted from locked m=16MB/t=2/p=1: {hash}"
        );
    }

    #[test]
    fn verify_rejects_wrong_code() {
        let hash = hash_backup_code("ABCDEFGH").expect("hash succeeds");
        assert!(!verify_backup_code("WRONGCDE", &hash));
    }

    #[test]
    fn verify_rejects_invalid_phc_string() {
        // Garbage hash must not panic — must return false.
        assert!(!verify_backup_code("ABCDEFGH", "not-a-phc-hash"));
        assert!(!verify_backup_code("ABCDEFGH", ""));
    }

    #[test]
    fn separate_hash_calls_yield_different_phc_strings() {
        // Fresh salt per call ⇒ same plaintext hashes differently.
        // Without this, an attacker who learns one hash trivially
        // recognises whether two users share the same code.
        let a = hash_backup_code("ABCDEFGH").expect("a");
        let b = hash_backup_code("ABCDEFGH").expect("b");
        assert_ne!(a, b, "fresh salt must produce different hashes");
        // But both still verify the original code.
        assert!(verify_backup_code("ABCDEFGH", &a));
        assert!(verify_backup_code("ABCDEFGH", &b));
    }

    // ---- TOTP RFC 6238 (R3 commit #5) ----------------------------

    /// RFC 6238 Appendix B test secret (20 ASCII bytes).
    const RFC6238_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn current_step_at_canonical_30s_interval() {
        // T=0 → step 0; T=29 → step 0; T=30 → step 1; T=59 → step 1;
        // T=60 → step 2.
        assert_eq!(current_step(0, 30), 0);
        assert_eq!(current_step(29, 30), 0);
        assert_eq!(current_step(30, 30), 1);
        assert_eq!(current_step(59, 30), 1);
        assert_eq!(current_step(60, 30), 2);
    }

    #[test]
    fn rfc6238_appendix_b_test_vectors_truncated_to_6_digits() {
        // The RFC 6238 Appendix B vectors are 8-digit codes.
        // Authenticator apps render 6 digits by default, so the
        // framework's generate_totp returns the 6-digit form
        // (the last 6 digits of the 8-digit RFC value, since
        // truncation is `bin_code % 10^digits`).
        //
        // Source: RFC 6238 Appendix B, "TOTP Algorithm: Test
        // Vectors", SHA-1 column.
        //
        //  T (sec)        | 8-digit (RFC)  | 6-digit (this fn)
        //  ---------------+----------------+------------------
        //  59             | 94287082       | 287082
        //  1111111109     | 07081804       |  81804
        //  1111111111     | 14050471       |  50471
        //  1234567890     | 89005924       |   5924
        //  2000000000     | 69279037       | 279037
        //  20000000000    | 65353130       | 353130
        let cases: &[(u64, u32)] = &[
            (59, 287_082),
            (1_111_111_109, 81_804),
            (1_111_111_111, 50_471),
            (1_234_567_890, 5_924),
            (2_000_000_000, 279_037),
            (20_000_000_000, 353_130),
        ];

        for &(t, expected) in cases {
            let step = current_step(t, 30);
            let got = generate_totp(RFC6238_SECRET, step);
            assert_eq!(got, expected, "RFC 6238 vector at T={t} mismatched");
        }
    }

    #[test]
    fn generate_totp_returns_six_digit_range() {
        // Across a sample of steps, the result must fit in
        // [0, 999_999] — the modulo guarantees this but a future
        // refactor could lose the modulo silently.
        for step in [0u64, 1, 100, 12_345, u64::MAX] {
            let code = generate_totp(RFC6238_SECRET, step);
            assert!(
                code < 1_000_000,
                "code out of range for step {step}: {code}"
            );
        }
    }

    #[test]
    fn verify_accepts_current_step() {
        let t = 1_111_111_111u64;
        let step = current_step(t, 30);
        let code = generate_totp(RFC6238_SECRET, step);
        assert_eq!(verify_totp(RFC6238_SECRET, code, t, 30, 1), Some(step));
    }

    #[test]
    fn verify_accepts_one_step_skew() {
        // Generate at step S, verify at step S+1's wall-clock
        // (T += step_seconds). With skew=1, the previous step
        // is still accepted.
        let t_gen = 1_111_111_111u64;
        let step_gen = current_step(t_gen, 30);
        let code = generate_totp(RFC6238_SECRET, step_gen);

        let t_verify = t_gen + 30; // one step later
        let result = verify_totp(RFC6238_SECRET, code, t_verify, 30, 1);
        assert_eq!(result, Some(step_gen), "skew ±1 must accept previous step");
    }

    #[test]
    fn verify_rejects_two_step_skew_when_window_is_one() {
        // Generate at step S, verify at step S+2's wall-clock
        // with skew=1. Falls outside the [S+1, S+3] acceptance
        // window seen from T=S+2.
        let t_gen = 1_111_111_111u64;
        let step_gen = current_step(t_gen, 30);
        let code = generate_totp(RFC6238_SECRET, step_gen);

        let t_verify = t_gen + 60; // two steps later
        let result = verify_totp(RFC6238_SECRET, code, t_verify, 30, 1);
        assert_eq!(result, None, "skew=1 must reject two-step drift");
    }

    #[test]
    fn totp_verify_rejects_wrong_code() {
        let t = 1_111_111_111u64;
        let result = verify_totp(RFC6238_SECRET, 999_999, t, 30, 1);
        assert_eq!(result, None);
    }

    #[test]
    fn verify_does_not_underflow_at_t_zero() {
        // Skew window at T=0 would mathematically include step
        // -1; the saturating_add().max(0) guard maps it to step
        // 0, so the verify just retries step 0 instead of
        // panicking on integer underflow.
        let code = generate_totp(RFC6238_SECRET, 0);
        let result = verify_totp(RFC6238_SECRET, code, 0, 30, 1);
        assert_eq!(result, Some(0));
    }

    // ---- enrolment runtime (R3 commit #6) ------------------------

    #[test]
    fn base32_rfc4648_test_vector_foobar() {
        // RFC 4648 §10 standard test vector. Pins the encoder
        // against the canonical reference; if the alphabet or
        // bit-packing drifts, this test fails.
        assert_eq!(base32_encode_no_pad(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn base32_rfc4648_progressive_test_vectors() {
        // Additional RFC 4648 §10 vectors covering 1, 2, 3, 4,
        // 5 input bytes — exercises every path through the
        // bit-packing loop's leftover-bits flush.
        // (Outputs are the no-padding form; standard test
        // vectors include `=` padding which we strip per the
        // otpauth:// URL convention.)
        assert_eq!(base32_encode_no_pad(b"f"), "MY");
        assert_eq!(base32_encode_no_pad(b"fo"), "MZXQ");
        assert_eq!(base32_encode_no_pad(b"foo"), "MZXW6");
        assert_eq!(base32_encode_no_pad(b"foob"), "MZXW6YQ");
        assert_eq!(base32_encode_no_pad(b"fooba"), "MZXW6YTB");
    }

    #[test]
    fn provision_secret_returns_20_bytes() {
        let secret = provision_secret();
        assert_eq!(
            secret.secret_bytes.len(),
            20,
            "RFC 6238 default + universal authenticator-app interop"
        );
    }

    #[test]
    fn provision_secret_base32_length_matches_secret() {
        // 20 bytes × 8 bits = 160 bits / 5 bits per base32 char
        // = 32 chars exactly (no padding needed).
        let secret = provision_secret();
        assert_eq!(secret.base32.len(), 32);
        // Every char is in the base32 alphabet.
        for c in secret.base32.chars() {
            assert!(
                c.is_ascii_uppercase() || ('2'..='7').contains(&c),
                "non-base32 char in encoding: {c:?}"
            );
        }
    }

    #[test]
    fn provision_secret_each_call_yields_different_secret() {
        // Birthday-bound for 20-byte secrets is astronomical;
        // a collision in 16 calls indicates the RNG is broken.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..16 {
            let secret = provision_secret();
            assert!(seen.insert(secret.secret_bytes), "RNG produced duplicate");
        }
    }
}
