//! `rustio reload` — dev-mode watcher.
//!
//! Thin wrapper around `cargo watch -x run`. Watches the project's
//! source tree and re-runs `cargo run` whenever a file changes —
//! the standard Rust hot-iterate loop for a long-running server.
//!
//! Requires the [`cargo-watch`](https://crates.io/crates/cargo-watch)
//! third-party subcommand. The verb refuses cleanly when it isn't
//! installed and prints the one-line install command instead of
//! exploding with cargo's default error.
//!
//! Stdio is forwarded transparently — the watcher's compile errors
//! and the project's stdout / stderr land in the operator's terminal
//! exactly as `cargo watch -x run` would.
//!
//! No DB, no network. Pure filesystem + process spawn.

use std::process::Command;

/// Dispatch for `rustio reload`. Returns the cargo-watch exit code
/// (zero on a clean shutdown of the watcher, non-zero on error
/// install or watcher failure).
pub(crate) fn run() -> Result<(), String> {
    // Pre-flight: probe `cargo watch --version` so we surface a
    // friendly install message rather than the raw "no such
    // subcommand" cargo would print. `cargo watch` exits non-zero
    // when the subcommand isn't installed.
    let probe = Command::new("cargo").args(["watch", "--version"]).output();
    match probe {
        Ok(out) if out.status.success() => {}
        _ => {
            return Err("`cargo-watch` isn't installed. Install once with:\n  \
                 cargo install cargo-watch\n\
                 Then re-run `rustio reload`."
                .into());
        }
    }

    println!("rustio reload — running `cargo watch -x run` (Ctrl-C to stop)…");
    let status = Command::new("cargo")
        .args(["watch", "-x", "run"])
        .status()
        .map_err(|e| format!("spawn cargo watch: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo watch exited with status {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "(signal)".into())
        ))
    }
}
