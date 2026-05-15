//! Canonical-form helpers for text the Builder emits or reads.
//!
//! Implements the environment fixings of `DESIGN_BUILDER.md` §4.4
//! that the deterministic core can enforce in-process:
//!
//! - #2 LF line endings.
//! - #3 UTF-8 NFC normalization.
//!
//! Fixings #1 (locale `LC_ALL=C`), #5 (ISO-8601 second-precision),
//! #6 (ULIDs), #7 (Builder version pin), and #8 (toolchain pin) are
//! enforced in their respective modules. Fixing #4 (canonical TOML
//! emitter) lives in [`crate::builder::toml_canon`].

use unicode_normalization::UnicodeNormalization;

/// Normalize a string to the Builder's canonical text form: UTF-8
/// NFC with LF line endings, no trailing whitespace on lines, no
/// trailing whitespace at end of file. Idempotent.
///
/// Use this on every byte sequence the Builder is about to write to
/// disk under `.rustio/` or `src/_generated/`.
pub(crate) fn canonicalize(input: &str) -> String {
    let nfc: String = input.nfc().collect();

    // Normalize line endings: CRLF / CR → LF.
    let mut out = String::with_capacity(nfc.len());
    let mut chars = nfc.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }

    // Strip trailing whitespace per line, then strip trailing
    // blank lines at EOF, then guarantee a single trailing LF.
    let lines: Vec<&str> = out.split('\n').collect();
    let mut trimmed: Vec<String> = lines
        .iter()
        .map(|line| line.trim_end_matches([' ', '\t']).to_string())
        .collect();
    while matches!(trimmed.last(), Some(line) if line.is_empty()) {
        trimmed.pop();
    }
    if trimmed.is_empty() {
        return String::new();
    }
    let mut result = trimmed.join("\n");
    result.push('\n');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_collapses_to_lf() {
        assert_eq!(canonicalize("a\r\nb\r\n"), "a\nb\n");
    }

    #[test]
    fn lone_cr_becomes_lf() {
        assert_eq!(canonicalize("a\rb"), "a\nb\n");
    }

    #[test]
    fn trailing_whitespace_stripped_per_line() {
        assert_eq!(canonicalize("hello   \nworld\t\n"), "hello\nworld\n");
    }

    #[test]
    fn final_newline_enforced() {
        assert_eq!(canonicalize("no trailing newline"), "no trailing newline\n");
    }

    #[test]
    fn trailing_blank_lines_collapsed() {
        assert_eq!(canonicalize("a\n\n\n\n"), "a\n");
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(canonicalize(""), "");
        assert_eq!(canonicalize("\n\n\n"), "");
    }

    #[test]
    fn nfc_normalizes_combining_marks() {
        // "é" as combining sequence (U+0065 U+0301) vs precomposed
        // (U+00E9). NFC collapses to the precomposed form.
        let combining = "e\u{0301}";
        let precomposed = "\u{00E9}";
        assert_eq!(canonicalize(combining), format!("{precomposed}\n"));
    }

    #[test]
    fn idempotent() {
        let once = canonicalize("a\r\nb\r\n  \n\n");
        let twice = canonicalize(&once);
        assert_eq!(once, twice);
    }
}
