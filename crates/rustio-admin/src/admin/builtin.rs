//! Built-in admin pages for users and groups. These are wired into
//! the admin automatically — projects don't have to opt in, since
//! every project needs them.
//!
//! The pages are simpler than normal model admin pages (smaller
//! forms, custom actions like "set password"), so they don't use the
//! `AdminModel` trait — they're rendered by bespoke handlers here.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use crate::auth::{self, guards, Identity, Role};
use crate::error::{Error, Result};
use crate::http::{Request, Response};
use crate::orm::{Db, Row};
use crate::templates::Templates;

use super::audit::{self, ActionType, LogEntry};
use super::render;
use super::render::{BaseContext, FlashCtx, SidebarEntry};
use super::types::Admin;

/// Read the per-request correlation id stamped by the
/// [`crate::middleware::correlation_id`] middleware. Used by every
/// audit-record call site so rows written under one HTTP request
/// share an id (doctrine 8 — forensically useful audit logs).
///
/// Returns `None` when the middleware isn't installed; the audit row
/// then lands with a NULL `correlation_id` rather than a fabricated
/// id. Projects should add `middleware::correlation_id` before
/// `csrf_protect` to make this column always populated.
fn correlation_id_from(req: &Request) -> Option<String> {
    req.ctx()
        .get::<crate::middleware::CorrelationId>()
        .map(|c| c.as_str().to_string())
}

/// Read the client IP from the proxy headers a typical reverse-proxy
/// emits. Used as the `ip_address` column on audit rows. Returns
/// `None` when neither header is present so direct-LAN traffic still
/// audits cleanly with a NULL ip_address.
fn client_ip(req: &Request) -> Option<String> {
    if let Some(xff) = req.header("x-forwarded-for") {
        // First entry is the original client; rest are proxy hops.
        return xff.split(',').next().map(|s| s.trim().to_string());
    }
    req.header("x-real-ip").map(|s| s.to_string())
}

/// Fetch (id, role, is_active) for a target user. Used by the
/// cross-rank guard before we let an editor touch another row.
async fn load_target_user(db: &Db, user_id: i64) -> Result<(i64, Role, bool, String)> {
    let row = sqlx::query("SELECT id, role, is_active, email FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| Error::NotFound(format!("user #{user_id}")))?;
    let r = Row::from_pg(&row);
    Ok((
        r.get_i64("id")?,
        Role::parse(&r.get_string("role")?)?,
        r.get_bool("is_active")?,
        r.get_string("email")?,
    ))
}

pub(crate) struct AuthAdminCtx {
    pub admin: Arc<Admin>,
    pub db: Db,
    pub templates: Arc<Templates>,
}

// ---------- Users list ----------

#[derive(Serialize)]
struct UserRow {
    id: i64,
    email: String,
    role: String,
    is_active: bool,
    created_at: String,
}

#[derive(Serialize)]
struct UsersListCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    entries: Vec<SidebarEntry>,
    users: Vec<UserRow>,
    flash: Option<FlashCtx>,
}

