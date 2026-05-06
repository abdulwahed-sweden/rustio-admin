//! HTTP handlers for the admin. All of them follow the same pattern:
//! check identity → load what you need from the DB → build a typed
//! context → hand it to `Templates::render`.
//!
//! Slimmed for Tier 1 P6: the legacy file's password-change handlers,
//! the developer-stub coming-soon pages (schema-browser, execution
//! logs, sql-console), the per-object history view, the global log
//! entries page, the FK remote-search endpoint, and the `search_hook`
//! call sites have been removed. Bespoke user/group page handlers
//! re-land with `admin/builtin.rs` in P6.b.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::auth::{self, Identity};
use crate::error::{Error, Result};
use crate::http::{Request, Response};
use crate::orm::Db;
use crate::templates::Templates;

use super::audit;
use super::render;
use super::render::BaseContext;
use super::types::Admin;

/// Lazy idempotent initializer for the `rustio_admin_actions` table.
/// `CREATE TABLE IF NOT EXISTS` is not race-safe under concurrent
/// DDL — the OnceCell collapses parallel first-requests into a single
/// DDL execution. Failures are logged and swallowed so the dashboard's
/// Recent Actions sidebar continues to silent-degrade rather than 500.
static AUDIT_TABLE_READY: OnceCell<()> = OnceCell::const_new();

async fn ensure_audit_ready(db: &Db) {
    AUDIT_TABLE_READY
        .get_or_init(|| async {
            if let Err(e) = audit::ensure_table(db).await {
                log::warn!("audit::ensure_table failed: {e}");
            }
        })
        .await;
}

/// Look up an admin entry by `admin_name`, treating core entries as
/// not-found. Core entries (the synthetic User) have bespoke admin
/// pages; routing the generic CRUD URLs at them would 500.
fn find_project_entry<'a>(
    admin: &'a Admin,
    admin_name: &str,
) -> Result<&'a super::types::AdminEntry> {
    admin
        .find(admin_name)
        .filter(|e| !e.core)
        .ok_or_else(|| Error::NotFound(format!("no admin model: {admin_name}")))
}

pub(crate) struct AdminCtx {
    pub admin: Arc<Admin>,
    pub db: Db,
    pub templates: Arc<Templates>,
}

impl AdminCtx {
    pub fn new(admin: Arc<Admin>, db: Db, templates: Arc<Templates>) -> Self {
        Self {
            admin,
            db,
            templates,
        }
    }
}

// ---- Login / logout -------------------------------------------------------

pub(super) fn csrf_token(req: &Request) -> String {
    req.ctx()
        .get::<crate::middleware::CsrfGuard>()
        .map(|g| g.token.clone())
        .unwrap_or_default()
}

pub(crate) async fn show_login(ctx: &AdminCtx, req: Request) -> Result<Response> {
    // When the user just signed out, `do_logout` redirects here with
    // `?logout=1`. Surface a green confirmation banner so the click
    // feels acknowledged.
    let flash = if req.query().get("logout").is_some() {
        Some(render::FlashCtx {
            kind: "success",
            message: "You've been signed out.".to_string(),
        })
    } else {
        None
    };
    let body = ctx.templates.render(
        "admin/login.html",
        &render::LoginCtx {
            base: BaseContext::new(None, csrf_token(&req), &ctx.admin),
            error: None,
            sections: render::login_form_sections(),
            flash,
        },
    )?;
    Ok(Response::html(body))
}

pub(crate) async fn do_login(ctx: &AdminCtx, req: Request) -> Result<Response> {
    let form = req.form()?;
    let email = form.required("email")?;
    let password = form.required("password")?;

    match auth::login(&ctx.db, email, password).await {
        Ok(token) => {
            let cookie = format!(
                "{}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=1209600",
                auth::SESSION_COOKIE
            );
            Ok(Response::redirect("/admin").with_header("set-cookie", cookie))
        }
        Err(_) => {
            let body = ctx.templates.render(
                "admin/login.html",
                &render::LoginCtx {
                    base: BaseContext::new(None, csrf_token(&req), &ctx.admin),
                    error: Some("Invalid email or password.".into()),
                    sections: render::login_form_sections(),
                    flash: None,
                },
            )?;
            Ok(Response::html(body).with_status(hyper::StatusCode::UNAUTHORIZED))
        }
    }
}

