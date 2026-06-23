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
use super::types::{Admin, AdminEntry, AdminField, EditRow, FieldType, ListRow};
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
    /// `true` when the admin was constructed with
    /// [`crate::admin::Admin::read_only`]. Drives the chrome
    /// banner in `_base.html` and the suppression of top-level
    /// "Add" buttons on the dashboard and list pages.
    pub read_only: bool,
    /// Unread-notification count for the active operator. `0` when
    /// the page didn't pre-fetch (default) or there's no signed-in
    /// identity. Drives the topbar bell's red-dot badge —
    /// `_topbar.html` renders the badge only when this is `> 0`.
    /// Page handlers that want the badge live on their page chain
    /// `.with_unread_count(n)` after [`BaseContext::new`].
    pub unread_count: i64,
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
            read_only: admin.is_read_only(),
            unread_count: 0,
        }
    }

    // internal:
    /// Builder helper — chain after [`Self::new`] to pin the
    /// operator's unread-notification count for the topbar badge.
    /// Handlers that want the badge to render on their page
    /// pre-fetch via [`crate::admin::notifications::unread_count`]
    /// and pass it in. Pages that skip this stay at the default
    /// `0` and the topbar shows just the bare bell.
    pub(crate) fn with_unread_count(mut self, n: i64) -> Self {
        self.unread_count = n.max(0);
        self
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
    /// Friendly greeting label — first segment of the identity's
    /// email (`alice` from `alice@example.com`), title-cased. The
    /// template uses it for the welcome header so it doesn't read
    /// as "Welcome, alice@example.com" (too formal / too long).
    pub greeting_name: String,
    /// Total project models registered (excludes the synthetic
    /// User entry). Surfaces in the stats strip.
    pub total_models: usize,
    /// Sum of `row_estimate` across every registered project model.
    /// Inherits the "approximate" caveat from `pg_class.reltuples`.
    pub total_rows: i64,
    /// `recent_actions.len()` — pre-computed so the template can
    /// show "Recent activity (N)" without a `length` filter.
    pub recent_actions_count: usize,
    /// 7-day audit-action counts (today − 6 .. today, UTC),
    /// padded with zeros for days with no activity. Drives an
    /// inline-SVG sparkline above the recent-activity list.
    pub activity_sparkline: Vec<DaySparkPoint>,
    /// Peak count in `activity_sparkline`, pre-computed so the
    /// template can size bars without a `max` filter. `0` when
    /// the audit table is empty or the query failed — the
    /// template clamps the divisor to 1 so the SVG still renders.
    pub activity_sparkline_max: i64,
    /// Total count across `activity_sparkline` — a one-liner
    /// "N actions in the last 7 days" caption.
    pub activity_sparkline_total: i64,
}

