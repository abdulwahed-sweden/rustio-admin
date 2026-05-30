//! `rustio ai` — AI assistant permissions.
//!
//! Implements `docs/design/DESIGN_AI_ASSISTANT.md`: the `.rustio/ai.toml`
//! policy file (§2–§3), the proposal model and approval lifecycle
//! (§4–§5), and the `rustio ai` verbs (§6). The whole surface is
//! **offline** — no AI is contacted and no database is opened.
//!
//! The policy sorts every known capability into one of three buckets:
//! **Allowed**, **Needs approval**, or **Blocked** (DESIGN §3). The set
//! of capabilities is a fixed catalogue ([`CATALOGUE`]); the policy file
//! only moves capabilities between buckets.
//!
//! A *proposal* is a change the AI wants to make. It moves through
//! `Suggested → Approved → Applied` (or `→ Rejected`). The bucket and the
//! number of required approvals are snapshotted onto the proposal when it
//! is created, so a later policy edit never silently changes the rules a
//! pending proposal was created under. Proposals live as JSON under
//! `.rustio/ai/proposals/`; every state change is appended to
//! `.rustio/ai/log.jsonl`.
//!
//! **Implementation status (DESIGN §5).** The record is the local
//! `.rustio/ai/log.jsonl`, and the approver is captured as a `--by`
//! string. Mirroring the record into `rustio_admin_actions` and
//! authenticating the approver against a live admin are later slices —
//! both need a database; this slice stays offline.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use clap::Subcommand;
use console::style;
use serde::{Deserialize, Serialize};

use crate::builder::ulid_gen::new_ulid;

/// `.rustio/ai.toml`, relative to the current working directory — the
/// same `.rustio/` state directory the builder uses.
const POLICY_PATH: &str = ".rustio/ai.toml";

/// The default policy file written by `rustio ai init`. Hand-authored
/// (not serialised) so the shipped template carries comments. A test
/// (`default_template_matches_coded_defaults`) asserts it parses back to
/// exactly the coded defaults, so the two can never drift.
const DEFAULT_POLICY_TOML: &str = r#"# AI assistant permissions for this project.
# Contract: docs/design/DESIGN_AI_ASSISTANT.md
#
# This file is the single source of truth for what an AI coding
# assistant may do here. It is version-controlled; changing a bucket is
# a reviewed change. The assistant cannot edit this file (edit_ai_policy
# is blocked).

[ai]
assistant = "Claude Code"

# Done directly by the AI. Recorded and reversible.
allowed = [
  "create_model",
  "create_form",
  "create_admin_page",
  "suggest_fields",
  "draft_migration",
]

# The AI prepares these and stops; a developer must approve before they run.
needs_approval = [
  "apply_migration",
  "modify_table",
  "edit_existing_code",
  "add_dependency",
]

# The AI cannot do these at all until the rule is moved out of `blocked`.
blocked = [
  "security_settings",
  "production_deploy",
  "delete_data",
  "edit_audit_log",
  "edit_ai_policy",
]

[ai.approval]
# Minimum role that may approve a change.
approver_role = "administrator"
# Capabilities that require two distinct approvers.
second_approver_for = ["modify_table", "apply_migration"]
"#;

/// `rustio ai` subcommands.
#[derive(Subcommand)]
pub(crate) enum Action {
    /// Show what the AI assistant may do (Allowed / Needs approval /
    /// Blocked), plus pending proposals and recent actions. Reads the
    /// policy and the local log only — no AI call, no database.
    Status,
    /// Write a default `.rustio/ai.toml` policy file. Refuses to
    /// overwrite an existing policy without `--force`.
    Init {
        /// Overwrite an existing `.rustio/ai.toml`.
        #[arg(long)]
        force: bool,
    },
    /// Register a change the AI wants to make. Refused outright when the
    /// capability is Blocked by the policy.
    Propose {
        /// Capability key (e.g. `create_model`, `modify_table`).
        #[arg(long)]
        capability: String,
        /// One-line description of the change.
        #[arg(long)]
        title: String,
        /// Optional longer summary.
        #[arg(long)]
        summary: Option<String>,
        /// Stage a file the change will write. `DEST=SRC` reads SRC now
        /// and writes it to DEST on apply; a bare `PATH` stages that file
        /// as-is. Repeatable.
        #[arg(long = "stage", value_name = "DEST=SRC")]
        stage: Vec<String>,
        /// Who is proposing (defaults to the OS user).
        #[arg(long)]
        by: Option<String>,
    },
    /// List proposals. By default shows only those awaiting a decision.
    List {
        /// Include applied and rejected proposals.
        #[arg(long)]
        all: bool,
    },
    /// Show one proposal's details and staged changes. Accepts a full id
    /// or any unique prefix.
    Review { id: String },
    /// Approve a proposal as `--by <name>` (defaults to the OS user).
    Approve {
        id: String,
        #[arg(long)]
        by: Option<String>,
    },
    /// Reject a proposal with a reason.
    Reject {
        id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        by: Option<String>,
    },
    /// Apply an approved (or Allowed) proposal — writes its staged files.
    Apply {
        id: String,
        #[arg(long)]
        by: Option<String>,
    },
    /// Show the action record: suggestions, approvals, rejections,
    /// applies, and blocked attempts, newest first.
    Log {
        /// Show at most N entries (default 20). Ignored with `--all`.
        #[arg(long)]
        limit: Option<usize>,
        /// Only entries for one proposal (full id or short handle).
        #[arg(long)]
        proposal: Option<String>,
        /// Show every entry, no limit.
        #[arg(long)]
        all: bool,
    },
    /// Move a capability into `allowed` (or `needs_approval` with
    /// `--needs-approval`). Edits `.rustio/ai.toml` and prints the diff.
    Allow {
        /// Capability key (e.g. `edit_existing_code`).
        capability: String,
        /// Place it in `needs_approval` instead of `allowed`.
        #[arg(long = "needs-approval")]
        needs_approval: bool,
    },
    /// Move a capability into `blocked`. Edits `.rustio/ai.toml` and
    /// prints the diff.
    Deny {
        /// Capability key (e.g. `apply_migration`).
        capability: String,
    },
}

/// Dispatch. Offline and synchronous — no Postgres connection.
pub(crate) fn run(action: Action) -> Result<(), String> {
    let policy_path = PathBuf::from(POLICY_PATH);
    let store = Store::new(".");
    match action {
        Action::Status => status(&policy_path, &store),
        Action::Init { force } => init(&policy_path, force),
        Action::Propose {
            capability,
            title,
            summary,
            stage,
            by,
        } => propose(
            &policy_path,
            &store,
            &capability,
            &title,
            summary,
            &stage,
            by,
        ),
        Action::List { all } => list(&store, all),
        Action::Review { id } => review(&store, &id),
        Action::Approve { id, by } => approve(&store, &id, by),
        Action::Reject { id, reason, by } => reject(&store, &id, &reason, by),
        Action::Apply { id, by } => apply(&store, &id, by),
        Action::Log {
            limit,
            proposal,
            all,
        } => log_cmd(&store, limit, proposal, all),
        Action::Allow {
            capability,
            needs_approval,
        } => set_bucket(
            &policy_path,
            &capability,
            if needs_approval {
                Bucket::NeedsApproval
            } else {
                Bucket::Allowed
            },
        ),
        Action::Deny { capability } => set_bucket(&policy_path, &capability, Bucket::Blocked),
    }
}

