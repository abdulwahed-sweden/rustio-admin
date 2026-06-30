//! `rustio-draft` CLI.
//!
//! F1: `rustio-draft new "<brief>"` → write a validated `schema.json`, then
//! print the deterministic next steps (`rustio-admin import` / `plan` / `commit`).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use rustio_draft::{schema, DraftClient, DEFAULT_MODEL};

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
        } => new(brief, out, model, max_tokens, force).await,
    }
}

async fn new(
    brief: String,
    out: PathBuf,
    model: String,
    max_tokens: u32,
    force: bool,
) -> Result<()> {
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
    eprintln!("\nReview it, then apply deterministically:");
    eprintln!("    rustio-admin import {}", out.display());
    eprintln!("    rustio-admin plan      # preview (read-only)");
    eprintln!("    rustio-admin commit    # apply atomically");
    Ok(())
}
