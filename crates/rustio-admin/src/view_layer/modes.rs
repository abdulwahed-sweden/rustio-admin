//! View modes — the layouts a collection of records can be rendered in.

use serde::{Deserialize, Serialize};

// public:
/// The visual layout used to render a collection of records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    /// Traditional column-based admin table.
    Table,
    /// Richer row layout built around primary/secondary/composed cells.
    List,
    /// Card grid using the primary field as the card title.
    Cards,
    /// Dense layout for large datasets.
    Compact,
}

impl ViewMode {
    // public:
    /// Slug used in URLs and the mode switcher, e.g. `?view=cards`.
    pub fn slug(self) -> &'static str {
        match self {
            ViewMode::Table => "table",
            ViewMode::List => "list",
            ViewMode::Cards => "cards",
            ViewMode::Compact => "compact",
        }
    }

    // public:
    /// Parse a slug coming from a query string. Unknown values return `None`
    /// so the caller can fall back to the spec default.
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "table" => Some(ViewMode::Table),
            "list" => Some(ViewMode::List),
            "cards" => Some(ViewMode::Cards),
            "compact" => Some(ViewMode::Compact),
            _ => None,
        }
    }
}
