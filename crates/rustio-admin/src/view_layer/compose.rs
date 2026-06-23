//! Cell composition — merging several schema fields into one visual unit
//! (e.g. "name over email", or "title with a trailing status badge").

use serde::{Deserialize, Serialize};

// public:
/// How several fields are visually merged into a single cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposeStyle {
    /// Fields stacked vertically, primary on top.
    Stacked,
    /// Leading icon followed by inline text.
    InlineIcon,
    /// Primary text with a trailing badge.
    BadgeInline,
}

impl ComposeStyle {
    // public:
    /// The stable slug used in forms and serde (matches the snake_case
    /// representation), e.g. `badge_inline`.
    pub fn slug(self) -> &'static str {
        match self {
            ComposeStyle::Stacked => "stacked",
            ComposeStyle::InlineIcon => "inline_icon",
            ComposeStyle::BadgeInline => "badge_inline",
        }
    }

    // public:
    /// Parse a style slug from the designer form. Unknown values return `None`.
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "stacked" => Some(ComposeStyle::Stacked),
            "inline_icon" => Some(ComposeStyle::InlineIcon),
            "badge_inline" => Some(ComposeStyle::BadgeInline),
            _ => None,
        }
    }

    // public:
    /// Every style, in display order — for building the style `<select>`.
    pub fn all() -> &'static [ComposeStyle] {
        &[
            ComposeStyle::Stacked,
            ComposeStyle::InlineIcon,
            ComposeStyle::BadgeInline,
        ]
    }
}

// public:
/// A composed cell merges multiple schema fields into one visual unit.
///
/// Example: an "avatar + name + email" cell, or "status badge next to title".
/// The renderer pulls `primary_field` first, then the `secondary_fields`, and
/// lays them out according to `style`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellComposition {
    /// Stable id so a ViewSpec editor can reference this composition.
    pub id: String,
    /// Optional column header / label for the composed cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The layout the renderer applies to the merged fields.
    pub style: ComposeStyle,
    /// The field that carries the strongest visual weight in the cell.
    pub primary_field: String,
    /// Supporting fields, rendered in order after the primary.
    #[serde(default)]
    pub secondary_fields: Vec<String>,
}

impl CellComposition {
    // public:
    /// Every field this composition touches, primary first. Used by the
    /// renderer to know which raw fields a cell already consumes so they
    /// aren't also rendered standalone.
    pub fn all_fields(&self) -> Vec<&str> {
        let mut names = Vec::with_capacity(1 + self.secondary_fields.len());
        names.push(self.primary_field.as_str());
        names.extend(self.secondary_fields.iter().map(String::as_str));
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_slug_roundtrips_and_matches_serde() {
        for style in ComposeStyle::all() {
            assert_eq!(ComposeStyle::from_slug(style.slug()), Some(*style));
            let json = serde_json::to_string(style).unwrap();
            assert_eq!(json, format!("\"{}\"", style.slug()));
        }
        assert_eq!(ComposeStyle::from_slug("nope"), None);
    }
}
