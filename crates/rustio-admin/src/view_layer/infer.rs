//! Deterministic default inference: schema metadata in, a [`ViewSpec`] out.
//!
//! Inference is pure and offline — no database, no network, no AI. The same
//! input always yields the same output, which is what makes it testable and
//! safe to use as a *starting point* for the future view designer. The output
//! is just a draft `ViewSpec`; once saved, the spec — not this function — is
//! the source of truth at render time.

use crate::admin::{AdminField, FieldType};

use super::modes::ViewMode;
use super::roles::FieldRole;
use super::spec::{FieldViewSpec, ViewSpec, VIEW_SPEC_VERSION};

// public:
/// Minimal description of a column, as the admin already knows it from the
/// schema. Inference works purely off this.
///
/// Build one from the framework's own field metadata with
/// [`FieldMeta::from_admin_field`]; the raw constructor exists for tests and
/// callers that already have name/kind/nullability in hand.
#[derive(Debug, Clone)]
pub struct FieldMeta {
    /// Column name.
    pub name: String,
    /// Coarse type bucket used by the inference rules.
    pub kind: FieldKind,
    /// Whether the column is nullable.
    pub nullable: bool,
}

impl FieldMeta {
    // public:
    /// Bridge the framework's [`AdminField`] into the inference vocabulary.
    ///
    /// The admin's `FieldType` has no `ForeignKey`/`Enum`/`Json` variants —
    /// those live on `AdminField` as `relation` and `choices` — so they are
    /// derived here. Anything textual (`String`/`Email`/`Phone`/`FilePath`/
    /// their `Optional*` forms) collapses to [`FieldKind::Text`].
    pub fn from_admin_field(field: &AdminField) -> Self {
        let kind = if field.relation.is_some() {
            FieldKind::ForeignKey
        } else if field.choices.is_some() {
            FieldKind::Enum
        } else {
            match field.field_type {
                FieldType::Bool => FieldKind::Bool,
                FieldType::DateTime
                | FieldType::OptionalDateTime
                | FieldType::Date
                | FieldType::Time => FieldKind::DateTime,
                FieldType::Uuid => FieldKind::Uuid,
                FieldType::I32 | FieldType::I64 | FieldType::OptionalI64 => FieldKind::Integer,
                FieldType::F64 | FieldType::Decimal => FieldKind::Float,
                // String, Email, Phone, OptionalString, FilePath,
                // OptionalFilePath, and any future variant -> Text.
                _ => FieldKind::Text,
            }
        };
        FieldMeta {
            name: field.name.to_string(),
            kind,
            nullable: field.field_type.nullable(),
        }
    }
}

// public:
/// Coarse type buckets. The admin maps real column types down to these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Free text.
    Text,
    /// Whole number.
    Integer,
    /// Floating-point / decimal number.
    Float,
    /// Boolean.
    Bool,
    /// A closed set of choices (status, type, …).
    Enum,
    /// A date/time value.
    DateTime,
    /// A foreign-key reference to another model.
    ForeignKey,
    /// A UUID identifier.
    Uuid,
    /// A structured JSON blob.
    Json,
}

/// Substrings that mark a field as sensitive. Matching is case-insensitive and
/// substring-based so `user_password_hash` and `reset_token` both catch.
const SENSITIVE_MARKERS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "private_key",
    "session",
    "hash",
    "pin",
];

fn is_sensitive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_MARKERS.iter().any(|m| lower.contains(m))
}

fn is_timestamp_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "created_at" || lower == "updated_at" || lower == "deleted_at"
}