/// One bar in the dashboard's 7-day activity sparkline.
#[derive(Serialize)]
pub(crate) struct DaySparkPoint {
    /// `YYYY-MM-DD` of the day this point covers (UTC).
    pub date_iso: String,
    /// Short weekday label (`Mon`, `Tue`, …) — locale-neutral,
    /// suitable for the x-axis legend.
    pub label: &'static str,
    /// Audit-action count for the day; `0` when there's no
    /// activity.
    pub count: i64,
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
    /// Approximate row count from `pg_class.reltuples`, surfaced
    /// on the dashboard as "~ N". Refreshed by `ANALYZE` /
    /// autovacuum, not by every INSERT — operators read it as
    /// approximate, not ground truth. Zero when the table hasn't
    /// been analysed yet (fresh DB) or when the lookup query
    /// failed.
    pub row_estimate: i64,
    /// Exact count of rows whose `created_at > NOW() - INTERVAL
    /// '7 days'`. `None` for models that don't declare a
    /// `created_at` column. Surfaced on the dashboard tile as
    /// "N new this week" when set.
    pub new_this_week: Option<i64>,
    /// 7-day creation series for this model (`today-6 .. today`
    /// UTC, zero-padded). `None` for models without a
    /// `created_at` column. Drives the inline mini-sparkline
    /// next to the row count.
    pub weekly_series: Option<Vec<i64>>,
    /// Peak value in `weekly_series` — pre-computed so the
    /// template sizes bars without a `max` filter. Stays 0
    /// when the series is absent or all-zero; the template
    /// clamps the divisor.
    pub weekly_series_max: i64,
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
///
/// `row_estimates` is keyed by `entry.table`; missing entries default
/// to `0` (fresh DB, never analysed, or query failed). Pass an empty
/// map to skip the per-model count column. `new_this_week` is keyed
/// by `entry.table` and carries an exact count for models that
/// declare `created_at`; absence (the common case for tables without
/// that column) renders the tile without the secondary stat.
pub(crate) fn group_entries_by_app(
    entries: &[AdminEntry],
    row_estimates: &HashMap<&str, i64>,
    new_this_week: &HashMap<&str, i64>,
    per_model_series: &HashMap<&str, Vec<i64>>,
) -> Vec<DashboardApp> {
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
        let estimate = row_estimates.get(entry.table).copied().unwrap_or(0).max(0);
        let week = new_this_week.get(entry.table).copied().map(|n| n.max(0));
        let series = per_model_series.get(entry.table).cloned();
        let series_max = series
            .as_ref()
            .and_then(|s| s.iter().copied().max())
            .unwrap_or(0)
            .max(0);
        app.models.push(DashboardModel {
            admin_name: entry.admin_name,
            display_name: entry.display_name,
            field_count: entry.fields.len(),
            row_estimate: estimate,
            new_this_week: week,
            weekly_series: series,
            weekly_series_max: series_max,
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn dashboard_ctx(
    identity: &Identity,
    admin: &Admin,
    recent_actions: Vec<AdminAction>,
    csrf_token: String,
    row_estimates: &HashMap<&str, i64>,
    new_this_week: &HashMap<&str, i64>,
    per_model_series: &HashMap<&str, Vec<i64>>,
    activity_sparkline: Vec<DaySparkPoint>,
) -> DashboardCtx {
    let recent: Vec<RecentActionCtx> = recent_actions
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

    let apps = group_entries_by_app(
        admin.entries(),
        row_estimates,
        new_this_week,
        per_model_series,
    );
    let total_models = apps.iter().map(|a| a.models.len()).sum();
    let total_rows = apps
        .iter()
        .flat_map(|a| a.models.iter())
        .map(|m| m.row_estimate)
        .sum();
    let recent_actions_count = recent.len();
    let greeting_name = greeting_from_email(&identity.email);
    let activity_sparkline_max = activity_sparkline
        .iter()
        .map(|p| p.count)
        .max()
        .unwrap_or(0)
        .max(0);
    let activity_sparkline_total = activity_sparkline.iter().map(|p| p.count).sum();

    DashboardCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        apps,
        recent_actions: recent,
        flash: None,
        greeting_name,
        total_models,
        total_rows,
        recent_actions_count,
        activity_sparkline,
        activity_sparkline_max,
        activity_sparkline_total,
    }
}

/// Pick a friendly greeting label from an identity email.
///
/// `alice@example.com` → `Alice`, `clinic.admin@…` → `Clinic Admin`,
/// `_root` → `Root`. The result is plain text used in the dashboard
/// welcome header; no HTML escaping required at this layer (minijinja
/// auto-escapes on render).
fn greeting_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    let cleaned: String = local
        .split(['.', '_', '-'])
        .filter(|s| !s.is_empty())
        .map(capitalise)
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.is_empty() {
        "there".to_string()
    } else {
        cleaned
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

        // Authentication events.
        "login_succeeded" => "Signed in",
        "login_failed" => "Failed sign-in",

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
        | "sessions_revoked_by_other"
        | "login_failed" => "badge-danger",

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
    /// URL the toolbar's "Download CSV" link navigates to. Preserves
    /// the current search / filters / sort but drops `page` /
    /// `per_page` (CSV is paginate-free). Routes to the bulk export
    /// endpoint registered at `/admin/<name>/export.csv`.
    pub csv_export_url: String,
    /// Legacy sort-dropdown options — every visible field × {asc, desc}
    /// plus a leading "Default order" reset link. Kept on `ListCtx`
    /// for back-compat with project templates that still consume the
    /// pre-0.21.1 mega-dropdown; the framework's own `list.html` now
    /// uses [`Self::sort_fields`] + the direction toggle below.
    pub sort_options: Vec<SortOptionCtx>,
    /// Toolbar label for the legacy Sort toggle: "Default order" when
    /// no override is in effect, otherwise the active option's label.
    pub current_sort_label: String,
    /// Per-field rows for the redesigned Sort menu — one entry per
    /// sortable column. See [`SortFieldCtx`] for the field shape and
    /// the rationale behind the field-once layout.
    pub sort_fields: Vec<SortFieldCtx>,
    /// Toolbar label for the redesigned Sort toggle: the active
    /// field's `label` (e.g. "Scheduled At"), or "Default order"
    /// when no override is in effect.
    pub current_sort_field_label: String,
    /// Direction-aware copy for the toolbar's direction toggle:
    /// "newest first" / "oldest first" for datetime, "A → Z" /
    /// "Z → A" for text, etc. Empty when no sort override is in
    /// effect (the toggle is hidden in that state).
    pub current_sort_dir_label: &'static str,
    /// URL the direction toggle navigates to — flips the active
    /// field's direction while preserving search / filters /
    /// per-page. Empty when no sort is active.
    pub sort_dir_toggle_link: String,
    /// URL the Sort menu's "Default order" reset row navigates to —
    /// clears the sort while preserving search / filters / per-page.
    /// Same URL as `sort_options[0].link`; surfaced separately so the
    /// new template can render it without indexing into the legacy
    /// vec.
    pub default_sort_link: String,
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
    /// Per-user bookmarkable filter presets for this model. Each
    /// entry carries an `apply_url`, a `delete_url` (per-row form),
    /// and an `is_current` flag set when its `query_string` matches
    /// the current request's. Empty vec hides the Saved dropdown.
    pub saved_filters: Vec<SavedFilterBtnCtx>,
    /// Raw query string of the current request (without leading
    /// `?`). The Saved dropdown's "Save current view" form posts
    /// this verbatim so the bookmark captures the exact state the
    /// operator is looking at.
    pub current_query_string: String,
    pub flash: Option<FlashCtx>,
    /// Adaptive view-layer render of the page, present only when a saved
    /// `ViewSpec` exists *and* a non-`table` `?view=` mode is active. When
    /// `None`, `list.html` renders the legacy table exactly as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive: Option<crate::view_layer::RenderedView>,
    /// Mode-switcher links (Table / List / Cards / Compact), present only when
    /// a saved `ViewSpec` exists. Empty → no switcher (default behaviour).
    pub mode_links: Vec<ModeLink>,
}

/// One link in the list-page view-mode switcher.
#[derive(Serialize)]
pub(crate) struct ModeLink {
    pub slug: &'static str,
    pub label: String,
    pub href: String,
    pub active: bool,
}

#[derive(Serialize)]
pub(crate) struct SavedFilterBtnCtx {
    pub id: i64,
    pub name: String,
    /// `/admin/<model>?<saved query>` — the apply link. Empty
    /// query collapses to `/admin/<model>` (the "default view"
    /// bookmark).
    pub apply_url: String,
    /// `/admin/<model>/saved_filters/<id>/delete` — POST target
    /// for the per-row delete form.
    pub delete_url: String,
    /// `true` when this saved filter's `query_string` matches the
    /// current request. Drives a visual `is-active` marker so the
    /// operator sees which preset they're looking at.
    pub is_current: bool,
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
    /// Pre-escaped HTML fragments wrapping `?q=…` matches in `<mark>`
    /// tags. Only populated for columns the search actually scanned
    /// (`ModelAdmin::search_fields()`) and that contain at least one
    /// match. Cells with no entry here render via the plain string
    /// path in `values`; cells WITH an entry render the HTML fragment
    /// via `|safe` in the template.
    pub highlights: HashMap<String, String>,
}

#[derive(Serialize)]
pub(crate) struct FilterGroupCtx {
    pub field: String,
    pub label: String,
    /// `"chips"` for the discrete-option dropdown (Yes/No, status, …).
    /// `"date_range"` for the two-date-input form. Template branches
    /// on this string. Defaults to `"chips"` so existing call sites
    /// stay correct without touching them.
    #[serde(default = "default_filter_kind")]
    pub kind: &'static str,
    pub options: Vec<FilterOptionCtx>,
    pub current: Option<String>,
    /// URL for the "All" chip — clears this group while keeping every
    /// other piece of list state (search query, other filters, sort).
    /// Pre-baked in `list_ctx` so the template doesn't reproduce the
    /// URL-composition rules. Defaults to empty before `list_ctx`
    /// patches it in (handlers don't construct this field directly).
    #[serde(default)]
    pub all_link: String,
    /// `date_range` only: the currently-selected lower bound
    /// (`YYYY-MM-DD`) and the URL-parameter name the form must
    /// submit it under (`<field>__gte`).
    #[serde(default)]
    pub date_from_name: String,
    #[serde(default)]
    pub date_from_value: String,
    /// Upper bound counterpart — `<field>__lte`.
    #[serde(default)]
    pub date_to_name: String,
    #[serde(default)]
    pub date_to_value: String,
    /// `date_range` only: hidden inputs the form must carry to keep
    /// every other filter / sort / per_page selection across an
    /// Apply submit. Patched by `list_ctx`. Empty until then.
    #[serde(default)]
    pub hidden_pairs: Vec<(String, String)>,
    /// `date_range` only: `true` when at least one bound is set —
    /// drives the "Clear" affordance.
    #[serde(default)]
    pub has_active_range: bool,
    /// `multi_select` only: the currently-checked values, in their
    /// declared order. Used both to render the active-filter pill
    /// (`"active, pending"`) and to round-trip the same selections
    /// as hidden inputs in every neighbouring widget's URL.
    #[serde(default)]
    pub multi_selected: Vec<String>,
    /// `fk_autocomplete` only: the currently-selected FK id (as a
    /// string for HTML attribute use) and the resolved display
    /// label. Both empty when nothing is selected.
    #[serde(default)]
    pub fk_selected_id: String,
    #[serde(default)]
    pub fk_selected_label: String,
    /// `fk_autocomplete` only: relative URL the JS typeahead fetches
    /// for candidate rows (`/admin/_lookup/<target_admin_name>`).
    #[serde(default)]
    pub fk_lookup_url: String,
    /// `fk_autocomplete` only: target model's display name, used
    /// for the input placeholder ("Search Author…").
    #[serde(default)]
    pub fk_target_label: String,
}

fn default_filter_kind() -> &'static str {
    "chips"
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

/// One row in the redesigned Sort menu — *one entry per sortable
/// field*. The toolbar's previous "View" mega-dropdown emitted two
/// rows per field (asc + desc), so a model with eight columns
/// produced a 17-row menu (8 × 2 + Default) that operators had to
/// scan twice for the same label. `SortFieldCtx` collapses that to
/// one row per field; direction is selected by a sibling toggle
/// control whose URL is pre-baked into `sort_dir_toggle_link` on
/// the parent `ListCtx`.
///
/// Each entry carries explicit `asc_link` / `desc_link` URLs (both
/// passed through `build_list_url`) so the field row can render
/// either as a single "sort by X" click that defaults to ascending,
/// or — for power use — as a two-button group exposing both
/// directions inline. Direction-aware *labels*
/// ("newest first" / "A → Z") stay alongside the URLs so the
/// template can surface them without re-running `sort_direction_label`.
#[derive(Serialize)]
pub(crate) struct SortFieldCtx {
    /// Field name as it appears on the URL (`?sort=<field>`). Used
    /// only when the template needs the raw key; rendering reads
    /// `label` instead.
    pub field: String,
    /// Human-readable column label, identical to the table header
    /// label so the menu reads as "Sort by <column name>".
    pub label: String,
    /// URL that activates ascending sort for this field while
    /// preserving search / filters / per-page. Always populated.
    pub asc_link: String,
    /// URL that activates descending sort for this field.
    pub desc_link: String,
    /// Field-type-aware ascending label — "A → Z" for text,
    /// "oldest first" for datetime, "off → on" for bool, etc.
    pub asc_label: &'static str,
    /// Counterpart descending label — "Z → A", "newest first",
    /// "on → off".
    pub desc_label: &'static str,
    /// `true` when this field is the currently sorted column.
    /// Drives the menu's "selected" affordance and tells the
    /// direction toggle which field's URL to anchor against.
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
///
/// `FilePath` / `OptionalFilePath` are stored as strings in the DB
/// (they're `TEXT` paths under the uploads dir) and sort
/// lexicographically — so they share the `String` branch's
/// `A → Z` / `Z → A` copy rather than the generic
/// `ascending` / `descending` fallback that read awkwardly in
/// the sort dropdown.
fn sort_direction_label(
    field_type: super::types::FieldType,
    dir: super::modeladmin::SortDir,
) -> &'static str {
    use super::modeladmin::SortDir;
    use super::types::FieldType::*;
    match (field_type, dir) {
        (DateTime | OptionalDateTime | Date, SortDir::Desc) => "newest first",
        (DateTime | OptionalDateTime | Date, SortDir::Asc) => "oldest first",
        (Time, SortDir::Desc) => "latest first",
        (Time, SortDir::Asc) => "earliest first",
        (
            String | OptionalString | FilePath | OptionalFilePath | Uuid | Email | Phone,
            SortDir::Asc,
        ) => "A → Z",
        (
            String | OptionalString | FilePath | OptionalFilePath | Uuid | Email | Phone,
            SortDir::Desc,
        ) => "Z → A",
        (I32 | I64 | OptionalI64 | F64 | Decimal, SortDir::Desc) => "highest first",
        (I32 | I64 | OptionalI64 | F64 | Decimal, SortDir::Asc) => "lowest first",
        (Bool, SortDir::Asc) => "off → on",
        (Bool, SortDir::Desc) => "on → off",
    }
    // FieldType is #[non_exhaustive] for downstream callers, but
    // within this crate the match must stay exhaustive — adding a
    // catch-all here would mask the missing arm a future variant
    // (e.g. `Numeric`, `Json`, `Uuid`) introduces. Forcing the next
    // contributor to pick deliberate direction copy for the new
    // type is the whole point of this function.
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
/// Append `c` to `out`, escaping the five HTML special chars.
/// Smaller than pulling a crate for the same job — the list view's
/// only escaping need is "treat this cell as plain text inside HTML."
fn push_html_escaped(out: &mut String, c: char) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        '\'' => out.push_str("&#39;"),
        other => out.push(other),
    }
}

fn push_html_escaped_str(out: &mut String, s: &str) {
    for c in s.chars() {
        push_html_escaped(out, c);
    }
}

/// Wrap every occurrence of `term` inside `text` with `<mark>…</mark>`,
/// HTML-escaping the rest. Returns `None` when the term doesn't
/// appear at all so the caller can keep using the plain-string
/// render path (avoids both unnecessary `|safe` work and template
/// branching cost for rows the search never touched).
///
/// **Matching semantics**: ASCII-case-insensitive — `"anna"` matches
/// `"Anna Lindqvist"`. Non-ASCII characters compare exactly
/// (`"Wåhlin"` matches the literal substring `"Wåhlin"` but not
/// `"WÅHLIN"`). The byte-length invariant of `char::to_ascii_lowercase`
/// means byte indices into the original `text` and its
/// ASCII-lowercased twin line up perfectly, so we can slice the
/// original by indices found in the lowercase. Full Unicode case
/// folding is deliberately deferred — most admin search terms are
/// ASCII, and the missed-non-ASCII case still renders the row
/// correctly; only the highlight is absent.
fn highlight_search_match(text: &str, term: &str) -> Option<String> {
    if term.is_empty() {
        return None;
    }
    // ASCII-lowercase both strings while preserving non-ASCII chars
    // unchanged. `char::to_ascii_lowercase` is byte-length-preserving
    // for both ASCII and non-ASCII inputs, so byte offsets carry over.
    let text_lower: String = text.chars().map(|c| c.to_ascii_lowercase()).collect();
    let term_lower: String = term.chars().map(|c| c.to_ascii_lowercase()).collect();
    let term_bytes = term_lower.len();

    let first = text_lower.find(&term_lower)?;
    let mut out = String::with_capacity(text.len() + 16);
    push_html_escaped_str(&mut out, &text[..first]);
    let mut cursor = first;
    loop {
        out.push_str("<mark>");
        push_html_escaped_str(&mut out, &text[cursor..cursor + term_bytes]);
        out.push_str("</mark>");
        cursor += term_bytes;
        match text_lower[cursor..].find(&term_lower) {
            Some(rel) => {
                let next = cursor + rel;
                push_html_escaped_str(&mut out, &text[cursor..next]);
                cursor = next;
            }
            None => break,
        }
    }
    push_html_escaped_str(&mut out, &text[cursor..]);
    Some(out)
}

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

/// CSV export URL: `<list-page filter/search/sort state>` against
/// `/admin/<name>/export.csv`. Mirrors `build_list_url` but drops
/// pagination — CSV ignores it. Kept narrow so the toolbar link is
/// guaranteed-distinct from any list-page link it sits next to.
fn build_csv_export_url(
    admin_name: &str,
    q: &str,
    filters: &[(String, String)],
    sort: Option<(&str, super::modeladmin::SortDir)>,
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
    if parts.is_empty() {
        format!("/admin/{admin_name}/export.csv")
    } else {
        format!("/admin/{}/export.csv?{}", admin_name, parts.join("&"))
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
    saved_filters: Vec<super::saved_filters::SavedFilter>,
    current_query_string: String,
) -> ListCtx {
    let total_pages = total_rows.div_ceil(per_page.max(1)).max(1);

    // ---- URL-state preservation -------------------------------------
    // Every link the list view emits — filter chips, sort options,
    // pagination, header sort arrows — composes its href via
    // `build_list_url` so clicking one widget never silently drops
    // the others. `active_filter_pairs` is the canonical view of
    // currently-set filters; widgets derive their override URLs from
    // a copy of it.
    // Active filter pairs span both the chip groups (one pair) and
    // the date-range groups (one pair per set bound, encoded under
    // `<field>__gte` / `<field>__lte`). Every state-preserving link
    // — chip All / chip option / sort / per-page / pagination — is
    // built from this flat list, so a single source of truth means
    // no widget can silently drop another widget's selection.
    let active_filter_pairs: Vec<(String, String)> = filters
        .iter()
        .flat_map(|g| {
            let mut pairs: Vec<(String, String)> = Vec::new();
            if let Some(v) = g.current.as_ref() {
                pairs.push((g.field.clone(), v.clone()));
            }
            if !g.date_from_value.is_empty() {
                pairs.push((g.date_from_name.clone(), g.date_from_value.clone()));
            }
            if !g.date_to_value.is_empty() {
                pairs.push((g.date_to_name.clone(), g.date_to_value.clone()));
            }
            // Multi-select round-trips one repeated pair per checked
            // value — `?status=active&status=pending`. Order matches
            // the field's declared `choices` so the rendered URL is
            // stable across reloads.
            for v in &g.multi_selected {
                pairs.push((g.field.clone(), v.clone()));
            }
            pairs
        })
        .collect();
    let active_sort_ref: Option<(&str, super::modeladmin::SortDir)> =
        active_sort.as_ref().map(|(c, d)| (c.as_str(), *d));

    // Patch each filter group's URLs in-place. "All" drops this
    // group from the active set; chip options replace the group's
    // current value; date-range forms keep every *other* pair as
    // hidden inputs and submit with their own field names. Page
    // resets to 1 — page N of one filter rarely matches up with
    // page N of another.
    for group in &mut filters {
        let other: Vec<(String, String)> = active_filter_pairs
            .iter()
            .filter(|(field, _)| {
                // A chip group owns one key (`field`); a date-range
                // group owns two (`field__gte`, `field__lte`); a
                // multi-select group owns its field name (with
                // repeated entries). The "other" list excludes
                // whichever keys this group controls so its own
                // URLs replace rather than duplicate.
                if group.kind == "date_range" {
                    field != &group.date_from_name && field != &group.date_to_name
                } else {
                    field != &group.field
                }
            })
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
        // Chip options (BoolYesNo): each chip composes a URL with
        // this group's value replaced. Multi-select uses the same
        // `options` slot, but the form handles state preservation
        // via hidden inputs + checkboxes, so per-option links here
        // would be misleading — leave them empty.
        if group.kind == "chips" {
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
        // Date-range, multi-select, and FK-autocomplete all render
        // as forms; each needs every *other* filter / sort / per_page
        // pair as hidden inputs so submitting one widget preserves
        // the rest of the list-view state.
        if group.kind == "date_range"
            || group.kind == "multi_select"
            || group.kind == "fk_autocomplete"
        {
            group.hidden_pairs = other;
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

    // CSV export URL: same filter/search/sort state as the list
    // page but addressed to the export endpoint. Page / per_page
    // are dropped — the CSV ignores pagination entirely, so
    // carrying them would just clutter the link.
    let csv_export_url = build_csv_export_url(
        entry.admin_name,
        &search_query,
        &active_filter_pairs,
        active_sort_ref,
    );

    // Display-ready pills for the "active filters" strip. Each pill
    // resolves the option's friendly `value_label` from the group's
    // option list (so a stored "true" reads as "Yes", etc.), and its
    // `remove_link` drops only this filter — search query, sort, and
    // every other filter stay intact.
    let active_filter_pills: Vec<ActiveFilterPillCtx> = filters
        .iter()
        .filter_map(|g| {
            if g.kind == "fk_autocomplete" {
                if g.fk_selected_id.is_empty() {
                    return None;
                }
                // Pill text uses the hydrated row label so the user
                // sees "Author: Anna Lindqvist" rather than "Author:
                // 42". A row that's been deleted falls back to
                // `#<id>` (set during hydration in the handler).
                let other: Vec<(String, String)> = active_filter_pairs
                    .iter()
                    .filter(|(field, _)| field != &g.field)
                    .cloned()
                    .collect();
                return Some(ActiveFilterPillCtx {
                    label: g.label.clone(),
                    value_label: if g.fk_selected_label.is_empty() {
                        format!("#{}", g.fk_selected_id)
                    } else {
                        g.fk_selected_label.clone()
                    },
                    remove_link: build_list_url(
                        entry.admin_name,
                        &search_query,
                        &other,
                        active_sort_ref,
                        1,
                        per_page_override,
                    ),
                });
            }
            if g.kind == "multi_select" {
                if g.multi_selected.is_empty() {
                    return None;
                }
                // One combined pill listing every checked value. The
                // remove link drops every pair under this field name
                // — search query, sort, every other filter survives.
                let value_label = g.multi_selected.join(", ");
                let other: Vec<(String, String)> = active_filter_pairs
                    .iter()
                    .filter(|(field, _)| field != &g.field)
                    .cloned()
                    .collect();
                return Some(ActiveFilterPillCtx {
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
                });
            }
            if g.kind == "date_range" {
                if !g.has_active_range {
                    return None;
                }
                // One combined pill — "from → to", "from only", or
                // "≤ to". The remove link drops both bounds at once.
                let value_label = match (g.date_from_value.as_str(), g.date_to_value.as_str()) {
                    ("", "") => return None,
                    ("", to) => format!("≤ {to}"),
                    (from, "") => format!("≥ {from}"),
                    (from, to) => format!("{from} → {to}"),
                };
                let other: Vec<(String, String)> = active_filter_pairs
                    .iter()
                    .filter(|(field, _)| field != &g.date_from_name && field != &g.date_to_name)
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
            } else {
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
            }
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
        // Hide the primary-key `id` column by default — the row links to the
        // record and the raw id is rarely useful for scanning. A project that
        // wants it back lists "id" explicitly in `list_display`. Guard the
        // edge case of an id-only model so the table never loses all columns.
        let shown: Vec<&AdminField> = entry.fields.iter().filter(|f| f.name != "id").collect();
        if shown.is_empty() {
            entry.fields.iter().collect()
        } else {
            shown
        }
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

    // Redesigned Sort menu — one entry per sortable field. The legacy
    // `sort_options` vec stays populated above for project templates
    // that haven't migrated; the framework's own list template now
    // renders the field menu plus a direction toggle, which the
    // template builds against `sort_fields` + `sort_dir_toggle_link`.
    let sort_fields: Vec<SortFieldCtx> = visible_fields
        .iter()
        .map(|f| {
            let asc_link = build_list_url(
                entry.admin_name,
                &search_query,
                &active_filter_pairs,
                Some((f.name, SortDir::Asc)),
                1,
                per_page_override,
            );
            let desc_link = build_list_url(
                entry.admin_name,
                &search_query,
                &active_filter_pairs,
                Some((f.name, SortDir::Desc)),
                1,
                per_page_override,
            );
            let is_active = matches!(
                &active_sort,
                Some((col, _)) if col == f.name
            );
            SortFieldCtx {
                field: f.name.to_string(),
                label: f.label.to_string(),
                asc_link,
                desc_link,
                asc_label: sort_direction_label(f.field_type, SortDir::Asc),
                desc_label: sort_direction_label(f.field_type, SortDir::Desc),
                is_active,
            }
        })
        .collect();

    // `default_sort_link` mirrors `sort_options[0].link` (the
    // "Default order" reset). Surfacing it as a standalone field
    // lets the new template emit the link without indexing into
    // the legacy vec, which keeps the two layouts independently
    // editable.
    let default_sort_link = build_list_url(
        entry.admin_name,
        &search_query,
        &active_filter_pairs,
        None,
        1,
        per_page_override,
    );

    // Direction-toggle pair: the active field's label drives the
    // toolbar Sort chip's caption; the active field's *opposite*
    // direction URL drives the toggle button. When no sort is
    // active, both stay empty and the template hides the toggle.
    let (current_sort_field_label, current_sort_dir_label, sort_dir_toggle_link) =
        match &active_sort {
            Some((col, dir)) => {
                let field = sort_fields.iter().find(|sf| sf.field == *col);
                let label = field
                    .map(|f| f.label.clone())
                    .unwrap_or_else(|| col.clone());
                let (dir_label, toggle_link) = match (field, dir) {
                    (Some(f), SortDir::Asc) => (f.desc_label, f.desc_link.clone()),
                    (Some(f), SortDir::Desc) => (f.asc_label, f.asc_link.clone()),
                    // `active_sort` references a column not in
                    // `visible_fields` (rare: column dropped from
                    // `list_display` between requests). Fall back
                    // to generic labels + an empty toggle URL so
                    // the template renders without crashing.
                    (None, _) => ("", String::new()),
                };
                (label, dir_label, toggle_link)
            }
            None => ("Default order".to_string(), "", String::new()),
        };

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
    // Pre-compute the lowercase search term once. Each row × column
    // would otherwise redo the allocation; for a 100-row × 5-searched-
    // column page that's 500 redundant lowercases.
    let search_term_trimmed = search_query.trim();
    let search_term_for_highlight: Option<String> = if search_term_trimmed.is_empty() {
        None
    } else {
        Some(search_term_trimmed.to_string())
    };
    // Columns whose cells the highlighter should scan — only those
    // the SQL search clause actually ILIKE'd. Highlighting a column
    // that wasn't searched would be a lie.
    let searched_columns: std::collections::HashSet<&str> =
        entry.search_fields.iter().copied().collect();
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
                let mut highlights: HashMap<String, String> = HashMap::new();
                let cell_links = r.cell_links;
                for (i, cell) in r.cells.into_iter().enumerate() {
                    if let Some(name) = field_names.get(i) {
                        // Skip the "id" key so the explicit struct field
                        // wins on serialization.
                        if *name == "id" {
                            continue;
                        }
                        // Build the highlight HTML BEFORE moving `cell`
                        // into the typed value below. Only emit a
                        // fragment when the column was actually scanned
                        // by the search clause AND at least one match
                        // landed; otherwise the template falls back to
                        // the plain string path.
                        if let Some(term) = &search_term_for_highlight {
                            let is_bool =
                                matches!(field_types.get(i), Some(crate::admin::FieldType::Bool));
                            if !is_bool && searched_columns.contains(*name) {
                                if let Some(html) = highlight_search_match(&cell, term) {
                                    highlights.insert((*name).to_string(), html);
                                }
                            }
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
                    highlights,
                }
            })
            .collect(),
        search_query,
        active_filter_count: filters
            .iter()
            .filter(|g| g.current.is_some() || g.has_active_range || !g.multi_selected.is_empty())
            .count(),
        active_filter_pairs,
        active_filter_pills,
        clear_all_filters_link,
        csv_export_url,
        filters,
        sort_options,
        current_sort_label,
        sort_fields,
        current_sort_field_label,
        current_sort_dir_label,
        sort_dir_toggle_link,
        default_sort_link,
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
        saved_filters: saved_filters
            .into_iter()
            .map(|sf| {
                let apply_url = if sf.query_string.is_empty() {
                    format!("/admin/{}", entry.admin_name)
                } else {
                    format!("/admin/{}?{}", entry.admin_name, sf.query_string)
                };
                let is_current = sf.query_string == current_query_string;
                SavedFilterBtnCtx {
                    delete_url: format!(
                        "/admin/{}/saved_filters/{}/delete",
                        entry.admin_name, sf.id
                    ),
                    apply_url,
                    name: sf.name,
                    is_current,
                    id: sf.id,
                }
            })
            .collect(),
        current_query_string,
        flash: None,
        // Set by the handler after construction when a saved ViewSpec exists.
        adaptive: None,
        mode_links: Vec::new(),
    }
}

/// Preserve the current query string but swap (or add) the `view=` param —
/// used for the mode-switcher hrefs so changing layout keeps the active
/// search / filters / sort / page.
fn set_view_param(admin_name: &str, query: &str, slug: &str) -> String {
    let mut parts: Vec<String> = query
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("view="))
        .map(str::to_string)
        .collect();
    parts.push(format!("view={slug}"));
    format!("/admin/{}?{}", admin_name, parts.join("&"))
}

/// Build the adaptive view + mode-switcher links for a list page from a saved
/// spec. Returns `(None, links)` for `table` mode (the legacy table renders);
/// `(Some(view), links)` for `list`/`cards`/`compact`. Row values come from
/// the same `entry.fields`-keyed cells the legacy `ListRowCtx` uses, so every
/// field the spec renders has a value, and each row carries its id for
/// click-through. Called by the list handler **before** `list_ctx` consumes
/// the rows.
pub(crate) fn adaptive_for_list(
    admin_name: &str,
    fields: &[AdminField],
    spec: &crate::view_layer::ViewSpec,
    rows: &[ListRow],
    requested_view: Option<&str>,
    query: &str,
) -> (Option<crate::view_layer::RenderedView>, Vec<ModeLink>) {
    use crate::view_layer::{render_view_with_ids, RowData, ViewMode};

    let mode = spec.resolve_mode(requested_view);
    let mode_links = spec
        .allowed_modes
        .iter()
        .map(|m| ModeLink {
            slug: m.slug(),
            label: humanise_field(m.slug()),
            href: set_view_param(admin_name, query, m.slug()),
            active: *m == mode,
        })
        .collect();

    let adaptive = if mode == ViewMode::Table {
        None
    } else {
        let data: Vec<(i64, RowData)> = rows
            .iter()
            .map(|r| {
                let mut rd = RowData::new();
                for (f, val) in fields.iter().zip(r.cells.iter()) {
                    // Humanise booleans for display, matching the legacy
                    // table's Yes/No pill (the renderer only sees strings).
                    let display = match (f.field_type, val.as_str()) {
                        (FieldType::Bool, "true") => "Yes".to_string(),
                        (FieldType::Bool, "false") => "No".to_string(),
                        _ => val.clone(),
                    };
                    rd.insert(f.name.to_string(), display);
                }
                (r.id, rd)
            })
            .collect();
        Some(render_view_with_ids(spec, mode, &data))
    };

    (adaptive, mode_links)
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
    /// Related-children sections rendered below the action bar
    /// when `mode == "edit"`. Always empty on `"new"` (no parent
    /// id yet to filter children on). Each entry is one
    /// declared [`super::modeladmin::Inline`] resolved to a
    /// concrete row list.
    pub inlines: Vec<FormInlineCtx>,
    /// `true` when at least one field on the form is a
    /// `<input type="file">`. Drives
    /// `enctype="multipart/form-data"` on the form open tag;
    /// non-file forms keep the default urlencoded encoding so
    /// the existing fast path on `Request::form()` stays in
    /// effect.
    pub has_file_field: bool,
    pub flash: Option<FlashCtx>,
}

/// One resolved [`super::modeladmin::Inline`] — a child-model
/// section under the parent's edit form. Read-only in v1: a table
/// of click-through rows plus an "Add new" affordance that lands
/// the operator on the child's normal new-form.
#[derive(Serialize)]
pub(crate) struct FormInlineCtx {
    pub label: String,
    /// Pluralised display name of the child for empty-state copy.
    pub target_display_name: String,
    /// Child's admin slug. Drives the "Add new …" and "View all"
    /// click-through URLs.
    pub target_admin_name: String,
    pub rows: Vec<FormInlineRowCtx>,
    /// `true` when the child query was capped at `max_rows` —
    /// drives the "…and N more" link to the child's list page
    /// pre-filtered to this parent.
    pub has_more: bool,
    /// Total matching rows on the child table. Renders as
    /// "Showing X of Y" when `has_more` is `true`.
    pub total: i64,
    /// `/admin/<child>/new` — link target for "Add new …". v1
    /// doesn't prefill the parent FK; the operator picks it from
    /// the child's normal relation widget.
    pub add_url: String,
    /// `/admin/<child>?<fk_field>=<parent_id>` — link target for
    /// "View all …" so the operator can paginate / search beyond
    /// the inline cap.
    pub list_url: String,
}

#[derive(Serialize)]
pub(crate) struct FormInlineRowCtx {
    pub id: i64,
    pub label: String,
    pub edit_url: String,
    pub delete_url: String,
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
///
/// Whole-word acronym recognition: each underscore-separated segment
/// is checked against [`HUMANISE_ACRONYMS`] before being
/// title-cased, so `id` → `ID`, `email_id` → `Email ID`,
/// `mfa_secret_key_id` → `MFA Secret Key ID`. Words *containing* but
/// not *being* an acronym (`video` is not `vIDeo`) are left to the
/// default first-letter-uppercase rule.
fn humanise_field(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(s.len());
    let mut first_segment = true;
    for segment in s.split('_') {
        if !first_segment {
            out.push(' ');
        }
        first_segment = false;
        let lower = segment.to_ascii_lowercase();
        if HUMANISE_ACRONYMS.contains(&lower.as_str()) {
            // Whole-word acronym — emit as uppercase regardless
            // of the source case (`Id`, `ID`, and `id` all
            // become `ID`).
            out.push_str(&lower.to_ascii_uppercase());
        } else {
            let mut chars = segment.chars();
            if let Some(first) = chars.next() {
                out.push(first.to_ascii_uppercase());
                for c in chars {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Acronyms that should be fully uppercase in humanised labels.
///
/// Kept tight and audit-able rather than open-ended: every entry
/// is one a Postgres admin framework realistically meets in column
/// names. Adding to this list also requires updating the byte-for-
/// byte mirror in `rustio_admin_macros::HUMANISE_ACRONYMS` — the
/// macros crate cannot depend on this crate (proc-macro cycle), so
/// the two lists are intentionally duplicated.
pub(crate) const HUMANISE_ACRONYMS: &[&str] = &[
    "id", "ip", "url", "uri", "api", "uuid", "mfa", "csv", "sql", "html", "http", "https", "json",
    "tls", "ssl", "smtp", "xml",
];

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
    inlines: Vec<FormInlineCtx>,
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

    let sections = if entry.fieldsets.is_empty() {
        group_fields_into_sections(fields)
    } else {
        group_fields_by_fieldsets(fields, entry.fieldsets)
    };

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
        inlines,
        has_file_field: entry.fields.iter().any(|f| {
            matches!(
                f.field_type,
                crate::admin::FieldType::FilePath | crate::admin::FieldType::OptionalFilePath
            )
        }),
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

/// Group the rendered fields according to the project-supplied
/// [`super::modeladmin::Fieldset`] slice. Each fieldset becomes a
/// [`FormSection`] with the declared title; fields are emitted in
/// the order the project listed them inside each fieldset. Names
/// the project misspelt (no matching `FormField`) are silently
/// skipped — they would otherwise force a runtime panic on
/// production traffic, and the missing field is visible to the
/// project by absence in the rendered form. Fields that the
/// project owns (in `entry.fields`) but did not list in any
/// fieldset are appended to a trailing untitled "Other" section so
/// the form is never silently incomplete; the project promotes them
/// into a real fieldset to take ownership of their placement.
fn group_fields_by_fieldsets(
    mut fields: Vec<FormField>,
    fieldsets: &'static [super::modeladmin::Fieldset],
) -> Vec<FormSection> {
    let mut sections: Vec<FormSection> = Vec::with_capacity(fieldsets.len() + 1);
    for fs in fieldsets {
        let mut grouped = Vec::with_capacity(fs.fields.len());
        for &name in fs.fields {
            if let Some(pos) = fields.iter().position(|f| f.name == name) {
                grouped.push(fields.remove(pos));
            }
        }
        if !grouped.is_empty() {
            sections.push(FormSection {
                title: Some(fs.title),
                fields: grouped,
            });
        }
    }
    if !fields.is_empty() {
        sections.push(FormSection {
            title: Some("Other"),
            fields,
        });
    }
    sections
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
        I32 | I64 | OptionalI64 | F64 | Decimal => ("input", "number"),
        DateTime | OptionalDateTime => ("input", "datetime-local"),
        Date => ("input", "date"),
        Time => ("input", "time"),
        Email => ("input", "email"),
        Phone => ("input", "tel"),
        FilePath | OptionalFilePath => ("file", "file"),
        Uuid | String | OptionalString => ("input", "text"),
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

pub(crate) fn pick_display_index(
    fields: &[AdminField],
    display_field: Option<&str>,
) -> Option<usize> {
    if let Some(preferred) = display_field {
        if let Some(i) = fields.iter().position(|f| f.name == preferred) {
            return Some(i);
        }
    }
    // Fallback ladder — checked in order, first match wins. `name` and
    // `title` cover Library / Catalogue / Article / Movie shapes;
    // `full_name` covers Person / Customer / Patient / Employee
    // shapes (the universal Patron / User column across both shipped
    // examples); `email` is the last resort for user-like rows that
    // skipped full_name. A row whose schema fits none of these
    // patterns still renders as `#<id>` from the caller's fallback.
    for fallback in ["name", "title", "full_name", "email"] {
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
    /// `YYYY-MM-DD` of the audit timestamp (UTC). Drives the
    /// day-divider rows in the history templates — when this
    /// changes between consecutive entries, `is_new_day` is
    /// `true` and the template emits a divider before the row.
    pub date_iso: String,
    /// `true` when this entry's `date_iso` differs from the
    /// previous entry's (or this is the first entry). Computed by
    /// `map_audit_actions`; template uses it to render a
    /// `YYYY-MM-DD` divider above the row.
    pub is_new_day: bool,
    pub user_id: i64,
    pub user_email: String,
    pub action_type: String,
    pub label: &'static str,
    pub pill_class: &'static str,
    pub model_name: String,
    pub model_admin_name: String,
    pub object_id: i64,
    pub summary: String,
    pub ip_address: String,
    /// Per-field before/after diff extracted from
    /// `audit_action.metadata.changes`. Empty when the row carries
    /// no diff (create / delete / pre-diff-feature updates). The
    /// object-history template iterates this to render a tight
    /// `<dl>` of changed columns under the audit row.
    pub changes: Vec<HistoryChangeCtx>,
}

// ---- /admin/apis HTML index ---------------------------------------------

#[derive(Serialize)]
pub(crate) struct ApisIndexCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    /// Sidebar nav models — same shape every other authenticated
    /// page uses.
    pub entries: Vec<SidebarEntry>,
    /// One row per non-core registered model.
    pub apis: Vec<ApiEntryCtx>,
    pub openapi_url: &'static str,
}

#[derive(Serialize)]
pub(crate) struct ApiEntryCtx {
    pub admin_name: &'static str,
    pub singular_name: &'static str,
    pub display_name: &'static str,
    /// Path documenting the model in OpenAPI shorthand — same
    /// shape the spec lists. Surfaced verbatim in the table.
    pub list_path: String,
    pub detail_path: String,
    pub field_count: usize,
    /// `(field_name, type_label, nullable)` triples for the
    /// compact field table under each model. `type_label` is the
    /// human-readable form ("integer", "string", "date-time",
    /// "boolean", "file path") so the page reads at a glance
    /// without leaking the internal `FieldType` Debug shape.
    pub fields: Vec<ApiFieldCtx>,
}

#[derive(Serialize)]
pub(crate) struct ApiFieldCtx {
    pub name: &'static str,
    pub type_label: &'static str,
    pub nullable: bool,
}

/// Human-friendly type label for one `AdminField`. Pulled out as
/// a small free fn so the API-index renderer and any future
/// schema dumper share one projection.
pub(crate) fn api_field_type_label(field: &AdminField) -> &'static str {
    use crate::admin::types::FieldType::*;
    match field.field_type {
        Bool => "boolean",
        I32 => "integer (i32)",
        I64 | OptionalI64 => "integer (i64)",
        F64 => "float (f64)",
        Decimal => "decimal",
        String | OptionalString => "string",
        DateTime | OptionalDateTime => "date-time",
        Date => "date",
        Time => "time",
        Uuid => "uuid",
        Email => "email",
        Phone => "phone",
        FilePath | OptionalFilePath => "file path",
    }
}

#[derive(Serialize)]
pub(crate) struct DocsIndexCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    pub docs: Vec<DocSummaryCtx>,
}

#[derive(Serialize)]
pub(crate) struct DocSummaryCtx {
    pub slug: &'static str,
    pub title: &'static str,
}

#[derive(Serialize)]
pub(crate) struct DocPageCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub doc_title: &'static str,
    /// Pre-rendered HTML fragment from `docs::render_markdown`.
    /// The template marks it `|safe` because this is the
    /// trusted boundary — markdown source is framework-owned,
    /// never user-supplied.
    pub body_html: String,
}

pub(crate) fn docs_index_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
) -> DocsIndexCtx {
    DocsIndexCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "Framework docs",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        docs: crate::admin::docs::EMBEDDED_DOCS
            .iter()
            .map(|d| DocSummaryCtx {
                slug: d.slug,
                title: d.title,
            })
            .collect(),
    }
}

pub(crate) fn doc_page_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
    doc: &crate::admin::docs::EmbeddedDoc,
) -> DocPageCtx {
    DocPageCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: format!("Docs — {}", doc.title),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        doc_title: doc.title,
        body_html: crate::admin::docs::render_markdown(doc.source),
    }
}

#[derive(Serialize)]
pub(crate) struct CsvImportResultCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub admin_name: String,
    pub display_name: String,
    pub total: usize,
    pub inserted: usize,
    pub failed: usize,
    pub outcomes: Vec<CsvOutcomeCtx>,
    /// Postgres table backing this model — used in the generated
    /// `ALTER TABLE` migration on the result page.
    pub table: String,
    /// CSV columns that don't match a declared field; their values
    /// were skipped. Empty when every header column matched.
    pub ignored_columns: Vec<String>,
    /// One generated field-definition per ignored column, so the
    /// developer can capture that data instead of losing it.
    pub suggested_fields: Vec<SuggestedFieldCtx>,
    /// Ready-to-run `ALTER TABLE` covering every ignored column.
    /// Empty when there are none.
    pub migration_sql: String,
}

#[derive(Serialize)]
pub(crate) struct CsvOutcomeCtx {
    pub row_number: usize,
    pub status: &'static str, // "inserted" | "failed"
    pub id: Option<i64>,
    pub errors: Vec<String>,
}

/// Template view of one [`crate::admin::csv_import::SuggestedField`].
#[derive(Serialize)]
pub(crate) struct SuggestedFieldCtx {
    pub column: String,
    pub sql_type: &'static str,
    pub rust_field: String,
    pub from_row: String,
    pub insert_value: String,
}

pub(crate) fn csv_import_result_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
    entry: &AdminEntry,
    report: crate::admin::csv_import::ImportReport,
) -> CsvImportResultCtx {
    use crate::admin::csv_import::RowOutcome;
    let outcomes = report
        .outcomes
        .into_iter()
        .map(|o| match o {
            RowOutcome::Inserted { row_number, id } => CsvOutcomeCtx {
                row_number,
                status: "inserted",
                id: Some(id),
                errors: Vec::new(),
            },
            RowOutcome::Failed { row_number, errors } => CsvOutcomeCtx {
                row_number,
                status: "failed",
                id: None,
                errors,
            },
        })
        .collect();
    // Turn the skipped columns into a generated migration + model
    // snippet so a mismatched upload becomes a guided next step,
    // not a dead end.
    let suggested: Vec<crate::admin::csv_import::SuggestedField> = report
        .ignored_columns
        .iter()
        .map(|c| crate::admin::csv_import::suggest_field(c))
        .collect();

    let migration_sql = if suggested.is_empty() {
        String::new()
    } else {
        let cols = suggested
            .iter()
            .map(|s| format!("  ADD COLUMN \"{}\" {}", s.column, s.sql_type))
            .collect::<Vec<_>>()
            .join(",\n");
        format!("ALTER TABLE {}\n{};", entry.table, cols)
    };

    let suggested_fields = suggested
        .into_iter()
        .map(|s| SuggestedFieldCtx {
            column: s.column,
            sql_type: s.sql_type,
            rust_field: s.rust_field,
            from_row: s.from_row,
            insert_value: s.insert_value,
        })
        .collect();

    CsvImportResultCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: format!("CSV import — {}", entry.display_name),
        admin_name: entry.admin_name.to_string(),
        display_name: entry.display_name.to_string(),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        total: report.total,
        inserted: report.inserted,
        failed: report.failed,
        outcomes,
        table: entry.table.to_string(),
        ignored_columns: report.ignored_columns,
        suggested_fields,
        migration_sql,
    }
}

#[derive(Serialize)]
pub(crate) struct NotificationsCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    pub notifications: Vec<NotificationRowCtx>,
    pub unread_count: i64,
}

#[derive(Serialize)]
pub(crate) struct NotificationRowCtx {
    pub id: i64,
    pub message: String,
    pub url: String,
    /// `true` while the notification is unread; controls the
    /// row's visual emphasis in the template.
    pub unread: bool,
    pub when_relative: String,
    pub created_iso: String,
}

pub(crate) fn notifications_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
    notifications: Vec<crate::admin::notifications::Notification>,
) -> NotificationsCtx {
    let unread_count = notifications.iter().filter(|n| n.read_at.is_none()).count() as i64;
    let rows = notifications
        .into_iter()
        .map(|n| NotificationRowCtx {
            id: n.id,
            message: n.message,
            url: n.url,
            unread: n.read_at.is_none(),
            when_relative: relative_time(n.created_at),
            created_iso: n.created_at.to_rfc3339(),
        })
        .collect();
    NotificationsCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "Notifications",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        notifications: rows,
        unread_count,
    }
}

