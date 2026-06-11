//! Product model and admin configuration.

use rustio_admin::{Decimal, Inline, ModelAdmin, RustioAdmin};

// `#[derive(RustioAdmin)]` now also generates `impl Model` (TABLE,
// COLUMNS, INSERT_COLUMNS, from_row, insert_values) from these fields —
// there is no hand-written ORM glue to keep in sync with the struct.
#[derive(RustioAdmin)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price: Decimal,
    pub in_stock: bool,
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Product {
    fn list_display() -> &'static [&'static str] {
        &["name", "price", "in_stock"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["in_stock"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["name"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-id"]
    }

    // The product's gallery, listed on its edit page.
    fn inlines() -> &'static [Inline] {
        &[Inline {
            target_model: "ProductImage",
            fk_field: "product_id",
            label: Some("Images"),
            max_rows: 20,
            display_field: Some("alt_text"),
        }]
    }
}
