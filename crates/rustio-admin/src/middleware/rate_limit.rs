//! Per-IP rate limiting using a token bucket. Kept in memory via
//! DashMap — good for single-node deployments. For multi-node, plug
//! the same shape against Redis or Postgres.
//!
//! Default: 120 requests / 60s. Override by constructing a
//! `RateLimiter` yourself and calling `rate_limit(limiter)`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::error::{Error, Result};
use crate::http::{Request, Response};
use crate::router::Next;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Inner>,
}

struct Inner {
    capacity: u32,
    refill_per_second: f64,
    buckets: DashMap<String, Bucket>,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    updated: Instant,
}

impl RateLimiter {
    pub fn new(capacity: u32, window: Duration) -> Self {
        let refill = capacity as f64 / window.as_secs_f64().max(0.001);
        Self {
            inner: Arc::new(Inner {
                capacity,
                refill_per_second: refill,
                buckets: DashMap::new(),
            }),
        }
    }

    pub fn default_limits() -> Self {
        Self::new(120, Duration::from_secs(60))
    }

    /// Try to consume one token from the bucket keyed by `key`.
    /// Returns `true` when the request is allowed (token consumed)
    /// and `false` when the bucket is empty.
    ///
    /// `pub(crate)` so the recovery module can drive its own
    /// scoped buckets (per-IP request + consume limits) without
    /// going through the global middleware path. The middleware
    /// closure in [`rate_limit`] continues to be the only public
    /// way to plug a limiter into the router.
    pub(crate) fn allow(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut entry = self.inner.buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.inner.capacity as f64,
            updated: now,
        });
        let elapsed = now.duration_since(entry.updated).as_secs_f64();
        let refill = elapsed * self.inner.refill_per_second;
        entry.tokens = (entry.tokens + refill).min(self.inner.capacity as f64);
        entry.updated = now;
        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// The middleware function. Wrap a limiter into a closure and hand
/// it to `Router::middleware(...)`.
pub fn rate_limit(
    limiter: RateLimiter,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response>> + Send + 'static>>
       + Clone
       + Send
       + Sync
       + 'static {
    move |req: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            // Prefer X-Forwarded-For (one-layer-deep) for deployments
            // behind a reverse proxy. Falls back to a fixed "anon" key
            // when no client identifier is available — we still rate
            // limit in that case, just globally.
            let key = req
                .header("x-forwarded-for")
                .and_then(|v| v.split(',').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "anon".to_string());

            if !limiter.allow(&key) {
                return Err(Error::BadRequest("rate limit exceeded".into()));
            }
            next.run(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_allows_burst_then_blocks() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.allow("1.2.3.4"));
        assert!(limiter.allow("1.2.3.4"));
        assert!(limiter.allow("1.2.3.4"));
        assert!(!limiter.allow("1.2.3.4"));
    }

    #[test]
    fn different_keys_tracked_separately() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        assert!(limiter.allow("a"));
        assert!(limiter.allow("b"));
        assert!(!limiter.allow("a"));
    }
}
