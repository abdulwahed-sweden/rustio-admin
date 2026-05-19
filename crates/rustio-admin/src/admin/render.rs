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
    /// Mirrors `Identity::mfa_enabled`. Surfaced into the topbar
    /// template so the chrome can pick between "Enable MFA" (un-
    /// enrolled) and "Two-factor" (already enrolled) links —
    /// `VISIBILITY_AUDIT.md` B1.
    pub mfa_enabled: bool,
}

impl From<&Identity> for IdentityCtx {
    fn from(i: &Identity) -> Self {
        Self {
            email: i.email.clone(),
            is_admin: i.is_admin(),
            is_developer: i.is_active && i.role.includes(crate::auth::Role::Developer),
            mfa_enabled: i.mfa_enabled,
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
    /// User-facing application identity (e.g. "Library Circulation").
    /// Sourced from [`SiteBranding::app_name`]. Templates render this
    /// in the chrome footer brand slot, page titles where
    /// appropriate, and auth-surface wordmarks. Framework name
    /// `RustIO` is intentionally absent from this field.
    pub app_name: String,
    /// Optional brand tagline ("Operational library management",
    /// "Account security notification" fallback for emails).
    pub app_tagline: Option<String>,
    /// `true` → templates render the small "Powered by RustIO" credit
    /// in the chrome footer + email footer. Opt-in per
    /// [`SiteBranding::show_powered_by`].
    pub show_powered_by: bool,
    /// Framework SemVer (`env!("CARGO_PKG_VERSION")`). Surfaced in
    /// the footer's identity column and used to deep-link the
    /// "Documentation" entry at the matching `docs.rs/rustio-admin/X.Y.Z`.
    pub framework_version: &'static str,
    /// Free-text label rendered in the footer's environment badge —
    /// `RUSTIO_ENV` if set, otherwise "Development" for debug builds
    /// and "Production" for release builds. Cached in a process-wide
    /// `OnceLock`; one env read per process, not per request.
    pub environment_label: &'static str,
    /// Stable kind discriminator for CSS targeting (`prod` lights the
    /// success dot, `dev` lights the amber dot, anything else stays
    /// neutral). Derived from `environment_label`.
    pub environment_kind: &'static str,
    /// UTC timestamp formatted at render time. Renders in the
    /// footer's context column so operators can read at-a-glance
    /// when a given page was generated.
    pub server_now: String,
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

/// Environment label resolved from `RUSTIO_ENV` (if set) else
/// derived from build kind. Cached process-wide: one env read at
/// first request, reused thereafter.
fn environment_label() -> &'static str {
    static ENV_LABEL: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ENV_LABEL.get_or_init(|| {
        std::env::var("RUSTIO_ENV").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "Development".into()
            } else {
                "Production".into()
            }
        })
    })
}

/// Stable CSS-class discriminator paired with `environment_label`.
/// Free-text labels (`Staging`, `Sandbox`, …) collapse to `other`
/// so the footer's coloured dot only lights for the two operational
/// extremes.
fn environment_kind(label: &str) -> &'static str {
    match label.to_ascii_lowercase().as_str() {
        "production" | "prod" => "prod",
        "development" | "dev" => "dev",
        _ => "other",
    }
}

impl BaseContext {
    // internal:
    pub(crate) fn new(identity: Option<&Identity>, csrf_token: String, admin: &Admin) -> Self {
        let b = admin.branding();
        let (is_demo_session, demo_label) = match identity {
            Some(i) => (i.is_demo, i.demo_label.clone()),
            None => (false, None),
        };
        let theme = admin.active_theme();
        let accent_hex = theme.accent.clone();
        let accent_rgb = accent_hex.as_deref().map(hex_to_rgb_triplet);
        let env_label = environment_label();
        Self {
            identity: identity.map(IdentityCtx::from),
            csrf_token,
            site_title: b.site_title.clone(),
            site_header: b.site_header.clone(),
            index_title: b.index_title.clone(),
            footer_copyright: b.footer_copyright.clone(),
            app_name: b.app_name.clone(),
            app_tagline: b.app_tagline.clone(),
            show_powered_by: b.show_powered_by,
            framework_version: env!("CARGO_PKG_VERSION"),
            environment_label: env_label,
            environment_kind: environment_kind(env_label),
            server_now: chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string(),
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

/// Template context for `/admin/account/sessions` (read-only in R0).
#[derive(Serialize)]
pub(crate) struct AccountSessionsCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    pub sessions: Vec<AccountSessionRowCtx>,
}

#[derive(Serialize)]
pub(crate) struct AccountSessionRowCtx {
    pub session_id: i64,
    pub trust_label: &'static str,
    pub is_current: bool,
    pub ip: String,
    pub ua_summary: String,
    pub created_at: String,
    pub last_seen_relative: String,
    pub expires_relative: String,
}

pub(crate) fn account_sessions_ctx(
    identity: &Identity,
    admin: &Admin,
    sessions: Vec<crate::auth::Session>,
    current_session_id: Option<i64>,
    csrf_token: String,
) -> AccountSessionsCtx {
    let rows = sessions
        .into_iter()
        .map(|s| AccountSessionRowCtx {
            session_id: s.session_id,
            trust_label: trust_label(s.trust_level),
            is_current: Some(s.session_id) == current_session_id,
            ip: s.ip.unwrap_or_else(|| "—".to_string()),
            ua_summary: summarise_user_agent(s.user_agent.as_deref()),
            created_at: s.created_at.format("%Y-%m-%d %H:%M").to_string(),
            last_seen_relative: relative_time(s.last_seen),
            expires_relative: relative_time(s.expires_at),
        })
        .collect();

    AccountSessionsCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "Active sessions",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        sessions: rows,
    }
}

const fn trust_label(t: crate::auth::SessionTrust) -> &'static str {
    match t {
        crate::auth::SessionTrust::Authenticated => "Signed in",
        crate::auth::SessionTrust::Elevated => "Elevated",
        crate::auth::SessionTrust::MfaVerified => "MFA verified",
    }
}

