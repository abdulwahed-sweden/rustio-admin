//! Generic proposal / approval / log lifecycle.
//!
//! This is the engine behind `rustio ai` (`DESIGN_AI_ASSISTANT.md` §4–§5)
//! and, from a later slice, `rustio memory` (`DESIGN_CLOUD_IMPL.md` §4).
//! Both features share **one** governance pipeline — there is no parallel
//! approval path. The lifecycle is pure of the cwd: it takes a [`Store`]
//! (parameterised by feature subdirectory) and a [`CapabilityPolicy`]
//! explicitly, so it is testable against a temp directory.
//!
//! What is *not* here, by design: the capability catalogue and the policy
//! file format (those are feature-specific — `ai` owns its `CATALOGUE` and
//! `.rustio/ai.toml`), and the DB-backed identity + audit mirror (still an
//! `ai` concern). This module owns only the offline state machine, the
//! file-backed store, and the append-only log.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::builder::ulid_gen::new_ulid;

/// The three buckets a capability can sit in (DESIGN §3).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Bucket {
    Allowed,
    NeedsApproval,
    Blocked,
}

/// What the lifecycle needs to know about a feature's policy to create a
/// proposal: which bucket a capability falls in, how many approvals it
/// needs, and the feature-specific error text for unknown / Blocked keys.
///
/// `ai`'s `Policy` implements this; a later memory policy will too. Keeping
/// it a trait is what lets one lifecycle serve multiple features without a
/// parallel governance path.
pub(crate) trait CapabilityPolicy {
    /// Resolve the bucket for a capability key, or `None` if unknown.
    fn bucket_of_key(&self, key: &str) -> Option<Bucket>;
    /// How many distinct approvals a capability needs before it may be
    /// applied (0 / 1 / 2). Blocked never reaches this.
    fn required_approvals(&self, key: &str, bucket: Bucket) -> u8;
    /// The known capability keys, for the unknown-capability error message.
    fn known_capability_keys(&self) -> Vec<&str>;
    /// The full error returned when a Blocked capability is proposed —
    /// feature-specific because it names the feature's policy file.
    fn blocked_hint(&self, key: &str) -> String;
}

/// The lifecycle state of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum State {
    Suggested,
    Approved,
    Rejected,
    Applied,
}

/// The log-event string for a state.
pub(crate) fn state_label(s: State) -> &'static str {
    match s {
        State::Suggested => "suggested",
        State::Approved => "approved",
        State::Rejected => "rejected",
        State::Applied => "applied",
    }
}

/// One approval signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Approval {
    pub(crate) by: String,
    pub(crate) at: String,
}

/// A file the proposal will write on apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StagedChange {
    /// Destination path, relative to the project root.
    pub(crate) path: String,
    /// Full content to write.
    pub(crate) content: String,
}

/// A proposed change plus its lifecycle record. The bucket and
/// `required_approvals` are snapshotted at creation so a later policy edit
/// cannot retroactively change a pending proposal's rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Proposal {
    pub(crate) id: String,
    pub(crate) capability: String,
    pub(crate) title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) summary: Option<String>,
    pub(crate) bucket: Bucket,
    pub(crate) required_approvals: u8,
    pub(crate) state: State,
    pub(crate) created_at: String,
    pub(crate) created_by: String,
    pub(crate) changes: Vec<StagedChange>,
    pub(crate) approvals: Vec<Approval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reject_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) decided_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) decided_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) applied_at: Option<String>,
    /// Feature-specific payload. `ai` leaves this `None` (its change is the
    /// staged files); `memory` stores its entry draft here, since a memory
    /// entry is materialised at apply (with the apply date and approver),
    /// not pre-staged. Absent in serialized `ai` proposals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) metadata: Option<serde_json::Value>,
}

impl Proposal {
    /// The last 8 chars of the id — short enough to type, and drawn from
    /// the ULID's random tail so two proposals created in the same second
    /// (whose timestamp *prefix* is identical) still get distinct handles.
    pub(crate) fn short(&self) -> &str {
        &self.id[self.id.len().saturating_sub(8)..]
    }

