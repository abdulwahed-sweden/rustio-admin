//! Event replay (Doctrine B2, §4.2.2).
//!
//! `DESIGN_BUILDER.md` §4.2.2: *"Replaying every event in
//! `history.jsonl` in line order against an empty `draft.toml`,
//! using `cli::toml::emit_canonical`, produces the current
//! `draft.toml` byte-for-byte under the environment fixings of
//! §4.4."*
//!
//! This module reads `history.jsonl`, applies each event to an
//! empty [`Draft`] in order, and returns the reconstructed draft.
//! The on-disk `draft.toml`, when canonical-serialized, must match
//! byte-for-byte. The drift test below exercises that property
//! against a fixture corpus.
//!
//! The replay function never writes — it is the inverse of the
//! single-writer `append` boundary. Doctrine B3 binds `append`;
//! this module is the read side.

use std::path::Path;

use serde_json::Value as JsonValue;

use crate::builder::draft::{Draft, Field, Model, Project, DRAFT_SCHEMA_V, FIELD_TYPES};

#[derive(Debug)]
pub(crate) enum ReplayError {
    Io(std::io::Error),
    InvalidJson {
        line_number: usize,
        source: serde_json::Error,
    },
    UnknownOp {
        line_number: usize,
        op: String,
    },
    MalformedArgs {
        line_number: usize,
        reason: &'static str,
    },
    UnknownFieldType {
        line_number: usize,
        ty: String,
    },
    AddFieldOnUnknownModel {
        line_number: usize,
        model: String,
    },
    DuplicateModel {
        line_number: usize,
        name: String,
    },
    DuplicateField {
        line_number: usize,
        model: String,
        name: String,
    },
    UnsupportedSchemaVersion {
        line_number: usize,
        schema_v: u32,
    },
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::Io(e) => write!(f, "I/O error reading history.jsonl: {e}"),
            ReplayError::InvalidJson {
                line_number,
                source,
            } => {
                write!(
                    f,
                    "history.jsonl line {line_number}: invalid JSON: {source}"
                )
            }
            ReplayError::UnknownOp { line_number, op } => {
                write!(f, "history.jsonl line {line_number}: unknown op '{op}'")
            }
            ReplayError::MalformedArgs {
                line_number,
                reason,
            } => {
                write!(
                    f,
                    "history.jsonl line {line_number}: malformed args ({reason})"
                )
            }
            ReplayError::UnknownFieldType { line_number, ty } => {
                write!(
                    f,
                    "history.jsonl line {line_number}: field type '{ty}' not in {FIELD_TYPES:?}"
                )
            }
            ReplayError::AddFieldOnUnknownModel { line_number, model } => write!(
                f,
                "history.jsonl line {line_number}: add_field references unknown model '{model}'"
            ),
            ReplayError::DuplicateModel { line_number, name } => write!(
                f,
                "history.jsonl line {line_number}: duplicate add_model for '{name}'"
            ),
            ReplayError::DuplicateField {
                line_number,
                model,
                name,
            } => write!(
                f,
                "history.jsonl line {line_number}: duplicate add_field {model}.{name}"
            ),
            ReplayError::UnsupportedSchemaVersion {
                line_number,
                schema_v,
            } => write!(
                f,
                "history.jsonl line {line_number}: schema_v {schema_v} not supported"
            ),
        }
    }
}

impl std::error::Error for ReplayError {}

impl From<std::io::Error> for ReplayError {
    fn from(e: std::io::Error) -> Self {
        ReplayError::Io(e)
    }
}

/// Replay every event in `history_path` against an empty draft.
/// Empty files produce an empty draft.
pub(crate) fn replay_from_file(history_path: &Path) -> Result<Draft, ReplayError> {
    if !history_path.exists() {
        return Ok(Draft::empty());
    }
    let content = std::fs::read_to_string(history_path)?;
    replay_str(&content)
}

/// Replay events from a string. Each non-empty line is one JSON
/// event; blank lines are skipped (canonical files don't carry
/// them, but a defensive parser tolerates them).
pub(crate) fn replay_str(content: &str) -> Result<Draft, ReplayError> {
    let mut draft = Draft::empty();
    for (idx, raw) in content.lines().enumerate() {
        let line_number = idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: JsonValue =
            serde_json::from_str(line).map_err(|e| ReplayError::InvalidJson {
                line_number,
                source: e,
            })?;
        apply_event(&mut draft, &parsed, line_number)?;
    }
    Ok(draft)
}

