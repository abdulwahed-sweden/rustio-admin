//! Sanitisation helpers for log lines, audit summaries, and error
//! messages.
//!
//! Doctrine 11: never log secrets. Recovery flows route every secret-
//! adjacent string through one of these helpers before it reaches the
//! audit trail or any log target. The functions return either a
//! fixed placeholder string (for genuinely opaque secrets like
//! passwords and MFA secrets) or a short fingerprint (for tokens that
//! benefit from being correlatable in support traffic without leaking
//! the full value).
//!
//! Adopted by:
//!
//! - `audit::record` summary text generation
//! - the upcoming `Mailer` debug logging path
//! - any handler that needs to format a status string mentioning a
//!   token or password
//!
//! If you find yourself wanting to log a secret directly, ask
//! whether the log line is more useful than the risk of disclosure.
//! In every case the framework has shipped so far, the answer is no.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

// public:
/// Replace any password-like value with a fixed placeholder. Use this
/// in summary strings and error messages — never the real password,
/// not even truncated.
pub const fn redact_password() -> &'static str {
    "<password>"
}

// public:
/// Render a short, privacy-preserving fingerprint of a token. The
/// returned string includes the first 8 chars of `sha256(token)` —
/// just enough for an operator to correlate two log lines about the
/// same token without disclosing it. Never reverses to the original.
///
/// Used for: session-cookie tokens, password-reset tokens, future
/// API keys.
pub fn redact_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let hex = URL_SAFE_NO_PAD.encode(digest);
    // 8 chars of the b64-encoded sha256 — ~48 bits of fingerprint.
    // Plenty for correlation; not reversible.
    let prefix: String = hex.chars().take(8).collect();
    format!("<token:…{prefix}>")
}

// public:
/// Replace an MFA secret with a fixed placeholder. MFA secrets are
/// always stored encrypted at rest; this helper exists so a stray
/// log statement during development can't accidentally write the
/// plaintext.
pub const fn redact_mfa_secret() -> &'static str {
    "<mfa-secret>"
}

// public:
/// Replace a backup code with a fixed placeholder. Codes are
/// short-lived single-use; like passwords, the right log line is
/// "redacted" with no fingerprint.
pub const fn redact_backup_code() -> &'static str {
    "<backup-code>"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_password_returns_fixed_placeholder() {
        assert_eq!(redact_password(), "<password>");
    }

    #[test]
    fn redact_token_is_deterministic() {
        let a = redact_token("abc123");
        let b = redact_token("abc123");
        assert_eq!(a, b);
    }

    #[test]
    fn redact_token_changes_per_token() {
        assert_ne!(redact_token("aaa"), redact_token("bbb"));
    }

    #[test]
    fn redact_token_reveals_no_substring_of_input() {
        // Property: the redacted form must not contain any 4-char
        // substring of the input. Catches accidental "show last N
        // chars" regressions.
        let plaintext = "secretSESSIONcookie";
        let r = redact_token(plaintext);
        for win in plaintext.as_bytes().windows(4) {
            let needle = std::str::from_utf8(win).unwrap();
            assert!(!r.contains(needle), "redaction leaked {needle:?}: {r}");
        }
    }

    #[test]
    fn redact_token_format_is_recognizable() {
        let r = redact_token("anything");
        assert!(r.starts_with("<token:…"));
        assert!(r.ends_with('>'));
        // 8-char fingerprint between the marker and closing >.
        assert_eq!(r.len(), "<token:…".len() + 8 + ">".len());
    }

    #[test]
    fn redact_mfa_secret_is_constant() {
        assert_eq!(redact_mfa_secret(), "<mfa-secret>");
    }

    #[test]
    fn redact_backup_code_is_constant() {
        assert_eq!(redact_backup_code(), "<backup-code>");
    }
}
