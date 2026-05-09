//! Self-service password recovery (R1).
//!
//! See `DESIGN_RECOVERY.md` for the canonical contract this module
//! implements. R1 ships in 0.5.0; this commit lands the schema and
//! the [`PasswordPolicy`] trait surface — the [`RecoveryPolicy`]
//! trait, the issue + consume flow, the mailer wiring, the routes,
//! and the templates land in subsequent atomic commits per
//! `DESIGN_RECOVERY.md` §16.
//!
//! ## What lives here today
//!
//! - [`init_recovery_tables`] — creates `rustio_password_reset_tokens`
//!   with the partial unique index that makes the consume path's
//!   atomic `UPDATE … RETURNING` an index seek
//!   (`DESIGN_RECOVERY.md` §9.1).
//! - [`migrate_user_recovery_schema`] — adds the additive
//!   `must_change_password` and `password_changed_at` columns on
//!   `rustio_users` (§9.2). R1's `set_password` populates
//!   `password_changed_at`; R2 enforces `must_change_password`.
//! - [`PasswordPolicy`] / [`DefaultPasswordPolicy`] /
//!   [`PasswordPolicyError`] / [`SharedPasswordPolicy`] — the
//!   policy surface (§13). `Admin::password_policy(...)` lives in
//!   `admin::types`; the trait and default impl live here so the
//!   recovery module owns its vocabulary.
//!
//! The migration functions are idempotent and safe to call on every
//! boot. `auth::init_tables` invokes them after the existing user /
//! session migrations. The policy surface is data-only at this
//! commit; no handler reads it yet.

use std::sync::Arc;

use crate::error::Result;
use crate::orm::Db;

/// Create the `rustio_password_reset_tokens` table and its indexes.
///
/// Schema (see `DESIGN_RECOVERY.md` §9.1 for the contract):
///
/// - `token_hash` is `sha256(token)` URL-safe-base64 — the plaintext
///   token never lands in this row.
/// - `mail_status` is one of `'pending' | 'sent' | 'failed'`; the state
///   evolves in the issue handler (one row per request).
/// - `correlation_id` mirrors the request's audit `correlation_id` so
///   an operator can pivot from token row → audit chain.
/// - The partial unique index `WHERE consumed_at IS NULL` is the index
///   the atomic consume statement seeks on.
///
/// Idempotent. Safe to call on every boot. Depends on `rustio_users`
/// existing first.
pub(crate) async fn init_recovery_tables(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_password_reset_tokens (
            id                    BIGSERIAL   PRIMARY KEY,
            user_id               BIGINT      NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
            token_hash            TEXT        NOT NULL,
            requested_ip          TEXT,
            requested_user_agent  TEXT,
            requested_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            expires_at            TIMESTAMPTZ NOT NULL,
            consumed_at           TIMESTAMPTZ,
            mail_status           TEXT        NOT NULL DEFAULT 'pending'
                                  CHECK (mail_status IN ('pending', 'sent', 'failed')),
            correlation_id        TEXT
        )",
    )
    .execute(db.pool())
    .await?;

    // Partial unique on the active-token lookup. Guarantees the
    // consume statement (`UPDATE … WHERE token_hash = $1 AND
    // consumed_at IS NULL RETURNING …`) is an index seek even after
    // the table accumulates consumed/expired rows for forensic
    // retention.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS rustio_password_reset_tokens_active_uq \
         ON rustio_password_reset_tokens (token_hash) \
         WHERE consumed_at IS NULL",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_user_idx \
         ON rustio_password_reset_tokens (user_id)",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_expires_idx \
         ON rustio_password_reset_tokens (expires_at) \
         WHERE consumed_at IS NULL",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

/// Add the additive recovery columns on `rustio_users`.
///
/// - `must_change_password BOOLEAN NOT NULL DEFAULT FALSE` — R2 will
///   read this on login to force a password reset on the next sign-in.
///   R1 introduces the column because R2's commit set stays narrower
///   when the column already exists.
/// - `password_changed_at TIMESTAMPTZ` (nullable) — populated by
///   `auth::set_password` from R1 onwards. NULL for users created
///   before the upgrade; the active-sessions UI renders "(unknown)" or
///   omits the row when NULL.
///
/// Idempotent. Safe to call on every boot. Depends on `rustio_users`
/// existing first.
pub(crate) async fn migrate_user_recovery_schema(db: &Db) -> Result<()> {
    sqlx::query(
        "ALTER TABLE rustio_users \
         ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS password_changed_at TIMESTAMPTZ")
        .execute(db.pool())
        .await?;

    Ok(())
}

