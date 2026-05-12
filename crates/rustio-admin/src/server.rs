//! The HTTP server. Binds a TCP listener, runs each connection on its
//! own Tokio task, and shuts down gracefully on Ctrl-C.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::error::Result;
use crate::http::{Request, Response};
use crate::router::Router;

// public:
pub struct Server {
    router: Arc<Router>,
    addr: SocketAddr,
}

impl Server {
    // public:
    pub fn new(router: Router, addr: SocketAddr) -> Self {
        Self {
            router: Arc::new(router),
            addr,
        }
    }

    // public:
    /// Run until Ctrl-C / SIGTERM. Active connections get a brief grace
    /// period to drain before the runtime drops them.
    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        log::info!("rustio listening on http://{}", self.addr);

        let shutdown = shutdown_signal();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                accept = listener.accept() => {
                    let (stream, peer) = accept?;
                    let io = TokioIo::new(stream);
                    let router = self.router.clone();
                    tokio::spawn(async move {
                        let svc = service_fn(move |req: hyper::Request<Incoming>| {
                            let router = router.clone();
                            async move { handle(router, req, peer).await }
                        });
                        let conn = hyper::server::conn::http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(io, svc);
                        if let Err(e) = conn.await {
                            // Normal client disconnects produce noisy errors;
                            // only log at debug level.
                            log::debug!("connection error: {e}");
                        }
                    });
                }
                _ = &mut shutdown => {
                    log::info!("shutdown signal received, stopping accept loop");
                    break;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }
}

async fn handle(
    router: Arc<Router>,
    hyper_req: hyper::Request<Incoming>,
    _peer: SocketAddr,
) -> std::result::Result<hyper::Response<Full<Bytes>>, hyper::Error> {
    let method = hyper_req.method().clone();
    let uri = hyper_req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query().unwrap_or("").to_string();

    let mut headers = HashMap::new();
    for (name, value) in hyper_req.headers() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.as_str().to_ascii_lowercase(), v.to_string());
        }
    }

    let body = match hyper_req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            return Ok(simple_response(
                StatusCode::BAD_REQUEST,
                "could not read body",
            ));
        }
    };

    let our_req = Request::new(method, path, query, headers, body);
    let our_resp = router.dispatch(our_req).await;
    Ok(to_hyper(our_resp))
}

fn to_hyper(resp: Response) -> hyper::Response<Full<Bytes>> {
    let mut builder = hyper::Response::builder().status(resp.status);
    for (name, value) in resp.headers {
        builder = builder.header(name, value);
    }
    builder.body(Full::new(resp.body)).unwrap_or_else(|_| {
        hyper::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("internal error")))
            .unwrap()
    })
}

fn simple_response(status: StatusCode, body: &str) -> hyper::Response<Full<Bytes>> {
    hyper::Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

// public:
/// Serve a static file from disk. Strips path separators and rejects
/// `..` traversal.
pub async fn serve_static(root: std::path::PathBuf, name: &str) -> Result<Response> {
    let safe: String = name
        .chars()
        .filter(|c| *c != '/' && *c != '\\' && *c != '\0')
        .collect();
    if safe.contains("..") {
        return Err(crate::error::Error::BadRequest("invalid path".into()));
    }
    let path = root.join(&safe);
    if !path.is_file() {
        return Err(crate::error::Error::NotFound(safe));
    }
    let bytes = tokio::fs::read(&path).await?;
    Ok(Response::new(StatusCode::OK, Bytes::from(bytes))
        .with_header("content-type", guess_content_type(&safe)))
}

fn guess_content_type(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, ext)| ext) {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("html") => "text/html; charset=utf-8",
        Some("woff2") => "font/woff2",
        Some("json") => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}