#[derive(Serialize)]
pub(crate) struct FeatureFlagsCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    pub flags: Vec<FeatureFlagCtx>,
    pub flash: Option<FlashCtx>,
}

#[derive(Serialize)]
pub(crate) struct FeatureFlagCtx {
    pub key: String,
    pub enabled: bool,
    pub description: String,
    pub created_iso: String,
    pub updated_iso: String,
}

pub(crate) fn feature_flags_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
    flags: Vec<crate::admin::feature_flags::FeatureFlag>,
    flash: Option<FlashCtx>,
) -> FeatureFlagsCtx {
    FeatureFlagsCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "Feature flags",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        flags: flags
            .into_iter()
            .map(|f| FeatureFlagCtx {
                key: f.key,
                enabled: f.enabled,
                description: f.description,
                created_iso: f.created_at.to_rfc3339(),
                updated_iso: f.updated_at.to_rfc3339(),
            })
            .collect(),
        flash,
    }
}

// ---- View designer (/admin/dev/view-designer) ------------------------------

/// A `slug` + human `label` pair for building `<select>` options in the
/// designer without hard-coding the role/mode lists in a template.
#[derive(Serialize)]
pub(crate) struct SlugLabel {
    pub slug: String,
    pub label: String,
}

/// One model link on the designer index.
#[derive(Serialize)]
pub(crate) struct DesignerModelCtx {
    pub admin_name: &'static str,
    pub display_name: &'static str,
    /// `true` when a spec has been saved; `false` means the editor will show
    /// the inferred draft.
    pub is_saved: bool,
}

