//! Minimal consumer of `rustio-admin`.
//!
//! Boots an HTTP server on `127.0.0.1:8000`, applies `migrations/`,
//! initialises auth tables, registers the `Post` model, mounts the
//! admin under `/admin`, and serves a tiny welcome page at `/`.
//!
//! Requires a reachable Postgres at `DATABASE_URL` (defaults to
//! `postgres://postgres:postgres@127.0.0.1:5432/rustio_admin_minimal`).
//!
//! Set `RUSTIO_BOOTSTRAP_ADMIN=1` on the very first boot to seed an
//! admin user with the credentials printed at startup; useful for
//! demos and the framework's browser-walk acceptance test. Re-runs
//! after a user exists are no-ops.

mod post;

use std::sync::Arc;

use rustio_admin::admin::Admin;
use rustio_admin::middleware;
use rustio_admin::templates::Templates;
use rustio_admin::{auth, migrations, register_admin_routes, Db, Response, Result, Role, Router, Server};

use post::Post;

const DEMO_ADMIN_EMAIL: &str = "admin@example.local";
const DEMO_ADMIN_PASSWORD: &str = "admin123!";

#[tokio::main]
async fn main() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/rustio_admin_minimal".into()
    });

    let db = Db::connect(&database_url).await?;
    auth::init_tables(&db).await?;
    migrations::apply(&db, "examples/minimal/migrations").await?;

    if std::env::var("RUSTIO_BOOTSTRAP_ADMIN").as_deref() == Ok("1") {
        bootstrap_admin(&db).await?;
    }

    let admin = Admin::new().model::<Post>();
    admin.seed_permissions(&db).await?;

    let templates = Templates::new(None)?;

    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect)
        .get("/", |_req| async {
            Ok(Response::text(
                "rustio-admin alive — see /admin for the admin panel",
            ))
        });

    let router = register_admin_routes(router, admin, db, Arc::clone(&templates));

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    println!("[minimal] listening on http://{addr}  (admin: /admin)");
    Server::new(router, addr).run().await
}

/// One-shot admin seed for the browser-walk acceptance test.
/// Idempotent: silently no-ops when an account with the demo email
/// already exists.
async fn bootstrap_admin(db: &Db) -> Result<()> {
    if auth::find_user_by_email(db, DEMO_ADMIN_EMAIL).await?.is_some() {
        println!("[minimal] admin already exists ({DEMO_ADMIN_EMAIL})");
        return Ok(());
    }
    let id = auth::create_user(
        db,
        DEMO_ADMIN_EMAIL,
        DEMO_ADMIN_PASSWORD,
        Role::Administrator,
    )
    .await?;
    println!(
        "[minimal] seeded admin id={id} email={DEMO_ADMIN_EMAIL} password={DEMO_ADMIN_PASSWORD}"
    );
    Ok(())
}
