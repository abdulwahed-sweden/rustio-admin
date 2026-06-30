//! `rustio-draft` CLI.
//!
//! F1: `rustio-draft new "<brief>"` → write a validated `schema.json`, then
//! print the deterministic next steps (`rustio-admin import` / `plan` / `commit`).
//! F2: `--apply` shells out to `rustio-admin import` + `plan` and stops for
//! review — it never runs `commit`.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use rustio_draft::{resolve_rustio_admin_bin, schema, DraftClient, DEFAULT_MODEL};

/// Setup-time genesis: a brief in, a rustio-admin schema.json out. Never runs at
/// runtime; the runtime and CLI contain no AI.
#[derive(Parser)]
#[command(name = "rustio-draft", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Design a schema.json from a natural-language brief.
    New {
        /// What to build, e.g. "a booking system for a salon: clients, staff, appointments".
        brief: String,
        /// Where to write the schema (refuses to overwrite unless --force).
        #[arg(long, default_value = "schema.json")]
        out: PathBuf,
        /// Claude model to use.
        #[arg(long, default_value = DEFAULT_MODEL)]
        model: String,
        /// Max response tokens.
        #[arg(long, default_value_t = 8000)]
        max_tokens: u32,
        /// Overwrite the output file if it already exists.
        #[arg(long)]
        force: bool,
        /// After writing, shell out to `rustio-admin import` + `plan` (stops for
        /// review — never runs `commit`). Run this inside a Builder project.
        #[arg(long)]
        apply: bool,
        /// Path/name of the `rustio-admin` binary for --apply (default:
        /// $RUSTIO_ADMIN_BIN or `rustio-admin` on PATH).
        #[arg(long)]
        rustio_admin: Option<String>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    match Cli::parse().command {
        Command::New {
            brief,
            out,
            model,
            max_tokens,
            force,
            apply,
            rustio_admin,
        } => {
            new(NewArgs {
                brief,
                out,
                model,
                max_tokens,
                force,
                apply,
                rustio_admin,
            })
            .await
        }
    }
}

struct NewArgs {
    brief: String,
    out: PathBuf,
    model: String,
    max_tokens: u32,
    force: bool,
    apply: bool,
    rustio_admin: Option<String>,
}

async fn new(args: NewArgs) -> Result<()> {
    let NewArgs {
        brief,
        out,
        model,
        max_tokens,
        force,
        apply,
        rustio_admin,
    } = args;
    if out.exists() && !force {
        bail!(
            "{} already exists; pass --force to overwrite",
            out.display()
        );
    }

    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        anyhow::anyhow!("ANTHROPIC_API_KEY is not set; export your Anthropic API key")
    })?;

    eprintln!("Designing a schema with {model}…");
    let client = DraftClient::new(api_key, model, max_tokens);
    let doc = client.generate(&brief).await?;

    // The model output is untrusted: re-check it the way `import` will.
    if let Err(problems) = schema::validate(&doc) {
        eprintln!("The proposed schema has problems:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        bail!(
            "refusing to write an invalid schema ({} problem(s))",
            problems.len()
        );
    }

    let pretty = serde_json::to_string_pretty(&doc).context("could not serialize the schema")?;
    std::fs::write(&out, format!("{pretty}\n"))
        .with_context(|| format!("could not write {}", out.display()))?;

    let model_count = doc.models.len();
    let field_count: usize = doc.models.iter().map(|m| m.fields.len()).sum();
    eprintln!(
        "Wrote {} — {model_count} model(s), {field_count} field(s).",
        out.display()
    );

    let out_str = out.display().to_string();
    if apply {
        // Hand off to the deterministic half. We import + plan and STOP — the
        // human reviews the plan and runs `commit`. rustio-draft never commits.
        let bin = resolve_rustio_admin_bin(rustio_admin.as_deref());
        eprintln!("\nApplying with `{bin}` (import + plan; will not commit)…\n");
        run_step(&bin, &["import", &out_str])?;
        run_step(&bin, &["plan"])?;
        eprintln!("\nReviewed the plan above? Apply it with:");
        eprintln!("    {bin} commit");
    } else {
        eprintln!("\nReview it, then apply deterministically:");
        eprintln!("    rustio-admin import {out_str}");
        eprintln!("    rustio-admin plan      # preview (read-only)");
        eprintln!("    rustio-admin commit    # apply atomically");
    }
    Ok(())
}

/// Run one `rustio-admin <args>` step, streaming its output to this terminal,
/// and fail loudly if the binary is missing or the step exits non-zero.
fn run_step(bin: &str, args: &[&str]) -> Result<()> {
    let pretty = format!("{bin} {}", args.join(" "));
    let status = std::process::Command::new(bin)
        .args(args)
        .status()
        .with_context(|| {
            format!(
                "could not run `{pretty}` — is rustio-admin installed and on PATH? \
             (set --rustio-admin or RUSTIO_ADMIN_BIN)"
            )
        })?;
    if !status.success() {
        bail!("`{pretty}` failed (exit {})", status.code().unwrap_or(-1));
    }
    Ok(())
}
