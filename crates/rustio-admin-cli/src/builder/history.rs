//! The Builder's append-only event log (Doctrine B2, B3).
//!
//! `DESIGN_BUILDER.md` §4.2 declares the contract this module
//! implements:
//!
//! - **Doctrine B3.** [`append`] is the sole function that appends
//!   to `.rustio/history.jsonl`. §10.1 carries the grep proof: no
//!   other file in `crates/rustio-admin-cli/src/` may construct a
//!   write handle to `history.jsonl`.
//! - **Doctrine B2.** The log is append-only. Reversal is always a
//!   new compensating event; never an in-place edit or truncate.
//! - **§4.2.1.** Line shape: one JSON object per line, with fields
//!   in fixed order -- `id`, `ts`, `op`, `actor`, `args`, optional
//!   `prev_hash`, `schema_v`.
//! - **§4.2.4.** The closed `op` enum is the source of truth for
//!   the serialized values. The drift test below asserts the
//!   `HistoryOp` ↔ on-disk string lockstep.
//!
//! The append boundary lives here. Redaction of secret-bearing
//! payloads happens in [`crate::builder::redact`] before reaching
//! this module -- `append` itself accepts an already-clean payload.
//!
//! ## Atomicity model
//!
//! POSIX guarantees that a write opened with `O_APPEND` and sized
//! at or below `PIPE_BUF` is atomic -- concurrent writers do not
//! interleave bytes within a single write call. `OpenOptions::new()
//! .append(true)` maps to `O_APPEND` on Unix. `PIPE_BUF` is at
//! least 512 bytes (POSIX minimum) and is 4096 on every Linux /
//! macOS we target. The serialized event lines this module emits
//! are well below 1 KB in MVP usage (no large `args` payloads;
//! actor strings bounded; redaction shortens any secret-bearing
//! field to a 35-character fingerprint).
//!
//! On Windows the equivalent guarantee is not load-bearing in the
//! current MVP; the doctrine review tracks this as a deferred
//! concern. Single-process usage is unaffected.
//!
//! A defensive size assertion in [`append`] rejects any line over
//! 4 KB so a future field that quietly grew the payload could not
//! cross the atomicity bound without surfacing.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::builder::canonical::canonicalize;
use crate::builder::ulid_gen::new_ulid;

/// The closed enumeration of operations that may appear in
/// `history.jsonl`. New variants here are doctrine amendments;
/// the `op_enum_matches_serialized` drift test below pins each
/// variant to its serialized string.
///
/// `DESIGN_BUILDER.md` §4.2.4: *"The closed enumeration of `op`
/// values lives in a single Rust `enum HistoryOp` whose `as_str()`
/// is the source of truth for the serialized values."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HistoryOp {
    /// Emitted once by `rustio new` to mark project birth.
    ProjectInit,
    /// `rustio add model <Name>`.
    AddModel,
    /// `rustio add field <Model> <name> <type>`.
    AddField,
    /// Emitted by every successful `rustio commit` invocation.
    Commit,
    /// Emitted as a child of a `Commit` event for each generated
    /// file written.
    FileCreated,
    /// Reserved for future use; currently never emitted by the MVP.
    /// Declared up front so the drift test exercises every variant
    /// the doctrine references.
    Undo,
}

impl HistoryOp {
    /// Stable on-disk string representation. Changes here are
    /// breaking and require a `schema_version` bump (§4.6).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            HistoryOp::ProjectInit => "project_init",
            HistoryOp::AddModel => "add_model",
            HistoryOp::AddField => "add_field",
            HistoryOp::Commit => "commit",
            HistoryOp::FileCreated => "file_created",
            HistoryOp::Undo => "undo",
        }
    }
}

/// The wire shape of a single line in `history.jsonl`.
///
/// Field order is fixed by §4.2.1. The `Serialize` impl on this
/// struct preserves that order; serde renders struct fields in
/// declaration order.
#[derive(Debug, Serialize)]
struct EventLine<'a> {
    id: &'a str,
    ts: &'a str,
    op: &'a str,
    actor: &'a str,
    args: &'a JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    prev_hash: Option<&'a str>,
    schema_v: u32,
}

/// The current `history.jsonl` line schema version. Bumps require
/// a `schema_upgraded` event in the log itself and a transformer
/// under `cli::schema_migration`.
const HISTORY_SCHEMA_V: u32 = 1;

