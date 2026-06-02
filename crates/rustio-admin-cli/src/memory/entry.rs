//! The memory entry model and its on-disk parser.
//!
//! Implements `docs/design/DESIGN_CLOUD_IMPL.md` §3: one immutable
//! per-entry file (`.rustio/memory/entries/<ulid>.md`) made of a TOML
//! frontmatter header (fenced by `+++`) and a Markdown prose body. The
//! frontmatter is parsed with `toml_edit` — already a CLI dependency, so
//! the format adds none (§3.1).
//!
//! Entries are write-once; this module only *reads* them. The status
//! (`active` / `superseded`) is **derived** (§3.3, see `store.rs`) and is
//! deliberately not a stored field.

/// The seven kinds of memory entry (DESIGN_CLOUD_IMPL §3.2). `OpenTension`
/// is an ordinary entry whose prose records an unresolved disagreement; it
/// closes by supersession like any other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EntryType {
    Decision,
    Rejected,
    Assumption,
    Intent,
    Onboarding,
    History,
    OpenTension,
}

impl EntryType {
    /// Parse the `type` frontmatter value. Unknown values are rejected so a
    /// typo never silently mis-files an entry.
    pub(crate) fn parse(s: &str) -> Result<Self, String> {
        Ok(match s {
            "decision" => Self::Decision,
            "rejected" => Self::Rejected,
            "assumption" => Self::Assumption,
            "intent" => Self::Intent,
            "onboarding" => Self::Onboarding,
            "history" => Self::History,
            "open-tension" => Self::OpenTension,
            other => {
                return Err(format!(
                    "unknown entry type {other:?} (expected one of: decision, rejected, \
                     assumption, intent, onboarding, history, open-tension)"
                ))
            }
        })
    }

    /// The stable string used in frontmatter and rendered output.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Rejected => "rejected",
            Self::Assumption => "assumption",
            Self::Intent => "intent",
            Self::Onboarding => "onboarding",
            Self::History => "history",
            Self::OpenTension => "open-tension",
        }
    }
}

/// One immutable memory entry: parsed frontmatter plus its prose body.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    /// ULID; also the filename stem. The unambiguous identity.
    pub(crate) id: String,
    pub(crate) entry_type: EntryType,
    /// Mechanical retrieval keys (§3.5). May be empty.
    pub(crate) subjects: Vec<String>,
    /// The entry id this one supersedes, if any (§3.3). `None` for the
    /// common case; an empty frontmatter string is treated as `None`.
    pub(crate) supersedes: Option<String>,
    /// Curated, second-approver-gated flag (§3.6). Read-only here.
    pub(crate) foundational: bool,
    /// Cite-or-hedge references (§10). May be empty.
    pub(crate) sources: Vec<String>,
    pub(crate) author: String,
    pub(crate) ratified_by: String,
    /// Apply (ratification) date, ISO `YYYY-MM-DD`. The render/show order
    /// key (§2.3) — ISO dates sort lexically, i.e. chronologically.
    pub(crate) date: String,
    /// UUID v7; joins to `rustio_admin_actions`.
    pub(crate) correlation_id: String,
    /// The reasoning prose (trimmed). Never specification.
    pub(crate) body: String,
}

impl Entry {
    /// Parse one entry file. `id_from_filename` is the file stem; the
    /// frontmatter `id` must match it, so a renamed file can never silently
    /// disagree with its own identity.
    pub(crate) fn parse(id_from_filename: &str, raw: &str) -> Result<Entry, String> {
        let (frontmatter, body) = split_frontmatter(raw)?;
        let doc = frontmatter
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| format!("invalid TOML frontmatter: {e}"))?;

        let get_str = |k: &str| doc.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let require =
            |k: &str| get_str(k).ok_or_else(|| format!("frontmatter missing required `{k}`"));

        let id = require("id")?;
        if id != id_from_filename {
            return Err(format!(
                "frontmatter id {id:?} does not match filename {id_from_filename:?}"
            ));
        }
        let entry_type = EntryType::parse(&require("type")?)?;
        let supersedes = get_str("supersedes").filter(|s| !s.is_empty());
        let foundational = doc
            .get("foundational")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let author = require("author")?;
        let ratified_by = require("ratified_by")?;
        let date = require("date")?;
        validate_iso_date(&date)?;
        let correlation_id = require("correlation_id")?;

