//! OrderItem model and admin configuration.

use rustio_admin::{Decimal, ModelAdmin, RustioAdmin};

#[derive(RustioAdmin)]
pub struct OrderItem {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub quantity: i32,
    pub unit_price: Decimal,
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for OrderItem {
    fn list_display() -> &'static [&'static str] {
        &["order_id", "product_id", "quantity", "unit_price"]
    }

    fn list_filter() -> &'static [&'static str] {
        &[]
    }

    fn search_fields() -> &'static [&'static str] {
        &[]
    }

    fn ordering() -> &'static [&'static str] {
        &["-id"]
    }
}
