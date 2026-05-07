//! Template context builders. Every piece of data the admin templates
//! need comes from here as a `serde::Serialize` struct. No HTML lives
//! in Rust code.
//!
//! Every page context embeds [`BaseContext`] via `#[serde(flatten)]`
//! so every template gets uniform access to `identity`, `csrf_token`,
//! site branding, and the active theme palette.
//!
//! Slimmed for Tier 1 P6: the legacy file's password-change /
//! user-new-edit / group-new-edit form sections, the developer-stub
//! "coming soon" context, the schema/search helpers, and the audit
//! history page contexts have been removed. They re-land with
//! `admin/builtin.rs` in P6.b.

#![allow(dead_code)]

use std::collections::HashMap;

use serde::Serialize;

use super::audit::AdminAction;
use super::types::{Admin, AdminEntry, AdminField, EditRow, ListRow};
use crate::auth::Identity;
use crate::error::Result;
use crate::http::FormData;
use crate::orm::Db;

#[derive(Serialize)]
pub(crate) struct IdentityCtx {
    pub email: String,
    pub is_admin: bool,
    pub is_developer: bool,
}

impl From<&Identity> for IdentityCtx {
    fn from(i: &Identity) -> Self {
        Self {
            email: i.email.clone(),
            is_admin: i.is_admin(),
            is_developer: i.is_active && i.role.includes(crate::auth::Role::Developer),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct BaseContext {
    pub identity: Option<IdentityCtx>,
    pub csrf_token: String,
    pub site_title: String,
    pub site_header: String,
    pub index_title: String,
    pub footer_copyright: String,
    /// `true` when the active session belongs to a demo user (`is_demo`
    /// column on `rustio_users`). Templates use this to render the red
    /// banner above the page content.
    pub is_demo_session: bool,
    pub demo_label: Option<String>,
    /// `true` when the active `AdminTheme` patch sets at least one
    /// field. Templates use it to skip emitting the inline `<style>`
    /// block entirely when no overrides are configured — the framework
    /// stylesheet is then the single source of truth.
    pub has_theme_overrides: bool,
    /// Accent colour in `#rrggbb` form, only `Some` when the project
    /// patched it. `None` means *no override — admin.css owns it*.
    pub accent_hex: Option<String>,
    /// Same colour as a space-separated RGB triplet (`"30 107 168"`)
    /// for use inside `rgb(... / opacity)` expressions. `None` paired
    /// with `accent_hex == None`.
    pub accent_rgb: Option<String>,
    pub theme_bg: Option<String>,
    pub theme_surface: Option<String>,
    pub theme_text: Option<String>,
    pub theme_text_muted: Option<String>,
    pub theme_border: Option<String>,
}

/// Convert an `#rrggbb` (or `rrggbb`) hex string into the
/// space-separated RGB-triplet form CSS variables expect (`160 52 26`
/// for `#A0341A`). On any parse failure returns the framework default
/// accent RGB so the admin chrome never breaks over a config typo.
pub(crate) fn hex_to_rgb_triplet(hex: &str) -> String {
    const FALLBACK: &str = "160 52 26"; // #A0341A — framework default crimson
    let h = hex.trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return FALLBACK.into();
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(160);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(52);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(26);
    format!("{r} {g} {b}")
}

impl BaseContext {
    pub fn new(identity: Option<&Identity>, csrf_token: String, admin: &Admin) -> Self {
        let b = admin.branding();
        let (is_demo_session, demo_label) = match identity {
            Some(i) => (i.is_demo, i.demo_label.clone()),
            None => (false, None),
        };
        let theme = admin.active_theme();
        let accent_hex = theme.accent.clone();
        let accent_rgb = accent_hex.as_deref().map(hex_to_rgb_triplet);
        Self {
            identity: identity.map(IdentityCtx::from),
            csrf_token,
            site_title: b.site_title.clone(),
            site_header: b.site_header.clone(),
            index_title: b.index_title.clone(),
            footer_copyright: b.footer_copyright.clone(),
            is_demo_session,
            demo_label,
            has_theme_overrides: theme.has_overrides(),
            accent_hex,
            accent_rgb,
            theme_bg: theme.bg.clone(),
            theme_surface: theme.surface.clone(),
            theme_text: theme.text.clone(),
            theme_text_muted: theme.text_muted.clone(),
            theme_border: theme.border.clone(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct SidebarEntry {
    pub admin_name: &'static str,
    pub display_name: &'static str,
}

impl From<&AdminEntry> for SidebarEntry {
    fn from(e: &AdminEntry) -> Self {
        Self {
            admin_name: e.admin_name,
            display_name: e.display_name,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct FlashCtx {
    pub kind: &'static str,
    pub message: String,
}

// ---- Login -----------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct LoginCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub error: Option<String>,
    pub sections: Vec<FormSection>,
    pub flash: Option<FlashCtx>,
}

/// Pre-built FormField list for the login form. Static because the
/// values never change between requests; built once and cloned.
pub(crate) fn login_form_sections() -> Vec<FormSection> {
    vec![FormSection {
        title: None,
        fields: vec![
            FormField {
                name: "email",
                label: "Email".to_string(),
                widget: "input",
                input_type: "email",
                value: String::new(),
                hint: None,
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 1,
                autocomplete: Some("username"),
                autofocus: true,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
            FormField {
                name: "password",
                label: "Password".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: None,
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 1,
                autocomplete: Some("current-password"),
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
        ],
    }]
}

// ---- Dashboard ------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct DashboardCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub entries: Vec<SidebarEntry>,
    pub apps: Vec<DashboardApp>,
    pub recent_actions: Vec<RecentActionCtx>,
    pub flash: Option<FlashCtx>,
}

#[derive(Serialize)]
pub(crate) struct DashboardApp {
    pub label: String,
    pub models: Vec<DashboardModel>,
}

#[derive(Serialize)]
pub(crate) struct DashboardModel {
    pub admin_name: &'static str,
    pub display_name: &'static str,
    pub field_count: usize,
}

#[derive(Serialize)]
pub(crate) struct RecentActionCtx {
    pub action_type: String,
    pub label: &'static str,
    pub pill_class: &'static str,
    pub model_name: String,
    pub object_id: i64,
    pub user_email: String,
    pub summary: String,
    pub when_relative: String,
}

/// Group every `AdminEntry` by `app_label` derived from `admin_name`.
///
/// Convention: if `admin_name` contains a `.`, the prefix is the app
/// label (`"tolkhuset.translators"` → label `"Tolkhuset"`); the
/// remaining path is the model slug. Otherwise the whole `admin_name`
/// becomes a single-app label, capitalised.
pub(crate) fn group_entries_by_app(entries: &[AdminEntry]) -> Vec<DashboardApp> {
    let mut apps: Vec<DashboardApp> = Vec::new();
    for entry in entries {
        // Core entries (the synthetic User) have a bespoke admin page;
        // listing them here would offer Add/Change actions that route
        // through `CoreUserOps`, which is route-only — those would 500.
        if entry.core {
            continue;
        }
        let label = app_label_for(entry.admin_name);
        let app = match apps.iter_mut().find(|a| a.label == label) {
            Some(a) => a,
            None => {
                apps.push(DashboardApp {
                    label: label.clone(),
                    models: Vec::new(),
                });
                apps.last_mut().unwrap()
            }
        };
        app.models.push(DashboardModel {
            admin_name: entry.admin_name,
            display_name: entry.display_name,
            field_count: entry.fields.len(),
        });
    }
    apps
}

pub(crate) fn app_label_for(admin_name: &str) -> String {
    let prefix = admin_name.split('.').next().unwrap_or(admin_name);
    capitalise(prefix)
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub(crate) fn dashboard_ctx(
    identity: &Identity,
    admin: &Admin,
    recent_actions: Vec<AdminAction>,
    csrf_token: String,
) -> DashboardCtx {
    let recent = recent_actions
        .into_iter()
        .map(|a| RecentActionCtx {
            action_type: a.action_type.clone(),
            label: action_label(&a.action_type),
            pill_class: action_pill_class(&a.action_type),
            model_name: a.model_name,
            object_id: a.object_id,
            user_email: a.user_email.unwrap_or_else(|| "—".to_string()),
            summary: a.summary,
            when_relative: relative_time(a.timestamp),
        })
        .collect();

    DashboardCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        apps: group_entries_by_app(admin.entries()),
        recent_actions: recent,
        flash: None,
    }
}

fn action_label(action_type: &str) -> &'static str {
    match action_type {
        "create" => "Created",
        "update" => "Changed",
        "delete" => "Deleted",
        _ => "Action",
    }
}

fn action_pill_class(action_type: &str) -> &'static str {
    match action_type {
        "create" => "badge-success",
        "update" => "badge-neutral",
        "delete" => "badge-danger",
        _ => "badge-neutral",
    }
}

pub(crate) fn relative_time(ts: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let delta = now - ts;
    if delta.num_seconds() < 60 {
        "just now".to_string()
    } else if delta.num_minutes() < 60 {
        format!("{}m ago", delta.num_minutes())
    } else if delta.num_hours() < 24 {
        format!("{}h ago", delta.num_hours())
    } else if delta.num_days() < 30 {
        format!("{}d ago", delta.num_days())
    } else {
        ts.format("%Y-%m-%d").to_string()
    }
}

// ---- Changelist (list page) ----------------------------------------------

#[derive(Serialize)]
pub(crate) struct ListField {
    pub name: String,
    pub label: String,
    /// `FieldType::widget()`'s output: `"text"` / `"number"` /
    /// `"checkbox"` / `"datetime"`. The list template dispatches on
    /// this rather than duck-typing on the cell's string shape.
    pub kind: &'static str,
    /// Sort hint for sortable column headers in `list.html`.
    /// `"asc"` → header link toggles to descending;
    /// `"desc"` → header link clears the sort (back to default);
    /// empty → header link sets ascending.
    pub sort_active: &'static str,
    /// URL the sortable header link points to. Pre-baked here so the
    /// template doesn't need to reproduce the toggle logic.
    pub sort_link: String,
}

#[derive(Serialize)]
pub(crate) struct ListCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub admin_name: &'static str,
    pub display_name: &'static str,
    pub singular_name: &'static str,
    pub fields: Vec<ListField>,
    pub rows: Vec<ListRowCtx>,
    pub search_query: String,
    pub filters: Vec<FilterGroupCtx>,
    /// Count of filter groups whose user-selected value is non-empty.
    /// Drives the "Filters (N)" badge on the toolbar dropdown toggle.
    pub active_filter_count: usize,
    pub page: usize,
    pub total_pages: usize,
    pub per_page: usize,
    pub total_rows: usize,
    /// Whether the bulk-action UI should render. Always `false` until
    /// the bulk-action POST endpoint is wired in a later phase.
    pub bulk_actions_enabled: bool,
    pub flash: Option<FlashCtx>,
}

/// `values` is flattened into the JSON object so template code can do
/// `row[field.name]` (minijinja resolves dict subscript on the merged
/// map). The explicit `id: i64` field stays out of the flattened map.
#[derive(Serialize)]
pub(crate) struct ListRowCtx {
    pub id: i64,
    #[serde(flatten)]
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub(crate) struct FilterGroupCtx {
    pub field: String,
    pub label: String,
    pub options: Vec<FilterOptionCtx>,
    pub current: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct FilterOptionCtx {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn list_ctx(
    identity: &Identity,
    admin: &Admin,
    entry: &AdminEntry,
    rows: Vec<ListRow>,
    search_query: String,
    filters: Vec<FilterGroupCtx>,
    page: usize,
    per_page: usize,
    total_rows: usize,
    // `active_sort = (column, direction)` carries the parsed override
    // from `?sort=&dir=`. `None` means the model's
    // `ModelAdmin::ordering()` default is in effect — sortable
    // header links still render, but no column gets the active arrow.
    active_sort: Option<(String, super::modeladmin::SortDir)>,
    csrf_token: String,
) -> ListCtx {
    let total_pages = total_rows.div_ceil(per_page.max(1)).max(1);

    // Honour `ModelAdmin::list_display()`: when non-empty, render only
    // those columns (in the declared order). Empty falls back to every
    // model field. This is the contract documented on
    // `AdminEntry::list_display`; previously the renderer iterated
    // over `entry.fields` unconditionally and showed every column,
    // including bulky `body` / `description` fields the model author
    // had explicitly excluded.
    let visible_fields: Vec<&AdminField> = if entry.list_display.is_empty() {
        entry.fields.iter().collect()
    } else {
        entry
            .list_display
            .iter()
            .filter_map(|name| entry.fields.iter().find(|f| f.name == *name))
            .collect()
    };
    let fields: Vec<ListField> = visible_fields
        .iter()
        .map(|f| {
            let (sort_active, sort_link) = build_sort_link(f.name, &active_sort);
            ListField {
                name: f.name.to_string(),
                label: f.label.to_string(),
                kind: f.field_type.widget(),
                sort_active,
                sort_link,
            }
        })
        .collect();
    let field_names: Vec<&'static str> = entry.fields.iter().map(|f| f.name).collect();
    let field_types: Vec<crate::admin::FieldType> =
        entry.fields.iter().map(|f| f.field_type).collect();
    ListCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: entry.display_name.to_string(),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        admin_name: entry.admin_name,
        display_name: entry.display_name,
        singular_name: entry.singular_name,
        fields,
        rows: rows
            .into_iter()
            .map(|r| {
                let mut values: HashMap<String, serde_json::Value> =
                    HashMap::with_capacity(field_names.len().saturating_sub(1));
                for (i, cell) in r.cells.into_iter().enumerate() {
                    if let Some(name) = field_names.get(i) {
                        // Skip the "id" key so the explicit struct field
                        // wins on serialization.
                        if *name == "id" {
                            continue;
                        }
                        let typed = match field_types.get(i) {
                            Some(crate::admin::FieldType::Bool) => {
                                serde_json::Value::Bool(cell == "true")
                            }
                            _ => serde_json::Value::String(cell),
                        };
                        values.insert((*name).to_string(), typed);
                    }
                }
                ListRowCtx { id: r.id, values }
            })
            .collect(),
        search_query,
        active_filter_count: filters.iter().filter(|g| g.current.is_some()).count(),
        filters,
        page,
        total_pages,
        per_page,
        total_rows,
        bulk_actions_enabled: false,
        flash: None,
    }
}

/// Pre-bake the sortable-header URL + active-direction marker for one
/// column. Three states:
///   - column is the current sort, ascending  → click toggles to desc
///   - column is the current sort, descending → click clears the sort
///   - column is not the current sort         → click sets ascending
fn build_sort_link(
    name: &'static str,
    active: &Option<(String, super::modeladmin::SortDir)>,
) -> (&'static str, String) {
    use super::modeladmin::SortDir;
    match active {
        Some((col, SortDir::Asc)) if col == name => ("asc", format!("?sort={name}&dir=desc")),
        Some((col, SortDir::Desc)) if col == name => ("desc", String::from("?")),
        _ => ("", format!("?sort={name}&dir=asc")),
    }
}

// ---- Change form ----------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct FormCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub admin_name: &'static str,
    pub display_name: &'static str,
    pub singular_name: &'static str,
    pub mode: &'static str, // "new" or "edit"
    pub object_id: Option<i64>,
    pub sections: Vec<FormSection>,
    pub errors: Vec<String>,
    pub flash: Option<FlashCtx>,
}

/// One option in a `<select>` list. Both fields are `String` because
/// options come from runtime data: enum choices, FK rows, M2M
/// memberships.
#[derive(Serialize, Clone)]
pub(crate) struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Serialize)]
pub(crate) struct FormField {
    pub name: &'static str,
    pub label: String,
    pub widget: &'static str,
    pub input_type: &'static str,
    pub value: String,
    pub hint: Option<String>,
    pub placeholder: Option<String>,
    pub required: bool,
    pub options: Option<Vec<SelectOption>>,
    pub multiple: bool,
    /// Grid-span hint. `1` (default) renders the field at half-width
    /// inside the section's `grid-cols-2`; `2` makes the field span
    /// both columns. Set to `2` for textareas, `1` everywhere else.
    pub span: u8,
    pub autocomplete: Option<&'static str>,
    pub autofocus: bool,
    pub disabled: bool,
    pub maxlength: Option<u16>,
    pub searchable: bool,
    pub has_more: bool,
    pub search_url: Option<String>,
    pub errors: Vec<String>,
    pub target_model: Option<String>,
    /// Computed checked-state for boolean fields, normalised once at
    /// FormField construction time using the same rules as
    /// `FormData::bool_flag` (`on` / `true` / `1` / `yes`).
    pub checked: bool,
}

/// One logical group of fields on a form. `title: None` renders
/// without an `<h3>` (used for the default "core fields" section).
#[derive(Serialize)]
pub(crate) struct FormSection {
    pub title: Option<&'static str>,
    pub fields: Vec<FormField>,
}

/// Snake-case → Title Case ("priority" → "Priority", "is_active" → "Is active").
///
/// Mirrors `rustio_admin_macros::humanise_field` byte-for-byte. The
/// macro emits validation messages prefixed with this transformed
/// label (`"Title is required."`); `bucket_errors_by_label` reverses
/// the mapping at runtime to route flat errors to their owning field.
fn humanise_field(s: &str) -> String {
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

/// Split a flat `Vec<String>` from `AdminOps::create / update` into a
/// global vec + a per-field map by prefix-matching against each
/// editable field's humanised label.
///
/// **Brittle by design.** Depends on `rustio-admin-macros` emitting
/// messages of the form `"<HumanisedLabel> ..."`. If the macro ever
/// changes that wording, unmatched errors fall through to the global
/// vec — the banner still shows them; only the inline / aria
/// attribution is lost.
pub(crate) fn bucket_errors_by_label(
    entry: &AdminEntry,
    errors: Vec<String>,
) -> (Vec<String>, HashMap<String, Vec<String>>) {
    // Pre-compute "<Label> " once per editable field. The trailing
    // space disambiguates `Title ` from `Title bar `.
    let labels: Vec<(&'static str, String)> = entry
        .fields
        .iter()
        .filter(|f| f.editable)
        .map(|f| (f.name, format!("{} ", humanise_field(f.name))))
        .collect();

    let mut global: Vec<String> = Vec::new();
    let mut per_field: HashMap<String, Vec<String>> = HashMap::new();
    'outer: for err in errors {
        for (name, prefix) in &labels {
            if err.starts_with(prefix.as_str()) {
                per_field.entry((*name).to_string()).or_default().push(err);
                continue 'outer;
            }
        }
        global.push(err);
    }
    (global, per_field)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn form_ctx(
    identity: &Identity,
    admin: &Admin,
    entry: &AdminEntry,
    mode: &'static str,
    object_id: Option<i64>,
    existing: Option<&EditRow>,
    errors: Vec<String>,
    csrf_token: String,
    relation_options: HashMap<&'static str, (Vec<SelectOption>, bool)>,
    field_errors: HashMap<String, Vec<String>>,
    submitted: Option<&FormData>,
) -> FormCtx {
    let fields = entry
        .fields
        .iter()
        .filter(|f| f.editable)
        .map(|f| {
            let value = if let Some(form) = submitted {
                form.get(f.name).map(str::to_string).unwrap_or_default()
            } else {
                existing
                    .and_then(|row| {
                        row.values
                            .iter()
                            .find(|(col, _)| col == f.name)
                            .map(|(_, v)| v.clone())
                    })
                    .unwrap_or_default()
            };
            let ui = super::filters::field_ui_metadata(f);
            let (base_widget, input_type) = map_field_to_ui(f);
            // String fields with content-y names (body / description /
            // notes / content / summary) render as <textarea> instead
            // of single-line <input>. The base widget mapping doesn't
            // see field names, so the override stays here.
            let widget = if base_widget == "input"
                && matches!(
                    f.field_type,
                    super::types::FieldType::String | super::types::FieldType::OptionalString
                )
                && is_long_text_name(f.name)
            {
                "textarea"
            } else {
                base_widget
            };
            // Bools always submit (checked = true, absent = false), so
            // they never carry a required-asterisk; every other
            // non-nullable field does.
            let required =
                !f.field_type.nullable() && !matches!(f.field_type, super::types::FieldType::Bool);
            let (mut options, multiple, mut searchable, mut has_more) = if let Some(values) =
                f.choices
            {
                let mut opts: Vec<SelectOption> = Vec::with_capacity(values.len() + 1);
                if f.field_type.nullable() {
                    opts.push(SelectOption {
                        value: String::new(),
                        label: "—".to_string(),
                    });
                }
                opts.extend(values.iter().map(|v| SelectOption {
                    value: (*v).to_string(),
                    label: (*v).to_string(),
                }));
                (Some(opts), false, false, false)
            } else if let Some(rel) = &f.relation {
                let (opts, has_more) = relation_options.get(f.name).cloned().unwrap_or_default();
                (Some(opts), rel.multi, true, has_more)
            } else {
                (None, false, false, false)
            };

            // Synthesise a status select when no enum is declared.
            // UI-only — the underlying field stays a String. Triggers
            // when field.name == "status" and no choices/relation.
            let mut widget = widget;
            if f.name == "status" && options.is_none() {
                options = Some(vec![
                    SelectOption {
                        value: "draft".to_string(),
                        label: "draft".to_string(),
                    },
                    SelectOption {
                        value: "published".to_string(),
                        label: "published".to_string(),
                    },
                ]);
                searchable = false;
                has_more = false;
                widget = "select";
            }
            let span: u8 = if widget == "textarea" { 2 } else { 1 };
            let search_url = f
                .relation
                .as_ref()
                .map(|rel| format!("/admin/search/{}", rel.target_model));
            let target_model = f.relation.as_ref().map(|rel| rel.target_model.to_string());
            let checked = matches!(value.as_str(), "on" | "true" | "1" | "yes");
            let placeholder = if let Some(rel) = &f.relation {
                Some(format!("Select {}…", rel.target_model))
            } else {
                ui.placeholder
            };
            FormField {
                name: f.name,
                label: ui.label,
                widget,
                input_type,
                value,
                hint: ui.hint,
                placeholder,
                required,
                options,
                multiple,
                span,
                autocomplete: None,
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable,
                has_more,
                search_url,
                errors: field_errors.get(f.name).cloned().unwrap_or_default(),
                target_model,
                checked,
            }
        })
        .collect::<Vec<FormField>>();

    let sections = group_fields_into_sections(fields);

    FormCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: match mode {
            "new" => format!("Add {}", entry.singular_name),
            _ => format!("Change {}", entry.singular_name),
        },
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        admin_name: entry.admin_name,
        display_name: entry.display_name,
        singular_name: entry.singular_name,
        mode,
        object_id,
        sections,
        errors,
        flash: None,
    }
}

/// Apply a per-field error map to an existing `Vec<FormSection>` in
/// place. Used by bespoke validators that already know which field a
/// given error belongs to.
pub(crate) fn apply_field_errors(
    sections: &mut [FormSection],
    field_errors: &HashMap<String, Vec<String>>,
) {
    for section in sections.iter_mut() {
        for field in section.fields.iter_mut() {
            if let Some(errs) = field_errors.get(field.name) {
                field.errors = errs.clone();
            }
        }
    }
}

/// Partition the form's flat field list into Default / System /
/// Advanced sections by name heuristic. Empty sections are dropped.
fn group_fields_into_sections(fields: Vec<FormField>) -> Vec<FormSection> {
    let mut default_fields = Vec::new();
    let mut metadata_fields = Vec::new();
    let mut advanced_fields = Vec::new();

    for field in fields {
        match classify_field_section(field.name) {
            FieldSection::Default => default_fields.push(field),
            FieldSection::Metadata => metadata_fields.push(field),
            FieldSection::Advanced => advanced_fields.push(field),
        }
    }

    let mut sections: Vec<FormSection> = Vec::with_capacity(3);
    if !default_fields.is_empty() {
        sections.push(FormSection {
            title: None,
            fields: default_fields,
        });
    }
    if !metadata_fields.is_empty() {
        sections.push(FormSection {
            title: Some("System"),
            fields: metadata_fields,
        });
    }
    if !advanced_fields.is_empty() {
        sections.push(FormSection {
            title: Some("Advanced"),
            fields: advanced_fields,
        });
    }
    sections
}

enum FieldSection {
    Default,
    Metadata,
    Advanced,
}

fn classify_field_section(name: &str) -> FieldSection {
    if name.contains("created") || name.contains("updated") || name.contains("timestamp") {
        FieldSection::Metadata
    } else if matches!(name, "id" | "uuid" | "slug") {
        FieldSection::Advanced
    } else {
        FieldSection::Default
    }
}

/// Names that imply multi-line content. Used by `form_ctx` to upgrade
/// a `String` / `OptionalString` field to a `<textarea>`.
fn is_long_text_name(name: &str) -> bool {
    matches!(
        name,
        "body" | "description" | "notes" | "content" | "summary" | "bio" | "details"
    )
}

/// Backend-driven field-to-UI mapping. Resolution priority (top-down):
///   1. `field.choices.is_some()` → enum-style `<select>`.
///   2. `field.relation.is_some()` && `relation.multi` → `<select multiple>`.
///   3. `field.relation.is_some()` (belongs-to) → single `<select>`.
///   4. Fall through to the `field.field_type` mapping.
fn map_field_to_ui(field: &super::types::AdminField) -> (&'static str, &'static str) {
    if field.choices.is_some() {
        return ("select", "select");
    }
    if let Some(rel) = &field.relation {
        if rel.multi {
            return ("select", "select-multiple");
        }
        return ("select", "select");
    }
    use super::types::FieldType::*;
    match field.field_type {
        Bool => ("checkbox", "checkbox"),
        I32 | I64 | OptionalI64 => ("input", "number"),
        DateTime | OptionalDateTime => ("input", "datetime-local"),
        String | OptionalString => ("input", "text"),
    }
}

/// Initial-render row cap for FK / M2M selects.
pub(crate) const FK_OPTIONS_LIMIT: usize = 50;

/// Fetch real `<select>` options for every FK / M2M field on an
/// `AdminEntry`, keyed by the field's name.
///
/// Return value is `(Vec<SelectOption>, bool)` per key. The bool is
/// `has_more`: `true` when the relation had more rows than
/// `FK_OPTIONS_LIMIT` and the option list was truncated. Empty target
/// lists, missing target models, and non-relation fields all produce
/// a benign empty entry — never a panic.
///
/// The label for each option follows the resolution ladder:
///   1. `relation.display_field` if present and the column exists.
///   2. `"name"` column if present.
///   3. `"title"` column if present.
///   4. Stringified id.
pub(crate) async fn resolve_relation_options(
    admin: &Admin,
    entry: &AdminEntry,
    db: &Db,
) -> Result<HashMap<&'static str, (Vec<SelectOption>, bool)>> {
    let mut out: HashMap<&'static str, (Vec<SelectOption>, bool)> = HashMap::new();
    for f in entry.fields.iter() {
        let Some(rel) = &f.relation else {
            continue;
        };
        let target = admin.entries().iter().find(|e| {
            e.singular_name == rel.target_model
                || e.admin_name == rel.target_model
                || e.display_name == rel.target_model
        });
        let Some(target) = target else {
            out.insert(f.name, (Vec::new(), false));
            continue;
        };
        // Cap to FK_OPTIONS_LIMIT in SQL; the total count tells us
        // whether to set `has_more` for the form's "showing first N"
        // hint. Pre-P10 this called `list()` and slung every row over
        // the wire before truncating client-side.
        let page = target
            .ops
            .list(
                db,
                super::types::ListOpts {
                    limit: Some(FK_OPTIONS_LIMIT as i64),
                    ..super::types::ListOpts::default()
                },
            )
            .await?;
        let display_idx = pick_display_index(target.fields, rel.display_field);
        let opts: Vec<SelectOption> = page
            .rows
            .into_iter()
            .map(|r| {
                let label = display_idx
                    .and_then(|i| r.cells.get(i).cloned())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| r.id.to_string());
                SelectOption {
                    value: r.id.to_string(),
                    label,
                }
            })
            .collect();
        let has_more = page.total > FK_OPTIONS_LIMIT as i64;
        out.insert(f.name, (opts, has_more));
    }
    Ok(out)
}

fn pick_display_index(fields: &[AdminField], display_field: Option<&str>) -> Option<usize> {
    if let Some(preferred) = display_field {
        if let Some(i) = fields.iter().position(|f| f.name == preferred) {
            return Some(i);
        }
    }
    for fallback in ["name", "title"] {
        if let Some(i) = fields.iter().position(|f| f.name == fallback) {
            return Some(i);
        }
    }
    None
}

// ---- Confirm-delete -------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct ConfirmDeleteCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub admin_name: &'static str,
    pub singular_name: &'static str,
    pub object_id: i64,
    pub object_label: String,
    /// Models that point at this one via a `BelongsTo` FK.
    pub cascading: Vec<CascadeItem>,
    pub flash: Option<FlashCtx>,
}

#[derive(Serialize)]
pub(crate) struct CascadeItem {
    pub source_display_name: String,
    pub source_admin_name: String,
    pub source_field: String,
}

pub(crate) fn confirm_delete_ctx(
    identity: &Identity,
    admin: &Admin,
    entry: &AdminEntry,
    object_id: i64,
    object_label: String,
    cascading: Vec<CascadeItem>,
    csrf_token: String,
) -> ConfirmDeleteCtx {
    ConfirmDeleteCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: format!("Delete {}", entry.singular_name),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        admin_name: entry.admin_name,
        singular_name: entry.singular_name,
        object_id,
        object_label,
        cascading,
        flash: None,
    }
}

// ---- 403 Forbidden + generic admin error ---------------------------------

#[derive(Serialize)]
pub(crate) struct ForbiddenCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub entries: Vec<SidebarEntry>,
    pub page_title: &'static str,
    /// The permission codename or URL the user tried to reach.
    pub attempted: Option<String>,
    /// The minimum role required by the page that rejected them.
    pub required_role: Option<&'static str>,
}

#[derive(Serialize)]
pub(crate) struct ErrorCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub status: u16,
    pub heading: String,
    pub message: String,
}

pub(crate) fn admin_error_heading(status: u16) -> &'static str {
    match status {
        400 => "Bad request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not found",
        405 => "Method not allowed",
        409 => "Conflict",
        500 => "Server error",
        _ => "Error",
    }
}