        if body.is_empty() {
            return Err("entry body (the reasoning prose) is empty".to_string());
        }

        Ok(Entry {
            id,
            entry_type,
            subjects: str_array(&doc, "subjects"),
            supersedes,
            foundational,
            sources: str_array(&doc, "sources"),
            author,
            ratified_by,
            date,
            correlation_id,
            body,
        })
    }

    /// Serialize the entry back to its on-disk form: a `+++`-fenced TOML
    /// frontmatter header followed by the prose body. Built with
    /// `toml_edit` so string escaping is always valid. Round-trips with
    /// [`Entry::parse`].
    pub(crate) fn to_file_string(&self) -> String {
        use toml_edit::{value, Array, DocumentMut};

        let mut subjects = Array::new();
        for s in &self.subjects {
            subjects.push(s.as_str());
        }
        let mut sources = Array::new();
        for s in &self.sources {
            sources.push(s.as_str());
        }

        let mut doc = DocumentMut::new();
        doc["id"] = value(self.id.as_str());
        doc["type"] = value(self.entry_type.as_str());
        doc["subjects"] = value(subjects);
        doc["supersedes"] = value(self.supersedes.as_deref().unwrap_or(""));
        doc["foundational"] = value(self.foundational);
        doc["sources"] = value(sources);
        doc["author"] = value(self.author.as_str());
        doc["ratified_by"] = value(self.ratified_by.as_str());
        doc["date"] = value(self.date.as_str());
        doc["correlation_id"] = value(self.correlation_id.as_str());

        format!("+++\n{doc}+++\n\n{}\n", self.body)
    }
}

/// The display handle: the last 8 characters of the id (ULIDs are ASCII,
/// so this is always a char boundary). Mirrors `ai.rs`'s suffix handle.
pub(crate) fn short(id: &str) -> &str {
    &id[id.len().saturating_sub(8)..]
}

/// Split a `+++`-fenced TOML frontmatter block from the Markdown body.
/// Returns `(frontmatter_toml, trimmed_body)`. Line endings are normalised
/// to `\n`.
fn split_frontmatter(raw: &str) -> Result<(String, String), String> {
    let mut lines = raw.lines();
    match lines.next() {
        Some(first) if first.trim() == "+++" => {}
        _ => return Err("entry must begin with a `+++` TOML frontmatter fence".to_string()),
    }
    let mut frontmatter = String::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim() == "+++" {
            closed = true;
            break;
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }
    if !closed {
        return Err("unterminated `+++` frontmatter fence".to_string());
    }
    let mut body = String::new();
    for line in lines {
        body.push_str(line);
        body.push('\n');
    }
    Ok((frontmatter, body.trim().to_string()))
}

