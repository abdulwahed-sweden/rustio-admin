//! **Contract: the public API a downstream project actually writes.**
//!
//! Until the lean-core reset this was guarded by `examples/clinic`, which CI
//! built against the framework at HEAD through `[patch.crates-io]`. That
//! example is gone; this file is its replacement, and it is the only place
//! that exercises the framework the way a consumer crate does.
//!
//! What it pins, all of it load-bearing for downstream compilation:
//!
//! - `#[derive(RustioAdmin)]` generates a working `Model` impl.
//! - `#[rustio(extra_columns = [...])]` adds a non-field column to
//!   `Model::COLUMNS` (clinic's `search_vector`), which is what makes
//!   `ModelAdmin::search_index_column` pass the injection-safety check in
//!   `admin::ops`.
//! - Field types resolve through the framework's own re-exports, so a
//!   consumer needs `rustio-admin` alone — no direct `chrono` / `uuid` /
//!   `rust_decimal` / `sqlx` dependency.
//! - Every `ModelAdmin` hook a real project overrides still type-checks.
//! - The `Admin` builder chains and accepts models.
//!
//! A breaking change to any of those fails this test at **compile** time,
//! which is exactly the signal the clinic job used to give.

use rustio_admin::admin::Admin;
use rustio_admin::{
    BulkAction, BulkActionResult, DateTime, Decimal, FieldValidationError, Fieldset, Inline, Model,
    ModelAdmin, NaiveDate, RustioAdmin, Utc, Uuid,
};

/// A model in the shape a consumer writes one: `i64` primary key, a
/// generated column declared through `extra_columns`, and field types named
/// only through the framework's re-exports.
#[derive(RustioAdmin)]
#[rustio(extra_columns = ["search_vector"])]
pub struct Patient {
    pub id: i64,
    pub full_name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl ModelAdmin for Patient {
    fn list_display() -> &'static [&'static str] {
        &["id", "full_name", "email", "created_at"]
    }
    fn search_fields() -> &'static [&'static str] {
        &["full_name", "email"]
    }
    /// The opt-in full-text path. Only legal because `search_vector` is in
    /// `COLUMNS` via `extra_columns`.
    fn search_index_column() -> Option<&'static str> {
        Some("search_vector")
    }
    fn ordering() -> &'static [&'static str] {
        &["-created_at"]
    }
    fn list_filter() -> &'static [&'static str] {
        &["email"]
    }
}

/// A second model exercising the remaining field types a project names
/// through the re-exports, plus the hooks `Patient` does not use.
#[derive(RustioAdmin)]
pub struct Invoice {
    pub id: i64,
    pub patient_id: i64,
    pub reference: Uuid,
    pub amount: Decimal,
    pub issued_on: NaiveDate,
    pub paid: bool,
}

impl ModelAdmin for Invoice {
    fn list_display() -> &'static [&'static str] {
        &["id", "reference", "amount", "issued_on", "paid"]
    }

    /// Inline children — the hook only `examples/shop` used to cover.
    fn inlines() -> &'static [Inline] {
        &[Inline {
            target_model: "invoice_lines",
            fk_field: "invoice_id",
            label: Some("Lines"),
            max_rows: 20,
            display_field: None,
        }]
    }

    /// Fieldsets — grouping on the edit form.
    fn fieldsets() -> &'static [Fieldset] {
        &[Fieldset {
            title: "Billing",
            fields: &["amount", "issued_on", "paid"],
        }]
    }

    /// Project-side validation returning the framework's error type.
    fn validate(model: &Self) -> std::result::Result<(), Vec<FieldValidationError>> {
        if model.amount.is_sign_negative() {
            return Err(vec![FieldValidationError::field(
                "amount",
                "amount must not be negative",
            )]);
        }
        Ok(())
    }

    /// A bulk action, declared the way a project declares one.
    fn bulk_actions() -> &'static [BulkAction] {
        &[BulkAction {
            name: "mark_paid",
            label: "Mark paid",
            destructive: false,
            confirm: true,
            permission: None,
        }]
    }
}

#[test]
fn derive_generates_a_usable_model_impl() {
    // TABLE and COLUMNS are what every SQL path in `admin::ops` validates
    // against; a derive regression shows up here first.
    assert_eq!(Patient::TABLE, "patients");
    assert!(Patient::COLUMNS.contains(&"id"));
    assert!(Patient::COLUMNS.contains(&"full_name"));
    assert_eq!(Invoice::TABLE, "invoices");
}

#[test]
fn extra_columns_reach_model_columns() {
    // The generated `search_vector` is not a struct field. It must still be
    // in COLUMNS or `search_index_column` is rejected by the injection guard
    // in `admin::ops` and full-text search silently degrades to ILIKE.
    assert!(
        Patient::COLUMNS.contains(&"search_vector"),
        "#[rustio(extra_columns)] must add the column to Model::COLUMNS"
    );
    assert_eq!(Patient::search_index_column(), Some("search_vector"));
}

#[test]
fn admin_builder_accepts_models_and_chains() {
    // The exact call shape a project's `register()` uses.
    let admin = Admin::new()
        .app_name("Contract")
        .accent_color("#059669")
        .model::<Patient>()
        .model::<Invoice>();
    let names: Vec<&str> = admin.entries().iter().map(|e| e.admin_name).collect();
    assert!(names.contains(&"patients"), "got {names:?}");
    assert!(names.contains(&"invoices"), "got {names:?}");
}

#[test]
fn model_admin_hooks_are_reachable_through_the_public_trait() {
    assert_eq!(Patient::ordering(), &["-created_at"]);
    assert_eq!(Invoice::inlines().len(), 1);
    assert_eq!(Invoice::fieldsets().len(), 1);
    assert_eq!(Invoice::bulk_actions().len(), 1);
}

#[test]
fn project_validation_returns_the_framework_error_type() {
    let mut invoice = Invoice {
        id: 1,
        patient_id: 1,
        reference: Uuid::nil(),
        amount: Decimal::from(-5),
        issued_on: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
        paid: false,
    };
    let errs = Invoice::validate(&invoice).expect_err("negative amount must fail");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].field, Some("amount"));

    invoice.amount = Decimal::from(5);
    assert!(Invoice::validate(&invoice).is_ok());
}

/// Compile-only: the re-export surface a scaffolded `Cargo.toml` relies on.
/// If any of these stops resolving, every generated project breaks.
#[test]
fn framework_reexports_cover_the_model_field_types() {
    fn _types(_: DateTime<Utc>, _: NaiveDate, _: Decimal, _: Uuid) {}
    let _ = rustio_admin::chrono::Utc::now();
    let _: BulkActionResult = BulkActionResult::default();
}
