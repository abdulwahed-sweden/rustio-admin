//! The `vitals` table — measurements recorded for a patient.

use clinic_core::prelude::*;

/// One set of vitals for a patient. `patient_id` links back to
/// `patients.id`; it is a plain `i64` column here (relation widgets are
/// an opt-in the model declares explicitly when wanted).
#[derive(RustioAdmin)]
pub struct Vitals {
    pub id: i64,
    pub patient_id: i64,
    pub heart_rate: i64,
    pub notes: String,
    pub created_at: DateTime<Utc>,
}

impl Model for Vitals {
    const TABLE: &'static str = "vitals";
    const COLUMNS: &'static [&'static str] =
        &["id", "patient_id", "heart_rate", "notes", "created_at"];
    const INSERT_COLUMNS: &'static [&'static str] =
        &["patient_id", "heart_rate", "notes", "created_at"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get_i64("id")?,
            patient_id: row.get_i64("patient_id")?,
            heart_rate: row.get_i64("heart_rate")?,
            notes: row.get_string("notes")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            self.patient_id.into(),
            self.heart_rate.into(),
            self.notes.clone().into(),
            self.created_at.into(),
        ]
    }
}

impl ModelAdmin for Vitals {
    fn list_display() -> &'static [&'static str] {
        &["id", "patient_id", "heart_rate", "notes", "created_at"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["patient_id"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-created_at"]
    }
}
