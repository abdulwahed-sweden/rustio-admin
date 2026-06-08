//! The `patients` table — one row per person receiving care.

use clinic_core::prelude::*;

/// A patient. Primary keys are `i64` (the admin runtime requires it).
///
/// `#[rustio(extra_columns = ["search_vector"])]` adds a column to
/// `Model::COLUMNS` that is NOT a struct field: a generated `tsvector`
/// (see `migrations/0001_patients.sql`). The framework only honours
/// `search_index_column()` for full-text search when the column is in
/// COLUMNS (an injection-safety check in ops.rs); without it, search
/// silently falls back to a slow `ILIKE` scan. The derive never inserts
/// it or reads it in `from_row`.
#[derive(RustioAdmin)]
#[rustio(extra_columns = ["search_vector"])]
pub struct Patient {
    pub id: i64,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub created_at: DateTime<Utc>,
}

impl ModelAdmin for Patient {
    fn list_display() -> &'static [&'static str] {
        &["id", "full_name", "email", "phone", "created_at"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["full_name", "email", "phone"]
    }

    /// Opt INTO Postgres full-text search. This is the only model that
    /// does — search is off by default everywhere else. The
    /// `search_vector` column is a generated `tsvector` maintained by
    /// Postgres (see `migrations/0001_patients.sql`); the framework only
    /// reads it in the `WHERE` clause and never indexes anything itself.
    fn search_index_column() -> Option<&'static str> {
        Some("search_vector")
    }

    fn ordering() -> &'static [&'static str] {
        &["-created_at"]
    }
}
