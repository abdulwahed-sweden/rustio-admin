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
    let mut rows = entry.ops.list(&ctx.db).await?;

    // In-memory search/filter/pagination. Pushdown to AdminOps would
    // mean touching types.rs (out of scope for P6). Acceptable for
    // small model lists; revisit when a project hits >10k rows.
    let qs = req.query();
    let search = qs.get("q").unwrap_or_default().to_string();
    if !search.is_empty() {
        let needle = search.to_ascii_lowercase();
        rows.retain(|r| {
            r.cells
                .iter()
                .any(|c| c.to_ascii_lowercase().contains(&needle))
        });
    }

    let mut filter_groups: Vec<render::FilterGroupCtx> = Vec::new();
    for f in super::filters::infer_filters(entry.fields) {
        let current = qs.get(&f.field).map(str::to_string);
        if let Some(val) = &current {
            if !val.is_empty() {
                let col_idx = entry.fields.iter().position(|af| af.name == f.field);
                if let Some(idx) = col_idx {
                    rows.retain(|r| r.cells.get(idx).map(String::as_str) == Some(val.as_str()));
                }
            }
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
            // P6 renders only Bool filters interactively. Other kinds
            // (DateRange, Dropdown, NumericExact, ExactMatch,
            // RelationSelect) need richer widgets — later phases.
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

    let total_rows = rows.len();
    let per_page: usize = qs
        .get("per_page")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| matches!(n, 10 | 25 | 50 | 100))
        .unwrap_or(25);
    let total_pages = total_rows.div_ceil(per_page.max(1)).max(1);
    let page_raw: usize = qs.get("p").and_then(|s| s.parse().ok()).unwrap_or(1).max(1);
    let page = page_raw.min(total_pages);
    let start = (page - 1) * per_page;
    let page_rows: Vec<_> = rows.into_iter().skip(start).take(per_page).collect();

    let list = render::list_ctx(
        &identity,
        &ctx.admin,
        entry,
        page_rows,
        search,
        filter_groups,
        page,
        per_page,
        total_rows,
        csrf_token(req),
    );
    let body = ctx.templates.render("admin/list.html", &list)?;
    Ok(Response::html(body))
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