fn apply_event(draft: &mut Draft, ev: &JsonValue, line_number: usize) -> Result<(), ReplayError> {
    // §4.2.1 line shape: every event carries a schema_v.
    let schema_v =
        ev.get("schema_v")
            .and_then(JsonValue::as_u64)
            .ok_or(ReplayError::MalformedArgs {
                line_number,
                reason: "missing schema_v",
            })?;
    if schema_v as u32 != DRAFT_SCHEMA_V {
        return Err(ReplayError::UnsupportedSchemaVersion {
            line_number,
            schema_v: schema_v as u32,
        });
    }

    let op = ev
        .get("op")
        .and_then(JsonValue::as_str)
        .ok_or(ReplayError::MalformedArgs {
            line_number,
            reason: "missing op",
        })?;
    let args = ev.get("args").ok_or(ReplayError::MalformedArgs {
        line_number,
        reason: "missing args",
    })?;

    match op {
        "project_init" => apply_project_init(draft, args, line_number),
        "add_model" => apply_add_model(draft, args, line_number),
        "add_field" => apply_add_field(draft, args, line_number),
        // `commit`, `file_created`, and `undo` are no-ops for the
        // replay→draft projection. `commit` and `file_created`
        // record what got written but do not change the
        // declarative intent; `undo` is out of MVP scope. The
        // replay test fixture must reflect this.
        "commit" | "file_created" => Ok(()),
        "undo" => Ok(()),
        other => Err(ReplayError::UnknownOp {
            line_number,
            op: other.to_string(),
        }),
    }
}

fn apply_project_init(
    draft: &mut Draft,
    args: &JsonValue,
    line_number: usize,
) -> Result<(), ReplayError> {
    let s = |k: &'static str| -> Result<String, ReplayError> {
        Ok(args
            .get(k)
            .and_then(JsonValue::as_str)
            .ok_or(ReplayError::MalformedArgs {
                line_number,
                reason: "project_init missing string field",
            })?
            .to_string())
    };
    draft.project = Project {
        name: s("name")?,
        rust_version: s("rust_version")?,
        builder_pinned: s("builder_pinned")?,
        created_at: s("created_at")?,
    };
    Ok(())
}

fn apply_add_model(
    draft: &mut Draft,
    args: &JsonValue,
    line_number: usize,
) -> Result<(), ReplayError> {
    let name = args
        .get("name")
        .and_then(JsonValue::as_str)
        .ok_or(ReplayError::MalformedArgs {
            line_number,
            reason: "add_model missing name",
        })?
        .to_string();
    let table = args
        .get("table")
        .and_then(JsonValue::as_str)
        .ok_or(ReplayError::MalformedArgs {
            line_number,
            reason: "add_model missing table",
        })?
        .to_string();
    if draft.models.iter().any(|m| m.name == name) {
        return Err(ReplayError::DuplicateModel { line_number, name });
    }
    // Symmetric with Draft::from_toml: refuse two models pointing
    // at the same SQL table.
    if draft.models.iter().any(|m| m.table == table) {
        return Err(ReplayError::DuplicateModel {
            line_number,
            name: format!("table `{table}` (declared by model `{name}`)"),
        });
    }
    draft.models.push(Model {
        name,
        table,
        fields: Vec::new(),
    });
    Ok(())
}

