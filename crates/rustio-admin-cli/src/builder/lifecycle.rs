//! The `plan` / `commit` lifecycle (§6, §5.4).
//!
//! `DESIGN_BUILDER.md` §6 binds two verbs:
//!
//! - **`plan`** (Doctrine B8): read-only. Computes what `commit`
//!   would do and returns a structured diff.
//! - **`commit`** (Doctrine B8): atomic. Either every file in the
//!   plan is written, or none are. Implementation uses
//!   `.rustio/tmp/<txn>/` as a staging area before swapping
//!   into place.
//!
//! ## Overwrite contract (§5.4)
//!
//! For each file under `src/_generated/` the planner emits:
//!
//! 1. **Absent on disk** → `create`.
//! 2. **Header matches, SchemaHash matches** → `no_op`.
//! 3. **Header parses, hash mismatch** → `overwrite_mismatch`.
//!    `commit` refuses unless `--force` is passed.
//! 4. **Header missing or malformed** → `unowned`. Same `--force`
//!    discipline.
//!
//! ## Migration discipline (§7.1, §7.6 + MVP scope note)
//!
//! Migrations are append-only (Doctrine B6). The MVP supports only
//! the *first* migration:
//!
//! - `migrations/` empty → emit `0001_initial.sql`.
//! - `migrations/` non-empty and the draft matches the last commit
//!   event → idempotent no-op.
//! - `migrations/` non-empty and the draft has diverged → **refuse**.
//!   Incremental migration support is a future doctrine-aware
//!   commit; MVP correctness prefers refusal over silent partial
//!   regeneration.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::builder::codegen::{generate_all, GeneratedFile};
use crate::builder::draft::{Draft, DraftError};
use crate::builder::history::{append, HistoryOp};
use crate::builder::lockfile::{BuilderLock, LockError};

/// Classification of a single generated file against its on-disk
/// state, per §5.4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileVerdict {
    /// File absent — `commit` writes it.
    Create,
    /// File present and SchemaHash matches — `commit` skips it.
    NoOp,
    /// File present, header parses, hash mismatch — `commit`
    /// refuses unless `--force`.
    Mismatch { found_hash: String },
    /// File present but header missing or malformed.
    Unowned,
}

/// One entry in a [`PlanReport`].
#[derive(Debug, Clone)]
pub(crate) struct PlanEntry {
    pub path: PathBuf,
    pub verdict: FileVerdict,
    pub schema_hash: String,
}

/// Result of `rustio plan`.
#[derive(Debug, Clone)]
pub(crate) struct PlanReport {
    pub project_root: PathBuf,
    pub entries: Vec<PlanEntry>,
    /// Migration that would be emitted, if any.
    pub migration: Option<PlanEntry>,
}

impl PlanReport {
    /// `true` iff `commit` would write nothing.
    pub(crate) fn is_no_op(&self) -> bool {
        self.entries
            .iter()
            .all(|e| matches!(e.verdict, FileVerdict::NoOp))
            && self
                .migration
                .as_ref()
                .map(|m| matches!(m.verdict, FileVerdict::NoOp))
                .unwrap_or(true)
    }

    /// All non-NoOp entries (the things `commit` would write).
    pub(crate) fn writable_entries(&self) -> Vec<&PlanEntry> {
        let mut v: Vec<&PlanEntry> = self
            .entries
            .iter()
            .filter(|e| !matches!(e.verdict, FileVerdict::NoOp))
            .collect();
        if let Some(m) = &self.migration {
            if !matches!(m.verdict, FileVerdict::NoOp) {
                v.push(m);
            }
        }
        v
    }

    /// Entries that would require `--force` to overwrite.
    pub(crate) fn mismatched_entries(&self) -> Vec<&PlanEntry> {
        let mut v: Vec<&PlanEntry> = self
            .entries
            .iter()
            .filter(|e| {
                matches!(
                    e.verdict,
                    FileVerdict::Mismatch { .. } | FileVerdict::Unowned
                )
            })
            .collect();
        if let Some(m) = &self.migration {
            if matches!(
                m.verdict,
                FileVerdict::Mismatch { .. } | FileVerdict::Unowned
            ) {
                v.push(m);
            }
        }
        v
    }
}

