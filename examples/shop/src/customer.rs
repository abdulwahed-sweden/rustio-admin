//! Customer model and admin configuration.

use rustio_admin::{Inline, ModelAdmin, RustioAdmin};

#[derive(RustioAdmin)]
pub struct Customer {
    pub id: i64,
    pub full_name: String,
    #[rustio(format = "email")]
    pub email: String,
    #[rustio(format = "phone")]
    pub phone: String,
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Customer {
    fn list_display() -> &'static [&'static str] {
        &["full_name", "email", "phone"]
    }

    fn list_filter() -> &'static [&'static str] {
        &[]
    }

    fn search_fields() -> &'static [&'static str] {
        &["full_name", "email", "phone"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-id"]
    }

    // A customer's orders and addresses, listed on their edit page.
    fn inlines() -> &'static [Inline] {
        &[
            Inline {
                target_model: "Order",
                fk_field: "customer_id",
                label: Some("Orders"),
                max_rows: 50,
                display_field: Some("status"),
            },
            Inline {
                target_model: "Address",
                fk_field: "customer_id",
                label: Some("Addresses"),
                max_rows: 20,
                display_field: Some("city"),
            },
        ]
    }
}
