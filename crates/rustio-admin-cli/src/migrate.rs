//! `rustio migrate` — drive the framework's `migrations::apply` /
//! `status` against `DATABASE_URL`. The directory defaults to
//! `migrations/` under the current working directory, mirroring the
//! convention every example/template uses.

use std::path::PathBuf;

use clap::Subcommand;

use rustio_admin::migrations;

#[derive(Subcommand)]
pub enum Action {
    /// Apply every pending migration in `--dir` (default `migrations/`).
    Apply {
        #[arg(long, default_value = "migrations")]
        dir: PathBuf,
    },
    /// Show every migration file with a ✓ or · for applied / pending.
    Status {
        #[arg(long, default_value = "migrations")]
        dir: PathBuf,
    },
}

pub async fn run(action: Action) -> Result<(), String> {
    let db = crate::db().await?;
    match action {
        Action::Apply { dir } => apply(db, dir).await,
        Action::Status { dir } => status(db, dir).await,
    }
}

async fn apply(db: rustio_admin::Db, dir: PathBuf) -> Result<(), String> {
    let opts = migrations::ApplyOptions { verbose: true };
    // Spinner while the library runs the per-file migration loop.
    // The library does not surface per-migration progress hooks, so
    // one outer spinner is the minimum-scope feedback that satisfies
    // PR 1.4 / DESIGN_ONBOARDING.md §9 without changing the migration
    // engine. The existing per-file `✓ <name>` summary below the
    // spinner remains untouched.
    let step = crate::progress::Step::start("Applying migrations");
    let applied = match migrations::apply_with(&db, &dir, opts).await {
        Ok(a) => {
            step.clear();
            a
        }
        Err(e) => {
            step.clear();
            return Err(crate::ui::classify_migration_error(&format!("apply: {e}")).format());
        }
    };
    if applied.is_empty() {
        println!("Nothing to apply — every migration is up to date.");
    } else {
        println!("Applied {} migration(s):", applied.len());
        for name in applied {
            println!("  ✓ {name}");
        }
    }
    Ok(())
}

async fn status(db: rustio_admin::Db, dir: PathBuf) -> Result<(), String> {
    let entries = migrations::status(&db, &dir)
        .await
        .map_err(|e| format!("status: {e}"))?;
    if entries.is_empty() {
        println!("No migration files found in {}.", dir.display());
        return Ok(());
    }
    let applied = entries.iter().filter(|(_, a)| *a).count();
    let pending = entries.len() - applied;
    for (name, ok) in &entries {
        let mark = if *ok { "✓" } else { "·" };
        println!(" {mark} {name}");
    }
    println!();
    println!("{applied} applied, {pending} pending.");
    Ok(())
}
