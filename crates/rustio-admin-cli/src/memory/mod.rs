//! `rustio memory` — project memory (CLOUD.md).
//!
//! Implements the read/derive half of `docs/design/DESIGN_CLOUD_IMPL.md`:
//! the per-entry store (§2), the entry model and derived status (§3), and
//! the generated `CLOUD.md` view (§2.1). This slice ships the **read-only**
//! verbs — `render`, `show`, `verify`. The governed write path
//! (`remember` / `supersede` / `redact`, approval, audit) reuses the
//! `DESIGN_AI_ASSISTANT.md` lifecycle and lands in a later slice.
//!
//! The whole surface is **offline and synchronous** — no AI is contacted
//! and no database is opened. The running admin never reads CLOUD.md
//! (`DESIGN_CLOUD.md` §12); this is dev-time tooling only.

mod entry;
mod render;
mod store;
mod write;

use clap::Subcommand;
use console::style;

use entry::{short, Entry, EntryType};
use store::{Memory, Status, Store};

/// `rustio memory` subcommands (read-only slice).
#[derive(Subcommand)]
pub(crate) enum Action {
    /// Regenerate `CLOUD.md` from the entry files. Idempotent; the only
    /// writer of CLOUD.md. Run after entries change so the view never
    /// drifts from its source (§2.6).
    Render,
    /// List memory entries, filtered. Read-only and mechanical — exact
    /// filters, never relevance ranking (relevance is the assistant's
    /// job, `DESIGN_CLOUD.md` §13).
    Show {
        /// Only entries carrying this subject.
        #[arg(long)]
        subject: Option<String>,
        /// Only entries of this type (decision, rejected, assumption,
        /// intent, onboarding, history, open-tension).
        #[arg(long = "type", value_name = "TYPE")]
        entry_type: Option<String>,
        /// Only active entries (hide superseded / forked).
        #[arg(long)]
        active: bool,
        /// Only entries whose body contains this text (case-insensitive).
        #[arg(long)]
        grep: Option<String>,
    },
    /// Check that `CLOUD.md` is fresh and the entries are well-formed (no
    /// dangling or cyclic supersessions). Non-zero exit on failure — the
    /// freshness gate (§2.6).
    Verify,

    /// Propose a new memory entry (the *why* behind a decision). Creates a
    /// `needs_approval` proposal; a human approves, then `apply` writes the
    /// entry and re-renders CLOUD.md. Offline — identity via `--by`.
    Remember {
        /// Entry type: decision | rejected | assumption | intent |
        /// onboarding | history | open-tension.
        #[arg(long = "type", value_name = "TYPE")]
        entry_type: String,
        /// Subject tag (repeatable) — the mechanical retrieval key.
        #[arg(long = "subject", value_name = "SUBJECT")]
        subjects: Vec<String>,
        /// The reasoning prose to remember.
        #[arg(long)]
        note: String,
        /// Mark as foundational (exempt from recency demotion). Requires
        /// two approvers.
        #[arg(long)]
        foundational: bool,
        /// A supporting reference (repeatable), e.g. `pr#41`.
        #[arg(long = "source", value_name = "REF")]
        sources: Vec<String>,
        /// Who is proposing (defaults to the OS user).
        #[arg(long)]
        by: Option<String>,
    },
    /// Propose an entry that supersedes an existing one. The superseded
    /// entry stays in the log, visibly demoted (never removed).
    Supersede {
        /// The entry id (full or suffix) being superseded.
        id: String,
        /// Type of the new (superseding) entry.
        #[arg(long = "type", value_name = "TYPE")]
        entry_type: String,
        #[arg(long = "subject", value_name = "SUBJECT")]
        subjects: Vec<String>,
        #[arg(long)]
        note: String,
        #[arg(long)]
        foundational: bool,
        #[arg(long = "source", value_name = "REF")]
        sources: Vec<String>,
        #[arg(long)]
        by: Option<String>,
    },
    /// List memory proposals awaiting a decision.
    Pending {
        #[arg(long)]
        by: Option<String>,
    },
    /// Approve a pending memory proposal (distinct approvers enforced).
    Approve {
        id: String,
        #[arg(long)]
        by: Option<String>,
    },
    /// Reject a memory proposal with a reason, keeping the record.
    Reject {
        id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        by: Option<String>,
    },
    /// Apply an approved memory proposal: write the entry file and
    /// re-render CLOUD.md.
    Apply {
        id: String,
        #[arg(long)]
        by: Option<String>,
    },
}

/// Dispatch. Offline and synchronous — no Postgres connection.
pub(crate) fn run(action: Action) -> Result<(), String> {
    let store = Store::new(".");
    match action {
        Action::Render => render_cmd(&store),
        Action::Show {
            subject,
            entry_type,
            active,
            grep,
        } => show_cmd(
            &store,
            subject.as_deref(),
            entry_type.as_deref(),
            active,
            grep.as_deref(),
        ),
        Action::Verify => verify_cmd(&store),
        Action::Remember {
            entry_type,
            subjects,
            note,
            foundational,
            sources,
            by,
        } => write::remember(entry_type, subjects, foundational, sources, note, by),
        Action::Supersede {
            id,
            entry_type,
            subjects,
            note,
            foundational,
            sources,
            by,
        } => write::supersede(id, entry_type, subjects, foundational, sources, note, by),
        Action::Pending { by } => write::pending(by),
        Action::Approve { id, by } => write::approve(id, by),
        Action::Reject { id, reason, by } => write::reject(id, reason, by),
        Action::Apply { id, by } => write::apply(id, by),
    }
}

