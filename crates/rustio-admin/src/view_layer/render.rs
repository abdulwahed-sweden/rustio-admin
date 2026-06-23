//! Deterministic rendering: a saved [`ViewSpec`] plus raw rows in, a typed,
//! serde-serializable [`RenderedView`] out.
//!
//! Rendering reads the spec and the row values and nothing else — no roles are
//! decided here, no inference runs. Each cell is emitted as a tagged
//! [`RenderedCell`]; templates switch on `cell.kind` and contain no logic of
//! their own. Two guarantees the renderer upholds:
//!
//! 1. `Hidden`/`DetailOnly` fields are never read from the row, so a sensitive
//!    value cannot reach the template context or the HTML.
//! 2. A field consumed by a [`CellComposition`] is not also rendered standalone.

use std::collections::BTreeMap;

use serde::Serialize;

use super::compose::{CellComposition, ComposeStyle};
use super::modes::ViewMode;
use super::roles::{FieldRole, SemanticClass};
use super::spec::{FieldViewSpec, ViewSpec};

// public:
/// A single row's raw values keyed by field name. The admin's data layer
/// builds these; the renderer only reads them.
pub type RowData = BTreeMap<String, String>;

// public:
/// One rendered cell, ready for the template. Serializes with a `kind` tag
/// (`primary`, `secondary`, `badge`, `timestamp`, `composed`) so minijinja
/// templates can switch on `cell.kind` without seeing roles or running logic.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RenderedCell {
    /// The row/card title cell.
    Primary {
        /// Column label for table headers.
        label: String,
        /// The rendered text.
        value: String,
    },
    /// A muted supporting cell.
    Secondary {
        /// Column label for table headers.
        label: String,
        /// The rendered text.
        value: String,
    },
    /// A pill/chip cell carrying a semantic colour intent.
    Badge {
        /// Column label for table headers.
        label: String,
        /// The rendered text.
        value: String,
        /// Colour intent the template maps to a CSS class.
        semantic: SemanticClass,
    },
    /// A date/time cell.
    Timestamp {
        /// Column label for table headers.
        label: String,
        /// The rendered text.
        value: String,
    },
    /// Several fields merged into one visual unit.
    Composed {
        /// Optional header for the composed cell.
        label: Option<String>,
        /// Layout the template applies to `parts`.
        style: ComposeStyle,
        /// The merged field values, primary first.
        parts: Vec<CellPart>,
    },
}

// public:
/// One field inside a composed cell.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellPart {
    /// The schema field this part came from.
    pub field: String,
    /// The rendered value.
    pub value: String,
    /// Whether this is the composition's primary field.
    pub is_primary: bool,
}

// public:
/// Everything a list/table/card template needs for one row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RenderedRow {
    /// The row's cells, in render order.
    pub cells: Vec<RenderedCell>,
}

// public:
/// The view-level context handed to a template. Carries the active mode and
/// the rows; the template just iterates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RenderedView {
    /// The model (admin name) being rendered.
    pub model: String,
    /// The resolved view mode for this page.
    pub mode: ViewMode,
    /// The rendered rows.
    pub rows: Vec<RenderedRow>,
}

fn label_for(field: &FieldViewSpec) -> String {
    field
        .label
        .clone()
        .unwrap_or_else(|| humanize(&field.field_name))
}