/// Errors returned by the lifecycle.
#[derive(Debug)]
pub(crate) enum LifecycleError {
    Io(std::io::Error),
    Draft(DraftError),
    Lock(LockError),
    /// No `.rustio/` directory found walking up from CWD.
    NotInProject {
        cwd: PathBuf,
    },
    /// `migrations/` already contains files but the draft diverges
    /// from the last commit. MVP refuses incremental migrations.
    IncrementalMigrationOutOfScope,
    /// Plan has mismatched files but `--force` was not passed.
    RefusedMismatch {
        mismatched: Vec<PathBuf>,
    },
    /// Reading on-disk `draft.toml` failed.
    DraftRead(PathBuf, std::io::Error),
    /// `draft.toml` is absent on disk.
    DraftMissing(PathBuf),
    /// The on-disk `draft.toml` does not match the canonical
    /// serialization of the in-memory draft (hand-edit drift).
    DraftDrift,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleError::Io(e) => write!(f, "I/O error: {e}"),
            LifecycleError::Draft(e) => write!(f, "{e}"),
            LifecycleError::Lock(e) => write!(f, "{e}"),
            LifecycleError::NotInProject { cwd } => write!(
                f,
                "no .rustio/ directory found at {} or any parent. Run `rustio new <name>` first.",
                cwd.display()
            ),
            LifecycleError::IncrementalMigrationOutOfScope => write!(
                f,
                "migrations/ contains existing files and draft.toml has diverged. \
                 Incremental migrations are out of MVP scope. \
                 Either reset .rustio/ or wait for an incremental-migration release."
            ),
            LifecycleError::RefusedMismatch { mismatched } => {
                writeln!(
                    f,
                    "{} generated file(s) carry a SchemaHash that does not match the current draft:",
                    mismatched.len()
                )?;
                for p in mismatched {
                    writeln!(f, "  - {}", p.display())?;
                }
                write!(
                    f,
                    "DESIGN_BUILDER.md §5.4 forbids silent overwrite. Re-run with `--force` \
                     to acknowledge that prior content will be moved to .rustio/forced/."
                )
            }
            LifecycleError::DraftRead(p, e) => write!(f, "could not read {}: {e}", p.display()),
            LifecycleError::DraftMissing(p) => {
                write!(f, "{} not found. Is this a rustio project?", p.display())
            }
            LifecycleError::DraftDrift => write!(
                f,
                "the on-disk .rustio/draft.toml has been hand-edited. \
                 Reconcile with the event log before running this command \
                 (rustio status / rustio merge — out of MVP scope)."
            ),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<std::io::Error> for LifecycleError {
    fn from(e: std::io::Error) -> Self {
        LifecycleError::Io(e)
    }
}

impl From<DraftError> for LifecycleError {
    fn from(e: DraftError) -> Self {
        LifecycleError::Draft(e)
    }
}

impl From<LockError> for LifecycleError {
    fn from(e: LockError) -> Self {
        LifecycleError::Lock(e)
    }
}

/// Walk up from `start` looking for a directory that contains a
/// `.rustio/` subdirectory. That directory is the project root.
pub(crate) fn find_project_root(start: &Path) -> Result<PathBuf, LifecycleError> {
    let mut cur = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    loop {
        if cur.join(".rustio").is_dir() {
            return Ok(cur);
        }
        match cur.parent() {
            Some(p) if p != cur => cur = p.to_path_buf(),
            _ => {
                return Err(LifecycleError::NotInProject {
                    cwd: start.to_path_buf(),
                });
            }
        }
    }
}

/// Read the on-disk draft.toml, verify the builder.lock pin, and
/// return the parsed [`Draft`]. Shared between `plan` and `commit`.
fn load_draft_and_verify(project_root: &Path) -> Result<Draft, LifecycleError> {
    // Doctrine B11 — version pin check.
    let lock_path = project_root.join(".rustio/builder.lock");
    let lock_text = std::fs::read_to_string(&lock_path)
        .map_err(|e| LifecycleError::DraftRead(lock_path.clone(), e))?;
    let lock = BuilderLock::from_toml(&lock_text)?;
    lock.verify_against_running()?;

    // Draft.
    let draft_path = project_root.join(".rustio/draft.toml");
    if !draft_path.exists() {
        return Err(LifecycleError::DraftMissing(draft_path));
    }
    let draft_text = std::fs::read_to_string(&draft_path)
        .map_err(|e| LifecycleError::DraftRead(draft_path.clone(), e))?;
    let draft = Draft::from_toml(&draft_text)?;

    // Soft drift check: the on-disk text should equal the canonical
    // emission of the parsed draft. Hand-edits that introduce
    // formatting drift surface here; semantic drift surfaces
    // anyway because subsequent emissions differ.
    if draft.to_toml() != draft_text {
        return Err(LifecycleError::DraftDrift);
    }

    Ok(draft)
}

/// Compute the plan without writing anything (Doctrine B8).
pub(crate) fn plan(start: &Path) -> Result<PlanReport, LifecycleError> {
    let project_root = find_project_root(start)?;
    let draft = load_draft_and_verify(&project_root)?;
    let files = generate_all(&draft);
    let migrations_dir = project_root.join("migrations");
    let migrations_existing = list_existing_migrations(&migrations_dir)?;

    let mut entries = Vec::new();
    let mut migration = None;

    for file in files {
        let is_migration = file
            .path
            .components()
            .any(|c| c.as_os_str() == "migrations");
        let target = project_root.join(&file.path);
        let verdict = classify(&target, &file);

        let entry = PlanEntry {
            path: file.path.clone(),
            verdict,
            schema_hash: file.schema_hash.clone(),
        };

        if is_migration {
            // MVP: only the initial migration is permitted. After
            // first commit, the only valid verdict on a migration
            // is NoOp; any other classification means the draft
            // has diverged in a way that would either create a
            // second migration or edit an existing one. Both
            // violate Doctrine B6. `--force` does not bypass this.
            if !migrations_existing.is_empty() && !matches!(entry.verdict, FileVerdict::NoOp) {
                return Err(LifecycleError::IncrementalMigrationOutOfScope);
            }
            migration = Some(entry);
        } else {
            entries.push(entry);
        }
    }

    Ok(PlanReport {
        project_root,
        entries,
        migration,
    })
}

fn classify(target: &Path, file: &GeneratedFile) -> FileVerdict {
    if !target.exists() {
        return FileVerdict::Create;
    }
    let existing = match std::fs::read_to_string(target) {
        Ok(s) => s,
        Err(_) => return FileVerdict::Unowned,
    };
    // Identical bytes → genuine no-op (idempotence).
    if existing == file.content {
        return FileVerdict::NoOp;
    }
    // Look for the header hash. If we can't find it, the file
    // is not generator-owned shape.
    if let Some(found) = parse_header_hash(&existing) {
        if found == file.schema_hash {
            // Hash matches but content differs — should not happen
            // under canonical serialization, but if it does, treat
            // as "owned but stale" → still safe to overwrite, so
            // classify NoOp would lie; emit Create-equivalent by
            // calling it a Mismatch with the same hash so the
            // operator sees the diff. Conservative.
            return FileVerdict::Mismatch { found_hash: found };
        }
        return FileVerdict::Mismatch { found_hash: found };
    }
    FileVerdict::Unowned
}

fn parse_header_hash(content: &str) -> Option<String> {
    for line in content.lines().take(20) {
        // Accept both `//` (Rust) and `--` (SQL) headers.
        let stripped = line
            .trim_start_matches("// ")
            .trim_start_matches("-- ")
            .trim_start_matches("//")
            .trim_start_matches("--");
        if let Some(rest) = stripped.strip_prefix("SPDX-SchemaHash:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn list_existing_migrations(dir: &Path) -> Result<Vec<PathBuf>, LifecycleError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) == Some("sql") {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

/// Execute the plan and write files. Atomic via staged-write into
/// `.rustio/tmp/<txn>/`. Doctrine B8.
///
/// Returns `Ok(None)` for a true no-op (no files changed, no event
/// emitted) — preserving §6.4 idempotence.
/// Returns `Ok(Some(report))` after a successful write with the
/// commit event ULID populated in `report.committed_event_id`.
pub(crate) fn commit(
    start: &Path,
    force: bool,
    actor: &str,
) -> Result<CommitResult, LifecycleError> {
    let report = plan(start)?;
    if report.is_no_op() {
        return Ok(CommitResult::NoOp);
    }
    if !force {
        let mismatched = report.mismatched_entries();
        if !mismatched.is_empty() {
            return Err(LifecycleError::RefusedMismatch {
                mismatched: mismatched.iter().map(|e| e.path.clone()).collect(),
            });
        }
    }

    let project_root = report.project_root.clone();

    // Stage: write every non-NoOp file to .rustio/tmp/<ulid>/ first.
    let txn_id = crate::builder::ulid_gen::new_ulid();
    let tmp = project_root.join(".rustio/tmp").join(&txn_id);
    std::fs::create_dir_all(&tmp)?;
    // We re-run generate_all so we have the content again (plan
    // only carries hashes, not content — keeping PlanReport small).
    let draft = load_draft_and_verify(&project_root)?;
    let files = generate_all(&draft);
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new(); // (final, tmp)
    for file in &files {
        let final_path = project_root.join(&file.path);
        // Skip files that are no-ops on disk.
        if let Some(entry) = report
            .entries
            .iter()
            .chain(report.migration.as_ref())
            .find(|e| e.path == file.path)
        {
            if matches!(entry.verdict, FileVerdict::NoOp) {
                continue;
            }
        }
        let tmp_path = tmp.join(&file.path);
        if let Some(parent) = tmp_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&tmp_path, &file.content)?;
        staged.push((final_path, tmp_path));
    }

    // §5.4 case 3: `--force` quarantines prior content under
    // `.rustio/forced/<ts>/<path>` before overwrite. MVP applies the
    // same discipline regardless of which `--force` case triggered.
    let forced_dir = if force {
        let ts = crate::builder::ulid_gen::new_ulid();
        Some(project_root.join(".rustio/forced").join(ts))
    } else {
        None
    };

    // Swap. Best-effort atomic per file: rename if destination
    // parent exists. Errors here leave partial state in tmp/ —
    // recovery is manual but the doctrine acknowledges this risk.
    let mut written: Vec<PathBuf> = Vec::new();
    for (final_path, tmp_path) in &staged {
        if let Some(parent) = final_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if force && final_path.exists() {
            if let Some(quarantine_root) = &forced_dir {
                let rel = final_path.strip_prefix(&project_root).unwrap_or(final_path);
                let dest = quarantine_root.join(rel);
                if let Some(p) = dest.parent() {
                    std::fs::create_dir_all(p)?;
                }
                std::fs::copy(final_path, &dest)?;
            }
        }
        std::fs::rename(tmp_path, final_path)?;
        written.push(final_path.clone());
    }
    let _ = std::fs::remove_dir_all(&tmp);

    // Append the commit event last so it records the actual
    // outcome. If this fails, the files are on disk but the log
    // does not record them — a known weak spot tracked in the
    // doctrine review (REVIEW_BUILDER_DOCTRINE.md §1.16, OPEN).
    let history_path = project_root.join(".rustio/history.jsonl");
    let files_rel: Vec<String> = written
        .iter()
        .filter_map(|p| {
            p.strip_prefix(&project_root)
                .ok()
                .map(|r| r.to_string_lossy().to_string())
        })
        .collect();
    let id = append(
        &history_path,
        HistoryOp::Commit,
        actor,
        json!({
            "files": files_rel,
            "txn": txn_id,
        }),
    )?;

    Ok(CommitResult::Wrote {
        event_id: id,
        files: written,
    })
}

#[derive(Debug)]
pub(crate) enum CommitResult {
    /// Plan was empty; nothing changed; no event emitted (§6.4).
    NoOp,
    /// At least one file was written; a `commit` event was logged.
    Wrote {
        event_id: String,
        files: Vec<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::draft::{Field, Model, Project};
    use crate::builder::history::{append, HistoryOp};
    use crate::builder::lockfile::BuilderLock;
    use crate::builder::ulid_gen::new_ulid;
    use serde_json::json;

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "rustio-lifecycle-test-{}-{}",
            std::process::id(),
            new_ulid()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    /// Bootstrap a minimal project tree on disk so lifecycle calls
    /// can run end-to-end. Returns the project root.
    fn bootstrap_project(models: Vec<Model>) -> PathBuf {
        let root = tempdir();
        let rustio_dir = root.join(".rustio");
        std::fs::create_dir_all(&rustio_dir).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();

        // Lockfile.
        std::fs::write(
            rustio_dir.join("builder.lock"),
            BuilderLock::current().to_toml(),
        )
        .unwrap();

        // History — project_init then add_model events for each
        // model so replay/draft round-trip is consistent.
        let actor = "test@example.com";
        let history_path = rustio_dir.join("history.jsonl");
        append(
            &history_path,
            HistoryOp::ProjectInit,
            actor,
            json!({
                "name": "demo",
                "rust_version": "1.88",
                "builder_pinned": env!("CARGO_PKG_VERSION"),
                "created_at": "2026-05-15T10:30:00Z",
            }),
        )
        .unwrap();
        for m in &models {
            append(
                &history_path,
                HistoryOp::AddModel,
                actor,
                json!({"name": m.name, "table": m.table}),
            )
            .unwrap();
            for f in &m.fields {
                append(
                    &history_path,
                    HistoryOp::AddField,
                    actor,
                    json!({
                        "model": m.name,
                        "name": f.name,
                        "type": f.r#type,
                        "required": f.required,
                        "unique": f.unique,
                    }),
                )
                .unwrap();
            }
        }

        // Draft.toml.
        let draft = Draft {
            schema_version: 1,
            project: Project {
                name: "demo".into(),
                rust_version: "1.88".into(),
                builder_pinned: env!("CARGO_PKG_VERSION").into(),
                created_at: "2026-05-15T10:30:00Z".into(),
            },
            models,
        };
        std::fs::write(rustio_dir.join("draft.toml"), draft.to_toml()).unwrap();

        root
    }

    fn sample_model() -> Model {
        Model {
            name: "Patient".into(),
            table: "patients".into(),
            fields: vec![Field {
                name: "full_name".into(),
                r#type: "text".into(),
                required: true,
                unique: false,
            }],
        }
    }

    #[test]
    fn plan_on_fresh_project_lists_all_creates() {
        let root = bootstrap_project(vec![sample_model()]);
        let report = plan(&root).unwrap();
        assert!(!report.is_no_op());
        let creates: Vec<_> = report
            .entries
            .iter()
            .filter(|e| matches!(e.verdict, FileVerdict::Create))
            .collect();
        // Expect mod.rs, admin.rs, models/mod.rs, models/patient.rs.
        assert_eq!(creates.len(), 4, "{:?}", report.entries);
        assert!(report.migration.is_some());
        assert!(matches!(
            report.migration.as_ref().unwrap().verdict,
            FileVerdict::Create
        ));
    }

    #[test]
    fn commit_then_replan_is_no_op() {
        let root = bootstrap_project(vec![sample_model()]);
        let r = commit(&root, false, "alice@example.com").unwrap();
        assert!(matches!(r, CommitResult::Wrote { .. }));

        let report = plan(&root).unwrap();
        assert!(
            report.is_no_op(),
            "plan after commit should be no-op: {:?}",
            report
        );
    }

    #[test]
    fn commit_is_idempotent() {
        // §6.4 — second commit with no intervening change is a true
        // no-op: no files modified, no event appended.
        let root = bootstrap_project(vec![sample_model()]);
        commit(&root, false, "alice@example.com").unwrap();

        let history_before = std::fs::read_to_string(root.join(".rustio/history.jsonl")).unwrap();
        let r = commit(&root, false, "alice@example.com").unwrap();
        assert!(matches!(r, CommitResult::NoOp));
        let history_after = std::fs::read_to_string(root.join(".rustio/history.jsonl")).unwrap();
        assert_eq!(
            history_before, history_after,
            "no-op commit must not append to history.jsonl"
        );
    }

    #[test]
    fn commit_writes_every_generated_file() {
        let root = bootstrap_project(vec![sample_model()]);
        commit(&root, false, "alice@example.com").unwrap();
        for rel in [
            "src/_generated/mod.rs",
            "src/_generated/admin.rs",
            "src/_generated/models/mod.rs",
            "src/_generated/models/patient.rs",
            "migrations/0001_initial.sql",
        ] {
            assert!(
                root.join(rel).exists(),
                "expected {rel} to exist after commit"
            );
        }
    }

    /// §5.4 case 3: hand-edited generated file whose
    /// SchemaHash no longer matches the doc-header on disk. Re-
    /// running `commit` without `--force` must refuse to silently
    /// overwrite developer work, even when the underlying draft is
    /// unchanged.
    #[test]
    fn commit_refuses_mismatched_generated_file_without_force() {
        let root = bootstrap_project(vec![sample_model()]);
        commit(&root, false, "alice@example.com").unwrap();

        // Hand-edit a generated file. Change body AND the recorded
        // SchemaHash so the file *parses* as generator-owned but
        // the hash mismatches what the current draft would emit.
        let path = root.join("src/_generated/models/patient.rs");
        let edited = std::fs::read_to_string(&path)
            .unwrap()
            // Substitute a clearly-wrong hash so the parsed header
            // does not match what the current draft would produce.
            .replace("SPDX-SchemaHash: sha256:", "SPDX-SchemaHash: sha256:0000")
            // Touch the body too so the bytes legitimately differ.
            + "\n// developer hand-edit\n";
        std::fs::write(&path, edited).unwrap();

        let err = commit(&root, false, "alice@example.com")
            .expect_err("must refuse mismatched overwrite without --force");
        assert!(
            matches!(err, LifecycleError::RefusedMismatch { .. }),
            "{err:?}"
        );
    }

    /// §5.4 case 3 with `--force`: prior content must be
    /// quarantined to `.rustio/forced/<ts>/<path>` before overwrite.
    #[test]
    fn force_overwrite_quarantines_prior_content() {
        let root = bootstrap_project(vec![sample_model()]);
        commit(&root, false, "alice@example.com").unwrap();

        // Hand-edit and corrupt the header hash so the planner
        // sees a Mismatch verdict for this file.
        let path = root.join("src/_generated/models/patient.rs");
        let edited = std::fs::read_to_string(&path)
            .unwrap()
            .replace("SPDX-SchemaHash: sha256:", "SPDX-SchemaHash: sha256:dead")
            + "\n// developer hand-edit\n";
        std::fs::write(&path, edited).unwrap();

        commit(&root, true, "alice@example.com").expect("--force must succeed");

        let forced_dir = root.join(".rustio/forced");
        assert!(forced_dir.exists(), "forced/ must be created");
        let mut found = false;
        for entry in std::fs::read_dir(&forced_dir).unwrap() {
            let p = entry
                .unwrap()
                .path()
                .join("src/_generated/models/patient.rs");
            if p.exists()
                && std::fs::read_to_string(&p)
                    .unwrap()
                    .contains("developer hand-edit")
            {
                found = true;
            }
        }
        assert!(found, "prior content not quarantined to .rustio/forced/");
    }

    #[test]
    fn incremental_migration_refused() {
        // Bootstrap, commit, then add another model and try to
        // commit again — MVP must refuse the new migration emission.
        let root = bootstrap_project(vec![sample_model()]);
        commit(&root, false, "alice@example.com").unwrap();
        let history_path = root.join(".rustio/history.jsonl");
        append(
            &history_path,
            HistoryOp::AddModel,
            "alice@example.com",
            json!({"name": "Doctor", "table": "doctors"}),
        )
        .unwrap();
        let replayed = crate::builder::replay::replay_from_file(&history_path).unwrap();
        std::fs::write(root.join(".rustio/draft.toml"), replayed.to_toml()).unwrap();

        let err = commit(&root, false, "alice@example.com").expect_err("must refuse");
        assert!(
            matches!(err, LifecycleError::IncrementalMigrationOutOfScope),
            "{err:?}"
        );
    }

    #[test]
    fn find_project_root_walks_up() {
        let root = bootstrap_project(vec![]);
        let deep = root.join("src/_generated/models");
        std::fs::create_dir_all(&deep).unwrap();
        let found = find_project_root(&deep).unwrap();
        assert_eq!(found, root.canonicalize().unwrap_or(root));
    }

    #[test]
    fn find_project_root_refuses_outside() {
        let dir = tempdir(); // no .rustio/ here
        let err = find_project_root(&dir).expect_err("must refuse");
        assert!(matches!(err, LifecycleError::NotInProject { .. }));
    }

    #[test]
    fn lockfile_version_mismatch_refused() {
        let root = bootstrap_project(vec![sample_model()]);
        // Forge a wrong-version lockfile.
        let mut lock = BuilderLock::current();
        lock.builder = "99.99.99".into();
        std::fs::write(root.join(".rustio/builder.lock"), lock.to_toml()).unwrap();
        let err = plan(&root).expect_err("must refuse");
        assert!(matches!(err, LifecycleError::Lock(_)), "{err:?}");
    }
}