/// The designer index — every project model, with its saved/draft status.
#[derive(Serialize)]
pub(crate) struct ViewDesignerIndexCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    pub models: Vec<DesignerModelCtx>,
}

/// One editable field row in the designer.
#[derive(Serialize)]
pub(crate) struct DesignerFieldCtx {
    pub name: String,
    pub label: String,
    /// The field's current role slug (matched against `role_options`).
    pub role: String,
    pub priority: i32,
    pub filterable: bool,
}

/// The per-model designer editor: editable field rows plus a live preview
/// rendered through the exact runtime [`render_view`](crate::view_layer::render_view).
#[derive(Serialize)]
pub(crate) struct ViewDesignerCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: String,
    pub entries: Vec<SidebarEntry>,
    pub admin_name: &'static str,
    pub display_name: &'static str,
    /// `true` when editing a saved spec, `false` when seeding from inference.
    pub is_saved: bool,
    /// Current default-mode slug (matched against `mode_options`).
    pub default_mode: String,
    pub mode_options: Vec<SlugLabel>,
    pub role_options: Vec<SlugLabel>,
    pub fields: Vec<DesignerFieldCtx>,
    /// Live preview produced by the runtime renderer from the effective spec.
    pub preview: crate::view_layer::RenderedView,
    pub flash: Option<FlashCtx>,
}