/// Default number of log entries shown by `rustio ai log`.
const DEFAULT_LOG_LIMIT: usize = 20;

// ---------------------------------------------------------------------------
// Policy (DESIGN §2–§3)
// ---------------------------------------------------------------------------

/// The three buckets a capability can sit in (DESIGN §3).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Bucket {
    Allowed,
    NeedsApproval,
    Blocked,
}

/// One known capability and the bucket it falls in by default.
struct CapDef {
    /// Stable key used in `.rustio/ai.toml`.
    key: &'static str,
    /// Human label shown by `rustio ai status`.
    label: &'static str,
    /// Bucket used when the policy file does not mention this key.
    default: Bucket,
}

/// The fixed catalogue of capabilities. The policy file may move a key
/// between buckets but cannot invent a new capability — an unknown key
/// in the file is reported as a warning and ignored. Order here is the
/// display order within each bucket.
const CATALOGUE: &[CapDef] = &[
    CapDef {
        key: "create_model",
        label: "Create models",
        default: Bucket::Allowed,
    },
    CapDef {
        key: "create_form",
        label: "Create forms",
        default: Bucket::Allowed,
    },
    CapDef {
        key: "create_admin_page",
        label: "Create admin pages",
        default: Bucket::Allowed,
    },
    CapDef {
        key: "suggest_fields",
        label: "Suggest fields",
        default: Bucket::Allowed,
    },
    CapDef {
        key: "draft_migration",
        label: "Draft migrations",
        default: Bucket::Allowed,
    },
    CapDef {
        key: "apply_migration",
        label: "Apply migrations",
        default: Bucket::NeedsApproval,
    },
    CapDef {
        key: "modify_table",
        label: "Change existing tables",
        default: Bucket::NeedsApproval,
    },
    CapDef {
        key: "edit_existing_code",
        label: "Edit existing code",
        default: Bucket::NeedsApproval,
    },
    CapDef {
        key: "add_dependency",
        label: "Add dependencies",
        default: Bucket::NeedsApproval,
    },
    CapDef {
        key: "security_settings",
        label: "Security settings",
        default: Bucket::Blocked,
    },
    CapDef {
        key: "production_deploy",
        label: "Production deployment",
        default: Bucket::Blocked,
    },
    CapDef {
        key: "delete_data",
        label: "Delete data",
        default: Bucket::Blocked,
    },
    CapDef {
        key: "edit_audit_log",
        label: "Edit the audit log",
        default: Bucket::Blocked,
    },
    CapDef {
        key: "edit_ai_policy",
        label: "Edit the AI policy",
        default: Bucket::Blocked,
    },
];

/// Where a resolved policy came from — drives the header line.
#[derive(Debug, PartialEq, Eq)]
enum Source {
    /// No `.rustio/ai.toml`; coded defaults are in effect.
    Default,
    /// Loaded from the given path.
    File(PathBuf),
}

/// A resolved policy: the catalogue defaults with the file's overrides
/// applied. Built once, then read by [`status`].
#[derive(Debug)]
struct Policy {
    assistant: String,
    approver_role: String,
    second_approver_for: Vec<String>,
    /// Per-capability bucket overrides parsed from the file. A key absent
    /// here keeps its catalogue default.
    overrides: BTreeMap<String, Bucket>,
    source: Source,
    /// Non-fatal problems found while reading the file (unknown keys,
    /// duplicate placements). Surfaced to the developer, never silent.
    warnings: Vec<String>,
}

impl Policy {
    /// Coded defaults — what ships before any file exists, and what
    /// [`DEFAULT_POLICY_TOML`] must round-trip to.
    fn defaults() -> Self {
        Policy {
            assistant: "Claude Code".to_string(),
            approver_role: "administrator".to_string(),
            second_approver_for: vec!["modify_table".to_string(), "apply_migration".to_string()],
            overrides: BTreeMap::new(),
            source: Source::Default,
            warnings: Vec::new(),
        }
    }

    /// Resolve the bucket for one capability: the file's placement if it
    /// set one, otherwise the catalogue default.
    fn bucket_of(&self, cap: &CapDef) -> Bucket {
        self.overrides.get(cap.key).copied().unwrap_or(cap.default)
    }

    /// Resolve the bucket for a capability key, or `None` if unknown.
    fn bucket_of_key(&self, key: &str) -> Option<Bucket> {
        CATALOGUE
            .iter()
            .find(|c| c.key == key)
            .map(|c| self.bucket_of(c))
    }

    /// True when `key` requires two approvers.
    fn needs_two_approvers(&self, key: &str) -> bool {
        self.second_approver_for.iter().any(|k| k == key)
    }

    /// How many distinct approvals a capability needs before it may be
    /// applied: 0 for Allowed, 1 for Needs-approval, 2 when the policy
    /// lists the capability under `second_approver_for`. Blocked never
    /// reaches this — `do_propose` refuses it first.
    fn required_approvals(&self, key: &str, bucket: Bucket) -> u8 {
        match bucket {
            Bucket::Allowed | Bucket::Blocked => 0,
            Bucket::NeedsApproval => {
                if self.needs_two_approvers(key) {
                    2
                } else {
                    1
                }
            }
        }
    }
}

/// Load the policy from `path`, or fall back to coded defaults when the
/// file is absent. A present-but-malformed file is a hard error — a
/// broken policy must be visible, not silently replaced by defaults.
fn load_policy(path: &Path) -> Result<Policy, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Policy::defaults()),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    parse_policy(&raw, path)
}

/// Parse `.rustio/ai.toml` text into a [`Policy`]. Pure on its input so
/// it can be unit-tested without the filesystem.
fn parse_policy(input: &str, path: &Path) -> Result<Policy, String> {
    let doc = input
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("{} is not valid TOML: {e}", path.display()))?;

    let mut policy = Policy::defaults();
    policy.source = Source::File(path.to_path_buf());
    // Start from an empty override map; the file is authoritative for
    // any capability it names.
    policy.overrides = BTreeMap::new();

    let ai = doc
        .get("ai")
        .and_then(|i| i.as_table_like())
        .ok_or_else(|| format!("{}: missing [ai] table", path.display()))?;

    if let Some(name) = ai.get("assistant").and_then(|v| v.as_str()) {
        policy.assistant = name.to_string();
    }

    // Each bucket list places capabilities. Later lists win on a
    // duplicate, but a duplicate is always reported.
    place_bucket(ai, "allowed", Bucket::Allowed, &mut policy);
    place_bucket(ai, "needs_approval", Bucket::NeedsApproval, &mut policy);
    place_bucket(ai, "blocked", Bucket::Blocked, &mut policy);

    if let Some(approval) = doc.get("ai").and_then(|i| i.get("approval")) {
        if let Some(role) = approval.get("approver_role").and_then(|v| v.as_str()) {
            policy.approver_role = role.to_string();
        }
        if let Some(arr) = approval
            .get("second_approver_for")
            .and_then(|v| v.as_array())
        {
            policy.second_approver_for = arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            for key in &policy.second_approver_for {
                if !is_known(key) {
                    policy
                        .warnings
                        .push(format!("unknown capability {key:?} in second_approver_for"));
                }
            }
        }
    }

    Ok(policy)
}

