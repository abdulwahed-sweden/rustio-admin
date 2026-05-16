//! `.rustio/draft.toml` — the declarative intent file.
//!
//! `DESIGN_BUILDER.md` §4.1 makes this file the deterministic
//! generator's sole input (Doctrine B1). This module defines the
//! in-memory schema and the round-trip to / from canonical TOML.
//!
//! ## Type discipline
//!
//! The struct shape mirrors the on-disk shape. Conversion runs
//! through the `toml_edit` DOM and routes every emission through
//! [`crate::builder::toml_canon::emit_canonical`] — the doctrine's
//! sole TOML emitter (§10.4 grep proof). The CLI crate never calls
//! `toml::to_string` or any other emitter directly.
//!
//! ## MVP scope
//!
//! - Models with their own field list.
//! - Field types limited to `text`, `integer`, `boolean`,
//!   `timestamp`. Adding a type is a localised change in
//!   [`FIELD_TYPES`] plus matching codegen in
//!   [`crate::builder::codegen`].
//! - Modifiers limited to `required` and `unique`.
//! - Relations are out of scope.
//! - The `[features]` table is out of MVP scope; doctrine-bound
//!   features (§8.2) are implicitly always-on because the Builder
//!   never emits code that disables them.

use std::collections::BTreeSet;

use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

use crate::builder::toml_canon::{emit_canonical, parse, TomlError};

/// The current `draft.toml` schema version. Bumps require a
/// `schema_upgraded` event in `history.jsonl` and a transformer
/// under `cli::schema_migration` (out of MVP scope).
pub(crate) const DRAFT_SCHEMA_V: u32 = 1;

/// Closed list of field types the MVP supports. Adding a type is
/// a doctrine-aware change: codegen, validation, and tests must
/// agree.
pub(crate) const FIELD_TYPES: &[&str] = &["text", "integer", "boolean", "timestamp"];

/// In-memory representation of `.rustio/draft.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Draft {
    pub schema_version: u32,
    pub project: Project,
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Project {
    pub name: String,
    pub rust_version: String,
    pub builder_pinned: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Model {
    /// CamelCase identifier (e.g. `"Patient"`).
    pub name: String,
    /// snake_case plural (e.g. `"patients"`).
    pub table: String,
    /// Developer-declared fields in insertion (event) order. The
    /// implicit `id` + `created_at` columns are NOT stored here;
    /// codegen adds them at emission time.
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    /// snake_case identifier (e.g. `"full_name"`).
    pub name: String,
    /// Declared type from [`FIELD_TYPES`].
    pub r#type: String,
    pub required: bool,
    pub unique: bool,
}

/// Errors surfaced while parsing or validating a `draft.toml`.
#[derive(Debug)]
pub(crate) enum DraftError {
    /// Underlying TOML parse failure.
    Toml(TomlError),
    /// Schema mismatch — version not understood by this Builder.
    UnsupportedSchemaVersion(u32),
    /// A required key is missing in the on-disk document.
    Missing(&'static str),
    /// A value has the wrong TOML type.
    BadShape(&'static str),
    /// A field declared a type outside [`FIELD_TYPES`].
    UnknownFieldType(String),
    /// Two models or fields share a name.
    DuplicateName(String),
}

impl std::fmt::Display for DraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DraftError::Toml(e) => write!(f, "{e}"),
            DraftError::UnsupportedSchemaVersion(v) => write!(
                f,
                "draft.toml schema_version {v} is not supported by this Builder \
                 (this Builder understands schema_version {DRAFT_SCHEMA_V})"
            ),
            DraftError::Missing(k) => write!(f, "draft.toml is missing required key '{k}'"),
            DraftError::BadShape(k) => write!(f, "draft.toml key '{k}' has the wrong shape"),
            DraftError::UnknownFieldType(t) => write!(
                f,
                "field type '{t}' is not in the MVP type list {FIELD_TYPES:?}"
            ),
            DraftError::DuplicateName(n) => write!(f, "duplicate name '{n}'"),
        }
    }
}

impl std::error::Error for DraftError {}

impl From<TomlError> for DraftError {
    fn from(e: TomlError) -> Self {
        DraftError::Toml(e)
    }
}

impl Draft {
    /// Empty draft seeded with the schema version. Used as the
    /// replay starting state.
    pub(crate) fn empty() -> Self {
        Self {
            schema_version: DRAFT_SCHEMA_V,
            project: Project::default(),
            models: Vec::new(),
        }
    }