pub(crate) async fn do_logout(ctx: &AdminCtx, req: Request) -> Result<Response> {
    if let Some(cookie) = req.header("cookie") {
        if let Some(token) = auth::session_token_from_cookie(cookie) {
            auth::delete_session(&ctx.db, &token).await?;
        }
    }
    let clear = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        auth::SESSION_COOKIE
    );
    Ok(Response::redirect("/admin/login?logout=1").with_header("set-cookie", clear))
}

// ---- Dashboard -----------------------------------------------------------

pub(crate) async fn dashboard(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    ensure_audit_ready(&ctx.db).await;
    let recent_actions = audit::recent(&ctx.db, 10, None, None)
        .await
        .unwrap_or_default();
    let dash = render::dashboard_ctx(&identity, &ctx.admin, recent_actions, csrf_token(req));
    let body = ctx.templates.render("admin/index.html", &dash)?;
    Ok(Response::html(body))
}

// ---- List page -----------------------------------------------------------

pub(crate) async fn list_model(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    req: &Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let qs = req.query();

    // ---- Sort: ?sort=col&dir=desc, validated against entry.fields.
    let active_sort = parse_active_sort(entry, qs.get("sort"), qs.get("dir"));
    let ordering = match &active_sort {
        Some((col, dir)) => vec![(col.clone(), *dir)],
        None => entry
            .ordering
            .iter()
            .map(|s| super::modeladmin::parse_order_spec(s))
            .collect(),
    };

    // ---- Filters: build the chip group list (and the active
    // selections) up front so the same struct drives both the
    // SQL WHERE clause and the rendered sidebar.
    let inferred = super::filters::infer_filters(entry.fields);
    let mut filter_groups: Vec<render::FilterGroupCtx> = Vec::new();
    let mut sql_filters: Vec<(String, String)> = Vec::new();
    for f in inferred {
        let current = qs
            .get(&f.field)
            .map(str::to_string)
            .filter(|s| !s.is_empty());
        if let Some(ref val) = current {
            sql_filters.push((f.field.clone(), val.clone()));
        }
        let options = match f.kind {
            super::filters::FilterKind::BoolYesNo => vec![
                render::FilterOptionCtx {
                    value: "true".into(),
                    label: "Yes".into(),
                    selected: current.as_deref() == Some("true"),
                },
                render::FilterOptionCtx {
                    value: "false".into(),
                    label: "No".into(),
                    selected: current.as_deref() == Some("false"),
                },
            ],
            // Other filter kinds need richer widgets — later phases.
            _ => Vec::new(),
        };
        if !options.is_empty() {
            filter_groups.push(render::FilterGroupCtx {
                field: f.field,
                label: f.label,
                options,
                current,
            });
        }
    }

    // ---- Search: ?q=term, scoped to the model's `search_fields()`.
    let search = qs.get("q").unwrap_or_default().to_string();
    let search_opt: Option<(String, Vec<String>)> = if search.is_empty() {
        None
    } else if entry.search_fields.is_empty() {
        // No search_fields registered → the search box is decorative;
        // ILIKE-ing every column would fight indexes. Drop the term.
        None
    } else {
        Some((
            search.clone(),
            entry.search_fields.iter().map(|s| s.to_string()).collect(),
        ))
    };

    // ---- Pagination: per_page from ?per_page= (allow-listed),
    // falling back to entry.list_per_page. page from ?page=, 1-indexed.
    let per_page: i64 = qs
        .get("per_page")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|n| matches!(*n, 10 | 25 | 50 | 100))
        .unwrap_or(entry.list_per_page as i64)
        .max(1);
    let page_raw: i64 = qs
        .get("page")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);

    // First fetch with the requested offset; if it lands past the
    // last page (stale URL after rows shrink), clamp and refetch
    // once. Two queries vs the legacy "fetch-all-then-slice"
    // approach is still net-fewer-rows over the wire.
    let initial_offset = (page_raw - 1) * per_page;
    let mut page_result = entry
        .ops
        .list(
            &ctx.db,
            super::types::ListOpts {
                ordering: ordering.clone(),
                filters: sql_filters.clone(),
                search: search_opt.clone(),
                limit: Some(per_page),
                offset: Some(initial_offset),
            },
        )
        .await?;

    let total_rows = page_result.total;
    let total_pages = ((total_rows.max(1) + per_page - 1) / per_page).max(1);
    let page = page_raw.min(total_pages);
    if page != page_raw && total_rows > 0 {
        let clamped_offset = (page - 1) * per_page;
        page_result = entry
            .ops
            .list(
                &ctx.db,
                super::types::ListOpts {
                    ordering: ordering.clone(),
                    filters: sql_filters,
                    search: search_opt,
                    limit: Some(per_page),
                    offset: Some(clamped_offset),
                },
            )
            .await?;
    }

    let list = render::list_ctx(
        &identity,
        &ctx.admin,
        entry,
        page_result.rows,
        search,
        filter_groups,
        page as usize,
        per_page as usize,
        total_rows as usize,
        active_sort.as_ref().map(|(c, d)| (c.clone(), *d)),
        csrf_token(req),
    );
    let body = ctx.templates.render("admin/list.html", &list)?;
    Ok(Response::html(body))
}

