//! library-circulation example — boots an admin panel for the
//! 4-table library schema defined in D.2.
//!
//! Run:
//!
//!   createdb library_circulation_demo
//!   DATABASE_URL=postgres://localhost/library_circulation_demo \
//!     cargo run -p library-circulation
//!
//! First-time admin superuser creation is out of scope for the
//! example main.rs; use the framework CLI (`rustio user create`)
//! against the same DATABASE_URL.

mod mailer;
mod models;

use std::net::SocketAddr;
use std::sync::Arc;

use rustio_admin::email::{LogMailer, SharedMailer};
use rustio_admin::templates::Templates;
use rustio_admin::{
    auth, middleware, migrations, register_admin_routes, Admin, Db, Result, Router, Server,
};

use crate::mailer::{smtp_config_from_env, LettreSmtpMailer};
use crate::models::branch::Branch;
use crate::models::item::Item;
use crate::models::loan::Loan;
use crate::models::patron::Patron;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .try_init()
        .ok();

    // 1. Connection string — env override, localhost dev fallback.
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/library_circulation_demo".into()
    });

    // 2. Connect; fails fast if URL or server is wrong.
    let db = Db::connect(&database_url).await?;

    // 3. Framework's own auth / session / permission / audit tables.
    auth::init_tables(&db).await?;

    // 4. Example domain migrations (branches, patrons, items, loans).
    //    Run before route registration so admin queries never target
    //    missing tables.
    migrations::apply(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).await?;

    // 5. Resolve the mailer from env. `SMTP_HOST` present → real SMTP
    //    via `LettreSmtpMailer`. Unset → `LogMailer` writes the
    //    would-be email body to stdout (safe dev default; recovery
    //    flow still emits audit + redirects normally). Malformed
    //    SMTP env causes a hard boot failure so misconfigurations
    //    fail loud instead of silently swallowing reset emails.
    let mailer: SharedMailer = match smtp_config_from_env() {
        Ok(Some(cfg)) => {
            println!(
                "mailer: SMTP via {host}:{port} (TLS={tls}) from {from}",
                host = cfg.host,
                port = cfg.port,
                tls = if cfg.implicit_tls { "implicit" } else { "starttls" },
                from = cfg.from,
            );
            Arc::new(
                LettreSmtpMailer::new(cfg)
                    .map_err(|e| rustio_admin::Error::Internal(format!("mailer: {e}")))?,
            )
        }
        Ok(None) => {
            println!(
                "mailer: SMTP_HOST unset; using LogMailer (recovery emails will \
                 print to stdout, not be delivered)"
            );
            Arc::new(LogMailer::new())
        }
        Err(e) => {
            return Err(rustio_admin::Error::Internal(format!(
                "mailer config: {e}"
            )));
        }
    };

    // 6. Register the four models on the admin builder + install the
    //    resolved mailer.
    let admin = Admin::new()
        .mailer(mailer)
        .model::<Branch>()
        .model::<Patron>()
        .model::<Item>()
        .model::<Loan>();

    // 7. Materialise view / add / change / delete perms per model.
    admin.seed_permissions(&db).await?;

    // 8. Embedded-template engine (no disk override directory).
    let templates = Templates::new(None)?;

    // 9. R0-canonical middleware chain. Order is load-bearing.
    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::correlation_id)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect);

    // 10. Mount /admin/* onto the middleware-wrapped router.
    let router = register_admin_routes(router, admin, db.clone(), templates);

    // 11. Bind and run; blocks until Ctrl-C.
    let addr: SocketAddr = "127.0.0.1:3000"
        .parse()
        .expect("hard-coded bind address parses");
    println!("library-circulation admin listening on http://{addr}/admin/");
    Server::new(router, addr).run().await?;

    Ok(())
}