/// Append a single event to `.rustio/history.jsonl`.
///
/// **Doctrine B3 -- sole writer.** This is the only function in the
/// CLI crate authorised to open `history.jsonl` for writing. The
/// grep proof at §10.1 enforces that no other file under
/// `crates/rustio-admin-cli/src/` calls `OpenOptions::append` /
/// `File::create` against a path ending in `history.jsonl`.
///
/// Caller responsibilities:
///
/// - `args` must already have been passed through
///   [`crate::builder::redact`] if it can carry secret content.
///   This function does not redact; it serializes what it is given.
/// - `actor` should be `git config user.email`, falling back to
///   `$RUSTIO_AGENT_ID`, falling back to `unknown`. Callers resolve
///   this; the append boundary does not.
///
/// The function is **append-only**. It opens the file with
/// `O_APPEND` semantics and writes one canonical line. No
/// truncation, no in-place edit.
pub(crate) fn append(
    history_path: &Path,
    op: HistoryOp,
    actor: &str,
    args: JsonValue,
) -> std::io::Result<String> {
    let id = new_ulid();
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let line = EventLine {
        id: &id,
        ts: &ts,
        op: op.as_str(),
        actor,
        args: &args,
        prev_hash: None, // chain mode is opt-in; off by default per §4.2.5
        schema_v: HISTORY_SCHEMA_V,
    };
    let mut json = serde_json::to_string(&line).map_err(std::io::Error::other)?;
    json.push('\n');
    let canonical = canonicalize(&json);

    // Atomicity bound: POSIX `O_APPEND` writes are atomic up to
    // `PIPE_BUF`, which is ≥ 512 bytes. We allow 4 KB to give
    // headroom on Linux/macOS (`PIPE_BUF` = 4096) while still
    // surfacing accidental payload growth. Lines beyond this risk
    // interleaving under concurrent writers.
    const MAX_ATOMIC_LINE_BYTES: usize = 4096;
    if canonical.len() > MAX_ATOMIC_LINE_BYTES {
        return Err(std::io::Error::other(format!(
            "history.jsonl event line is {} bytes -- exceeds the {}-byte atomic-write \
             bound from `DESIGN_BUILDER.md` §4.2.1. Refusing to write a payload that \
             could interleave under concurrent CLI invocations.",
            canonical.len(),
            MAX_ATOMIC_LINE_BYTES
        )));
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)?;
    file.write_all(canonical.as_bytes())?;
    file.flush()?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Drift test (§10.8). The closed enum and its serialized
    /// strings must stay in lockstep. A PR that adds a variant
    /// without updating the test list -- or vice versa -- fails here.
    #[test]
    fn op_enum_matches_serialized() {
        let pairs = [
            (HistoryOp::ProjectInit, "project_init"),
            (HistoryOp::AddModel, "add_model"),
            (HistoryOp::AddField, "add_field"),
            (HistoryOp::Commit, "commit"),
            (HistoryOp::FileCreated, "file_created"),
            (HistoryOp::Undo, "undo"),
        ];
        for (variant, expected) in pairs {
            assert_eq!(
                variant.as_str(),
                expected,
                "HistoryOp::{variant:?} must serialize as {expected:?}"
            );
        }
        // Reverse direction: every serialized string must be
        // snake_case ASCII, symmetric to AuditEvent's discipline.
        for (variant, expected) in pairs {
            for c in expected.chars() {
                assert!(
                    c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit(),
                    "HistoryOp::{variant:?} serialized as {expected:?} \
                     contains non-snake_case char {c:?}"
                );
            }
        }
    }

    #[test]
    fn append_writes_one_line_per_call() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        append(
            &path,
            HistoryOp::ProjectInit,
            "alice@example.com",
            json!({}),
        )
        .unwrap();
        append(
            &path,
            HistoryOp::AddModel,
            "alice@example.com",
            json!({"name": "Patient"}),
        )
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.split('\n').filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2, "expected 2 lines, got: {content:?}");
    }

    #[test]
    fn append_emits_required_fields_in_order() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        let id = append(
            &path,
            HistoryOp::AddModel,
            "alice@example.com",
            json!({"name": "Patient"}),
        )
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let line = content.lines().next().unwrap();

        // Parse as JSON and check the field set.
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(parsed["id"], id);
        assert_eq!(parsed["op"], "add_model");
        assert_eq!(parsed["actor"], "alice@example.com");
        assert_eq!(parsed["args"]["name"], "Patient");
        assert_eq!(parsed["schema_v"], 1);
        assert!(parsed["ts"].is_string());

        // Per §4.2.1, field order is fixed in the on-disk byte
        // stream. `serde_json::Value` re-sorts keys alphabetically
        // when iterated, so the order check has to read the raw
        // line. Find each key's first occurrence and assert the
        // sequence.
        let raw = line; // the single line we read from disk
        let expected_order = [
            "\"id\":",
            "\"ts\":",
            "\"op\":",
            "\"actor\":",
            "\"args\":",
            "\"schema_v\":",
        ];
        let mut prev_pos = 0usize;
        for marker in expected_order {
            let pos = raw[prev_pos..]
                .find(marker)
                .unwrap_or_else(|| panic!("missing or out-of-order {marker} in {raw}"));
            prev_pos += pos + marker.len();
        }
    }

    #[test]
    fn append_is_append_only() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        append(&path, HistoryOp::ProjectInit, "a", json!({})).unwrap();
        let after_first = std::fs::read_to_string(&path).unwrap();
        append(&path, HistoryOp::AddModel, "a", json!({"name": "X"})).unwrap();
        let after_second = std::fs::read_to_string(&path).unwrap();
        // Doctrine B2: the second append must extend, never replace.
        assert!(
            after_second.starts_with(&after_first),
            "second append truncated prior content:\n\
             after_first  = {after_first:?}\n\
             after_second = {after_second:?}"
        );
        assert!(after_second.len() > after_first.len());
    }

    #[test]
    fn append_uses_second_precision_timestamps() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        append(&path, HistoryOp::ProjectInit, "a", json!({})).unwrap();
        let line = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let ts = parsed["ts"].as_str().unwrap();
        // §4.4 #5: second-precision ISO-8601 with UTC Z suffix. No
        // sub-second component.
        assert!(ts.ends_with('Z'), "timestamp must end with Z: {ts}");
        assert!(
            !ts.contains('.'),
            "timestamp must not have sub-seconds: {ts}"
        );
    }

    /// Sole-writer self-check (§10.1). Reads this crate's own
    /// source tree and asserts no file outside `history.rs` opens
    /// a write handle to `history.jsonl`. Belt-and-suspenders
    /// alongside the CI grep proof.
    #[test]
    fn no_other_writer_to_history_jsonl_in_cli_crate() {
        let cli_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        scan_for_history_writes(&cli_src, &mut offenders);
        assert!(
            offenders.is_empty(),
            "Doctrine B3 violation: files other than \
             builder/history.rs construct write handles to \
             history.jsonl: {offenders:?}",
        );
    }

    fn scan_for_history_writes(dir: &std::path::Path, offenders: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_for_history_writes(&path, offenders);
                continue;
            }
            // The file that legitimately defines `append` is exempt.
            if path.ends_with("builder/history.rs") {
                continue;
            }
            // Only Rust sources count.
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            // Strip line comments before scanning so doc comments
            // referencing `history.jsonl` do not register as
            // violations. This is not a complete comment parser, but
            // it is enough for the doctrinal smell test.
            for line in src.lines() {
                let code = match line.find("//") {
                    Some(idx) => &line[..idx],
                    None => line,
                };
                let opens_write = (code.contains("OpenOptions")
                    && (code.contains("append(true)") || code.contains(".write(true)")))
                    || code.contains("File::create");
                if opens_write && code.contains("history.jsonl") {
                    offenders.push(path.clone());
                    break;
                }
            }
        }
    }

    #[test]
    fn append_refuses_oversized_lines() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        // Inflate the args payload past the 4 KB atomic bound.
        let big = "x".repeat(5000);
        let err = append(
            &path,
            HistoryOp::AddModel,
            "a",
            json!({ "name": "X", "table": big }),
        )
        .expect_err("must refuse oversized line");
        assert!(err.to_string().contains("atomic-write"), "{err}");
        // And no partial line written.
        assert!(
            !path.exists() || std::fs::read_to_string(&path).unwrap().is_empty(),
            "oversized refusal must not leave a partial line on disk"
        );
    }

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "rustio-history-test-{}-{}",
            std::process::id(),
            new_ulid()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }
}
