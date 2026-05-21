//! Per-request negotiated locale.
//!
//! Reads the inbound `Accept-Language` header, picks the operator's
//! first preferred language tag, and stashes it in the request
//! context so future code (the upcoming message-catalog surface;
//! see `ROADMAP.md` § Internationalization & RTL) can render
//! framework strings in that locale.
//!
//! ## Scope of v1
//!
//! This middleware is the *foundation* for locale-aware rendering.
//! It does not translate anything yet — there is no message catalog
//! shipped today. Adding `.middleware(middleware::locale)` to a
//! project's router is free: it parses the header, stashes the
//! result, and otherwise leaves rendering unchanged. When the
//! catalog lands, templates / handlers read the locale via
//! `req.ctx().get::<Locale>()` and pick the matching message.
//!
//! ## Placement in the middleware chain
//!
//! Order is irrelevant for locale negotiation — the value is read
//! from the inbound header, not produced by any other middleware,
//! and not consumed until a handler renders. Drop it anywhere
//! after `logger` for cleanliness. The framework does not require
//! this middleware to be registered; project apps that never
//! consume the locale are free to leave it out.
//!
//! ## Parser scope
//!
//! [`parse_accept_language`] returns the **first** valid language
//! tag in the header. Real `Accept-Language` headers carry quality
//! values (`en-US,en;q=0.9,fr;q=0.8`) and the RFC 9110 negotiation
//! rule sorts by `q` descending. The simplification is intentional:
//! every browser sends the preferred locale **first** in practice,
//! so first-wins matches client intent without the cost of a
//! sorted-vector traversal on every request. When the message
//! catalog ships, we can revisit if data shows real clients hitting
//! the edge case.
//!
//! Tag shape is validated lightly — ASCII letters, digits, and `-`
//! only, length-capped at 16 characters — so a malicious sender
//! can't smuggle control bytes into a downstream consumer that
//! later interpolates the value into a path or filename.
//!
//! ## Reading the locale from a handler
//!
//! ```ignore
//! use rustio_admin::middleware::Locale;
//!
//! let locale = req
//!     .ctx()
//!     .get::<Locale>()
//!     .map(|l| l.as_str().to_string())
//!     .unwrap_or_else(|| "en".to_string());
//! ```

use crate::error::Result;
use crate::http::{Request, Response};
use crate::router::Next;

/// Header the middleware reads. Lower-case matches HTTP/2 wire
/// format and what every browser sends in practice.
const ACCEPT_LANGUAGE_HEADER: &str = "accept-language";

/// Fallback locale when no header is sent, the header parses to
/// nothing valid, or the project app didn't mount the middleware.
/// `"en"` matches the framework's existing default strings (every
/// label and button is currently English).
pub const DEFAULT_LOCALE: &str = "en";

/// Cap on the locale tag length kept after parsing. Real tags fit
/// well under this (`en-US-x-foo-bar` is 15); the limit defends
/// downstream consumers from adversarial inputs without rejecting
/// any realistic locale.
const MAX_TAG_LEN: usize = 16;

// public:
/// Wrapper carried in the request context so handlers can pull
/// the negotiated locale out via `req.ctx().get::<Locale>()`. The
/// inner string is the tag as it appeared in `Accept-Language`
/// (e.g. `"en"`, `"en-US"`, `"ar-EG"`), already validated.
#[derive(Debug, Clone)]
pub struct Locale(pub String);

