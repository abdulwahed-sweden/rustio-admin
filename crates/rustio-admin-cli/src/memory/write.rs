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
//! Identity: `--by` records an offline approver (the local `log.jsonl` is
//! the record); `--as <email>` authenticates against the DB (active user,
//! role ≥ the policy's `approver_role`, via the shared
//! [`ai::resolve_approver`]) and mirrors the decision into
//! `rustio_admin_actions` — reusing the `ai_proposal_*` events for
//! write/supersede and the typed `memory_redacted` event for redaction
//! (`DESIGN_CLOUD_IMPL.md` §6).

use std::path::Path;

use console::style;
use serde::{Deserialize, Serialize};

use rustio_admin::admin::audit::{record, ActionType, AuditEvent, LogEntry as AuditLogEntry};
use rustio_admin::auth::emergency::fresh_correlation_id;
use rustio_admin::auth::StoredUser;
use rustio_admin::Db;

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

/// `rustio memory approve` — record one approval. `--by` is offline;
/// `--as <email>` authenticates against the DB and mirrors to the audit
/// trail.
pub(crate) fn approve(
    id: String,
    by: Option<String>,
    as_user: Option<String>,
) -> Result<(), String> {
    match as_user {
        Some(email) => approve_db(Path::new("."), &id, &email),
        None => {
            let store = ProposalStore::new(".", SUBDIR);
            let p = proposal::do_approve(&store, &id, ai::whoami(by), "rustio memory apply")?;
            print_approved(&p, None);
            Ok(())
        }
    }
}

/// `rustio memory reject` — decline a proposal, keeping the record.
pub(crate) fn reject(
    id: String,
    reason: String,
    by: Option<String>,
    as_user: Option<String>,
) -> Result<(), String> {
    match as_user {
        Some(email) => reject_db(Path::new("."), &id, &reason, &email),
        None => {
            let store = ProposalStore::new(".", SUBDIR);
            let p = proposal::do_reject(&store, &id, &reason, ai::whoami(by))?;
            println!(
                "{} memory proposal {} rejected",
                style("ok").yellow(),
                p.short()
            );
            Ok(())
        }
    }
}

/// `rustio memory apply` — materialise an approved entry and re-render.
pub(crate) fn apply(id: String, by: Option<String>, as_user: Option<String>) -> Result<(), String> {
    match as_user {
        Some(email) => apply_db(Path::new("."), &id, &email),
        None => apply_at(Path::new("."), id, by),
    }
}

