//! `rustio-admin new <name>` interactive wizard.
//!
//! Stage 1 / PR 1.2 of the FTUX redesign. Calm guided creation:
//! confirm the project name, pick a project type (metadata only),
//! print PostgreSQL guidance, choose a database name, and hand
//! the collected inputs back to the scaffold so a matching `.env`
//! lands alongside the standard files.
//!
//! Voice (clarity-first, within `DESIGN_ONBOARDING.md`): the first
//! screen says *what this is* and *what you'll have at the end* before any
//! prompt; each question carries one calm line of *why it matters*; each
//! answered step gets a sober `✓` confirmation (§9.1). Understanding
//! before mechanics — a first-time reader should never face a prompt whose
//! purpose they have to guess.
//!
//! Discipline:
//!
//! - Stdlib-only. No `dialoguer` / `indicatif` / `console`.
//! - TTY-gated. Pipes, CI, and `--no-interactive` bypass entirely.
//! - Project-type is recorded as intent only -- PR 1.5 turns it into
//!   project-typed starter content (`DESIGN_ONBOARDING.md` §6).
//! - No emojis, no hype, no banners beyond a single divider. The `✓` is a
//!   single sober Unicode mark (§9.1), never decoration.

use std::io::{self, IsTerminal, Write};

use crate::style;

/// Curated project types from `DESIGN_ONBOARDING.md` §6. Order is
/// the order shown to the user; `custom` is the default and is
/// listed first.
pub(crate) const PROJECT_TYPES: &[&str] = &["custom", "translation-agency", "clinic", "blog"];

/// Brief Postgres install hint printed before the database-name
/// prompt. Text-only -- `DESIGN_ONBOARDING.md` §4 forbids the CLI
/// from shelling out to package managers.
const POSTGRES_GUIDANCE: &str = "\
RustIO is built on PostgreSQL — one database, by design (no SQLite fallback).
If you don't have PostgreSQL 16 yet, install and start it:

  macOS    brew install postgresql@16 && brew services start postgresql@16
  Ubuntu   sudo apt install postgresql-16 && sudo systemctl start postgresql
  Windows  https://www.postgresql.org/download/windows/

The wizard does not install PostgreSQL for you -- that is deliberate. You
stay in control of your machine.";

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
    println!();
    println!("  {}", style::title("RustIO  ›  new project"));
    println!();
    println!(
        "  {}",
        style::hint("Three short questions, then a ready-to-run admin panel —")
    );
    println!(
        "  {}",
        style::hint("login, roles, and an audit trail already wired in.")
    );
    println!();
    println!(
        "  {}",
        style::hint("Press Enter to accept the default in [brackets]. Nothing is")
    );
    println!(
        "  {}",
        style::hint("written to disk until you've answered all three.")
    );
    println!();
    println!("  {}", style::divider());

    // Each step prints its own header, hints, and prompt; `run` owns the
    // blank-line spacing between stages and the `✓` confirmation after
    // each answer, so the whole flow reads as: see → answer → confirmed.
    println!();
    println!("  {}", style::step(1, 3, "Project name"));
    let project_name = ask_project_name(suggested_name)?;
    println!("{}", style::confirm(&project_name));

    println!();
    println!("  {}", style::step(2, 3, "Project type"));
    let project_type = ask_project_type()?;
    println!("{}", style::confirm(&project_type));

    println!();
    println!("  {}", style::step(3, 3, "Database"));
    let db_name = ask_db_name(&project_name)?;
    println!("{}", style::confirm(&db_name));

    println!();
    println!("  {}", style::divider());
    println!();
    println!("  {}", style::hint("Creating your project…"));
    println!();

    Ok(WizardInput {
        project_name,
        project_type,
        db_name,
    })
}

fn ask_project_name(suggested: Option<&str>) -> Result<String, String> {
    println!(
        "  {}",
        style::hint("Names the new folder and the Cargo crate. Letters, digits,")
    );
    println!(
        "  {}",
        style::hint("'-' and '_'; must start with a letter.")
    );
    println!();
    loop {
        let prompt = match suggested {
            Some(s) => format!("  {} Project name [{s}]: ", style::arrow()),
            None => format!("  {} Project name: ", style::arrow()),
        };
        let input = prompt_line(&prompt)?;
        let chosen = if input.is_empty() {
            suggested.unwrap_or("").to_string()
        } else {
            input
        };
        if chosen.is_empty() {
            println!("    {}", style::warn("Project name is required."));
            continue;
        }
        match validate_project_name(&chosen) {
            Ok(()) => return Ok(chosen),
            Err(msg) => println!("    {}", style::warn(&msg)),
        }
    }
}

/// One-line honest hint shown beside each project type. Only the types
/// with a real content preset (`clinic`, `blog`) advertise example
/// models; every other type produces the neutral `minimal` scaffold, so
/// it is labelled a clean slate rather than a "starting point" it does
/// not deliver. Keep in lock-step with `scaffold::preset_layers`.
fn project_type_hint(t: &str) -> &'static str {
    match t {
        "translation-agency" => "example models — translators, tasks",
        "clinic" => "example models — patients, appointments",
        "blog" => "example models — posts, comments",
        _ => "clean slate (no models yet)",
    }
}

fn ask_project_type() -> Result<String, String> {
    println!(
        "  {}",
        style::hint("clinic and blog come with example models you can run right away;")
    );
    println!(
        "  {}",
        style::hint("the rest start clean. You can change direction at any time.")
    );
    println!();
    for (i, t) in PROJECT_TYPES.iter().enumerate() {
        // Pad the plain name *before* styling — padding a styled string
        // would count the invisible escape bytes and misalign the column.
        println!(
            "    {}  {} {}",
            style::accent(&(i + 1).to_string()),
            style::value(&format!("{t:<9}")),
            style::hint(project_type_hint(t))
        );
    }
    println!();
    loop {
        let input = prompt_line(&format!("  {} Type [1]: ", style::arrow()))?;
        if input.is_empty() {
            // Item 1 is `custom` — the clean-slate default.
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
            "    {}",
            style::warn(&format!(
                "Choose 1–{}, or one of: {}",
                PROJECT_TYPES.len(),
                PROJECT_TYPES.join(", ")
            ))
        );
    }
}

fn ask_db_name(project: &str) -> Result<String, String> {
    println!(
        "  {}",
        style::hint("Your local PostgreSQL database for development. RustIO writes it")
    );
    println!(
        "  {}",
        style::hint("into .env; you create it with `createdb` in the next steps.")
    );
    println!();
    // The Postgres install/start guidance, dimmed as reference material.
    for line in POSTGRES_GUIDANCE.lines() {
        if line.is_empty() {
            println!();
        } else {
            println!("  {}", style::hint(line));
        }
    }
    println!();
    let default = format!("{project}_dev");
    loop {
        let input = prompt_line(&format!("  {} Database name [{default}]: ", style::arrow()))?;
        let chosen = if input.is_empty() {
            default.clone()
        } else {
            input
        };
        match validate_db_name(&chosen) {
            Ok(()) => return Ok(chosen),
            Err(msg) => println!("    {}", style::warn(&msg)),
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
