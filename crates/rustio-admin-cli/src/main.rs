//! `rustio-admin` -- command-line companion for the `rustio-admin` framework.
//!
//! Beginner surface (the verbs the welcome help promotes first;
//! see `docs/design/DESIGN_ONBOARDING.md` for the doctrine):
//!
//! - `rustio-admin new <name>` -- friendly alias for `startproject`.
//!   Identical behaviour, identical files, identical exit codes.
//! - `rustio-admin doctor` -- health check: reachable DB? auth tables
//!   present? migrations up to date? at least one administrator?
//! - `rustio-admin docs [--open]` -- print where the framework's docs
//!   live; probes the local server and reflects reachability
//!   in the output; `--open` launches the local docs in your
//!   default browser (PR 2.4).
//!
//! Full surface (still listed in `--help` after the welcome banner):
//!
//! - `rustio-admin startproject` / `startapp` -- original scaffolding
//!   verbs. `new` is the friendly alias; `startproject` keeps
//!   working unchanged for scripts.
//! - `rustio-admin migrate apply` / `status` -- numerically prefixed
//!   `migrations/*.sql` runner.
//! - `rustio-admin user create` / `list` / `role` / `delete` -- auth
//!   table CRUD with Argon2 hashing.
//! - `rustio-admin group create` / `list` / `add-user` -- group CRUD
//!   and membership.
//! - `rustio-admin perm grant-user` / `grant-group` / `list` -- permission
//!   grants on top of `auth::permissions`.
//! - `rustio-admin builder new` -- declarative-Builder bootstrap. The
//!   `rustio-admin new` slot was reclaimed for the friendly scaffold
//!   alias; the legacy entrypoint is reachable as the hidden
//!   `rustio-admin new --builder` for a one-release deprecation window.
//! - `rustio-admin add` / `plan` / `commit` -- Builder authoring,
//!   plan, and apply (`docs/design/DESIGN_BUILDER.md`). Unchanged.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod app_fields;
mod audit;
mod builder;
mod docs;
mod doctor;
mod doctor_email;
mod emergency_ui;
mod group;
mod migrate;
mod perm;
mod progress;
mod reload;
mod scaffold;
mod template_override;
mod test_init;
mod theme;
mod ui;
mod user;
mod wizard;

/// Welcome banner printed before clap's auto-generated `--help`
/// command list. The Phase 1 surface promoted here is governed by
/// `docs/design/DESIGN_ONBOARDING.md` §10 (Command-surface doctrine):
/// promoted, never amputated -- the full command list still follows.
const WELCOME_HELP: &str = "\
Welcome to RustIO Admin

Start a project:

  rustio-admin new <project-name>

You can build:

  - clinic systems
  - school systems
  - inventory systems
  - blogs
  - or custom projects

Helpful commands:

  rustio-admin doctor
  rustio-admin docs

────────────────────────────────────────────────────────────\
";

#[derive(Parser)]
#[command(
    name = "rustio-admin",
    version,
    about = "The rustio-admin command-line tool.",
    before_help = WELCOME_HELP,
)]
struct Cli {
    /// Suppress progress spinners and status feedback. Errors still
    /// print. PR 1.4 of `DESIGN_ONBOARDING.md` §9. Synonym of
    /// `--no-progress`; either flag is enough on its own.
    #[arg(long, global = true)]
    quiet: bool,
    /// Suppress progress spinners. Equivalent to `--quiet`.
    #[arg(long, global = true)]
    no_progress: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    // ---- Beginner surface (promoted by the welcome banner) ----
    /// Create a new project (friendly alias for `startproject`).
    ///
    /// In an interactive terminal, a calm wizard collects the
    /// project name, project type, and database name, then writes
    /// a matching `.env` alongside the standard files (PR 1.2 of
    /// `DESIGN_ONBOARDING.md`). Under `--no-interactive`, when
    /// stdin/stdout is not a TTY, or under CI, the wizard is
    /// bypassed and behaviour is byte-identical to `startproject`.
    /// The legacy Builder bootstrap that used to live in this slot
    /// moved to `rustio-admin builder new`; the hidden `--builder` flag
    /// below is the one-release deprecation shim.
    New {
        /// Project name -- also the cargo crate name. Letters,
        /// digits, '-', and '_' only; must start with a letter.
        /// Optional: the wizard prompts for it when omitted in
        /// interactive mode. Required in non-interactive mode.
        name: Option<String>,
        /// Forwarded to `startproject`: preset choice. Defaults to
        /// `minimal` (no domain models — the scaffold is neutral so
        /// the project does not feel like someone else's demo);
        /// `blog` adds `Post` + `Comment` with their migrations. In
        /// wizard mode the user's project-type choice supersedes
        /// this flag; in non-interactive mode this flag wins.
        /// Ignored when `--builder` is set.
        #[arg(long, default_value = "minimal")]
        preset: String,
        /// Skip the interactive wizard even when running in a
        /// terminal. Behaviour matches `rustio-admin startproject` exactly
        /// -- required positional name, scaffold writes `.env.example`
        /// only (no auto-generated `.env`), same Next Steps block.
        #[arg(long)]
        no_interactive: bool,
        /// Deprecated: run the legacy Builder bootstrap. Use
        /// `rustio-admin builder new <name>` instead. Hidden from
        /// `--help`; one-release grace.
        #[arg(long, hide = true)]
        builder: bool,
    },

