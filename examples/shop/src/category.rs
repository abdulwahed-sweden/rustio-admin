//! Category model and admin configuration.

use rustio_admin::{Error, Model, ModelAdmin, Row, RustioAdmin, Value};

#[derive(RustioAdmin)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

impl Model for Category {
    const TABLE: &'static str = "categories";
    const COLUMNS: &'static [&'static str] = &["id", "name", "slug"];
    const INSERT_COLUMNS: &'static [&'static str] = &["name", "slug"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get_i64("id")?,
            name: row.get_string("name")?,
            slug: row.get_string("slug")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![self.name.clone().into(), self.slug.clone().into()]
    }
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Category {
    fn list_display() -> &'static [&'static str] {
        &["id", "name", "slug"]
    }

    fn list_filter() -> &'static [&'static str] {
        &[]
    }

    fn search_fields() -> &'static [&'static str] {
        &["name", "slug"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-id"]
    }
}
