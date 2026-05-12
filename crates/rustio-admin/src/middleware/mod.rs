//! Built-in middleware. Each piece is an async function with the
//! signature `async fn(Request, Next) -> Result<Response>`. Drop any of
//! them into `Router::middleware(...)` in whatever order you want.

mod compression;
mod correlation_id;
mod csrf;
mod logger;
mod rate_limit;
mod security_headers;

// public:
pub use compression::gzip;
// public:
pub use correlation_id::{correlation_id, CorrelationId, CORRELATION_ID_HEADER};
// public:
pub use csrf::{csrf_protect, CsrfGuard, CSRF_HEADER};
// public:
pub use logger::logger;
// public:
pub use rate_limit::{rate_limit, RateLimiter};
// public:
pub use security_headers::security_headers;
