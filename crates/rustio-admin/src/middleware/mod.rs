//! Built-in middleware. Each piece is an async function with the
//! signature `async fn(Request, Next) -> Result<Response>`. Drop any of
//! them into `Router::middleware(...)` in whatever order you want.

mod compression;
mod csrf;
mod logger;
mod rate_limit;
mod security_headers;

pub use compression::gzip;
pub use csrf::{csrf_protect, CsrfGuard, CSRF_HEADER};
pub use logger::logger;
pub use rate_limit::{rate_limit, RateLimiter};
pub use security_headers::security_headers;