/// Heuristic User-Agent → short summary. Doctrine 20 — no fancy
/// risk scoring or device fingerprinting; just a deterministic
/// substring lookup so the table cell reads "macOS · Safari" instead
/// of an 80-char Mozilla string.
///
/// Returns "—" when no UA is recorded.
pub(crate) fn summarise_user_agent(ua: Option<&str>) -> String {
    let Some(ua) = ua else {
        return "—".to_string();
    };
    let lc = ua.to_ascii_lowercase();

    // Order matters: iPhone / iPad UAs still include "Mac OS X"
    // (Apple convention), and Android UAs include "Linux". Check the
    // most-specific identifiers first.
    let os = if lc.contains("windows") {
        "Windows"
    } else if lc.contains("iphone") {
        "iOS"
    } else if lc.contains("ipad") {
        "iPadOS"
    } else if lc.contains("android") {
        "Android"
    } else if lc.contains("mac os x") || lc.contains("macos") {
        "macOS"
    } else if lc.contains("linux") {
        "Linux"
    } else {
        "—"
    };

    let browser = if lc.contains("firefox") {
        "Firefox"
    } else if lc.contains("edg/") {
        "Edge"
    } else if lc.contains("opr/") || lc.contains("opera") {
        "Opera"
    } else if lc.contains("chrome") {
        "Chrome"
    } else if lc.contains("safari") {
        "Safari"
    } else if lc.contains("curl") {
        "curl"
    } else {
        "—"
    };

    if os == "—" && browser == "—" {
        ua.chars().take(40).collect()
    } else {
        format!("{os} · {browser}")
    }
}

/// Human label for the `Action` column on `/admin/history` and the
/// per-object history pages. Covers every `AuditEvent::as_str()`
/// string (`admin/audit.rs`'s `ActionType` + `AuditEvent` namespaces
/// together — see `audit::tests::action_type_and_audit_event_vocabularies_dont_collide`).
///
/// `VISIBILITY_AUDIT.md` finding B3: pre-0.8.1 this function knew
/// only `create / update / delete` and fell through to a generic
/// "Action" label for every R1+ event. The history table rendered
/// rows of identical green chips that hid which user action
/// produced the row — exactly the audit-log readability regression
/// the brief flagged.
fn action_label(action_type: &str) -> &'static str {
    match action_type {
        // Legacy `ActionType` namespace (generic CRUD on
        // project-registered models).
        "create" => "Created",
        "update" => "Changed",
        "delete" => "Deleted",

        // User / Group lifecycle (R0).
        "user_created" => "User created",
        "user_updated" => "User updated",
        "user_deleted" => "User deleted",
        "group_created" => "Group created",
        "group_updated" => "Group updated",
        "group_deleted" => "Group deleted",

        // R1 self-recovery.
        "password_changed_self" => "Password changed",
        "password_reset_self_request" => "Reset link requested",
        "password_reset_self_consume" => "Reset link consumed",

        // R2 organisational recovery.
        "password_reset_by_other" => "Password reset by admin",
        "forced_password_change_completed" => "Forced password change",
        "account_locked" => "Account locked",
        "account_unlocked" => "Account unlocked",

        // R3 TOTP MFA.
        "mfa_enabled" => "MFA enabled",
        "mfa_disabled" => "MFA disabled",
        "mfa_reset_by_other" => "MFA reset by admin",
        "mfa_code_consumed" => "Backup code used",
        "backup_codes_regenerated" => "Backup codes regenerated",

        // R0/R1 session lifecycle.
        "sessions_revoked_self" => "Sessions revoked (self)",
        "sessions_revoked_by_other" => "Sessions revoked by admin",
        "session_logout" => "Logged out",

        // R4 shell-tier emergency recovery (CLI-only emissions).
        "emergency_recovery" => "Emergency recovery",

        _ => "Action",
    }
}

