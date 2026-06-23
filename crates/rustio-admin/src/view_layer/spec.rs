//! The saved `ViewSpec` — the single source of truth at render time.
//!
//! Once a `ViewSpec` exists, rendering reads it and nothing else: no
//! inference, no AI, no guessing. Inference ([`super::infer`]) and the future
//! view designer only ever *produce* one of these.

use serde::{Deserialize, Serialize};

use super::compose::CellComposition;
use super::modes::ViewMode;
use super::roles::{FieldRole, SemanticClass};

// public:
/// Current `ViewSpec` schema version. Bumped when the on-disk shape changes so
/// older saved specs can be migrated rather than silently misread.
pub const VIEW_SPEC_VERSION: u32 = 1;

// public:
/// Per-field display configuration. One of these exists for every column the
/// admin knows about, even hidden ones (we still need to know to hide them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldViewSpec {
    /// The schema column this spec governs.
    pub field_name: String,
    /// Human label shown in headers. Falls back to a humanized `field_name`
    /// at the template level when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The field's visual role.
    pub role: FieldRole,
    /// Lower numbers render first. Ties break on declaration order.
    #[serde(default)]
    pub priority: i32,
    /// Whether the column offers a sort control.
    #[serde(default)]
    pub sortable: bool,
    /// Whether the column offers a filter control.
    #[serde(default)]
    pub filterable: bool,
    /// If true and `filterable`, this filter is shown without the user having
    /// to open an "advanced filters" panel.
    #[serde(default)]
    pub default_filter: bool,
    /// Optional width hint for table mode, e.g. `"120px"` or `"20%"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    /// Badge colour intent. Only meaningful when `role == Badge`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_class: Option<SemanticClass>,
}

impl FieldViewSpec {
    // public:
    /// Convenience constructor for the common "just a name and a role" case.
    pub fn new(field_name: impl Into<String>, role: FieldRole) -> Self {
        FieldViewSpec {
            field_name: field_name.into(),
            label: None,
            role,
            priority: 0,
            sortable: false,
            filterable: false,
            default_filter: false,
            width: None,
            semantic_class: None,
        }
    }
}

// public:
/// The full visual contract for one model. This is the single source of truth
/// at render time — no inference, no AI, no guessing happens once a `ViewSpec`
/// exists. Inference and the (future) designer only ever *produce* one of these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewSpec {
    /// The model (admin name) this spec describes.
    pub model: String,
    /// The mode rendered when no `?view=` slug is supplied.
    pub default_mode: ViewMode,
    /// Modes offered in the switcher. Should always contain `default_mode`.
    pub allowed_modes: Vec<ViewMode>,
    /// Per-field display configuration, in declaration order.
    pub fields: Vec<FieldViewSpec>,
    /// Composed cells that merge several fields into one visual unit.
    #[serde(default)]
    pub compositions: Vec<CellComposition>,
    /// Field names whose filters are open by default. Redundant with the
    /// per-field flag but convenient for the designer to reorder.
    #[serde(default)]
    pub default_filters: Vec<String>,
    /// Schema version of this saved spec; see [`VIEW_SPEC_VERSION`].
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    VIEW_SPEC_VERSION
}

impl ViewSpec {
    // public:
    /// Fields that should appear in list/table/card layouts, sorted by
    /// priority then declaration order. `Hidden` and `DetailOnly` are dropped.
    pub fn list_fields(&self) -> Vec<&FieldViewSpec> {
        let mut visible: Vec<&FieldViewSpec> = self
            .fields
            .iter()
            .filter(|f| f.role.shows_in_list())
            .collect();
        // stable sort keeps declaration order for equal priorities
        visible.sort_by_key(|f| f.priority);
        visible
    }

    // public:
    /// The primary field, if one is declared. Used as the row/card title.
    pub fn primary_field(&self) -> Option<&FieldViewSpec> {
        self.fields.iter().find(|f| f.role == FieldRole::Primary)
    }

    // public:
    /// Field names that must never reach a template (sensitive/hidden). The
    /// renderer uses this to strip values before building any context.
    pub fn redacted_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|f| !f.role.reaches_template())
            .map(|f| f.field_name.as_str())
            .collect()
    }

    // public:
    /// Resolve the active mode from an optional query slug, falling back to
    /// the default and rejecting modes not in `allowed_modes`.
    pub fn resolve_mode(&self, requested: Option<&str>) -> ViewMode {
        let wanted = requested.and_then(ViewMode::from_slug);
        match wanted {
            Some(mode) if self.allowed_modes.contains(&mode) => mode,
            _ => self.default_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> ViewSpec {
        ViewSpec {
            model: "customer".into(),
            default_mode: ViewMode::List,
            allowed_modes: vec![ViewMode::List, ViewMode::Table, ViewMode::Cards],
            fields: vec![
                FieldViewSpec::new("full_name", FieldRole::Primary),
                {
                    let mut f = FieldViewSpec::new("status", FieldRole::Badge);
                    f.semantic_class = Some(SemanticClass::Success);
                    f.filterable = true;
                    f.default_filter = true;
                    f
                },
                FieldViewSpec::new("password_hash", FieldRole::Hidden),
            ],
            compositions: vec![],
            default_filters: vec!["status".into()],
            version: VIEW_SPEC_VERSION,
        }
    }

    #[test]
    fn roundtrips_through_json() {
        let spec = sample_spec();
        let json = serde_json::to_string(&spec).unwrap();
        let back: ViewSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, back);
    }

    #[test]
    fn enums_serialize_as_snake_case() {
        let json = serde_json::to_string(&sample_spec()).unwrap();
        assert!(json.contains("\"primary\""));
        assert!(json.contains("\"badge\""));
        assert!(json.contains("\"list\""));
        assert!(json.contains("\"success\""));
    }

    #[test]
    fn version_defaults_when_missing() {
        // an older spec saved without a version field should still load
        let json = r#"{
            "model": "thing",
            "default_mode": "table",
            "allowed_modes": ["table"],
            "fields": []
        }"#;
        let spec: ViewSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.version, VIEW_SPEC_VERSION);
    }

    #[test]
    fn list_fields_skip_hidden_and_sort_by_priority() {
        let mut spec = sample_spec();
        spec.fields[0].priority = 10;
        spec.fields[1].priority = 1;
        let listed: Vec<&str> = spec
            .list_fields()
            .iter()
            .map(|f| f.field_name.as_str())
            .collect();
        assert_eq!(listed, vec!["status", "full_name"]);
        assert!(!listed.contains(&"password_hash"));
    }

    #[test]
    fn resolve_mode_rejects_disallowed() {
        let spec = sample_spec();
        // compact is not allowed -> fall back to default (list)
        assert_eq!(spec.resolve_mode(Some("compact")), ViewMode::List);
        assert_eq!(spec.resolve_mode(Some("cards")), ViewMode::Cards);
        assert_eq!(spec.resolve_mode(None), ViewMode::List);
    }
}