    /// Number of *distinct* approvers — the same person signing twice
    /// counts once.
    pub(crate) fn distinct_approvals(&self) -> usize {
        self.approvals
            .iter()
            .map(|a| a.by.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Can this proposal be applied right now?
    pub(crate) fn is_applyable(&self) -> bool {
        match self.state {
            State::Approved => true,
            // Allowed capabilities (0 required) skip the approval gate.
            State::Suggested => self.required_approvals == 0,
            State::Rejected | State::Applied => false,
        }
    }
}

/// A line in `.rustio/<feature>/log.jsonl`. Append-only; one JSON object
/// per line, mirroring the builder's `history.jsonl` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LogEntry {
    /// ULID of the log line itself.
    pub(crate) id: String,
    pub(crate) ts: String,
    /// `suggested` | `approved` | `rejected` | `applied` | `blocked`.
    pub(crate) event: String,
    /// Proposal id, or `-` for a blocked attempt that never became one.
    pub(crate) proposal: String,
    pub(crate) capability: String,
    pub(crate) by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) detail: Option<String>,
}

/// File-backed proposal store rooted at a project directory, under
/// `.rustio/<subdir>/`. Parameterised by `root` (rather than assuming the
/// cwd) so the lifecycle is testable against a temp directory, and by
/// `subdir` so each feature keeps its own proposals + log.
pub(crate) struct Store {
    root: PathBuf,
    subdir: &'static str,
}

impl Store {
    pub(crate) fn new(root: impl Into<PathBuf>, subdir: &'static str) -> Self {
        Store {
            root: root.into(),
            subdir,
        }
    }

    /// The project root this store is anchored to.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    fn dir(&self) -> PathBuf {
        self.root.join(".rustio").join(self.subdir)
    }

    fn proposals_dir(&self) -> PathBuf {
        self.dir().join("proposals")
    }

    fn log_path(&self) -> PathBuf {
        self.dir().join("log.jsonl")
    }

    /// Write a proposal as pretty JSON, creating directories as needed.
    pub(crate) fn save(&self, p: &Proposal) -> Result<(), String> {
        let dir = self.proposals_dir();
        fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
        let path = dir.join(format!("{}.json", p.id));
        let json = serde_json::to_string_pretty(p)
            .map_err(|e| format!("could not encode proposal: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("could not write {}: {e}", path.display()))
    }

    /// Load every proposal, oldest first (ULIDs sort chronologically).
    pub(crate) fn load_all(&self) -> Result<Vec<Proposal>, String> {
        let dir = self.proposals_dir();
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("could not read {}: {e}", dir.display())),
        };
        let mut out = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|e| format!("could not read {}: {e}", dir.display()))?
                .path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let raw = fs::read_to_string(&path)
                .map_err(|e| format!("could not read {}: {e}", path.display()))?;
            let p: Proposal = serde_json::from_str(&raw)
                .map_err(|e| format!("{} is not a valid proposal: {e}", path.display()))?;
            out.push(p);
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Resolve a full id, or a unique prefix or suffix, to one proposal.
    /// The displayed handle ([`Proposal::short`]) is a suffix, so suffix
    /// matching is what makes a copied handle resolve; prefix matching is
    /// accepted too for anyone reading the raw id.
    pub(crate) fn load(&self, query: &str) -> Result<Proposal, String> {
        let all = self.load_all()?;
        let matches: Vec<Proposal> = all
            .into_iter()
            .filter(|p| p.id == query || p.id.ends_with(query) || p.id.starts_with(query))
            .collect();
        match matches.len() {
            0 => Err(format!("no proposal matches {query:?}")),
            1 => Ok(matches.into_iter().next().expect("len == 1")),
            n => Err(format!("{n} proposals match {query:?}; use a longer id")),
        }
    }

    /// Append one line to the log. `O_APPEND` keeps concurrent writers
    /// from interleaving (same guarantee the builder's history relies on).
    pub(crate) fn append_log(&self, entry: &LogEntry) -> Result<(), String> {
        let dir = self.dir();
        fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
        let mut line =
            serde_json::to_string(entry).map_err(|e| format!("could not encode log entry: {e}"))?;
        line.push('\n');
        let path = self.log_path();
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("could not open {}: {e}", path.display()))?;
        f.write_all(line.as_bytes())
            .map_err(|e| format!("could not write {}: {e}", path.display()))
    }

    /// Every log entry, oldest-first. A line that fails to parse is
    /// skipped rather than aborting the read — the record is best-effort
    /// readable even if a future field is added.
    pub(crate) fn read_log(&self) -> Vec<LogEntry> {
        let raw = match fs::read_to_string(self.log_path()) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        raw.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// The most recent `n` log entries, oldest-first.
    pub(crate) fn recent_log(&self, n: usize) -> Vec<LogEntry> {
        let mut entries = self.read_log();
        let start = entries.len().saturating_sub(n);
        entries.split_off(start)
    }
}