/// "created_at" -> "Created At". Plain and predictable; the designer can
/// always override with an explicit label.
fn humanize(name: &str) -> String {
    name.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// public:
/// Render a single row according to a spec.
///
/// Composed cells render first (in spec order) and mark their fields consumed;
/// the remaining `list_fields()` (already filtered to exclude `Hidden`/
/// `DetailOnly` and sorted by priority) render as simple cells. Hidden fields
/// are never read from `row`.
pub fn render_row(spec: &ViewSpec, row: &RowData) -> RenderedRow {
    let mut cells = Vec::new();
    let mut consumed: Vec<&str> = Vec::new();

    for comp in &spec.compositions {
        cells.push(render_composition(comp, row));
        consumed.extend(comp.all_fields());
    }

    for field in spec.list_fields() {
        if consumed.contains(&field.field_name.as_str()) {
            continue;
        }
        let value = row.get(&field.field_name).cloned().unwrap_or_default();
        let label = label_for(field);
        let cell = match field.role {
            FieldRole::Primary => RenderedCell::Primary { label, value },
            FieldRole::Secondary => RenderedCell::Secondary { label, value },
            FieldRole::Badge => RenderedCell::Badge {
                label,
                value,
                semantic: field.semantic_class.unwrap_or_default(),
            },
            FieldRole::Timestamp => RenderedCell::Timestamp { label, value },
            // list_fields() already filtered these out, but be explicit.
            FieldRole::DetailOnly | FieldRole::Hidden => continue,
        };
        cells.push(cell);
    }

    RenderedRow { cells }
}

fn render_composition(comp: &CellComposition, row: &RowData) -> RenderedCell {
    let mut parts = Vec::with_capacity(1 + comp.secondary_fields.len());
    parts.push(CellPart {
        field: comp.primary_field.clone(),
        value: row.get(&comp.primary_field).cloned().unwrap_or_default(),
        is_primary: true,
    });
    for name in &comp.secondary_fields {
        parts.push(CellPart {
            field: name.clone(),
            value: row.get(name).cloned().unwrap_or_default(),
            is_primary: false,
        });
    }
    RenderedCell::Composed {
        label: comp.label.clone(),
        style: comp.style,
        parts,
    }
}

// public:
/// Render a full page of rows for the resolved mode.
pub fn render_view(spec: &ViewSpec, mode: ViewMode, rows: &[RowData]) -> RenderedView {
    RenderedView {
        model: spec.model.clone(),
        mode,
        rows: rows.iter().map(|r| render_row(spec, r)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_layer::compose::ComposeStyle;
    use crate::view_layer::spec::VIEW_SPEC_VERSION;

    fn customer_spec() -> ViewSpec {
        ViewSpec {
            model: "customer".into(),
            default_mode: ViewMode::List,
            allowed_modes: vec![ViewMode::List, ViewMode::Table],
            fields: vec![
                FieldViewSpec::new("full_name", FieldRole::Primary),
                FieldViewSpec::new("email", FieldRole::Secondary),
                {
                    let mut f = FieldViewSpec::new("status", FieldRole::Badge);
                    f.semantic_class = Some(SemanticClass::Success);
                    f
                },
                FieldViewSpec::new("password_hash", FieldRole::Hidden),
                FieldViewSpec::new("internal_notes", FieldRole::DetailOnly),
            ],
            compositions: vec![],
            default_filters: vec![],
            version: VIEW_SPEC_VERSION,
        }
    }

    fn customer_row() -> RowData {
        let mut row = RowData::new();
        row.insert("full_name".into(), "Nadim Shahin".into());
        row.insert("email".into(), "nadim@example.com".into());
        row.insert("status".into(), "active".into());
        row.insert("password_hash".into(), "$2b$super-secret".into());
        row.insert("internal_notes".into(), "VIP".into());
        row
    }

    #[test]
    fn hidden_field_value_never_rendered() {
        let rendered = render_row(&customer_spec(), &customer_row());
        let leaked = rendered.cells.iter().any(|c| match c {
            RenderedCell::Primary { value, .. }
            | RenderedCell::Secondary { value, .. }
            | RenderedCell::Badge { value, .. }
            | RenderedCell::Timestamp { value, .. } => value.contains("secret"),
            RenderedCell::Composed { parts, .. } => {
                parts.iter().any(|p| p.value.contains("secret"))
            }
        });
        assert!(!leaked, "sensitive value leaked into rendered cells");
    }

    #[test]
    fn detail_only_excluded_from_list() {
        let rendered = render_row(&customer_spec(), &customer_row());
        let has_notes = rendered
            .cells
            .iter()
            .any(|c| matches!(c, RenderedCell::Secondary { value, .. } if value == "VIP"));
        assert!(!has_notes);
    }

    #[test]
    fn badge_carries_semantic_class() {
        let rendered = render_row(&customer_spec(), &customer_row());
        let badge = rendered
            .cells
            .iter()
            .find(|c| matches!(c, RenderedCell::Badge { .. }));
        match badge {
            Some(RenderedCell::Badge { semantic, .. }) => {
                assert_eq!(*semantic, SemanticClass::Success);
            }
            _ => panic!("expected a badge cell"),
        }
    }

    #[test]
    fn composed_field_not_rendered_twice() {
        let mut spec = customer_spec();
        spec.compositions.push(CellComposition {
            id: "identity".into(),
            label: Some("Customer".into()),
            style: ComposeStyle::Stacked,
            primary_field: "full_name".into(),
            secondary_fields: vec!["email".into()],
        });
        let rendered = render_row(&spec, &customer_row());

        // full_name and email should only appear inside the composed cell now
        let standalone_primary = rendered
            .cells
            .iter()
            .any(|c| matches!(c, RenderedCell::Primary { value, .. } if value == "Nadim Shahin"));
        assert!(!standalone_primary);

        let composed = rendered
            .cells
            .iter()
            .find(|c| matches!(c, RenderedCell::Composed { .. }));
        assert!(composed.is_some());
    }

    #[test]
    fn humanize_handles_snake_case() {
        assert_eq!(humanize("created_at"), "Created At");
        assert_eq!(humanize("full_name"), "Full Name");
    }

    #[test]
    fn cell_serializes_with_kind_tag() {
        let cell = RenderedCell::Badge {
            label: "Status".into(),
            value: "active".into(),
            semantic: SemanticClass::Success,
        };
        let json = serde_json::to_value(&cell).unwrap();
        assert_eq!(json["kind"], "badge");
        assert_eq!(json["semantic"], "success");
        assert_eq!(json["value"], "active");
    }

    // Proves the template contract end-to-end against the SHIPPED partials:
    // templates switch on `cell.kind` only, primary/badge render, and the
    // Hidden `password_hash` value never appears in the HTML.
    #[test]
    fn shipped_partials_switch_on_kind_and_drop_hidden() {
        use minijinja::{context, Environment, Value};

        const CELL: &str = include_str!("../../assets/templates/admin/view_layer/_cell.html");
        const ROW: &str = include_str!("../../assets/templates/admin/view_layer/_row.html");

        let mut env = Environment::new();
        env.add_template("admin/view_layer/_cell.html", CELL)
            .unwrap();
        env.add_template("admin/view_layer/_row.html", ROW).unwrap();

        let rendered = render_row(&customer_spec(), &customer_row());
        let tmpl = env.get_template("admin/view_layer/_row.html").unwrap();
        let html = tmpl
            .render(context! { row => Value::from_serialize(&rendered) })
            .unwrap();

        assert!(html.contains("av-primary"));
        assert!(html.contains("Nadim Shahin"));
        assert!(html.contains("badge--success"));
        assert!(
            !html.contains("secret"),
            "hidden value reached HTML: {html}"
        );
        assert!(
            !html.contains("VIP"),
            "detail-only value reached HTML: {html}"
        );
    }
}