    /// Scaffold a new rustio-admin project at ./<name>.
    #[command(name = "startproject")]
    Startproject {
        /// Name of the project -- also the cargo crate name. Letters,
        /// digits, '-', and '_' only.
        name: String,
        /// Project preset: `minimal` (default -- neutral scaffold,
        /// no domain models) or `blog` (adds `Post` + `Comment`
        /// with their migrations). Unknown presets error out with
        /// the valid list.
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
        /// Field declaration in `<name>:<type>` form, repeatable.
        /// Closed vocabulary: `str / text / int / bigint / bool /
        /// timestamp / json / fk:<Model>`. PR 2.1 of
        /// `DESIGN_ONBOARDING.md`. Omit the flag entirely (or
        /// run interactively at a TTY) to get the historical
        /// one-field placeholder model.
        #[arg(long = "field", value_name = "NAME:TYPE")]
        fields: Vec<String>,
        /// Skip the interactive field-prompt loop even when running
        /// in a terminal. Behaviour matches non-TTY / CI exactly --
        /// if no `--field` flags are passed, the historical
        /// placeholder model is generated.
        #[arg(long)]
        no_interactive: bool,
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

    /// Print where the framework's documentation lives.
    ///
    /// Probes `http://127.0.0.1:8000/admin/health` with a short
    /// timeout to detect a running local server; the printed URL
    /// block reflects whether the local `/admin/docs` is reachable.
    /// Pass `--open` to launch the local docs in your default
    /// browser (no-op without a running server; exits 1).
    Docs {
        /// Open the local `/admin/docs` page in your default
        /// browser. Requires the project's server to be running
        /// (`cargo run`). Exits 1 if the server is unreachable
        /// or the browser opener fails. Default off — auto-
        /// opening a browser is the kind of magic
        /// `DESIGN_ONBOARDING.md` §4 forbids.
        #[arg(long)]
        open: bool,
    },

    /// Curated `AdminTheme` palette presets. Subcommands print a
    /// Rust snippet to stdout -- the operator pastes it into their
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
        /// default. Off by default -- the verb refuses to clobber so
        /// in-progress edits stay safe.
        #[arg(long)]
        force: bool,
        /// Destination root for the copy. Defaults to `./templates`
        /// (matches the `RUSTIO_TEMPLATE_DIR=./templates` convention
        /// from CLAUDE.md / `docs/architecture.md`).
        #[arg(long, default_value = "templates")]
        out: String,
    },

    /// Watch the project's source tree and re-run `cargo run` on
    /// change. Thin wrapper around `cargo watch -x run`. Requires
    /// `cargo-watch` to be installed; the verb surfaces a friendly
    /// install message when it isn't.
    Reload,

    /// Generate a starter integration test at `./<out>/smoke.rs`.
    /// The test spawns the project binary, probes the bound port,
    /// and asserts that `/admin/` redirects to `/admin/login` --
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