/// Read a frontmatter array of strings, ignoring non-string members.
/// Missing key yields an empty vector.
fn str_array(doc: &toml_edit::DocumentMut, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Validate an ISO `YYYY-MM-DD` date shape (the order key must sort
/// lexically). Shape-only — not a calendar check.
fn validate_iso_date(d: &str) -> Result<(), String> {
    let b = d.as_bytes();
    let ok = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && d.char_indices().all(|(i, c)| match i {
            4 | 7 => c == '-',
            _ => c.is_ascii_digit(),
        });
    if ok {
        Ok(())
    } else {
        Err(format!("date {d:?} is not ISO YYYY-MM-DD"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_raw(id: &str) -> String {
        format!(
            "+++\n\
             id = \"{id}\"\n\
             type = \"rejected\"\n\
             subjects = [\"auth\", \"sessions\"]\n\
             supersedes = \"\"\n\
             foundational = false\n\
             sources = [\"pr#41\"]\n\
             author = \"ai:claude-code\"\n\
             ratified_by = \"amir@team\"\n\
             date = \"2026-06-02\"\n\
             correlation_id = \"0190abcd\"\n\
             +++\n\
             \n\
             We considered LISTEN/NOTIFY and rejected it because polling was simpler.\n"
        )
    }

    #[test]
    fn parses_a_valid_entry() {
        let e = Entry::parse("entry-1", &valid_raw("entry-1")).expect("parses");
        assert_eq!(e.id, "entry-1");
        assert_eq!(e.entry_type, EntryType::Rejected);
        assert_eq!(e.subjects, vec!["auth", "sessions"]);
        assert_eq!(e.supersedes, None);
        assert!(!e.foundational);
        assert_eq!(e.sources, vec!["pr#41"]);
        assert_eq!(e.date, "2026-06-02");
        assert!(e.body.starts_with("We considered"));
    }

    #[test]
    fn empty_supersedes_is_none() {
        let e = Entry::parse("x", &valid_raw("x")).expect("parses");
        assert_eq!(e.supersedes, None);
    }

    #[test]
    fn nonempty_supersedes_is_kept() {
        let raw = valid_raw("x").replace("supersedes = \"\"", "supersedes = \"older-1\"");
        let e = Entry::parse("x", &raw).expect("parses");
        assert_eq!(e.supersedes.as_deref(), Some("older-1"));
    }

    #[test]
    fn filename_id_mismatch_is_rejected() {
        let err = Entry::parse("WRONG", &valid_raw("entry-1")).unwrap_err();
        assert!(err.contains("does not match filename"), "{err}");
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let raw = valid_raw("x").replace("author = \"ai:claude-code\"\n", "");
        let err = Entry::parse("x", &raw).unwrap_err();
        assert!(err.contains("missing required `author`"), "{err}");
    }

    #[test]
    fn unknown_type_is_rejected() {
        let raw = valid_raw("x").replace("type = \"rejected\"", "type = \"musing\"");
        let err = Entry::parse("x", &raw).unwrap_err();
        assert!(err.contains("unknown entry type"), "{err}");
    }

    #[test]
    fn bad_date_is_rejected() {
        let raw = valid_raw("x").replace("date = \"2026-06-02\"", "date = \"June 2\"");
        let err = Entry::parse("x", &raw).unwrap_err();
        assert!(err.contains("ISO YYYY-MM-DD"), "{err}");
    }

    #[test]
    fn empty_body_is_rejected() {
        let raw = valid_raw("x");
        let head = &raw[..raw.rfind("+++").unwrap() + 3];
        let err = Entry::parse("x", &format!("{head}\n\n")).unwrap_err();
        assert!(err.contains("body"), "{err}");
    }

    #[test]
    fn missing_opening_fence_is_rejected() {
        let err = Entry::parse("x", "id = \"x\"\n").unwrap_err();
        assert!(err.contains("must begin with a `+++`"), "{err}");
    }

    #[test]
    fn unterminated_fence_is_rejected() {
        let err = Entry::parse("x", "+++\nid = \"x\"\n").unwrap_err();
        assert!(err.contains("unterminated"), "{err}");
    }

    #[test]
    fn short_handle_is_suffix() {
        assert_eq!(short("01J9ZABCDEFGH"), "ABCDEFGH");
        assert_eq!(short("tiny"), "tiny");
    }

    #[test]
    fn serialize_round_trips_through_parse() {
        let original = Entry::parse("entry-1", &valid_raw("entry-1")).expect("parse");
        let serialized = original.to_file_string();
        let reparsed = Entry::parse("entry-1", &serialized).expect("reparse");
        assert_eq!(reparsed.id, original.id);
        assert_eq!(reparsed.entry_type, original.entry_type);
        assert_eq!(reparsed.subjects, original.subjects);
        assert_eq!(reparsed.supersedes, original.supersedes);
        assert_eq!(reparsed.foundational, original.foundational);
        assert_eq!(reparsed.sources, original.sources);
        assert_eq!(reparsed.author, original.author);
        assert_eq!(reparsed.ratified_by, original.ratified_by);
        assert_eq!(reparsed.date, original.date);
        assert_eq!(reparsed.correlation_id, original.correlation_id);
        assert_eq!(reparsed.body, original.body);
    }

    #[test]
    fn serialized_supersedes_round_trips_as_none() {
        let e = Entry::parse("x", &valid_raw("x")).expect("parse");
        // valid_raw has supersedes = "" → None; round-trip preserves None.
        let reparsed = Entry::parse("x", &e.to_file_string()).expect("reparse");
        assert_eq!(reparsed.supersedes, None);
    }
}
