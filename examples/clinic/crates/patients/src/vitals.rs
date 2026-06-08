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