/// Build the designer index context.
pub(crate) fn view_designer_index_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
    saved: &std::collections::HashSet<String>,
) -> ViewDesignerIndexCtx {
    ViewDesignerIndexCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "View designer",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        models: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(|e| DesignerModelCtx {
                admin_name: e.admin_name,
                display_name: e.display_name,
                is_saved: saved.contains(e.admin_name),
            })
            .collect(),
    }
}

fn role_options() -> Vec<SlugLabel> {
    crate::view_layer::FieldRole::all()
        .iter()
        .map(|r| SlugLabel {
            slug: r.slug().to_string(),
            label: humanise_field(r.slug()),
        })
        .collect()
}

fn mode_options() -> Vec<SlugLabel> {
    [
        crate::view_layer::ViewMode::Table,
        crate::view_layer::ViewMode::List,
        crate::view_layer::ViewMode::Cards,
        crate::view_layer::ViewMode::Compact,
    ]
    .iter()
    .map(|m| SlugLabel {
        slug: m.slug().to_string(),
        label: humanise_field(m.slug()),
    })
    .collect()
}

/// A placeholder cell value for the preview, so the designer can show layout
/// without fetching real rows (keeps the page side-effect-free). Uses a
/// declared choice when present, a date for temporal fields, else a labelled
/// sample.
fn sample_value(field: &AdminField, idx: usize) -> String {
    if let Some(choices) = field.choices {
        if !choices.is_empty() {
            return choices[idx % choices.len()].to_string();
        }
    }
    match field.field_type {
        FieldType::Bool => if idx.is_multiple_of(2) { "Yes" } else { "No" }.to_string(),
        FieldType::DateTime | FieldType::OptionalDateTime | FieldType::Date | FieldType::Time => {
            "2026-01-15".to_string()
        }
        _ => format!("{} {}", field.label, idx + 1),
    }
}

