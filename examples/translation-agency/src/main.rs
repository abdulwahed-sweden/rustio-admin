use std::sync::Arc;

use rustio_admin::admin::Admin;
use rustio_admin::middleware;
use rustio_admin::templates::Templates;
use rustio_admin::{
    auth, background, migrations, register_admin_routes, Db, Error, Response, Result, Router,
    Server,
};

// rustio: modules — `mod <name>;` declarations from `rustio-admin startapp` go directly under this marker.
mod task;
mod translator;

// rustio: imports — `use <name>::<Name>;` lines from `rustio-admin startapp` go directly under this marker.
use task::Task;
use translator::Translator;

// The first-boot homepage at `/`. Baked at compile time so the binary stays
// single-file. Remove the `.get("/", …)` route below to free the path.
const HOMEPAGE_HTML: &str = include_str!("../templates/home.html");

#[tokio::main]
async fn main() -> Result<()> {
    // .env is optional; production deploys typically use real env vars.
    let _ = dotenvy::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let db_url = std::env::var("DATABASE_URL").map_err(|_| {
        Error::Internal(
            "DATABASE_URL is not set. Copy .env.example to .env and edit it before running.".into(),
        )
    })?;

    let db = Db::connect(&db_url).await?;
    auth::init_tables(&db).await?;
    migrations::apply(&db, "migrations").await?;
    background::spawn_housekeeping(db.clone());

    // Register the two models. The sidebar matches this list exactly —
    // nothing is auto-discovered behind your back.
    let admin = Admin::new()
        .app_name("Translation Agency")
        // rustio: models
        .model::<Translator>()
        .model::<Task>();

    admin.seed_permissions(&db).await?;

    // Templates::new(None) → embedded admin defaults only. Set
    // RUSTIO_TEMPLATE_DIR to override a page from disk without recompiling.
    let template_dir = std::env::var("RUSTIO_TEMPLATE_DIR").ok().map(Into::into);
    let templates = Templates::new(template_dir)?;

    // Middleware order is locked by DESIGN_AUDIT.md §11:
    //   logger → correlation_id → security_headers → csrf_protect
    // `correlation_id` MUST sit between logger and csrf_protect so every
    // audit row under one HTTP request shares one UUID v7.
    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::correlation_id)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect)
        .get("/", |_req| async { Ok(Response::html(HOMEPAGE_HTML)) });

    let router = register_admin_routes(router, admin, db, Arc::clone(&templates));

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    log::info!("listening on http://{addr}/admin");
    Server::new(router, addr).run().await
}
