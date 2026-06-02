//! The file-backed entry store and the derived memory model.
//!
//! Implements `docs/design/DESIGN_CLOUD_IMPL.md` §2 (storage) and §3.3
//! (derived status). The store reads write-once entry files; the
//! [`Memory`] model derives supersession status and validates referential
//! integrity (no dangling links, no cycles — review-pass-2 finding D) and
//! the merge-fork case (one entry, two successors → an open tension —
//! finding E).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::entry::Entry;

/// File-backed store rooted at a project directory. Parameterised by `root`
/// (rather than assuming the cwd) so it is testable against a temp dir —
/// the same shape as `ai.rs`'s `Store`.
pub(crate) struct Store {
    root: PathBuf,
}

impl Store {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Store { root: root.into() }
    }

    /// `.rustio/memory/entries/` — the canonical per-entry files (§2.3).
    pub(crate) fn entries_dir(&self) -> PathBuf {
        self.root.join(".rustio").join("memory").join("entries")
    }

    /// The generated human view at the project root (§2.1).
    pub(crate) fn cloud_md_path(&self) -> PathBuf {
        self.root.join("CLOUD.md")
    }

    /// Read and parse every `*.md` entry, sorted by `(date, id)` — the
    /// apply-date order key (§2.3). A missing directory is an empty store.
    pub(crate) fn load_entries(&self) -> Result<Vec<Entry>, String> {
        let dir = self.entries_dir();
        let read = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("could not read {}: {e}", dir.display())),
        };
        let mut out = Vec::new();
        for entry in read {
            let path = entry
                .map_err(|e| format!("could not read {}: {e}", dir.display()))?
                .path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("{}: non-UTF-8 filename", path.display()))?
                .to_string();
            let raw = fs::read_to_string(&path)
                .map_err(|e| format!("could not read {}: {e}", path.display()))?;
            let parsed =
                Entry::parse(&stem, &raw).map_err(|e| format!("{}: {e}", path.display()))?;
            out.push(parsed);
        }
        out.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.id.cmp(&b.id)));
        Ok(out)
    }
}

/// The derived status of an entry (§3.3) — never stored, always computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// No later entry supersedes this one.
    Active,
    /// Superseded by exactly one later entry (its id).
    Superseded(String),
    /// Superseded by more than one entry — an unresolved merge fork,
    /// surfaced as an open tension (§3.3, finding E).
    Forked(Vec<String>),
}

/// Entries plus the derived supersession graph. Construction validates
/// referential integrity, so a `Memory` always represents a well-formed
/// store.
#[derive(Debug)]
pub(crate) struct Memory {
    pub(crate) entries: Vec<Entry>,
    /// id → ids of the entries that supersede it.
    superseded_by: BTreeMap<String, Vec<String>>,
}

impl Memory {
    /// Build the model from loaded entries, validating that every
    /// `supersedes:` target exists (no dangling links) and that the
    /// supersession graph is acyclic.
    pub(crate) fn build(entries: Vec<Entry>) -> Result<Memory, String> {
        let known: std::collections::BTreeSet<&str> =
            entries.iter().map(|e| e.id.as_str()).collect();

        let mut superseded_by: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for e in &entries {
            if let Some(target) = &e.supersedes {
                if !known.contains(target.as_str()) {
                    return Err(format!(
                        "entry {:?} supersedes unknown entry {target:?} (dangling link)",
                        e.id
                    ));
                }
                superseded_by
                    .entry(target.clone())
                    .or_default()
                    .push(e.id.clone());
            }
        }
        detect_cycles(&entries)?;
        // Stable ordering of successors for deterministic rendering.
        for v in superseded_by.values_mut() {
            v.sort();
        }
        Ok(Memory {
            entries,
            superseded_by,
        })
    }

    /// The derived status of an entry id (§3.3).
    pub(crate) fn status_of(&self, id: &str) -> Status {
        match self.superseded_by.get(id) {
            None => Status::Active,
            Some(v) if v.len() == 1 => Status::Superseded(v[0].clone()),
            Some(v) => Status::Forked(v.clone()),
        }
    }
}