/// Parse `?sort=col&dir=desc` against the entry's static field set.
/// Drops any column the model doesn't declare so a hand-crafted URL
/// can't leak SQL through the ORDER BY clause. `dir` is permissive
/// (anything starting with `d` → Desc, otherwise Asc).
fn parse_active_sort(
    entry: &super::types::AdminEntry,
    sort: Option<&str>,
    dir: Option<&str>,
) -> Option<(String, super::modeladmin::SortDir)> {
    let raw = sort?.trim();
    if raw.is_empty() {
        return None;
    }
    if !entry.fields.iter().any(|f| f.name == raw) {
        return None;
    }
    let direction = match dir.map(str::to_ascii_lowercase).as_deref() {
        Some(s) if s.starts_with('d') => super::modeladmin::SortDir::Desc,
        _ => super::modeladmin::SortDir::Asc,
    };
    Some((raw.to_string(), direction))
}

// ---- New / Create --------------------------------------------------------

pub(crate) async fn show_new_form(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    req: &Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let relation_options = render::resolve_relation_options(&ctx.admin, entry, &ctx.db).await?;
    let form = render::form_ctx(
        &identity,
        &ctx.admin,
        entry,
        "new",
        None,
        None,
        vec![],
        csrf_token(req),
        relation_options,
        HashMap::new(),
        None,
    );
    let body = ctx.templates.render("admin/form.html", &form)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_create(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    req: Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let form = req.form()?;
    let intent = submit_intent(&form);
    match entry.ops.create(&ctx.db, &form).await? {
        Ok(id) => Ok(Response::redirect(redirect_after_save(
            intent, admin_name, id,
        ))),
        Err(errors) => {
            let token = csrf_token(&req);
            let relation_options =
                render::resolve_relation_options(&ctx.admin, entry, &ctx.db).await?;
            let (global_errors, field_errors) = render::bucket_errors_by_label(entry, errors);
            let ctx_view = render::form_ctx(
                &identity,
                &ctx.admin,
                entry,
                "new",
                None,
                None,
                global_errors,
                token,
                relation_options,
                field_errors,
                Some(&form),
            );
            let body = ctx.templates.render("admin/form.html", &ctx_view)?;
            Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST))
        }
    }
}