fn apply_add_field(
    draft: &mut Draft,
    args: &JsonValue,
    line_number: usize,
) -> Result<(), ReplayError> {
    let model_name = args
        .get("model")
        .and_then(JsonValue::as_str)
        .ok_or(ReplayError::MalformedArgs {
            line_number,
            reason: "add_field missing model",
        })?
        .to_string();
    let name = args
        .get("name")
        .and_then(JsonValue::as_str)
        .ok_or(ReplayError::MalformedArgs {
            line_number,
            reason: "add_field missing name",
        })?
        .to_string();
    let ty = args
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(ReplayError::MalformedArgs {
            line_number,
            reason: "add_field missing type",
        })?
        .to_string();
    if !FIELD_TYPES.contains(&ty.as_str()) {
        return Err(ReplayError::UnknownFieldType { line_number, ty });
    }
    let required =
        args.get("required")
            .and_then(JsonValue::as_bool)
            .ok_or(ReplayError::MalformedArgs {
                line_number,
                reason: "add_field missing required",
            })?;
    let unique =
        args.get("unique")
            .and_then(JsonValue::as_bool)
            .ok_or(ReplayError::MalformedArgs {
                line_number,
                reason: "add_field missing unique",
            })?;

    let model = draft
        .models
        .iter_mut()
        .find(|m| m.name == model_name)
        .ok_or_else(|| ReplayError::AddFieldOnUnknownModel {
            line_number,
            model: model_name.clone(),
        })?;
    if model.fields.iter().any(|f| f.name == name) {
        return Err(ReplayError::DuplicateField {
            line_number,
            model: model_name,
            name,
        });
    }
    model.fields.push(Field {
        name,
        r#type: ty,
        required,
        unique,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::history::{append, HistoryOp};
    use crate::builder::ulid_gen::new_ulid;
    use serde_json::json;

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "rustio-replay-test-{}-{}",
            std::process::id(),
            new_ulid()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    /// The load-bearing replay invariant (§10.8): writing a
    /// sequence of events via `append` and then replaying the
    /// resulting `history.jsonl` reconstructs a draft whose
    /// canonical TOML emission is byte-identical to the draft
    /// the events logically describe.
    #[test]
    fn replay_rebuilds_draft() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");

        append(
            &path,
            HistoryOp::ProjectInit,
            "alice@example.com",
            json!({
                "name": "demo",
                "rust_version": "1.88",
                "builder_pinned": env!("CARGO_PKG_VERSION"),
                "created_at": "2026-05-15T10:30:00Z",
            }),
        )
        .unwrap();
        append(
            &path,
            HistoryOp::AddModel,
            "alice@example.com",
            json!({"name": "Patient", "table": "patients"}),
        )
        .unwrap();
        append(
            &path,
            HistoryOp::AddField,
            "alice@example.com",
            json!({
                "model": "Patient",
                "name": "full_name",
                "type": "text",
                "required": true,
                "unique": false,
            }),
        )
        .unwrap();
        append(
            &path,
            HistoryOp::Commit,
            "alice@example.com",
            json!({"files": 3}),
        )
        .unwrap();

        let replayed = replay_from_file(&path).unwrap();

        // Construct the expected draft directly and verify
        // byte-equality on canonical TOML.
        let expected = Draft {
            schema_version: 1,
            project: Project {
                name: "demo".into(),
                rust_version: "1.88".into(),
                builder_pinned: env!("CARGO_PKG_VERSION").into(),
                created_at: "2026-05-15T10:30:00Z".into(),
            },
            models: vec![Model {
                name: "Patient".into(),
                table: "patients".into(),
                fields: vec![Field {
                    name: "full_name".into(),
                    r#type: "text".into(),
                    required: true,
                    unique: false,
                }],
            }],
        };
        assert_eq!(
            replayed.to_toml(),
            expected.to_toml(),
            "replay must produce byte-identical canonical TOML"
        );
    }

    #[test]
    fn empty_history_produces_empty_draft() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        std::fs::write(&path, "").unwrap();
        let d = replay_from_file(&path).unwrap();
        assert_eq!(d.models.len(), 0);
        assert_eq!(d.schema_version, 1);
    }

    #[test]
    fn missing_history_file_produces_empty_draft() {
        let dir = tempdir();
        let d = replay_from_file(&dir.join("history.jsonl")).unwrap();
        assert!(d.models.is_empty());
    }

    #[test]
    fn duplicate_add_model_refused() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        append(
            &path,
            HistoryOp::ProjectInit,
            "a",
            json!({
                "name": "x",
                "rust_version": "1.88",
                "builder_pinned": "0.15.0",
                "created_at": "2026-05-15T10:30:00Z",
            }),
        )
        .unwrap();
        append(
            &path,
            HistoryOp::AddModel,
            "a",
            json!({"name": "X", "table": "xs"}),
        )
        .unwrap();
        append(
            &path,
            HistoryOp::AddModel,
            "a",
            json!({"name": "X", "table": "xs_2"}),
        )
        .unwrap();
        let err = replay_from_file(&path).expect_err("duplicate must refuse");
        assert!(matches!(err, ReplayError::DuplicateModel { .. }));
    }

    #[test]
    fn add_field_on_unknown_model_refused() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        append(
            &path,
            HistoryOp::ProjectInit,
            "a",
            json!({
                "name": "x",
                "rust_version": "1.88",
                "builder_pinned": "0.15.0",
                "created_at": "2026-05-15T10:30:00Z",
            }),
        )
        .unwrap();
        append(
            &path,
            HistoryOp::AddField,
            "a",
            json!({
                "model": "Ghost",
                "name": "n",
                "type": "text",
                "required": false,
                "unique": false,
            }),
        )
        .unwrap();
        let err = replay_from_file(&path).expect_err("must refuse");
        assert!(matches!(err, ReplayError::AddFieldOnUnknownModel { .. }));
    }

    #[test]
    fn unknown_op_refused() {
        let dir = tempdir();
        let path = dir.join("history.jsonl");
        std::fs::write(
            &path,
            r#"{"id":"01H","ts":"2026-05-15T10:30:00Z","op":"do_evil","actor":"a","args":{},"schema_v":1}"#,
        )
        .unwrap();
        let err = replay_from_file(&path).expect_err("must refuse unknown op");
        assert!(matches!(err, ReplayError::UnknownOp { .. }));
    }
}