// public:
/// Produce a reasonable [`ViewSpec`] from a model name and its columns.
///
/// Deterministic: same input always yields the same output. The rules err on
/// the side of hiding rather than exposing, and only the first human-readable
/// text field is promoted to `Primary`.
pub fn infer_view_spec(model: &str, columns: &[FieldMeta]) -> ViewSpec {
    let mut fields = Vec::with_capacity(columns.len());
    let mut primary_taken = false;
    let mut default_filters = Vec::new();

    for (index, col) in columns.iter().enumerate() {
        let mut spec = FieldViewSpec::new(&col.name, FieldRole::Secondary);
        // priority spreads fields out so the designer has room to reorder later
        spec.priority = (index as i32) * 10;

        if is_sensitive(&col.name) {
            spec.role = FieldRole::Hidden;
            fields.push(spec);
            continue;
        }

        match col.kind {
            FieldKind::Uuid => {
                spec.role = FieldRole::Hidden;
            }
            FieldKind::DateTime => {
                // explicit audit timestamps stay out of the main row;
                // other datetimes are quiet but visible
                spec.role = if is_timestamp_name(&col.name) {
                    FieldRole::DetailOnly
                } else {
                    FieldRole::Timestamp
                };
                spec.sortable = true;
            }
            FieldKind::Enum | FieldKind::Bool => {
                spec.role = FieldRole::Badge;
                spec.filterable = true;
                if default_filters.len() < 2 {
                    spec.default_filter = true;
                    default_filters.push(col.name.clone());
                }
            }
            FieldKind::ForeignKey => {
                // FKs shouldn't dominate; keep them secondary and filterable
                spec.role = FieldRole::Secondary;
                spec.filterable = true;
            }
            FieldKind::Json => {
                spec.role = FieldRole::DetailOnly;
            }
            FieldKind::Text => {
                let lower = col.name.to_ascii_lowercase();
                if lower == "id" {
                    spec.role = FieldRole::DetailOnly;
                } else if !primary_taken {
                    spec.role = FieldRole::Primary;
                    spec.sortable = true;
                    primary_taken = true;
                } else if col.nullable {
                    // optional secondary text is rarely worth a column
                    spec.role = FieldRole::DetailOnly;
                } else {
                    spec.role = FieldRole::Secondary;
                }
            }
            FieldKind::Integer | FieldKind::Float => {
                let lower = col.name.to_ascii_lowercase();
                if lower == "id" {
                    spec.role = FieldRole::DetailOnly;
                } else {
                    spec.role = FieldRole::Secondary;
                    spec.sortable = true;
                }
            }
        }

        fields.push(spec);
    }

    ViewSpec {
        model: model.to_string(),
        default_mode: ViewMode::Table,
        allowed_modes: vec![
            ViewMode::Table,
            ViewMode::List,
            ViewMode::Cards,
            ViewMode::Compact,
        ],
        fields,
        compositions: Vec::new(),
        default_filters,
        version: VIEW_SPEC_VERSION,
    }
}