pub(crate) fn render_admin_error_response(
    admin: &Admin,
    templates: &crate::templates::Templates,
    identity: Option<&Identity>,
    status: u16,
    message: String,
) -> crate::http::Response {
    let heading = admin_error_heading(status).to_string();
    let view = ErrorCtx {
        base: BaseContext::new(identity, String::new(), admin),
        page_title: format!("{status} {heading}"),
        status,
        heading: heading.clone(),
        message,
    };
    let html_status =
        hyper::StatusCode::from_u16(status).unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);
    match templates.render("admin/error.html", &view) {
        Ok(body) => crate::http::Response::html(body).with_status(html_status),
        Err(e) => {
            log::error!("admin/error.html render failed: {e}");
            crate::http::Response::text(format!("{status} {heading}: {}", view.message))
                .with_status(html_status)
        }
    }
}

pub(crate) fn render_forbidden_body(
    admin: &Admin,
    templates: &crate::templates::Templates,
    identity: &Identity,
    csrf_token: String,
    attempted: Option<String>,
    required_role: Option<&'static str>,
) -> crate::error::Result<String> {
    let view = ForbiddenCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        page_title: "Permission denied",
        attempted,
        required_role,
    };
    templates.render("admin/forbidden.html", &view)
}

// ---- History pages -------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct HistoryEntryCtx {
    pub timestamp_iso: String,
    pub when_relative: String,
    pub user_email: String,
    pub action_type: String,
    pub label: &'static str,
    pub pill_class: &'static str,
    pub model_name: String,
    pub model_admin_name: String,
    pub object_id: i64,
    pub summary: String,
    pub ip_address: String,
}

