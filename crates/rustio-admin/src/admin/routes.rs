//! Admin route registration with permission checks.
//!
//! Every admin URL is gated by a specific permission:
//!   GET  /admin/:model            → posts.view_post
//!   GET  /admin/:model/new        → posts.add_post
//!   POST /admin/:model/new        → posts.add_post
//!   GET  /admin/:model/:id/edit   → posts.change_post
//!   POST /admin/:model/:id/edit   → posts.change_post
//!   GET  /admin/:model/:id/delete → posts.delete_post
//!   POST /admin/:model/:id/delete → posts.delete_post
//!
//! Administrator + Developer bypass every check (see
//! `Role::bypasses_group_checks`). Staff and Supervisor need the
//! specific permission granted either directly or via a group.
//!
//! Slimmed for Tier 1: the legacy file's static-asset routes
//! (P8 reintroduces them once `admin.css`/`admin.js` exist), the
//! developer stub routes (`__schema__`, `__logs__`, `__sql_console__`),
//! and the FK remote-search endpoint have been dropped. Everything
//! else — login/logout, dashboard, /admin/users/*, /admin/groups/*,
//! /admin/history, /admin/password_change, /admin/:model/* CRUD,
//! /admin/:model/:id/history — is wired below.

use std::sync::Arc;

use crate::auth::{self, Identity, Role};
use crate::error::{Error, Result};
use crate::http::{Request, Response};
use crate::orm::Db;
use crate::router::Router;
use crate::templates::Templates;

use super::handlers::{self, AdminCtx};
use super::render;
use super::types::Admin;

/// Either an identity + a permission check passed, or any non-Allow
/// response the route closure should return as-is (a 303 redirect to
/// /admin/login, a 403 forbidden body, etc.).
enum Guard {
    Allow(Identity),
    Redirect(Response),
}

async fn login_guard(ctx: &AdminCtx, req: &Request) -> Result<Guard> {
    let cookie = match req.header("cookie") {
        Some(c) => c,
        None => return Ok(Guard::Redirect(Response::redirect("/admin/login"))),
    };
    let token = match auth::session_token_from_cookie(cookie) {
        Some(t) => t,
        None => return Ok(Guard::Redirect(Response::redirect("/admin/login"))),
    };
    let ident = match auth::identity_from_session(&ctx.db, &token).await? {
        Some(i) => i,
        None => return Ok(Guard::Redirect(Response::redirect("/admin/login"))),
    };
    if !ident.is_active {
        return Ok(Guard::Redirect(Response::redirect("/admin/login")));
    }
    Ok(Guard::Allow(ident))
}

async fn role_guard(ctx: &AdminCtx, req: &Request, min: Role) -> Result<Guard> {
    match login_guard(ctx, req).await? {
        Guard::Redirect(r) => Ok(Guard::Redirect(r)),
        Guard::Allow(ident) => {
            if ident.role.includes(min) {
                Ok(Guard::Allow(ident))
            } else {
                let body = render::render_forbidden_body(
                    &ctx.admin,
                    &ctx.templates,
                    &ident,
                    handlers::csrf_token(req),
                    None,
                    Some(min.label()),
                )?;
                Ok(Guard::Redirect(
                    Response::html(body).with_status(hyper::StatusCode::FORBIDDEN),
                ))
            }
        }
    }
}

async fn perm_guard(ctx: &AdminCtx, req: &Request, perm: &str) -> Result<Guard> {
    match role_guard(ctx, req, Role::Staff).await? {
        Guard::Redirect(r) => Ok(Guard::Redirect(r)),
        Guard::Allow(ident) => {
            if ident.role.bypasses_group_checks() {
                return Ok(Guard::Allow(ident));
            }
            if auth::check_permission(&ctx.db, &ident, perm).await? {
                Ok(Guard::Allow(ident))
            } else {
                let body = render::render_forbidden_body(
                    &ctx.admin,
                    &ctx.templates,
                    &ident,
                    handlers::csrf_token(req),
                    Some(perm.to_string()),
                    None,
                )?;
                Ok(Guard::Redirect(
                    Response::html(body).with_status(hyper::StatusCode::FORBIDDEN),
                ))
            }
        }
    }
}

/// Pure decision logic for `perm_guard`, factored out so it can be
/// unit-tested without a `Db`.
#[cfg(test)]
fn perm_guard_verdict(ident: &Identity, perm_held: bool) -> bool {
    if !ident.is_active {
        return false;
    }
    if ident.role.bypasses_group_checks() {
        return true;
    }
    perm_held
}

fn parse_id(raw: Option<&str>) -> Result<i64> {
    raw.and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::BadRequest("invalid id".into()))
}

fn model_name_from_req(req: &Request) -> Result<String> {
    req.param("admin_name")
        .map(|s| s.to_string())
        .ok_or_else(|| Error::BadRequest("missing model".into()))
}

