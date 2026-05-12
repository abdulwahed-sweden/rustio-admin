//! Add sensible security headers to every response. No arguments,
//! no config — if someone needs something custom, they can write
//! their own.

use crate::error::Result;
use crate::http::{Request, Response};
use crate::router::Next;

// public:
pub async fn security_headers(req: Request, next: Next) -> Result<Response> {
    let mut resp = next.run(req).await?;
    let headers_to_add = [
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "geolocation=(), microphone=(), camera=()",
        ),
    ];
    for (name, value) in headers_to_add {
        resp.headers.push((name.to_string(), value.to_string()));
    }
    Ok(resp)
}
