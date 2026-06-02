//! Working-tree integrity backstop for `verify` (`DESIGN_CLOUD_IMPL.md`
//! §11).
//!
//! Entry files are write-once; the only sanctioned in-place change is a
//! ratified redaction (which sets `redacted = true`). This best-effort
//! check shells out to git to flag tracked entry files that have been
//! **modified** without the redacted flag, **renamed**, or **deleted** in
//! the working tree — a hand-edit or removal that bypassed the lifecycle.
//!
//! Honest limits (§11): it compares the working tree against the index /
//! HEAD only. It does **not** forensically audit committed history, and it
//! degrades under squash-merge / force-push (a tamper folded into an
//! already-merged commit is invisible here). It is a *backstop*, not a
//! guarantee. It skips cleanly when git is unavailable or the project is not
//! a git repo.

use std::path::Path;
use std::process::Command;

use super::entry::Entry;
use super::store::Store;

/// The outcome of the working-tree integrity check.
pub(crate) enum Report {
    /// Not a git repo (or git unavailable) — the check was skipped.
    NotARepo,
    /// No tracked entry file is modified/renamed/deleted in the working tree.
    Clean,
    /// Entry files changed outside the lifecycle (one description per file).
    Tampered(Vec<String>),
}

/// A working-tree change to a tracked entry file.
enum Change {
    Modified,
    Deleted,
    Renamed,
}

/// Run the check against `store`'s project root.
pub(crate) fn check(store: &Store) -> Report {
    let root = store.root();
    if !inside_repo(root) {
        return Report::NotARepo;
    }
    let porcelain = match git_status(root) {
        Some(s) => s,
        None => return Report::NotARepo,
    };

    let mut tampered = Vec::new();
    for (change, rel) in parse_porcelain(&porcelain) {
        match change {
            Change::Deleted => {
                tampered.push(format!("{rel} (deleted — entries are never removed)"))
            }
            Change::Renamed => {
                tampered.push(format!("{rel} (renamed — the filename is the entry id)"))
            }
            Change::Modified => {
                // A modification is sanctioned only if it is a redaction:
                // the entry now carries `redacted = true`. Anything else is
                // a hand-edit of write-once reasoning.
                if !is_redaction(root, &rel) {
                    tampered.push(format!("{rel} (modified outside a redaction)"));
                }
            }
        }
    }

    if tampered.is_empty() {
        Report::Clean
    } else {
        Report::Tampered(tampered)
    }
}

fn inside_repo(root: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git_status(root: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "--", ".rustio/memory/entries"])
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

/// Does the working-tree entry at `rel` carry `redacted = true`? Used to
/// distinguish a sanctioned redaction from a hand-edit. A file that no
/// longer parses counts as *not* a redaction (tamper).
fn is_redaction(root: &Path, rel: &str) -> bool {
    let stem = match Path::new(rel).file_stem().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => return false,
    };
    std::fs::read_to_string(root.join(rel))
        .ok()
        .and_then(|raw| Entry::parse(&stem, &raw).ok())
        .map(|e| e.redacted)
        .unwrap_or(false)
}

/// Classify `git status --porcelain` lines that touch an entry `*.md` file.
/// Pure (no git, no filesystem) so it is unit-testable. Untracked (`??`) and
/// staged-new (`A `) entries are *not* flagged — creating an entry is
/// normal; only changing or removing an existing one is suspect.
fn parse_porcelain(out: &str) -> Vec<(Change, String)> {
    let mut changes = Vec::new();
    for line in out.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = &line[..2];
        // Rename lines look like `R  old -> new`; take the destination.
        let rest = line[3..].trim();
        let path = rest.rsplit(" -> ").next().unwrap_or(rest).trim_matches('"');
        // Restrict to entry files (git's pathspec already scopes real runs;
        // this keeps the pure parser correct on its own).
        if !(path.starts_with(".rustio/memory/entries/") && path.ends_with(".md")) {
            continue;
        }
        if xy.contains('D') {
            changes.push((Change::Deleted, path.to_string()));
        } else if xy.contains('R') {
            changes.push((Change::Renamed, path.to_string()));
        } else if xy.contains('M') {
            changes.push((Change::Modified, path.to_string()));
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(out: &str) -> Vec<(&'static str, String)> {
        parse_porcelain(out)
            .into_iter()
            .map(|(c, p)| {
                let k = match c {
                    Change::Modified => "M",
                    Change::Deleted => "D",
                    Change::Renamed => "R",
                };
                (k, p)
            })
            .collect()
    }

    #[test]
    fn flags_modified_deleted_renamed_entries() {
        let out = " M .rustio/memory/entries/aaa.md\n\
                    D  .rustio/memory/entries/bbb.md\n\
                    R  .rustio/memory/entries/old.md -> .rustio/memory/entries/new.md\n";
        let got = kinds(out);
        assert_eq!(got[0], ("M", ".rustio/memory/entries/aaa.md".into()));
        assert_eq!(got[1], ("D", ".rustio/memory/entries/bbb.md".into()));
        assert_eq!(got[2], ("R", ".rustio/memory/entries/new.md".into()));
    }

    #[test]
    fn ignores_new_and_nonentry_changes() {
        // Untracked new entry, staged-new entry, and a non-.md file.
        let out = "?? .rustio/memory/entries/ccc.md\n\
                   A  .rustio/memory/entries/ddd.md\n\
                    M CLOUD.md\n";
        assert!(kinds(out).is_empty(), "got {:?}", kinds(out));
    }

    #[test]
    fn not_a_repo_when_git_absent() {
        // A bare temp dir is not a git repo → the check skips.
        let dir = std::env::temp_dir().join("rustio-integrity-norepo-xyz123");
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(&dir);
        assert!(matches!(check(&store), Report::NotARepo));
    }
}
