use crate::error::Result;
use crate::http::{Request, Response};
use crate::router::Next;

pub async fn logger(req: Request, next: Next) -> Result<Response> {
    let method = req.method().clone();
    let path = req.path().to_string();
    let start = std::time::Instant::now();
    let resp = next.run(req).await;
    let elapsed = start.elapsed();
    match &resp {
        Ok(r) => log::info!("{method} {path} -> {} ({elapsed:?})", r.status),
        Err(e) => log::warn!("{method} {path} -> error: {e}"),
    }
    resp
}
