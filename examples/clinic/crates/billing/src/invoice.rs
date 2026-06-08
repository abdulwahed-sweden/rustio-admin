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
