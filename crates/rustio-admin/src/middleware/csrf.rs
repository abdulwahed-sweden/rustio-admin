//! CSRF protection using the double-submit cookie pattern.
//!
//! How it works:
//! 1. First response from a "safe" method (GET/HEAD/OPTIONS) sets a
//!    `rustio_csrf` cookie if one isn't already present. The value is
//!    a 32-byte random token, URL-safe-base64 encoded.
//! 2. Every "unsafe" request (POST/PUT/DELETE/PATCH) must carry the
//!    same token in either a header (`X-CSRF-Token`) or a form field
//!    (`_csrf`), matching the cookie.
//! 3. Mismatch → 403.
//!
//! This is the standard Django/Rails pattern. It's simple, stateless,
//! and works with HTML forms and JS clients alike.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hyper::Method;
use rand::RngCore;
use subtle::ConstantTimeEq;

use crate::error::{Error, Result};
use crate::http::{Request, Response};
use crate::router::Next;

// public:
pub const CSRF_COOKIE: &str = "rustio_csrf";
// public:
pub const CSRF_HEADER: &str = "x-csrf-token";
// public:
pub const CSRF_FIELD: &str = "_csrf";

// public:
/// A value a handler can pull out of the request context to embed in a
/// rendered form. Lightweight; just wraps a string.
#[derive(Debug, Clone)]
pub struct CsrfGuard {
    pub token: String,
}

// public:
pub async fn csrf_protect(mut req: Request, next: Next) -> Result<Response> {
    let existing_token = cookie_value(&req, CSRF_COOKIE);
    let needs_cookie = existing_token.is_none();
    let token = existing_token.unwrap_or_else(random_token);

    // Give the handler access to the current token via context, so
    // templates can render a hidden input without re-reading the cookie.
    req.ctx_mut().insert(CsrfGuard {
        token: token.clone(),
    });

    // On unsafe methods: compare header-or-form against cookie.
    if !is_safe(req.method()) {
        let provided = req.header(CSRF_HEADER).map(|s| s.to_string()).or_else(|| {
            req.form()
                .ok()
                .and_then(|f| f.get(CSRF_FIELD).map(|v| v.to_string()))
        });
        let provided = match provided {
            Some(p) => p,
            None => return Err(Error::Forbidden("CSRF token missing".into())),
        };
        if !constant_time_eq(&provided, &token) {
            return Err(Error::Forbidden("CSRF token mismatch".into()));
        }
    }

    let mut resp = next.run(req).await?;
    if needs_cookie {
        let cookie = format!("{CSRF_COOKIE}={token}; Path=/; SameSite=Strict; Max-Age=86400");
        resp.headers.push(("set-cookie".into(), cookie));
    }
    Ok(resp)
}

fn is_safe(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn cookie_value(req: &Request, name: &str) -> Option<String> {
    let header = req.header("cookie")?;
    let prefix = format!("{name}=");
    for part in header.split(';') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix(&prefix) {
            return Some(v.to_string());
        }
    }
    None
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_safe_recognises_read_methods() {
        assert!(is_safe(&Method::GET));
        assert!(is_safe(&Method::HEAD));
        assert!(is_safe(&Method::OPTIONS));
        assert!(!is_safe(&Method::POST));
        assert!(!is_safe(&Method::DELETE));
    }

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
    }
}
