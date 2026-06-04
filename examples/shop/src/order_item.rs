//! OrderItem model and admin configuration.

use rust_decimal::Decimal;
use rustio_admin::{Error, Model, ModelAdmin, Row, RustioAdmin, Value};

#[derive(RustioAdmin)]
pub struct OrderItem {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub quantity: i32,
    pub unit_price: Decimal,
}

impl Model for OrderItem {
    const TABLE: &'static str = "order_items";
    const COLUMNS: &'static [&'static str] =
        &["id", "order_id", "product_id", "quantity", "unit_price"];
    const INSERT_COLUMNS: &'static [&'static str] =
        &["order_id", "product_id", "quantity", "unit_price"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get_i64("id")?,
            order_id: row.get_i64("order_id")?,
            product_id: row.get_i64("product_id")?,
            quantity: row.get_i32("quantity")?,
            unit_price: row.get_decimal("unit_price")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            self.order_id.into(),
            self.product_id.into(),
            self.quantity.into(),
            self.unit_price.into(),
        ]
    }
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for OrderItem {
    fn list_display() -> &'static [&'static str] {
        &["id", "order_id", "product_id", "quantity", "unit_price"]
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
