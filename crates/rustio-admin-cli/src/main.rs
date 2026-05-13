//! `rustio` — command-line companion for `rustio-admin`.
//!
//! Subcommands available in this build:
//!
//! - `rustio migrate apply` / `status` — drive the framework's
//!   numerically prefixed `migrations/*.sql` runner.
//! - `rustio user create` / `list` / `role` / `delete` — auth
//!   table CRUD with Argon2 hashing and a confirm-twice password
//!   prompt.
//! - `rustio group create` / `list` / `add-user` — group CRUD and
//!   membership management.
//! - `rustio perm grant-user` / `grant-group` / `list` — permission
//!   grants on top of the framework's `auth::permissions` API.
//! - `rustio doctor` — health check: reachable DB? auth tables
//!   present? migrations up to date? at least one administrator?
//!
//! `startproject` / `startapp` scaffolding lands in a follow-up
//! phase; the templates need to be designed alongside.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod doctor;
mod doctor_email;
mod emergency_ui;
mod group;
mod migrate;
mod perm;
mod scaffold;
mod user;

#[derive(Parser)]
#[command(
    name = "rustio",
    version,
    about = "The rustio-admin command-line tool."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a new rustio-admin project at ./<name>.
    #[command(name = "startproject")]
    Startproject {
        /// Name of the project — also the cargo crate name. Letters,
        /// digits, '-', and '_' only.
        name: String,
    },
    /// Scaffold a new model + migration inside the current project.
    #[command(name = "startapp")]
    Startapp {
        /// Singular lowercase identifier (e.g. `post`, `course`,
        /// `book_review`). Becomes both the module file name and
        /// the snake_case prefix; the struct gets the CamelCase
        /// form (`Post`, `BookReview`); the table gets the
        /// pluralised form (`posts`, `book_reviews`).
        name: String,
    },
    /// Apply / inspect SQL migrations from a directory.
    Migrate {
        #[command(subcommand)]
        action: migrate::Action,
    },
    /// User management.
    User {
        #[command(subcommand)]
        action: user::Action,
    },
    /// Group management.
    Group {
        #[command(subcommand)]
        action: group::Action,
    },
    /// Permission management.
    Perm {
        #[command(subcommand)]
        action: perm::Action,
    },
    /// Diagnose the local environment.
    Doctor {
        #[command(subcommand)]
        action: Option<DoctorAction>,
    },
}

#[derive(Subcommand)]
enum DoctorAction {
    /// SMTP self-validation. Reads `SMTP_*` + `MAIL_FROM` from
    /// the environment, opens a TLS + AUTH handshake against the
    /// configured server, and optionally sends a single test
    /// message when `--to <address>` is supplied. No credentials
    /// are echoed — the password is reported as `(set, N chars)`.
    Email {
        /// Optional recipient. When set, the doctor sends a tiny
        /// test message after the handshake passes. When omitted,
        /// the handshake is the deepest check (no email goes out).
        #[arg(long)]
        to: Option<String>,
    },
}

fn main() -> ExitCode {
    // .env is optional; production deploys typically use real env vars.
    let _ = dotenvy::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();
    let result = match cli.command {
        // Pure filesystem; no async / db needed.
        Command::Startproject { name } => scaffold::project(&name),
        Command::Startapp { name } => scaffold::app(&name),
        // Everything else opens a Postgres connection.
        other => tokio_run(async {
            match other {
                Command::Startproject { .. } | Command::Startapp { .. } => {
                    unreachable!("handled above")
                }
                Command::Migrate { action } => migrate::run(action).await,
                Command::User { action } => user::run(action).await,
                Command::Group { action } => group::run(action).await,
                Command::Perm { action } => perm::run(action).await,
                Command::Doctor { action } => match action {
                    None => doctor::run().await,
                    Some(DoctorAction::Email { to }) => doctor_email::run(to).await,
                },
            }
        }),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn tokio_run<F>(fut: F) -> Result<(), String>
where
    F: std::future::Future<Output = Result<(), String>>,
{
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?
        .block_on(fut)
}

/// Connect to the database read from `DATABASE_URL` (loaded from
/// `.env` if present). Every subcommand uses this — failing here
/// produces a single, consistent error message.
pub(crate) async fn db() -> Result<rustio_admin::Db, String> {
    let url = std::env::var("DATABASE_URL").map_err(|_| {
        "DATABASE_URL is not set. Add it to .env or your shell environment.".to_string()
    })?;
    rustio_admin::Db::connect(&url)
        .await
        .map_err(|e| format!("could not connect to {}: {e}", redact_password(&url)))
}

/// Strip the password component from a DATABASE_URL for log output.
/// Returns the input unchanged if it doesn't parse as a URL.
fn redact_password(url: &str) -> String {
    // postgres://user:pw@host/db → postgres://user:***@host/db
    if let Some(at) = url.rfind('@') {
        if let Some(scheme_end) = url.find("://") {
            let prefix = &url[..scheme_end + 3];
            let creds_and_after = &url[scheme_end + 3..];
            let creds_end = at - (scheme_end + 3);
            let creds = &creds_and_after[..creds_end];
            let after = &creds_and_after[creds_end..];
            if let Some((user, _)) = creds.split_once(':') {
                return format!("{prefix}{user}:***{after}");
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::redact_password;

    #[test]
    fn redact_strips_password() {
        assert_eq!(
            redact_password("postgres://postgres:secret@localhost/db"),
            "postgres://postgres:***@localhost/db"
        );
    }

    #[test]
    fn redact_passthrough_when_no_password() {
        assert_eq!(
            redact_password("postgres://localhost/db"),
            "postgres://localhost/db"
        );
    }
}
