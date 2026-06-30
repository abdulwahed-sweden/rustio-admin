//! The `schema.json` contract shared with `rustio-admin import`.
//!
//! This module owns three things, all derived from the same closed vocabulary:
//! the Rust types the document deserializes into, the JSON Schema we hand to the
//! Claude API to *constrain* its output, and the validators that re-check the
//! model's output before we write a file. The validators mirror the builder's
//! own rules so a document rustio-draft writes will pass `rustio-admin import`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Closed list of field types the builder's MVP supports. Hand-mirrors
/// `FIELD_TYPES` in `crates/rustio-admin-cli/src/builder/draft.rs` — kept here
/// (rather than imported) so this crate stays a standalone workspace with no
/// compile-time dependency on the framework. A CI guard ("rustio-draft
/// FIELD_TYPES tracks the builder") fails the build if the two lists drift, so
/// the structured-output enum the model receives can never accept a type
/// `import` rejects. The test below additionally pins the JSON-Schema enum to
/// this const. (F5 in `docs/RUSTIO_DRAFT_SCOPE.md`.)
pub const FIELD_TYPES: &[&str] = &["text", "integer", "boolean", "timestamp"];

/// A schema document: an optional project name and one or more models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub models: Vec<SchemaModel>,
}

/// One model: a CamelCase name and its fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaModel {
    pub name: String,
    #[serde(default)]
    pub fields: Vec<SchemaField>,
}

/// One field: a snake_case name, a type from [`FIELD_TYPES`], and an optional
/// `unique` flag (defaults to `false`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The JSON Schema handed to the Claude API as `output_config.format`. The
/// `type` property is an `enum` of [`FIELD_TYPES`], so the model is *unable* to
/// emit a field type `rustio-admin import` would reject. Uses only the subset of
/// JSON Schema that structured outputs support (object/array/string/enum/bool +
/// `additionalProperties:false`).
pub fn import_json_schema() -> Value {
    let type_enum: Vec<Value> = FIELD_TYPES.iter().map(|t| json!(t)).collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["models"],
        "properties": {
            "project": { "type": "string", "description": "Optional project name (lowercase, no spaces)." },
            "models": {
                "type": "array",
                "description": "The models to generate.",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "fields"],
                    "properties": {
                        "name": { "type": "string", "description": "CamelCase model name, e.g. Invoice. No 'id' or timestamps." },
                        "fields": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["name", "type"],
                                "properties": {
                                    "name": { "type": "string", "description": "snake_case field name, e.g. full_name. Never 'id' or 'created_at'." },
                                    "type": { "type": "string", "enum": type_enum },
                                    "unique": { "type": "boolean", "description": "Whether values must be unique." }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Validate a document the way `rustio-admin import` will, collecting every
/// problem so the user can fix them in one pass. Returns `Ok` if the document
/// would import cleanly, or `Err(messages)` otherwise.
pub fn validate(doc: &SchemaDoc) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if doc.models.is_empty() {
        errors.push("schema has no models".to_string());
    }
    for m in &doc.models {
        if let Err(e) = validate_model_name(&m.name) {
            errors.push(e);
        }
        if m.fields.is_empty() {
            errors.push(format!("model '{}' has no fields", m.name));
        }
        for f in &m.fields {
            if let Err(e) = validate_field_name(&f.name) {
                errors.push(format!("{}: {}", m.name, e));
            }
            if !FIELD_TYPES.contains(&f.ty.as_str()) {
                errors.push(format!(
                    "{}.{}: type '{}' is not in the closed list {:?}",
                    m.name, f.name, f.ty, FIELD_TYPES
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Mirror of the builder's model-name rule: non-empty, CamelCase, ASCII
/// letters/digits only, first char uppercase.
fn validate_model_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("model name must not be empty".into());
    }
    if !name.chars().next().unwrap().is_ascii_uppercase() {
        return Err(format!(
            "model name '{name}' must be CamelCase and start with an uppercase ASCII letter"
        ));
    }
    if let Some(c) = name.chars().find(|c| !c.is_ascii_alphanumeric()) {
        return Err(format!(
            "model name '{name}' contains invalid character {c:?}; only ASCII letters and digits allowed"
        ));
    }
    Ok(())
}

/// Mirror of the builder's field-name rule: non-empty, snake_case `[a-z0-9_]`,
/// first char lowercase, and not a reserved implicit column.
fn validate_field_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("field name must not be empty".into());
    }
    if !name.chars().next().unwrap().is_ascii_lowercase() {
        return Err(format!(
            "field name '{name}' must be snake_case and start with a lowercase ASCII letter"
        ));
    }
    if let Some(c) = name
        .chars()
        .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_'))
    {
        return Err(format!(
            "field name '{name}' contains invalid character {c:?}; only [a-z0-9_] allowed"
        ));
    }
    if matches!(name, "id" | "created_at") {
        return Err(format!(
            "field name '{name}' is reserved; the generator emits it implicitly"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(json: &str) -> SchemaDoc {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn json_schema_enum_matches_field_types() {
        let schema = import_json_schema();
        let enum_vals = &schema["properties"]["models"]["items"]["properties"]["fields"]["items"]
            ["properties"]["type"]["enum"];
        let got: Vec<&str> = enum_vals
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(got, FIELD_TYPES);
    }

    #[test]
    fn valid_document_passes() {
        let d = doc(r#"{ "project": "salon", "models": [
                { "name": "Client", "fields": [
                    { "name": "full_name", "type": "text" },
                    { "name": "joined_at", "type": "timestamp" } ] } ] }"#);
        assert!(validate(&d).is_ok());
    }

    #[test]
    fn collects_every_problem() {
        let d = doc(r#"{ "models": [
                { "name": "lowercase", "fields": [
                    { "name": "id", "type": "text" },
                    { "name": "amount", "type": "money" } ] } ] }"#);
        let errs = validate(&d).unwrap_err();
        // bad model name + reserved field name + unknown type = 3 problems
        assert_eq!(errs.len(), 3, "{errs:?}");
    }

    #[test]
    fn empty_models_is_rejected() {
        let d = doc(r#"{ "models": [] }"#);
        assert!(validate(&d).is_err());
    }

    #[test]
    fn round_trips_and_omits_default_unique() {
        let d = doc(
            r#"{ "models": [ { "name": "X", "fields": [ { "name": "a", "type": "text" } ] } ] }"#,
        );
        let out = serde_json::to_string(&d).unwrap();
        assert!(
            !out.contains("unique"),
            "default unique=false should be omitted: {out}"
        );
        assert!(
            !out.contains("project"),
            "absent project should be omitted: {out}"
        );
    }
}