// ---- Password policy -------------------------------------------------------

/// Validates a candidate password against project-defined rules.
///
/// The framework ships [`DefaultPasswordPolicy`] (length-only floor)
/// as the secure-by-default baseline. Projects layer a stronger
/// policy via [`crate::admin::Admin::password_policy`] when
/// regulation or risk requires it. The trait is `Send + Sync` so the
/// `Arc<dyn PasswordPolicy>` lives on `Admin` and is cheap to clone
/// into async futures.
///
/// ## Implementing a custom policy
///
/// ```ignore
/// use rustio_admin::auth::{PasswordPolicy, PasswordPolicyError};
///
/// struct OrgPolicy;
/// impl PasswordPolicy for OrgPolicy {
///     fn validate(&self, candidate: &str) -> Result<(), PasswordPolicyError> {
///         let len = candidate.chars().count();
///         if len < 16 {
///             return Err(PasswordPolicyError::TooShort { min: 16, actual: len });
///         }
///         if !candidate.chars().any(|c| c.is_ascii_digit()) {
///             return Err(PasswordPolicyError::Custom(
///                 "Password must contain at least one digit.".into(),
///             ));
///         }
///         Ok(())
///     }
///     fn min_length(&self) -> usize { 16 }
/// }
/// ```
///
/// Implementations MUST treat the borrowed candidate as a secret:
/// no logging, no panic-with-the-plaintext, no inclusion in the
/// returned error. The framework's audit + log helpers redact
/// passwords (`audit::redact_password()`); custom policies that
/// want to surface a project-specific message use
/// [`PasswordPolicyError::Custom`] with a user-safe string.
pub trait PasswordPolicy: Send + Sync {
    /// Approve or reject the candidate.
    fn validate(&self, candidate: &str) -> std::result::Result<(), PasswordPolicyError>;

    /// The minimum length the policy enforces, in Unicode `char`s.
    /// Templates display this on the new-password form so users see
    /// the floor before submitting.
    fn min_length(&self) -> usize;
}

/// Type-erased shared password-policy reference, mirroring
/// [`crate::email::SharedMailer`]. The framework's `Admin` holds one
/// of these; defaults to `Arc::new(DefaultPasswordPolicy::new())`
/// until a project overrides via
/// `Admin::password_policy(Arc::new(...))`.
pub type SharedPasswordPolicy = Arc<dyn PasswordPolicy>;

/// Reasons a candidate password fails policy validation.
///
/// Variants intentionally omit the candidate plaintext — none of the
/// fields carry the rejected password, so a `Display` / `Debug`
/// rendering of any error value is safe to log, audit, or pass to a
/// form-field renderer. Project-supplied policies that emit
/// [`PasswordPolicyError::Custom`] are responsible for keeping their
/// message free of the plaintext as well.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PasswordPolicyError {
    /// Length floor not met. Both fields are character counts (not
    /// bytes), matching `min_length()`.
    TooShort { min: usize, actual: usize },
    /// Project-defined rejection. The string renders to the user
    /// verbatim and lands in logs verbatim — keep it free of secrets.
    Custom(String),
}