fn action_pill_class(action_type: &str) -> &'static str {
    match action_type {
        // Created / enabled (good news) → success green.
        "create" | "user_created" | "group_created" | "account_unlocked" | "mfa_enabled" => {
            "badge-success"
        }

        // Destructive or compromise-shaped events → danger red.
        "delete"
        | "user_deleted"
        | "group_deleted"
        | "account_locked"
        | "mfa_disabled"
        | "mfa_reset_by_other"
        | "sessions_revoked_by_other" => "badge-danger",

        // Admin-initiated mutations on a user → warning amber. Same
        // visual weight as the "by other" R2 events; signals the
        // row was driven by someone other than the subject.
        "password_reset_by_other" | "forced_password_change_completed" | "emergency_recovery" => {
            "badge-warning"
        }

        // Routine changes and self-driven events → neutral.
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
    /// `(field, value)` pairs for every currently-set filter. The
    /// search form renders these as hidden inputs so submitting a
    /// query doesn't drop the active filters.
    pub active_filter_pairs: Vec<(String, String)>,
    /// Display-ready pills for every currently-set filter: friendly
    /// label, pretty value, and a remove-link that drops only this
    /// filter while keeping query / sort / other filters. Drives the
    /// "active filters" strip below the toolbar.
    pub active_filter_pills: Vec<ActiveFilterPillCtx>,
    /// URL the "Clear all" filters action navigates to: keeps the
    /// search query and sort, drops every filter.
    pub clear_all_filters_link: String,
    /// Sort dropdown options — every visible field × {asc, desc} plus
    /// a "Default order" reset link. Pre-baked into ready-to-render
    /// `(label, href, is_active)` triplets.
    pub sort_options: Vec<SortOptionCtx>,
    /// Toolbar label for the Sort toggle: "Default order" when no
    /// override is in effect, otherwise the active option's label.
    pub current_sort_label: String,
    /// Active sort field + direction surfaced as plain strings so the
    /// search form can carry them as hidden inputs. `None` when no
    /// sort override is in effect.
    pub active_sort_field: Option<String>,
    pub active_sort_dir: Option<&'static str>,
    /// Per-page dropdown options (allow-listed: 25 / 50 / 100 / 200).
    pub per_page_options: Vec<PerPageOptionCtx>,
    /// Toolbar label for the per-page toggle: "50 / page" etc.
    pub current_per_page_label: String,
    /// `Some` when the URL carries an explicit `?per_page=…`. Hidden
    /// in the search form so query submission keeps the user's
    /// row-density choice; absent → fall back to model default.
    pub active_per_page_override: Option<usize>,
    pub page: usize,
    pub total_pages: usize,
    pub per_page: usize,
    pub total_rows: usize,
    /// Pre-baked URLs for the pagination strip. `None` when at the
    /// boundary (page 1 has no prev; last page has no next).
    pub prev_page_link: Option<String>,
    pub next_page_link: Option<String>,
    /// Numbered-page items for the pagination strip. For `total_pages
    /// ≤ 7` every page is listed in order; otherwise the list is
    /// compressed to first / current ±1 / last with `Ellipsis`
    /// markers in the gaps. Always empty when `total_pages == 1`.
    pub page_items: Vec<PageItem>,
    /// Whether the bulk-action UI should render. Always `false` until
    /// the bulk-action POST endpoint is wired in a later phase.
    pub bulk_actions_enabled: bool,
    /// Project-defined bulk actions registered via
    /// `ModelAdmin::bulk_actions()`. Rendered as extra buttons in
    /// the list-view bulk bar next to the framework's built-in
    /// Delete. Each button POSTs to `/admin/:model/bulk/:name`.
    pub bulk_action_buttons: Vec<BulkActionBtnCtx>,
    pub flash: Option<FlashCtx>,
}

#[derive(Serialize)]
pub(crate) struct BulkActionBtnCtx {
    pub name: &'static str,
    pub label: &'static str,
    pub destructive: bool,
    pub form_action: String,
}

/// `values` is flattened into the JSON object so template code can do
/// `row[field.name]` (minijinja resolves dict subscript on the merged
/// map). The explicit `id: i64` field stays out of the flattened map.
#[derive(Serialize)]
pub(crate) struct ListRowCtx {
    pub id: i64,
    #[serde(flatten)]
    pub values: HashMap<String, serde_json::Value>,
    /// Per-column FK click-through links. Keyed by column name; value
    /// is the target's `/admin/{admin_name}/{id}/edit` URL. Populated
    /// by `handlers::hydrate_fk_cells` for relation-bearing columns
    /// and consumed by the list template to wrap the cell in `<a>`.
    pub links: HashMap<String, String>,
}

#[derive(Serialize)]
pub(crate) struct FilterGroupCtx {
    pub field: String,
    pub label: String,
    pub options: Vec<FilterOptionCtx>,
    pub current: Option<String>,
    /// URL for the "All" chip — clears this group while keeping every
    /// other piece of list state (search query, other filters, sort).
    /// Pre-baked in `list_ctx` so the template doesn't reproduce the
    /// URL-composition rules. Defaults to empty before `list_ctx`
    /// patches it in (handlers don't construct this field directly).
    #[serde(default)]
    pub all_link: String,
}

#[derive(Serialize)]
pub(crate) struct FilterOptionCtx {
    pub value: String,
    pub label: String,
    pub selected: bool,
    /// URL the chip navigates to: applies this option to its group
    /// while preserving search / other filters / sort. Pre-baked in
    /// `list_ctx`. Empty until then.
    #[serde(default)]
    pub link: String,
}

/// One option in the toolbar's Sort dropdown — a field × direction
/// pair, or the "Default order" reset link. The label is field-type
/// aware ("A → Z" for text, "newest first" for datetime, etc.) so the
/// dropdown reads as English, not as a query string.
#[derive(Serialize)]
pub(crate) struct SortOptionCtx {
    pub label: String,
    pub link: String,
    pub is_active: bool,
}

/// One option in the toolbar's per-page dropdown. The link goes
/// through `build_list_url` so search / filter / sort survive a
/// row-density change. Page resets to 1 — staying on page N at a
/// new density would land somewhere arbitrary.
#[derive(Serialize)]
pub(crate) struct PerPageOptionCtx {
    pub value: usize,
    pub label: String,
    pub link: String,
    pub is_active: bool,
}

/// One pill in the "active filters" strip below the toolbar. `label`
/// is the field's human label, `value_label` is the option's display
/// text (not the raw URL value), and `remove_link` is the URL that
/// drops only this filter — query, sort, and the rest of the filter
/// set are preserved.
#[derive(Serialize)]
pub(crate) struct ActiveFilterPillCtx {
    pub label: String,
    pub value_label: String,
    pub remove_link: String,
}

/// One slot in the pagination strip — either a numbered page (with a
/// pre-baked link and an active marker) or a `…` gap. The template
/// renders the variant via the serialized `kind` discriminant.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PageItem {
    Number {
        number: usize,
        link: String,
        is_active: bool,
    },
    Ellipsis,
}

/// Build the numbered-page strip. Up to 7 pages render in full; past
/// that the list compresses to first, current ± 1, last with `…` in
/// the gaps. The build_link closure handles URL composition so this
/// helper stays unaware of search / filter / sort state.
fn build_page_items(
    current: usize,
    total: usize,
    build_link: impl Fn(usize) -> String,
) -> Vec<PageItem> {
    if total <= 1 {
        return Vec::new();
    }
    let mk = |n: usize| PageItem::Number {
        number: n,
        link: build_link(n),
        is_active: n == current,
    };
    if total <= 7 {
        return (1..=total).map(mk).collect();
    }
    let mut items: Vec<PageItem> = Vec::with_capacity(9);
    items.push(mk(1));
    if current > 3 {
        items.push(PageItem::Ellipsis);
    }
    let start = current.saturating_sub(1).max(2);
    let end = (current + 1).min(total - 1);
    for n in start..=end {
        items.push(mk(n));
    }
    if current + 2 < total {
        items.push(PageItem::Ellipsis);
    }
    items.push(mk(total));
    items
}

