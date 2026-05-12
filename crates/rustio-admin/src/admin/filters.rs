//! Field classification + filter inference (renamed from
//! `intelligence` per Section 4 of the strategic reset plan).
//!
//! Pure helpers that turn *schema metadata* into *user-facing hints*:
//! the form-field label beside an input, the masked display of a
//! sensitive value on a list page, the filter dropdown inferred from
//! a `status` column. Nothing in this module touches the filesystem,
//! the database, or produces HTML — it returns structured data that
//! the admin renderer consumes.
//!
//! ## Public API
//!
//! - [`classify_field`] — labels a field by role (`Id`, `Email`,
//!   `Status`, …). Every downstream renderer branches on this enum.
//! - [`field_ui_metadata`] — packages the label, placeholder, hint,
//!   and sensitivity marker a form needs to render one input.
//! - [`infer_filters`] — walks a model's fields and decides which
//!   filters make sense on its list page.
//! - [`format_relation_cell`] — render a foreign-key cell on the list
//!   page as `Target #42` (or `42` when the target name is unknown).
//! - [`mask_pii`] — deterministic string masker used to hide personal
//!   data by default on list views.
//!
//! Slimmed for Tier 1: the legacy module's country/industry/GDPR
//! `ContextConfig` plumbing was wired through `crate::ai` (Tier 2) so
//! it's been removed; classification is now shape-based only.
//! `classify_search` / `SearchIntent` are dropped per Section 3.

use crate::admin::{AdminField, FieldType};

// public:
/// The role a field plays in the admin UI. One field maps to exactly
/// one role; the ordering of branches in [`classify_field`] resolves
/// overlaps (e.g. an `email` column is `FieldRole::Email`, not
/// `FieldRole::PlainText`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRole {
    /// Primary key. Rendered monospace, excluded from edit forms.
    Id,
    /// `DateTime<Utc>`-shaped columns.
    Timestamp,
    /// Booleans — rendered as a pill on the list page, a checkbox in forms.
    Bool,
    /// Numeric values that aren't identifiers.
    NumericCount,
    /// `<something>_id` integer column that points at another model.
    ForeignKey,
    /// A `status` / `*_status` column.
    Status,
    /// An email address. Masked by default on list views.
    Email,
    /// A phone number. Masked by default.
    Phone,
    /// Everything else.
    PlainText,
}

impl FieldRole {
    // public:
    /// `true` when the role carries personal data and should be masked
    /// by default on list views.
    pub fn is_sensitive(self) -> bool {
        matches!(self, FieldRole::Email | FieldRole::Phone)
    }
}

// public:
/// Everything a form / list renderer needs to present one field to a
/// human. All strings are plain text (no HTML) — the caller escapes
/// before emitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldUI {
    pub role: FieldRole,
    pub label: String,
    pub placeholder: Option<String>,
    pub hint: Option<String>,
    /// `true` when the field should carry the lock marker and (for
    /// list views) be masked by default.
    pub sensitive: bool,
    /// One-line explanation of *why* the field is sensitive.
    pub sensitivity_note: Option<String>,
    /// Set when the field is a FK to a known model. Carries the
    /// *singular* display name of the target (e.g. `"Applicant"`).
    pub relation_label: Option<String>,
}

// public:
/// What shape of filter the admin list page should render for a given
/// field.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterKind {
    /// `<select>` over distinct string values.
    DropdownText,
    /// Yes / No dropdown over a boolean column.
    BoolYesNo,
    /// Two `date` / `datetime-local` inputs bounding a range.
    DateRange,
    /// Numeric exact-match input.
    NumericExact,
    /// Single-line input, compared exactly.
    ExactMatch,
    /// `<select>` populated by the admin runtime from rows of the
    /// target model.
    RelationSelect { target_model: String },
}

// public:
/// One filter the list page should show for a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterDef {
    pub field: String,
    pub label: String,
    pub kind: FilterKind,
}

// ---------------------------------------------------------------------------
// classify_field
// ---------------------------------------------------------------------------

