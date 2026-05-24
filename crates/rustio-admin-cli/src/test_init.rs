//! `rustio test-init` -- generate a starter integration test at
//! `./tests/smoke.rs`.
//!
//! The test spawns the project binary (`cargo run`), waits for the
//! HTTP port to bind, sends a raw GET to `/admin/`, and asserts the
//! response is a 302 / 303 redirect to `/admin/login`. That's
//! enough signal to catch the "server doesn't start" / "auth
//! middleware doesn't redirect anonymous traffic" classes of
//! regression -- the cheapest possible first CI check for a fresh
//! Rustio project.
//!
//! Stdlib-only -- the generated test uses `std::process::Command`
//! and `std::net::TcpStream`. No new project dependencies are
//! introduced. Operators are free to rewrite the file later (a
//! richer `reqwest`-based check is a one-line dep away).
//!
//! No network, no DB -- pure filesystem. Mirrors the
//! `template_override` verb's shape.

use std::fs;
use std::path::{Path, PathBuf};

/// Embedded body of the generated `tests/smoke.rs`. Kept inline so
/// the CLI binary stays single-file deploy. `{{port}}` is the only
/// substitution -- defaults to `3000`, the framework's documented
/// default.
const SMOKE_TEMPLATE: &str = include_str!("../templates/test_init/smoke.rs.tmpl");

/// Default file the verb writes. Sits under `<out>/smoke.rs` --
/// `tests/smoke.rs` with the default `--out=tests`.
const TARGET_FILE: &str = "smoke.rs";

/// Dispatch for `rustio test-init`.
pub(crate) fn run(force: bool, out: &str) -> Result<(), String> {
    let out_root = Path::new(out);
    let dest: PathBuf = out_root.join(TARGET_FILE);
    if dest.exists() && !force {
        return Err(format!(
            "{} already exists. Pass --force to overwrite, or move the existing file first.",
            dest.display()
        ));
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create directory {}: {e}", parent.display()))?;
    }
    fs::write(&dest, SMOKE_TEMPLATE).map_err(|e| format!("write {}: {e}", dest.display()))?;

    println!("Wrote {} ({} bytes).", dest.display(), SMOKE_TEMPLATE.len());
    println!();
    println!("Run it with:");
    println!("    cargo test --test smoke");
    println!();
    println!("The test boots `cargo run`, probes the bound HTTP port, and");
    println!("asserts /admin/ → 302 /admin/login. Edit it as your project grows.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_template_is_a_valid_rust_test_skeleton() {
        // Sanity: the embedded body must contain a `#[test]`
        // attribute and an actual test fn so consumers get a
        // working starting point.
        assert!(
            SMOKE_TEMPLATE.contains("#[test]"),
            "smoke template missing #[test] attribute"
        );
        assert!(
            SMOKE_TEMPLATE.contains("fn admin_root_redirects_to_login"),
            "smoke template missing the expected test fn"
        );
    }

    #[test]
    fn run_refuses_to_clobber_without_force() {
        let dir = tempdir();
        let nested = dir.join("tests");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("smoke.rs"), "existing").unwrap();

        let err =
            run(false, dir.join("tests").to_str().unwrap()).expect_err("must refuse to clobber");
        assert!(
            err.contains("already exists"),
            "expected clobber-refusal, got: {err}"
        );

        // Original file untouched.
        let content = fs::read_to_string(nested.join("smoke.rs")).unwrap();
        assert_eq!(content, "existing");
    }

    #[test]
    fn run_writes_smoke_rs_to_default_out_under_force() {
        let dir = tempdir();
        let out = dir.join("custom_tests");
        run(true, out.to_str().unwrap()).expect("should write");
        let written = fs::read_to_string(out.join("smoke.rs")).expect("smoke.rs must exist");
        assert!(written.contains("#[test]"));
    }

    /// Stdlib-only tempdir -- no `tempfile` dep just for tests.
    /// Returns a unique-per-call path under the OS temp dir; the
    /// directory is created lazily by the caller's first `fs::write`.
    fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("rustio-test-init-{pid}-{n}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
