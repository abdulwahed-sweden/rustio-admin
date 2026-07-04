//! Payment-method catalogue — the payment "systems" the shop accepts,
//! managed from the admin. Sweden-first (Swish, Klarna, the bank rails),
//! with the global wallets/cards alongside. The `code` here is the stable
//! slug that `Payment::method` references.

use rustio_admin::{ModelAdmin, RustioAdmin};

#[derive(RustioAdmin)]
pub struct PaymentMethod {
    pub id: i64,
    pub code: String,
    pub display_name: String,
    #[rustio(choices = ["mobile", "invoice", "card", "bank_transfer", "direct_debit", "wallet"])]
    pub category: String,
    pub provider: String,
    pub country: String,
    pub currency: String,
    pub is_active: bool,
    pub display_order: i64,
}

impl ModelAdmin for PaymentMethod {
    fn list_display() -> &'static [&'static str] {
        &[
            "display_name",
            "category",
            "provider",
            "country",
            "currency",
            "is_active",
        ]
    }

    fn list_filter() -> &'static [&'static str] {
        &["category", "country", "currency", "is_active"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["code", "display_name", "provider"]
    }

    // Everyday rails first (lowest `display_order`), then by id for stability.
    fn ordering() -> &'static [&'static str] {
        &["display_order", "id"]
    }
}