/// Build the per-model designer editor context from the *effective* spec
/// (saved, or an inferred draft). The preview reuses the runtime renderer.
pub(crate) fn view_designer_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
    entry: &AdminEntry,
    spec: &crate::view_layer::ViewSpec,
    is_saved: bool,
    flash: Option<FlashCtx>,
) -> ViewDesignerCtx {
    let fields = spec
        .fields
        .iter()
        .map(|f| DesignerFieldCtx {
            name: f.field_name.clone(),
            label: f
                .label
                .clone()
                .unwrap_or_else(|| humanise_field(&f.field_name)),
            role: f.role.slug().to_string(),
            priority: f.priority,
            filterable: f.filterable,
        })
        .collect();

    // Two placeholder rows keyed by field name, fed through render_view.
    let rows: Vec<crate::view_layer::RowData> = (0..2)
        .map(|idx| {
            entry
                .fields
                .iter()
                .map(|f| (f.name.to_string(), sample_value(f, idx)))
                .collect()
        })
        .collect();
    let preview = crate::view_layer::render_view(spec, spec.default_mode, &rows);

    ViewDesignerCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: format!("View designer · {}", entry.display_name),
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        admin_name: entry.admin_name,
        display_name: entry.display_name,
        is_saved,
        default_mode: spec.default_mode.slug().to_string(),
        mode_options: mode_options(),
        role_options: role_options(),
        fields,
        preview,
        flash,
    }
}

#[derive(Serialize)]
pub(crate) struct HealthCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    pub checks: Vec<HealthCheckCtx>,
    /// `true` when every check returned `Ok` — drives the
    /// overall-status banner.
    pub all_ok: bool,
    /// Pre-computed counts for the banner copy.
    pub ok_count: usize,
    pub warn_count: usize,
    pub error_count: usize,
}

#[derive(Serialize)]
pub(crate) struct HealthCheckCtx {
    pub label: &'static str,
    /// `ok` / `warn` / `error` — mirrors `HealthStatus::as_str`.
    pub status: &'static str,
    pub message: String,
}

pub(crate) fn health_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
    checks: Vec<crate::admin::health_dashboard::HealthCheck>,
) -> HealthCtx {
    use crate::admin::health_dashboard::HealthStatus;
    let mut ok_count = 0;
    let mut warn_count = 0;
    let mut error_count = 0;
    for c in &checks {
        match c.status {
            HealthStatus::Ok => ok_count += 1,
            HealthStatus::Warn => warn_count += 1,
            HealthStatus::Error => error_count += 1,
        }
    }
    let all_ok = warn_count == 0 && error_count == 0;
    let ctx_checks: Vec<HealthCheckCtx> = checks
        .into_iter()
        .map(|c| HealthCheckCtx {
            label: c.label,
            status: c.status.as_str(),
            message: c.message,
        })
        .collect();
    HealthCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "Health",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        checks: ctx_checks,
        all_ok,
        ok_count,
        warn_count,
        error_count,
    }
}

#[derive(Serialize)]
pub(crate) struct PlaygroundCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    pub entries: Vec<SidebarEntry>,
    /// Models eligible for the playground — admin_name + a human-
    /// readable label for the dropdown.
    pub models: Vec<PlaygroundModelCtx>,
}

#[derive(Serialize)]
pub(crate) struct PlaygroundModelCtx {
    pub admin_name: &'static str,
    pub display_name: &'static str,
}

pub(crate) fn playground_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
) -> PlaygroundCtx {
    PlaygroundCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "API playground",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        models: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(|e| PlaygroundModelCtx {
                admin_name: e.admin_name,
                display_name: e.display_name,
            })
            .collect(),
    }
}

pub(crate) fn apis_index_ctx(
    identity: &Identity,
    admin: &Admin,
    csrf_token: String,
) -> ApisIndexCtx {
    let apis: Vec<ApiEntryCtx> = admin
        .entries()
        .iter()
        .filter(|e| !e.core)
        .map(|e| ApiEntryCtx {
            admin_name: e.admin_name,
            singular_name: e.singular_name,
            display_name: e.display_name,
            list_path: format!("/admin/{}", e.admin_name),
            detail_path: format!("/admin/{}/{{id}}", e.admin_name),
            field_count: e.fields.len(),
            fields: e
                .fields
                .iter()
                .map(|f| ApiFieldCtx {
                    name: f.name,
                    type_label: api_field_type_label(f),
                    nullable: f.field_type.nullable(),
                })
                .collect(),
        })
        .collect();
    ApisIndexCtx {
        base: BaseContext::new(Some(identity), csrf_token, admin),
        page_title: "API surface",
        entries: admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        apis,
        openapi_url: "/admin/apis/openapi.json",
    }
}

#[derive(Serialize, Clone)]
pub(crate) struct HistoryChangeCtx {
    /// Snake-case field identifier (`title`, `published_at`).
    /// Surfaced as a `<dt>`'s `data-field` attribute mostly so
    /// downstream CSS can hook in if needed.
    pub field: String,
    /// Humanised label rendered as the row header (`Title`,
    /// `Published at`).
    pub label: String,
    /// Stringified previous value. Empty string means "was unset"
    /// (matches the column's display representation of NULL).
    pub from: String,
    /// Stringified new value. Same empty-means-unset convention.
    pub to: String,
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
    /// Sidebar nav models — read by `_sidebar.html` as
    /// `{{ entry.admin_name }}` / `{{ entry.display_name }}`.
    /// Kept under the conventional `entries` name to match every
    /// other rendered page; the audit rows for the page body live
    /// in `history_entries` to avoid colliding on the sidebar
    /// template's reads.
    pub entries: Vec<SidebarEntry>,
    pub history_entries: Vec<HistoryEntryCtx>,
    pub flash: Option<FlashCtx>,
}

#[derive(Serialize)]
pub(crate) struct LogEntriesCtx {
    #[serde(flatten)]
    pub base: BaseContext,
    pub page_title: &'static str,
    /// Sidebar nav models — see `ObjectHistoryCtx::entries`.
    pub entries: Vec<SidebarEntry>,
    pub history_entries: Vec<HistoryEntryCtx>,
    pub flash: Option<FlashCtx>,
    /// When `Some(label)`, the page is showing audit entries
    /// filtered by `?user_id=N`. The label is the actor's email
    /// (or `#<id>` fallback) for the banner. `None` → no filter
    /// banner rendered; the table shows the unfiltered feed.
    pub user_filter_label: Option<String>,
}

pub(crate) fn map_audit_actions(actions: Vec<AdminAction>) -> Vec<HistoryEntryCtx> {
    let mut prev_date_iso: Option<String> = None;
    actions
        .into_iter()
        .map(|a| {
            let changes = extract_changes_from_metadata(a.metadata.as_ref());
            let date_iso = a.timestamp.format("%Y-%m-%d").to_string();
            let is_new_day = prev_date_iso.as_deref() != Some(date_iso.as_str());
            prev_date_iso = Some(date_iso.clone());
            HistoryEntryCtx {
                timestamp_iso: a.timestamp.to_rfc3339(),
                when_relative: relative_time(a.timestamp),
                date_iso,
                is_new_day,
                user_id: a.user_id,
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
                changes,
            }
        })
        .collect()
}

/// Pull a `changes: [{field, label, from, to}, …]` array out of an
/// audit row's `metadata` JSONB. Any structural mismatch (no key,
/// not an array, entries missing fields) collapses to an empty
/// vec — the History page renders the row without a diff and the
/// caller never has to defend against malformed metadata.
///
/// The shape mirrors `handlers::FieldChange` so the JSON written
/// by `do_update` round-trips losslessly through Postgres' JSONB
/// column.
fn extract_changes_from_metadata(metadata: Option<&serde_json::Value>) -> Vec<HistoryChangeCtx> {
    let Some(obj) = metadata.and_then(|m| m.as_object()) else {
        return Vec::new();
    };
    let Some(arr) = obj.get("changes").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|entry| {
            let o = entry.as_object()?;
            let field = o.get("field")?.as_str()?.to_string();
            let label = o
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(&field)
                .to_string();
            let from = o
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let to = o
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(HistoryChangeCtx {
                field,
                label,
                from,
                to,
            })
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
                    span: 1,
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
                    label: "Temporary password".to_string(),
                    widget: "input",
                    input_type: "password",
                    value: String::new(),
                    hint: Some(password_hint),
                    placeholder: None,
                    required: true,
                    options: None,
                    multiple: false,
                    span: 1,
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
                    // Full-width. The reference (user_new.png) pairs Role with an
                    // "Active" checkbox, but creating an inactive user would need
                    // create-path behaviour the framework doesn't expose
                    // (`create_user` always inserts `is_active = TRUE`). Active is
                    // a documented deferral — see VISUAL-CONTRACT v2.1 §3 note.
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
        },
    ]
}

