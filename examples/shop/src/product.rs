//! Product model and admin configuration.

use rust_decimal::Decimal;
use rustio_admin::{Error, Inline, Model, ModelAdmin, Row, RustioAdmin, Value};

#[derive(RustioAdmin)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price: Decimal,
    pub in_stock: bool,
}

impl Model for Product {
    const TABLE: &'static str = "products";
    const COLUMNS: &'static [&'static str] = &["id", "name", "price", "in_stock"];
    const INSERT_COLUMNS: &'static [&'static str] = &["name", "price", "in_stock"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get_i64("id")?,
            name: row.get_string("name")?,
            price: row.get_decimal("price")?,
            in_stock: row.get_bool("in_stock")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            self.name.clone().into(),
            self.price.into(),
            self.in_stock.into(),
        ]
    }
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Product {
    fn list_display() -> &'static [&'static str] {
        &["id", "name", "price", "in_stock"]
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
