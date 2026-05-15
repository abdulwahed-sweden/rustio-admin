//! CLI verb dispatchers for the Builder.
//!
//! Each function maps one `rustio <verb>` invocation onto the
//! underlying primitives. Keeping the dispatchers in a single
//! module makes the grep proofs in DESIGN_BUILDER.md §10 easy to
//! reason about — every Builder-side filesystem write either goes
//! through here or through one of the primitives this module
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{SecondsFormat, Utc};
use serde_json::json;

use crate::builder::draft::{Draft, Project, FIELD_TYPES};
use crate::builder::history::{append, HistoryOp};
use crate::builder::lifecycle::{
    commit as lifecycle_commit, find_project_root, plan as lifecycle_plan, CommitResult,
    FileVerdict, LifecycleError, PlanReport,
};
use crate::builder::lockfile::BuilderLock;
use crate::builder::redact::is_secret_field_type;

/// Resolve the actor identifier per §4.2.1: prefer
/// `RUSTIO_AGENT_ID`, fall back to `git config user.email`, fall
/// back to a literal `"unknown"`.
///
/// When no real actor can be resolved, the caller surfaces a
/// warning on stderr — an audit row with `actor = "unknown"` is
/// honest (per `DESIGN_AUDIT.md` §2.2 the framework never invents
/// an id) but degrades the trail. Set either env var or
/// `git config user.email` to restore attribution.
pub(crate) fn resolve_actor() -> (String, ActorSource) {
    if let Some(env) = std::env::var("RUSTIO_AGENT_ID")
        .ok()
        .filter(|s| !s.is_empty())
    {
        return (env, ActorSource::AgentEnv);
    }
    if let Ok(out) = Command::new("git")
        .args(["config", "--get", "user.email"])
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() {
                return (text, ActorSource::GitConfig);
            }
        }
    }
    ("unknown".to_string(), ActorSource::Degraded)
}

/// Where the actor identifier came from. Callers that record an
/// event use [`ActorSource::Degraded`] to decide whether to print
/// a one-time warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActorSource {
    AgentEnv,
    GitConfig,
    Degraded,
}

/// Print a stderr warning when actor attribution has degraded.
/// Idempotent within a single CLI invocation by gating on a static
/// flag — multiple events in one command produce one warning.
fn warn_if_degraded(source: ActorSource) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static WARNED: AtomicBool = AtomicBool::new(false);
    if source == ActorSource::Degraded && !WARNED.swap(true, Ordering::SeqCst) {
        eprintln!(
            "warning: could not resolve an actor for history.jsonl. \
             Set RUSTIO_AGENT_ID or `git config user.email`. \
             Recording events as `actor = \"unknown\"`."
        );
    }
}

/// Singularize a CamelCase model name to a snake_case plural table.
/// Mirrors the proc-macro's rule set in `rustio-admin-macros`.
fn plural_snake(name: &str) -> String {
    let mut snake = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            snake.push('_');
        }
        snake.push(c.to_ascii_lowercase());
    }
    if snake.ends_with('s') {
        return snake;
    }
    if snake.ends_with('x')
        || snake.ends_with('z')
        || snake.ends_with("ch")
        || snake.ends_with("sh")
    {
        return format!("{snake}es");
    }
    if let Some(stem) = snake.strip_suffix('y') {
        let before = stem.chars().last();
        if matches!(before, Some('a' | 'e' | 'i' | 'o' | 'u')) || stem.is_empty() {
            return format!("{snake}s");
        }
        return format!("{stem}ies");
    }
    format!("{snake}s")
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("project name must not be empty".into());
    }
    if !name.chars().next().unwrap().is_ascii_alphabetic() {
        return Err("project name must start with an ASCII letter".into());
    }
    for c in name.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if !ok {
            return Err(format!(
                "project name '{name}' contains invalid character {c:?}; only [A-Za-z0-9_-] allowed"
            ));
        }
    }
    Ok(())
}

fn validate_model_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("model name must not be empty".into());
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_uppercase() {
        return Err(format!(
            "model name '{name}' must be CamelCase and start with an uppercase ASCII letter"
        ));
    }
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() {
            return Err(format!(
                "model name '{name}' contains invalid character {c:?}; only ASCII letters and digits allowed"
            ));
        }
    }
    Ok(())
}

