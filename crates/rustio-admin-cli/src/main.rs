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

mod audit;
mod builder;
mod doctor;
mod doctor_email;
mod emergency_ui;
mod group;
mod migrate;
mod perm;
mod scaffold;
mod template_override;
mod test_init;
mod theme;
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
        /// Project preset: `minimal` (default — one `Post` model) or
        /// `blog` (adds a `Comment` model with a `post_id` FK + a
        /// migration). Unknown presets error out with the valid list.
        #[arg(long, default_value = "minimal")]
        preset: String,
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
    /// Inspect the audit trail (rustio_admin_actions). Read-only.
    Audit {
        #[command(subcommand)]
        action: audit::Action,
    },
    /// Diagnose the local environment.
    Doctor {
        #[command(subcommand)]
        action: Option<DoctorAction>,
    },

    /// Curated `AdminTheme` palette presets. Subcommands print a
    /// Rust snippet to stdout — the operator pastes it into their
    /// `Admin::new()` builder chain. The verb never touches
    /// `main.rs` or any other project file.
    Theme {
        #[command(subcommand)]
        action: theme::Action,
    },

    /// Copy an embedded admin template into the project's
    /// `templates/` directory so it can be edited. Pair with
    /// `RUSTIO_TEMPLATE_DIR=./templates` at runtime to make the
    /// override take effect. With no arguments, lists every
    /// available template name.
    #[command(name = "override")]
    Override {
        /// Canonical template name (e.g. `admin/list.html`). Omit
        /// to list every available template.
        name: Option<String>,
        /// Overwrite an existing on-disk file with the framework
        /// default. Off by default — the verb refuses to clobber so
        /// in-progress edits stay safe.
        #[arg(long)]
        force: bool,
        /// Destination root for the copy. Defaults to `./templates`
        /// (matches the `RUSTIO_TEMPLATE_DIR=./templates` convention
        /// from CLAUDE.md / `docs/architecture.md`).
        #[arg(long, default_value = "templates")]
        out: String,
    },

    /// Generate a starter integration test at `./<out>/smoke.rs`.
    /// The test spawns the project binary, probes the bound port,
    /// and asserts that `/admin/` redirects to `/admin/login` —
    /// useful as a project's first CI check.
    #[command(name = "test-init")]
    TestInit {
        /// Overwrite an existing `<out>/smoke.rs`. Off by default
        /// so the verb refuses to clobber in-progress edits.
        #[arg(long)]
        force: bool,
        /// Destination root for the test file. Defaults to
        /// `./tests` (Cargo's integration-test convention).
        #[arg(long, default_value = "tests")]
        out: String,
    },

    // ----- Builder verbs (DESIGN_BUILDER.md). All run synchronously
    // and require no database — Doctrine B9 (network-free).
    /// Bootstrap a new Builder-managed project at ./<name>.
    ///
    /// Writes `<name>/{Cargo.toml,src/main.rs,.rustio/{draft.toml,history.jsonl,builder.lock},migrations/}`.
    /// See `docs/design/DESIGN_BUILDER.md`.
    New {
        /// Project name — also the cargo crate name. Letters,
        /// digits, `-`, and `_` only; must start with a letter.
        name: String,
    },

    /// Builder authoring verbs.
    Add {
        #[command(subcommand)]
        action: BuilderAddAction,
    },

    /// Show the diff `commit` would apply. Read-only (Doctrine B8).
    Plan,

    /// Apply the plan atomically (Doctrine B8).
    Commit {
        /// Overwrite generator-owned files whose SchemaHash does
        /// not match the current draft. Prior content is
        /// quarantined to `.rustio/forced/<ts>/<path>` first
        /// (DESIGN_BUILDER.md §5.4).
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum BuilderAddAction {
    /// Record an `add_model` event in `.rustio/history.jsonl`.
    Model {
        /// CamelCase model name (e.g. `Patient`).
        name: String,
    },
    /// Record an `add_field` event in `.rustio/history.jsonl`.
    Field {
        /// CamelCase model name the field belongs to.
        model: String,
        /// snake_case field name (e.g. `full_name`).
        name: String,
        /// Declared field type. MVP closed list: text, integer,
        /// boolean, timestamp.
        #[arg(name = "type")]
        type_name: String,
        /// Add a UNIQUE constraint to this column.
        #[arg(long)]
        unique: bool,
    },
}

#[derive(Subcommand)]
enum DoctorAction {
    /// SMTP self-validation. Reads `SMTP_*` + `MAIL_FROM` (or
    /// `MAIL_PROVIDER` for a preset host/port/TLS) from the
    /// environment, opens a TLS + AUTH handshake against the
    /// configured server, and optionally sends a single test
    /// message when `--to <address>` is supplied. No credentials
    /// are echoed — the password is reported as `(set, N chars)`.
    ///
    /// `--html-preview` skips SMTP entirely and renders the
    /// recovery-email template to /tmp + opens it in the
    /// default browser. Useful for iterating on email design.
    Email {
        /// Optional recipient. When set, the doctor sends a tiny
        /// test message after the handshake passes. When omitted,
        /// the handshake is the deepest check (no email goes out).
        /// 30-second cooldown between consecutive `--to` sends
        /// (prevents accidental spam loops).
        #[arg(long)]
        to: Option<String>,
        /// Render the recovery-email body to /tmp and open it
        /// in your default browser. No SMTP traffic.
        #[arg(long)]
        html_preview: bool,
    },
}

fn main() -> ExitCode {
    // .env is optional; production deploys typically use real env vars.
    let _ = dotenvy::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();
    let result = match cli.command {
        // Pure filesystem; no async / db needed. Builder verbs also
        // sit here — DESIGN_BUILDER.md Doctrine B9 forbids network
        // calls in plan/commit, and `new` / `add` are even simpler.
        Command::Startproject { name, preset } => scaffold::project(&name, &preset),
        Command::Startapp { name } => scaffold::app(&name),
        Command::New { name } => builder_new(&name),
        Command::Add { action } => builder_add(action),
        Command::Plan => builder_plan(),
        Command::Commit { force } => builder_commit(force),
        Command::Override { name, force, out } => template_override::run(name, force, &out),
        Command::TestInit { force, out } => test_init::run(force, &out),
        Command::Theme { action } => theme::run(action),
        // Everything else opens a Postgres connection.
        other => tokio_run(async {
            match other {
                Command::Startproject { .. }
                | Command::Startapp { .. }
                | Command::New { .. }
                | Command::Add { .. }
                | Command::Plan
                | Command::Commit { .. }
                | Command::Override { .. }
                | Command::TestInit { .. }
                | Command::Theme { .. } => unreachable!("handled above"),
                Command::Migrate { action } => migrate::run(action).await,
                Command::User { action } => user::run(action).await,
                Command::Group { action } => group::run(action).await,
                Command::Perm { action } => perm::run(action).await,
                Command::Audit { action } => audit::run(action).await,
                Command::Doctor { action } => match action {
                    None => doctor::run().await,
                    Some(DoctorAction::Email { to, html_preview }) => {
                        doctor_email::run(to, html_preview).await
                    }
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

/// `rustio new <name>` dispatch — pure filesystem, no async.
fn builder_new(name: &str) -> Result<(), String> {
    let summary = builder::cmd::run_new(name)?;
    println!("{summary}");
    Ok(())
}

/// `rustio add ...` dispatch.
fn builder_add(action: BuilderAddAction) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let msg = match action {
        BuilderAddAction::Model { name } => builder::cmd::run_add_model(&cwd, &name)?,
        BuilderAddAction::Field {
            model,
            name,
            type_name,
            unique,
        } => builder::cmd::run_add_field(&cwd, &model, &name, &type_name, unique)?,
    };
    println!("{msg}");
    Ok(())
}

/// `rustio plan` dispatch.
fn builder_plan() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let out = builder::cmd::run_plan(&cwd)?;
    print!("{out}");
    Ok(())
}

/// `rustio commit` dispatch.
fn builder_commit(force: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let out = builder::cmd::run_commit(&cwd, force)?;
    print!("{out}");
    Ok(())
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
