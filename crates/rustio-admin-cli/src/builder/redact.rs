//! The Builder's sole redactor (Doctrine B4).
//!
//! `DESIGN_BUILDER.md` §4.2.3 declares this module the only path by
//! which an `args` payload destined for `.rustio/history.jsonl` may
//! be constructed. The grep proof at `DESIGN_BUILDER.md` §10.5
//! forbids any other file under `crates/rustio-admin-cli/src/` from
//! defining a function named `redact*`.
//!
//! The discipline is symmetric to `DESIGN_AUDIT.md` §5.3 (Doctrine
//! 11). Plaintext secrets must never enter `history.jsonl`; the
//! redactor produces SHA-256 fingerprints in place of values.
//!
//! ## Closed category list (§4.2.3)
//!
//! 1. Default values for fields whose declared type is one of
//!    `password`, `secret`, `token`, `api_key`, `private_key`,
//!    `encryption_key`.
//! 2. Values where the source field carries `redact = true`.
//! 3. Strings the developer marks with the `@redact:` sigil at the
//!    CLI boundary.
//! 4. Environment-variable contents read via `--from-env <VAR>`.
//!
//! Categories 1 and 2 are enforced at the call site (the builder
//! commands that construct events). This module implements the
//! redaction primitive that those call sites use.
//!
//! ## Property
//!
//! `cli::redact::tests::no_4char_input_substring_leaks` asserts the
//! redactor never lets a 4-character contiguous substring of the
//! input appear in the redacted output. Matches the property the
//! framework's `admin::redact::redact_token` carries.

use sha2::{Digest, Sha256};

/// Fingerprint a secret value for inclusion in `history.jsonl`
/// payloads. Returns a stable, opaque string that cannot be
/// inverted to the original input.
///
/// Output format: `sha256:<first-16-hex-chars>...truncated`.
///
/// This is the sole function in the CLI crate authorised to emit a
/// secret-derived string. Doctrine B4 forbids any other code path
/// from constructing such a value.
pub(crate) fn redact_secret(plaintext: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("sha256:{hex}...truncated")
}

/// Field-type categories that always carry secret content. Default
/// values declared for fields of these types are passed through
/// [`redact_secret`] before reaching any event payload.
///
/// Closed list per `DESIGN_BUILDER.md` §4.2.3 #1. Adding a category
/// is a doctrine amendment, not an implementation choice.
pub(crate) const SECRET_FIELD_TYPES: &[&str] = &[
    "password",
    "secret",
    "token",
    "api_key",
    "private_key",
    "encryption_key",
];

/// Returns `true` if a declared field type is in the closed
/// secret-category list. Call sites that emit field defaults to the
/// event log gate the plaintext through this check.
pub(crate) fn is_secret_field_type(declared_type: &str) -> bool {
    SECRET_FIELD_TYPES.contains(&declared_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_is_deterministic() {
        assert_eq!(redact_secret("hunter2"), redact_secret("hunter2"));
    }

    #[test]
    fn redact_changes_with_input() {
        assert_ne!(redact_secret("hunter2"), redact_secret("hunter3"));
    }

    #[test]
    fn redact_format_matches_design() {
        let out = redact_secret("any-token-value");
        assert!(out.starts_with("sha256:"), "format prefix: {out}");
        assert!(out.ends_with("...truncated"), "format suffix: {out}");
        // sha256:<16 hex chars>...truncated  → 7 + 16 + 12 = 35
        assert_eq!(out.len(), 35);
    }

    /// Property test: no 4-character contiguous substring of the
    /// input appears in the redacted output. Symmetric to
    /// `DESIGN_AUDIT.md` §5.3 / Doctrine 11.
    #[test]
    fn no_4char_input_substring_leaks() {
        // Sample inputs covering ASCII secrets, multi-byte UTF-8,
        // newlines, and edge cases. Each input is checked against
        // every 4-char window of itself.
        let corpus = [
            "hunter2",
            "correct horse battery staple",
            "sk-live-1234567890abcdef",
            "Bearer eyJhbGciOiJIUzI1NiJ9",
            "اختبار-سري",         // Arabic
            "🔒-emoji-secret-🔑", // multi-byte
            "line\nwith\nnewlines",
            "tab\there\ttoo",
            "",    // empty edge case
            "a",   // shorter than window
            "abc", // shorter than window
        ];
        for input in corpus {
            let out = redact_secret(input);
            let chars: Vec<char> = input.chars().collect();
            for window in chars.windows(4) {
                let needle: String = window.iter().collect();
                assert!(
                    !out.contains(&needle),
                    "redacted output {out:?} leaks 4-char substring {needle:?} from input {input:?}",
                );
            }
        }
    }

    #[test]
    fn secret_field_categories_are_closed() {
        // Match the doctrine §4.2.3 #1 verbatim. A new category is a
        // doctrine amendment; if this test changes without a
        // matching DESIGN_BUILDER.md edit, the PR is doctrine drift.
        assert_eq!(
            SECRET_FIELD_TYPES,
            &[
                "password",
                "secret",
                "token",
                "api_key",
                "private_key",
                "encryption_key",
            ]
        );
    }

    #[test]
    fn is_secret_field_type_detects_category() {
        assert!(is_secret_field_type("password"));
        assert!(is_secret_field_type("api_key"));
        assert!(!is_secret_field_type("text"));
        assert!(!is_secret_field_type("integer"));
        assert!(!is_secret_field_type("Password")); // case-sensitive
    }
}