pub(crate) async fn list_users(
    ctx: &AuthAdminCtx,
    identity: Identity,
    csrf: String,
) -> Result<Response> {
    let rows = sqlx::query(
        "SELECT id, email, role, is_active, created_at
           FROM rustio_users
          ORDER BY id ASC",
    )
    .fetch_all(ctx.db.pool())
    .await?;

    let users = rows
        .iter()
        .map(|r| {
            let r = Row::from_pg(r);
            Ok(UserRow {
                id: r.get_i64("id")?,
                email: r.get_string("email")?,
                role: r.get_string("role")?,
                is_active: r.get_bool("is_active")?,
                created_at: r
                    .get_datetime("created_at")?
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let view = UsersListCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: "Users",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        users,
        flash: None,
    };
    let body = ctx.templates.render("admin/users_list.html", &view)?;
    Ok(Response::html(body))
}

// ---------- User edit ----------

#[derive(Serialize)]
struct UserEditCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    entries: Vec<SidebarEntry>,
    user_id: i64,
    email: String,
    role: String,
    is_active: bool,
    all_groups: Vec<GroupRow>,
    user_groups: Vec<i64>,
    errors: Vec<String>,
    flash: Option<FlashCtx>,
    /// Set when this user is the sole active developer. Templates
    /// render a yellow banner so admins know a role change here will
    /// be rejected by `do_user_edit`.
    is_last_developer: bool,
    identity_sections: Vec<render::FormSection>,
    password_sections: Vec<render::FormSection>,
}

#[derive(Serialize)]
struct GroupRow {
    id: i64,
    name: String,
    description: String,
}

async fn load_groups(db: &Db) -> Result<Vec<GroupRow>> {
    let rows = sqlx::query("SELECT id, name, description FROM rustio_groups ORDER BY name ASC")
        .fetch_all(db.pool())
        .await?;
    rows.iter()
        .map(|r| {
            let r = Row::from_pg(r);
            Ok(GroupRow {
                id: r.get_i64("id")?,
                name: r.get_string("name")?,
                description: r.get_string("description")?,
            })
        })
        .collect()
}

pub(crate) async fn show_user_edit(
    ctx: &AuthAdminCtx,
    identity: Identity,
    user_id: i64,
    csrf: String,
) -> Result<Response> {
    let row = sqlx::query("SELECT id, email, role, is_active FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(ctx.db.pool())
        .await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("user #{user_id}")))?;
    let r = Row::from_pg(&row);

    let group_ids: Vec<i64> =
        sqlx::query_scalar::<_, i64>("SELECT group_id FROM rustio_user_groups WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(ctx.db.pool())
            .await?;

    let is_last_developer =
        auth::would_orphan_role(&ctx.db, user_id, Role::Developer, Role::User, false).await?;

    let email_str = r.get_string("email")?;
    let role_str = r.get_string("role")?;
    let is_active_val = r.get_bool("is_active")?;
    let view = UserEditCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: format!("Edit user #{user_id}"),
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        user_id,
        identity_sections: render::user_edit_identity_sections(
            &email_str,
            &role_str,
            is_active_val,
            identity.role.rank(),
        ),
        password_sections: render::user_edit_password_sections(),
        email: email_str,
        role: role_str,
        is_active: is_active_val,
        all_groups: load_groups(&ctx.db).await?,
        user_groups: group_ids,
        errors: vec![],
        flash: None,
        is_last_developer,
    };
    let body = ctx.templates.render("admin/user_edit.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_user_edit(
    ctx: &AuthAdminCtx,
    identity: Identity,
    user_id: i64,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let role = Role::parse(form.required("role")?)?;
    let is_active = form.bool_flag("is_active");

    // Collect the ticked group ids upfront — used either to apply
    // below, or to preserve the user's selection on a re-render with
    // errors.
    let mut wanted: Vec<i64> = Vec::new();
    for (k, v) in form.as_map() {
        if let Some(id_str) = k.strip_prefix("group_") {
            if v == "on" {
                if let Ok(gid) = id_str.parse::<i64>() {
                    wanted.push(gid);
                }
            }
        }
    }
    let new_password = form
        .get("new_password")
        .map(|s| s.to_string())
        .unwrap_or_default();

    // ---- Authority guards -------------------------------------------------
    // Order: pure rank checks first (cheap, no DB), then the orphan
    // check (one query). All guards return `Error::Forbidden` so the
    // HTTP layer renders a 403 with the supplied reason.
    let (_, target_role_before, _is_active_before, _) = load_target_user(&ctx.db, user_id).await?;
    guards::enforce_self_demote_safe(&identity, user_id, role, is_active)?;
    guards::enforce_cross_rank_safe(&identity, user_id, target_role_before)?;
    guards::enforce_role_ceiling(&identity, role)?;
    guards::enforce_no_orphan_role(&ctx.db, user_id, role, is_active).await?;

    // Capture state BEFORE applying changes so audit rows carry a
    // before/after diff.
    let groups_before: Vec<i64> =
        sqlx::query_scalar::<_, i64>("SELECT group_id FROM rustio_user_groups WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(ctx.db.pool())
            .await?;

    sqlx::query(
        "UPDATE rustio_users SET role = $1, is_active = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(role.as_str())
    .bind(is_active)
    .bind(user_id)
    .execute(ctx.db.pool())
    .await?;

    sqlx::query("DELETE FROM rustio_user_groups WHERE user_id = $1")
        .bind(user_id)
        .execute(ctx.db.pool())
        .await?;
    // The wholesale DELETE above bypasses `remove_user_from_group`'s
    // built-in cache invalidation. Without this explicit call, a user
    // demoted to zero groups keeps every permission for up to 60s
    // (PERM_CACHE_TTL). When the loop below adds at least one group
    // back, that path's own invalidation covers us — but the
    // all-unchecked case lands here.
    auth::invalidate_user_cache(user_id);
    for gid in &wanted {
        auth::add_user_to_group(&ctx.db, user_id, *gid).await?;
    }

    if !new_password.is_empty() {
        // TODO(R2): the generic admin-edit form's password field is a
        // doctrine-22 spirit-violation. It changes another user's
        // password without invalidating their sessions, without
        // setting `must_change_password`, and without a typed
        // `PasswordResetByOther` audit row. R2 introduces the
        // dedicated `/admin/users/:id/reset-password` recovery route
        // with the correct semantics; once it ships this code path
        // either routes through the same recovery pipeline or is
        // removed entirely. See `DESIGN_RECOVERY.md` §14.4 for the
        // full callout. R1 leaves the path untouched (set_password
        // now stamps `password_changed_at` for it automatically per
        // §14.1) so R2 owns the recovery semantics in one place.
        auth::set_password(&ctx.db, user_id, &new_password).await?;
    }

    // ---- Audit ------------------------------------------------------------
    let mut summary = format!(
        "role: {} → {}; active: {}",
        target_role_before.as_str(),
        role.as_str(),
        is_active,
    );
    let added: Vec<i64> = wanted
        .iter()
        .copied()
        .filter(|g| !groups_before.contains(g))
        .collect();
    let removed: Vec<i64> = groups_before
        .iter()
        .copied()
        .filter(|g| !wanted.contains(g))
        .collect();
    if !added.is_empty() {
        summary.push_str(&format!(
            "; +groups [{}]",
            added
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if !removed.is_empty() {
        summary.push_str(&format!(
            "; -groups [{}]",
            removed
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if !new_password.is_empty() {
        summary.push_str("; password reset");
    }
    let ip = client_ip(&req);
    let cid = correlation_id_from(&req);
    let _ = audit::record(
        &ctx.db,
        LogEntry {
            user_id: identity.user_id,
            action_type: ActionType::Update,
            model_name: "User",
            object_id: user_id,
            ip_address: ip.as_deref(),
            summary,
            correlation_id: cid.as_deref(),
            session_id: None,
            metadata: None,
            event: None,
        },
    )
    .await;

    Ok(Response::redirect("/admin/users"))
}

// `render_user_edit_with_errors` was removed — the only error it
// surfaced (last-developer orphan) now flows through the
// authority-guard layer as `Error::Forbidden`, which the framework
// renders via `forbidden.html` so we get a 403 with the same human
// message at no extra cost.

// ---------- User view (read-only profile) ----------

#[derive(Serialize)]
struct UserViewCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    entries: Vec<SidebarEntry>,
    user: UserViewTarget,
    users: Vec<UserListItem>,
    total: i64,
    activity_count: i64,
    permission_count: i64,
    session_count: i64,
    /// `"overview" | "activity" | "permissions" | "sessions"`.
    tab: &'static str,
    /// Overview: last 7 events. Activity: 50 events for current page.
    /// Empty for permissions/sessions tabs.
    recent_events: Vec<TimelineEvent>,
    activity_page: i64,
    activity_total_pages: i64,
    permissions: Vec<PermissionItem>,
    sessions: Vec<SessionItem>,
    /// Sections contributed by the project's
    /// [`Admin::user_profile_extension`] closure, if registered.
    project_fields: Vec<super::types::UserProfileSection>,
    can_edit: bool,
}

#[derive(Serialize)]
struct UserViewTarget {
    id: i64,
    email: String,
    /// Display label; `full_name` if set, else humanised email local-part.
    full_name: String,
    full_name_value: Option<String>,
    role: String,
    is_admin: bool,
    is_developer: bool,
    is_active: bool,
    is_demo: bool,
    demo_label: Option<String>,
    locale: Option<String>,
    timezone: Option<String>,
    created_at_iso: String,
    last_seen_relative: String,
    last_login_iso: String,
    groups: Vec<String>,
}

#[derive(Serialize)]
struct UserListItem {
    id: i64,
    email: String,
    full_name: String,
    is_active: bool,
    last_seen_relative: String,
}

#[derive(Serialize)]
struct TimelineEvent {
    id: i64,
    /// `"success" | "info" | "warning" | "error" | "muted"`.
    kind: &'static str,
    /// Already-escaped, ready for `{{ event.message|safe }}`.
    message: String,
    timestamp_relative: String,
    actor: String,
}

#[derive(Serialize)]
struct PermissionItem {
    name: String,
    /// `"direct"` for `rustio_user_permissions` rows, `"via <Group>"`
    /// for inheritance.
    source: String,
}

#[derive(Serialize)]
struct SessionItem {
    /// First 7 chars of the session token; the full token is never
    /// exposed in the rendered HTML.
    token_short: String,
    created_at_iso: String,
    last_seen_relative: String,
    ip: Option<String>,
    user_agent: Option<String>,
}

const ACTIVITY_PER_PAGE: i64 = 50;
const OVERVIEW_RECENT_LIMIT: i64 = 7;
const LIST_PANE_LIMIT: i64 = 50;

pub(crate) async fn show_user_view(
    ctx: &AuthAdminCtx,
    identity: Identity,
    user_id: i64,
    csrf: String,
    tab: Option<String>,
    page: i64,
) -> Result<Response> {
    let profile = auth::load_user_profile(&ctx.db, user_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("user #{user_id}")))?;

    let groups = load_user_groups(&ctx.db, user_id).await?;
    let last_seen = load_max_session_ts(&ctx.db, user_id, "last_seen").await;
    let last_login = load_max_session_ts(&ctx.db, user_id, "created_at").await;
    let activity_count = load_user_activity_count(&ctx.db, user_id).await;
    let permission_count = load_user_permission_count(&ctx.db, user_id).await;
    let session_count = load_user_session_count(&ctx.db, user_id).await;

    let tab_str: &'static str = match tab.as_deref() {
        Some("activity") => "activity",
        Some("permissions") => "permissions",
        Some("sessions") => "sessions",
        _ => "overview",
    };
    let page = page.max(1);

    let (recent_events, activity_page, activity_total_pages) = match tab_str {
        "activity" => {
            let total_pages = (activity_count.max(1) + ACTIVITY_PER_PAGE - 1) / ACTIVITY_PER_PAGE;
            let total_pages = total_pages.max(1);
            let page = page.min(total_pages);
            let offset = (page - 1) * ACTIVITY_PER_PAGE;
            let evts = load_user_audit(&ctx.db, user_id, ACTIVITY_PER_PAGE, offset).await?;
            (evts, page, total_pages)
        }
        "overview" => {
            let evts = load_user_audit(&ctx.db, user_id, OVERVIEW_RECENT_LIMIT, 0).await?;
            (evts, 1, 1)
        }
        _ => (Vec::new(), 1, 1),
    };

    let permissions = if tab_str == "permissions" {
        load_user_permissions(&ctx.db, user_id).await?
    } else {
        Vec::new()
    };
    let sessions = if tab_str == "sessions" {
        load_user_sessions(&ctx.db, user_id).await?
    } else {
        Vec::new()
    };

    let users = load_user_list(&ctx.db, LIST_PANE_LIMIT).await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rustio_users")
        .fetch_one(ctx.db.pool())
        .await
        .unwrap_or(0);

    // Call the project-registered extension closure (if any) only on
    // the Overview tab.
    let project_fields = if tab_str == "overview" {
        match ctx.admin.user_profile_ext() {
            Some(ext) => ext(ctx.db.clone(), profile.clone()).await?,
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let role_label = profile.role.label().to_string();
    let display_name = profile
        .full_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| humanize_email(&profile.email));

    let view = UserViewCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: format!("{} — Users", profile.email),
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        user: UserViewTarget {
            id: profile.id,
            email: profile.email.clone(),
            full_name: display_name,
            full_name_value: profile.full_name.clone(),
            role: role_label,
            is_admin: profile.role.includes(Role::Administrator),
            is_developer: profile.role.includes(Role::Developer),
            is_active: profile.is_active,
            is_demo: profile.is_demo,
            demo_label: profile.demo_label.clone(),
            locale: profile.locale.clone(),
            timezone: profile.timezone.clone(),
            created_at_iso: profile.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
            last_seen_relative: last_seen
                .map(render::relative_time)
                .unwrap_or_else(|| "never".into()),
            last_login_iso: last_login
                .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "never".into()),
            groups,
        },
        users,
        total,
        activity_count,
        permission_count,
        session_count,
        tab: tab_str,
        recent_events,
        activity_page,
        activity_total_pages,
        permissions,
        sessions,
        project_fields,
        can_edit: true,
    };
    let body = ctx.templates.render("admin/user_view.html", &view)?;
    Ok(Response::html(body))
}

// ---------- show_user_view helpers ----------

async fn load_user_groups(db: &Db, user_id: i64) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT g.name FROM rustio_groups g
         JOIN rustio_user_groups ug ON ug.group_id = g.id
         WHERE ug.user_id = $1
         ORDER BY g.name ASC",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| Error::Internal(format!("query user groups: {e}")))?;
    Ok(rows.into_iter().map(|(n,)| n).collect())
}

async fn load_max_session_ts(
    db: &Db,
    user_id: i64,
    col: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
    // `col` is one of "last_seen" / "created_at" — never user input.
    // String interpolation here is bound to the function's call sites
    // in this module.
    let sql = format!("SELECT MAX({col}) FROM rustio_sessions WHERE user_id = $1");
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(&sql)
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .ok()
        .flatten()
}

async fn load_user_activity_count(db: &Db, user_id: i64) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM rustio_admin_actions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .unwrap_or(0)
}

async fn load_user_session_count(db: &Db, user_id: i64) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM rustio_sessions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .unwrap_or(0)
}

async fn load_user_permission_count(db: &Db, user_id: i64) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(DISTINCT p.id)
         FROM rustio_permissions p
         LEFT JOIN rustio_user_permissions up
                ON up.permission_id = p.id AND up.user_id = $1
         LEFT JOIN rustio_group_permissions gp
                ON gp.permission_id = p.id
         LEFT JOIN rustio_user_groups ug
                ON ug.group_id = gp.group_id AND ug.user_id = $1
         WHERE up.user_id IS NOT NULL OR ug.user_id IS NOT NULL",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0)
}

async fn load_user_audit(
    db: &Db,
    user_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<TimelineEvent>> {
    let rows: Vec<(
        i64,
        String,
        String,
        i64,
        chrono::DateTime<chrono::Utc>,
        String,
    )> = sqlx::query_as(
        "SELECT id, action_type, model_name, object_id, timestamp, summary
             FROM rustio_admin_actions
             WHERE user_id = $1
             ORDER BY timestamp DESC, id DESC
             LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db.pool())
    .await
    .map_err(|e| Error::Internal(format!("query audit: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(id, action, model, obj, ts, summary)| TimelineEvent {
            id,
            kind: match action.as_str() {
                "create" => "success",
                "delete" => "error",
                "update" => "info",
                _ => "muted",
            },
            message: format!(
                "<strong>{}</strong> on {} #{}",
                html_escape(&summary),
                html_escape(&model),
                obj,
            ),
            timestamp_relative: render::relative_time(ts),
            actor: format!("user:{user_id}"),
        })
        .collect())
}

async fn load_user_permissions(db: &Db, user_id: i64) -> Result<Vec<PermissionItem>> {
    // Direct grants → source = "direct".
    let direct: Vec<(String,)> = sqlx::query_as(
        "SELECT p.name
         FROM rustio_permissions p
         JOIN rustio_user_permissions up ON up.permission_id = p.id
         WHERE up.user_id = $1
         ORDER BY p.name ASC",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| Error::Internal(format!("query direct perms: {e}")))?;

    // Inherited via groups → source = "via <Group name>".
    let inherited: Vec<(String, String)> = sqlx::query_as(
        "SELECT p.name, g.name
         FROM rustio_permissions p
         JOIN rustio_group_permissions gp ON gp.permission_id = p.id
         JOIN rustio_groups g ON g.id = gp.group_id
         JOIN rustio_user_groups ug ON ug.group_id = g.id
         WHERE ug.user_id = $1
         ORDER BY p.name ASC, g.name ASC",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| Error::Internal(format!("query inherited perms: {e}")))?;

    let mut out: Vec<PermissionItem> = Vec::with_capacity(direct.len() + inherited.len());
    for (name,) in direct {
        out.push(PermissionItem {
            name,
            source: "direct".into(),
        });
    }
    for (name, group) in inherited {
        out.push(PermissionItem {
            name,
            source: format!("via {group}"),
        });
    }
    Ok(out)
}

async fn load_user_sessions(db: &Db, user_id: i64) -> Result<Vec<SessionItem>> {
    type SessionRow = (
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<SessionRow> = sqlx::query_as(
        "SELECT token, created_at, last_seen, ip, user_agent
         FROM rustio_sessions
         WHERE user_id = $1
         ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| Error::Internal(format!("query sessions: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(token, created_at, last_seen, ip, user_agent)| {
            let token_short: String = token.chars().take(7).collect();
            SessionItem {
                token_short,
                created_at_iso: created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                last_seen_relative: render::relative_time(last_seen),
                ip,
                user_agent,
            }
        })
        .collect())
}

async fn load_user_list(db: &Db, limit: i64) -> Result<Vec<UserListItem>> {
    let rows: Vec<(i64, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT id, email, full_name, is_active
         FROM rustio_users
         ORDER BY created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(db.pool())
    .await
    .map_err(|e| Error::Internal(format!("query user list: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, email, full_name, is_active) in rows {
        let last_seen = load_max_session_ts(db, id, "last_seen").await;
        let display = full_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| humanize_email(&email));
        out.push(UserListItem {
            id,
            email,
            full_name: display,
            is_active,
            last_seen_relative: last_seen
                .map(render::relative_time)
                .unwrap_or_else(|| "—".into()),
        });
    }
    Ok(out)
}

fn humanize_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    let humanized: String = local
        .split(['.', '_', '-'])
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if humanized.is_empty() {
        email.to_string()
    } else {
        humanized
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------- User delete ----------

#[derive(Serialize)]
struct UserDeleteCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    entries: Vec<SidebarEntry>,
    user_id: i64,
    email: String,
    role: String,
    /// Memberships dropped on cascade.
    group_count: i64,
    /// Active sessions terminated on cascade.
    session_count: i64,
    /// Direct permission grants dropped on cascade.
    direct_perm_count: i64,
    /// Set when the target is the currently logged-in user.
    is_self: bool,
    /// Set when removing this user would leave zero active developers.
    is_last_developer: bool,
}

pub(crate) async fn show_user_delete(
    ctx: &AuthAdminCtx,
    identity: Identity,
    user_id: i64,
    csrf: String,
) -> Result<Response> {
    let row = sqlx::query("SELECT id, email, role FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(ctx.db.pool())
        .await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("user #{user_id}")))?;
    let r = Row::from_pg(&row);

    let group_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rustio_user_groups WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(ctx.db.pool())
            .await?;
    let session_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rustio_sessions WHERE user_id = $1 AND expires_at > NOW()",
    )
    .bind(user_id)
    .fetch_one(ctx.db.pool())
    .await?;
    let direct_perm_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rustio_user_permissions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(ctx.db.pool())
            .await?;

    let is_self = identity.user_id == user_id;
    let is_last_developer =
        auth::would_orphan_role(&ctx.db, user_id, Role::Developer, Role::User, false).await?;

    let email = r.get_string("email")?;
    let view = UserDeleteCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: format!("Delete user: {email}"),
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        user_id,
        email,
        role: r.get_string("role")?,
        group_count,
        session_count,
        direct_perm_count,
        is_self,
        is_last_developer,
    };
    let body = ctx
        .templates
        .render("admin/user_confirm_delete.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_user_delete(
    ctx: &AuthAdminCtx,
    identity: Identity,
    user_id: i64,
    req: Request,
) -> Result<Response> {
    // Self-delete guard.
    if identity.user_id == user_id {
        return Err(Error::BadRequest(
            "You cannot delete your own account while signed in.".into(),
        ));
    }

    // Authority guards. Cross-rank prevents a lower-rank user from
    // deleting someone above them; orphan check covers every protected
    // role (Administrator + Developer), not just Developer.
    let (_, target_role, _, target_email) = load_target_user(&ctx.db, user_id).await?;
    guards::enforce_cross_rank_safe(&identity, user_id, target_role)?;
    // For delete, model the new state as `inactive` with a sentinel
    // role; the orphan helper only cares whether the target stays in
    // the protected pool, and `is_active=false` says "no".
    guards::enforce_no_orphan_role(&ctx.db, user_id, Role::User, false).await?;

    sqlx::query("DELETE FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .execute(ctx.db.pool())
        .await?;

    // Cascade through user_groups, user_permissions, and sessions
    // happens at the FK level. Drop the perm-cache entry.
    auth::invalidate_user_cache(user_id);

    let ip = client_ip(&req);
    let cid = correlation_id_from(&req);
    let _ = audit::record(
        &ctx.db,
        LogEntry {
            user_id: identity.user_id,
            action_type: ActionType::Delete,
            model_name: "User",
            object_id: user_id,
            ip_address: ip.as_deref(),
            summary: format!("deleted user {} ({})", target_email, target_role.as_str()),
            correlation_id: cid.as_deref(),
            session_id: None,
            metadata: None,
            event: None,
        },
    )
    .await;

    Ok(Response::redirect("/admin/users"))
}

// ---------- Groups ----------

#[derive(Serialize)]
struct GroupsListCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    entries: Vec<SidebarEntry>,
    groups: Vec<GroupRow>,
    flash: Option<FlashCtx>,
}

pub(crate) async fn list_groups(
    ctx: &AuthAdminCtx,
    identity: Identity,
    csrf: String,
) -> Result<Response> {
    let view = GroupsListCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: "Groups",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        groups: load_groups(&ctx.db).await?,
        flash: None,
    };
    let body = ctx.templates.render("admin/groups_list.html", &view)?;
    Ok(Response::html(body))
}

#[derive(Serialize)]
struct GroupEditCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    entries: Vec<SidebarEntry>,
    group_id: i64,
    name: String,
    description: String,
    /// Permissions organised as a model × action matrix. Rendered as a
    /// table with one row per model and one column per CRUD action so
    /// the operator can scan the whole grant set at a glance instead
    /// of reading 60+ codenames in alphabetical order.
    matrix: PermissionMatrix,
    errors: Vec<String>,
    flash: Option<FlashCtx>,
    sections: Vec<render::FormSection>,
}

#[derive(Serialize)]
struct PermRow {
    id: i64,
    name: String,
}

/// One model's row in the permissions matrix. `cells` is parallel to
/// `PermissionMatrix::columns` — index `i` corresponds to the i-th
/// canonical action (view / add / change / delete). `None` means the
/// model didn't register that action.
#[derive(Serialize)]
struct PermissionMatrixRow {
    /// Humanised label, e.g. `"Cashiers"` from a `"cashiers.add_cashier"`
    /// codename family.
    label: String,
    cells: Vec<Option<PermCell>>,
}

#[derive(Serialize, Clone)]
struct PermCell {
    id: i64,
    /// Full codename, surfaced as `title=` / `aria-label` on the
    /// checkbox so a power user can still confirm the exact
    /// permission.
    name: String,
    checked: bool,
}

#[derive(Serialize)]
struct PermissionMatrix {
    /// Action column headers, in display order: View / Add / Change /
    /// Delete. Action names follow Django convention because that's
    /// what `Admin::seed_permissions` emits.
    columns: &'static [&'static str],
    rows: Vec<PermissionMatrixRow>,
    /// Permissions whose codename doesn't fit the
    /// `<table>.<action>_<singular>` shape. Rendered below the matrix
    /// as a flat checkbox list so nothing is silently dropped.
    extras: Vec<PermCell>,
}

/// Canonical CRUD actions, in scan order.
const PERMISSION_ACTIONS: &[&str] = &["view", "add", "change", "delete"];

/// Group `all` by table prefix and bucket each entry into the
/// canonical view / add / change / delete column. Anything that
/// doesn't match `<table>.<action>_<rest>` ends up in `extras` so the
/// page never silently drops a permission.
fn build_permission_matrix(all: &[PermRow], current: &[i64]) -> PermissionMatrix {
    use std::collections::BTreeMap;

    let current_set: std::collections::HashSet<i64> = current.iter().copied().collect();
    let mut groups: BTreeMap<String, Vec<Option<PermCell>>> = BTreeMap::new();
    let mut extras: Vec<PermCell> = Vec::new();

    for p in all {
        let parsed = p.name.split_once('.').and_then(|(table, rest)| {
            rest.split_once('_').and_then(|(action, _tail)| {
                PERMISSION_ACTIONS
                    .iter()
                    .position(|&a| a == action)
                    .map(|idx| (table.to_string(), idx))
            })
        });
        let cell = PermCell {
            id: p.id,
            name: p.name.clone(),
            checked: current_set.contains(&p.id),
        };
        match parsed {
            Some((table, idx)) => {
                let row = groups
                    .entry(table)
                    .or_insert_with(|| vec![None; PERMISSION_ACTIONS.len()]);
                row[idx] = Some(cell);
            }
            None => extras.push(cell),
        }
    }

    let rows: Vec<PermissionMatrixRow> = groups
        .into_iter()
        .map(|(table, cells)| PermissionMatrixRow {
            label: humanise_table(&table),
            cells,
        })
        .collect();

    PermissionMatrix {
        columns: PERMISSION_ACTIONS,
        rows,
        extras,
    }
}

/// Title-case a snake-case table name into a model heading. Mirrors
/// the macro's runtime humaniser so the matrix label matches what
/// the rest of the admin uses for that model.
fn humanise_table(s: &str) -> String {
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

pub(crate) async fn show_group_edit(
    ctx: &AuthAdminCtx,
    identity: Identity,
    group_id: i64,
    csrf: String,
) -> Result<Response> {
    let row = sqlx::query("SELECT id, name, description FROM rustio_groups WHERE id = $1")
        .bind(group_id)
        .fetch_optional(ctx.db.pool())
        .await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("group #{group_id}")))?;
    let r = Row::from_pg(&row);

    let all: Vec<PermRow> = {
        let rows = sqlx::query("SELECT id, name FROM rustio_permissions ORDER BY name ASC")
            .fetch_all(ctx.db.pool())
            .await?;
        rows.iter()
            .map(|r| {
                let r = Row::from_pg(r);
                Ok(PermRow {
                    id: r.get_i64("id")?,
                    name: r.get_string("name")?,
                })
            })
            .collect::<Result<Vec<_>>>()?
    };

    let current: Vec<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT permission_id FROM rustio_group_permissions WHERE group_id = $1",
    )
    .bind(group_id)
    .fetch_all(ctx.db.pool())
    .await?;

    let name_str = r.get_string("name")?;
    let description_str = r.get_string("description")?;
    let matrix = build_permission_matrix(&all, &current);
    let view = GroupEditCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: format!("Edit group #{group_id}"),
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        group_id,
        sections: render::group_form_sections(&name_str, &description_str),
        name: name_str,
        description: description_str,
        matrix,
        errors: vec![],
        flash: None,
    };
    let body = ctx.templates.render("admin/group_edit.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_group_edit(
    ctx: &AuthAdminCtx,
    identity: Identity,
    group_id: i64,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let name = form.required("name")?.to_string();
    let description = form.get("description").unwrap_or("").to_string();

    // Capture before-state for the audit diff.
    let perms_before: Vec<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT permission_id FROM rustio_group_permissions WHERE group_id = $1",
    )
    .bind(group_id)
    .fetch_all(ctx.db.pool())
    .await?;

    sqlx::query("UPDATE rustio_groups SET name = $1, description = $2 WHERE id = $3")
        .bind(&name)
        .bind(&description)
        .bind(group_id)
        .execute(ctx.db.pool())
        .await?;

    // Rewrite permission assignment.
    sqlx::query("DELETE FROM rustio_group_permissions WHERE group_id = $1")
        .bind(group_id)
        .execute(ctx.db.pool())
        .await?;

    let mut perms_after: Vec<i64> = Vec::new();
    for (k, v) in form.as_map() {
        if let Some(id_str) = k.strip_prefix("perm_") {
            if v == "on" {
                if let Ok(pid) = id_str.parse::<i64>() {
                    sqlx::query(
                        "INSERT INTO rustio_group_permissions (group_id, permission_id)
                         VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    )
                    .bind(group_id)
                    .bind(pid)
                    .execute(ctx.db.pool())
                    .await?;
                    perms_after.push(pid);
                }
            }
        }
    }

    let added: Vec<i64> = perms_after
        .iter()
        .copied()
        .filter(|p| !perms_before.contains(p))
        .collect();
    let removed: Vec<i64> = perms_before
        .iter()
        .copied()
        .filter(|p| !perms_after.contains(p))
        .collect();
    let mut summary = format!("name: {name}");
    if !added.is_empty() {
        summary.push_str(&format!(
            "; +perms [{}]",
            added
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if !removed.is_empty() {
        summary.push_str(&format!(
            "; -perms [{}]",
            removed
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    let ip = client_ip(&req);
    let cid = correlation_id_from(&req);
    let _ = audit::record(
        &ctx.db,
        LogEntry {
            user_id: identity.user_id,
            action_type: ActionType::Update,
            model_name: "Group",
            object_id: group_id,
            ip_address: ip.as_deref(),
            summary,
            correlation_id: cid.as_deref(),
            session_id: None,
            metadata: None,
            event: None,
        },
    )
    .await;

    Ok(Response::redirect("/admin/groups"))
}

// ---------- Group delete ----------

#[derive(Serialize)]
struct GroupDeleteCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    entries: Vec<SidebarEntry>,
    group_id: i64,
    name: String,
    description: String,
    /// Cascade through `rustio_user_groups`.
    user_count: i64,
    /// Cascade through `rustio_group_permissions`.
    perm_count: i64,
}

pub(crate) async fn show_group_delete(
    ctx: &AuthAdminCtx,
    identity: Identity,
    group_id: i64,
    csrf: String,
) -> Result<Response> {
    let row = sqlx::query("SELECT id, name, description FROM rustio_groups WHERE id = $1")
        .bind(group_id)
        .fetch_optional(ctx.db.pool())
        .await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("group #{group_id}")))?;
    let r = Row::from_pg(&row);

    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rustio_user_groups WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(ctx.db.pool())
            .await?;
    let perm_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rustio_group_permissions WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(ctx.db.pool())
            .await?;

    let name = r.get_string("name")?;
    let view = GroupDeleteCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: format!("Delete group: {name}"),
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        group_id,
        name,
        description: r.get_string("description")?,
        user_count,
        perm_count,
    };
    let body = ctx
        .templates
        .render("admin/group_confirm_delete.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_group_delete(
    ctx: &AuthAdminCtx,
    identity: Identity,
    group_id: i64,
    req: Request,
) -> Result<Response> {
    // Capture every user that's losing this group BEFORE the cascade
    // wipes the M2M table.
    let user_ids: Vec<i64> =
        sqlx::query_scalar("SELECT user_id FROM rustio_user_groups WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(ctx.db.pool())
            .await?;
    let group_name: Option<String> =
        sqlx::query_scalar("SELECT name FROM rustio_groups WHERE id = $1")
            .bind(group_id)
            .fetch_optional(ctx.db.pool())
            .await?;

    sqlx::query("DELETE FROM rustio_groups WHERE id = $1")
        .bind(group_id)
        .execute(ctx.db.pool())
        .await?;

    // Cache invalidation has to be explicit — the cascade ran in PG,
    // not via `remove_user_from_group`.
    for uid in &user_ids {
        crate::auth::invalidate_user_cache(*uid);
    }

    let ip = client_ip(&req);
    let cid = correlation_id_from(&req);
    let _ = audit::record(
        &ctx.db,
        LogEntry {
            user_id: identity.user_id,
            action_type: ActionType::Delete,
            model_name: "Group",
            object_id: group_id,
            ip_address: ip.as_deref(),
            summary: format!(
                "deleted group {} ({} users detached)",
                group_name.unwrap_or_else(|| format!("#{group_id}")),
                user_ids.len(),
            ),
            correlation_id: cid.as_deref(),
            session_id: None,
            metadata: None,
            event: None,
        },
    )
    .await;

    Ok(Response::redirect("/admin/groups"))
}

// ---------- New user ----------

#[derive(Serialize)]
struct UserNewCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    entries: Vec<SidebarEntry>,
    email: String,
    role: String,
    errors: Vec<String>,
    sections: Vec<render::FormSection>,
}

pub(crate) async fn show_new_user(
    ctx: &AuthAdminCtx,
    identity: Identity,
    csrf: String,
) -> Result<Response> {
    let email = String::new();
    let role: String = "staff".into();
    let editor_rank = identity.role.rank();
    let view = UserNewCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: "Add user",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        sections: render::user_new_form_sections(
            &email,
            &role,
            editor_rank,
            ctx.admin.active_password_policy().min_length(),
        ),
        email,
        role,
        errors: Vec::new(),
    };
    let body = ctx.templates.render("admin/user_new.html", &view)?;
    Ok(Response::html(body))
}

/// Cheap email-shape check — `<x>@<y>.<z>` with non-empty parts.
fn looks_like_email(s: &str) -> bool {
    let s = s.trim();
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    let Some((host, tld)) = domain.rsplit_once('.') else {
        return false;
    };
    !host.is_empty() && !tld.is_empty()
}

pub(crate) async fn do_new_user(
    ctx: &AuthAdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let email = form.get("email").unwrap_or("").trim().to_string();
    let password = form.get("password").unwrap_or("");
    let role_str = form.get("role").unwrap_or("staff").to_string();

    // Push every error twice: once into the global Vec (banner) and
    // once into the field-keyed map.
    let mut errors: Vec<String> = Vec::new();
    let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

    let role_parsed = Role::parse(&role_str).ok();
    if role_parsed.is_none() {
        let msg = format!("Unknown role: \"{role_str}\".");
        errors.push(msg.clone());
        field_errors.entry("role".into()).or_default().push(msg);
    }

    if email.is_empty() {
        let msg = "Email is required.";
        errors.push(msg.into());
        field_errors
            .entry("email".into())
            .or_default()
            .push(msg.into());
    } else if !looks_like_email(&email) {
        let msg = "Enter a valid email address.";
        errors.push(msg.into());
        field_errors
            .entry("email".into())
            .or_default()
            .push(msg.into());
    } else {
        // Pre-check uniqueness for a clean message.
        let existing = auth::find_user_by_email(&ctx.db, &email).await?;
        if existing.is_some() {
            let msg = format!("A user with email \"{email}\" already exists.");
            errors.push(msg.clone());
            field_errors.entry("email".into()).or_default().push(msg);
        }
    }

    // Validate against the live `Admin::active_password_policy()` so
    // form floor + form hint + framework's `set_password` all agree on
    // a single source of truth (R2 commit #3 — see
    // `DESIGN_R2_ORGANISATIONAL.md` §11). Pre-R2 this used a local
    // 8-char constant which could disagree with a project's
    // `Admin::password_policy(...)` override and produce confusing
    // "the form said 16 but it accepted 9" UX.
    if let Err(policy_err) = ctx.admin.active_password_policy().validate(password) {
        let msg = policy_err.to_string();
        errors.push(msg.clone());
        field_errors.entry("password".into()).or_default().push(msg);
    }

    // Role-ceiling guard: a user cannot create accounts with rank
    // higher than their own. Runs only when the role string parsed
    // cleanly; otherwise the validation block above already flagged
    // it as an unknown role.
    if errors.is_empty() {
        if let Some(role) = role_parsed {
            if let Err(e) = guards::enforce_role_ceiling(&identity, role) {
                let msg = match e {
                    Error::Forbidden(m) => m,
                    other => other.to_string(),
                };
                errors.push(msg.clone());
                field_errors.entry("role".into()).or_default().push(msg);
            }
        }
    }

    if errors.is_empty() {
        let role = role_parsed.expect("role parsed when errors empty");
        let new_id = auth::create_user(&ctx.db, &email, password, role).await?;
        let ip = client_ip(&req);
        let cid = correlation_id_from(&req);
        let _ = audit::record(
            &ctx.db,
            LogEntry {
                user_id: identity.user_id,
                action_type: ActionType::Create,
                model_name: "User",
                object_id: new_id,
                ip_address: ip.as_deref(),
                summary: format!("created user {} as {}", email, role.as_str()),
                correlation_id: cid.as_deref(),
                session_id: None,
                metadata: None,
                event: None,
            },
        )
        .await;
        return Ok(Response::redirect(format!("/admin/users/{new_id}/edit")));
    }

    let csrf = req
        .ctx()
        .get::<crate::middleware::CsrfGuard>()
        .map(|g| g.token.clone())
        .unwrap_or_default();
    let mut sections = render::user_new_form_sections(
        &email,
        &role_str,
        identity.role.rank(),
        ctx.admin.active_password_policy().min_length(),
    );
    render::apply_field_errors(&mut sections, &field_errors);
    let view = UserNewCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: "Add user",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        sections,
        email,
        role: role_str,
        errors,
    };
    let body = ctx.templates.render("admin/user_new.html", &view)?;
    Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST))
}

