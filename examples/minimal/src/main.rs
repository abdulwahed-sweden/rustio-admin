//! Minimal consumer of `rustio-admin`.
//!
//! P5: defines a `Post` model with `#[derive(RustioAdmin)]` + a
//! hand-written `impl Model`, then boots a small HTTP server. The
//! model is *not* yet registered with `Admin` or wired into the
//! router — that happens in P6.

mod post;

use rustio_admin::{middleware, AdminModel, Response, Result, Router, Server};

use post::Post;

#[tokio::main]
async fn main() -> Result<()> {
    // Sanity-check that the derive expansion produced the expected
    // metadata. Surfaces the values on `/` so a curl smoke test
    // verifies the macro path end-to-end.
    let body = format!(
        "rustio-admin alive\n\nPost::ADMIN_NAME = {}\nPost::SINGULAR_NAME = {}\nPost::FIELDS.len() = {}\n",
        Post::ADMIN_NAME,
        Post::SINGULAR_NAME,
        Post::FIELDS.len(),
    );

    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::security_headers)
        .get("/", move |_req| {
            let body = body.clone();
            async move { Ok(Response::text(body)) }
        });

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    Server::new(router, addr).run().await
}