/// Field-type-aware copy for an `(field_type, direction)` pair.
/// Datetime descending reads as "newest first"; string ascending as
/// "A → Z"; everything else falls back to ascending/descending.
fn sort_direction_label(
    field_type: super::types::FieldType,
    dir: super::modeladmin::SortDir,
) -> &'static str {
    use super::modeladmin::SortDir;
    use super::types::FieldType::*;
    match (field_type, dir) {
        (DateTime | OptionalDateTime, SortDir::Desc) => "newest first",
        (DateTime | OptionalDateTime, SortDir::Asc) => "oldest first",
        (String | OptionalString, SortDir::Asc) => "A → Z",
        (String | OptionalString, SortDir::Desc) => "Z → A",
        (Bool, SortDir::Asc) => "off → on",
        (Bool, SortDir::Desc) => "on → off",
        (_, SortDir::Asc) => "ascending",
        (_, SortDir::Desc) => "descending",
    }
}

/// Compose a list-view URL with full query-state preservation.
///
/// Every link the list view emits — filter chips, sort options,
/// pagination, header-sort arrows, per-page picker — runs through
/// here so a click on one widget doesn't silently drop the others.
/// Inputs:
///
///   - `q` — current search query; `""` skipped
///   - `filters` — currently-set filters as `(field, value)` pairs;
///     callers compose their own override (set, clear, swap) before
///     passing this in
///   - `sort` — the desired sort, or `None` for "model default"
///   - `page` — `1` is implicit and skipped from the URL
///   - `per_page` — `Some(N)` carries an explicit row-density choice
///     into the URL; `None` means "use the model default" (no
///     `&per_page=…` segment emitted)
///
/// Values are URL-encoded so search strings with spaces or unicode
/// don't break the link.
fn build_list_url(
    admin_name: &str,
    q: &str,
    filters: &[(String, String)],
    sort: Option<(&str, super::modeladmin::SortDir)>,
    page: usize,
    per_page: Option<usize>,
) -> String {
    use super::modeladmin::SortDir;
    let mut parts: Vec<String> = Vec::new();
    if !q.is_empty() {
        parts.push(format!("q={}", urlencoding::encode(q)));
    }
    for (field, value) in filters {
        parts.push(format!(
            "{}={}",
            urlencoding::encode(field),
            urlencoding::encode(value),
        ));
    }
    if let Some((col, dir)) = sort {
        parts.push(format!("sort={}", urlencoding::encode(col)));
        parts.push(
            match dir {
                SortDir::Asc => "dir=asc",
                SortDir::Desc => "dir=desc",
            }
            .to_string(),
        );
    }
    if page > 1 {
        parts.push(format!("page={page}"));
    }
    if let Some(n) = per_page {
        parts.push(format!("per_page={n}"));
    }
    if parts.is_empty() {
        format!("/admin/{admin_name}")
    } else {
        format!("/admin/{}?{}", admin_name, parts.join("&"))
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn list_ctx(
    identity: &Identity,
    admin: &Admin,
    entry: &AdminEntry,
    rows: Vec<ListRow>,
    search_query: String,
    mut filters: Vec<FilterGroupCtx>,
    page: usize,
    per_page: usize,
    // `per_page_override = Some(N)` when the URL carried an allow-listed
    // `?per_page=…`. `None` means the model default is in effect —
    // every state-preserving link below then omits the segment so
    // default URLs stay clean.
    per_page_override: Option<usize>,
    total_rows: usize,
    // `active_sort = (column, direction)` carries the parsed override
    // from `?sort=&dir=`. `None` means the model's
    // `ModelAdmin::ordering()` default is in effect — sortable
    // header links still render, but no column gets the active arrow.
    active_sort: Option<(String, super::modeladmin::SortDir)>,
    csrf_token: String,
) -> ListCtx {
    let total_pages = total_rows.div_ceil(per_page.max(1)).max(1);

    // ---- URL-state preservation -------------------------------------
    // Every link the list view emits — filter chips, sort options,
    // pagination, header sort arrows — composes its href via
    // `build_list_url` so clicking one widget never silently drops
    // the others. `active_filter_pairs` is the canonical view of
    // currently-set filters; widgets derive their override URLs from
    // a copy of it.
    let active_filter_pairs: Vec<(String, String)> = filters
        .iter()
        .filter_map(|g| g.current.as_ref().map(|v| (g.field.clone(), v.clone())))
        .collect();
    let active_sort_ref: Option<(&str, super::modeladmin::SortDir)> =
        active_sort.as_ref().map(|(c, d)| (c.as_str(), *d));

    // Patch each filter group's chip URLs in-place. "All" drops this
    // group from the active set; an option link replaces the group's
    // current value. Page resets to 1 — page N of one filter rarely
    // matches up with page N of another.
    for group in &mut filters {
        let other: Vec<(String, String)> = active_filter_pairs
            .iter()
            .filter(|(field, _)| field != &group.field)
            .cloned()
            .collect();
        group.all_link = build_list_url(
            entry.admin_name,
            &search_query,
            &other,
            active_sort_ref,
            1,
            per_page_override,
        );
        for opt in &mut group.options {
            let mut combined = other.clone();
            combined.push((group.field.clone(), opt.value.clone()));
            opt.link = build_list_url(
                entry.admin_name,
                &search_query,
                &combined,
                active_sort_ref,
                1,
                per_page_override,
            );
        }
    }

    let clear_all_filters_link = build_list_url(
        entry.admin_name,
        &search_query,
        &[],
        active_sort_ref,
        1,
        per_page_override,
    );

    // Display-ready pills for the "active filters" strip. Each pill
    // resolves the option's friendly `value_label` from the group's
    // option list (so a stored "true" reads as "Yes", etc.), and its
    // `remove_link` drops only this filter — search query, sort, and
    // every other filter stay intact.
    let active_filter_pills: Vec<ActiveFilterPillCtx> = filters
        .iter()
        .filter_map(|g| {
            let v = g.current.as_ref()?;
            let value_label = g
                .options
                .iter()
                .find(|o| &o.value == v)
                .map(|o| o.label.clone())
                .unwrap_or_else(|| v.clone());
            let other: Vec<(String, String)> = active_filter_pairs
                .iter()
                .filter(|(field, _)| field != &g.field)
                .cloned()
                .collect();
            Some(ActiveFilterPillCtx {
                label: g.label.clone(),
                value_label,
                remove_link: build_list_url(
                    entry.admin_name,
                    &search_query,
                    &other,
                    active_sort_ref,
                    1,
                    per_page_override,
                ),
            })
        })
        .collect();

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
            let (sort_active, sort_link) = build_sort_link(
                f.name,
                &active_sort,
                entry.admin_name,
                &search_query,
                &active_filter_pairs,
                per_page_override,
            );
            ListField {
                name: f.name.to_string(),
                label: f.label.to_string(),
                kind: f.field_type.widget(),
                sort_active,
                sort_link,
            }
        })
        .collect();

    // Build the toolbar's Sort dropdown options. Each visible field
    // contributes two entries (asc + desc); a leading "Default order"
    // entry resets to `ModelAdmin::ordering()`. Every link goes
    // through `build_list_url` so search + filters survive a sort
    // change.
    use super::modeladmin::SortDir;
    let mut sort_options: Vec<SortOptionCtx> = Vec::with_capacity(visible_fields.len() * 2 + 1);
    sort_options.push(SortOptionCtx {
        label: "Default order".to_string(),
        link: build_list_url(
            entry.admin_name,
            &search_query,
            &active_filter_pairs,
            None,
            1,
            per_page_override,
        ),
        is_active: active_sort.is_none(),
    });
    for f in &visible_fields {
        for dir in [SortDir::Asc, SortDir::Desc] {
            let dir_label = sort_direction_label(f.field_type, dir);
            let is_active = matches!(
                &active_sort,
                Some((col, d)) if col == f.name && *d == dir
            );
            sort_options.push(SortOptionCtx {
                label: format!("{} ({})", f.label, dir_label),
                link: build_list_url(
                    entry.admin_name,
                    &search_query,
                    &active_filter_pairs,
                    Some((f.name, dir)),
                    1,
                    per_page_override,
                ),
                is_active,
            });
        }
    }
    let current_sort_label = sort_options
        .iter()
        .find(|o| o.is_active)
        .map(|o| o.label.clone())
        .unwrap_or_else(|| "Default order".to_string());

    let prev_page_link = (page > 1).then(|| {
        build_list_url(
            entry.admin_name,
            &search_query,
            &active_filter_pairs,
            active_sort_ref,
            page - 1,
            per_page_override,
        )
    });
    let next_page_link = (page < total_pages).then(|| {
        build_list_url(
            entry.admin_name,
            &search_query,
            &active_filter_pairs,
            active_sort_ref,
            page + 1,
            per_page_override,
        )
    });

    let page_items = build_page_items(page, total_pages, |n| {
        build_list_url(
            entry.admin_name,
            &search_query,
            &active_filter_pairs,
            active_sort_ref,
            n,
            per_page_override,
        )
    });

    let (active_sort_field, active_sort_dir) = match &active_sort {
        Some((col, SortDir::Asc)) => (Some(col.clone()), Some("asc")),
        Some((col, SortDir::Desc)) => (Some(col.clone()), Some("desc")),
        None => (None, None),
    };

    // Per-page allow-list mirrors the handler's set; values outside it
    // are silently dropped server-side. Each option's link uses
    // `Some(value)` for non-default densities so the override carries
    // through, and `None` for the model default so the URL stays clean.
    let per_page_choices: [usize; 4] = [25, 50, 100, 200];
    let model_default_per_page = entry.list_per_page;
    let per_page_options: Vec<PerPageOptionCtx> = per_page_choices
        .iter()
        .map(|&n| {
            let override_for_link = (n != model_default_per_page).then_some(n);
            PerPageOptionCtx {
                value: n,
                label: format!("{n} / page"),
                link: build_list_url(
                    entry.admin_name,
                    &search_query,
                    &active_filter_pairs,
                    active_sort_ref,
                    1,
                    override_for_link,
                ),
                is_active: per_page == n,
            }
        })
        .collect();
    let current_per_page_label = format!("{per_page} / page");
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
                let mut links: HashMap<String, String> = HashMap::new();
                let cell_links = r.cell_links;
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
                        if let Some(Some(link)) = cell_links.get(i) {
                            links.insert(
                                (*name).to_string(),
                                format!("/admin/{}/{}/edit", link.admin_name, link.id),
                            );
                        }
                    }
                }
                ListRowCtx {
                    id: r.id,
                    values,
                    links,
                }
            })
            .collect(),
        search_query,
        active_filter_count: filters.iter().filter(|g| g.current.is_some()).count(),
        active_filter_pairs,
        active_filter_pills,
        clear_all_filters_link,
        filters,
        sort_options,
        current_sort_label,
        active_sort_field,
        active_sort_dir,
        per_page_options,
        current_per_page_label,
        active_per_page_override: per_page_override,
        page,
        total_pages,
        per_page,
        total_rows,
        prev_page_link,
        next_page_link,
        page_items,
        bulk_actions_enabled: false,
        bulk_action_buttons: entry
            .bulk_actions
            .iter()
            .map(|a| BulkActionBtnCtx {
                name: a.name,
                label: a.label,
                destructive: a.destructive,
                form_action: format!("/admin/{}/bulk/{}", entry.admin_name, a.name),
            })
            .collect(),
        flash: None,
    }
}

