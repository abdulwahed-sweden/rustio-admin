//! HTTP handlers for the R2 organisational-recovery surface
//! (`DESIGN_R2_ORGANISATIONAL.md` §3.5 + §11). R2 commit #11 lands
//! the standalone re-auth wall; the admin-driven password reset,
//! lock, unlock, and revoke handlers land in commits #15 / #16, and
//! the must-change-password interstitial lands in commit #12.
//!
//! ## Re-auth wall (this commit)
//!
//! - `GET  /admin/reauth` → [`show_reauth`]
//! - `POST /admin/reauth` → [`do_reauth`]
//!
//! Routes are NOT registered in this commit; commit #17 wires every
//! R2 route together. The handlers exist so the wall is functionally
//! complete and unit-testable in isolation.
//!
//! ## What this module does NOT do
//!
//! - Sensitive route integration. Commits #15 / #16 add
//!   `check_session_elevated` calls in front of admin reset / lock /
//!   unlock / revoke handlers. Today nothing requires re-auth.
//! - Audit rows. Re-auth itself does not write to
//!   `rustio_admin_actions` — the audit chain begins at the
//!   destructive action (commits #14-#16). A failed re-auth attempt
//!   is silently re-rendered with a uniform error, no log row.
//! - Session revocation. Doctrine 22 untouched; promotion only
//!   extends the elevation window via
//!   `auth::recovery_admin::promote_session_elevated`, which never
//!   touches `revoked_at`.
//!
//! ## `return_to` validation
//!
//! [`validate_return_to`] is the security-load-bearing helper. The
//! input is attacker-controlled (query string on GET, hidden form
//! field on POST). The validator accepts only:
//!
//! - exactly `/admin`
//! - any path beginning with `/admin/` or `/admin?`
//!
//! and rejects:
//!
//! - empty / whitespace-only inputs;
//! - any control byte (CR, LF, NUL, ≤ 0x1f, 0x7f) — defends against
//!   header injection in the redirect Location;
//! - protocol-relative URLs starting with `//` — those resolve to a
//!   different origin in the browser;
//! - backslash anywhere — some browsers normalise `\\` → `//`;
//! - `..` anywhere — defensive path-traversal block;
//! - any input not starting with `/admin` (rejects schemes like
//!   `javascript:`, `data:`, absolute URLs, and bare paths outside
//!   the admin surface).
//!
//! Failed validation collapses to `/admin` (the dashboard) — the
//! safe default. Open-redirect through `?return_to=` is closed.

use serde::Serialize;

use crate::auth::{self, Identity};
use crate::error::Result;
use crate::http::{Request, Response};

use super::handlers::{csrf_token, AdminCtx};
use super::render::BaseContext;

// ---- Pure helpers (unit-testable without DB / Request) ---------------------

/// Validate a candidate `return_to` URL. Returns `Some(path)` only
/// for safe internal admin paths; otherwise `None`. See module-level
/// docs for the exact accept/reject rules.
fn validate_return_to(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    // Reject control bytes (header-injection defense). Includes
    // CR (0x0d), LF (0x0a), NUL (0x00), tab (0x09), DEL (0x7f),
    // and every other byte below 0x20.
    if raw.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return None;
    }
    // Protocol-relative URLs: `//external.com/path` resolves to
    // http(s)://external.com.
    if raw.starts_with("//") {
        return None;
    }
    // Backslash: some browsers normalise `\\evil.com` to `//evil.com`.
    if raw.contains('\\') {
        return None;
    }
    // Path traversal — defensive. Even though the router itself
    // doesn't dereference `..`, reject unconditionally so projects
    // that mount additional routes inside `/admin/` get the same
    // floor.
    if raw.contains("..") {
        return None;
    }
    // Must be inside the /admin surface.
    if raw == "/admin" || raw.starts_with("/admin/") || raw.starts_with("/admin?") {
        Some(raw.to_string())
    } else {
        None
    }
}