fn perm_for(ctx: &AdminCtx, admin_name: &str, action: &str) -> Result<String> {
    let entry = ctx
        .admin
        .find(admin_name)
        .ok_or_else(|| Error::NotFound(format!("no admin model: {admin_name}")))?;
    let singular = entry.singular_name.to_ascii_lowercase();
    Ok(format!("{admin_name}.{action}_{singular}"))
}

pub fn register_admin_routes(
    router: Router,
    admin: Admin,
    db: Db,
    templates: Arc<Templates>,
) -> Router {
    let ctx = Arc::new(AdminCtx::new(
        Arc::new(admin),
        db.clone(),
        templates.clone(),
    ));

    // Bespoke user/group pages share the same DB / templates / Admin
    // arc but live in their own ctx type with the same shape.
    let auth_ctx = Arc::new(super::builtin::AuthAdminCtx {
        admin: ctx.admin.clone(),
        db,
        templates,
    });

    // Render `Err(_)` from /admin/* handlers as styled HTML instead of
    // the framework default `text/plain`. Non-admin paths bubble
    // through unchanged so JSON / curl consumers still get the text
    // body. `Error::Forbidden` (handled by `role_guard` via
    // `admin/forbidden.html`) and login-required redirects come
    // through as `Ok` responses and bypass this branch.
    let err_admin = ctx.admin.clone();
    let err_templates = ctx.templates.clone();
    let router = router.middleware(move |req, next| {
        let admin = err_admin.clone();
        let templates = err_templates.clone();
        Box::pin(async move {
            let is_admin_path = req.path().starts_with("/admin");
            let result = next.run(req).await;
            match result {
                Ok(resp) => Ok(resp),
                Err(err) if is_admin_path => Ok(render::render_admin_error_response(
                    &admin,
                    &templates,
                    None,
                    err.status(),
                    err.client_message().to_string(),
                )),
                Err(err) => Err(err),
            }
        })
    });

    // Public: login/logout.
    let c = ctx.clone();
    let router = router.get("/admin/login", move |req| {
        let c = c.clone();
        async move { handlers::show_login(&c, req).await }
    });

    let c = ctx.clone();
    let router = router.post("/admin/login", move |req| {
        let c = c.clone();
        async move { handlers::do_login(&c, req).await }
    });

    let c = ctx.clone();
    let router = router.post("/admin/logout", move |req| {
        let c = c.clone();
        async move { handlers::do_logout(&c, req).await }
    });

    // Dashboard — Staff floor. User-tier sees the forbidden page.
    let c = ctx.clone();
    let router = router.get("/admin", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Staff).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::dashboard(&c, ident, &req).await,
            }
        }
    });

    // Global history log (admin-only; high-signal page).
    let c = ctx.clone();
    let router = router.get("/admin/history", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::show_log_entries(&c, ident, &req).await,
            }
        }
    });

    // Self-service password change. Any logged-in user (User-tier and
    // above). User-tier can change their own password even though
    // they can't access the dashboard.
    let c = ctx.clone();
    let router = router.get("/admin/password_change", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::show_password_change(&c, ident, &req).await,
            }
        }
    });
    let c = ctx.clone();
    let router = router.post("/admin/password_change", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::do_password_change(&c, ident, req).await,
            }
        }
    });

    // --- Built-in users admin (admin-only) ---
    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/users", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::builtin::list_users(&ac, ident, handlers::csrf_token(&req)).await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/users/new", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::builtin::show_new_user(&ac, ident, handlers::csrf_token(&req)).await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.post("/admin/users/new", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::builtin::do_new_user(&ac, ident, req).await,
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/users/:id/edit", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::show_user_edit(&ac, ident, id, handlers::csrf_token(&req)).await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.post("/admin/users/:id/edit", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::do_user_edit(&ac, ident, id, req).await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/users/:id/delete", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::show_user_delete(&ac, ident, id, handlers::csrf_token(&req))
                        .await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.post("/admin/users/:id/delete", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::do_user_delete(&ac, ident, id, req).await
                }
            }
        }
    });

    // Read-only user profile view. MUST be registered AFTER
    // `/admin/users/new` and the `:id/edit` + `:id/delete` routes
    // above: the router matches in insertion order, and `:id` is a
    // wildcard that would happily swallow "new" or extra path
    // segments. Putting this last preserves the more-specific routes'
    // priority.
    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/users/:id", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    let q = req.query();
                    let tab = q.get("tab").map(|s| s.to_string());
                    let page: i64 = q.get("page").and_then(|s| s.parse().ok()).unwrap_or(1);
                    super::builtin::show_user_view(
                        &ac,
                        ident,
                        id,
                        handlers::csrf_token(&req),
                        tab,
                        page,
                    )
                    .await
                }
            }
        }
    });

    // --- Built-in groups admin (admin-only) ---
    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/groups", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::builtin::list_groups(&ac, ident, handlers::csrf_token(&req)).await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/groups/new", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::builtin::show_new_group(&ac, ident, handlers::csrf_token(&req)).await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.post("/admin/groups/new", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::builtin::do_new_group(&ac, ident, req).await,
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/groups/:id/edit", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::show_group_edit(&ac, ident, id, handlers::csrf_token(&req))
                        .await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.post("/admin/groups/:id/edit", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::do_group_edit(&ac, ident, id, req).await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.get("/admin/groups/:id/delete", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::show_group_delete(&ac, ident, id, handlers::csrf_token(&req))
                        .await
                }
            }
        }
    });

    let c = ctx.clone();
    let ac = auth_ctx.clone();
    let router = router.post("/admin/groups/:id/delete", move |req| {
        let c = c.clone();
        let ac = ac.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::builtin::do_group_delete(&ac, ident, id, req).await
                }
            }
        }
    });

    // Per-model list — needs `view` permission.
    let c = ctx.clone();
    let router = router.get("/admin/:admin_name", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "view")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::list_model(&c, ident, &name, &req).await,
            }
        }
    });

    // Create.
    let c = ctx.clone();
    let router = router.get("/admin/:admin_name/new", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "add")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::show_new_form(&c, ident, &name, &req).await,
            }
        }
    });
    let c = ctx.clone();
    let router = router.post("/admin/:admin_name/new", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "add")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::do_create(&c, ident, &name, req).await,
            }
        }
    });

    // Edit.
    let c = ctx.clone();
    let router = router.get("/admin/:admin_name/:id/edit", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "change")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::show_edit_form(&c, ident, &name, id, &req).await
                }
            }
        }
    });
    let c = ctx.clone();
    let router = router.post("/admin/:admin_name/:id/edit", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "change")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::do_update(&c, ident, &name, id, req).await
                }
            }
        }
    });

    // Per-object history. Read-only; same `view` permission as the
    // changelist (if you can list, you can read the audit trail).
    let c = ctx.clone();
    let router = router.get("/admin/:admin_name/:id/history", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "view")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::show_object_history(&c, ident, &name, id, &req).await
                }
            }
        }
    });

    // Delete.
    let c = ctx.clone();
    let router = router.get("/admin/:admin_name/:id/delete", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "delete")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::show_delete_confirm(&c, ident, &name, id, &req).await
                }
            }
        }
    });
    let c = ctx.clone();
    router.post("/admin/:admin_name/:id/delete", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "delete")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::do_delete(&c, ident, &name, id).await
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_identity(role: Role, is_active: bool) -> Identity {
        Identity {
            user_id: 42,
            email: "test@example.com".into(),
            role,
            is_active,
            is_demo: false,
            demo_label: None,
        }
    }

    // role_guard's decision is `Role::includes(min)`. The 25-case
    // matrix lives in `auth::role::tests::includes_matrix_…`; the
    // cases below pin the most operator-relevant pairings.

    #[test]
    fn role_guard_decision_admin_meets_staff_floor() {
        let id = make_identity(Role::Administrator, true);
        assert!(id.role.includes(Role::Staff));
    }

    #[test]
    fn role_guard_decision_user_does_not_meet_staff() {
        let id = make_identity(Role::User, true);
        assert!(!id.role.includes(Role::Staff));
    }

    #[test]
    fn role_guard_decision_administrator_does_not_meet_developer() {
        let id = make_identity(Role::Administrator, true);
        assert!(!id.role.includes(Role::Developer));
    }

    #[test]
    fn role_guard_decision_developer_meets_everything() {
        let id = make_identity(Role::Developer, true);
        for &min in &[
            Role::User,
            Role::Staff,
            Role::Supervisor,
            Role::Administrator,
            Role::Developer,
        ] {
            assert!(id.role.includes(min), "Developer should meet {min:?}");
        }
    }

    // ---- perm_guard_verdict matrix --------------------------------------

    #[test]
    fn perm_guard_admin_short_circuits_without_perm() {
        let id = make_identity(Role::Administrator, true);
        assert!(perm_guard_verdict(&id, false));
    }

    #[test]
    fn perm_guard_developer_short_circuits_without_perm() {
        let id = make_identity(Role::Developer, true);
        assert!(perm_guard_verdict(&id, false));
    }

    #[test]
    fn perm_guard_staff_with_perm_passes() {
        let id = make_identity(Role::Staff, true);
        assert!(perm_guard_verdict(&id, true));
    }

    #[test]
    fn perm_guard_staff_without_perm_denies() {
        let id = make_identity(Role::Staff, true);
        assert!(!perm_guard_verdict(&id, false));
    }

    #[test]
    fn perm_guard_inactive_admin_denies_even_with_bypass() {
        // Defense-in-depth invariant.
        let id = make_identity(Role::Administrator, false);
        assert!(!perm_guard_verdict(&id, true));
    }

    #[test]
    fn perm_guard_supervisor_without_perm_denies() {
        // Supervisor doesn't bypass; needs the per-model perm.
        let id = make_identity(Role::Supervisor, true);
        assert!(!perm_guard_verdict(&id, false));
    }
}