/// Which `Save*` button the form submitted with. The change form has
/// three submit buttons (`_save`, `_continue`, `_addanother`); this
/// picks the redirect target after a successful create / update.
#[derive(Debug, Clone, Copy)]
enum SubmitIntent {
    Save,
    Continue,
    AddAnother,
}

fn submit_intent(form: &crate::http::FormData) -> SubmitIntent {
    if form.get("_continue").is_some() {
        SubmitIntent::Continue
    } else if form.get("_addanother").is_some() {
        SubmitIntent::AddAnother
    } else {
        SubmitIntent::Save
    }
}

fn redirect_after_save(intent: SubmitIntent, admin_name: &str, id: i64) -> String {
    match intent {
        SubmitIntent::Save => format!("/admin/{admin_name}"),
        SubmitIntent::Continue => format!("/admin/{admin_name}/{id}/edit"),
        SubmitIntent::AddAnother => format!("/admin/{admin_name}/new"),
    }
}

// ---- Edit / Update -------------------------------------------------------

pub(crate) async fn show_edit_form(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    id: i64,
    req: &Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let row = entry
        .ops
        .find_row(&ctx.db, id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("{admin_name}/{id}")))?;
    let relation_options = render::resolve_relation_options(&ctx.admin, entry, &ctx.db).await?;
    let form = render::form_ctx(
        &identity,
        &ctx.admin,
        entry,
        "edit",
        Some(id),
        Some(&row),
        vec![],
        csrf_token(req),
        relation_options,
        HashMap::new(),
        None,
    );
    let body = ctx.templates.render("admin/form.html", &form)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_update(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    id: i64,
    req: Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let form = req.form()?;
    let intent = submit_intent(&form);
    match entry.ops.update(&ctx.db, id, &form).await? {
        Ok(()) => Ok(Response::redirect(redirect_after_save(
            intent, admin_name, id,
        ))),
        Err(errors) => {
            let existing = entry.ops.find_row(&ctx.db, id).await?;
            let token = csrf_token(&req);
            let relation_options =
                render::resolve_relation_options(&ctx.admin, entry, &ctx.db).await?;
            let (global_errors, field_errors) = render::bucket_errors_by_label(entry, errors);
            let ctx_view = render::form_ctx(
                &identity,
                &ctx.admin,
                entry,
                "edit",
                Some(id),
                existing.as_ref(),
                global_errors,
                token,
                relation_options,
                field_errors,
                Some(&form),
            );
            let body = ctx.templates.render("admin/form.html", &ctx_view)?;
            Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST))
        }
    }
}

// ---- Delete --------------------------------------------------------------

pub(crate) async fn show_delete_confirm(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    id: i64,
    req: &Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let label = entry
        .ops
        .object_label(&ctx.db, id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("{admin_name}/{id}")))?;

    // Build a fresh registry from the current admin to identify which
    // models point at this one via a BelongsTo FK. Cheap — runs once
    // per delete-confirm GET, and the entry list is small.
    let registry = super::relations::RelationRegistry::from_admin_entries(ctx.admin.entries());
    let cascading: Vec<render::CascadeItem> = registry
        .has_many(entry.singular_name)
        .iter()
        .map(|inv| render::CascadeItem {
            source_display_name: inv.source_display_name.clone(),
            source_admin_name: inv.source_admin_name.clone(),
            source_field: inv.source_field.clone(),
        })
        .collect();

    let view = render::confirm_delete_ctx(
        &identity,
        &ctx.admin,
        entry,
        id,
        label,
        cascading,
        csrf_token(req),
    );
    let body = ctx.templates.render("admin/confirm_delete.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_delete(
    ctx: &AdminCtx,
    _identity: Identity,
    admin_name: &str,
    id: i64,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    entry.ops.delete(&ctx.db, id).await?;
    Ok(Response::redirect(format!("/admin/{admin_name}")))
}

