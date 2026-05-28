//! `rustio-admin new <name>` interactive wizard.
//!
//! Stage 1 / PR 1.2 of the FTUX redesign. Calm guided creation:
//! confirm the project name, pick a project type (metadata only),
//! print PostgreSQL guidance, choose a database name, and hand
//! the collected inputs back to the scaffold so a matching `.env`
//! lands alongside the standard files.
//!
//! Discipline:
//!
//! - Stdlib-only. No `dialoguer` / `indicatif` / `console`.
//! - TTY-gated. Pipes, CI, and `--no-interactive` bypass entirely.
//! - Project-type is recorded as intent only -- PR 1.5 turns it into
//!   project-typed starter content (`DESIGN_ONBOARDING.md` §6).
//! - No emojis, no hype, no banners beyond a single divider.

use std::io::{self, IsTerminal, Write};

/// Curated project types from `DESIGN_ONBOARDING.md` §6. Order is
/// the order shown to the user; `custom` is the default and is
/// listed first.
pub(crate) const PROJECT_TYPES: &[&str] = &["custom", "clinic", "school", "inventory", "blog"];

/// Brief Postgres install hint printed before the database-name
/// prompt. Text-only -- `DESIGN_ONBOARDING.md` §4 forbids the CLI
/// from shelling out to package managers.
const POSTGRES_GUIDANCE: &str = "\
RustIO uses PostgreSQL. Recommended version: PostgreSQL 16.

  macOS    brew install postgresql@16 && brew services start postgresql@16
  Ubuntu   sudo apt install postgresql-16 && sudo systemctl start postgresql
  Windows  https://www.postgresql.org/download/windows/

The wizard does not install PostgreSQL for you -- that is deliberate.";

/// The inputs the wizard collects before handing off to the scaffold.
pub(crate) struct WizardInput {
    pub project_name: String,
    /// One of [`PROJECT_TYPES`]. Stored as intent only in PR 1.2;
    /// PR 1.5 will branch the starter content on it.
    pub project_type: String,
    pub db_name: String,
}

/// Decide whether the interactive wizard should run.
///
/// Bypassed when (a) the caller passed `--no-interactive`, (b) the
/// process is running under CI (the `CI` env var is set, the common
/// GitHub Actions / CircleCI / GitLab / Travis convention), or
/// (c) either stdin or stdout is not a terminal (pipes, redirection).
pub(crate) fn should_run(no_interactive: bool) -> bool {
    if no_interactive {
        return false;
    }
    if std::env::var_os("CI").is_some() {
        return false;
    }
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Run the wizard to collect a [`WizardInput`].
///
/// `suggested_name` carries the positional argument from the
/// command line (when the user typed `rustio-admin new clinic`) so the
/// first prompt can offer it as a default the user just hits
/// Enter to accept.
pub(crate) fn run(suggested_name: Option<&str>) -> Result<WizardInput, String> {
    println!("────────────────────────────────────────────────────────────");
    println!("RustIO Project Wizard");
    println!("────────────────────────────────────────────────────────────");
    println!();

    let project_name = ask_project_name(suggested_name)?;
    let project_type = ask_project_type()?;

    println!();
    println!("{POSTGRES_GUIDANCE}");
    println!();

    let db_name = ask_db_name(&project_name)?;

    println!();
    println!("Summary");
    println!("  project   {project_name}");
    println!("  type      {project_type}");
    println!("  database  {db_name}");
    println!();

    Ok(WizardInput {
        project_name,
        project_type,
        db_name,
    })
}

fn ask_project_name(suggested: Option<&str>) -> Result<String, String> {
    loop {
        let prompt = match suggested {
            Some(s) => format!("Project name [{s}]: "),
            None => "Project name: ".into(),
        };
        let input = prompt_line(&prompt)?;
        let chosen = if input.is_empty() {
            suggested.unwrap_or("").to_string()
        } else {
            input
        };
        if chosen.is_empty() {
            println!("  Project name is required.");
            continue;
        }
        match validate_project_name(&chosen) {
            Ok(()) => return Ok(chosen),
            Err(msg) => println!("  {msg}"),
        }
    }
}

fn ask_project_type() -> Result<String, String> {
    println!();
    println!("Project type:");
    for (i, t) in PROJECT_TYPES.iter().enumerate() {
        println!("  {}) {t}", i + 1);
    }
    loop {
        let input = prompt_line("Type [custom]: ")?;
        if input.is_empty() {
            return Ok("custom".into());
        }
        if let Ok(n) = input.parse::<usize>() {
            if (1..=PROJECT_TYPES.len()).contains(&n) {
                return Ok(PROJECT_TYPES[n - 1].into());
            }
        }
        if let Some(t) = PROJECT_TYPES
            .iter()
            .find(|t| t.eq_ignore_ascii_case(&input))
        {
            return Ok((*t).into());
        }
        println!(
            "  Choose 1–{}, or one of: {}",
            PROJECT_TYPES.len(),
            PROJECT_TYPES.join(", ")
        );
    }
}

fn ask_db_name(project: &str) -> Result<String, String> {
    let default = format!("{project}_dev");
    loop {
        let input = prompt_line(&format!("Database name [{default}]: "))?;
        let chosen = if input.is_empty() {
            default.clone()
        } else {
            input
        };
        match validate_db_name(&chosen) {
            Ok(()) => return Ok(chosen),
            Err(msg) => println!("  {msg}"),
        }
    }
}

fn prompt_line(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|e| format!("flush stdout: {e}"))?;
    let mut buf = String::new();
    let n = io::stdin()
        .read_line(&mut buf)
        .map_err(|e| format!("read stdin: {e}"))?;
    if n == 0 {
        return Err(
            "stdin closed unexpectedly; re-run with `--no-interactive` for scripted use".into(),
        );
    }
    Ok(buf.trim().to_string())
}

