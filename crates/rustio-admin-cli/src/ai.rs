//! `rustio ai` — AI assistant permissions.
//!
//! This is the read-only, offline first slice of
//! `docs/design/DESIGN_AI_ASSISTANT.md`: the `.rustio/ai.toml` policy
//! file and `rustio ai status`. It answers one question — *what is the
//! AI assistant currently allowed to do?* — by reading the policy and
//! printing it. No AI is contacted; no database is opened.
//!
//! The policy sorts every known capability into one of three buckets:
//! **Allowed**, **Needs approval**, or **Blocked** (DESIGN §3). The set
//! of capabilities is a fixed catalogue ([`CATALOGUE`]); the policy file
//! only moves capabilities between buckets. A capability the file does
//! not mention keeps its catalogue default, so `status` always shows the
//! full known surface even against a partial or absent file.
//!
//! The approval lifecycle, the proposal record, and the rest of the
//! `rustio ai` verbs (`review` / `approve` / `apply` / `log`) are later
//! slices and are intentionally not implemented here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use console::style;

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

/// `rustio ai` subcommands. Only the read-only / policy verbs ship in
/// this slice.
#[derive(Subcommand)]
pub(crate) enum Action {
    /// Show what the AI assistant may do: Allowed / Needs approval /
    /// Blocked. Reads the policy and the (future) action log only — no
    /// AI call, no database.
    Status,
    /// Write a default `.rustio/ai.toml` policy file. Refuses to
    /// overwrite an existing policy without `--force`.
    Init {
        /// Overwrite an existing `.rustio/ai.toml`.
        #[arg(long)]
        force: bool,
    },
}

/// Dispatch. Offline and synchronous — no Postgres connection.
pub(crate) fn run(action: Action) -> Result<(), String> {
    match action {
        Action::Status => status(&PathBuf::from(POLICY_PATH)),
        Action::Init { force } => init(&PathBuf::from(POLICY_PATH), force),
    }
}

/// The three buckets a capability can sit in (DESIGN §3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    /// True when `key` requires two approvers.
    fn needs_two_approvers(&self, key: &str) -> bool {
        self.second_approver_for.iter().any(|k| k == key)
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

/// `rustio ai status` — print the resolved policy.
fn status(path: &Path) -> Result<(), String> {
    let policy = load_policy(path)?;

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

    // The proposal record is a later slice; be honest about it rather
    // than print an empty section that looks like a feature.
    println!();
    println!("Pending your review:");
    println!(
        "  {}",
        style("(none — approval tracking not enabled yet)").dim()
    );
    println!();
    println!("Recent actions:");
    println!("  {}", style("(none — action log not enabled yet)").dim());

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
        // The shipped `rustio ai init` template must resolve to exactly
        // the coded defaults, or the two silently drift.
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
        // Moved out of their defaults:
        assert_eq!(bucket_for(&policy, "edit_existing_code"), Bucket::Allowed);
        assert_eq!(bucket_for(&policy, "create_model"), Bucket::Blocked);
        // Untouched capability keeps its catalogue default:
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
        // The known key still applied; the unknown one did not crash it.
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
}
