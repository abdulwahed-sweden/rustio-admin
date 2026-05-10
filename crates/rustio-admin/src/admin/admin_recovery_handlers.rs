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

use std::collections::HashMap;

use serde::Serialize;

use crate::auth::{self, Identity};
use crate::error::Result;
use crate::http::{Request, Response};

use super::audit;
use super::builtin::{client_ip, correlation_id_from};
use super::handlers::{csrf_token, AdminCtx};
use super::render::{self, BaseContext, FormSection};

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

// ---- Forced password rotation (R2 commit #12) ------------------------------
//
// `must_change_password = TRUE` is set by the admin-driven password
// reset (R2 commit #15) — typically alongside a temp password the
// admin issued. The user signs in with that temp password, the
// `login_guard` (R2 commit #13) sees the flag, and redirects every
// non-whitelisted /admin/* request to `/admin/must-change-password`.
// The interstitial below is the only writeable path while the flag
// is set.

#[derive(Serialize)]
struct MustChangePasswordCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    sections: Vec<FormSection>,
    /// Form-level error banner. Field-keyed errors live on the
    /// individual `FormField` rows via `apply_field_errors`.
    errors: Vec<String>,
}

/// `GET /admin/must-change-password` — render the rotation form.
///
/// The handler refuses (303 → /admin) when `must_change_password`
/// is FALSE on the identity, so a curious authenticated user can't
/// bypass the old-password requirement that the regular
/// `/admin/password_change` flow enforces.
#[allow(dead_code)] // route registration lands in R2 commit #17
pub(crate) async fn show_must_change_password(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    if !identity.must_change_password {
        return Ok(Response::redirect("/admin"));
    }
    let min_length = ctx.admin.active_password_policy().min_length();
    let view = MustChangePasswordCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Set a new password",
        sections: render::must_change_password_form_sections(min_length),
        errors: Vec::new(),
    };
    let body = ctx
        .templates
        .render("admin/must_change_password.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/must-change-password` — apply the rotation.
///
/// On success:
/// 1. `auth::set_password` writes the new hash + stamps
///    `password_changed_at`.
/// 2. `UPDATE rustio_users SET must_change_password = FALSE`.
/// 3. `auth::invalidate_sessions(UserExceptCurrent, UserRequested)` —
///    the current device stays signed in (Doctrine 22: the engine
///    is the sole writer of `revoked_at`).
/// 4. One `AuditEvent::SessionsRevokedSelf` row per revoked sibling
///    session via `handlers::record_session_revocations`.
/// 5. One `AuditEvent::ForcedPasswordChangeCompleted` row with
///    metadata `{ triggered_by_audit_id?, invalidated_session_count }`.
///    The chain `PasswordResetByOther → ForcedPasswordChangeCompleted
///    → N × SessionsRevokedSelf` (`DESIGN_R2_ORGANISATIONAL.md` §5.3)
///    is now complete.
/// 6. PRG: 303 → /admin.
///
/// Validation failures re-render the form with field-keyed errors
/// and HTTP 400; password is never echoed back into the form value.
///
/// Defensive checks:
/// - empty / mismatched passwords → field-level errors;
/// - `PasswordPolicy::validate(...)` rejection → field error on
///   `new_password1` with the policy's user-safe message;
/// - **rejecting reuse of the current password** — prevents the
///   user "rotating" to the same temp password the admin issued
///   (the temp value may have been logged or shared); forces a
///   fresh secret the admin no longer knows.
#[allow(dead_code)] // route registration lands in R2 commit #17
pub(crate) async fn do_must_change_password(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    if !identity.must_change_password {
        return Ok(Response::redirect("/admin"));
    }
    let form = req.form()?;
    let new1 = form.get("new_password1").unwrap_or("").to_string();
    let new2 = form.get("new_password2").unwrap_or("").to_string();

    let mut errors: Vec<String> = Vec::new();
    let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

    if new1.is_empty() {
        let msg = "Enter a new password.".to_string();
        errors.push(msg.clone());
        field_errors
            .entry("new_password1".into())
            .or_default()
            .push(msg);
    } else if new1 != new2 {
        let msg = "Passwords do not match.".to_string();
        errors.push(msg.clone());
        field_errors
            .entry("new_password2".into())
            .or_default()
            .push(msg);
    } else if let Err(e) = ctx.admin.active_password_policy().validate(&new1) {
        let msg = e.to_string();
        errors.push(msg.clone());
        field_errors
            .entry("new_password1".into())
            .or_default()
            .push(msg);
    }

    // Reject reuse of the current password. Done after the cheap
    // validations to avoid an unnecessary Argon2 verify on a
    // mismatched / empty input.
    let user = if errors.is_empty() {
        match auth::find_user_by_email(&ctx.db, &identity.email).await? {
            Some(u) => Some(u),
            None => {
                // The user disappeared between cookie load and POST —
                // fail soft. Render the form with a generic banner so
                // the next request goes through login_guard which will
                // redirect to the login page.
                let msg = "Could not load your account. Please sign in again.".to_string();
                errors.push(msg);
                None
            }
        }
    } else {
        None
    };
    if errors.is_empty() {
        if let Some(u) = user.as_ref() {
            if auth::verify_password(&new1, &u.password_hash) {
                let msg = "New password must be different from your current password.".to_string();
                errors.push(msg.clone());
                field_errors
                    .entry("new_password1".into())
                    .or_default()
                    .push(msg);
            }
        }
    }

    if !errors.is_empty() {
        let min_length = ctx.admin.active_password_policy().min_length();
        let mut sections = render::must_change_password_form_sections(min_length);
        render::apply_field_errors(&mut sections, &field_errors);
        let view = MustChangePasswordCtx {
            base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
            page_title: "Set a new password",
            sections,
            errors,
        };
        let body = ctx
            .templates
            .render("admin/must_change_password.html", &view)?;
        return Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST));
    }

    // Apply the rotation.
    auth::set_password(&ctx.db, identity.user_id, &new1).await?;
    sqlx::query("UPDATE rustio_users SET must_change_password = FALSE WHERE id = $1")
        .bind(identity.user_id)
        .execute(ctx.db.pool())
        .await?;

    // Resolve current session (to spare the current device) and
    // invalidate every other session for this user. Doctrine 22:
    // `invalidate_sessions` is the sole writer of `revoked_at`.
    let cookie_token = req
        .header("cookie")
        .and_then(crate::auth::session_token_from_cookie);
    let current_session_id = match &cookie_token {
        Some(t) => crate::auth::current_session_id(&ctx.db, t).await?,
        None => None,
    };
    let target = match current_session_id {
        Some(sid) => crate::auth::SessionTarget::UserExceptCurrent {
            user_id: identity.user_id,
            current_session_id: sid,
        },
        None => crate::auth::SessionTarget::User {
            user_id: identity.user_id,
        },
    };
    let outcome = crate::auth::invalidate_sessions(
        &ctx.db,
        target,
        crate::auth::SessionInvalidationReason::UserRequested,
    )
    .await?;
    let revoked_count = outcome.revoked_session_ids.len();

    // Per-revoked-session SessionsRevokedSelf audit rows. Same helper
    // R1's do_password_change uses; via='must_change_password'
    // distinguishes the source in the metadata.
    super::handlers::record_session_revocations(
        ctx,
        &identity,
        &outcome.revoked_session_ids,
        &req,
        "must_change_password",
    )
    .await;

    // Pivot link to the originating PasswordResetByOther row, if
    // any — populated when an admin reset (R2 commit #15) is what
    // set the flag. Best-effort: a missing row simply omits the
    // metadata key.
    let triggered_by_audit_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM rustio_admin_actions \
          WHERE action_type = 'password_reset_by_other' \
            AND object_id = $1 \
          ORDER BY id DESC LIMIT 1",
    )
    .bind(identity.user_id)
    .fetch_optional(ctx.db.pool())
    .await
    .unwrap_or(None);

    // Emit the typed ForcedPasswordChangeCompleted row. Self-action:
    // actor = subject; LogEntry::user_id carries it (per §5.2 the
    // typed `actor_user_id` is None for self-actions to keep the
    // metadata footprint minimal).
    let cid = correlation_id_from(&req);
    let ip = client_ip(&req);
    let metadata = match triggered_by_audit_id {
        Some(id) => serde_json::json!({
            "triggered_by_audit_id": id,
            "invalidated_session_count": revoked_count,
        }),
        None => serde_json::json!({
            "invalidated_session_count": revoked_count,
        }),
    };
    let _ = audit::record(
        &ctx.db,
        audit::LogEntry {
            user_id: identity.user_id,
            action_type: audit::ActionType::Update,
            model_name: "user",
            object_id: identity.user_id,
            ip_address: ip.as_deref(),
            summary: format!(
                "forced password rotation completed; {revoked_count} other session(s) revoked"
            ),
            correlation_id: cid.as_deref(),
            session_id: None,
            metadata: Some(metadata),
            actor_user_id: None,
            event: Some(audit::AuditEvent::ForcedPasswordChangeCompleted),
        },
    )
    .await;

    Ok(Response::redirect("/admin"))
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
