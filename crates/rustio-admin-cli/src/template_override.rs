//! `rustio override` ----- copy an embedded admin template into the
//! project's `templates/` directory so the project can edit it.
//!
//! Pairs with the disk-loader path in
//! `crates/rustio-admin/src/templates.rs`: at runtime,
//! `RUSTIO_TEMPLATE_DIR=./templates` makes any file under that root
//! shadow the matching embedded default. Before this verb shipped,
//! operators reproduced the same effect by hand-copying the source
//! file out of the crate's `assets/templates/admin/` tree.
//!
//! No network, no DB ----- pure filesystem. Returns a one-line summary
//! the caller prints on success.

use std::fs;
use std::path::{Path, PathBuf};

/// Dispatch for `rustio override`.
///
/// - `name = None` → list every available template name.
/// - `name = Some(n)` → materialise `<out>/<n>` from the embedded
///   default. Refuses to clobber an existing file unless `force`.
pub(crate) fn run(name: Option<String>, force: bool, out: &str) -> Result<(), String> {
    match name {
        None => list_templates(),
        Some(n) => copy_template(&n, force, Path::new(out)),
    }
}

fn list_templates() -> Result<(), String> {
    let names = rustio_admin::embedded_template_names();
    println!("{} embedded templates:", names.len());
    for n in &names {
        println!("  {n}");
    }
    println!();
    println!("Copy one with `rustio override <name>` ----- drops it into");
    println!("`./templates/<name>` so a `RUSTIO_TEMPLATE_DIR=./templates`");
    println!("run picks it up instead of the embedded default.");
    Ok(())
}

fn copy_template(name: &str, force: bool, out_root: &Path) -> Result<(), String> {
    let body = rustio_admin::embedded_template_source(name).ok_or_else(|| {
        format!("unknown template `{name}`. Run `rustio override` (no args) for the full list.")
    })?;

    // Defense in depth: refuse path-traversal attempts even though
    // the source comes from a closed `embedded_template_names()`
    // list. Future-proofing ----- the closed set is the security
    // boundary today, but operator habits (running `override` from
    // a parent directory) make the path-write surface worth
    // double-checking.
    if name.contains("..") || name.starts_with('/') {
        return Err(format!(
            "refusing to write template `{name}` ----- path contains parent or absolute segment"
        ));
    }

    let dest: PathBuf = out_root.join(name);
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
    fs::write(&dest, body).map_err(|e| format!("write {}: {e}", dest.display()))?;

    println!("Copied {name} → {} ({} bytes)", dest.display(), body.len());
    println!();
    println!("Edit the file, then run your project with");
    println!("    RUSTIO_TEMPLATE_DIR={} cargo run", out_root.display());
    println!("to pick up the override.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn unknown_template_returns_clear_error() {
        let tmp = env::temp_dir().join("rustio-cli-override-tests-unknown");
        let _ = fs::remove_dir_all(&tmp);
        let err = copy_template("admin/does-not-exist.html", false, &tmp).unwrap_err();
        assert!(err.contains("unknown template"), "got: {err}");
        // Nothing should have been written.
        assert!(!tmp.exists());
    }

    #[test]
    fn refuses_path_traversal() {
        let tmp = env::temp_dir().join("rustio-cli-override-tests-traversal");
        let _ = fs::remove_dir_all(&tmp);
        // `../../etc/passwd` doesn't appear in `embedded_template_names()`,
        // so we never get past the membership check ----- but if a future
        // refactor accidentally widens the input, the explicit
        // path-segment check catches it. Exercise both layers by
        // poking the inner helper with a name that contains `..`.
        let err = copy_template("admin/../etc/passwd.html", false, &tmp).unwrap_err();
        // Could fail on either the "unknown template" check OR the
        // path-segment check, depending on whether the name happens
        // to be in the embedded set. Both are correct outcomes.
        assert!(
            err.contains("unknown template") || err.contains("parent or absolute"),
            "got: {err}"
        );
        assert!(!tmp.exists());
    }

    #[test]
    fn copies_known_template_and_writes_correct_bytes() {
        let tmp = env::temp_dir().join("rustio-cli-override-tests-copy");
        let _ = fs::remove_dir_all(&tmp);
        // Pick the first embedded name ----- the test stays robust
        // across reorderings.
        let names = rustio_admin::embedded_template_names();
        let target = names
            .first()
            .copied()
            .expect("at least one embedded template");
        copy_template(target, false, &tmp).expect("copy succeeds");
        let dest = tmp.join(target);
        assert!(dest.is_file(), "destination {} not written", dest.display());
        let body = fs::read_to_string(&dest).unwrap();
        let expected = rustio_admin::embedded_template_source(target).unwrap();
        assert_eq!(body, expected, "written body diverges from embedded source");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn refuses_to_clobber_without_force() {
        let tmp = env::temp_dir().join("rustio-cli-override-tests-clobber");
        let _ = fs::remove_dir_all(&tmp);
        let names = rustio_admin::embedded_template_names();
        let target = names.first().copied().unwrap();
        copy_template(target, false, &tmp).unwrap();
        // Second copy without force fails.
        let err = copy_template(target, false, &tmp).unwrap_err();
        assert!(err.contains("already exists"), "got: {err}");
        // With --force it succeeds.
        copy_template(target, true, &tmp).expect("force overrides existing file");
        let _ = fs::remove_dir_all(&tmp);
    }
}
