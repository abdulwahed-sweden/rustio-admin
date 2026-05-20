//! clinic-appointments example — boots an admin panel for the
//! 4-table clinic schema (clinics + patients + practitioners +
//! appointments).
//!
//! Run:
//!
//!   createdb clinic_appointments_demo
//!   DATABASE_URL=postgres://localhost/clinic_appointments_demo \
//!     cargo run -p clinic-appointments
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
use crate::models::appointment::Appointment;
use crate::models::clinic::Clinic;
use crate::models::patient::Patient;
use crate::models::practitioner::Practitioner;

#[tokio::main]
async fn main() -> Result<()> {
    // Auto-load `.env` from the cwd / parent dirs so operators don't
    // have to `set -a; source .env; set +a` before `cargo run`. If
    // no `.env` exists this is a harmless no-op.
    let dotenv_path = dotenvy::dotenv().ok();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .try_init()
        .ok();

    if let Some(p) = dotenv_path {
        println!("env: loaded {}", p.display());
    } else {
        println!("env: no .env file found; reading from process environment only");
    }

    // 1. Connection string — env override, localhost dev fallback.
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/clinic_appointments_demo".into()
    });

    // 2. Connect; fails fast if URL or server is wrong.
    let db = Db::connect(&database_url).await?;

    // 3. Framework's own auth / session / permission / audit tables.
    auth::init_tables(&db).await?;

    // 4. Example domain migrations (clinics, patients, practitioners,
    //    appointments). Run before route registration so admin queries
    //    never target missing tables.
    migrations::apply(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).await?;

    // 5. Resolve the mailer from env. `SMTP_HOST` present → real SMTP
    //    via `LettreSmtpMailer`, validated with a boot-time
    //    handshake. Unset → `LogMailer`. Misconfiguration is fatal:
    //    silent fallback would hide the misconfiguration from the
    //    operator until users started complaining about missing
    //    reset emails.
    let mailer: SharedMailer = match smtp_config_from_env() {
        Ok(Some(cfg)) => {
            let host = cfg.host.clone();
            let port = cfg.port;
            let tls = if cfg.implicit_tls {
                "implicit"
            } else {
                "starttls"
            };
            let from = cfg.from.to_string();
            let smtp_mailer = LettreSmtpMailer::new(cfg)
                .map_err(|e| rustio_admin::Error::Internal(format!("mailer: {e}")))?;

            // Boot-time SMTP handshake. Refuse to start on auth /
            // TLS / DNS failure — the operator gets a clear error
            // on `cargo run` instead of a silent "the reset email
            // never arrived" later.
            print!("mailer: validating SMTP to {host}:{port} (TLS={tls})… ");
            use std::io::Write;
            std::io::stdout().flush().ok();
            match smtp_mailer.smoke_test().await {
                Ok(()) => {
                    println!("OK");
                    println!(
                        "mailer: SMTP authenticated; recovery emails will be delivered as {from}"
                    );
                }
                Err(e) => {
                    println!("FAILED");
                    eprintln!();
                    eprintln!("mailer: SMTP boot-time handshake to {host}:{port} failed:");
                    eprintln!("        {e}");
                    eprintln!();
                    eprintln!("Common causes:");
                    eprintln!("  • SMTP_PASSWORD is wrong (Gmail: must be a 16-char App Password, no spaces)");
                    eprintln!("  • 2-Step Verification is not enabled on the Google account");
                    eprintln!(
                        "  • Wrong port for TLS mode (use 465 + implicit, or 587 + starttls)"
                    );
                    eprintln!("  • Network egress to {host}:{port} is blocked");
                    eprintln!();
                    return Err(rustio_admin::Error::Internal(
                        "mailer: SMTP handshake failed (see stderr above)".into(),
                    ));
                }
            }
            Arc::new(smtp_mailer)
        }
        Ok(None) => {
            println!("mailer: SMTP_HOST not set — using LogMailer.");
            println!("        Recovery emails will be written to stdout, NOT delivered.");
            println!(
                "        To enable real delivery, see examples/clinic-appointments/.env.example."
            );
            Arc::new(LogMailer::new())
        }
        Err(e) => {
            return Err(rustio_admin::Error::Internal(format!("mailer config: {e}")));
        }
    };

    // 6. Register the four models on the admin builder + install the
    //    resolved mailer + the user-facing identity. The app's
    //    branding owns every customer-visible surface (emails, login
    //    page, recovery flow, chrome). RustIO is intentionally
    //    absent from these — projects pass their real product name.
    //
    //    `APP_NAME` and `SUPPORT_EMAIL` flow in from the env so a
    //    developer can fork this example without editing source —
    //    matches the `.env.example` developer contract.
    let app_name = std::env::var("APP_NAME").unwrap_or_else(|_| "Clinic Appointments".into());
    let support_email = std::env::var("SUPPORT_EMAIL").ok();
    // `READ_ONLY=1` flips the whole admin into view-only mode. Useful
    // when the example boots for a demo and you want browsers to
    // poke at the UI without risking destructive clicks. Auth-flow
    // POSTs still work (login / logout / password recovery / own
    // sessions); project-data mutations get a 403 with a styled
    // error page.
    let read_only = matches!(
        std::env::var("READ_ONLY").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    );
    // Storage root for `#[rustio(file)]` columns. The framework
    // creates the directory lazily on first upload; for the
    // example we just point at a project-local `uploads/` so the
    // demo stays self-contained.
    let uploads_dir = std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "./uploads".into());
    let _ = std::fs::create_dir_all(&uploads_dir);
    let mut admin_builder = Admin::new()
        .app_name(app_name)
        .app_tagline("Operational clinic management")
        .public_url("http://127.0.0.1:3000")
        .read_only(read_only)
        .uploads_dir(&uploads_dir)
        .mailer(mailer);
    if let Some(s) = support_email {
        admin_builder = admin_builder.support_email(s);
    }
    let admin = admin_builder
        .model::<Clinic>()
        .model::<Patient>()
        .model::<Practitioner>()
        .model::<Appointment>();

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
    println!("clinic-appointments admin listening on http://{addr}/admin/");
    Server::new(router, addr).run().await?;

    Ok(())
}