fn print_approved(p: &Proposal, corr: Option<&str>) {
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
    if let Some(c) = corr {
        println!("  audit: rustio_admin_actions row written (correlation {c})");
    }
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

/// The injected apply action, shared by the offline and DB paths.
/// Redaction edits an existing entry in place; everything else materialises
/// a new one (§3.4).
fn apply_action(p: &Proposal, root: &Path, actor: &str) -> Result<Vec<String>, String> {
    if p.capability == "redact_memory" {
        redact_materialize(p, root, actor)
    } else {
        materialize(p, root, actor)
    }
}

/// Print the outcome of an apply, including the redaction history-scrub
/// warning (DESIGN_CLOUD.md §3) and the optional audit correlation id.
fn print_apply_result(p: &Proposal, count: usize, corr: Option<&str>) {
    let plural = if count == 1 { "y" } else { "ies" };
    if p.capability == "redact_memory" {
        println!(
            "{} memory entry redacted; CLOUD.md re-rendered ({count} entr{plural})",
            style("ok").green().bold()
        );
        println!(
            "{} redaction cleans the working tree only — the prohibited content remains in git history.\n  A genuinely leaked secret additionally requires:\n    1. rotating / invalidating the secret, and\n    2. rewriting history across all clones (git filter-repo / BFG).",
            style("warning:").yellow().bold()
        );
    } else {
        println!(
            "{} memory entry {} applied; CLOUD.md re-rendered ({count} entr{plural})",
            style("ok").green().bold(),
            short(&p.id)
        );
    }
    if let Some(c) = corr {
        println!("  audit: rustio_admin_actions row written (correlation {c})");
    }
}

fn apply_at(root: &Path, id: String, by: Option<String>) -> Result<(), String> {
    let store = ProposalStore::new(root, SUBDIR);
    let actor = ai::whoami(by);
    let act = actor.clone();
    let (p, _written) =
        proposal::do_apply_with(&store, &id, actor, "rustio memory approve", |p, r| {
            apply_action(p, r, &act)
        })?;
    let count = render::write_view(&EntryStore::new(root))?;
    print_apply_result(&p, count, None);
    Ok(())
}

// ---- DB-backed identity + audit mirror (`--as`) --------------------------

/// Write one mirrored row into `rustio_admin_actions`, attributed to the
/// acting user, with the memory proposal in `metadata`. Returns the
/// correlation id. Identity resolution is shared with `ai`
/// ([`ai::resolve_approver`]); only the audit phrasing + event differ.
async fn mirror(
    db: &Db,
    user: &StoredUser,
    p: &Proposal,
    event: AuditEvent,
    action: &str,
    extra: serde_json::Value,
) -> Result<String, String> {
    let correlation_id = fresh_correlation_id();
    let mut md = serde_json::Map::new();
    md.insert("proposal_id".into(), p.id.clone().into());
    md.insert("capability".into(), p.capability.clone().into());
    md.insert("title".into(), p.title.clone().into());
    md.insert("action".into(), action.to_string().into());
    if let serde_json::Value::Object(extra) = extra {
        for (k, v) in extra {
            md.insert(k, v);
        }
    }
    let summary = format!("memory proposal {} {}: {}", p.short(), action, p.title);
    let entry = AuditLogEntry {
        user_id: user.id,
        action_type: ActionType::Update,
        model_name: "users",
        object_id: user.id,
        ip_address: None,
        summary,
        correlation_id: Some(&correlation_id),
        session_id: None,
        metadata: Some(serde_json::Value::Object(md)),
        actor_user_id: None,
        event: Some(event),
    };
    record(db, entry)
        .await
        .map_err(|e| format!("audit record failed: {e}"))?;
    Ok(correlation_id)
}

fn approve_db(root: &Path, id: &str, email: &str) -> Result<(), String> {
    let policy = ai::load_policy(&root.join(POLICY_PATH))?;
    let store = ProposalStore::new(root, SUBDIR);
    let mut captured: Option<(Proposal, String)> = None;
    crate::tokio_run(async {
        let db = crate::db().await?;
        let user = ai::resolve_approver(&db, email, &policy).await?;
        let p = proposal::do_approve(&store, id, user.email.clone(), "rustio memory apply")?;
        let extra = serde_json::json!({
            "approvals": p.distinct_approvals(),
            "required_approvals": p.required_approvals,
            "state": proposal::state_label(p.state),
        });
        let corr = mirror(
            &db,
            &user,
            &p,
            AuditEvent::AiProposalApproved,
            "approved",
            extra,
        )
        .await
        .map_err(|e| {
            format!(
                "proposal {} is now {} locally, but the audit mirror failed: {e}",
                p.short(),
                proposal::state_label(p.state)
            )
        })?;
        captured = Some((p, corr));
        Ok(())
    })?;
    let (p, corr) = captured.expect("set on success");
    print_approved(&p, Some(&corr));
    Ok(())
}

fn reject_db(root: &Path, id: &str, reason: &str, email: &str) -> Result<(), String> {
    let policy = ai::load_policy(&root.join(POLICY_PATH))?;
    let store = ProposalStore::new(root, SUBDIR);
    let mut captured: Option<(Proposal, String)> = None;
    crate::tokio_run(async {
        let db = crate::db().await?;
        let user = ai::resolve_approver(&db, email, &policy).await?;
        let p = proposal::do_reject(&store, id, reason, user.email.clone())?;
        let extra = serde_json::json!({ "reason": reason });
        let corr = mirror(
            &db,
            &user,
            &p,
            AuditEvent::AiProposalRejected,
            "rejected",
            extra,
        )
        .await
        .map_err(|e| {
            format!(
                "proposal {} is now rejected locally, but the audit mirror failed: {e}",
                p.short()
            )
        })?;
        captured = Some((p, corr));
        Ok(())
    })?;
    let (p, corr) = captured.expect("set on success");
    println!(
        "{} memory proposal {} rejected (audit {corr})",
        style("ok").yellow(),
        p.short()
    );
    Ok(())
}

fn apply_db(root: &Path, id: &str, email: &str) -> Result<(), String> {
    let policy = ai::load_policy(&root.join(POLICY_PATH))?;
    let store = ProposalStore::new(root, SUBDIR);
    let mut captured: Option<(Proposal, usize, String)> = None;
    crate::tokio_run(async {
        let db = crate::db().await?;
        let user = ai::resolve_approver(&db, email, &policy).await?;
        let act = user.email.clone();
        let (p, _written) = proposal::do_apply_with(
            &store,
            id,
            user.email.clone(),
            "rustio memory approve",
            |p, r| apply_action(p, r, &act),
        )?;
        let count = render::write_view(&EntryStore::new(root))?;
        // write/supersede reuse ai_proposal_applied; redaction is its own
        // typed event recording the class removed (DESIGN_CLOUD.md §6, §3).
        let (event, extra) = if p.capability == "redact_memory" {
            let field = |k: &str| {
                p.metadata
                    .as_ref()
                    .and_then(|m| m.get(k))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            (
                AuditEvent::MemoryRedacted,
                serde_json::json!({ "entry_id": field("target"), "class": field("class") }),
            )
        } else {
            (
                AuditEvent::AiProposalApplied,
                serde_json::json!({ "entry_id": p.id, "entry_count": count }),
            )
        };
        let corr = mirror(&db, &user, &p, event, "applied", extra)
            .await
            .map_err(|e| {
                format!(
                    "proposal {} is now applied locally, but the audit mirror failed: {e}",
                    p.short()
                )
            })?;
        captured = Some((p, count, corr));
        Ok(())
    })?;
    let (p, count, corr) = captured.expect("set on success");
    print_apply_result(&p, count, Some(&corr));
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
        redacted: false,
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

// ---- Redaction (the bounded append-only exception, §3.4) -----------------

/// The redaction class taxonomy, aligned with the framework's existing
/// redaction helpers (`admin/redact.rs` + `filters.rs::mask_pii`) rather
/// than a parallel vocabulary.
const REDACTION_CLASSES: &[&str] = &[
    "password",
    "token",
    "mfa_secret",
    "backup_code",
    "pii",
    "credential",
    "operational",
];

/// The redaction request, carried on a `redact_memory` proposal's metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RedactDraft {
    target: String,
    class: String,
    reason: String,
}

/// `rustio memory redact` — propose removing prohibited content from an
/// existing entry. Two approvers required (§5).
pub(crate) fn redact(
    target: String,
    class: String,
    reason: String,
    by: Option<String>,
) -> Result<(), String> {
    redact_at(Path::new("."), target, class, reason, by)
}

fn redact_at(
    root: &Path,
    target: String,
    class: String,
    reason: String,
    by: Option<String>,
) -> Result<(), String> {
    if !REDACTION_CLASSES.contains(&class.as_str()) {
        return Err(format!(
            "unknown redaction class {class:?} (expected one of: {})",
            REDACTION_CLASSES.join(", ")
        ));
    }
    if reason.trim().is_empty() {
        return Err("a redaction needs a --reason".to_string());
    }
    let entries_dir = root.join(".rustio").join("memory").join("entries");
    if !entries_dir.join(format!("{target}.md")).exists() {
        return Err(format!(
            "cannot redact unknown entry {target:?} — no such entry file"
        ));
    }

    let actor = ai::whoami(by);
    let policy = ai::load_policy(&root.join(POLICY_PATH))?;
    let store = ProposalStore::new(root, SUBDIR);
    let draft = RedactDraft {
        target: target.clone(),
        class: class.clone(),
        reason,
    };
    let metadata =
        serde_json::to_value(&draft).map_err(|e| format!("could not encode redaction: {e}"))?;
    let title = format!("redact {} from entry {}", class, short(&target));

    // Floor of 2 enforces the two-approver rule even if the policy file is
    // edited to drop `redact_memory` from `second_approver_for`.
    let p = proposal::do_propose_meta(
        &store,
        &policy,
        "redact_memory",
        &title,
        None,
        Vec::new(),
        Some(metadata),
        2,
        actor,
    )?;
    println!(
        "{} redaction proposal {} created — needs {} approvers:\n  rustio memory approve {} --by <name>",
        style("rustio memory:").bold(),
        p.short(),
        p.required_approvals,
        p.short()
    );
    Ok(())
}

/// The injected apply action for `redact_memory`: replace the target
/// entry's body with a recorded marker and set its `redacted` flag, leaving
/// the rest of the frontmatter intact. The single in-place edit a ratified
/// memory operation may perform (§3.4, §11).
fn redact_materialize(p: &Proposal, root: &Path, approver: &str) -> Result<Vec<String>, String> {
    let draft: RedactDraft = p
        .metadata
        .clone()
        .ok_or_else(|| "redaction proposal has no metadata".to_string())
        .and_then(|v| {
            serde_json::from_value(v).map_err(|e| format!("malformed redaction metadata: {e}"))
        })?;

    let entries_dir = root.join(".rustio").join("memory").join("entries");
    let path = entries_dir.join(format!("{}.md", draft.target));
    if !path.exists() {
        return Err(format!(
            "cannot redact unknown entry {:?} — no such entry file",
            draft.target
        ));
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut entry =
        Entry::parse(&draft.target, &raw).map_err(|e| format!("{}: {e}", path.display()))?;

    let correlation = fresh_correlation_id();
    entry.body = format!(
        "> [content removed: class={} · {} · by={} · audit={}]",
        draft.class,
        today(),
        approver,
        correlation
    );
    entry.redacted = true;

    std::fs::write(&path, entry.to_file_string())
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(vec![format!(".rustio/memory/entries/{}.md", draft.target)])
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

    /// Propose → approve → apply a normal entry, returning its id.
    fn make_entry(root: &Path, body: &str) -> String {
        propose_at(root, "write_memory", draft(body), Some("amir".into())).expect("propose");
        let store = ProposalStore::new(root, SUBDIR);
        let p = store.load_all().unwrap().into_iter().next().unwrap();
        proposal::do_approve(&store, &p.id, "amir".into(), "x").expect("approve");
        apply_at(root, p.id.clone(), Some("sara".into())).expect("apply");
        p.id
    }

    #[test]
    fn redaction_requires_two_approvers_then_excises_body() {
        let root = temp_root();
        let entry_id = make_entry(&root, "leaked AKIA-secret-do-not-keep");

        redact_at(
            &root,
            entry_id.clone(),
            "credential".into(),
            "AWS key pasted by mistake".into(),
            Some("amir".into()),
        )
        .expect("redact propose");

        let store = ProposalStore::new(&root, SUBDIR);
        let rp = store
            .load_all()
            .unwrap()
            .into_iter()
            .find(|p| p.capability == "redact_memory")
            .expect("a redaction proposal");
        assert_eq!(rp.required_approvals, 2, "redaction needs two approvers");

        // One approval is not enough.
        proposal::do_approve(&store, &rp.id, "amir".into(), "x").expect("approve 1");
        assert!(apply_at(&root, rp.id.clone(), Some("amir".into())).is_err());

        // Second distinct approver, then apply.
        proposal::do_approve(&store, &rp.id, "sara".into(), "x").expect("approve 2");
        apply_at(&root, rp.id, Some("lee".into())).expect("apply redaction");

        // The prohibited content is gone from the working-tree file; the
        // entry is marked redacted and CLOUD.md still verifies fresh.
        let path = root
            .join(".rustio/memory/entries")
            .join(format!("{entry_id}.md"));
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("AKIA-secret-do-not-keep"), "secret excised");
        assert!(raw.contains("content removed: class=credential"));
        let reparsed = Entry::parse(&entry_id, &raw).unwrap();
        assert!(reparsed.redacted);
        render::verify(&EntryStore::new(&root)).expect("CLOUD.md fresh after redaction");
    }

    #[test]
    fn unknown_redaction_class_is_rejected() {
        let root = temp_root();
        let entry_id = make_entry(&root, "something");
        let err = redact_at(
            &root,
            entry_id,
            "selfie".into(),
            "x".into(),
            Some("amir".into()),
        )
        .unwrap_err();
        assert!(err.contains("unknown redaction class"), "{err}");
    }

    #[test]
    fn redact_unknown_entry_is_rejected_at_propose() {
        let root = temp_root();
        std::fs::create_dir_all(root.join(".rustio/memory/entries")).unwrap();
        let err = redact_at(
            &root,
            "ghost".into(),
            "token".into(),
            "x".into(),
            Some("amir".into()),
        )
        .unwrap_err();
        assert!(err.contains("cannot redact unknown entry"), "{err}");
    }
}