impl std::fmt::Display for PasswordPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { min, actual } => write!(
                f,
                "This password is too short. It must contain at least {min} characters \
                 (you entered {actual})."
            ),
            Self::Custom(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for PasswordPolicyError {}

/// Length-only password policy. Default `min_len` is **10** — the
/// secure-by-default baseline R1 ships with: long enough to defeat
/// trivial guessing under Argon2id + per-IP rate-limiting (NIST SP
/// 800-63B's recommended length floor is 8, with longer being
/// preferable), short enough not to drive operators toward sticky-
/// note workarounds. Production / regulated deployments are
/// encouraged to override to 12+ via
/// [`crate::admin::Admin::password_policy`]; high-sensitivity
/// deployments may want 16+ paired with an organisational
/// complexity rule or breach blocklist.
///
/// The framework deliberately ships **no complexity-class rules**
/// ("must contain a symbol", "must include uppercase") in the
/// default — they demonstrably push humans toward predictable
/// patterns without improving entropy meaningfully (NIST SP
/// 800-63B Appendix A). Projects that need them implement a
/// custom `PasswordPolicy`.
#[derive(Debug, Clone, Copy)]
pub struct DefaultPasswordPolicy {
    pub min_len: usize,
}

impl DefaultPasswordPolicy {
    /// New policy with the framework's default floor (`min_len = 10`).
    pub const fn new() -> Self {
        Self { min_len: 10 }
    }

    /// New policy with an explicit floor. Useful for projects that
    /// want a stronger length baseline without authoring a full
    /// `PasswordPolicy` impl.
    pub const fn with_min_len(min_len: usize) -> Self {
        Self { min_len }
    }
}

impl Default for DefaultPasswordPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordPolicy for DefaultPasswordPolicy {
    fn validate(&self, candidate: &str) -> std::result::Result<(), PasswordPolicyError> {
        // Count Unicode `char`s, not bytes — a 10-char password is
        // 10 user-visible characters regardless of UTF-8 byte width.
        // Grapheme-cluster counting is left to project policies that
        // need it.
        let actual = candidate.chars().count();
        if actual < self.min_len {
            return Err(PasswordPolicyError::TooShort {
                min: self.min_len,
                actual,
            });
        }
        Ok(())
    }

    fn min_length(&self) -> usize {
        self.min_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_floor_is_ten() {
        assert_eq!(DefaultPasswordPolicy::new().min_length(), 10);
        assert_eq!(DefaultPasswordPolicy::default().min_length(), 10);
    }

    #[test]
    fn default_policy_accepts_password_at_floor() {
        let p = DefaultPasswordPolicy::new();
        // Exactly 10 chars — the doctrine-locked default floor.
        assert!(p.validate("aaaaaaaaaa").is_ok());
        // Comfortable margin.
        assert!(p.validate("correct horse battery staple").is_ok());
    }

    #[test]
    fn default_policy_rejects_short_password() {
        let p = DefaultPasswordPolicy::new();
        let err = p.validate("nine_char").unwrap_err();
        assert_eq!(err, PasswordPolicyError::TooShort { min: 10, actual: 9 });
    }

    #[test]
    fn default_policy_rejects_empty_password() {
        let p = DefaultPasswordPolicy::new();
        let err = p.validate("").unwrap_err();
        assert_eq!(err, PasswordPolicyError::TooShort { min: 10, actual: 0 });
    }

    #[test]
    fn default_policy_with_min_len_overrides_floor() {
        let p = DefaultPasswordPolicy::with_min_len(16);
        assert_eq!(p.min_length(), 16);
        assert!(p.validate("fifteen_chars__").is_err()); // 15 chars
        assert!(p.validate("sixteen_chars___").is_ok()); //  16 chars
    }

    #[test]
    fn default_policy_counts_chars_not_bytes() {
        let p = DefaultPasswordPolicy::new();
        // 10 Cyrillic chars = 20 bytes. Char count passes the floor.
        let pw = "пароль1234";
        assert_eq!(pw.chars().count(), 10);
        assert!(pw.len() > 10);
        assert!(p.validate(pw).is_ok());

        // 9 Cyrillic chars must fail with the char count, not the
        // byte count.
        let pw = "пароль123";
        let err = p.validate(pw).unwrap_err();
        assert_eq!(err, PasswordPolicyError::TooShort { min: 10, actual: 9 });
    }

    #[test]
    fn error_renderings_do_not_leak_plaintext() {
        // Property: neither Display nor Debug formatting of a
        // policy error rendered for a rejected candidate leaks the
        // candidate string. Picked plaintext is unlikely to collide
        // with English words in the default error message.
        let p = DefaultPasswordPolicy::new();
        let plaintext = "Pwn4Ge#xy"; // 9 chars — fails the 10-char floor
        let err = p.validate(plaintext).unwrap_err();
        let display = format!("{err}");
        let debug = format!("{err:?}");
        assert!(
            !display.contains(plaintext),
            "Display leaked plaintext: {display}"
        );
        assert!(
            !debug.contains(plaintext),
            "Debug leaked plaintext: {debug}"
        );
    }

    #[test]
    fn custom_error_renders_message_verbatim() {
        let err = PasswordPolicyError::Custom("breached password rejected".into());
        assert_eq!(format!("{err}"), "breached password rejected");
    }

    #[test]
    fn shared_password_policy_is_send_sync() {
        // Compile-time guarantee that the trait-object alias retains
        // the bounds the framework relies on.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SharedPasswordPolicy>();
    }
}