fn validate_field_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("field name must not be empty".into());
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_lowercase() {
        return Err(format!(
            "field name '{name}' must be snake_case and start with a lowercase ASCII letter"
        ));
    }
    for c in name.chars() {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(format!(
                "field name '{name}' contains invalid character {c:?}; only [a-z0-9_] allowed"
            ));
        }
    }
    // Reserved column names that conflict with the implicit
    // BIGSERIAL/created_at columns.
    if matches!(name, "id" | "created_at") {
        return Err(format!(
            "field name '{name}' is reserved; the generator emits it implicitly"
        ));
    }
    Ok(())
}

/// `rustio new <name>` — bootstrap a fresh project.
///
/// Writes:
/// - `<name>/Cargo.toml` (developer-owned after scaffolding)
/// - `<name>/src/main.rs` (developer-owned)
/// - `<name>/migrations/` (empty)
/// - `<name>/.rustio/draft.toml`
/// - `<name>/.rustio/builder.lock`
/// - `<name>/.rustio/history.jsonl` with one `project_init` event
pub(crate) fn run_new(name: &str) -> Result<String, String> {
    validate_project_name(name)?;
    let root = PathBuf::from(name);
    if root.exists() {
        return Err(format!("'{name}' already exists; refusing to overwrite"));
    }

    std::fs::create_dir_all(root.join(".rustio")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(root.join("src")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(root.join("migrations")).map_err(|e| e.to_string())?;

    let builder_version = env!("CARGO_PKG_VERSION");
    let rust_version = env!("CARGO_PKG_RUST_VERSION");
    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // Lockfile.
    let lock = BuilderLock::current();
    std::fs::write(root.join(".rustio/builder.lock"), lock.to_toml()).map_err(|e| e.to_string())?;

    // Draft seeded with project metadata; no models yet.
    let mut draft = Draft::empty();
    draft.project = Project {
        name: name.to_string(),
        rust_version: rust_version.to_string(),
        builder_pinned: builder_version.to_string(),
        created_at: created_at.clone(),
    };
    std::fs::write(root.join(".rustio/draft.toml"), draft.to_toml()).map_err(|e| e.to_string())?;

    // History — single project_init event.
    let (actor, source) = resolve_actor();
    warn_if_degraded(source);
    let history_path = root.join(".rustio/history.jsonl");
    append(
        &history_path,
        HistoryOp::ProjectInit,
        &actor,
        json!({
            "name": name,
            "rust_version": rust_version,
            "builder_pinned": builder_version,
            "created_at": created_at,
        }),
    )
    .map_err(|e| e.to_string())?;

    // Cargo.toml — developer-owned. Pins rustio-admin and the
    // minimum runtime deps the generated model files reference.
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
rust-version = "{rust_version}"

[dependencies]
rustio-admin = "{builder_version}"
tokio = {{ version = "1", features = ["macros", "rt-multi-thread"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
"#
    );
    std::fs::write(root.join("Cargo.toml"), cargo_toml).map_err(|e| e.to_string())?;

    // main.rs — developer-owned skeleton. Compiles after the
    // first `rustio commit` populates src/_generated/.
    let main_rs = "//! Project entry point. Generator scaffolds this once;\n\
                   //! thereafter it is developer-owned.\n\
                   //!\n\
                   //! Run `rustio add model <Name>` and `rustio commit` to\n\
                   //! populate `src/_generated/`. Then wire your server here.\n\
                   \n\
                   mod _generated;\n\
                   \n\
                   #[tokio::main]\n\
                   async fn main() {\n\
                   \x20   let _admin = _generated::admin::build_admin();\n\
                   \x20   println!(\"rustio: scaffold ready. Edit src/main.rs to wire your server.\");\n\
                   }\n";
    std::fs::write(root.join("src/main.rs"), main_rs).map_err(|e| e.to_string())?;

    Ok(format!(
        "Created project '{name}'.\nNext: cd {name} && rustio add model <Name> && rustio commit"
    ))
}

/// `rustio add model <Name>` — append an `add_model` event.
///
/// The generator does not run; the developer runs `rustio commit`
/// to materialize the change. Doctrine separation.
pub(crate) fn run_add_model(start: &Path, model_name: &str) -> Result<String, String> {
    validate_model_name(model_name)?;
    let root = find_project_root(start).map_err(format_lifecycle_err)?;
    let history_path = root.join(".rustio/history.jsonl");

    // Verify lock + replay so we refuse fast if the project is
    // already in a bad state.
    preflight(&root)?;

    let table = plural_snake(model_name);
    let (actor, source) = resolve_actor();
    warn_if_degraded(source);
    let id = append(
        &history_path,
        HistoryOp::AddModel,
        &actor,
        json!({"name": model_name, "table": table.clone()}),
    )
    .map_err(|e| e.to_string())?;

    // Replay everything and rewrite draft.toml from the events.
    redraft(&root)?;

    Ok(format!(
        "Recorded add_model {model_name} (table = {table}) [event {id}]\n\
         Run `rustio commit` to generate code."
    ))
}

/// `rustio add field <Model> <name> <type>` — append an `add_field`
/// event. Refuses unknown field types and reserved field names.
pub(crate) fn run_add_field(
    start: &Path,
    model_name: &str,
    field_name: &str,
    type_name: &str,
    unique: bool,
) -> Result<String, String> {
    validate_model_name(model_name)?;
    validate_field_name(field_name)?;
    if !FIELD_TYPES.contains(&type_name) {
        return Err(format!(
            "type '{type_name}' is not in the closed MVP type list {FIELD_TYPES:?}"
        ));
    }
    // Doctrine B4: secret-category field types carry implicit
    // redaction requirements (default values, log-redaction sigils
    // at the CLI boundary). The MVP has no `--default` plumbing
    // and no per-field `redact` modifier, so a secret-typed field
    // cannot be handled safely. Refuse rather than ship code that
    // would silently leak when those features land.
    if is_secret_field_type(type_name) {
        return Err(format!(
            "field type '{type_name}' is a secret-category type (DESIGN_BUILDER.md §4.2.3). \
             MVP refuses these until the `--default` and `redact = true` plumbing lands; the \
             event log has no path to redact the future defaults safely."
        ));
    }
    let root = find_project_root(start).map_err(format_lifecycle_err)?;
    preflight(&root)?;

    // Confirm the model exists before recording the event so the
    // history.jsonl never carries dangling add_field events.
    let history_path = root.join(".rustio/history.jsonl");
    let draft =
        crate::builder::replay::replay_from_file(&history_path).map_err(|e| e.to_string())?;
    if !draft.models.iter().any(|m| m.name == model_name) {
        return Err(format!(
            "model '{model_name}' is not registered; run `rustio add model {model_name}` first"
        ));
    }
    if draft
        .models
        .iter()
        .find(|m| m.name == model_name)
        .map(|m| m.fields.iter().any(|f| f.name == field_name))
        .unwrap_or(false)
    {
        return Err(format!(
            "field {model_name}.{field_name} is already declared"
        ));
    }

    let (actor, source) = resolve_actor();
    warn_if_degraded(source);
    let id = append(
        &history_path,
        HistoryOp::AddField,
        &actor,
        json!({
            "model": model_name,
            "name": field_name,
            "type": type_name,
            "required": true,
            "unique": unique,
        }),
    )
    .map_err(|e| e.to_string())?;
    redraft(&root)?;

    Ok(format!(
        "Recorded add_field {model_name}.{field_name}: {type_name} [event {id}]\n\
         Run `rustio commit` to regenerate code."
    ))
}

/// `rustio plan` — print the diff `commit` would apply.
pub(crate) fn run_plan(start: &Path) -> Result<String, String> {
    let report = lifecycle_plan(start).map_err(format_lifecycle_err)?;
    Ok(render_plan(&report))
}

/// `rustio commit` — atomic write per §6.2.
pub(crate) fn run_commit(start: &Path, force: bool) -> Result<String, String> {
    let (actor, source) = resolve_actor();
    warn_if_degraded(source);
    let result = lifecycle_commit(start, force, &actor).map_err(format_lifecycle_err)?;
    match result {
        CommitResult::NoOp => Ok("Nothing to do — project is in sync with draft.".to_string()),
        CommitResult::Wrote { event_id, files } => {
            let mut out = format!("Committed {} file(s) [event {event_id}]\n", files.len());
            for f in &files {
                out.push_str(&format!("  + {}\n", f.display()));
            }
            Ok(out)
        }
    }
}

fn preflight(root: &Path) -> Result<(), String> {
    // Lock verify.
    let lock_text =
        std::fs::read_to_string(root.join(".rustio/builder.lock")).map_err(|e| e.to_string())?;
    let lock = BuilderLock::from_toml(&lock_text).map_err(|e| e.to_string())?;
    lock.verify_against_running().map_err(|e| e.to_string())?;
    Ok(())
}

fn redraft(root: &Path) -> Result<(), String> {
    let history_path = root.join(".rustio/history.jsonl");
    let draft =
        crate::builder::replay::replay_from_file(&history_path).map_err(|e| e.to_string())?;
    std::fs::write(root.join(".rustio/draft.toml"), draft.to_toml()).map_err(|e| e.to_string())?;
    Ok(())
}

fn format_lifecycle_err(e: LifecycleError) -> String {
    e.to_string()
}

fn render_plan(report: &PlanReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Project root: {}\n\n",
        report.project_root.display()
    ));
    if report.is_no_op() {
        out.push_str("Plan: no changes. Project is in sync with draft.\n");
        return out;
    }
    out.push_str("Plan:\n");
    for entry in &report.entries {
        out.push_str(&render_entry(entry));
    }
    if let Some(m) = &report.migration {
        out.push_str(&render_entry(m));
    }
    out
}

fn render_entry(entry: &crate::builder::lifecycle::PlanEntry) -> String {
    let marker = match &entry.verdict {
        FileVerdict::Create => "+ create   ",
        FileVerdict::NoOp => "= no-op    ",
        FileVerdict::Mismatch { .. } => "! mismatch ",
        FileVerdict::Unowned => "! unowned  ",
    };
    format!("  {} {}\n", marker, entry.path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ulid_gen::new_ulid;

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "rustio-cmd-test-{}-{}",
            std::process::id(),
            new_ulid()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn plural_snake_handles_common_cases() {
        assert_eq!(plural_snake("Patient"), "patients");
        assert_eq!(plural_snake("Doctor"), "doctors");
        assert_eq!(plural_snake("BlogPost"), "blog_posts");
        assert_eq!(plural_snake("Box"), "boxes");
        assert_eq!(plural_snake("Story"), "stories");
        assert_eq!(plural_snake("Status"), "status"); // s-suffix passthrough
    }

    #[test]
    fn project_name_validator_refuses_garbage() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("1abc").is_err());
        assert!(validate_project_name("a/b").is_err());
        assert!(validate_project_name("a b").is_err());
        assert!(validate_project_name("valid-name_1").is_ok());
    }

    #[test]
    fn model_name_validator_requires_camel_case() {
        assert!(validate_model_name("patient").is_err());
        assert!(validate_model_name("Patient").is_ok());
        assert!(validate_model_name("BlogPost").is_ok());
        assert!(validate_model_name("Patient!").is_err());
    }

    #[test]
    fn field_name_validator_refuses_reserved_names() {
        assert!(validate_field_name("id").is_err());
        assert!(validate_field_name("created_at").is_err());
        assert!(validate_field_name("FullName").is_err());
        assert!(validate_field_name("full_name").is_ok());
    }

    /// End-to-end: new → add model → add field → plan → commit →
    /// commit-again-is-no-op.
    #[test]
    fn end_to_end_lifecycle() {
        // Work in an isolated CWD so `rustio new` writes into the
        // tempdir, not the repo. Hold the global guard for the
        // duration of the CWD mutation.
        let work = tempdir();
        let project_name = "demo-project";
        let project_root = work.join(project_name);

        let summary = {
            let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            let original_cwd = std::env::current_dir().unwrap();
            std::env::set_current_dir(&work).unwrap();
            let res = run_new(project_name);
            std::env::set_current_dir(&original_cwd).unwrap();
            res.unwrap()
        };
        assert!(summary.contains("Created project"));

        // Verify the .rustio/ scaffold.
        for rel in [
            ".rustio/draft.toml",
            ".rustio/history.jsonl",
            ".rustio/builder.lock",
            "Cargo.toml",
            "src/main.rs",
        ] {
            assert!(
                project_root.join(rel).exists(),
                "{rel} must exist after rustio new"
            );
        }

        // add model Patient
        let msg = run_add_model(&project_root, "Patient").unwrap();
        assert!(msg.contains("add_model Patient"));

        // add field Patient full_name text
        let msg = run_add_field(&project_root, "Patient", "full_name", "text", false).unwrap();
        assert!(msg.contains("add_field Patient.full_name"));

        // plan should list 4 creates + 1 migration create.
        let plan_out = run_plan(&project_root).unwrap();
        assert!(plan_out.contains("create"));
        assert!(plan_out.contains("0001_initial.sql"));

        // commit
        let commit_out = run_commit(&project_root, false).unwrap();
        assert!(commit_out.starts_with("Committed"));
        assert!(project_root
            .join("src/_generated/models/patient.rs")
            .exists());
        assert!(project_root.join("migrations/0001_initial.sql").exists());

        // second commit is a no-op
        let again = run_commit(&project_root, false).unwrap();
        assert!(
            again.contains("Nothing to do"),
            "second commit must be a no-op, got: {again}",
        );
    }

    /// Bootstrap a project without going through `run_new` — `run_new`
    /// writes to a CWD-relative path, and `std::env::set_current_dir`
    /// is process-global, so any test using it would race with the
    /// `end_to_end_lifecycle` test under cargo's default parallel
    /// runner. Tests that only need a project on disk (not the
    /// `rustio new` verb itself) bootstrap here.
    fn bootstrap_project_at(root: &Path) {
        std::fs::create_dir_all(root.join(".rustio")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("migrations")).unwrap();
        std::fs::write(
            root.join(".rustio/builder.lock"),
            crate::builder::lockfile::BuilderLock::current().to_toml(),
        )
        .unwrap();
        let mut draft = crate::builder::draft::Draft::empty();
        draft.project = crate::builder::draft::Project {
            name: "p".into(),
            rust_version: env!("CARGO_PKG_RUST_VERSION").into(),
            builder_pinned: env!("CARGO_PKG_VERSION").into(),
            created_at: "2026-05-15T10:30:00Z".into(),
        };
        std::fs::write(root.join(".rustio/draft.toml"), draft.to_toml()).unwrap();
        crate::builder::history::append(
            &root.join(".rustio/history.jsonl"),
            HistoryOp::ProjectInit,
            "test@example.com",
            json!({
                "name": "p",
                "rust_version": env!("CARGO_PKG_RUST_VERSION"),
                "builder_pinned": env!("CARGO_PKG_VERSION"),
                "created_at": "2026-05-15T10:30:00Z",
            }),
        )
        .unwrap();
    }

    #[test]
    fn add_field_refuses_unknown_type() {
        let root = tempdir();
        bootstrap_project_at(&root);
        run_add_model(&root, "Item").unwrap();
        let err = run_add_field(&root, "Item", "weird", "geography", false).unwrap_err();
        assert!(err.contains("not in the closed MVP type list"), "{err}");
    }

    /// MVP refuses every secret-category field type. Today the
    /// closed FIELD_TYPES check is what refuses them; the explicit
    /// `is_secret_field_type` guard is a forward-defensive tripwire
    /// for the day a secret type is added to FIELD_TYPES without
    /// matching `--default` / `redact = true` infrastructure.
    #[test]
    fn add_field_refuses_secret_category_types() {
        let root = tempdir();
        bootstrap_project_at(&root);
        run_add_model(&root, "User").unwrap();
        for ty in [
            "password",
            "secret",
            "token",
            "api_key",
            "private_key",
            "encryption_key",
        ] {
            let err = run_add_field(&root, "User", "secret_field", ty, false)
                .expect_err(&format!("type {ty} must be refused"));
            assert!(
                err.contains("not in the closed MVP type list") || err.contains("secret-category")
            );
        }
    }

    #[test]
    fn add_field_refuses_unknown_model() {
        let root = tempdir();
        bootstrap_project_at(&root);
        let err = run_add_field(&root, "Ghost", "x", "text", false).unwrap_err();
        assert!(err.contains("is not registered"), "{err}");
    }

    /// Sole test that exercises `run_new`. CWD is mutated; this test
    /// must run alone to avoid racing other tests that also touch
    /// CWD. The test runner's default parallelism is safe here only
    /// because no sibling test in this module mutates CWD.
    #[test]
    fn new_refuses_existing_directory() {
        // Use a tempdir as parent and serialize via a process-wide
        // guard so concurrent tests do not race on CWD.
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let work = tempdir();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&work).unwrap();
        std::fs::create_dir(work.join("collide")).unwrap();
        let err = run_new("collide").unwrap_err();
        std::env::set_current_dir(&original_cwd).unwrap();
        assert!(err.contains("already exists"));
    }

    /// Process-wide guard for tests that mutate CWD. Rust's default
    /// test runner shares one process; `std::env::set_current_dir`
    /// is global; concurrent CWD writes corrupt each other. Hold
    /// this mutex for the duration of any test that calls
    /// `set_current_dir`.
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
