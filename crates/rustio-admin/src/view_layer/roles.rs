//! Field roles and badge colour intent — the vocabulary of *visual
//! importance* the database schema can't express on its own.

use serde::{Deserialize, Serialize};

// public:
/// How a field participates in the visual layout of a generated admin view.
///
/// The schema gives us names and types; it does not tell us what matters
/// visually. `FieldRole` is where that intent is recorded, once, in a
/// [`ViewSpec`](super::spec::ViewSpec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldRole {
    /// The strongest field. Becomes the row title in list/card modes.
    Primary,
    /// Muted supporting information shown next to the primary.
    Secondary,
    /// Rendered as a pill/chip, usually an enum or status.
    Badge,
    /// A date/time field, formatted consistently and kept visually quiet.
    Timestamp,
    /// Only shown on the detail page, never in list/table/card.
    DetailOnly,
    /// Never rendered anywhere in the visible UI.
    Hidden,
}

impl FieldRole {
    // public:
    /// Whether a field with this role should appear in list/table/card views.
    pub fn shows_in_list(self) -> bool {
        !matches!(self, FieldRole::DetailOnly | FieldRole::Hidden)
    }

    // public:
    /// Whether the field should reach the template context at all. `Hidden`
    /// fields are stripped before rendering so they never leak into HTML.
    pub fn reaches_template(self) -> bool {
        self != FieldRole::Hidden
    }
}

// public:
/// Semantic colour intent for badge fields. Kept deliberately small so
/// templates can map these to a fixed set of CSS classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticClass {
    /// No colour intent — the default grey pill.
    #[default]
    Neutral,
    /// Informational (blue) intent.
    Info,
    /// Positive / success (green) intent.
    Success,
    /// Caution (amber) intent.
    Warning,
    /// Negative / destructive (red) intent.
    Danger,
}

impl SemanticClass {
    // public:
    /// The CSS modifier suffix templates use, e.g. `badge--success`.
    pub fn css_suffix(self) -> &'static str {
        match self {
            SemanticClass::Neutral => "neutral",
            SemanticClass::Info => "info",
            SemanticClass::Success => "success",
            SemanticClass::Warning => "warning",
            SemanticClass::Danger => "danger",
        }
    }
}
