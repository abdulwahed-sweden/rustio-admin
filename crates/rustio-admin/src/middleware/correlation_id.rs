//! Per-request correlation identifier.
//!
//! Doctrine 8: audit logs must be forensically useful. Every audit
//! row written under a single HTTP request shares one `correlation_id`
//! so a future `/admin/history/<id>` page can reconstruct the chain
//! ("admin reset password → all sessions revoked → security email
//! dispatched"). The id is also returned to clients in the
//! `x-correlation-id` response header so external consumers (proxy
//! logs, browser dev tools, support tickets) can pivot through the
//! same trail.
//!
//! ## Placement in the middleware chain
//!
//! This middleware **must** sit before [`csrf_protect`](super::csrf_protect)
//! so that even rejected requests (403 from CSRF, 429 from rate-limit)
//! are emitted with a correlation id. That is intentional: every
//! reasonable observability story for an admin panel needs the
//! "request 7f… was blocked by CSRF" trace, not just successful
//! requests.
//!
//! Recommended ordering inside `Router::middleware`:
//!
//! ```ignore
//! Router::new()
//!     .middleware(middleware::logger)
//!     .middleware(middleware::correlation_id)   // ← before csrf_protect
//!     .middleware(middleware::security_headers)
//!     .middleware(middleware::csrf_protect)
//! ```
//!
//! ## Reading the id from a handler
//!
//! ```ignore
//! use rustio_admin::middleware::CorrelationId;
//!
//! let cid = req.ctx().get::<CorrelationId>().map(|c| c.0.as_str());
//! ```

use uuid::Uuid;

use crate::error::Result;
use crate::http::{Request, Response};
use crate::router::Next;

// public:
/// Response + request header name. Lower-case to keep parity with the
/// HTTP/2 wire format and to match what other observability tooling
/// (OpenTelemetry, Cloudflare, etc.) writes.
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

// public:
/// Wrapper carried in the request context so handlers can pull it
/// out via `req.ctx().get::<CorrelationId>()`. The inner string is
/// the UUID rendered in hyphenated lowercase form.
#[derive(Debug, Clone)]
pub struct CorrelationId(pub String);

impl CorrelationId {
    // public:
    /// Borrow the underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// public:
/// Middleware: attach a UUID v7 to every request, surface it in the
/// response, and stash it in the request context for the audit
/// pipeline to pick up.
///
/// Honours an inbound `x-correlation-id` header so a proxy or test
/// harness can pin the id from outside. Only accepts values that
/// look like a UUID (rough sanity: between 16 and 64 chars, no
/// whitespace, no control bytes); anything else is replaced with a
/// fresh UUID v7 so a malicious sender can't poison the audit trail
/// with adversarial strings.
pub async fn correlation_id(mut req: Request, next: Next) -> Result<Response> {
    let id = inbound_id_or_fresh(req.header(CORRELATION_ID_HEADER));
    req.ctx_mut().insert(CorrelationId(id.clone()));
    let mut resp = next.run(req).await?;
    resp = resp.with_header(CORRELATION_ID_HEADER, id);
    Ok(resp)
}

/// Either trust an inbound id (when it looks safe) or mint a fresh
/// time-sortable UUID v7. Pulled out for unit testing.
fn inbound_id_or_fresh(header: Option<&str>) -> String {
    if let Some(raw) = header {
        let trimmed = raw.trim();
        if looks_like_id(trimmed) {
            return trimmed.to_string();
        }
    }
    Uuid::now_v7().hyphenated().to_string()
}

fn looks_like_id(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 || s.len() < 16 {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_id_is_uuid_v7_shape() {
        let id = inbound_id_or_fresh(None);
        // 36 chars, hyphenated, version 7.
        assert_eq!(id.len(), 36);
        let uuid = Uuid::parse_str(&id).expect("valid uuid");
        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
    }

    #[test]
    fn inbound_safe_value_is_kept() {
        // A reasonable-looking inbound id passes through.
        let inbound = "01910e4f-4b0a-7e62-8d87-1e7a09c3b1aa";
        let out = inbound_id_or_fresh(Some(inbound));
        assert_eq!(out, inbound);
    }

    #[test]
    fn inbound_with_whitespace_is_trimmed() {
        let inbound = "  01910e4f-4b0a-7e62-8d87-1e7a09c3b1aa  ";
        let out = inbound_id_or_fresh(Some(inbound));
        assert_eq!(out, inbound.trim());
    }

    #[test]
    fn inbound_too_short_is_replaced() {
        let out = inbound_id_or_fresh(Some("short"));
        assert_ne!(out, "short");
        assert!(Uuid::parse_str(&out).is_ok());
    }

    #[test]
    fn inbound_too_long_is_replaced() {
        let evil = "x".repeat(200);
        let out = inbound_id_or_fresh(Some(&evil));
        assert_ne!(out, evil);
    }

    #[test]
    fn inbound_with_control_chars_is_replaced() {
        // Spaces, newlines, ANSI escapes — any are rejected.
        for evil in [
            "abc def 1234567890",
            "abc\ndef-1234567890",
            "\x1b[31mevil\x1b[0m-1234",
        ] {
            let out = inbound_id_or_fresh(Some(evil));
            assert_ne!(out, evil, "header {evil:?} should have been replaced");
        }
    }

    #[test]
    fn fresh_ids_are_unique() {
        // Two fresh ids back-to-back must differ — ensures the v7
        // mint isn't returning a stale singleton.
        assert_ne!(inbound_id_or_fresh(None), inbound_id_or_fresh(None));
    }
}
