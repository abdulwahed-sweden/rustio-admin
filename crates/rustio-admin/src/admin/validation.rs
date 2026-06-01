//! Format validators for the `email` / `phone` field types.
//!
//! These back the `#[rustio(format = "email" | "phone")]` field
//! attribute the scaffolder emits: at rest both columns are plain
//! `TEXT`, so the only thing that distinguishes them from a `str`
//! field is the widget (`<input type="email">` / `"tel"`) and the
//! check applied in the derived `from_form`.
//!
//! The checks are deliberately **structural, not exhaustive** — full
//! RFC 5322 e-mail and E.164 phone validation are rabbit holes that
//! reject legitimate values. The goal is to catch obvious typos
//! (`alice` instead of `alice@example.com`, `call me` instead of a
//! number), not to be a spec-complete parser. Projects that need
//! stricter rules override `from_form` or add a `ModelAdmin`
//! validation hook.

/// `true` when `s` looks like an e-mail address: a non-empty local
/// part, a single `@`, and a domain that contains a dot and doesn't
/// start or end with one. Whitespace is rejected. This is a typo
/// guard, not RFC 5322.
pub fn is_valid_email(s: &str) -> bool {
    if s.chars().any(char::is_whitespace) {
        return false;
    }
    let mut parts = s.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        // Zero or more than one `@`.
        return false;
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

/// `true` when `s` looks like a phone number: between 7 and 15 digits
/// (the E.164 ceiling), with only digits and the common separators
/// `+ - ( ) space .` in between. This is a typo guard, not E.164.
pub fn is_valid_phone(s: &str) -> bool {
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '(' | ')' | ' ' | '.'))
    {
        return false;
    }
    let digits = s.chars().filter(char::is_ascii_digit).count();
    (7..=15).contains(&digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_accepts_typical_addresses() {
        assert!(is_valid_email("alice@example.com"));
        assert!(is_valid_email("a.b+tag@sub.example.co.uk"));
    }

    #[test]
    fn email_rejects_obvious_typos() {
        assert!(!is_valid_email("alice"));
        assert!(!is_valid_email("alice@"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("alice@example"));
        assert!(!is_valid_email("a@b@c.com"));
        assert!(!is_valid_email("alice @example.com"));
        assert!(!is_valid_email("alice@.com"));
        assert!(!is_valid_email("alice@example."));
        assert!(!is_valid_email(""));
    }

    #[test]
    fn phone_accepts_common_shapes() {
        assert!(is_valid_phone("+1 (555) 123-4567"));
        assert!(is_valid_phone("0701234567"));
        assert!(is_valid_phone("555.123.4567"));
    }

    #[test]
    fn phone_rejects_letters_and_bad_lengths() {
        assert!(!is_valid_phone("call me"));
        assert!(!is_valid_phone("123456")); // 6 digits, below floor
        assert!(!is_valid_phone("1234567890123456")); // 16 digits, above ceiling
        assert!(!is_valid_phone(""));
    }
}