    /// Parse a canonical-form TOML document into a [`Draft`].
    /// Rejects unknown schema versions and structurally malformed
    /// documents.
    pub(crate) fn from_toml(input: &str) -> Result<Self, DraftError> {
        let doc = parse(input)?;
        let schema_version = doc
            .get("schema_version")
            .and_then(Item::as_integer)
            .ok_or(DraftError::Missing("schema_version"))? as u32;
        if schema_version != DRAFT_SCHEMA_V {
            return Err(DraftError::UnsupportedSchemaVersion(schema_version));
        }

        let project = parse_project(
            doc.get("project")
                .and_then(Item::as_table)
                .ok_or(DraftError::Missing("project"))?,
        )?;

        let mut models = Vec::new();
        let mut seen_models: BTreeSet<String> = BTreeSet::new();
        let mut seen_tables: BTreeSet<String> = BTreeSet::new();
        if let Some(aot) = doc.get("models").and_then(Item::as_array_of_tables) {
            for entry in aot.iter() {
                let model = parse_model(entry)?;
                if !seen_models.insert(model.name.clone()) {
                    return Err(DraftError::DuplicateName(model.name));
                }
                // Two distinct CamelCase names can plural_snake to
                // the same table (e.g. user types `Boxes` after
                // `Box`). Doctrine §7.4 reasoning: silent table
                // collision is a footgun; refuse at parse time.
                if !seen_tables.insert(model.table.clone()) {
                    return Err(DraftError::DuplicateName(format!(
                        "table `{}` (declared by model `{}`)",
                        model.table, model.name
                    )));
                }
                models.push(model);
            }
        }

        Ok(Self {
            schema_version,
            project,
            models,
        })
    }

    /// Serialize this draft to canonical-form TOML.
    pub(crate) fn to_toml(&self) -> String {
        let mut doc = DocumentMut::new();
        doc["schema_version"] = value(i64::from(self.schema_version));

        let mut project = Table::new();
        project["builder_pinned"] = value(self.project.builder_pinned.clone());
        project["created_at"] = value(self.project.created_at.clone());
        project["name"] = value(self.project.name.clone());
        project["rust_version"] = value(self.project.rust_version.clone());
        doc["project"] = Item::Table(project);

        if !self.models.is_empty() {
            let mut aot = ArrayOfTables::new();
            for model in &self.models {
                aot.push(model_to_table(model));
            }
            doc["models"] = Item::ArrayOfTables(aot);
        }

        emit_canonical(&doc)
    }
}

fn parse_project(t: &Table) -> Result<Project, DraftError> {
    Ok(Project {
        builder_pinned: t
            .get("builder_pinned")
            .and_then(Item::as_str)
            .ok_or(DraftError::Missing("project.builder_pinned"))?
            .to_string(),
        created_at: t
            .get("created_at")
            .and_then(Item::as_str)
            .ok_or(DraftError::Missing("project.created_at"))?
            .to_string(),
        name: t
            .get("name")
            .and_then(Item::as_str)
            .ok_or(DraftError::Missing("project.name"))?
            .to_string(),
        rust_version: t
            .get("rust_version")
            .and_then(Item::as_str)
            .ok_or(DraftError::Missing("project.rust_version"))?
            .to_string(),
    })
}

fn parse_model(t: &Table) -> Result<Model, DraftError> {
    let name = t
        .get("name")
        .and_then(Item::as_str)
        .ok_or(DraftError::Missing("models[].name"))?
        .to_string();
    let table_name = t
        .get("table")
        .and_then(Item::as_str)
        .ok_or(DraftError::Missing("models[].table"))?
        .to_string();
    let mut fields = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    if let Some(aot) = t.get("fields").and_then(Item::as_array_of_tables) {
        for entry in aot.iter() {
            let field = parse_field(entry)?;
            if !seen.insert(field.name.clone()) {
                return Err(DraftError::DuplicateName(format!("{name}.{}", field.name)));
            }
            fields.push(field);
        }
    }
    Ok(Model {
        name,
        table: table_name,
        fields,
    })
}

fn parse_field(t: &Table) -> Result<Field, DraftError> {
    let name = t
        .get("name")
        .and_then(Item::as_str)
        .ok_or(DraftError::Missing("models[].fields[].name"))?
        .to_string();
    let ty = t
        .get("type")
        .and_then(Item::as_str)
        .ok_or(DraftError::Missing("models[].fields[].type"))?
        .to_string();
    if !FIELD_TYPES.contains(&ty.as_str()) {
        return Err(DraftError::UnknownFieldType(ty));
    }
    let required = t
        .get("required")
        .and_then(Item::as_bool)
        .ok_or(DraftError::BadShape("models[].fields[].required"))?;
    let unique = t
        .get("unique")
        .and_then(Item::as_bool)
        .ok_or(DraftError::BadShape("models[].fields[].unique"))?;
    Ok(Field {
        name,
        r#type: ty,
        required,
        unique,
    })
}

fn model_to_table(model: &Model) -> Table {
    let mut t = Table::new();
    t["name"] = value(model.name.clone());
    t["table"] = value(model.table.clone());
    if !model.fields.is_empty() {
        let mut aot = ArrayOfTables::new();
        for f in &model.fields {
            aot.push(field_to_table(f));
        }
        t["fields"] = Item::ArrayOfTables(aot);
    }
    t
}