/// Pre-bake the sortable-header URL + active-direction marker for one
/// column. Three states:
///   - column is the current sort, ascending  → click toggles to desc
///   - column is the current sort, descending → click clears the sort
///   - column is not the current sort         → click sets ascending
///
/// The URL goes through `build_list_url` so search query and active
/// filters are preserved across header clicks. Page resets to 1
/// because page N of one ordering rarely lines up with page N of
/// another.
fn build_sort_link(
    name: &'static str,
    active: &Option<(String, super::modeladmin::SortDir)>,
    admin_name: &str,
    q: &str,
    filters: &[(String, String)],
    per_page: Option<usize>,
) -> (&'static str, String) {
    use super::modeladmin::SortDir;
    let (marker, new_sort) = match active {
        Some((col, SortDir::Asc)) if col == name => ("asc", Some((name, SortDir::Desc))),
        Some((col, SortDir::Desc)) if col == name => ("desc", None),
        _ => ("", Some((name, SortDir::Asc))),
    };
    (
        marker,
        build_list_url(admin_name, q, filters, new_sort, 1, per_page),
    )
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
            // `ModelAdmin::readonly_fields()` flagged this column. The
            // browser-side `disabled` attribute prevents submission, so
            // the value always reflects the existing row (never the
            // re-rendered submitted form). On `new` there is no row
            // yet, so readonly has no effect until the row is saved.
            let readonly = mode == "edit" && entry.readonly_fields.contains(&f.name);
            let value = if let Some(form) = submitted.filter(|_| !readonly) {
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
            // non-nullable field does. Readonly drops the asterisk too
            // — the user can't satisfy it from this form.
            let required = !readonly
                && !f.field_type.nullable()
                && !matches!(f.field_type, super::types::FieldType::Bool);
            let (options, multiple, searchable, has_more) = if let Some(values) = f.choices {
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
                disabled: readonly,
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
    pub display_name: &'static str,
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
        display_name: entry.display_name,
        singular_name: entry.singular_name,
        object_id,
        object_label,
        cascading,
        flash: None,
    }
}

// ---- Bulk-delete confirmation -------------------------------------------

#[derive(Serialize)]
pub(crate) struct BulkConfirmDeleteCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub admin_name: &'static str,
    pub display_name: &'static str,
    pub singular_name: &'static str,
    /// `(id, label)` for each row the user selected, in selection
    /// order. Rendered as a list on the confirm page so the user
    /// sees exactly what will be deleted.
    pub items: Vec<BulkDeleteItem>,
    /// Comma-separated IDs replayed into the confirm form's hidden
    /// `_ids` field — same wire format the checkbox form posts.
    pub ids_csv: String,
    pub flash: Option<FlashCtx>,
}