// ---------- New group ----------

#[derive(Serialize)]
struct GroupNewCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    entries: Vec<SidebarEntry>,
    name: String,
    description: String,
    errors: Vec<String>,
    sections: Vec<render::FormSection>,
}

pub(crate) async fn show_new_group(
    ctx: &AuthAdminCtx,
    identity: Identity,
    csrf: String,
) -> Result<Response> {
    let name = String::new();
    let description = String::new();
    let view = GroupNewCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: "Add group",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        sections: render::group_form_sections(&name, &description),
        name,
        description,
        errors: Vec::new(),
    };
    let body = ctx.templates.render("admin/group_new.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_new_group(
    ctx: &AuthAdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let name = form.get("name").unwrap_or("").trim().to_string();
    let description = form.get("description").unwrap_or("").to_string();

    let mut errors: Vec<String> = Vec::new();
    let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();
    if name.is_empty() {
        let msg = "Name is required.";
        errors.push(msg.into());
        field_errors
            .entry("name".into())
            .or_default()
            .push(msg.into());
    } else if name.len() > 150 {
        let msg = "Name must be 150 characters or fewer.";
        errors.push(msg.into());
        field_errors
            .entry("name".into())
            .or_default()
            .push(msg.into());
    }

    if errors.is_empty() {
        // Let the unique-constraint violation bubble up.
        let result = sqlx::query(
            "INSERT INTO rustio_groups (name, description) VALUES ($1, $2) RETURNING id",
        )
        .bind(&name)
        .bind(&description)
        .fetch_one(ctx.db.pool())
        .await;

        match result {
            Ok(row) => {
                let r = Row::from_pg(&row);
                let new_id: i64 = r.get_i64("id")?;
                let ip = client_ip(&req);
                let cid = correlation_id_from(&req);
                let _ = audit::record(
                    &ctx.db,
                    LogEntry {
                        user_id: identity.user_id,
                        action_type: ActionType::Create,
                        model_name: "Group",
                        object_id: new_id,
                        ip_address: ip.as_deref(),
                        summary: format!("created group {name}"),
                        correlation_id: cid.as_deref(),
                        session_id: None,
                        metadata: None,
                        event: None,
                    },
                )
                .await;
                return Ok(Response::redirect(format!("/admin/groups/{new_id}/edit")));
            }
            Err(sqlx::Error::Database(db_err)) if db_err.constraint().is_some() => {
                let msg = format!("A group named \"{name}\" already exists.");
                errors.push(msg.clone());
                field_errors.entry("name".into()).or_default().push(msg);
            }
            Err(e) => return Err(e.into()),
        }
    }

    let csrf = req
        .ctx()
        .get::<crate::middleware::CsrfGuard>()
        .map(|g| g.token.clone())
        .unwrap_or_default();
    let mut sections = render::group_form_sections(&name, &description);
    render::apply_field_errors(&mut sections, &field_errors);
    let view = GroupNewCtx {
        base: BaseContext::new(Some(&identity), csrf, &ctx.admin),
        page_title: "Add group",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(SidebarEntry::from)
            .collect(),
        sections,
        name,
        description,
        errors,
    };
    let body = ctx.templates.render("admin/group_new.html", &view)?;
    Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_email_accepts_simple() {
        assert!(looks_like_email("alice@example.com"));
        assert!(looks_like_email("a.b+c@example.io"));
    }

    #[test]
    fn looks_like_email_rejects_malformed() {
        assert!(!looks_like_email(""));
        assert!(!looks_like_email("alice"));
        assert!(!looks_like_email("alice@"));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("alice@example"));
        assert!(!looks_like_email("alice@.com"));
    }

    #[test]
    fn humanize_email_capitalises_local_part() {
        assert_eq!(humanize_email("alice.smith@example.com"), "Alice Smith");
        assert_eq!(humanize_email("bob_jones@x.com"), "Bob Jones");
        assert_eq!(humanize_email("anna-may@x.com"), "Anna May");
    }

    #[test]
    fn humanize_email_falls_back_to_full_string() {
        assert_eq!(humanize_email("alice"), "Alice");
        assert_eq!(humanize_email(""), "");
    }

    #[test]
    fn html_escape_handles_each_entity() {
        assert_eq!(html_escape("<b>&\"</b>"), "&lt;b&gt;&amp;&quot;&lt;/b&gt;");
        assert_eq!(html_escape("plain"), "plain");
    }
}