/// The single name/description card for group_new / group_edit. Two fields:
/// name (text, required, 150-char max) and description (textarea). Per Visual
/// Contract v2.1 §3(c) a lone field-group card is **legend-less** — the title is
/// `None` so the template's `{% if section.title %}` guard emits no legend. (On
/// group_edit the PERMISSIONS card keeps its own legend, set in its template.)
pub(crate) fn group_form_sections(name: &str, description: &str) -> Vec<FormSection> {
    vec![FormSection {
        title: None,
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
                label: "Current password".to_string(),
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
                span: 1,
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
                label: "Confirm new password".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: None,
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 1,
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
                span: 1,
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
                label: "Confirm new password".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: None,
                placeholder: None,
                required: true,
                options: None,
                multiple: false,
                span: 1,
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

    /// `humanise_field` and its macro-side mirror must produce
    /// identical output. This battery pins the contract; if a
    /// future change drifts one without the other the validation-
    /// error routing in [`bucket_errors_by_label`] breaks because
    /// the runtime label no longer matches the macro-emitted one.
    #[test]
    fn humanise_field_standalone_acronyms() {
        // The shipped fix: `id` no longer humanises to `Id`.
        assert_eq!(humanise_field("id"), "ID");
        assert_eq!(humanise_field("ip"), "IP");
        assert_eq!(humanise_field("url"), "URL");
        assert_eq!(humanise_field("uuid"), "UUID");
        assert_eq!(humanise_field("mfa"), "MFA");
    }

    #[test]
    fn humanise_field_compound_acronyms() {
        assert_eq!(humanise_field("email_id"), "Email ID");
        assert_eq!(humanise_field("id_card"), "ID Card");
        assert_eq!(humanise_field("user_ip"), "User IP");
        assert_eq!(humanise_field("api_token"), "API Token");
        assert_eq!(humanise_field("mfa_secret_key_id"), "MFA Secret Key ID");
        assert_eq!(humanise_field("csv_export_path"), "CSV Export Path");
    }

    #[test]
    fn humanise_field_acronym_substrings_left_alone() {
        // Whole-word rule: `id` inside `video` does not become
        // `vIDeo`. Same for `ip` inside `recipe`, etc.
        assert_eq!(humanise_field("video"), "Video");
        assert_eq!(humanise_field("video_url"), "Video URL");
        assert_eq!(humanise_field("hidden_field"), "Hidden Field");
        assert_eq!(humanise_field("idle_seconds"), "Idle Seconds");
    }

    #[test]
    fn humanise_field_plain_snake_case() {
        assert_eq!(humanise_field("title"), "Title");
        assert_eq!(humanise_field("chart_number"), "Chart Number");
        assert_eq!(humanise_field("full_name"), "Full Name");
        assert_eq!(
            humanise_field("performed_by_technician"),
            "Performed By Technician",
        );
    }

    #[test]
    fn humanise_field_edge_cases() {
        assert_eq!(humanise_field(""), "");
        assert_eq!(humanise_field("a"), "A");
        assert_eq!(humanise_field("created_at"), "Created At");
        assert_eq!(humanise_field("revoked_by"), "Revoked By");
    }

    #[test]
    fn sort_direction_label_strings_read_alphabetical() {
        use super::super::modeladmin::SortDir;
        use super::super::types::FieldType::*;
        // Required and optional flavours share the same copy.
        assert_eq!(sort_direction_label(String, SortDir::Asc), "A → Z");
        assert_eq!(sort_direction_label(String, SortDir::Desc), "Z → A");
        assert_eq!(sort_direction_label(OptionalString, SortDir::Asc), "A → Z");
        assert_eq!(sort_direction_label(OptionalString, SortDir::Desc), "Z → A");
    }

    #[test]
    fn sort_direction_label_filepaths_read_alphabetical() {
        // FilePath / OptionalFilePath store strings on the wire
        // and sort lexicographically — they belong in the same
        // copy bucket as String, not the numeric fallback that
        // used to read "Photo Path (ascending)" / "(descending)"
        // and surfaced as a UX nit on the patients list page.
        use super::super::modeladmin::SortDir;
        use super::super::types::FieldType::*;
        assert_eq!(sort_direction_label(FilePath, SortDir::Asc), "A → Z");
        assert_eq!(sort_direction_label(FilePath, SortDir::Desc), "Z → A");
        assert_eq!(
            sort_direction_label(OptionalFilePath, SortDir::Asc),
            "A → Z"
        );
        assert_eq!(
            sort_direction_label(OptionalFilePath, SortDir::Desc),
            "Z → A"
        );
    }

    #[test]
    fn sort_direction_label_datetimes_read_chronological() {
        use super::super::modeladmin::SortDir;
        use super::super::types::FieldType::*;
        assert_eq!(sort_direction_label(DateTime, SortDir::Asc), "oldest first");
        assert_eq!(
            sort_direction_label(DateTime, SortDir::Desc),
            "newest first"
        );
        assert_eq!(
            sort_direction_label(OptionalDateTime, SortDir::Asc),
            "oldest first",
        );
        assert_eq!(
            sort_direction_label(OptionalDateTime, SortDir::Desc),
            "newest first",
        );
    }

    #[test]
    fn sort_direction_label_bools_read_off_on() {
        use super::super::modeladmin::SortDir;
        use super::super::types::FieldType::*;
        assert_eq!(sort_direction_label(Bool, SortDir::Asc), "off → on");
        assert_eq!(sort_direction_label(Bool, SortDir::Desc), "on → off");
    }

    #[test]
    fn sort_direction_label_numerics_read_highest_lowest() {
        // Numeric columns get the same direction-aware treatment
        // as datetime / string / bool: "lowest first" /
        // "highest first" beats generic "ascending" / "descending"
        // for a column named `count`, `score`, `priority`, or any
        // other quantity an operator scans by magnitude.
        use super::super::modeladmin::SortDir;
        use super::super::types::FieldType::*;
        assert_eq!(sort_direction_label(I32, SortDir::Asc), "lowest first");
        assert_eq!(sort_direction_label(I64, SortDir::Asc), "lowest first");
        assert_eq!(
            sort_direction_label(OptionalI64, SortDir::Desc),
            "highest first"
        );
    }

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
            // Authentication events.
            "login_succeeded",
            "login_failed",
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
            "login_succeeded",
            "login_failed",
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

    /// `ModelAdmin::fieldsets()` honoured: each fieldset becomes a
    /// titled section in declaration order, fields render in the
    /// order the project listed them inside each fieldset.
    #[test]
    fn fieldsets_drive_section_order_and_titles() {
        use super::super::modeladmin::Fieldset;
        let fields = vec![
            stub_form_field("title"),
            stub_form_field("body"),
            stub_form_field("created_at"),
            stub_form_field("slug"),
        ];
        // Note: declare "body" before "title" so the section flips
        // the in-flat order — proves we respect Fieldset.fields
        // ordering, not the original Vec ordering.
        let fieldsets: &'static [Fieldset] = &[
            Fieldset {
                title: "Content",
                fields: &["body", "title"],
            },
            Fieldset {
                title: "Metadata",
                fields: &["created_at"],
            },
        ];
        let sections = group_fields_by_fieldsets(fields, fieldsets);
        // Two declared sections + trailing "Other" for `slug` (the
        // model field nobody listed in a fieldset).
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].title, Some("Content"));
        assert_eq!(
            sections[0]
                .fields
                .iter()
                .map(|f| f.name)
                .collect::<Vec<_>>(),
            vec!["body", "title"],
        );
        assert_eq!(sections[1].title, Some("Metadata"));
        assert_eq!(
            sections[1]
                .fields
                .iter()
                .map(|f| f.name)
                .collect::<Vec<_>>(),
            vec!["created_at"],
        );
        assert_eq!(sections[2].title, Some("Other"));
        assert_eq!(
            sections[2]
                .fields
                .iter()
                .map(|f| f.name)
                .collect::<Vec<_>>(),
            vec!["slug"],
        );
    }

    /// Names in a `Fieldset` with no matching `FormField` are
    /// silently skipped — a typo doesn't panic in production. An
    /// empty section is also dropped so the form doesn't render a
    /// titled blank.
    #[test]
    fn fieldsets_with_unknown_field_names_skip_silently() {
        use super::super::modeladmin::Fieldset;
        let fields = vec![stub_form_field("title")];
        let fieldsets: &'static [Fieldset] = &[
            Fieldset {
                title: "Real",
                fields: &["title", "typo_nonexistent"],
            },
            Fieldset {
                title: "Empty",
                fields: &["also_missing"],
            },
        ];
        let sections = group_fields_by_fieldsets(fields, fieldsets);
        // "Empty" is dropped (zero matches); "Real" stays with just
        // the one real field.
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].title, Some("Real"));
        assert_eq!(sections[0].fields.len(), 1);
        assert_eq!(sections[0].fields[0].name, "title");
    }

    fn stub_form_field(name: &'static str) -> FormField {
        FormField {
            name,
            label: name.to_string(),
            widget: "input",
            input_type: "text",
            value: String::new(),
            hint: None,
            placeholder: None,
            required: false,
            options: None,
            multiple: false,
            span: 1,
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
        }
    }

    // ---- highlight_search_match -------------------------------------

    #[test]
    fn highlight_returns_none_when_no_match() {
        assert_eq!(highlight_search_match("Anna Lindqvist", "zzz"), None);
    }

    #[test]
    fn highlight_returns_none_for_empty_term() {
        // The caller has already short-circuited on empty term, but
        // make the function defensive anyway.
        assert_eq!(highlight_search_match("anything", ""), None);
    }

    #[test]
    fn highlight_wraps_ascii_case_insensitive() {
        let html = highlight_search_match("Anna Lindqvist", "anna").unwrap();
        assert_eq!(html, "<mark>Anna</mark> Lindqvist");
    }

    #[test]
    fn highlight_wraps_every_occurrence() {
        let html = highlight_search_match("abc ABC abC", "abc").unwrap();
        assert_eq!(html, "<mark>abc</mark> <mark>ABC</mark> <mark>abC</mark>");
    }

    #[test]
    fn highlight_escapes_html_around_marks() {
        // A cell containing literal HTML special chars must not
        // smuggle them into the rendered DOM. The mark wraps the
        // matched substring; everything else is escaped.
        let html = highlight_search_match("name<script>alert('x')</script>", "name").unwrap();
        assert!(html.starts_with("<mark>name</mark>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&#39;x&#39;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn highlight_escapes_inside_mark_when_term_contains_specials() {
        // Pathological but possible: an operator searches for a
        // literal `<`. The mark wraps the match, and the match
        // itself escapes the special char.
        let html = highlight_search_match("a<b<c", "<").unwrap();
        assert_eq!(html, "a<mark>&lt;</mark>b<mark>&lt;</mark>c");
    }

    #[test]
    fn highlight_preserves_non_ascii_bytes_around_match() {
        // `to_ascii_lowercase` leaves non-ASCII bytes untouched, so
        // byte offsets line up. A Swedish name with a Latin search
        // term should still highlight the ASCII portion that matched.
        let html = highlight_search_match("Anna Wåhlin", "ann").unwrap();
        assert_eq!(html, "<mark>Ann</mark>a Wåhlin");
    }

    #[test]
    fn highlight_handles_adjacent_matches() {
        let html = highlight_search_match("aaaa", "aa").unwrap();
        // Greedy non-overlapping match: positions 0..2 and 2..4.
        assert_eq!(html, "<mark>aa</mark><mark>aa</mark>");
    }

    // ---- extract_changes_from_metadata -----------------------------

    #[test]
    fn extract_changes_returns_empty_when_metadata_is_none() {
        let r = extract_changes_from_metadata(None);
        assert!(r.is_empty());
    }

    #[test]
    fn extract_changes_returns_empty_when_changes_key_missing() {
        let meta = serde_json::json!({ "actor_user_id": 42 });
        let r = extract_changes_from_metadata(Some(&meta));
        assert!(r.is_empty());
    }

    #[test]
    fn extract_changes_returns_empty_when_changes_not_array() {
        let meta = serde_json::json!({ "changes": "not-an-array" });
        let r = extract_changes_from_metadata(Some(&meta));
        assert!(r.is_empty());
    }

    #[test]
    fn extract_changes_round_trips_full_entry() {
        let meta = serde_json::json!({
            "changes": [
                { "field": "title", "label": "Title", "from": "Old", "to": "New" },
                { "field": "body",  "label": "Body",  "from": "",    "to": "Filled in" },
            ]
        });
        let r = extract_changes_from_metadata(Some(&meta));
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].field, "title");
        assert_eq!(r[0].label, "Title");
        assert_eq!(r[0].from, "Old");
        assert_eq!(r[0].to, "New");
        // Empty `from` survives — represents "was unset."
        assert_eq!(r[1].from, "");
        assert_eq!(r[1].to, "Filled in");
    }

    #[test]
    fn extract_changes_falls_back_to_field_when_label_missing() {
        // Old audit rows (or hand-edited entries) might omit `label`.
        // Fall back to the snake-case field name so the UI still
        // renders something — better than blanking the row.
        let meta = serde_json::json!({
            "changes": [
                { "field": "title", "from": "a", "to": "b" }
            ]
        });
        let r = extract_changes_from_metadata(Some(&meta));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].label, "title");
    }

    #[test]
    fn extract_changes_skips_malformed_entries() {
        // Entries missing `field` are dropped silently — the
        // metadata is producer-controlled but we still defend.
        let meta = serde_json::json!({
            "changes": [
                { "field": "title", "from": "a", "to": "b" },
                { "label": "missing-field", "from": "x", "to": "y" },
                "not-an-object",
            ]
        });
        let r = extract_changes_from_metadata(Some(&meta));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].field, "title");
    }
}

