//! `rustio memory` write path — propose / approve / apply memory entries
//! through the **shared** proposal lifecycle (`DESIGN_CLOUD_IMPL.md` §4,
//! §7). There is no parallel governance path: memory-write is a capability
//! in `.rustio/ai.toml` (`write_memory` / `supersede_memory`, both
//! `needs_approval`), and the state machine, store, and log are the ones in
//! `proposal.rs` that `rustio ai` uses.
//!
//! A memory entry is **materialised at apply**, not pre-staged: the entry's
//! `date` (apply date), `ratified_by` (approver), and `correlation_id` are
//! only known once a human approves, so the proposal carries the *draft* on
//! its `metadata` and the injected apply action builds the entry file and
//! re-renders `CLOUD.md`.
//!
//! Offline-first: identity comes from `--by`; the local `log.jsonl` is the
//! record. DB-backed identity + audit mirroring (reusing the
//! `ai_proposal_*` events) is a later slice, exactly as `rustio ai` shipped.

use std::path::Path;

use console::style;
use serde::{Deserialize, Serialize};

use rustio_admin::auth::emergency::fresh_correlation_id;

use crate::ai::{self, POLICY_PATH};
use crate::proposal::{self, Proposal, Store as ProposalStore};

use super::entry::{short, Entry, EntryType};
use super::render;
use super::store::Store as EntryStore;

/// The proposal-store subdirectory for memory (`.rustio/memory/`).
const SUBDIR: &str = "memory";

/// The memory entry draft, carried on a proposal's `metadata` between
/// propose and apply. The apply-time fields (`date`, `ratified_by`,
/// `correlation_id`) are deliberately absent — they are stamped when a
/// human ratifies the entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Draft {
    #[serde(rename = "type")]
    pub(crate) entry_type: String,
    pub(crate) subjects: Vec<String>,
    #[serde(default)]
    pub(crate) supersedes: Option<String>,
    pub(crate) foundational: bool,
    pub(crate) sources: Vec<String>,
    pub(crate) body: String,
    /// Who authored the reasoning (the proposer).
    pub(crate) author: String,
}

// ---- CLI entry points (rooted at the cwd) --------------------------------

/// `rustio memory remember` — propose a new memory entry.
#[allow(clippy::too_many_arguments)]
pub(crate) fn remember(
    entry_type: String,
    subjects: Vec<String>,
    foundational: bool,
    sources: Vec<String>,
    note: String,
    by: Option<String>,
) -> Result<(), String> {
    let draft = Draft {
        entry_type,
        subjects,
        supersedes: None,
        foundational,
        sources,
        body: note,
        author: String::new(),
    };
    propose_at(Path::new("."), "write_memory", draft, by)
}

/// `rustio memory supersede` — propose an entry that supersedes another.
#[allow(clippy::too_many_arguments)]
pub(crate) fn supersede(
    target: String,
    entry_type: String,
    subjects: Vec<String>,
    foundational: bool,
    sources: Vec<String>,
    note: String,
    by: Option<String>,
) -> Result<(), String> {
    let draft = Draft {
        entry_type,
        subjects,
        supersedes: Some(target),
        foundational,
        sources,
        body: note,
        author: String::new(),
    };
    propose_at(Path::new("."), "supersede_memory", draft, by)
}

/// `rustio memory pending` — list memory proposals awaiting a decision.
pub(crate) fn pending(by: Option<String>) -> Result<(), String> {
    let _ = by;
    pending_at(Path::new("."))
}

/// `rustio memory approve` — record one approval.
pub(crate) fn approve(id: String, by: Option<String>) -> Result<(), String> {
    let store = ProposalStore::new(".", SUBDIR);
    let p = proposal::do_approve(&store, &id, ai::whoami(by), "rustio memory apply")?;
    if p.state == proposal::State::Approved {
        println!(
            "{} memory proposal {} approved — apply with `rustio memory apply {}`",
            style("ok").green().bold(),
            p.short(),
            p.short()
        );
    } else {
        println!(
            "{} recorded approval for {} ({}/{} distinct approvers)",
            style("ok").green().bold(),
            p.short(),
            p.distinct_approvals(),
            p.required_approvals
        );
    }
    Ok(())
}

/// `rustio memory reject` — decline a proposal, keeping the record.
pub(crate) fn reject(id: String, reason: String, by: Option<String>) -> Result<(), String> {
    let store = ProposalStore::new(".", SUBDIR);
    let p = proposal::do_reject(&store, &id, &reason, ai::whoami(by))?;
    println!(
        "{} memory proposal {} rejected",
        style("ok").yellow(),
        p.short()
    );
    Ok(())
}