/// Current UTC timestamp, RFC 3339 to the second.
pub(crate) fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Build a log entry with a fresh ULID and the current timestamp.
pub(crate) fn log_entry(
    event: &str,
    proposal: &str,
    capability: &str,
    by: &str,
    detail: Option<String>,
) -> LogEntry {
    LogEntry {
        id: new_ulid(),
        ts: now_ts(),
        event: event.to_string(),
        proposal: proposal.to_string(),
        capability: capability.to_string(),
        by: by.to_string(),
        detail,
    }
}

/// Create a proposal. Refuses a Blocked capability (logging the attempt)
/// and an unknown capability key.
pub(crate) fn do_propose(
    store: &Store,
    policy: &dyn CapabilityPolicy,
    capability: &str,
    title: &str,
    summary: Option<String>,
    changes: Vec<StagedChange>,
    actor: String,
) -> Result<Proposal, String> {
    do_propose_meta(
        store, policy, capability, title, summary, changes, None, 0, actor,
    )
}

/// Create a proposal carrying a feature payload (`metadata`) and an
/// approvals floor. `required_approvals` is the policy's value raised to at
/// least `required_floor` — `memory` uses the floor to force two approvers
/// on a foundational entry (a per-entry rule that can't live in the
/// per-capability `second_approver_for`). `do_propose` is the
/// `metadata = None, required_floor = 0` case.
#[allow(clippy::too_many_arguments)]
pub(crate) fn do_propose_meta(
    store: &Store,
    policy: &dyn CapabilityPolicy,
    capability: &str,
    title: &str,
    summary: Option<String>,
    changes: Vec<StagedChange>,
    metadata: Option<serde_json::Value>,
    required_floor: u8,
    actor: String,
) -> Result<Proposal, String> {
    let bucket = policy.bucket_of_key(capability).ok_or_else(|| {
        let known = policy.known_capability_keys();
        format!(
            "unknown capability {capability:?}. Known capabilities: {}",
            known.join(", ")
        )
    })?;

    if bucket == Bucket::Blocked {
        store.append_log(&log_entry(
            "blocked",
            "-",
            capability,
            &actor,
            Some(title.to_string()),
        ))?;
        return Err(policy.blocked_hint(capability));
    }

    let p = Proposal {
        id: new_ulid(),
        capability: capability.to_string(),
        title: title.to_string(),
        summary,
        bucket,
        required_approvals: policy
            .required_approvals(capability, bucket)
            .max(required_floor),
        state: State::Suggested,
        created_at: now_ts(),
        created_by: actor,
        changes,
        approvals: Vec::new(),
        reject_reason: None,
        decided_by: None,
        decided_at: None,
        applied_at: None,
        metadata,
    };
    store.save(&p)?;
    store.append_log(&log_entry(
        "suggested",
        &p.id,
        &p.capability,
        &p.created_by,
        Some(p.title.clone()),
    ))?;
    Ok(p)
}