/// Resolve the destination of the post-success redirect. The handler
/// reads the form's hidden `return_to`; this fn collapses an invalid
/// or missing value to `/admin` (the dashboard).
fn redirect_after_reauth(return_to: Option<&str>) -> String {
    return_to
        .and_then(validate_return_to)
        .unwrap_or_else(|| "/admin".to_string())
}

// ---- Render context --------------------------------------------------------

#[derive(Serialize)]
struct ReauthCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    /// Actor's email, displayed read-only above the password input
    /// so the user understands which identity they're confirming.
    email: String,
    /// Validated `return_to` path; embedded into the form as a
    /// hidden field and re-validated server-side on POST.
    return_to: String,
    /// Uniform error banner. Set on bad password (or any other
    /// failure mode that collapses to "could not verify"); never
    /// distinguishes which underlying check failed.
    error: Option<String>,
}

// ---- Handlers --------------------------------------------------------------

/// `GET /admin/reauth` — render the password-confirmation form.
///
/// Reads `?return_to=<path>` from the query string and validates it
/// before passing through to the template. An invalid or missing
/// value collapses to `/admin`. The rendered hidden field is the
/// VALIDATED value, never the raw query string — the POST handler
/// therefore receives a value that has already passed at least one
/// validation pass (and re-validates anyway).
#[allow(dead_code)] // route registration lands in R2 commit #17
pub(crate) async fn show_reauth(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    let raw = req.query().get("return_to").map(|s| s.to_string());
    let return_to = redirect_after_reauth(raw.as_deref());
    let view = ReauthCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Confirm your identity",
        email: identity.email.clone(),
        return_to,
        error: None,
    };
    let body = ctx.templates.render("admin/reauth.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/reauth` — verify the actor's password and promote
/// their current session into the elevated trust band per
/// [`crate::auth::recovery_admin::promote_session_elevated`].
///
/// Success: 303 redirect to the validated `return_to` (or `/admin`
/// if invalid / missing). Preserves PRG so a refresh on the
/// destination URL is a plain GET.
///
/// Failure (any cause): re-render the form with a single uniform
/// "Could not verify your password." error and HTTP 401. No audit
/// row, no `revoked_at` mutation. Doctrine 22 holds — promotion
/// writes only `elevated_until` and `trust_level`, never
/// `revoked_at`.
#[allow(dead_code)] // route registration lands in R2 commit #17
pub(crate) async fn do_reauth(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let password = form.get("password").unwrap_or("").to_string();
    let raw_return_to = form.get("return_to").map(|s| s.to_string());
    let return_to = redirect_after_reauth(raw_return_to.as_deref());

    // Single uniform failure renderer used by every error branch
    // below. Same status, same wording, same CSRF refresh — no
    // distinction between "user gone", "wrong password", or "no
    // current session" reaches the client.
    let uniform_failure = |ctx: &AdminCtx, return_to: &str| -> Result<Response> {
        let view = ReauthCtx {
            base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
            page_title: "Confirm your identity",
            email: identity.email.clone(),
            return_to: return_to.to_string(),
            error: Some("Could not verify your password.".to_string()),
        };
        let body = ctx.templates.render("admin/reauth.html", &view)?;
        Ok(Response::html(body).with_status(hyper::StatusCode::UNAUTHORIZED))
    };

    // Look the user up by email (Identity carries the email but not
    // the password hash). A None here would only happen if the user
    // was deleted between the cookie's session row resolving and
    // this POST landing — fail uniformly.
    let user = match auth::find_user_by_email(&ctx.db, &identity.email).await? {
        Some(u) => u,
        None => return uniform_failure(ctx, &return_to),
    };
    if !auth::verify_password(&password, &user.password_hash) {
        return uniform_failure(ctx, &return_to);
    }

    // Resolve the current session id via the cookie token. Failure
    // here also collapses to the uniform error — without a session
    // we have nothing to promote, and showing a different page
    // would leak that state to the actor.
    let cookie = match req.header("cookie") {
        Some(c) => c,
        None => return uniform_failure(ctx, &return_to),
    };
    let token = match auth::session_token_from_cookie(cookie) {
        Some(t) => t,
        None => return uniform_failure(ctx, &return_to),
    };
    let session_id = match auth::current_session_id(&ctx.db, &token).await? {
        Some(id) => id,
        None => return uniform_failure(ctx, &return_to),
    };

    let ttl = ctx.admin.active_recovery_policy().reauth_window();
    crate::auth::recovery_admin::promote_session_elevated(&ctx.db, session_id, ttl).await?;

    // PRG: 303 See Other so a refresh on the destination is a GET.
    Ok(Response::redirect(return_to))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- validate_return_to ------------------------------------------------

    #[test]
    fn valid_internal_admin_paths_are_accepted() {
        for path in [
            "/admin",
            "/admin/",
            "/admin/users",
            "/admin/users/42/reset-password",
            "/admin/users/42/lock",
            "/admin?ok=1",
            "/admin/users?role=admin&page=2",
        ] {
            assert_eq!(
                validate_return_to(path),
                Some(path.to_string()),
                "expected accept for {path:?}"
            );
        }
    }

    #[test]
    fn external_urls_are_rejected() {
        for raw in [
            "https://evil.com/admin",
            "http://evil.com/admin",
            "//evil.com/admin",    // protocol-relative
            "//evil.com",          // protocol-relative without path
            r"\\evil.com\admin",   // backslash → some browsers normalise to //
            "javascript:alert(1)", // scheme without //
            "data:text/html,<script>alert(1)</script>",
            "ftp://evil.com/admin",
        ] {
            assert!(
                validate_return_to(raw).is_none(),
                "expected reject for {raw:?}"
            );
        }
    }

    #[test]
    fn paths_outside_admin_surface_are_rejected() {
        for raw in [
            "/",
            "/login",
            "/admin-evil",       // close-but-not-/admin
            "/admin\u{0000}/x",  // NUL byte
            "/static/admin.css", // sibling surface, not /admin
            "/api/users",
            "  /admin/users", // leading whitespace prevents starts_with
            "/Admin/users",   // case-sensitive — only literal /admin
        ] {
            assert!(
                validate_return_to(raw).is_none(),
                "expected reject for {raw:?}"
            );
        }
    }

    #[test]
    fn path_traversal_is_rejected() {
        for raw in [
            "/admin/../etc/passwd",
            "/admin/users/..",
            "/admin/..//evil.com",
            "/admin/../../",
        ] {
            assert!(
                validate_return_to(raw).is_none(),
                "expected reject for {raw:?}"
            );
        }
    }

    #[test]
    fn malformed_inputs_are_rejected() {
        // Empty, control bytes, CR/LF (header injection), DEL.
        assert!(validate_return_to("").is_none());
        assert!(validate_return_to("/admin\r\nLocation: /evil").is_none());
        assert!(validate_return_to("/admin\n").is_none());
        assert!(validate_return_to("/admin\t").is_none());
        assert!(validate_return_to("/admin\x7f/").is_none());
    }

    // ---- redirect_after_reauth ---------------------------------------------

    #[test]
    fn redirect_collapses_invalid_to_admin_dashboard() {
        assert_eq!(redirect_after_reauth(None), "/admin");
        assert_eq!(redirect_after_reauth(Some("")), "/admin");
        assert_eq!(redirect_after_reauth(Some("https://evil.com")), "/admin");
        assert_eq!(redirect_after_reauth(Some("//evil.com")), "/admin");
        assert_eq!(redirect_after_reauth(Some("/login")), "/admin");
    }

    #[test]
    fn redirect_passes_valid_internal_paths() {
        assert_eq!(
            redirect_after_reauth(Some("/admin/users/42/reset-password")),
            "/admin/users/42/reset-password"
        );
        assert_eq!(redirect_after_reauth(Some("/admin")), "/admin");
        assert_eq!(
            redirect_after_reauth(Some("/admin?next=1")),
            "/admin?next=1"
        );
    }
}