#[derive(Serialize)]
pub(crate) struct ObjectHistoryCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub admin_name: String,
    pub display_name: String,
    pub singular_name: String,
    pub object_id: i64,
    pub object_label: String,
    pub entries: Vec<HistoryEntryCtx>,
    pub flash: Option<FlashCtx>,
}

#[derive(Serialize)]
pub(crate) struct LogEntriesCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<HistoryEntryCtx>,
    pub flash: Option<FlashCtx>,
}

pub(crate) fn map_audit_actions(actions: Vec<AdminAction>) -> Vec<HistoryEntryCtx> {
    actions
        .into_iter()
        .map(|a| HistoryEntryCtx {
            timestamp_iso: a.timestamp.to_rfc3339(),
            when_relative: relative_time(a.timestamp),
            user_email: a.user_email.unwrap_or_else(|| "—".to_string()),
            label: action_label(&a.action_type),
            pill_class: action_pill_class(&a.action_type),
            model_name: a.model_name.clone(),
            // The audit row's `model_name` IS the admin_name slug per
            // the convention enforced at `audit::record` call sites.
            model_admin_name: a.model_name,
            action_type: a.action_type,
            object_id: a.object_id,
            summary: a.summary,
            ip_address: a.ip_address.unwrap_or_default(),
        })
        .collect()
}