/// Drift guard for the duplicated acronym list. `rustio-admin-macros`
/// keeps its own byte-for-byte copy of [`HUMANISE_ACRONYMS`] because a
/// proc-macro crate cannot depend on this crate (see the doc comment on
/// the const). This test reads the macro crate's source and asserts the
/// two lists never drift — the one "keep these in sync by hand" hazard
/// that previously had no machine check.
#[cfg(test)]
mod acronym_lockstep_tests {
    use super::HUMANISE_ACRONYMS;

    #[test]
    fn macro_crate_mirror_is_identical() {
        let macro_src = include_str!("../../../rustio-admin-macros/src/lib.rs");
        let marker = "const HUMANISE_ACRONYMS: &[&str] = &[";
        let start = macro_src
            .find(marker)
            .expect("HUMANISE_ACRONYMS const not found in rustio-admin-macros");
        let body = &macro_src[start + marker.len()..];
        let body = &body[..body.find("];").expect("unterminated HUMANISE_ACRONYMS")];

        let macro_list: Vec<&str> = body
            .split(',')
            .filter(|tok| !tok.trim().is_empty())
            .map(|tok| tok.trim().trim_matches('"'))
            .collect();

        assert_eq!(
            macro_list,
            HUMANISE_ACRONYMS.to_vec(),
            "rustio_admin_macros::HUMANISE_ACRONYMS has drifted from \
             admin::render::HUMANISE_ACRONYMS — update both lists together"
        );
    }
}

#[cfg(test)]
mod adaptive_list_tests {
    use super::adaptive_for_list;
    use crate::admin::types::{AdminField, FieldType, ListRow};
    use crate::view_layer::{infer_view_spec_from_fields, ViewMode};
    use minijinja::{context, Environment, Value};

    fn field(
        name: &'static str,
        ft: FieldType,
        choices: Option<&'static [&'static str]>,
    ) -> AdminField {
        AdminField {
            name,
            label: name,
            field_type: ft,
            editable: true,
            relation: None,
            choices,
        }
    }

    fn fields() -> Vec<AdminField> {
        vec![
            field("id", FieldType::I64, None),
            field("full_name", FieldType::String, None),
            field("status", FieldType::String, Some(&["active", "closed"])),
            field("in_stock", FieldType::Bool, None),
            field("password_hash", FieldType::String, None),
        ]
    }

    fn row() -> ListRow {
        ListRow {
            id: 42,
            cells: vec![
                "42".into(),
                "Nadim".into(),
                "active".into(),
                "true".into(),
                "$2b$secret".into(),
            ],
            cell_links: vec![None, None, None, None, None],
        }
    }

    #[test]
    fn table_mode_keeps_legacy_table_but_offers_switcher() {
        let f = fields();
        let spec = infer_view_spec_from_fields("customer", &f);
        let (adaptive, links) = adaptive_for_list("customer", &f, &spec, &[row()], None, "q=x");

        assert!(adaptive.is_none(), "table mode must keep the legacy table");
        assert!(links.iter().any(|l| l.slug == "table" && l.active));
        // every switcher href preserves the active query and sets its own view
        assert!(links
            .iter()
            .all(|l| l.href.contains("q=x") && l.href.contains(&format!("view={}", l.slug))));
    }

    #[test]
    fn cards_mode_builds_adaptive_with_ids_and_hides_secret() {
        let f = fields();
        let spec = infer_view_spec_from_fields("customer", &f);
        let (adaptive, links) =
            adaptive_for_list("customer", &f, &spec, &[row()], Some("cards"), "");

        let view = adaptive.expect("adaptive render for cards mode");
        assert_eq!(view.mode, ViewMode::Cards);
        assert_eq!(view.rows[0].id, Some(42));
        assert!(links.iter().any(|l| l.slug == "cards" && l.active));

        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("Nadim"));
        // booleans are humanised to Yes/No, not rendered as "true"/"false"
        assert!(json.contains("Yes"), "bool not humanised: {json}");
        assert!(!json.contains("\"true\""), "raw bool leaked: {json}");
        assert!(
            !json.contains("secret"),
            "hidden password_hash leaked: {json}"
        );
    }

    #[test]
    fn adaptive_partial_renders_clickable_rows_without_hidden() {
        let f = fields();
        let spec = infer_view_spec_from_fields("customer", &f);
        let (adaptive, _) = adaptive_for_list("customer", &f, &spec, &[row()], Some("cards"), "");
        let view = adaptive.unwrap();

        let mut env = Environment::new();
        env.add_template(
            "admin/_list_adaptive.html",
            include_str!("../../assets/templates/admin/_list_adaptive.html"),
        )
        .unwrap();
        env.add_template(
            "admin/view_layer/_row.html",
            include_str!("../../assets/templates/admin/view_layer/_row.html"),
        )
        .unwrap();
        env.add_template(
            "admin/view_layer/_cell.html",
            include_str!("../../assets/templates/admin/view_layer/_cell.html"),
        )
        .unwrap();

        let tmpl = env.get_template("admin/_list_adaptive.html").unwrap();
        let html = tmpl
            .render(context! {
                admin_name => "customer",
                adaptive => Value::from_serialize(&view),
            })
            .unwrap();

        assert!(html.contains("/admin/customer/42/edit"));
        assert!(html.contains("Nadim"));
        assert!(
            !html.contains("secret"),
            "hidden value reached HTML: {html}"
        );
    }

    // Render the REAL admin/list.html through both branches (stub base + icon
    // stub) to guard (a) the legacy table is unchanged when no spec, and
    // (b) the adaptive wrapper swaps cleanly when a spec is active.
    fn render_list_html(ctx: Value) -> String {
        let mut env = Environment::new();
        env.add_function(
            "icon",
            |_name: String, _kwargs: minijinja::value::Kwargs| -> String { String::new() },
        );
        env.add_template("admin/_base.html", "{% block content %}{% endblock %}")
            .unwrap();
        for (name, src) in [
            (
                "admin/list.html",
                include_str!("../../assets/templates/admin/list.html") as &str,
            ),
            (
                "admin/_list_adaptive.html",
                include_str!("../../assets/templates/admin/_list_adaptive.html"),
            ),
            (
                "admin/view_layer/_row.html",
                include_str!("../../assets/templates/admin/view_layer/_row.html"),
            ),
            (
                "admin/view_layer/_cell.html",
                include_str!("../../assets/templates/admin/view_layer/_cell.html"),
            ),
        ] {
            env.add_template(name, src).unwrap();
        }
        env.get_template("admin/list.html")
            .unwrap()
            .render(ctx)
            .unwrap()
    }

    #[test]
    fn list_html_legacy_branch_unchanged_when_no_spec() {
        let html = render_list_html(context! {
            admin_name => "customer",
            display_name => "Customers",
            singular_name => "Customer",
            read_only => false,
            fields => vec![context! { name => "full_name", kind => "text", label => "Full Name", sort_active => "", sort_link => "#" }],
            // ListRowCtx always carries highlights/links maps (often empty).
            rows => vec![context! {
                id => 7,
                full_name => "Nadim",
                highlights => std::collections::HashMap::<String, String>::new(),
                links => std::collections::HashMap::<String, String>::new(),
            }],
            total_rows => 1,
            total_pages => 1,
        });
        // legacy table renders with bulk checkbox + edit/delete links
        assert!(html.contains("rio-dtable"));
        assert!(html.contains("value=\"7\""));
        assert!(html.contains("/admin/customer/7/edit"));
        assert!(html.contains("/admin/customer/7/delete"));
        // no mode switcher, no adaptive region
        assert!(!html.contains("rio-view-modes"));
        assert!(!html.contains("av-list"));
    }

    #[test]
    fn list_html_adaptive_branch_swaps_table_for_cards() {
        let f = fields();
        let spec = infer_view_spec_from_fields("customer", &f);
        let (adaptive, links) =
            adaptive_for_list("customer", &f, &spec, &[row()], Some("cards"), "");

        let html = render_list_html(context! {
            admin_name => "customer",
            display_name => "Customers",
            singular_name => "Customer",
            read_only => false,
            rows => vec![context! { id => 42 }], // truthy so the board enters its row branch
            total_rows => 1,
            total_pages => 1,
            adaptive => Value::from_serialize(adaptive.unwrap()),
            mode_links => Value::from_serialize(&links),
        });
        // mode switcher + adaptive region present; clickable record
        assert!(html.contains("rio-view-modes"));
        assert!(html.contains("av-list"));
        assert!(html.contains("/admin/customer/42/edit"));
        assert!(html.contains("Nadim"));
        // the legacy table and its bulk form are gone in adaptive mode
        assert!(!html.contains("rio-dtable"));
        assert!(!html.contains("rio-bulk-form"));
        assert!(!html.contains("secret"));
    }
}
