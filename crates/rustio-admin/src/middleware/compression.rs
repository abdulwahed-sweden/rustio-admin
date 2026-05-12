//! Transparent gzip compression. Kicks in for text responses over
//! ~1KB when the client sends `Accept-Encoding: gzip`.
//!
//! We don't compress binary types or small bodies — the overhead isn't
//! worth it. The threshold and content-type list are deliberately
//! small and not configurable; if you need more, add a custom
//! middleware.

use std::io::Write;

use bytes::Bytes;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::error::Result;
use crate::http::{Request, Response};
use crate::router::Next;

const MIN_SIZE: usize = 1024;

// public:
pub async fn gzip(req: Request, next: Next) -> Result<Response> {
    let accepts_gzip = req
        .header("accept-encoding")
        .map(|v| v.contains("gzip"))
        .unwrap_or(false);
    let mut resp = next.run(req).await?;

    if !accepts_gzip || resp.body.len() < MIN_SIZE {
        return Ok(resp);
    }

    let content_type = resp
        .headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.to_ascii_lowercase())
        .unwrap_or_default();

    if !should_compress(&content_type) {
        return Ok(resp);
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    if encoder.write_all(&resp.body).is_err() {
        return Ok(resp);
    }
    let compressed = match encoder.finish() {
        Ok(b) => b,
        Err(_) => return Ok(resp),
    };

    resp.body = Bytes::from(compressed);
    resp.headers
        .push(("content-encoding".into(), "gzip".into()));
    // Remove any stale content-length; the server sets the new one.
    resp.headers
        .retain(|(n, _)| !n.eq_ignore_ascii_case("content-length"));
    Ok(resp)
}

fn should_compress(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || content_type.starts_with("application/json")
        || content_type.starts_with("application/javascript")
        || content_type.starts_with("application/xml")
        || content_type.contains("svg+xml")
}