/// Record one approval. Enforces distinct approvers and flips the
/// proposal to `Approved` once the threshold is met. `apply_cmd` is the
/// feature's apply command (e.g. `rustio ai apply`) used in hint text.
pub(crate) fn do_approve(
    store: &Store,
    id: &str,
    actor: String,
    apply_cmd: &str,
) -> Result<Proposal, String> {
    let mut p = store.load(id)?;
    match p.state {
        State::Suggested => {}
        State::Approved => {
            return Err(format!(
                "proposal {} is already approved — run `{apply_cmd} {}`",
                p.short(),
                p.short()
            ))
        }
        State::Applied => return Err(format!("proposal {} was already applied", p.short())),
        State::Rejected => {
            return Err(format!(
                "proposal {} was rejected; it cannot be approved",
                p.short()
            ))
        }
    }
    if p.required_approvals == 0 {
        return Err(format!(
            "`{}` is Allowed — no approval needed. Run `{apply_cmd} {}`.",
            p.capability,
            p.short()
        ));
    }
    if p.approvals.iter().any(|a| a.by == actor) {
        return Err(format!(
            "{actor} has already approved this; a second, distinct approver is required"
        ));
    }
    p.approvals.push(Approval {
        by: actor.clone(),
        at: now_ts(),
    });
    if p.distinct_approvals() as u8 >= p.required_approvals {
        p.state = State::Approved;
    }
    store.save(&p)?;
    store.append_log(&log_entry("approved", &p.id, &p.capability, &actor, None))?;
    Ok(p)
}

/// Reject a proposal that has not yet been applied.
pub(crate) fn do_reject(
    store: &Store,
    id: &str,
    reason: &str,
    actor: String,
) -> Result<Proposal, String> {
    let mut p = store.load(id)?;
    match p.state {
        State::Suggested | State::Approved => {}
        State::Applied => return Err(format!("proposal {} was already applied", p.short())),
        State::Rejected => return Err(format!("proposal {} is already rejected", p.short())),
    }
    p.state = State::Rejected;
    p.reject_reason = Some(reason.to_string());
    p.decided_by = Some(actor.clone());
    p.decided_at = Some(now_ts());
    store.save(&p)?;
    store.append_log(&log_entry(
        "rejected",
        &p.id,
        &p.capability,
        &actor,
        Some(reason.to_string()),
    ))?;
    Ok(p)
}

/// Apply an approved (or Allowed) proposal. The side effect is injected:
/// `apply_fn` receives the proposal and the project root and returns the
/// list of paths it touched, so `ai` writes staged files while a later
/// feature can materialise its own artifact. The state transition and the
/// `applied` log line are shared. `approve_cmd` is the feature's approve
/// command (e.g. `rustio ai approve`) used in hint text.
pub(crate) fn do_apply_with<F>(
    store: &Store,
    id: &str,
    actor: String,
    approve_cmd: &str,
    apply_fn: F,
) -> Result<(Proposal, Vec<String>), String>
where
    F: FnOnce(&Proposal, &Path) -> Result<Vec<String>, String>,
{
    let mut p = store.load(id)?;
    match p.state {
        State::Applied => return Err(format!("proposal {} was already applied", p.short())),
        State::Rejected => return Err(format!("proposal {} was rejected", p.short())),
        State::Suggested | State::Approved => {}
    }
    if !p.is_applyable() {
        return Err(format!(
            "proposal {} needs approval first ({}/{} approvals). Run `{approve_cmd} {} --by <name>`.",
            p.short(),
            p.distinct_approvals(),
            p.required_approvals,
            p.short()
        ));
    }

    let written = apply_fn(&p, store.root())?;

    p.state = State::Applied;
    p.applied_at = Some(now_ts());
    store.save(&p)?;
    store.append_log(&log_entry(
        "applied",
        &p.id,
        &p.capability,
        &actor,
        Some(format!("{} file(s)", written.len())),
    ))?;
    Ok((p, written))
}