fn render_cmd(store: &Store) -> Result<(), String> {
    let mem = Memory::build(store.load_entries()?)?;
    let out = render::render(&mem);
    let path = store.cloud_md_path();
    std::fs::write(&path, &out).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    let n = mem.entries.len();
    println!(
        "{} {} ({n} entr{})",
        style("rendered").green().bold(),
        path.display(),
        if n == 1 { "y" } else { "ies" }
    );
    Ok(())
}

fn show_cmd(
    store: &Store,
    subject: Option<&str>,
    entry_type: Option<&str>,
    active_only: bool,
    grep: Option<&str>,
) -> Result<(), String> {
    let want_type = entry_type.map(EntryType::parse).transpose()?;
    let needle = grep.map(str::to_lowercase);
    let mem = Memory::build(store.load_entries()?)?;

    let mut shown = 0usize;
    for e in &mem.entries {
        let status = mem.status_of(&e.id);
        if active_only && status != Status::Active {
            continue;
        }
        if let Some(s) = subject {
            if !e.subjects.iter().any(|x| x == s) {
                continue;
            }
        }
        if let Some(t) = want_type {
            if e.entry_type != t {
                continue;
            }
        }
        if let Some(n) = &needle {
            if !e.body.to_lowercase().contains(n) {
                continue;
            }
        }
        print_row(e, &status);
        shown += 1;
    }
    if shown == 0 {
        println!("{}", style("no matching entries").dim());
    }
    Ok(())
}

fn verify_cmd(store: &Store) -> Result<(), String> {
    render::verify(store)?;
    println!(
        "{} CLOUD.md is fresh and entries are well-formed",
        style("ok").green().bold()
    );
    Ok(())
}

/// One entry's summary line plus the first line of its body.
fn print_row(e: &Entry, status: &Status) {
    let tag = match status {
        Status::Active => style("active").green(),
        Status::Superseded(_) => style("superseded").dim(),
        Status::Forked(_) => style("open-tension").yellow(),
    };
    let star = if e.foundational { " ⭑" } else { "" };
    println!(
        "{}  {}  {}  [{}]{}",
        style(short(&e.id)).cyan(),
        e.date,
        e.entry_type.as_str(),
        tag,
        star
    );
    let first = e.body.lines().next().unwrap_or("");
    let subjects = if e.subjects.is_empty() {
        String::new()
    } else {
        format!("   ({})", e.subjects.join(", "))
    };
    println!("    {first}{}", style(subjects).dim());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ulid_gen::new_ulid;

    fn temp_store_with(entries: &[(&str, &str, &str, Option<&str>)]) -> Store {
        let root = std::env::temp_dir().join(format!("rustio-memory-cmd-{}", new_ulid()));
        let dir = root.join(".rustio").join("memory").join("entries");
        std::fs::create_dir_all(&dir).unwrap();
        for (id, ty, date, sup) in entries {
            let content = format!(
                "+++\nid = \"{id}\"\ntype = \"{ty}\"\nsubjects = [\"core\"]\n\
                 supersedes = \"{}\"\nfoundational = false\nsources = []\n\
                 author = \"ai:test\"\nratified_by = \"t@e\"\ndate = \"{date}\"\n\
                 correlation_id = \"c\"\n+++\n\nBody of {id}.\n",
                sup.unwrap_or("")
            );
            std::fs::write(dir.join(format!("{id}.md")), content).unwrap();
        }
        Store::new(root)
    }

    #[test]
    fn render_then_verify_round_trips() {
        let s = temp_store_with(&[("aaa", "decision", "2026-01-01", None)]);
        render_cmd(&s).expect("render");
        verify_cmd(&s).expect("verify after render");
    }

    #[test]
    fn verify_fails_before_render() {
        let s = temp_store_with(&[("aaa", "decision", "2026-01-01", None)]);
        assert!(verify_cmd(&s).is_err());
    }

    #[test]
    fn show_rejects_unknown_type_filter() {
        let s = temp_store_with(&[("aaa", "decision", "2026-01-01", None)]);
        let err = show_cmd(&s, None, Some("musing"), false, None).unwrap_err();
        assert!(err.contains("unknown entry type"), "{err}");
    }

    #[test]
    fn show_accepts_valid_filters() {
        let s = temp_store_with(&[
            ("aaa", "decision", "2026-01-01", None),
            ("bbb", "rejected", "2026-02-01", None),
        ]);
        // Smoke: filters parse and run without error.
        show_cmd(&s, Some("core"), Some("rejected"), true, Some("bbb")).expect("show");
    }
}
