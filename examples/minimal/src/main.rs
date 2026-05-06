//! Minimal consumer of `rustio-admin`.
//!
//! P2: boots an HTTP server with a single hello route. The admin
//! itself is wired in later phases (P5–P6).

use rustio_admin::{Response, Result, Router, Server};

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new().get("/", |_req| async {
        Ok(Response::text("rustio-admin alive"))
    });

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    Server::new(router, addr).run().await
}
