//! The `patients` table — one row per person receiving care.

use clinic_core::prelude::*;

/// A patient. Primary keys are `i64` (the admin runtime requires it).
#[derive(RustioAdmin)]
pub struct Patient {
    pub id: i64,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub created_at: DateTime<Utc>,
}

impl Model for Patient {
    const TABLE: &'static str = "patients";
    // `search_vector` is listed here even though it is not a struct field:
    // the framework only honours `search_index_column()` for FTS when the
    // column is in COLUMNS (an injection-safety check in ops.rs). Without
    // it, search silently falls back to a slow `ILIKE` scan. It is a
    // generated `tsvector` (see 0001_patients.sql); `from_row` ignores it.
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "full_name",
        "email",
        "phone",
        "created_at",
        "search_vector",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &["full_name", "email", "phone", "created_at"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get_i64("id")?,
            full_name: row.get_string("full_name")?,
            email: row.get_string("email")?,
            phone: row.get_string("phone")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            self.full_name.clone().into(),
            self.email.clone().into(),
            self.phone.clone().into(),
            self.created_at.into(),
        ]
    }
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
