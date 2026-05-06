//! `rustio startproject <name>` — generate a fresh project skeleton
//! at `./<name>/`.
//!
//! Templates are baked into the binary via `include_str!` so the CLI
//! stays single-binary. Each template carries a `{{name}}` placeholder
//! that we substitute for the project name; everything else is
//! verbatim.

use std::fs;
use std::path::Path;

/// `(relative_target_path, template_body)` pairs. `target_path` is
/// relative to the new project's root and creates parent directories
/// on demand.
const PROJECT_TEMPLATES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        include_str!("../templates/project/Cargo.toml.tmpl"),
    ),
    (
        ".env.example",
        include_str!("../templates/project/.env.example"),
    ),
    (
        ".gitignore",
        include_str!("../templates/project/.gitignore"),
    ),
    (
        "README.md",
        include_str!("../templates/project/README.md.tmpl"),
    ),
    (
        "src/main.rs",
        include_str!("../templates/project/src/main.rs.tmpl"),
    ),
    (
        "src/post.rs",
        include_str!("../templates/project/src/post.rs.tmpl"),
    ),
    (
        "migrations/0001_create_posts.sql",
        include_str!("../templates/project/migrations/0001_create_posts.sql"),
    ),
];

pub fn project(name: &str) -> Result<(), String> {
    validate_name(name)?;

    let dir = Path::new(name);
    if dir.exists() {
        return Err(format!(
            "`{name}` already exists in the current directory. Pick a fresh name or remove it first."
        ));
    }

    let mut written = 0usize;
    for (rel, body) in PROJECT_TEMPLATES {
        let target = dir.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        let body = body.replace("{{name}}", name);
        fs::write(&target, body).map_err(|e| format!("write {}: {e}", target.display()))?;
        written += 1;
    }

    println!("Created `{name}/` with {written} files.");
    println!();
    println!("Next steps:");
    println!("  cd {name}");
    println!("  cp .env.example .env       # edit DATABASE_URL if needed");
    println!("  rustio migrate apply       # creates the posts table");
    println!("  rustio user create --email admin@{name}.local --role administrator");
    println!("  cargo run                  # boots http://127.0.0.1:8000/admin");
    Ok(())
}

/// A project name must be a valid Rust crate identifier: ASCII
/// letters / digits / `-` / `_`, not starting with a digit, not
/// empty. The Cargo.toml template uses the name verbatim, so any
/// character cargo would reject here would just shift the failure
/// downstream.
fn validate_name(name: &str) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names_accepted() {
        for name in &["my-app", "my_app", "MyApp", "app1", "a-b_c-1"] {
            assert!(validate_name(name).is_ok(), "should accept {name}");
        }
    }

    #[test]
    fn invalid_names_rejected() {
        for name in &["", "1app", "my app", "my/app", "my.app", "my\u{1F600}app"] {
            assert!(validate_name(name).is_err(), "should reject {name:?}");
        }
    }

    #[test]
    fn every_template_carries_at_least_one_placeholder_or_fixed_content() {
        // Sanity check that the static slice is wired correctly.
        // Empty templates are also a regression — `include_str!` would
        // happily load a zero-byte file but the scaffold would write
        // empty files into the new project.
        for (rel, body) in PROJECT_TEMPLATES {
            assert!(!body.is_empty(), "template {rel} is empty");
        }
    }
}
