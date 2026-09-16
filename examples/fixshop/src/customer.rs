//! Whoever walked in with the broken thing. Name and phone, nothing
//! else — a repair shop does not need a CRM.

use rustio_admin::{DateTime, ModelAdmin, RustioAdmin, Utc};

#[derive(RustioAdmin)]
#[rustio(table = "customers")]
pub struct Customer {
    pub id: i64,
    pub name: String,
    /// `format = "phone"` gives the form an `<input type="tel">` and a
    /// typo guard in the generated `from_form`. The column is plain
    /// `TEXT` at rest.
    #[rustio(format = "phone")]
    pub phone: String,
    /// Named `created_at`, so the derive treats it as framework-managed:
    /// hidden from the form, filled with `Utc::now()` on insert.
    pub created_at: DateTime<Utc>,
}

impl ModelAdmin for Customer {
    fn list_display() -> &'static [&'static str] {
        &["name", "phone", "created_at"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["name", "phone"]
    }

    fn ordering() -> &'static [&'static str] {
        &["name"]
    }
}
