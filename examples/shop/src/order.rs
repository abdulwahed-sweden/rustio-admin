//! Order model and admin configuration.

use rustio_admin::{Decimal, FieldValidationError, Inline, ModelAdmin, RustioAdmin};

#[derive(RustioAdmin)]
pub struct Order {
    pub id: i64,
    pub customer_id: i64,
    pub total: Decimal,
    #[rustio(choices = ["pending", "paid", "shipped", "cancelled"])]
    pub status: String,
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Order {
    fn list_display() -> &'static [&'static str] {
        &["customer_id", "total", "status"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["status"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["status"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-id"]
    }

    // Show the order's line items and payment right on its edit page,
    // so an operator sees the whole order without hunting through the
    // Order items / Payments lists. `target_model` is the child struct
    // name; `fk_field` is the column on the child that points back here.
    fn inlines() -> &'static [Inline] {
        &[
            Inline {
                target_model: "OrderItem",
                fk_field: "order_id",
                label: Some("Line items"),
                max_rows: 100,
                display_field: Some("quantity"),
            },
            Inline {
                target_model: "Payment",
                fk_field: "order_id",
                label: Some("Payments"),
                max_rows: 20,
                display_field: Some("status"),
            },
        ]
    }

    // Business rule checked before the row hits Postgres. A good place
    // to copy from when you add your own validation.
    fn validate(order: &Self) -> std::result::Result<(), Vec<FieldValidationError>> {
        if order.total < Decimal::ZERO {
            return Err(vec![FieldValidationError::field(
                "total",
                "Order total cannot be negative.",
            )]);
        }
        Ok(())
    }
}