/// Read one bucket array (`allowed` / `needs_approval` / `blocked`) and
/// record each known capability's placement. Unknown keys and duplicate
/// placements become warnings.
fn place_bucket(ai: &dyn toml_edit::TableLike, field: &str, bucket: Bucket, policy: &mut Policy) {
    let Some(arr) = ai.get(field).and_then(|v| v.as_array()) else {
        return;
    };
    for key in arr.iter().filter_map(|v| v.as_str()) {
        if !is_known(key) {
            policy
                .warnings
                .push(format!("unknown capability {key:?} in `{field}` (ignored)"));
            continue;
        }
        if let Some(prev) = policy.overrides.insert(key.to_string(), bucket) {
            if prev != bucket {
                policy
                    .warnings
                    .push(format!("capability {key:?} listed in more than one bucket"));
            }
        }
    }
}

/// True when `key` is a capability the framework knows about.
fn is_known(key: &str) -> bool {
    CATALOGUE.iter().any(|c| c.key == key)
}

/// Human label for a bucket.
fn bucket_label(b: Bucket) -> &'static str {
    match b {
        Bucket::Allowed => "Allowed",
        Bucket::NeedsApproval => "Needs approval",
        Bucket::Blocked => "Blocked",
    }
}

// ---------------------------------------------------------------------------
// Proposal model (DESIGN §4)
// ---------------------------------------------------------------------------

/// The lifecycle state of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Suggested,
    Approved,
    Rejected,
    Applied,
}

fn state_label(s: State) -> &'static str {
    match s {
        State::Suggested => "suggested",
        State::Approved => "approved",
        State::Rejected => "rejected",
        State::Applied => "applied",
    }
}

/// One approval signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Approval {
    by: String,
    at: String,
}

/// A file the proposal will write on apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StagedChange {
    /// Destination path, relative to the project root.
    path: String,
    /// Full content to write.
    content: String,
}

/// A change the AI wants to make, plus its lifecycle record. The bucket
/// and `required_approvals` are snapshotted at creation so a later policy
/// edit cannot retroactively change a pending proposal's rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Proposal {
    id: String,
    capability: String,
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    bucket: Bucket,
    required_approvals: u8,
    state: State,
    created_at: String,
    created_by: String,
    changes: Vec<StagedChange>,
    approvals: Vec<Approval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reject_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decided_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decided_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    applied_at: Option<String>,
}

impl Proposal {
    /// The last 8 chars of the id — short enough to type, and drawn from
    /// the ULID's random tail so two proposals created in the same second
    /// (whose timestamp *prefix* is identical) still get distinct handles.
    fn short(&self) -> &str {
        &self.id[self.id.len().saturating_sub(8)..]
    }