// ---- Password change page -----------------------------------------------

#[derive(Serialize)]
pub(crate) struct PasswordChangeCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub errors: Vec<String>,
    pub success: bool,
    pub sections: Vec<FormSection>,
}

// ---- Bespoke form sections (used by admin/builtin.rs) -------------------

/// Role options for user_new / user_edit. Labels carry privilege
/// descriptions; values are the role slugs the auth layer expects.
pub(crate) fn role_select_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            value: "user".to_string(),
            label: "User (no admin access)".to_string(),
        },
        SelectOption {
            value: "staff".to_string(),
            label: "Staff (admin access; per-model group permissions)".to_string(),
        },
        SelectOption {
            value: "supervisor".to_string(),
            label: "Supervisor (view + edit; no destructive ops)".to_string(),
        },
        SelectOption {
            value: "administrator".to_string(),
            label: "Administrator (full coverage; bypasses group checks)".to_string(),
        },
        SelectOption {
            value: "developer".to_string(),
            label: "Developer (highest tier)".to_string(),
        },
    ]
}

/// FormField list for the user_new form. Two sections: Identity
/// (email + password) and Role (the 5-option select). Caller passes
/// the current values so re-render after validation failure preserves
/// them.
pub(crate) fn user_new_form_sections(email: &str, role: &str) -> Vec<FormSection> {
    vec![
        FormSection {
            title: Some("Identity"),
            fields: vec![
                FormField {
                    name: "email",
                    label: "Email".to_string(),
                    widget: "input",
                    input_type: "email",
                    value: email.to_string(),
                    hint: Some("Must be unique across all users.".to_string()),
                    placeholder: None,
                    required: true,
                    options: None,
                    multiple: false,
                    span: 2,
                    autocomplete: Some("off"),
                    autofocus: true,
                    disabled: false,
                    maxlength: None,
                    searchable: false,
                    has_more: false,
                    search_url: None,
                    errors: vec![],
                    target_model: None,
                    checked: false,
                },
                FormField {
                    name: "password",
                    label: "Password".to_string(),
                    widget: "input",
                    input_type: "password",
                    value: String::new(),
                    hint: Some(
                        "At least 8 characters. The user can change it later via Change password."
                            .to_string(),
                    ),
                    placeholder: None,
                    required: true,
                    options: None,
                    multiple: false,
                    span: 2,
                    autocomplete: Some("new-password"),
                    autofocus: false,
                    disabled: false,
                    maxlength: None,
                    searchable: false,
                    has_more: false,
                    search_url: None,
                    errors: vec![],
                    target_model: None,
                    checked: false,
                },
            ],
        },
        FormSection {
            title: Some("Role"),
            fields: vec![FormField {
                name: "role",
                label: "Role".to_string(),
                widget: "select",
                input_type: "select",
                value: role.to_string(),
                hint: Some(
                    "Higher roles include all lower-role capabilities. Group memberships are assigned on the next page after save."
                        .to_string(),
                ),
                placeholder: None,
                required: true,
                options: Some(role_select_options()),
                multiple: false,
                span: 2,
                autocomplete: None,
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            }],
        },
    ]
}

