//! The `appointments` table — a booked visit for a patient.

use clinic_core::prelude::*;

/// An appointment. `status` is a plain text column (e.g. `scheduled`,
/// `completed`, `cancelled`) and makes a good list filter.
#[derive(RustioAdmin)]
pub struct Appointment {
    pub id: i64,
    pub patient_id: i64,
    pub scheduled_at: DateTime<Utc>,
    pub reason: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl ModelAdmin for Appointment {
    fn list_display() -> &'static [&'static str] {
        &["id", "patient_id", "scheduled_at", "reason", "status"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["status"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["reason"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-scheduled_at"]
    }
}