#[derive(Serialize)]
pub(crate) struct BulkDeleteItem {
    pub id: i64,
    pub label: String,
}

// ---- Bulk action confirmation (project-defined) -------------------------

#[derive(Serialize)]
pub(crate) struct BulkConfirmActionCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub admin_name: &'static str,
    pub display_name: &'static str,
    pub singular_name: &'static str,
    /// The action's URL slug (e.g. `"publish"`) — replayed back into
    /// the confirm form's `formaction` URL.
    pub action_name: &'static str,
    pub action_label: &'static str,
    pub action_destructive: bool,
    pub items: Vec<BulkDeleteItem>,
    pub ids_csv: String,
    pub flash: Option<FlashCtx>,
}

pub(crate) fn bulk_confirm_action_ctx(
    identity: &Identity,
    admin: &Admin,
    entry: &AdminEntry,
    action: super::modeladmin::BulkAction,
    items: Vec<BulkDeleteItem>,
    csrf_token: String,
) -> BulkConfirmActionCtx {
    let ids_csv = items
        .iter()
        .map(|i| i.id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    BulkConfirmActionCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: format!("{} — {} {}", action.label, items.len(), entry.display_name),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        admin_name: entry.admin_name,
        display_name: entry.display_name,
        singular_name: entry.singular_name,
        action_name: action.name,
        action_label: action.label,
        action_destructive: action.destructive,
        items,
        ids_csv,
        flash: None,
    }
}