/// Reject a cycle in the supersession graph. Each entry has at most one
/// outgoing `supersedes` edge, so following edges from any node either
/// terminates or revisits — bounded by the entry count.
fn detect_cycles(entries: &[Entry]) -> Result<(), String> {
    let next: BTreeMap<&str, &str> = entries
        .iter()
        .filter_map(|e| e.supersedes.as_deref().map(|t| (e.id.as_str(), t)))
        .collect();
    for e in entries {
        let mut cur = e.id.as_str();
        let mut steps = 0usize;
        while let Some(&target) = next.get(cur) {
            steps += 1;
            if steps > entries.len() {
                return Err(format!("supersession cycle detected involving {:?}", e.id));
            }
            cur = target;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ulid_gen::new_ulid;

    fn temp_store() -> Store {
        let root = std::env::temp_dir().join(format!("rustio-memory-test-{}", new_ulid()));
        fs::create_dir_all(root.join(".rustio").join("memory").join("entries")).expect("temp dir");
        Store::new(root)
    }

    fn write_entry(store: &Store, id: &str, ty: &str, date: &str, supersedes: Option<&str>) {
        let sup = supersedes.unwrap_or("");
        let content = format!(
            "+++\n\
             id = \"{id}\"\n\
             type = \"{ty}\"\n\
             subjects = [\"core\"]\n\
             supersedes = \"{sup}\"\n\
             foundational = false\n\
             sources = []\n\
             author = \"ai:test\"\n\
             ratified_by = \"t@e\"\n\
             date = \"{date}\"\n\
             correlation_id = \"c-{id}\"\n\
             +++\n\
             \n\
             Reasoning for {id}.\n"
        );
        fs::write(store.entries_dir().join(format!("{id}.md")), content).expect("write entry");
    }

    fn memory(store: &Store) -> Result<Memory, String> {
        Memory::build(store.load_entries()?)
    }

    #[test]
    fn empty_store_loads_nothing() {
        let s = Store::new(std::env::temp_dir().join(format!("rustio-empty-{}", new_ulid())));
        assert!(s.load_entries().expect("ok").is_empty());
    }

    #[test]
    fn entries_load_sorted_by_date_then_id() {
        let s = temp_store();
        write_entry(&s, "bbb", "decision", "2026-02-01", None);
        write_entry(&s, "aaa", "decision", "2026-01-01", None);
        write_entry(&s, "ccc", "decision", "2026-02-01", None);
        let ids: Vec<_> = s
            .load_entries()
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, vec!["aaa", "bbb", "ccc"]);
    }

    #[test]
    fn active_when_nothing_supersedes() {
        let s = temp_store();
        write_entry(&s, "aaa", "assumption", "2026-01-01", None);
        let m = memory(&s).unwrap();
        assert_eq!(m.status_of("aaa"), Status::Active);
    }

    #[test]
    fn superseded_when_one_successor() {
        let s = temp_store();
        write_entry(&s, "aaa", "assumption", "2026-01-01", None);
        write_entry(&s, "bbb", "assumption", "2026-02-01", Some("aaa"));
        let m = memory(&s).unwrap();
        assert_eq!(m.status_of("aaa"), Status::Superseded("bbb".to_string()));
        assert_eq!(m.status_of("bbb"), Status::Active);
    }

    #[test]
    fn merge_fork_becomes_open_tension() {
        let s = temp_store();
        write_entry(&s, "aaa", "assumption", "2026-01-01", None);
        write_entry(&s, "bbb", "assumption", "2026-02-01", Some("aaa"));
        write_entry(&s, "ccc", "assumption", "2026-03-01", Some("aaa"));
        let m = memory(&s).unwrap();
        assert_eq!(
            m.status_of("aaa"),
            Status::Forked(vec!["bbb".to_string(), "ccc".to_string()])
        );
    }

    #[test]
    fn dangling_supersedes_is_rejected() {
        let s = temp_store();
        write_entry(&s, "bbb", "assumption", "2026-02-01", Some("ghost"));
        let err = memory(&s).unwrap_err();
        assert!(err.contains("dangling link"), "{err}");
    }

    #[test]
    fn cyclic_supersession_is_rejected() {
        let s = temp_store();
        write_entry(&s, "aaa", "assumption", "2026-01-01", Some("bbb"));
        write_entry(&s, "bbb", "assumption", "2026-02-01", Some("aaa"));
        let err = memory(&s).unwrap_err();
        assert!(err.contains("cycle"), "{err}");
    }
}
