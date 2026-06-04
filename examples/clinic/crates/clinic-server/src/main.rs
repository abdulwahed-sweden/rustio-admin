//! The clinic server — the one and only binary.
//!
//! It owns nothing domain-specific. It wires the database, runs
//! migrations, asks each capability to register its models, and serves
//! the admin. To add a capability: add a dependency in `Cargo.toml` and
//! one `register()` call below. That is the whole integration story.

use std::path::PathBuf;

use rustio_admin::admin::Admin;
use rustio_admin::middleware;
use rustio_admin::templates::Templates;
use rustio_admin::{
    auth, background, migrations, register_admin_routes, Response, Result, Router, Server,
};

/// First-boot homepage served at `/`. Baked at compile time so the binary
/// stays single-file; edit `templates/home.html` and rebuild to change it.
const HOMEPAGE_HTML: &str = include_str!("../../../templates/home.html");

/// Where the central, append-only migrations live. Resolved from this
/// crate's location so `cargo run -p clinic-server` works from anywhere.
fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 1. Foundation: configuration + database (from clinic-core).
    let cfg = clinic_core::config::load()?;
    let db = clinic_core::db::pool(&cfg).await?;

    // 2. Schema: framework auth/admin tables, then the clinic's own
    //    central migrations. Both are idempotent and append-only.
    auth::init_tables(&db).await?;
    migrations::apply(&db, migrations_dir()).await?;
    background::spawn_housekeeping(db.clone());

    // 3. Build the admin and let each capability register its models.
    //    The server lists the capabilities; it knows nothing else
    //    about them.
    let admin = Admin::new().app_name("Clinic").accent_color("#059669");
    let admin = patients::register(admin);
    let admin = scheduling::register(admin);
    let admin = billing::register(admin);
    admin.seed_permissions(&db).await?;

    // 4. Templates: embedded defaults, with optional disk overrides when
    //    RUSTIO_TEMPLATE_DIR is set (see templates/admin/...).
    let template_dir = std::env::var("RUSTIO_TEMPLATE_DIR").ok().map(Into::into);
    let templates = Templates::new(template_dir)?;

    // 5. Serve. Middleware order is fixed by DESIGN_AUDIT.md §11.
    //    `/` is the homepage; everything else is mounted under /admin.
    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::correlation_id)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect)
        .get("/", |_req| async { Ok(Response::html(HOMEPAGE_HTML)) });
    let router = register_admin_routes(router, admin, db, templates);

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    log::info!("Clinic admin on http://{addr}/admin");
    Server::new(router, addr).run().await
}
