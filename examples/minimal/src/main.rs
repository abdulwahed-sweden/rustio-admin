//! Minimal consumer of `rustio-admin`.
//!
//! P6: registers the demo `Post` model and mounts `/admin/*`. The
//! routes wire end-to-end: list, create, edit, delete, plus
//! login/logout and the dashboard. Every page produces a runtime
//! "template not found" until P7 ships the rewritten templates.
//!
//! Requires a reachable Postgres at `DATABASE_URL` (defaults to
//! `postgres://postgres:postgres@127.0.0.1:5432/rustio_dev`).

mod post;

use std::sync::Arc;

use rustio_admin::admin::Admin;
use rustio_admin::middleware;
use rustio_admin::templates::Templates;
use rustio_admin::{auth, register_admin_routes, Db, Response, Result, Router, Server};

use post::Post;

#[tokio::main]
async fn main() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/rustio_dev".into());

    let db = Db::connect(&database_url).await?;
    auth::init_tables(&db).await?;

    let admin = Admin::new().model::<Post>();
    admin.seed_permissions(&db).await?;

    let templates = Templates::new(None)?;

    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::security_headers)
        .get("/", |_req| async {
            Ok(Response::text(
                "rustio-admin alive — see /admin for the admin panel",
            ))
        });

    let router = register_admin_routes(router, admin, db, Arc::clone(&templates));

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    println!("[minimal] listening on http://127.0.0.1:8000  (admin: /admin)");
    Server::new(router, addr).run().await
}
