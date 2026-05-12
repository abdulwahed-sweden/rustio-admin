//! `ModelAdmin` — Django-style customisation surface.
//!
//! Every model that ships through `Admin::model::<M>()` must
//! implement `ModelAdmin`. The trait defines defaults for every
//! method, so a project that wants standard behaviour writes a one-
//! line empty impl:
//!
//! ```ignore
//! use rustio_admin::ModelAdmin;
//!
//! impl ModelAdmin for Course {}            // accept every default
//! ```
//!
//! Override only the methods you care about; the rest inherit the
//! trait defaults:
//!
//! ```ignore
//! impl ModelAdmin for Course {
//!     fn list_display() -> &'static [&'static str] {
//!         &["code", "title", "credit_hours", "is_published"]
//!     }
//!     fn list_filter()  -> &'static [&'static str] { &["status", "level"] }
//!     fn search_fields() -> &'static [&'static str] { &["code", "title"] }
//!     fn ordering()     -> &'static [&'static str] { &["code"] }
//! }
//! ```
//!
//! The values are captured into [`super::AdminEntry`] at registration
//! time. The runtime reads them straight from the entry — no
//! per-request virtual dispatch beyond the existing `dyn AdminOps`.
//!
//! ### Why no blanket impl?
//!
//! An earlier draft shipped `impl<T: AdminModel> ModelAdmin for T {}`
//! so every derived `AdminModel` would auto-pick-up the defaults.
//! That collides with Rust's coherence rules — without
//! `feature(specialization)` (nightly-only), a blanket impl forbids
//! any per-type impl, which would block project overrides entirely.
//! The opt-in `impl ModelAdmin for X {}` is the standard stable-Rust
//! pattern (serde, axum, std).

use super::AdminModel;

// public:
/// One named group of fields on the change form. The framework's
/// default heuristic in [`super::render::form_ctx`] groups by name
/// (Default / System / Advanced); a project that wants explicit
/// section ordering returns a non-empty `&'static [Fieldset]` from
/// [`ModelAdmin::fieldsets`] and the renderer honours that instead.
#[derive(Debug, Clone)]
pub struct Fieldset {
    pub title: &'static str,
    pub fields: &'static [&'static str],
}

// public:
/// Django-style customisation surface for a registered admin model.
///
/// Every type that implements [`AdminModel`] gets a default impl via
/// the blanket below. Override the methods you care about; everything
/// else inherits sensible defaults.
pub trait ModelAdmin: AdminModel {
    /// Columns shown on the list page, in order. Default: every
    /// field declared on `AdminModel::FIELDS`.
    ///
    /// Returning `&[]` means "use the model's full field list" — the
    /// list page expands the empty default into `M::FIELDS`. Any
    /// non-empty slice replaces the defaults verbatim.
    fn list_display() -> &'static [&'static str] {
        &[]
    }

    /// Columns offered as filter chips in the sidebar. Default: none.
    fn list_filter() -> &'static [&'static str] {
        &[]
    }

    /// Columns searched by the list-page search box (case-insensitive
    /// substring match). Default: none.
    fn search_fields() -> &'static [&'static str] {
        &[]
    }

    /// Default ordering. `-foo` for `foo DESC`, `foo` for `foo ASC`.
    /// Multiple entries → multi-column ORDER BY in slice order.
    /// Default: `["-id"]` (newest first).
    fn ordering() -> &'static [&'static str] {
        &["-id"]
    }

    /// Rows per page on the list view. Default: 50.
    fn list_per_page() -> usize {
        50
    }

    /// Read-only fields on the change form. Default: none.
    fn readonly_fields() -> &'static [&'static str] {
        &[]
    }

    /// Field grouping on the change form. Default: empty — fall back
    /// to the framework heuristic (`Default` / `System` / `Advanced`).
    fn fieldsets() -> &'static [Fieldset] {
        &[]
    }

    /// Custom bulk actions surfaced as extra buttons in the list-view
    /// bulk bar (next to the framework's built-in Delete). Default:
    /// none.
    ///
    /// `BulkAction` is metadata only — the dispatcher
    /// (`AdminOps::execute_bulk_action`) is what actually runs the
    /// action on the selected rows. Project models that need a custom
    /// action override `AdminOps::execute_bulk_action` to match on
    /// `name` and apply the work; the framework's default impl
    /// returns a clear `BadRequest` for any name it doesn't recognise,
    /// so a forgotten implementation surfaces as an error page rather
    /// than a silent no-op.
    fn bulk_actions() -> &'static [BulkAction] {
        &[]
    }
}

// public:
/// One project-defined bulk action declared by
/// [`ModelAdmin::bulk_actions`]. Static metadata only — see
/// `AdminOps::execute_bulk_action` for the runtime dispatcher.
#[derive(Debug, Clone, Copy)]
pub struct BulkAction {
    /// Stable URL slug. Routed at `POST /admin/:model/bulk/:name`.
    /// Use snake_case identifiers; the framework reserves `delete`
    /// for its built-in cascade-aware delete (handled separately at
    /// `/bulk_delete`).
    pub name: &'static str,
    /// Human-readable button label. Rendered as-is in the bulk bar
    /// and on the confirmation page header.
    pub label: &'static str,
    /// `true` → render the button with the framework's destructive
    /// (red) styling. Use for actions that lose data or change state
    /// in a hard-to-undo way.
    pub destructive: bool,
    /// `true` → POST shows a confirmation page first listing every
    /// selected row; the user must click again to commit. `false` →
    /// execute on the first POST. Default in the recommended call
    /// pattern is `true` for any action a user might regret.
    pub confirm: bool,
}

// public:
/// One column to sort by, with direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    // public:
    /// Stable SQL fragment.
    pub fn sql(self) -> &'static str {
        match self {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        }
    }
}

// public:
/// Parse one `ordering()` slice entry. `"-foo"` → (`"foo"`, Desc);
/// `"foo"` → (`"foo"`, Asc).
pub fn parse_order_spec(spec: &str) -> (String, SortDir) {
    if let Some(rest) = spec.strip_prefix('-') {
        (rest.to_string(), SortDir::Desc)
    } else {
        (spec.to_string(), SortDir::Asc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_order_spec_handles_leading_minus() {
        assert_eq!(parse_order_spec("-id"), ("id".to_string(), SortDir::Desc));
        assert_eq!(parse_order_spec("name"), ("name".to_string(), SortDir::Asc));
    }

    #[test]
    fn sort_dir_sql_is_stable() {
        assert_eq!(SortDir::Asc.sql(), "ASC");
        assert_eq!(SortDir::Desc.sql(), "DESC");
    }
}
