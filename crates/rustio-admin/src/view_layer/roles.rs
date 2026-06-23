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

    // public:
    /// The stable slug used in forms and serde, e.g. `detail_only`. Matches
    /// the `snake_case` serde representation.
    pub fn slug(self) -> &'static str {
        match self {
            FieldRole::Primary => "primary",
            FieldRole::Secondary => "secondary",
            FieldRole::Badge => "badge",
            FieldRole::Timestamp => "timestamp",
            FieldRole::DetailOnly => "detail_only",
            FieldRole::Hidden => "hidden",
        }
    }

    // public:
    /// Parse a role slug coming from the designer form. Unknown values return
    /// `None` so the caller can keep the field's previous role.
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "primary" => Some(FieldRole::Primary),
            "secondary" => Some(FieldRole::Secondary),
            "badge" => Some(FieldRole::Badge),
            "timestamp" => Some(FieldRole::Timestamp),
            "detail_only" => Some(FieldRole::DetailOnly),
            "hidden" => Some(FieldRole::Hidden),
            _ => None,
        }
    }

    // public:
    /// Every role and its slug, in display order — for building the role
    /// `<select>` in the designer without hard-coding the list in a template.
    pub fn all() -> &'static [FieldRole] {
        &[
            FieldRole::Primary,
            FieldRole::Secondary,
            FieldRole::Badge,
            FieldRole::Timestamp,
            FieldRole::DetailOnly,
            FieldRole::Hidden,
        ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_slug_roundtrips_for_every_variant() {
        for role in FieldRole::all() {
            assert_eq!(FieldRole::from_slug(role.slug()), Some(*role));
        }
    }

    #[test]
    fn role_slug_matches_serde_repr() {
        // slug() must equal the snake_case serde tag the renderer/spec use.
        for role in FieldRole::all() {
            let json = serde_json::to_string(role).unwrap();
            assert_eq!(json, format!("\"{}\"", role.slug()));
        }
    }

    #[test]
    fn unknown_role_slug_is_none() {
        assert_eq!(FieldRole::from_slug("nope"), None);
    }
}
