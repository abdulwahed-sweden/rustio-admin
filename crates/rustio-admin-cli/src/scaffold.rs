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
    println!("  cp .env.example .env       # safe local defaults; edit before production");
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

// ---- startapp -----------------------------------------------------------

const APP_MODEL_TEMPLATE: &str = include_str!("../templates/app/model.rs.tmpl");
const APP_MIGRATION_TEMPLATE: &str = include_str!("../templates/app/migration.sql.tmpl");

pub fn app(name: &str) -> Result<(), String> {
    validate_app_name(name)?;
    ensure_in_project_root()?;

    let singular = camel_case(name);
    let table = format!("{name}s");

    let model_path = Path::new("src").join(format!("{name}.rs"));
    if model_path.exists() {
        return Err(format!(
            "{} already exists; pick a different name or remove the file first.",
            model_path.display()
        ));
    }

    let next_version = next_migration_version()?;
    let migration_path =
        Path::new("migrations").join(format!("{:04}_create_{}.sql", next_version, table));
    fs::create_dir_all("migrations").map_err(|e| format!("mkdir migrations: {e}"))?;

    let model_body = APP_MODEL_TEMPLATE
        .replace("{{Singular}}", &singular)
        .replace("{{name}}", name)
        .replace("{{table}}", &table);
    let migration_body = APP_MIGRATION_TEMPLATE
        .replace("{{name}}", name)
        .replace("{{table}}", &table);

    fs::write(&model_path, model_body)
        .map_err(|e| format!("write {}: {e}", model_path.display()))?;
    fs::write(&migration_path, migration_body)
        .map_err(|e| format!("write {}: {e}", migration_path.display()))?;

    println!("Created {}", model_path.display());
    println!("Created {}", migration_path.display());
    println!();
    println!("Next steps:");
    println!("  1. Edit `src/main.rs` and add the lines:");
    println!();
    println!("       mod {name};");
    println!("       use {name}::{singular};");
    println!();
    println!("     Then chain it onto the Admin builder:");
    println!();
    println!("       Admin::new()");
    println!("           // … other models …");
    println!("           .model::<{singular}>()");
    println!();
    println!("  2. Apply the migration:");
    println!();
    println!("       rustio migrate apply");
    println!();
    println!("  3. Reboot the server. The `{singular}` admin pages land at /admin/{table}.");
    Ok(())
}

/// App names follow Rust module rules: ASCII lowercase letters /
/// digits / `_`, not starting with a digit. We deliberately reject
/// `-` here because it can't appear in a Rust module path without
/// `r#`-escapes. Stricter than `validate_name` on purpose.
fn validate_app_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("app name is required".into());
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("app name may not start with a digit".into());
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !valid {
        return Err(
            "app name may only contain lowercase ASCII letters, digits, and '_' \
             (e.g. `post`, `course`, `book_review`)"
                .into(),
        );
    }
    Ok(())
}

/// Refuse to scaffold an app outside a project. We recognise a
/// project root by the combo of `Cargo.toml` and `src/main.rs`,
/// which `rustio startproject` always lays down — and which
/// `cargo new --bin` produces too.
fn ensure_in_project_root() -> Result<(), String> {
    if !Path::new("Cargo.toml").exists() {
        return Err(
            "no Cargo.toml in the current directory. Run `rustio startapp` from the \
             project root, or scaffold a fresh project with `rustio startproject <name>` \
             first."
                .into(),
        );
    }
    if !Path::new("src").join("main.rs").exists() {
        return Err(
            "no src/main.rs in the current directory. The CLI scaffolds models for \
             binary projects."
                .into(),
        );
    }
    Ok(())
}

/// snake_case → CamelCase. `book_review` → `BookReview`. Single-word
/// inputs hit the Title Case path, which is the same shape.
fn camel_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut next_upper = true;
    for c in snake.chars() {
        if c == '_' {
            next_upper = true;
        } else if next_upper {
            out.extend(c.to_uppercase());
            next_upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Walk `migrations/` and return the next available `NNNN` prefix.
/// Picks `1` for an empty / missing directory; otherwise `max + 1`
/// across every parseable filename.
fn next_migration_version() -> Result<i64, String> {
    let dir = Path::new("migrations");
    if !dir.exists() {
        return Ok(1);
    }
    let mut highest: i64 = 0;
    for entry in fs::read_dir(dir).map_err(|e| format!("read_dir migrations: {e}"))? {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        let prefix = stem.split_once('_').map(|(p, _)| p).unwrap_or(stem);
        if let Ok(n) = prefix.parse::<i64>() {
            if n > highest {
                highest = n;
            }
        }
    }
    Ok(highest + 1)
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
        assert!(
            !APP_MODEL_TEMPLATE.is_empty(),
            "app model template is empty"
        );
        assert!(
            !APP_MIGRATION_TEMPLATE.is_empty(),
            "app migration template is empty"
        );
    }

    #[test]
    fn camel_case_handles_single_word_and_snake() {
        assert_eq!(camel_case("post"), "Post");
        assert_eq!(camel_case("book_review"), "BookReview");
        assert_eq!(camel_case("a_b_c"), "ABC");
        assert_eq!(camel_case(""), "");
    }

    #[test]
    fn validate_app_name_accepts_typical_names() {
        for name in &["post", "course", "book_review", "user2", "v1"] {
            assert!(validate_app_name(name).is_ok(), "should accept {name}");
        }
    }

    #[test]
    fn validate_app_name_rejects_capitals_and_dashes() {
        for name in &["Post", "BOOK", "book-review", "1book", "", "book.review"] {
            assert!(validate_app_name(name).is_err(), "should reject {name:?}");
        }
    }
}