// ---- History pages -------------------------------------------------------

pub(crate) async fn show_object_history(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    id: i64,
    req: &Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let label = entry
        .ops
        .object_label(&ctx.db, id)
        .await?
        .unwrap_or_else(|| format!("#{id}"));

    ensure_audit_ready(&ctx.db).await;
    let actions = audit::for_object(&ctx.db, admin_name, id)
        .await
        .unwrap_or_default();

    let view = render::ObjectHistoryCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: format!("History: {} — {}", entry.singular_name, label),
        admin_name: admin_name.to_string(),
        display_name: entry.display_name.to_string(),
        singular_name: entry.singular_name.to_string(),
        object_id: id,
        object_label: label,
        entries: render::map_audit_actions(actions),
        flash: None,
    };
    let body = ctx.templates.render("admin/object_history.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn show_log_entries(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    ensure_audit_ready(&ctx.db).await;
    let actions = audit::recent(&ctx.db, 100, None, None)
        .await
        .unwrap_or_default();
    let view = render::LogEntriesCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Recent admin actions",
        entries: render::map_audit_actions(actions),
        flash: None,
    };
    let body = ctx.templates.render("admin/log_entries.html", &view)?;
    Ok(Response::html(body))
}

// ---- Self-service password change ---------------------------------------

/// Minimum acceptable password length. argon2 accepts arbitrary input,
/// so this is policy not crypto.
const MIN_PASSWORD_LEN: usize = 8;

pub(crate) async fn show_password_change(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    let view = render::PasswordChangeCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Change password",
        errors: Vec::new(),
        success: false,
        sections: render::password_change_form_sections(),
    };
    let body = ctx.templates.render("admin/password_change.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_password_change(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let old = form.get("old_password").unwrap_or("");
    let new1 = form.get("new_password1").unwrap_or("");
    let new2 = form.get("new_password2").unwrap_or("");

    let user = auth::find_user_by_email(&ctx.db, &identity.email)
        .await?
        .ok_or_else(|| {
            Error::Internal(format!(
                "session identity {} has no matching user row",
                identity.email
            ))
        })?;

    // Push every error twice: once into the global Vec (catch-all
    // banner) and once into the field-keyed map (`apply_field_errors`
    // copies the matching entry onto each FormField at re-render
    // time). Both views render from the same source of truth.
    let mut errors: Vec<String> = Vec::new();
    let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();
    if !auth::verify_password(old, &user.password_hash) {
        let msg = "Your old password was entered incorrectly. Please enter it again.";
        errors.push(msg.into());
        field_errors
            .entry("old_password".into())
            .or_default()
            .push(msg.into());
    }
    if new1 != new2 {
        let msg = "The two password fields didn't match.";
        errors.push(msg.into());
        field_errors
            .entry("new_password2".into())
            .or_default()
            .push(msg.into());
    }
    if new1.len() < MIN_PASSWORD_LEN {
        let msg = format!(
            "This password is too short. It must contain at least {MIN_PASSWORD_LEN} characters."
        );
        errors.push(msg.clone());
        field_errors
            .entry("new_password1".into())
            .or_default()
            .push(msg);
    }

    if errors.is_empty() {
        auth::set_password(&ctx.db, user.id, new1).await?;
        let view = render::PasswordChangeCtx {
            base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
            page_title: "Password changed",
            errors: Vec::new(),
            success: true,
            sections: Vec::new(),
        };
        let body = ctx.templates.render("admin/password_change.html", &view)?;
        return Ok(Response::html(body));
    }

    let mut sections = render::password_change_form_sections();
    render::apply_field_errors(&mut sections, &field_errors);
    let view = render::PasswordChangeCtx {
        base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
        page_title: "Change password",
        errors,
        success: false,
        sections,
    };
    let body = ctx.templates.render("admin/password_change.html", &view)?;
    Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST))
}
