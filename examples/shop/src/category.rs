//! Category model and admin configuration.

use rustio_admin::{ModelAdmin, RustioAdmin};

#[derive(RustioAdmin)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Category {
    fn list_display() -> &'static [&'static str] {
        &["name", "slug"]
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
