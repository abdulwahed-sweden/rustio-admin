//! **Contract: `rustio-admin startproject` produces a project that holds
//! together.**
//!
//! Scaffolding is a supported product feature, not a convenience script: it
//! is the documented replacement for the bundled examples this repository
//! used to carry. The three promises checked here are the ones a new user
//! hits in their first five minutes:
//!
//! 1. Every preset scaffolds without error and lays down the files the
//!    getting-started guide tells the user to expect.
//! 2. The generated `Cargo.toml` pins the framework version this workspace
//!    ships — the drift that silently broke three releases (0.17.0, 0.17.1,
//!    0.18.0) and is also guarded in CI.
//! 3. A content preset ships its models **and** the migrations that create
//!    their tables, so `migrate apply` has something to apply.
//!
//! The test drives the real binary through `CARGO_BIN_EXE_`, so it exercises
//! clap wiring, the preset table, and the file writer exactly as a user does.
//! It needs no database: `startproject` is an offline verb.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_rustio-admin")
}

fn workspace_version() -> String {
    let manifest =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml"))
            .expect("workspace Cargo.toml is readable");
    let mut in_section = false;
    for line in manifest.lines() {
        if line.trim() == "[workspace.package]" {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with('[') {
            break;
        }
        if in_section {
            if let Some(rest) = line.trim().strip_prefix("version") {
                return rest
                    .trim_start_matches([' ', '='])
                    .trim()
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    panic!("no [workspace.package] version found");
}

/// Scaffold `preset` into a fresh scratch directory and return the project root.
fn scaffold(preset: &str) -> PathBuf {
    let root = scratch(preset);
    let out = Command::new(bin())
        .current_dir(&root)
        .args(["startproject", "demo", "--preset", preset])
        .output()
        .expect("the CLI binary runs");
    assert!(
        out.status.success(),
        "startproject --preset {preset} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    root.join("demo")
}

#[test]
fn every_preset_scaffolds_a_buildable_project_layout() {
    // The file set the getting-started guide promises. If a preset stops
    // writing one of these, the guide's first five minutes break.
    for preset in [
        "minimal",
        "blog",
        "clinic",
        "translation-agency",
        "ecommerce",
    ] {
        let project = scaffold(preset);
        for expected in [
            "Cargo.toml",
            "src/main.rs",
            ".env.example",
            ".gitignore",
            "README.md",
            "templates/home.html",
        ] {
            assert!(
                project.join(expected).is_file(),
                "preset {preset} did not write {expected}"
            );
        }
        assert!(
            project.join("migrations").is_dir(),
            "preset {preset} did not create migrations/"
        );
    }
}

#[test]
fn the_scaffold_pins_the_framework_version_this_workspace_ships() {
    let project = scaffold("minimal");
    let manifest =
        std::fs::read_to_string(project.join("Cargo.toml")).expect("generated Cargo.toml");
    let want = workspace_version();
    assert!(
        manifest.contains(&format!("rustio-admin = \"{want}\"")),
        "generated Cargo.toml must pin rustio-admin = \"{want}\"; got:\n{manifest}"
    );
}

#[test]
fn a_content_preset_ships_models_and_their_migrations() {
    // translation-agency is the preset the docs are written against, and the
    // one that replaced the deleted examples/translation-agency.
    let project = scaffold("translation-agency");

    for model in ["src/translator.rs", "src/task.rs"] {
        assert!(project.join(model).is_file(), "missing {model}");
    }

    let migrations: Vec<String> = std::fs::read_dir(project.join("migrations"))
        .expect("migrations/ is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".sql"))
        .collect();
    assert!(
        migrations.len() >= 2,
        "a content preset must ship the migrations that create its tables; got {migrations:?}"
    );

    // The models must actually be registered, or the admin boots empty.
    let main = std::fs::read_to_string(project.join("src/main.rs")).expect("src/main.rs");
    assert!(
        main.contains("Translator") && main.contains("Task"),
        "the generated main.rs must register the preset's models"
    );
}

#[test]
fn an_unknown_preset_is_refused_rather_than_silently_scaffolding_minimal() {
    let root = scratch("unknown-preset");
    let out = Command::new(bin())
        .current_dir(&root)
        .args(["startproject", "demo", "--preset", "not-a-real-preset"])
        .output()
        .expect("the CLI binary runs");
    assert!(
        !out.status.success(),
        "an unknown preset must fail loudly, not fall back to minimal"
    );
}

/// A unique scratch directory. Keeps the generated trees out of the repo.
fn scratch(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rustio-scaffold-{}-{tag}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    assert!(Path::new(&dir).is_dir());
    dir
}
