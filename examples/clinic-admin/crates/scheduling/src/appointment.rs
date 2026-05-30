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

impl Model for Appointment {
    const TABLE: &'static str = "appointments";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "patient_id",
        "scheduled_at",
        "reason",
        "status",
        "created_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "patient_id",
        "scheduled_at",
        "reason",
        "status",
        "created_at",
    ];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get_i64("id")?,
            patient_id: row.get_i64("patient_id")?,
            scheduled_at: row.get_datetime("scheduled_at")?,
            reason: row.get_string("reason")?,
            status: row.get_string("status")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            self.patient_id.into(),
            self.scheduled_at.into(),
            self.reason.clone().into(),
            self.status.clone().into(),
            self.created_at.into(),
        ]
    }
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