fn field_to_table(field: &Field) -> Table {
    let mut t = Table::new();
    t["name"] = value(field.name.clone());
    t["required"] = value(field.required);
    t["type"] = value(field.r#type.clone());
    t["unique"] = value(field.unique);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_draft() -> Draft {
        Draft {
            schema_version: DRAFT_SCHEMA_V,
            project: Project {
                name: "demo".into(),
                rust_version: "1.88".into(),
                builder_pinned: "0.15.1".into(),
                created_at: "2026-05-15T10:30:00Z".into(),
            },
            models: vec![
                Model {
                    name: "Patient".into(),
                    table: "patients".into(),
                    fields: vec![Field {
                        name: "full_name".into(),
                        r#type: "text".into(),
                        required: true,
                        unique: false,
                    }],
                },
                Model {
                    name: "Doctor".into(),
                    table: "doctors".into(),
                    fields: vec![],
                },
            ],
        }
    }

    #[test]
    fn round_trip_via_canonical_toml() {
        let original = sample_draft();
        let serialized = original.to_toml();
        let parsed = Draft::from_toml(&serialized).expect("must parse own output");
        assert_eq!(parsed, original);
    }

    #[test]
    fn serialized_output_is_byte_stable() {
        // Calling to_toml() twice from the same draft must produce
        // identical bytes — the reproducibility invariant at §4.4.
        let draft = sample_draft();
        assert_eq!(draft.to_toml(), draft.to_toml());
    }

    #[test]
    fn empty_draft_round_trips() {
        let mut draft = Draft::empty();
        draft.project = Project {
            name: "x".into(),
            rust_version: "1.88".into(),
            builder_pinned: "0.15.1".into(),
            created_at: "2026-05-15T10:30:00Z".into(),
        };
        let s = draft.to_toml();
        let back = Draft::from_toml(&s).unwrap();
        assert_eq!(back, draft);
        assert!(back.models.is_empty());
    }

    #[test]
    fn unknown_schema_version_refused() {
        let mut doc = DocumentMut::new();
        doc["schema_version"] = value(99_i64);
        let mut p = Table::new();
        p["builder_pinned"] = value("0.15.1");
        p["created_at"] = value("2026-05-15T10:30:00Z");
        p["name"] = value("x");
        p["rust_version"] = value("1.88");
        doc["project"] = Item::Table(p);
        let s = emit_canonical(&doc);
        assert!(matches!(
            Draft::from_toml(&s),
            Err(DraftError::UnsupportedSchemaVersion(99))
        ));
    }

    #[test]
    fn unknown_field_type_refused() {
        let mut draft = sample_draft();
        draft.models[0].fields.push(Field {
            name: "weird".into(),
            r#type: "geography".into(),
            required: false,
            unique: false,
        });
        let s = draft.to_toml();
        assert!(
            matches!(Draft::from_toml(&s), Err(DraftError::UnknownFieldType(t)) if t == "geography")
        );
    }

    #[test]
    fn duplicate_model_names_refused() {
        let mut draft = sample_draft();
        draft.models.push(Model {
            name: "Patient".into(),
            table: "patients_2".into(),
            fields: vec![],
        });
        let s = draft.to_toml();
        assert!(
            matches!(Draft::from_toml(&s), Err(DraftError::DuplicateName(n)) if n == "Patient")
        );
    }

    #[test]
    fn duplicate_table_names_refused() {
        let mut draft = sample_draft();
        // Distinct model name, same table — the silent-collision
        // footgun from §7.4.
        draft.models.push(Model {
            name: "Resident".into(),
            table: "patients".into(),
            fields: vec![],
        });
        let s = draft.to_toml();
        match Draft::from_toml(&s) {
            Err(DraftError::DuplicateName(msg)) => {
                assert!(msg.contains("patients"), "{msg}");
                assert!(msg.contains("Resident"), "{msg}");
            }
            other => panic!("expected DuplicateName for collision, got {other:?}"),
        }
    }

    #[test]
    fn keys_sorted_in_output() {
        // Doctrine §4.4 canonical form: keys alphabetical within
        // every table. Spot-check the project section.
        let out = sample_draft().to_toml();
        let project_section = out
            .split("\n[")
            .find(|s| s.starts_with("project]"))
            .or_else(|| out.strip_prefix("[project]"))
            .or_else(|| Some(&out[out.find("[project]").unwrap()..]))
            .unwrap();
        let bp = project_section.find("builder_pinned").unwrap();
        let ca = project_section.find("created_at").unwrap();
        let nm = project_section.find("\nname").unwrap();
        let rv = project_section.find("rust_version").unwrap();
        assert!(bp < ca, "builder_pinned before created_at");
        assert!(ca < nm, "created_at before name");
        assert!(nm < rv, "name before rust_version");
    }
}