/// `rustio memory apply` — materialise an approved entry and re-render.
pub(crate) fn apply(id: String, by: Option<String>) -> Result<(), String> {
    apply_at(Path::new("."), id, by)
}

// ---- Root-parameterised core (testable against a temp directory) ---------

fn propose_at(
    root: &Path,
    capability: &str,
    mut draft: Draft,
    by: Option<String>,
) -> Result<(), String> {
    // Validate the draft up front so a bad entry never becomes a proposal.
    EntryType::parse(&draft.entry_type)?;
    if draft.body.trim().is_empty() {
        return Err("a memory entry needs a body — pass --note \"<the reasoning>\"".to_string());
    }

    let actor = ai::whoami(by);
    draft.author = actor.clone();

    let policy = ai::load_policy(&root.join(POLICY_PATH))?;
    let store = ProposalStore::new(root, SUBDIR);
    let title = title_from_body(&draft.body);
    let floor = if draft.foundational { 2 } else { 0 };
    let metadata =
        serde_json::to_value(&draft).map_err(|e| format!("could not encode draft: {e}"))?;

    let p = proposal::do_propose_meta(
        &store,
        &policy,
        capability,
        &title,
        None,
        Vec::new(),
        Some(metadata),
        floor,
        actor,
    )?;

    let hint = if p.required_approvals == 0 {
        format!("apply directly:\n  rustio memory apply {}", p.short())
    } else {
        format!(
            "needs {} approval(s):\n  rustio memory approve {} --by <name>",
            p.required_approvals,
            p.short()
        )
    };
    println!(
        "{} memory proposal {} created ({})\n  {}\n\n{}",
        style("rustio memory:").bold(),
        p.short(),
        capability,
        p.title,
        hint
    );
    Ok(())
}

fn pending_at(root: &Path) -> Result<(), String> {
    let store = ProposalStore::new(root, SUBDIR);
    let mut shown = 0usize;
    for p in store.load_all()? {
        if matches!(
            p.state,
            proposal::State::Suggested | proposal::State::Approved
        ) {
            println!(
                "{}  {}  [{}]  {}/{}  {}",
                style(p.short()).cyan(),
                p.capability,
                proposal::state_label(p.state),
                p.distinct_approvals(),
                p.required_approvals,
                p.title
            );
            shown += 1;
        }
    }
    if shown == 0 {
        println!("{}", style("no pending memory proposals").dim());
    }
    Ok(())
}

fn apply_at(root: &Path, id: String, by: Option<String>) -> Result<(), String> {
    let store = ProposalStore::new(root, SUBDIR);
    let actor = ai::whoami(by);
    let root_owned = root.to_path_buf();
    let act = actor.clone();
    let (p, _written) = proposal::do_apply_with(
        &store,
        &id,
        actor,
        "rustio memory approve",
        |p, apply_root| materialize(p, apply_root, &act),
    )?;

    // Re-render the human view from the (now larger) entry set (§2.6).
    let count = render::write_view(&EntryStore::new(root_owned))?;
    println!(
        "{} memory entry {} applied; CLOUD.md re-rendered ({} entr{})",
        style("ok").green().bold(),
        short(&p.id),
        count,
        if count == 1 { "y" } else { "ies" }
    );
    Ok(())
}

/// The injected apply action: turn the proposal's draft into an immutable
/// entry file, stamping the apply date, approver, and a fresh audit
/// correlation id. Returns the written path.
fn materialize(p: &Proposal, root: &Path, approver: &str) -> Result<Vec<String>, String> {
    let draft: Draft = p
        .metadata
        .clone()
        .ok_or_else(|| "memory proposal has no draft metadata".to_string())
        .and_then(|v| {
            serde_json::from_value(v).map_err(|e| format!("malformed draft metadata: {e}"))
        })?;
    let entry_type = EntryType::parse(&draft.entry_type)?;

    let entries_dir = root.join(".rustio").join("memory").join("entries");
    if let Some(target) = &draft.supersedes {
        if !entries_dir.join(format!("{target}.md")).exists() {
            return Err(format!(
                "cannot supersede unknown entry {target:?} — no such entry file"
            ));
        }
    }

    let entry = Entry {
        id: p.id.clone(),
        entry_type,
        subjects: draft.subjects,
        supersedes: draft.supersedes,
        foundational: draft.foundational,
        sources: draft.sources,
        author: draft.author,
        ratified_by: approver.to_string(),
        date: today(),
        correlation_id: fresh_correlation_id(),
        body: draft.body,
    };

    std::fs::create_dir_all(&entries_dir)
        .map_err(|e| format!("could not create {}: {e}", entries_dir.display()))?;
    let path = entries_dir.join(format!("{}.md", entry.id));
    std::fs::write(&path, entry.to_file_string())
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(vec![format!(".rustio/memory/entries/{}.md", entry.id)])
}

