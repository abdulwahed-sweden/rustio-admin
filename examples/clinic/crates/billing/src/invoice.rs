//! The `invoices` table — money owed by a patient.

use clinic_core::prelude::*;

/// An invoice. The amount is stored in whole cents (`i64`) to avoid
/// floating-point money; format it for display in the UI layer.
#[derive(RustioAdmin)]
pub struct Invoice {
    pub id: i64,
    pub patient_id: i64,
    pub amount_cents: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl Model for Invoice {
    const TABLE: &'static str = "invoices";
    const COLUMNS: &'static [&'static str] =
        &["id", "patient_id", "amount_cents", "status", "created_at"];
    const INSERT_COLUMNS: &'static [&'static str] =
        &["patient_id", "amount_cents", "status", "created_at"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get_i64("id")?,
            patient_id: row.get_i64("patient_id")?,
            amount_cents: row.get_i64("amount_cents")?,
            status: row.get_string("status")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            self.patient_id.into(),
            self.amount_cents.into(),
            self.status.clone().into(),
            self.created_at.into(),
        ]
    }
}

impl ModelAdmin for Invoice {
    fn list_display() -> &'static [&'static str] {
        &["id", "patient_id", "amount_cents", "status", "created_at"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["status"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-created_at"]
    }
}