// public:
/// Assign a [`FieldRole`] to one field.
///
/// Order of precedence (highest first):
/// 1. Shape: `id`, `email`, `phone`, `status`, bool, datetime, FK, numeric.
/// 2. Fallback: `PlainText`.
pub fn classify_field(f: &AdminField) -> FieldRole {
    let name = f.name;
    if name == "id" {
        return FieldRole::Id;
    }
    if name == "email" {
        return FieldRole::Email;
    }
    if name == "phone" {
        return FieldRole::Phone;
    }
    if matches!(f.field_type, FieldType::Bool) {
        return FieldRole::Bool;
    }
    if matches!(
        f.field_type,
        FieldType::DateTime | FieldType::OptionalDateTime
    ) {
        return FieldRole::Timestamp;
    }
    if name == "status" || name.ends_with("_status") {
        return FieldRole::Status;
    }
    // FK only when the column is integer-typed. A String column
    // ending in `_id` is an opaque identifier, not a FK.
    if name.ends_with("_id") && matches!(f.field_type, FieldType::I32 | FieldType::I64) {
        return FieldRole::ForeignKey;
    }
    if matches!(f.field_type, FieldType::I32 | FieldType::I64) {
        return FieldRole::NumericCount;
    }
    FieldRole::PlainText
}

// ---------------------------------------------------------------------------
// field_ui_metadata
// ---------------------------------------------------------------------------

// public:
/// Package a field's display metadata for the admin form / list
/// renderers. All strings are plain text — escape before emitting.
pub fn field_ui_metadata(f: &AdminField) -> FieldUI {
    let role = classify_field(f);
    let label = humanise(f.name);
    let mut placeholder: Option<String> = None;
    let mut hint: Option<String> = None;
    let mut sensitive = false;
    let mut sensitivity_note: Option<String> = None;

    match role {
        FieldRole::Email => {
            placeholder = Some("name@example.com".into());
        }
        FieldRole::Phone => {
            placeholder = Some("+1 555 123 4567".into());
        }
        FieldRole::Timestamp => {
            placeholder = Some("YYYY-MM-DDTHH:MM".into());
            hint = Some("Interpreted as UTC.".into());
        }
        FieldRole::Status => {
            hint = Some("Short status label (e.g. active, pending, resolved).".into());
        }
        FieldRole::ForeignKey => {
            hint = Some("Foreign-key id — must reference an existing row.".into());
        }
        FieldRole::Id | FieldRole::Bool | FieldRole::NumericCount | FieldRole::PlainText => {}
    }

    // Name-based UI hints applied AFTER role classification.
    if f.name == "slug" {
        placeholder = Some("my-post-title".into());
        hint = Some("URL-friendly identifier".into());
    }

    if role.is_sensitive() {
        sensitive = true;
        sensitivity_note = Some("Personal data.".into());
    }

    FieldUI {
        role,
        label,
        placeholder,
        hint,
        sensitive,
        sensitivity_note,
        relation_label: None,
    }
}

// public:
/// Like [`field_ui_metadata`] but relation-aware. Pass the singular
/// display name of the target model when the schema records a relation
/// for this field; the returned [`FieldUI`] then carries
/// `relation_label` and a hint of the form "Foreign key to Target".
pub fn field_ui_metadata_with_relation(f: &AdminField, relation_target: Option<&str>) -> FieldUI {
    let mut ui = field_ui_metadata(f);
    if let Some(target) = relation_target.filter(|t| !t.is_empty()) {
        // A known relation always renders as ForeignKey even if the
        // column name wouldn't hit the `_id` heuristic.
        ui.role = FieldRole::ForeignKey;
        ui.relation_label = Some(target.to_string());
        ui.hint = Some(format!("Foreign key to {target}."));
    }
    ui
}

// public:
/// Render "Target #42" for a foreign-key cell on a list view. Falls
/// back to the raw id when the caller doesn't have a target name.
pub fn format_relation_cell(id: i64, target: Option<&str>) -> String {
    match target {
        Some(t) if !t.is_empty() => format!("{t} #{id}"),
        _ => id.to_string(),
    }
}

// ---------------------------------------------------------------------------
// infer_filters
// ---------------------------------------------------------------------------

// public:
/// Infer the filter controls for a model's list page from its fields.
/// Order follows the order of `fields`; every filter references a
/// field that actually exists on the model.
pub fn infer_filters(fields: &[AdminField]) -> Vec<FilterDef> {
    infer_filters_with_relations(fields, |_| None)
}