pub(crate) fn bulk_confirm_delete_ctx(
    identity: &Identity,
    admin: &Admin,
    entry: &AdminEntry,
    items: Vec<BulkDeleteItem>,
    csrf_token: String,
) -> BulkConfirmDeleteCtx {
    let ids_csv = items
        .iter()
        .map(|i| i.id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    BulkConfirmDeleteCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: format!("Delete {} {}", items.len(), entry.display_name),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        admin_name: entry.admin_name,
        display_name: entry.display_name,
        singular_name: entry.singular_name,
        items,
        ids_csv,
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
    /// Project-model entries for the sidebar. Required to keep the
    /// chrome navigable on 4xx/5xx pages (`VISIBILITY_AUDIT.md` A2):
    /// previously the error page rendered without a sidebar because
    /// `entries` was absent, so the operator hit a navigational
    /// dead-end the moment they bounced off a 404.
    pub entries: Vec<SidebarEntry>,
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
    // Sidebar entries for the chrome. `core=true` entries (User /
    // Group) are excluded from the dynamic Models loop the way
    // every other page does it — they live in the hardcoded Auth
    // block of `_sidebar.html`.
    let sidebar_entries: Vec<SidebarEntry> = admin
        .entries()
        .iter()
        .filter(|e| !e.core)
        .map(SidebarEntry::from)
        .collect();
    let view = ErrorCtx {
        base: BaseContext::new(identity, String::new(), admin),
        page_title: format!("{status} {heading}"),
        status,
        heading: heading.clone(),
        message,
        entries: sidebar_entries,
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
///
/// `editor_rank` filters out roles strictly above the editor's own
/// rank — first-line defense for the role-ceiling guard, so the user
/// never sees an option the server would reject. Server-side
/// `enforce_role_ceiling` catches forged POSTs as defense-in-depth;
/// this function is reflection, not security.
pub(crate) fn role_select_options(editor_rank: u32) -> Vec<SelectOption> {
    let all = [
        (crate::auth::Role::User, "user", "User (no admin access)"),
        (
            crate::auth::Role::Staff,
            "staff",
            "Staff (admin access; per-model group permissions)",
        ),
        (
            crate::auth::Role::Supervisor,
            "supervisor",
            "Supervisor (view + edit; no destructive ops)",
        ),
        (
            crate::auth::Role::Administrator,
            "administrator",
            "Administrator (full coverage; bypasses group checks)",
        ),
        (
            crate::auth::Role::Developer,
            "developer",
            "Developer (highest tier)",
        ),
    ];
    all.iter()
        .filter(|(role, _, _)| role.rank() <= editor_rank)
        .map(|(_, slug, label)| SelectOption {
            value: (*slug).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

/// FormField list for the user_new form. Two sections: Identity
/// (email + password) and Role (the 5-option select). Caller passes
/// the current values so re-render after validation failure preserves
/// them. `editor_rank` filters the role select per the ceiling guard.
/// `min_length` populates the password hint so a project that
/// overrides `Admin::password_policy(...)` sees its actual floor on
/// the form — passed in from `Admin::active_password_policy().min_length()`,
/// the same plumbing R1 commit #11 added for `password_change_form_sections`.
///
/// Pre-R2 the hint string was hardcoded to "8 characters"; R2
/// commit #3 routed it through the policy so the framework default
/// (10) and project overrides (12 / 16 / …) both render correctly.
pub(crate) fn user_new_form_sections(
    email: &str,
    role: &str,
    editor_rank: u32,
    min_length: usize,
) -> Vec<FormSection> {
    let password_hint = format!(
        "At least {min_length} characters. The user can change it later via Change password."
    );
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
                    hint: Some(password_hint),
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
                options: Some(role_select_options(editor_rank)),
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
/// so values reflect the current row. `editor_rank` filters the role
/// select per the ceiling guard.
pub(crate) fn user_edit_identity_sections(
    email: &str,
    role: &str,
    is_active: bool,
    editor_rank: u32,
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
                options: Some(role_select_options(editor_rank)),
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

/// Pre-built FormField list for the password-change form. Values are
/// always empty (we never echo passwords back). The
/// `min_length` parameter controls the live policy hint shown
/// beneath the new-password input — passed in from
/// `Admin::active_password_policy().min_length()` so a project that
/// overrides the policy gets accurate copy on the form
/// (`DESIGN_RECOVERY.md` §13).
///
/// Pre-R1 the hint string was hardcoded to "8 characters"; R1
/// commit #11 routed it through the policy so the framework
/// default (10) and project overrides (12 / 16 / …) both render
/// correctly.
pub(crate) fn password_change_form_sections(min_length: usize) -> Vec<FormSection> {
    let new_password_hint = format!("At least {min_length} characters.");
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
                hint: Some(new_password_hint),
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

/// FormField list for the R2 forced-rotation interstitial
/// (`/admin/must-change-password`). Two fields — `new_password1`
/// and `new_password2` — and no `old_password`: the user has just
/// authenticated with the temp password the admin issued seconds
/// ago, and the design contract (`DESIGN_R2_ORGANISATIONAL.md`
/// §3.4) intentionally skips collecting it again.
///
/// `min_length` is read from
/// `Admin::active_password_policy().min_length()`, mirroring R1's
/// [`password_change_form_sections`].
pub(crate) fn must_change_password_form_sections(min_length: usize) -> Vec<FormSection> {
    let new_password_hint = format!("At least {min_length} characters.");
    vec![FormSection {
        title: None,
        fields: vec![
            FormField {
                name: "new_password1",
                label: "New password".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: Some(new_password_hint),
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 2,
                autocomplete: Some("new-password"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `VISIBILITY_AUDIT.md` finding B3 enforcement.
    ///
    /// Every `AuditEvent::as_str()` value MUST have a non-generic
    /// label entry in [`action_label`]. Pre-0.8.1 the function knew
    /// only `create / update / delete` and fell through to "Action"
    /// for everything else, so the History page rendered identical
    /// generic pills for every R1+ event.
    ///
    /// When a new `AuditEvent` variant ships, this test fails until
    /// the new event-string is added to the `action_label` match.
    /// Same drift-protection shape as the
    /// `audit_event_existing_variants_have_stable_strings` test in
    /// `admin/audit.rs`.
    #[test]
    fn action_label_covers_every_audit_event_string() {
        // Canonical list of every audit-event string written into
        // `rustio_admin_actions.action_type`. Mirrors the
        // `ALL_AUDIT_EVENTS` array in `admin/audit.rs::tests` —
        // duplicated here because the audit array lives under
        // `#[cfg(test)]` in a different module.
        let known_event_strings: &[&str] = &[
            // Legacy CRUD namespace.
            "create",
            "update",
            "delete",
            // R0 user / group lifecycle.
            "user_created",
            "user_updated",
            "user_deleted",
            "group_created",
            "group_updated",
            "group_deleted",
            // R1 self-recovery.
            "password_changed_self",
            "password_reset_self_request",
            "password_reset_self_consume",
            // R2 organisational recovery.
            "password_reset_by_other",
            "forced_password_change_completed",
            "account_locked",
            "account_unlocked",
            // R3 TOTP MFA.
            "mfa_enabled",
            "mfa_disabled",
            "mfa_reset_by_other",
            "mfa_code_consumed",
            "backup_codes_regenerated",
            // R0/R1 session lifecycle.
            "sessions_revoked_self",
            "sessions_revoked_by_other",
            "session_logout",
            // R4 emergency recovery.
            "emergency_recovery",
        ];
        let mut missing: Vec<&'static str> = Vec::new();
        for &s in known_event_strings {
            if action_label(s) == "Action" {
                missing.push(s);
            }
        }
        assert!(
            missing.is_empty(),
            "action_label falls through to the generic \"Action\" \
             label for these event strings — add explicit match arms \
             in `admin/render.rs::action_label` (and pick a pill class \
             in `action_pill_class`): {missing:?}"
        );
    }

    #[test]
    fn action_pill_class_returns_known_classes() {
        // Every pill class must be one the CSS knows about. New
        // arms must use one of `badge-success / badge-neutral /
        // badge-danger / badge-warning` — see
        // `assets/static/admin.css` for the rio-pill-- definitions.
        let known_strings: &[&str] = &[
            "create",
            "update",
            "delete",
            "user_created",
            "user_updated",
            "user_deleted",
            "group_created",
            "group_updated",
            "group_deleted",
            "password_changed_self",
            "password_reset_self_request",
            "password_reset_self_consume",
            "password_reset_by_other",
            "forced_password_change_completed",
            "account_locked",
            "account_unlocked",
            "mfa_enabled",
            "mfa_disabled",
            "mfa_reset_by_other",
            "mfa_code_consumed",
            "backup_codes_regenerated",
            "sessions_revoked_self",
            "sessions_revoked_by_other",
            "session_logout",
            "emergency_recovery",
        ];
        let known_classes = [
            "badge-success",
            "badge-neutral",
            "badge-danger",
            "badge-warning",
        ];
        for &s in known_strings {
            let class = action_pill_class(s);
            assert!(
                known_classes.contains(&class),
                "action_pill_class({s:?}) returned {class:?} which is \
                 not one of {known_classes:?}"
            );
        }
    }

    #[test]
    fn ua_summary_macos_safari() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
        assert_eq!(summarise_user_agent(Some(ua)), "macOS · Safari");
    }

    #[test]
    fn ua_summary_windows_chrome() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        assert_eq!(summarise_user_agent(Some(ua)), "Windows · Chrome");
    }

    #[test]
    fn ua_summary_linux_firefox() {
        let ua = "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0";
        assert_eq!(summarise_user_agent(Some(ua)), "Linux · Firefox");
    }

    #[test]
    fn ua_summary_android_chrome() {
        let ua = "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";
        assert_eq!(summarise_user_agent(Some(ua)), "Android · Chrome");
    }

    #[test]
    fn ua_summary_ios_safari() {
        let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
        assert_eq!(summarise_user_agent(Some(ua)), "iOS · Safari");
    }

    #[test]
    fn ua_summary_curl_falls_through_to_unknown_os() {
        // curl/8.4.0 — no OS identifier, only browser. Returns the raw
        // UA truncated.
        let ua = "curl/8.4.0";
        let s = summarise_user_agent(Some(ua));
        assert!(s.contains("curl"));
    }

    #[test]
    fn ua_summary_unknown_returns_truncated() {
        let ua =
            "QuiteUnusualUserAgent/1.0 with extremely long descriptor that should be truncated";
        let s = summarise_user_agent(Some(ua));
        assert!(s.len() <= 40);
    }

    #[test]
    fn ua_summary_none_returns_dash() {
        assert_eq!(summarise_user_agent(None), "—");
    }

    #[test]
    fn trust_label_strings() {
        assert_eq!(
            trust_label(crate::auth::SessionTrust::Authenticated),
            "Signed in"
        );
        assert_eq!(trust_label(crate::auth::SessionTrust::Elevated), "Elevated");
        assert_eq!(
            trust_label(crate::auth::SessionTrust::MfaVerified),
            "MFA verified"
        );
    }

    /// R1 commit #11 — `password_change_form_sections` reflects the
    /// caller-supplied `min_length` so a project that overrides the
    /// `PasswordPolicy` floor sees accurate copy on the form. The
    /// pre-R1 hardcoded "8 characters" is gone.
    #[test]
    fn password_change_form_sections_renders_live_min_length() {
        let sections = password_change_form_sections(10);
        assert_eq!(sections.len(), 1);
        let fields = &sections[0].fields;
        // Three fields: old_password, new_password1, new_password2.
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "old_password");
        assert_eq!(fields[1].name, "new_password1");
        assert_eq!(fields[2].name, "new_password2");
        // Hint reflects the caller's parameter, not a hardcoded
        // "8" or "12".
        assert_eq!(
            fields[1].hint.as_deref(),
            Some("At least 10 characters."),
            "default-policy floor 10 must surface in the hint"
        );

        // Project override propagates.
        let sections = password_change_form_sections(16);
        assert_eq!(
            sections[0].fields[1].hint.as_deref(),
            Some("At least 16 characters."),
        );
    }

    /// Old + confirm fields don't carry the hint — only the new-
    /// password field does. Belt-and-braces: the policy minimum is
    /// surfaced exactly once, beneath the input the user is typing
    /// the new password into.
    #[test]
    fn password_change_form_sections_only_new_password_carries_hint() {
        let sections = password_change_form_sections(10);
        let fields = &sections[0].fields;
        assert!(fields[0].hint.is_none(), "old_password must have no hint");
        assert!(fields[1].hint.is_some(), "new_password1 must have the hint");
        assert!(fields[2].hint.is_none(), "new_password2 must have no hint");
    }
}
