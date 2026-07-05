//! Translator model — a freelancer with the languages they cover.
//!
//! `#[derive(RustioAdmin)]` emits both the admin pages and the `orm::Model`
//! glue (table, columns, row mapping) from these fields; there is no
//! hand-written ORM boilerplate.

use rustio_admin::{ModelAdmin, RustioAdmin};

#[derive(RustioAdmin)]
pub struct Translator {
    pub id: i64,
    pub name: String,
    pub email: String,
    /// Free text for the Quick Start, e.g. `"en, ar, sv"`. A real system
    /// would model this as its own table and a join — `fk:` handles that.
    pub languages: String,
    pub active: bool,
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Translator {
    fn list_display() -> &'static [&'static str] {
        &["name", "email", "languages", "active"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["active"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["name", "email", "languages"]
    }

    fn ordering() -> &'static [&'static str] {
        &["name"]
    }
}