// public:
/// Like [`infer_filters`] but invokes `relation_target_of` for each
/// field to detect relation columns. If the callback returns
/// `Some(target)`, the filter is emitted as
/// [`FilterKind::RelationSelect`] instead of the numeric-exact fallback.
pub fn infer_filters_with_relations<F>(
    fields: &[AdminField],
    relation_target_of: F,
) -> Vec<FilterDef>
where
    F: Fn(&AdminField) -> Option<String>,
{
    let mut out: Vec<FilterDef> = Vec::new();
    for f in fields {
        if f.name == "id" {
            continue;
        }
        let role = classify_field(f);
        let kind = match role {
            FieldRole::Status => FilterKind::DropdownText,
            FieldRole::Bool => FilterKind::BoolYesNo,
            FieldRole::Timestamp => FilterKind::DateRange,
            FieldRole::NumericCount => FilterKind::NumericExact,
            FieldRole::ForeignKey => match relation_target_of(f) {
                Some(target_model) if !target_model.is_empty() => {
                    FilterKind::RelationSelect { target_model }
                }
                _ => FilterKind::NumericExact,
            },
            // Plain text, email, phone — no stock filter.
            _ => continue,
        };
        out.push(FilterDef {
            field: f.name.to_string(),
            label: humanise(f.name),
            kind,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// mask_pii
// ---------------------------------------------------------------------------

// public:
/// Produce a masked display string for a sensitive value. Keeps the
/// first few characters so a reviewer can tell which row they're
/// looking at, replaces the rest with `•`. Length of the output
/// matches the input. Deterministic, Unicode-safe.
pub fn mask_pii(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = value.chars().collect();
    let n = chars.len();
    let keep = (n / 3).clamp(2, 4).min(n);
    let mut out = String::with_capacity(n);
    for (i, c) in chars.iter().enumerate() {
        if i < keep {
            out.push(*c);
        } else {
            out.push('•');
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn humanise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut next_upper = true;
    for ch in s.chars() {
        if ch == '_' {
            out.push(' ');
            next_upper = true;
        } else if next_upper {
            out.push(ch.to_ascii_uppercase());
            next_upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &'static str, ty: FieldType) -> AdminField {
        AdminField {
            name,
            label: name,
            field_type: ty,
            editable: true,
            relation: None,
            choices: None,
        }
    }

    #[test]
    fn classify_id_email_status_bool_timestamp() {
        assert_eq!(classify_field(&field("id", FieldType::I64)), FieldRole::Id);
        assert_eq!(
            classify_field(&field("email", FieldType::String)),
            FieldRole::Email
        );
        assert_eq!(
            classify_field(&field("status", FieldType::String)),
            FieldRole::Status
        );
        assert_eq!(
            classify_field(&field("order_status", FieldType::String)),
            FieldRole::Status
        );
        assert_eq!(
            classify_field(&field("active", FieldType::Bool)),
            FieldRole::Bool
        );
        assert_eq!(
            classify_field(&field("created_at", FieldType::DateTime)),
            FieldRole::Timestamp
        );
    }

    #[test]
    fn fk_only_for_integer_id_columns() {
        assert_eq!(
            classify_field(&field("user_id", FieldType::I64)),
            FieldRole::ForeignKey
        );
        // String column ending _id is opaque, not FK.
        assert_eq!(
            classify_field(&field("national_id", FieldType::String)),
            FieldRole::PlainText
        );
    }

    #[test]
    fn infer_filters_skips_id_and_picks_kinds() {
        let fields = vec![
            field("id", FieldType::I64),
            field("status", FieldType::String),
            field("active", FieldType::Bool),
            field("created_at", FieldType::DateTime),
            field("title", FieldType::String),
        ];
        let filters = infer_filters(&fields);
        assert_eq!(filters.len(), 3);
        assert!(matches!(filters[0].kind, FilterKind::DropdownText));
        assert!(matches!(filters[1].kind, FilterKind::BoolYesNo));
        assert!(matches!(filters[2].kind, FilterKind::DateRange));
    }

    #[test]
    fn mask_pii_keeps_prefix_and_replaces_with_bullets() {
        assert_eq!(mask_pii("alice@example.com"), "alic•••••••••••••");
        assert_eq!(mask_pii(""), "");
    }

    #[test]
    fn relation_label_overrides_role() {
        let f = field("user_id", FieldType::I64);
        let ui = field_ui_metadata_with_relation(&f, Some("User"));
        assert_eq!(ui.role, FieldRole::ForeignKey);
        assert_eq!(ui.relation_label.as_deref(), Some("User"));
        assert!(ui.hint.unwrap().contains("Foreign key to User"));
    }

    #[test]
    fn format_relation_cell_with_and_without_target() {
        assert_eq!(format_relation_cell(42, Some("User")), "User #42");
        assert_eq!(format_relation_cell(42, None), "42");
        assert_eq!(format_relation_cell(42, Some("")), "42");
    }
}