/// Project name validation -- mirrors `scaffold::validate_name`
/// semantics (ASCII letters / digits / `-` / `_`, not starting
/// with a digit). Duplicated here so the wizard can re-prompt
/// without round-tripping through the scaffold.
pub(crate) fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("project name is required".into());
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("project name may not start with a digit".into());
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err("project name may only contain ASCII letters, digits, '-', and '_'".into());
    }
    Ok(())
}

/// PostgreSQL identifier rules without quoting: ASCII letters /
/// digits / `_`, not starting with a digit, at most 63 bytes
/// (the standard `NAMEDATALEN - 1`). Rejects `postgres`,
/// `template0`, and `template1` outright -- picking those names
/// breaks `createdb` later, and surfacing it now is kinder than
/// surfacing it after `rustio-admin migrate apply`.
pub(crate) fn validate_db_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("database name is required".into());
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("database name may not start with a digit".into());
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("database name may only contain ASCII letters, digits, and '_'".into());
    }
    if name.len() > 63 {
        return Err("database name must be 63 characters or fewer (PostgreSQL limit)".into());
    }
    let reserved = ["postgres", "template0", "template1"];
    if reserved.iter().any(|r| r.eq_ignore_ascii_case(name)) {
        return Err(format!(
            "'{name}' is reserved by PostgreSQL -- pick a different name"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_run_respects_no_interactive_flag() {
        assert!(!should_run(true), "--no-interactive must always bypass");
    }

    #[test]
    fn validate_db_name_accepts_typical_names() {
        for n in &["clinic_dev", "school_2026", "inventory", "a_b_c", "x1"] {
            assert!(validate_db_name(n).is_ok(), "should accept {n}");
        }
    }

    #[test]
    fn validate_db_name_rejects_bad_names() {
        for n in &[
            "",
            "1clinic",
            "clinic-dev",
            "clinic.dev",
            "clinic dev",
            "postgres",
            "TEMPLATE0",
            "Template1",
        ] {
            assert!(validate_db_name(n).is_err(), "should reject {n:?}");
        }
    }

    #[test]
    fn validate_db_name_enforces_63_byte_limit() {
        let ok = "a".repeat(63);
        let too_long = "a".repeat(64);
        assert!(validate_db_name(&ok).is_ok(), "63 chars must be accepted");
        assert!(
            validate_db_name(&too_long).is_err(),
            "64 chars must be rejected"
        );
    }

    #[test]
    fn validate_project_name_mirrors_scaffold_semantics() {
        // Sample of the same cases scaffold::tests::valid_names_accepted
        // / invalid_names_rejected exercise -- sanity that the two
        // validators stay in lock-step.
        for ok in &["my-app", "my_app", "MyApp", "app1"] {
            assert!(validate_project_name(ok).is_ok(), "should accept {ok}");
        }
        for bad in &["", "1app", "my app", "my/app"] {
            assert!(validate_project_name(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn project_types_list_starts_with_custom() {
        assert_eq!(PROJECT_TYPES.first(), Some(&"custom"));
    }
}
