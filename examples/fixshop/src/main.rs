//! FixShop — a small repair shop, run from `rustio-admin`.
//!
//! Boot order matters and is the same in every rustio-admin project:
//!
//!   `auth::init_tables` → `migrations::apply` → `Admin::seed_permissions`
//!
//! The framework's own tables (users, sessions, groups, permissions,
//! audit) come first; FixShop's migrations then add the shop's tables
//! *and* its two groups, which is why migration 0005 can grant
//! permissions in plain SQL.

use std::sync::Arc;
use std::time::Duration;

use rustio_admin::admin::Admin;
use rustio_admin::middleware;
use rustio_admin::templates::Templates;
use rustio_admin::{
    auth, background, migrations, register_admin_routes, Db, Error, Response, Result, Router,
    Server,
};

mod customer;
mod job;
mod job_event;
mod quote;
mod workflow;

use customer::Customer;
use job::Job;
use job_event::JobEvent;
use quote::Quote;

const HOMEPAGE_HTML: &str = include_str!("../templates/home.html");

#[tokio::main]
async fn main() -> Result<()> {
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
    spawn_overdue_refresh(db.clone());

    // `JobEvent` is registered like any other model and then marked
    // read-only, so it gets a list page, search, filters and links for
    // free while losing the add / edit / delete affordances. A log you
    // can read and cannot rewrite.
    let admin = Admin::new()
        .app_name("FixShop")
        .app_tagline("Repairs, front to back")
        .model::<Customer>()
        .model::<Job>()
        .model::<Quote>()
        .model::<JobEvent>()
        .read_only_model("job_events");
    admin.seed_permissions(&db).await?;

    let template_dir = std::env::var("RUSTIO_TEMPLATE_DIR").ok().map(Into::into);
    let templates = Templates::new(template_dir)?;

    // Middleware order is fixed by DESIGN_AUDIT.md §11: correlation_id
    // must sit between logger and csrf_protect so every audit row
    // written under one request — the framework's and the one
    // `workflow.rs` adds per job — shares a single UUID v7.
    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::correlation_id)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect)
        .get("/", |_req| async { Ok(Response::html(HOMEPAGE_HTML)) });

    let router = register_admin_routes(router, admin, db, Arc::clone(&templates));

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    log::info!("FixShop listening on http://{addr}/admin");
    Server::new(router, addr).run().await
}

/// Keep `jobs.overdue` honest.
///
/// A job is overdue when its promised date has passed and the item is
/// still in the shop. That is the one fact in this project no write
/// can maintain — nobody touches the row when midnight passes — so it
/// is recomputed on a timer, in the same shape as the framework's own
/// `background::spawn_housekeeping` sweeper.
///
/// The column earns its keep: it gives the Jobs list a pill, a
/// one-click sidebar filter, and an `ORDER BY` that pins late jobs to
/// the top of the first page.
fn spawn_overdue_refresh(db: Db) {
    tokio::spawn(async move {
        loop {
            match refresh_overdue_flags(&db).await {
                Ok(0) => {}
                Ok(n) => log::info!("overdue flags: {n} job(s) changed"),
                Err(e) => log::warn!("overdue refresh failed: {e}"),
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

/// The `UPDATE` behind [`spawn_overdue_refresh`]. `IS DISTINCT FROM`
/// means only rows that actually change are written, so the returned
/// count is the number of jobs that crossed the line. The same
/// statement is the last thing migration 0006 runs, so `migrate apply`
/// on its own already leaves a correct database.
async fn refresh_overdue_flags(db: &Db) -> Result<u64> {
    let done = rustio_admin::sqlx::query(
        "UPDATE jobs
            SET overdue = (promised_date < CURRENT_DATE
                           AND status NOT IN ('collected', 'declined'))
          WHERE overdue IS DISTINCT FROM (promised_date < CURRENT_DATE
                                          AND status NOT IN ('collected', 'declined'))",
    )
    .execute(db.pool())
    .await?;
    Ok(done.rows_affected())
}