    /// Number of *distinct* approvers — the same person signing twice
    /// counts once.
    fn distinct_approvals(&self) -> usize {
        self.approvals
            .iter()
            .map(|a| a.by.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Can this proposal be applied right now?
    fn is_applyable(&self) -> bool {
        match self.state {
            State::Approved => true,
            // Allowed capabilities (0 required) skip the approval gate.
            State::Suggested => self.required_approvals == 0,
            State::Rejected | State::Applied => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Store — proposals + append-only log under .rustio/ai/ (DESIGN §5)
// ---------------------------------------------------------------------------

/// A line in `.rustio/ai/log.jsonl`. Append-only; one JSON object per
/// line, mirroring the builder's `history.jsonl` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogEntry {
    /// ULID of the log line itself.
    id: String,
    ts: String,
    /// `suggested` | `approved` | `rejected` | `applied` | `blocked`.
    event: String,
    /// Proposal id, or `-` for a blocked attempt that never became one.
    proposal: String,
    capability: String,
    by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// File-backed proposal store rooted at a project directory. Parameterised
/// by `root` (rather than assuming the cwd) so the lifecycle is testable
/// against a temp directory.
struct Store {
    root: PathBuf,
}

impl Store {
    fn new(root: impl Into<PathBuf>) -> Self {
        Store { root: root.into() }
    }

    fn dir(&self) -> PathBuf {
        self.root.join(".rustio").join("ai")
    }

    fn proposals_dir(&self) -> PathBuf {
        self.dir().join("proposals")
    }

    fn log_path(&self) -> PathBuf {
        self.dir().join("log.jsonl")
    }

    /// Write a proposal as pretty JSON, creating directories as needed.
    fn save(&self, p: &Proposal) -> Result<(), String> {
        let dir = self.proposals_dir();
        fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
        let path = dir.join(format!("{}.json", p.id));
        let json = serde_json::to_string_pretty(p)
            .map_err(|e| format!("could not encode proposal: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("could not write {}: {e}", path.display()))
    }

    /// Load every proposal, oldest first (ULIDs sort chronologically).
    fn load_all(&self) -> Result<Vec<Proposal>, String> {
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
    fn load(&self, query: &str) -> Result<Proposal, String> {
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
    fn append_log(&self, entry: &LogEntry) -> Result<(), String> {
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
    fn read_log(&self) -> Vec<LogEntry> {
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
    fn recent_log(&self, n: usize) -> Vec<LogEntry> {
        let mut entries = self.read_log();
        let start = entries.len().saturating_sub(n);
        entries.split_off(start)
    }
}

/// Current UTC timestamp, RFC 3339 to the second.
fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Resolve the acting identity: an explicit `--by`, else the OS user,
/// else `"unknown"`.
fn whoami(by: Option<String>) -> String {
    by.map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn log_entry(
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

// ---------------------------------------------------------------------------
// Lifecycle core (pure of the cwd; takes Store + Policy explicitly)
// ---------------------------------------------------------------------------

/// Create a proposal. Refuses a Blocked capability (logging the attempt)
/// and an unknown capability key.
fn do_propose(
    store: &Store,
    policy: &Policy,
    capability: &str,
    title: &str,
    summary: Option<String>,
    changes: Vec<StagedChange>,
    actor: String,
) -> Result<Proposal, String> {
    let bucket = policy.bucket_of_key(capability).ok_or_else(|| {
        let known: Vec<&str> = CATALOGUE.iter().map(|c| c.key).collect();
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
        return Err(format!(
            "`{capability}` is Blocked by the policy — the AI cannot do this. A developer must act by hand, or move it out of `blocked` in {POLICY_PATH} (itself a reviewed change)."
        ));
    }

    let p = Proposal {
        id: new_ulid(),
        capability: capability.to_string(),
        title: title.to_string(),
        summary,
        bucket,
        required_approvals: policy.required_approvals(capability, bucket),
        state: State::Suggested,
        created_at: now_ts(),
        created_by: actor,
        changes,
        approvals: Vec::new(),
        reject_reason: None,
        decided_by: None,
        decided_at: None,
        applied_at: None,
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
/// proposal to `Approved` once the threshold is met.
fn do_approve(store: &Store, id: &str, actor: String) -> Result<Proposal, String> {
    let mut p = store.load(id)?;
    match p.state {
        State::Suggested => {}
        State::Approved => {
            return Err(format!(
                "proposal {} is already approved — run `rustio ai apply {}`",
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
            "`{}` is Allowed — no approval needed. Run `rustio ai apply {}`.",
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
fn do_reject(store: &Store, id: &str, reason: &str, actor: String) -> Result<Proposal, String> {
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

/// Apply an approved (or Allowed) proposal: write its staged files, then
/// mark it applied. Returns the proposal and the list of paths written.
fn do_apply(store: &Store, id: &str, actor: String) -> Result<(Proposal, Vec<String>), String> {
    let mut p = store.load(id)?;
    match p.state {
        State::Applied => return Err(format!("proposal {} was already applied", p.short())),
        State::Rejected => return Err(format!("proposal {} was rejected", p.short())),
        State::Suggested | State::Approved => {}
    }
    if !p.is_applyable() {
        return Err(format!(
            "proposal {} needs approval first ({}/{} approvals). Run `rustio ai approve {} --by <name>`.",
            p.short(),
            p.distinct_approvals(),
            p.required_approvals,
            p.short()
        ));
    }

    let mut written = Vec::new();
    for ch in &p.changes {
        let dest = store.root.join(&ch.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        fs::write(&dest, ch.content.as_bytes())
            .map_err(|e| format!("could not write {}: {e}", dest.display()))?;
        written.push(ch.path.clone());
    }

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

/// Turn `--stage` specs into staged changes. `DEST=SRC` reads SRC now;
/// a bare `PATH` stages that file as-is. Destinations must be relative
/// and free of `..` so an applied proposal can't escape the project.
fn parse_stages(specs: &[String]) -> Result<Vec<StagedChange>, String> {
    let mut out = Vec::new();
    for spec in specs {
        let (dest, src) = match spec.split_once('=') {
            Some((d, s)) => (d.to_string(), s.to_string()),
            None => (spec.clone(), spec.clone()),
        };
        validate_dest(&dest)?;
        let content = fs::read_to_string(&src)
            .map_err(|e| format!("could not read staged source {src:?}: {e}"))?;
        out.push(StagedChange {
            path: dest,
            content,
        });
    }
    Ok(out)
}

/// Reject absolute or `..`-bearing staged destinations.
fn validate_dest(dest: &str) -> Result<(), String> {
    let pb = Path::new(dest);
    let escapes = pb.is_absolute()
        || pb
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir));
    if escapes {
        return Err(format!(
            "staged destination {dest:?} must be a relative path inside the project (no `..`, no absolute paths)"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Verb wrappers — load policy / build store, call core, print
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn propose(
    policy_path: &Path,
    store: &Store,
    capability: &str,
    title: &str,
    summary: Option<String>,
    stage: &[String],
    by: Option<String>,
) -> Result<(), String> {
    let policy = load_policy(policy_path)?;
    let actor = whoami(by);
    let changes = parse_stages(stage)?;
    let p = do_propose(store, &policy, capability, title, summary, changes, actor)?;

    println!("rustio ai: proposal {} created", p.short());
    println!(
        "  capability: {}  ({})",
        p.capability,
        bucket_label(p.bucket)
    );
    println!("  title:      {}", p.title);
    if !p.changes.is_empty() {
        println!("  staged:     {} file(s)", p.changes.len());
    }
    println!();
    if p.required_approvals == 0 {
        println!("This capability is Allowed — apply it directly:");
        println!("  rustio ai apply {}", p.short());
    } else {
        let approvers = if p.required_approvals == 1 {
            "1 approver".to_string()
        } else {
            format!("{} distinct approvers", p.required_approvals)
        };
        println!("Needs approval ({approvers}) before it can be applied:");
        println!("  rustio ai review  {}", p.short());
        println!("  rustio ai approve {} --by <name>", p.short());
    }
    Ok(())
}

fn approve(store: &Store, id: &str, by: Option<String>) -> Result<(), String> {
    let actor = whoami(by);
    let p = do_approve(store, id, actor.clone())?;
    println!("rustio ai: {} approved proposal {}", actor, p.short());
    if p.state == State::Approved {
        println!("  fully approved — apply it:");
        println!("    rustio ai apply {}", p.short());
    } else {
        let have = p.distinct_approvals();
        let need = p.required_approvals as usize;
        println!(
            "  {have}/{need} approvals — needs {} more distinct approver(s)",
            need.saturating_sub(have)
        );
    }
    Ok(())
}

fn reject(store: &Store, id: &str, reason: &str, by: Option<String>) -> Result<(), String> {
    let actor = whoami(by);
    let p = do_reject(store, id, reason, actor)?;
    println!("rustio ai: proposal {} rejected", p.short());
    Ok(())
}

fn apply(store: &Store, id: &str, by: Option<String>) -> Result<(), String> {
    let actor = whoami(by);
    let (p, written) = do_apply(store, id, actor)?;
    println!("rustio ai: proposal {} applied", p.short());
    if written.is_empty() {
        println!("  (no files staged)");
    } else {
        for path in &written {
            println!("  wrote {path}");
        }
    }
    Ok(())
}

fn review(store: &Store, id: &str) -> Result<(), String> {
    let p = store.load(id)?;
    println!("Proposal {}", p.id);
    println!(
        "  capability: {}  ({})",
        p.capability,
        bucket_label(p.bucket)
    );
    println!("  title:      {}", p.title);
    if let Some(s) = &p.summary {
        println!("  summary:    {s}");
    }
    println!("  state:      {}", state_label(p.state));
    println!("  created:    {} by {}", p.created_at, p.created_by);

    if p.required_approvals == 0 {
        println!("  approvals:  none required (Allowed)");
    } else {
        println!(
            "  approvals:  {}/{}",
            p.distinct_approvals(),
            p.required_approvals
        );
        for a in &p.approvals {
            println!("    - {} at {}", a.by, a.at);
        }
    }
    if let (Some(by), Some(reason)) = (&p.decided_by, &p.reject_reason) {
        println!("  rejected:   by {by} — {reason}");
    }

    if p.changes.is_empty() {
        println!("  changes:    (none staged)");
    } else {
        println!("  changes:");
        for ch in &p.changes {
            let exists = store.root.join(&ch.path).exists();
            let verb = if exists { "overwrite" } else { "create" };
            println!(
                "    {} {} ({} lines, {} bytes)",
                verb,
                ch.path,
                ch.content.lines().count(),
                ch.content.len()
            );
        }
    }

    match p.state {
        State::Suggested if p.required_approvals == 0 => {
            println!();
            println!("  apply →  rustio ai apply {}", p.short());
        }
        State::Suggested => {
            println!();
            println!("  approve →  rustio ai approve {} --by <name>", p.short());
            println!(
                "  reject  →  rustio ai reject {} --reason \"...\"",
                p.short()
            );
        }
        State::Approved => {
            println!();
            println!("  apply →  rustio ai apply {}", p.short());
        }
        State::Rejected | State::Applied => {}
    }
    Ok(())
}

fn list(store: &Store, all: bool) -> Result<(), String> {
    let mut ps = store.load_all()?;
    if !all {
        ps.retain(|p| matches!(p.state, State::Suggested | State::Approved));
    }
    if ps.is_empty() {
        if all {
            println!("No proposals.");
        } else {
            println!("No proposals awaiting a decision. (use --all to include applied / rejected)");
        }
        return Ok(());
    }
    for p in &ps {
        let appr = if p.required_approvals > 0 {
            format!("{}/{}", p.distinct_approvals(), p.required_approvals)
        } else {
            "-".to_string()
        };
        println!(
            "  {:<8}  {:<16} {:<9} {:<5} {}",
            p.short(),
            p.capability,
            state_label(p.state),
            appr,
            p.title
        );
    }
    Ok(())
}

/// `rustio ai log` — render the action record, newest first.
fn log_cmd(
    store: &Store,
    limit: Option<usize>,
    proposal: Option<String>,
    all: bool,
) -> Result<(), String> {
    let mut entries = store.read_log();
    if let Some(q) = &proposal {
        // Match the same way `Store::load` resolves a handle.
        entries
            .retain(|e| e.proposal == *q || e.proposal.ends_with(q) || e.proposal.starts_with(q));
    }
    if entries.is_empty() {
        println!("No actions recorded yet.");
        return Ok(());
    }
    entries.reverse(); // newest first

    let total = entries.len();
    let cap = if all {
        total
    } else {
        limit.unwrap_or(DEFAULT_LOG_LIMIT)
    };
    let shown = total.min(cap);

    print!("{}", format_log(&entries[..shown]));
    if shown < total {
        println!();
        println!(
            "{}",
            style(format!(
                "… {} older entr{} (use --all or --limit N)",
                total - shown,
                if total - shown == 1 { "y" } else { "ies" }
            ))
            .dim()
        );
    }
    Ok(())
}

/// Format log entries (already in display order) into aligned lines.
/// Pure so it can be unit-tested without the filesystem.
fn format_log(entries: &[LogEntry]) -> String {
    let mut s = String::new();
    for e in entries {
        let pid = &e.proposal[e.proposal.len().saturating_sub(8)..];
        let detail = e.detail.as_deref().unwrap_or("");
        let line = format!(
            "{}  {:<9} {:<8}  {:<16} {:<10} {}",
            e.ts, e.event, pid, e.capability, e.by, detail
        );
        s.push_str(line.trim_end());
        s.push('\n');
    }
    s
}

// ---------------------------------------------------------------------------
// status / init
// ---------------------------------------------------------------------------

/// `rustio ai status` — print the resolved policy plus pending proposals
/// and recent actions from the local store.
fn status(policy_path: &Path, store: &Store) -> Result<(), String> {
    let policy = load_policy(policy_path)?;

    println!("AI Assistant: {}", policy.assistant);
    match &policy.source {
        Source::File(p) => println!(
            "Policy:       {}   (approver: {})",
            p.display(),
            policy.approver_role
        ),
        Source::Default => println!(
            "Policy:       defaults — no {POLICY_PATH} (run `rustio ai init`)   (approver: {})",
            policy.approver_role
        ),
    }

    if !policy.warnings.is_empty() {
        println!();
        println!("{}", style("Policy warnings:").yellow().bold());
        for w in &policy.warnings {
            println!("  {} {w}", style("!").yellow());
        }
    }

    print_bucket(
        &policy,
        "Allowed:",
        Bucket::Allowed,
        style("✓").green().to_string(),
    );
    print_bucket(
        &policy,
        "Needs approval:",
        Bucket::NeedsApproval,
        style("⚠").yellow().to_string(),
    );
    print_bucket(
        &policy,
        "Blocked:",
        Bucket::Blocked,
        style("✗").red().to_string(),
    );

    // Pending proposals (Suggested or Approved — i.e. not yet resolved).
    let pending: Vec<Proposal> = store
        .load_all()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| matches!(p.state, State::Suggested | State::Approved))
        .collect();
    println!();
    println!("Pending your review:");
    if pending.is_empty() {
        println!("  {}", style("(none)").dim());
    } else {
        for p in &pending {
            let appr = if p.required_approvals > 0 {
                format!("  [{}/{}]", p.distinct_approvals(), p.required_approvals)
            } else {
                String::new()
            };
            println!(
                "  {}  {} — {} ({}){}",
                p.short(),
                p.capability,
                p.title,
                state_label(p.state),
                appr
            );
        }
    }

    // Recent actions, newest first.
    let recent = store.recent_log(5);
    println!();
    println!("Recent actions:");
    if recent.is_empty() {
        println!("  {}", style("(none)").dim());
    } else {
        for e in recent.iter().rev() {
            // Match the suffix handle shown elsewhere (`-` for blocked).
            let pid = &e.proposal[e.proposal.len().saturating_sub(8)..];
            println!(
                "  {:<8}  {:<9} {:<16} by {}",
                pid, e.event, e.capability, e.by
            );
        }
    }

    Ok(())
}

/// Print one bucket: every catalogue capability that resolves to it, in
/// catalogue order, annotated with a two-approver note where it applies.
fn print_bucket(policy: &Policy, heading: &str, bucket: Bucket, glyph: String) {
    println!();
    println!("{heading}");
    for cap in CATALOGUE.iter().filter(|c| policy.bucket_of(c) == bucket) {
        let note = if bucket == Bucket::NeedsApproval && policy.needs_two_approvers(cap.key) {
            "   (2 approvers)"
        } else {
            ""
        };
        println!("  {glyph} {}{note}", cap.label);
    }
}

/// `rustio ai init` — write the default policy file.
fn init(path: &Path, force: bool) -> Result<(), String> {
    if path.exists() && !force {
        return Err(format!(
            "{} already exists. Pass --force to overwrite it.",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, DEFAULT_POLICY_TOML)
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;

    println!("rustio ai: wrote {}", path.display());
    println!();
    println!("next step:");
    println!("  rustio ai status        # see what the AI may do");
    Ok(())
}

// ---------------------------------------------------------------------------
// allow / deny — edit the policy buckets (DESIGN §6)
// ---------------------------------------------------------------------------

/// The `.rustio/ai.toml` array name for a bucket.
fn bucket_field(b: Bucket) -> &'static str {
    match b {
        Bucket::Allowed => "allowed",
        Bucket::NeedsApproval => "needs_approval",
        Bucket::Blocked => "blocked",
    }
}

/// `rustio ai allow` / `deny` — move a capability into `target`, edit the
/// policy file, and print the diff. Requires the file to exist (run
/// `rustio ai init` first), so the change is always an explicit edit to a
/// version-controlled file rather than a surprise creation.
fn set_bucket(policy_path: &Path, capability: &str, target: Bucket) -> Result<(), String> {
    if !is_known(capability) {
        let known: Vec<&str> = CATALOGUE.iter().map(|c| c.key).collect();
        return Err(format!(
            "unknown capability {capability:?}. Known capabilities: {}",
            known.join(", ")
        ));
    }
    let raw = match std::fs::read_to_string(policy_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "no {} yet — run `rustio ai init` first, then edit buckets",
                policy_path.display()
            ))
        }
        Err(e) => return Err(format!("could not read {}: {e}", policy_path.display())),
    };

    let new = edit_policy_text(&raw, capability, target)?;
    if new == raw {
        println!(
            "`{capability}` is already in `{}`. No change.",
            bucket_field(target)
        );
        return Ok(());
    }

    println!(
        "{} → `{}`  in {}",
        capability,
        bucket_field(target),
        policy_path.display()
    );
    println!();
    print!("{}", line_diff(&raw, &new));
    println!();

    std::fs::write(policy_path, &new)
        .map_err(|e| format!("could not write {}: {e}", policy_path.display()))?;

    // Moving a capability that is Blocked by default out of `blocked`
    // widens the AI's reach — call it out, but don't block it: this is a
    // deliberate developer edit to a reviewed file.
    if target != Bucket::Blocked {
        if let Some(cap) = CATALOGUE.iter().find(|c| c.key == capability) {
            if cap.default == Bucket::Blocked {
                println!(
                    "{}",
                    style(format!(
                        "note: `{capability}` is blocked by default — this widens what the AI may do."
                    ))
                    .yellow()
                );
            }
        }
    }
    println!(
        "Wrote {}. Commit it — the policy is version-controlled.",
        policy_path.display()
    );
    Ok(())
}

/// Move `capability` into `target`'s bucket array, removing it from the
/// others, and return the new file text. Uses `toml_edit` in place so the
/// template's comments and the untouched buckets are preserved. Pure on
/// its input — unit-tested without the filesystem.
fn edit_policy_text(raw: &str, capability: &str, target: Bucket) -> Result<String, String> {
    if !is_known(capability) {
        return Err(format!("unknown capability {capability:?}"));
    }
    let mut doc = raw
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("`.rustio/ai.toml` is not valid TOML: {e}"))?;

    let ai = doc
        .get_mut("ai")
        .and_then(|i| i.as_table_mut())
        .ok_or_else(|| "`.rustio/ai.toml`: missing [ai] table".to_string())?;

    let target_field = bucket_field(target);

    // Remove the capability from every bucket *except* the target, so a
    // no-op (already in target) leaves the file byte-identical.
    for field in ["allowed", "needs_approval", "blocked"] {
        if field == target_field {
            continue;
        }
        if let Some(arr) = ai.get_mut(field).and_then(|i| i.as_array_mut()) {
            arr.retain(|v| v.as_str() != Some(capability));
        }
    }

    // Ensure the target array exists, then add the capability if absent,
    // matching the template's one-per-line, trailing-comma style.
    if ai.get(target_field).and_then(|i| i.as_array()).is_none() {
        ai[target_field] = toml_edit::Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
    }
    let arr = ai
        .get_mut(target_field)
        .and_then(|i| i.as_array_mut())
        .expect("target array ensured above");
    if !arr.iter().any(|v| v.as_str() == Some(capability)) {
        arr.push(capability);
        let last = arr.len() - 1;
        if let Some(v) = arr.get_mut(last) {
            v.decor_mut().set_prefix("\n  ");
        }
        arr.set_trailing("\n");
        arr.set_trailing_comma(true);
    }

    Ok(doc.to_string())
}

/// A minimal line diff: shared prefix/suffix as context, the changed
/// middle as `-`/`+`. Enough to show which bucket lines moved without a
/// diff dependency. Assumes `old != new`.
fn line_diff(old: &str, new: &str) -> String {
    let o: Vec<&str> = old.lines().collect();
    let n: Vec<&str> = new.lines().collect();

    let mut p = 0;
    while p < o.len() && p < n.len() && o[p] == n[p] {
        p += 1;
    }
    let mut s = 0;
    while s < o.len().saturating_sub(p)
        && s < n.len().saturating_sub(p)
        && o[o.len() - 1 - s] == n[n.len() - 1 - s]
    {
        s += 1;
    }

    let ctx = 2;
    let mut out = String::new();
    let lead = p.saturating_sub(ctx);
    for line in &o[lead..p] {
        out.push_str(&format!("  {line}\n"));
    }
    for line in &o[p..o.len() - s] {
        out.push_str(&format!("{}\n", style(format!("- {line}")).red()));
    }
    for line in &n[p..n.len() - s] {
        out.push_str(&format!("{}\n", style(format!("+ {line}")).green()));
    }
    let tail_end = (o.len() - s + ctx).min(o.len());
    for line in &o[o.len() - s..tail_end] {
        out.push_str(&format!("  {line}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> PathBuf {
        PathBuf::from(POLICY_PATH)
    }

    fn bucket_for(policy: &Policy, key: &str) -> Bucket {
        let cap = CATALOGUE.iter().find(|c| c.key == key).expect("known key");
        policy.bucket_of(cap)
    }

    fn temp_store() -> Store {
        let root = std::env::temp_dir().join(format!("rustio-ai-test-{}", new_ulid()));
        std::fs::create_dir_all(&root).expect("temp dir");
        Store::new(root)
    }

    fn stage(path: &str, content: &str) -> StagedChange {
        StagedChange {
            path: path.to_string(),
            content: content.to_string(),
        }
    }

    // ---- policy ----------------------------------------------------------

    #[test]
    fn defaults_sort_capabilities_into_expected_buckets() {
        let d = Policy::defaults();
        assert_eq!(bucket_for(&d, "create_model"), Bucket::Allowed);
        assert_eq!(bucket_for(&d, "draft_migration"), Bucket::Allowed);
        assert_eq!(bucket_for(&d, "apply_migration"), Bucket::NeedsApproval);
        assert_eq!(bucket_for(&d, "modify_table"), Bucket::NeedsApproval);
        assert_eq!(bucket_for(&d, "security_settings"), Bucket::Blocked);
        assert_eq!(bucket_for(&d, "edit_ai_policy"), Bucket::Blocked);
    }

    #[test]
    fn default_template_matches_coded_defaults() {
        let parsed = parse_policy(DEFAULT_POLICY_TOML, &p()).expect("template parses");
        let defaults = Policy::defaults();
        assert_eq!(parsed.assistant, defaults.assistant);
        assert_eq!(parsed.approver_role, defaults.approver_role);
        assert_eq!(parsed.second_approver_for, defaults.second_approver_for);
        assert!(
            parsed.warnings.is_empty(),
            "template warnings: {:?}",
            parsed.warnings
        );
        for cap in CATALOGUE {
            assert_eq!(
                parsed.bucket_of(cap),
                defaults.bucket_of(cap),
                "bucket drift for {}",
                cap.key
            );
        }
    }

    #[test]
    fn file_can_move_a_capability_between_buckets() {
        let toml = r#"
            [ai]
            assistant = "Cursor"
            allowed = ["edit_existing_code"]
            blocked = ["create_model"]
        "#;
        let policy = parse_policy(toml, &p()).expect("parses");
        assert_eq!(policy.assistant, "Cursor");
        assert_eq!(bucket_for(&policy, "edit_existing_code"), Bucket::Allowed);
        assert_eq!(bucket_for(&policy, "create_model"), Bucket::Blocked);
        assert_eq!(bucket_for(&policy, "security_settings"), Bucket::Blocked);
    }

    #[test]
    fn unknown_capability_is_warned_and_ignored() {
        let toml = r#"
            [ai]
            allowed = ["create_model", "launch_missiles"]
        "#;
        let policy = parse_policy(toml, &p()).expect("parses");
        assert!(
            policy
                .warnings
                .iter()
                .any(|w| w.contains("launch_missiles")),
            "expected a warning, got {:?}",
            policy.warnings
        );
        assert_eq!(bucket_for(&policy, "create_model"), Bucket::Allowed);
    }

    #[test]
    fn missing_ai_table_is_an_error() {
        let err = parse_policy("schema_version = 1\n", &p()).unwrap_err();
        assert!(err.contains("[ai]"), "got: {err}");
    }

    #[test]
    fn malformed_toml_is_an_error() {
        let err = parse_policy("[ai\nassistant =", &p()).unwrap_err();
        assert!(err.contains("not valid TOML"), "got: {err}");
    }

    #[test]
    fn second_approver_override_is_read() {
        let toml = r#"
            [ai]
            needs_approval = ["apply_migration"]
            [ai.approval]
            approver_role = "developer"
            second_approver_for = ["apply_migration"]
        "#;
        let policy = parse_policy(toml, &p()).expect("parses");
        assert_eq!(policy.approver_role, "developer");
        assert!(policy.needs_two_approvers("apply_migration"));
        assert!(!policy.needs_two_approvers("modify_table"));
    }

    // ---- required approvals ---------------------------------------------

    #[test]
    fn required_approvals_follow_bucket_and_second_approver() {
        let d = Policy::defaults();
        assert_eq!(d.required_approvals("create_model", Bucket::Allowed), 0);
        assert_eq!(
            d.required_approvals("edit_existing_code", Bucket::NeedsApproval),
            1
        );
        // modify_table is in the default second_approver_for list.
        assert_eq!(
            d.required_approvals("modify_table", Bucket::NeedsApproval),
            2
        );
    }

    // ---- proposal lifecycle ---------------------------------------------

    #[test]
    fn propose_creates_a_loadable_proposal() {
        let store = temp_store();
        let policy = Policy::defaults();
        let p = do_propose(
            &store,
            &policy,
            "create_model",
            "Add Customer",
            None,
            vec![stage("src/models/customer.rs", "// model")],
            "claude".into(),
        )
        .expect("created");
        assert_eq!(p.state, State::Suggested);
        assert_eq!(p.required_approvals, 0); // Allowed
        let reloaded = store.load(p.short()).expect("loads by prefix");
        assert_eq!(reloaded.id, p.id);
        assert_eq!(reloaded.changes.len(), 1);
    }

    #[test]
    fn blocked_capability_is_refused_and_logged() {
        let store = temp_store();
        let policy = Policy::defaults();
        let err = do_propose(
            &store,
            &policy,
            "security_settings",
            "Grant delete",
            None,
            vec![],
            "claude".into(),
        )
        .unwrap_err();
        assert!(err.contains("Blocked"), "got: {err}");
        // The refusal is recorded.
        let log = store.recent_log(10);
        assert!(log.iter().any(|e| e.event == "blocked"));
    }

    #[test]
    fn unknown_capability_is_refused() {
        let store = temp_store();
        let policy = Policy::defaults();
        let err = do_propose(&store, &policy, "fly", "x", None, vec![], "c".into()).unwrap_err();
        assert!(err.contains("unknown capability"), "got: {err}");
    }

    #[test]
    fn single_approval_flips_needs_approval_to_approved() {
        let store = temp_store();
        let policy = Policy::defaults();
        let p = do_propose(
            &store,
            &policy,
            "edit_existing_code",
            "tweak",
            None,
            vec![],
            "claude".into(),
        )
        .unwrap();
        assert_eq!(p.required_approvals, 1);
        let approved = do_approve(&store, p.short(), "amir".into()).unwrap();
        assert_eq!(approved.state, State::Approved);
    }

    #[test]
    fn two_approver_capability_needs_two_distinct_approvers() {
        let store = temp_store();
        let policy = Policy::defaults();
        let p = do_propose(
            &store,
            &policy,
            "modify_table",
            "add column",
            None,
            vec![],
            "claude".into(),
        )
        .unwrap();
        assert_eq!(p.required_approvals, 2);

        // First approval: still pending.
        let after_one = do_approve(&store, p.short(), "amir".into()).unwrap();
        assert_eq!(after_one.state, State::Suggested);

        // Same approver again: refused.
        let dup = do_approve(&store, p.short(), "amir".into()).unwrap_err();
        assert!(dup.contains("distinct"), "got: {dup}");

        // Second, distinct approver: now approved.
        let after_two = do_approve(&store, p.short(), "sara".into()).unwrap();
        assert_eq!(after_two.state, State::Approved);
    }

    #[test]
    fn apply_writes_staged_files_and_marks_applied() {
        let store = temp_store();
        let policy = Policy::defaults();
        let p = do_propose(
            &store,
            &policy,
            "create_model",
            "Add Customer",
            None,
            vec![stage("src/models/customer.rs", "// the model\n")],
            "claude".into(),
        )
        .unwrap();
        let (applied, written) = do_apply(&store, p.short(), "amir".into()).unwrap();
        assert_eq!(applied.state, State::Applied);
        assert_eq!(written, vec!["src/models/customer.rs".to_string()]);
        let on_disk = std::fs::read_to_string(store.root.join("src/models/customer.rs"))
            .expect("file written");
        assert_eq!(on_disk, "// the model\n");
    }

    #[test]
    fn apply_is_refused_before_approval() {
        let store = temp_store();
        let policy = Policy::defaults();
        let p = do_propose(
            &store,
            &policy,
            "modify_table",
            "add column",
            None,
            vec![],
            "claude".into(),
        )
        .unwrap();
        let err = do_apply(&store, p.short(), "amir".into()).unwrap_err();
        assert!(err.contains("needs approval"), "got: {err}");
    }

    #[test]
    fn rejected_proposal_cannot_be_approved_or_applied() {
        let store = temp_store();
        let policy = Policy::defaults();
        let p = do_propose(
            &store,
            &policy,
            "edit_existing_code",
            "tweak",
            None,
            vec![],
            "claude".into(),
        )
        .unwrap();
        let rejected = do_reject(&store, p.short(), "not now", "amir".into()).unwrap();
        assert_eq!(rejected.state, State::Rejected);
        assert!(do_approve(&store, p.short(), "sara".into()).is_err());
        assert!(do_apply(&store, p.short(), "sara".into()).is_err());
    }

    #[test]
    fn staged_destination_rejects_path_traversal() {
        assert!(validate_dest("src/ok.rs").is_ok());
        assert!(validate_dest("../escape.rs").is_err());
        assert!(validate_dest("/etc/passwd").is_err());
    }

    #[test]
    fn lifecycle_actions_are_recorded_in_the_log_oldest_first() {
        let store = temp_store();
        let policy = Policy::defaults();
        let p = do_propose(
            &store,
            &policy,
            "edit_existing_code",
            "tweak",
            None,
            vec![],
            "claude".into(),
        )
        .unwrap();
        do_approve(&store, p.short(), "amir".into()).unwrap();
        do_apply(&store, p.short(), "amir".into()).unwrap();

        let log = store.read_log();
        let events: Vec<&str> = log.iter().map(|e| e.event.as_str()).collect();
        assert_eq!(events, vec!["suggested", "approved", "applied"]);
        assert!(log.iter().all(|e| e.proposal == p.id));
    }

    #[test]
    fn format_log_aligns_and_trims() {
        let entries = vec![
            LogEntry {
                id: new_ulid(),
                ts: "2026-05-30T18:00:00Z".into(),
                event: "suggested".into(),
                proposal: "01KSX0N1HYP380HBBFV16F7Z8G".into(),
                capability: "modify_table".into(),
                by: "claude".into(),
                detail: Some("Add phone column".into()),
            },
            LogEntry {
                id: new_ulid(),
                ts: "2026-05-30T18:01:00Z".into(),
                event: "blocked".into(),
                proposal: "-".into(),
                capability: "security_settings".into(),
                by: "claude".into(),
                detail: None,
            },
        ];
        let out = format_log(&entries);
        // Suffix handle, capability, and detail all present.
        assert!(out.contains("V16F7Z8G"));
        assert!(out.contains("Add phone column"));
        // No trailing whitespace on the detail-less line.
        for line in out.lines() {
            assert_eq!(line, line.trim_end(), "line has trailing space: {line:?}");
        }
        // The `-` proposal renders as `-`, not a panic on slicing.
        assert!(out.lines().nth(1).unwrap().contains("blocked"));
    }

    // ---- allow / deny (policy editing) ----------------------------------

    #[test]
    fn edit_moves_capability_and_preserves_comments() {
        let new = edit_policy_text(DEFAULT_POLICY_TOML, "edit_existing_code", Bucket::Allowed)
            .expect("edit");
        let policy = parse_policy(&new, &p()).expect("re-parses");
        assert_eq!(bucket_for(&policy, "edit_existing_code"), Bucket::Allowed);
        // It left needs_approval (its default).
        assert_ne!(
            bucket_for(&policy, "edit_existing_code"),
            Bucket::NeedsApproval
        );
        // The template's header comment survives the rewrite.
        assert!(new.contains("# Contract: docs/design/DESIGN_AI_ASSISTANT.md"));
        // No stray duplicate placement warning.
        assert!(
            policy.warnings.is_empty(),
            "warnings: {:?}",
            policy.warnings
        );
    }

    #[test]
    fn edit_to_current_bucket_is_a_noop() {
        // create_model is already Allowed in the template.
        let new =
            edit_policy_text(DEFAULT_POLICY_TOML, "create_model", Bucket::Allowed).expect("edit");
        assert_eq!(new, DEFAULT_POLICY_TOML, "no-op must be byte-identical");
    }

    #[test]
    fn deny_then_allow_round_trips_a_capability() {
        let denied =
            edit_policy_text(DEFAULT_POLICY_TOML, "create_model", Bucket::Blocked).expect("deny");
        let denied_policy = parse_policy(&denied, &p()).expect("parses");
        assert_eq!(bucket_for(&denied_policy, "create_model"), Bucket::Blocked);

        let allowed =
            edit_policy_text(&denied, "create_model", Bucket::Allowed).expect("allow back");
        let allowed_policy = parse_policy(&allowed, &p()).expect("parses");
        assert_eq!(bucket_for(&allowed_policy, "create_model"), Bucket::Allowed);
        // The capability must appear in exactly one bucket array — its key
        // is unique to those arrays, so a whole-file count is a fair check.
        assert_eq!(allowed.matches("\"create_model\"").count(), 1, "{allowed}");
    }

    #[test]
    fn edit_rejects_unknown_capability() {
        let err = edit_policy_text(DEFAULT_POLICY_TOML, "fly", Bucket::Allowed).unwrap_err();
        assert!(err.contains("unknown capability"), "got: {err}");
    }

    #[test]
    fn line_diff_marks_changed_lines() {
        let old = "a\nb\nc\n";
        let new = "a\nB\nc\n";
        let d = line_diff(old, new);
        assert!(d.contains("- b"), "got: {d}");
        assert!(d.contains("+ B"), "got: {d}");
        assert!(d.contains("  a"), "context missing: {d}");
    }

    #[test]
    fn proposals_made_together_have_distinct_resolvable_handles() {
        // Two proposals in the same second share a ULID timestamp prefix;
        // the suffix handle must still distinguish them and resolve.
        let store = temp_store();
        let policy = Policy::defaults();
        let a = do_propose(
            &store,
            &policy,
            "create_model",
            "A",
            None,
            vec![],
            "c".into(),
        )
        .unwrap();
        let b = do_propose(
            &store,
            &policy,
            "create_form",
            "B",
            None,
            vec![],
            "c".into(),
        )
        .unwrap();
        assert_ne!(a.short(), b.short(), "short handles must differ");
        assert_eq!(store.load(a.short()).unwrap().id, a.id);
        assert_eq!(store.load(b.short()).unwrap().id, b.id);
    }
}