/// Today's date in the entry order-key format (`YYYY-MM-DD`, UTC).
fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// A short proposal title from the body's first line (trimmed to 72 chars).
fn title_from_body(body: &str) -> String {
    let first = body.lines().next().unwrap_or("").trim();
    if first.chars().count() <= 72 {
        first.to_string()
    } else {
        let truncated: String = first.chars().take(71).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ulid_gen::new_ulid;

    fn temp_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("rustio-memwrite-{}", new_ulid()));
        std::fs::create_dir_all(&root).expect("temp dir");
        root
    }

    fn draft(body: &str) -> Draft {
        Draft {
            entry_type: "rejected".to_string(),
            subjects: vec!["jobs".to_string()],
            supersedes: None,
            foundational: false,
            sources: vec![],
            body: body.to_string(),
            author: String::new(),
        }
    }

    #[test]
    fn propose_then_apply_writes_entry_and_renders() {
        let root = temp_root();
        propose_at(
            &root,
            "write_memory",
            draft("Rejected LISTEN/NOTIFY"),
            Some("amir".into()),
        )
        .expect("propose");

        // One pending proposal; needs 1 approval (needs_approval default).
        let store = ProposalStore::new(&root, SUBDIR);
        let p = store
            .load_all()
            .unwrap()
            .into_iter()
            .next()
            .expect("a proposal");
        assert_eq!(p.required_approvals, 1);

        // Apply before approval must fail.
        assert!(apply_at(&root, p.id.clone(), Some("amir".into())).is_err());

        // Approve (distinct approver) then apply.
        proposal::do_approve(&store, &p.id, "amir".into(), "x").expect("approve");
        apply_at(&root, p.id.clone(), Some("sara".into())).expect("apply");

        // Entry file exists and CLOUD.md verifies fresh.
        let entry_path = root
            .join(".rustio/memory/entries")
            .join(format!("{}.md", p.id));
        assert!(entry_path.exists(), "entry file written");
        let entry_store = EntryStore::new(&root);
        render::verify(&entry_store).expect("CLOUD.md fresh after apply");
    }

    #[test]
    fn foundational_requires_two_approvers() {
        let root = temp_root();
        let mut d = draft("Internal-only forever");
        d.entry_type = "intent".to_string();
        d.foundational = true;
        propose_at(&root, "write_memory", d, Some("amir".into())).expect("propose");

        let store = ProposalStore::new(&root, SUBDIR);
        let p = store.load_all().unwrap().into_iter().next().unwrap();
        assert_eq!(
            p.required_approvals, 2,
            "foundational entry needs two approvers"
        );
    }

    #[test]
    fn unknown_type_is_rejected_at_propose() {
        let root = temp_root();
        let mut d = draft("x");
        d.entry_type = "musing".to_string();
        let err = propose_at(&root, "write_memory", d, Some("amir".into())).unwrap_err();
        assert!(err.contains("unknown entry type"), "{err}");
    }

    #[test]
    fn empty_body_is_rejected_at_propose() {
        let root = temp_root();
        let err = propose_at(&root, "write_memory", draft("   "), Some("amir".into())).unwrap_err();
        assert!(err.contains("needs a body"), "{err}");
    }

    #[test]
    fn supersede_unknown_target_fails_at_apply() {
        let root = temp_root();
        let mut d = draft("replaces a ghost");
        d.entry_type = "decision".to_string();
        d.supersedes = Some("ghost-entry".to_string());
        propose_at(&root, "supersede_memory", d, Some("amir".into())).expect("propose");

        let store = ProposalStore::new(&root, SUBDIR);
        let p = store.load_all().unwrap().into_iter().next().unwrap();
        proposal::do_approve(&store, &p.id, "amir".into(), "x").expect("approve");
        let err = apply_at(&root, p.id, Some("sara".into())).unwrap_err();
        assert!(err.contains("cannot supersede unknown entry"), "{err}");
    }

    #[test]
    fn title_truncates_long_first_line() {
        let long = "x".repeat(100);
        let t = title_from_body(&long);
        assert!(t.chars().count() <= 72, "len {}", t.chars().count());
        assert!(t.ends_with('…'));
    }
}