/// General section for group_new / group_edit. Two fields: name
/// (text, required, 150-char max) and description (textarea).
pub(crate) fn group_form_sections(name: &str, description: &str) -> Vec<FormSection> {
    vec![FormSection {
        title: Some("General"),
        fields: vec![
            FormField {
                name: "name",
                label: "Name".to_string(),
                widget: "input",
                input_type: "text",
                value: name.to_string(),
                hint: Some(
                    "A short identifier — letters, digits, dots and dashes only. Example: editors."
                        .to_string(),
                ),
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: Some("off"),
                autofocus: true,
                disabled: false,
                maxlength: Some(150),
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
            FormField {
                name: "description",
                label: "Description".to_string(),
                widget: "textarea",
                input_type: "text",
                value: description.to_string(),
                hint: Some("Optional. What this group is for.".to_string()),
                placeholder: None,
                required: false,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: None,
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
        ],
    }]
}

/// Identity section for user_edit. Email is disabled (read-only);
/// role is the select; is_active is the checkbox. Built per render
/// so values reflect the current row.
pub(crate) fn user_edit_identity_sections(
    email: &str,
    role: &str,
    is_active: bool,
) -> Vec<FormSection> {
    vec![FormSection {
        title: Some("Identity"),
        fields: vec![
            FormField {
                name: "email",
                label: "Email".to_string(),
                widget: "input",
                input_type: "email",
                value: email.to_string(),
                hint: Some(
                    "Email changes aren't exposed here — they require a full user update."
                        .to_string(),
                ),
                placeholder: None,
                required: false,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: None,
                autofocus: false,
                disabled: true,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
            FormField {
                name: "role",
                label: "Role".to_string(),
                widget: "select",
                input_type: "select",
                value: role.to_string(),
                hint: None,
                placeholder: None,
                required: true,
                options: Some(role_select_options()),
                multiple: false,
                span: 2,
                autocomplete: None,
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
            FormField {
                name: "is_active",
                label: "Active".to_string(),
                widget: "checkbox",
                input_type: "checkbox",
                value: if is_active {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
                hint: Some("Inactive users cannot sign in or hold sessions.".to_string()),
                placeholder: None,
                required: false,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: None,
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: is_active,
            },
        ],
    }]
}

/// Reset password section for user_edit. Single optional field;
/// blank → keep existing password.
pub(crate) fn user_edit_password_sections() -> Vec<FormSection> {
    vec![FormSection {
        title: Some("Reset password (optional)"),
        fields: vec![FormField {
            name: "new_password",
            label: "New password".to_string(),
            widget: "input",
            input_type: "password",
            value: String::new(),
            hint: Some("Leave blank to keep the current password unchanged.".to_string()),
            placeholder: None,
            required: false,
            options: None,
            multiple: false,
            span: 2,
            autocomplete: Some("new-password"),
            autofocus: false,
            disabled: false,
            maxlength: None,
            searchable: false,
            has_more: false,
            search_url: None,
            errors: vec![],
            target_model: None,
            checked: false,
        }],
    }]
}

/// Pre-built FormField list for the password-change form. Static; the
/// values are always empty (we never echo passwords back).
pub(crate) fn password_change_form_sections() -> Vec<FormSection> {
    vec![FormSection {
        title: None,
        fields: vec![
            FormField {
                name: "old_password",
                label: "Old password".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: None,
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: Some("current-password"),
                autofocus: true,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
            FormField {
                name: "new_password1",
                label: "New password".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: Some("Your password must contain at least 8 characters.".to_string()),
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: Some("new-password"),
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
            FormField {
                name: "new_password2",
                label: "Confirm".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: None,
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: Some("new-password"),
                autofocus: false,
                disabled: false,
                maxlength: None,
                searchable: false,
                has_more: false,
                search_url: None,
                errors: vec![],
                target_model: None,
                checked: false,
            },
        ],
    }]
}