impl Locale {
    // public:
    /// Borrow the underlying tag string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// public:
/// Middleware: parse the inbound `Accept-Language` header and
/// stash the negotiated locale in the request context. Falls back
/// to [`DEFAULT_LOCALE`] when the header is absent or contains no
/// valid tag. Never fails — locale negotiation is best-effort by
/// design, and a missing or malformed header collapses to the
/// default rather than rejecting the request.
pub async fn locale(mut req: Request, next: Next) -> Result<Response> {
    let tag = parse_accept_language(req.header(ACCEPT_LANGUAGE_HEADER))
        .unwrap_or_else(|| DEFAULT_LOCALE.to_string());
    req.ctx_mut().insert(Locale(tag));
    next.run(req).await
}

/// Parse an `Accept-Language` header value, returning the first
/// language tag that passes the validator. Pulled out as a free
/// function so the negotiation policy is unit-testable without a
/// `Request`.
///
/// Returns:
///
/// - `Some(tag)` when the header contains at least one valid tag.
///   Validation: ASCII letters / digits / hyphens only, length
///   1..=[`MAX_TAG_LEN`].
/// - `None` when the header is missing, empty after trimming, or
///   contains no valid tag.
pub fn parse_accept_language(header: Option<&str>) -> Option<String> {
    let raw = header?.trim();
    if raw.is_empty() {
        return None;
    }
    for segment in raw.split(',') {
        // Strip the `;q=…` weight if present; we don't sort by
        // q-value (see module docs for the rationale).
        let tag = segment.split(';').next().unwrap_or(segment).trim();
        if is_valid_tag(tag) {
            return Some(tag.to_string());
        }
    }
    None
}

/// `true` when `tag` looks like a well-formed language tag.
/// Conservative — rejects empty strings, oversized inputs, and
/// anything outside ASCII letters / digits / hyphens.
fn is_valid_tag(tag: &str) -> bool {
    if tag.is_empty() || tag.len() > MAX_TAG_LEN {
        return false;
    }
    tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_header_returns_none() {
        assert!(parse_accept_language(None).is_none());
    }

    #[test]
    fn empty_header_returns_none() {
        assert!(parse_accept_language(Some("")).is_none());
        assert!(parse_accept_language(Some("   ")).is_none());
    }

    #[test]
    fn single_tag_returned_as_is() {
        assert_eq!(parse_accept_language(Some("en")), Some("en".to_string()));
        assert_eq!(
            parse_accept_language(Some("en-US")),
            Some("en-US".to_string())
        );
    }

    #[test]
    fn first_tag_wins_in_comma_separated_list() {
        // Realistic browser header — preferred locale comes first.
        let header = "en-US,en;q=0.9,fr;q=0.8,*;q=0.5";
        assert_eq!(
            parse_accept_language(Some(header)),
            Some("en-US".to_string())
        );
    }

    #[test]
    fn q_value_is_stripped_from_the_tag() {
        // The `;q=N` weight must not bleed into the stored tag.
        assert_eq!(
            parse_accept_language(Some("ar;q=0.9")),
            Some("ar".to_string())
        );
    }

    #[test]
    fn invalid_first_tag_falls_through_to_next() {
        // `*` is a wildcard, not a tag; a tag with control bytes
        // is rejected. The parser skips both and finds `de`.
        let header = "*,en\nbad,de;q=0.8";
        assert_eq!(parse_accept_language(Some(header)), Some("de".to_string()));
    }

    #[test]
    fn oversized_tag_is_rejected() {
        // A pathological header that would smuggle in a long
        // attacker-controlled string. Must collapse to None when
        // it is the only segment.
        let evil = "a".repeat(50);
        assert!(parse_accept_language(Some(&evil)).is_none());
    }

    #[test]
    fn header_with_only_invalid_tags_returns_none() {
        assert!(parse_accept_language(Some("*,?,!!!")).is_none());
        assert!(parse_accept_language(Some(",,,")).is_none());
    }

    #[test]
    fn whitespace_between_tags_is_tolerated() {
        // RFC permits optional whitespace after commas;
        // real-world Firefox + Safari both emit it.
        assert_eq!(
            parse_accept_language(Some("en-US, en;q=0.9, fr;q=0.8")),
            Some("en-US".to_string())
        );
    }

    #[test]
    fn arabic_and_cjk_tags_are_accepted() {
        // The framework already self-hosts Tajawal + Noto Naskh
        // Arabic; the locale middleware must accept the tag those
        // fonts target.
        assert_eq!(
            parse_accept_language(Some("ar-EG,ar;q=0.9,en;q=0.6")),
            Some("ar-EG".to_string())
        );
        assert_eq!(
            parse_accept_language(Some("zh-Hans-CN,zh;q=0.9")),
            Some("zh-Hans-CN".to_string())
        );
    }

    #[test]
    fn locale_as_str_round_trips() {
        let l = Locale("en-US".to_string());
        assert_eq!(l.as_str(), "en-US");
    }
}