    // -- Builder verbs (DESIGN_BUILDER.md). All run synchronously
    // and require no database -- Doctrine B9 (network-free). The
    // top-level `new` slot was reclaimed for the friendly scaffold
    // alias; the canonical Builder bootstrap is now `rustio-admin builder
    // new`. `add`, `plan`, `commit` keep their top-level positions
    // for script compatibility.
    /// Builder workflow namespace (`DESIGN_BUILDER.md`).
    ///
    /// Currently exposes `new` (project bootstrap); `add`, `plan`,
    /// and `commit` keep their top-level positions for script
    /// compatibility.
    Builder {
        #[command(subcommand)]
        action: BuilderAction,
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
enum BuilderAction {
    /// Bootstrap a new Builder-managed project at ./<name>.
    ///
    /// Writes `<name>/{Cargo.toml,src/main.rs,.rustio/{draft.toml,history.jsonl,builder.lock},migrations/}`.
    /// See `docs/design/DESIGN_BUILDER.md`.
    New {
        /// Project name -- also the cargo crate name. Letters,
        /// digits, `-`, and `_` only; must start with a letter.
        name: String,
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
    /// are echoed -- the password is reported as `(set, N chars)`.
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

    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            // Intercept clap's InvalidValue error (e.g. `--role admin`
            // when valid roles are user/staff/...) and reformat using
            // the four-part onboarding shape from DESIGN_ONBOARDING.md §8.
            // Every other clap error (missing arg, --help, --version, …)
            // falls through to clap's default printing untouched.
            if let Some(rewritten) = rewrite_clap_invalid_value(&e) {
                eprintln!("{}", rewritten.format());
                return ExitCode::from(2);
            }
            e.exit();
        }
    };
    // Wire the global `--quiet` / `--no-progress` flags into the
    // progress layer before any Step::start is reachable. PR 1.4 /
    // DESIGN_ONBOARDING.md §9.
    progress::configure(cli.quiet, cli.no_progress);
    let result = match cli.command {
        // Pure filesystem; no async / db needed. Builder verbs also
        // sit here -- DESIGN_BUILDER.md Doctrine B9 forbids network
        // calls in plan/commit, and `new` / `add` are even simpler.
        Command::New {
            name,
            preset,
            no_interactive,
            builder,
        } => dispatch_new(name, preset, no_interactive, builder),
        Command::Startproject { name, preset } => scaffold::project(&name, &preset),
        Command::Startapp {
            name,
            fields,
            no_interactive,
        } => scaffold::app(&name, fields, no_interactive),
        Command::Builder { action } => match action {
            BuilderAction::New { name } => builder_new(&name),
        },
        Command::Add { action } => builder_add(action),
        Command::Plan => builder_plan(),
        Command::Commit { force } => builder_commit(force),
        Command::Docs { open } => docs::print_docs(open),
        Command::Override { name, force, out } => template_override::run(name, force, &out),
        Command::Reload => reload::run(),
        Command::TestInit { force, out } => test_init::run(force, &out),
        Command::Theme { action } => theme::run(action),
        // Everything else opens a Postgres connection.
        other => tokio_run(async {
            match other {
                Command::New { .. }
                | Command::Startproject { .. }
                | Command::Startapp { .. }
                | Command::Builder { .. }
                | Command::Add { .. }
                | Command::Plan
                | Command::Commit { .. }
                | Command::Docs { .. }
                | Command::Override { .. }
                | Command::Reload
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
            // Four-part onboarding errors (DESIGN_ONBOARDING.md §8)
            // carry their own structural headline; the `error: ` label
            // would just clutter it. Plain-string errors keep the
            // label so non-rewritten paths look unchanged.
            if e.starts_with(ui::ONBOARDING_SENTINEL) {
                eprintln!("{e}");
            } else {
                eprintln!("error: {e}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Inspect a clap parse error and, if it's an `InvalidValue` (a
/// `ValueEnum` mismatch like `--role admin`), build the matching
/// four-part [`ui::OnboardingError`]. Returns `None` for every
/// other clap error kind so the default printing still runs.
fn rewrite_clap_invalid_value(e: &clap::Error) -> Option<ui::OnboardingError> {
    use clap::error::{ContextKind, ContextValue, ErrorKind};
    if e.kind() != ErrorKind::InvalidValue {
        return None;
    }
    let arg = match e.get(ContextKind::InvalidArg)? {
        ContextValue::String(s) => s.clone(),
        _ => return None,
    };
    let bad = match e.get(ContextKind::InvalidValue)? {
        ContextValue::String(s) => s.clone(),
        _ => return None,
    };
    let valid: Vec<String> = match e.get(ContextKind::ValidValue)? {
        ContextValue::Strings(v) => v.clone(),
        _ => return None,
    };
    Some(ui::invalid_value(&arg, &bad, &valid))
}

/// Builder bootstrap dispatch -- pure filesystem, no async.
/// Reached via `rustio-admin builder new <name>` (canonical) and via
/// the hidden one-release `rustio-admin new --builder <name>` shim.
fn builder_new(name: &str) -> Result<(), String> {
    let summary = builder::cmd::run_new(name)?;
    println!("{summary}");
    Ok(())
}

/// `rustio-admin new` dispatch -- wizard or legacy scaffold path.
///
/// Decision order: `--builder` → legacy Builder shim (PR 1.1); else
/// if the wizard is eligible (TTY, not CI, not `--no-interactive`)
/// → run the wizard and emit a matching `.env`; else → call
/// `scaffold::project` exactly as PR 1.1 shipped it (byte-identical,
/// no `.env`, mandatory positional name).
fn dispatch_new(
    name: Option<String>,
    preset: String,
    no_interactive: bool,
    builder: bool,
) -> Result<(), String> {
    if builder {
        let name =
            name.ok_or_else(|| "`rustio-admin new --builder <name>` requires a name".to_string())?;
        eprintln!("note: `rustio-admin new --builder` is the legacy Builder entrypoint.");
        eprintln!("      Prefer `rustio-admin builder new <name>` going forward.");
        eprintln!();
        return builder_new(&name);
    }
    if wizard::should_run(no_interactive) {
        let input = wizard::run(name.as_deref())?;
        // PR 1.5: project_type drives preset selection — only the
        // explicit `blog` choice writes blog-shaped files. Every
        // other type (custom / clinic / school / inventory) gets
        // the neutral minimal scaffold and a project-type-aware
        // homepage. `DESIGN_ONBOARDING.md` §6.
        let resolved_preset = if input.project_type == "blog" {
            "blog"
        } else {
            "minimal"
        };
        return scaffold::project_with_db(
            &input.project_name,
            resolved_preset,
            &input.db_name,
            &input.project_type,
        );
    }
    // Non-interactive path -- name is mandatory (matches `startproject`).
    let name = name.ok_or_else(|| {
        "project name is required in non-interactive mode (try `rustio-admin new <name>`)"
            .to_string()
    })?;
    scaffold::project(&name, &preset)
}

/// `rustio-admin add ...` dispatch.
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

/// `rustio-admin plan` dispatch.
fn builder_plan() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let out = builder::cmd::run_plan(&cwd)?;
    print!("{out}");
    Ok(())
}

/// `rustio-admin commit` dispatch.
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
/// `.env` if present). Every subcommand uses this -- failures here
/// flow through the four-part onboarding error shape
/// (`DESIGN_ONBOARDING.md` §8) so beginners see plain-English
/// guidance instead of raw driver text. The verbatim backend error
/// is preserved in the `Details:` block for senior engineers.
pub(crate) async fn db() -> Result<rustio_admin::Db, String> {
    let url = std::env::var("DATABASE_URL").map_err(|_| ui::database_url_missing().format())?;
    // Silent-on-success spinner -- `rustio-admin user list` and friends
    // produce their own output and don't need a "✓ Connected" noise
    // line. The spinner exists only so a slow connect doesn't
    // freeze the terminal. PR 1.4 / DESIGN_ONBOARDING.md §9.
    let step = progress::Step::start("Connecting to PostgreSQL");
    match rustio_admin::Db::connect(&url).await {
        Ok(db) => {
            step.clear();
            Ok(db)
        }
        Err(e) => {
            step.clear();
            Err(ui::classify_db_connect_error(&redact_password(&url), &e.to_string()).format())
        }
    }
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