// public:
/// Convenience wrapper: infer directly from the framework's own field
/// metadata (`AdminModel::FIELDS` / `AdminEntry::fields`). Maps each
/// [`AdminField`] through [`FieldMeta::from_admin_field`] then calls
/// [`infer_view_spec`].
pub fn infer_view_spec_from_fields(model: &str, fields: &[AdminField]) -> ViewSpec {
    let columns: Vec<FieldMeta> = fields.iter().map(FieldMeta::from_admin_field).collect();
    infer_view_spec(model, &columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, kind: FieldKind, nullable: bool) -> FieldMeta {
        FieldMeta {
            name: name.into(),
            kind,
            nullable,
        }
    }

    fn role_of(spec: &ViewSpec, name: &str) -> FieldRole {
        spec.fields
            .iter()
            .find(|f| f.field_name == name)
            .unwrap()
            .role
    }

    #[test]
    fn id_is_detail_only() {
        let spec = infer_view_spec("thing", &[col("id", FieldKind::Integer, false)]);
        assert_eq!(role_of(&spec, "id"), FieldRole::DetailOnly);
    }

    #[test]
    fn first_text_becomes_primary() {
        let spec = infer_view_spec(
            "customer",
            &[
                col("id", FieldKind::Integer, false),
                col("full_name", FieldKind::Text, false),
                col("company", FieldKind::Text, false),
            ],
        );
        assert_eq!(role_of(&spec, "full_name"), FieldRole::Primary);
        assert_eq!(role_of(&spec, "company"), FieldRole::Secondary);
    }

    #[test]
    fn enums_become_filterable_badges() {
        let spec = infer_view_spec("order", &[col("status", FieldKind::Enum, false)]);
        let status = spec
            .fields
            .iter()
            .find(|f| f.field_name == "status")
            .unwrap();
        assert_eq!(status.role, FieldRole::Badge);
        assert!(status.filterable);
        assert!(status.default_filter);
    }

    #[test]
    fn audit_timestamps_are_detail_only() {
        let spec = infer_view_spec(
            "thing",
            &[
                col("created_at", FieldKind::DateTime, false),
                col("scheduled_for", FieldKind::DateTime, true),
            ],
        );
        assert_eq!(role_of(&spec, "created_at"), FieldRole::DetailOnly);
        assert_eq!(role_of(&spec, "scheduled_for"), FieldRole::Timestamp);
    }

    #[test]
    fn nullable_secondary_text_is_demoted() {
        let spec = infer_view_spec(
            "customer",
            &[
                col("name", FieldKind::Text, false),
                col("notes", FieldKind::Text, true),
            ],
        );
        assert_eq!(role_of(&spec, "notes"), FieldRole::DetailOnly);
    }

    #[test]
    fn inference_is_deterministic() {
        let cols = [
            col("id", FieldKind::Integer, false),
            col("email", FieldKind::Text, false),
            col("status", FieldKind::Enum, false),
        ];
        let a = infer_view_spec("user", &cols);
        let b = infer_view_spec("user", &cols);
        assert_eq!(a, b);
    }

    // ---- adapter from the framework's own AdminField metadata ----

    #[test]
    fn from_admin_field_maps_relation_and_choices() {
        use crate::admin::AdminRelation;

        let fk = AdminField {
            name: "customer_id",
            label: "Customer",
            field_type: FieldType::I64,
            editable: true,
            relation: Some(AdminRelation {
                target_model: "customer",
                display_field: None,
                multi: false,
            }),
            choices: None,
        };
        assert_eq!(FieldMeta::from_admin_field(&fk).kind, FieldKind::ForeignKey);

        let status = AdminField {
            name: "status",
            label: "Status",
            field_type: FieldType::String,
            editable: true,
            relation: None,
            choices: Some(&["open", "closed"]),
        };
        assert_eq!(FieldMeta::from_admin_field(&status).kind, FieldKind::Enum);

        let created = AdminField {
            name: "created_at",
            label: "Created At",
            field_type: FieldType::DateTime,
            editable: false,
            relation: None,
            choices: None,
        };
        let meta = FieldMeta::from_admin_field(&created);
        assert_eq!(meta.kind, FieldKind::DateTime);
        assert!(!meta.nullable);

        let bio = AdminField {
            name: "bio",
            label: "Bio",
            field_type: FieldType::OptionalString,
            editable: true,
            relation: None,
            choices: None,
        };
        let meta = FieldMeta::from_admin_field(&bio);
        assert_eq!(meta.kind, FieldKind::Text);
        assert!(meta.nullable);
    }

    // ---- worked example specs (the deliverable models) ----

    #[test]
    fn customers_example_spec() {
        let spec = infer_view_spec(
            "customer",
            &[
                col("id", FieldKind::Integer, false),
                col("full_name", FieldKind::Text, false),
                col("email", FieldKind::Text, false),
                col("company", FieldKind::Text, true),
                col("status", FieldKind::Enum, false),
                col("region_id", FieldKind::ForeignKey, true),
                col("password_hash", FieldKind::Text, false),
                col("api_key", FieldKind::Text, true),
                col("created_at", FieldKind::DateTime, false),
                col("updated_at", FieldKind::DateTime, false),
            ],
        );
        assert_eq!(role_of(&spec, "id"), FieldRole::DetailOnly);
        assert_eq!(role_of(&spec, "full_name"), FieldRole::Primary);
        assert_eq!(role_of(&spec, "email"), FieldRole::Secondary);
        assert_eq!(role_of(&spec, "company"), FieldRole::DetailOnly);
        assert_eq!(role_of(&spec, "status"), FieldRole::Badge);
        assert_eq!(role_of(&spec, "region_id"), FieldRole::Secondary);
        assert_eq!(role_of(&spec, "password_hash"), FieldRole::Hidden);
        assert_eq!(role_of(&spec, "api_key"), FieldRole::Hidden);
        assert_eq!(role_of(&spec, "created_at"), FieldRole::DetailOnly);
        assert_eq!(spec.default_filters, vec!["status".to_string()]);
    }

    #[test]
    fn bookings_example_spec() {
        let spec = infer_view_spec(
            "booking",
            &[
                col("id", FieldKind::Uuid, false),
                col("reference", FieldKind::Text, false),
                col("customer_id", FieldKind::ForeignKey, false),
                col("service", FieldKind::Text, false),
                col("state", FieldKind::Enum, false),
                col("is_paid", FieldKind::Bool, false),
                col("scheduled_for", FieldKind::DateTime, false),
                col("notes", FieldKind::Text, true),
                col("created_at", FieldKind::DateTime, false),
            ],
        );
        assert_eq!(role_of(&spec, "id"), FieldRole::Hidden); // uuid
        assert_eq!(role_of(&spec, "reference"), FieldRole::Primary);
        assert_eq!(role_of(&spec, "customer_id"), FieldRole::Secondary);
        assert_eq!(role_of(&spec, "service"), FieldRole::Secondary);
        assert_eq!(role_of(&spec, "state"), FieldRole::Badge);
        assert_eq!(role_of(&spec, "is_paid"), FieldRole::Badge);
        assert_eq!(role_of(&spec, "scheduled_for"), FieldRole::Timestamp);
        assert_eq!(role_of(&spec, "notes"), FieldRole::DetailOnly);
        assert_eq!(
            spec.default_filters,
            vec!["state".to_string(), "is_paid".to_string()]
        );
    }
}
