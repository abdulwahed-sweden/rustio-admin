//! HTTP handlers for the R2 organisational-recovery surface
//! (`DESIGN_R2_ORGANISATIONAL.md` §3 + §11). Routes are registered
//! in `admin::routes::register_admin_routes` (R2 commit #17). This
//! module owns the handlers themselves; runtime fns live in
//! `auth::recovery_admin`.
//!
//! ## Routes wired through this module
//!
//! - `GET  /admin/reauth`                       → [`show_reauth`]
//! - `POST /admin/reauth`                       → [`do_reauth`]
//! - `GET  /admin/must-change-password`         → [`show_must_change_password`]
//! - `POST /admin/must-change-password`         → [`do_must_change_password`]
//! - `GET  /admin/users/:id/reset-password`     → [`show_admin_reset_password`]
//! - `POST /admin/users/:id/reset-password`     → [`do_admin_reset_password`]
//! - `GET  /admin/users/:id/lock`               → [`show_lock_user`]
//! - `POST /admin/users/:id/lock`               → [`do_lock_user`]
//! - `GET  /admin/users/:id/unlock`             → [`show_unlock_user`]
//! - `POST /admin/users/:id/unlock`             → [`do_unlock_user`]
//! - `GET  /admin/users/:id/revoke-sessions`    → [`show_admin_revoke_sessions`]
//! - `POST /admin/users/:id/revoke-sessions`    → [`do_admin_revoke_sessions`]
//!
//! ## What this module does NOT do
//!
//! - Audit rows on the re-auth wall. Re-auth itself does not write
//!   to `rustio_admin_actions` — the audit chain begins at the
//!   destructive action. A failed re-auth attempt is silently
//!   re-rendered with a uniform error, no log row.
//! - Session revocation in the re-auth wall. Doctrine 22 untouched;
//!   promotion only extends the elevation window via
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
use sqlx::Row as SqlxRow;

use crate::auth::guards::enforce_cross_rank_safe;
use crate::auth::recovery_admin::{
    admin_revoke_sessions, admin_set_temp_password, check_session_elevated,
    issue_admin_reset_token, lock_user_account, unlock_user_account, AdminActor, AdminIssueOutcome,
    AdminRevokeOutcome, AdminTempPwOutcome, LockDuration, LockOutcome, UnlockOutcome,
};
use crate::auth::{self, Identity, Role};
use crate::error::{Error, Result};
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
    /// R3: whether to render the second-factor input. When TRUE,
    /// the template adds a `code` field accepting a 6-digit TOTP
    /// or `XXXX-XXXX` backup code. The POST handler verifies
    /// both factors before stamping `trust_level = 'mfa_verified'`.
    mfa_enabled: bool,
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
        mfa_enabled: identity.mfa_enabled,
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
pub(crate) async fn do_reauth(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let password = form.get("password").unwrap_or("").to_string();
    let code = form.get("code").unwrap_or("").trim().to_string();
    let raw_return_to = form.get("return_to").map(|s| s.to_string());
    let return_to = redirect_after_reauth(raw_return_to.as_deref());

    // Single uniform failure renderer used by every error branch
    // below. Same status, same wording, same CSRF refresh — no
    // distinction between "user gone", "wrong password", "wrong
    // TOTP", "replay", or "no current session" reaches the client.
    // Per DESIGN_R3_MFA.md §3.1, the disclosure rule applies to
    // every re-auth failure mode.
    let uniform_failure = |ctx: &AdminCtx, return_to: &str| -> Result<Response> {
        let view = ReauthCtx {
            base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
            page_title: "Confirm your identity",
            email: identity.email.clone(),
            return_to: return_to.to_string(),
            error: Some("Could not verify your password.".to_string()),
            mfa_enabled: identity.mfa_enabled,
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

    // R3 — second-factor verification when MFA is enrolled. The
    // re-auth wall demands BOTH factors for an MFA-enrolled user;
    // a stolen cookie + a password (e.g. via phishing) cannot
    // mutate authority without the second factor. Per
    // DESIGN_R3_MFA.md §12.2.
    //
    // Order: try TOTP first, fall back to backup-code on
    // VerifyOutcome::Invalid. Replay / NotEnrolled collapse to
    // uniform failure WITHOUT the backup-code attempt — a
    // replayed TOTP is a security signal we should not paper
    // over with a backup-code retry.
    if identity.mfa_enabled {
        if code.is_empty() {
            return uniform_failure(ctx, &return_to);
        }
        use crate::auth::mfa::{
            consume_backup_code, verify_totp_for_user, BackupConsumeOutcome, MfaKey, VerifyOutcome,
        };
        let policy = ctx.admin.active_recovery_policy();
        let step_seconds = policy.mfa_step_seconds();
        let skew_steps = policy.mfa_skew_steps();
        let key = MfaKey::from_env()?;
        let correlation_id = req.header("x-correlation-id");

        let totp_outcome =
            verify_totp_for_user(&ctx.db, user.id, &code, step_seconds, skew_steps, &key).await?;
        let totp_verified = matches!(totp_outcome, VerifyOutcome::Verified { .. });

        let second_factor_ok = if totp_verified {
            true
        } else {
            match totp_outcome {
                VerifyOutcome::Invalid => {
                    let backup_outcome = consume_backup_code(
                        &ctx.db,
                        &req,
                        user.id,
                        &code,
                        "reauth",
                        correlation_id,
                    )
                    .await?;
                    matches!(backup_outcome, BackupConsumeOutcome::Consumed { .. })
                }
                VerifyOutcome::Replay { .. }
                | VerifyOutcome::NotEnrolled
                | VerifyOutcome::Verified { .. } => false,
            }
        };

        if !second_factor_ok {
            return uniform_failure(ctx, &return_to);
        }
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

    // R3: pick the promote variant by the actor's mfa_enabled
    // state. MFA-enrolled users land on mfa_verified (preserves
    // any higher pre-existing trust); non-enrolled users land on
    // the R2 'elevated' band.
    if identity.mfa_enabled {
        crate::auth::mfa::promote_session_mfa_elevated(&ctx.db, session_id, ttl).await?;
    } else {
        crate::auth::recovery_admin::promote_session_elevated(&ctx.db, session_id, ttl).await?;
    }

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
            model_name: "users",
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

// ---- Admin-driven password reset (R2 commit #15) ---------------------------
//
// Routes (registered in commit #17, gated `Role::Administrator` +
// cross-rank-safe + re-auth):
//   GET  /admin/users/:id/reset-password → show_admin_reset_password
//   POST /admin/users/:id/reset-password → do_admin_reset_password
//
// The runtime fns in `auth::recovery_admin` (commit #14) do the
// heavy lifting; this layer wraps them in HTTP, enforces the
// re-auth wall, validates the form, and renders outcomes.
//
// Audit emissions all live in commit #14's runtime — the headline
// `PasswordResetByOther` row plus per-revoked-session
// `SessionsRevokedByOther` rows for temp_pw mode. This handler
// adds no new audit rows.

#[derive(Serialize)]
struct AdminResetPasswordCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    target_user_id: i64,
    target_email: String,
    /// `true` when a temp_pw / email-mode operation just succeeded;
    /// the template switches into the success-card layout. The
    /// runtime split is preserved (form vs. success rendered from
    /// the same template) because the temp_pw success page has no
    /// distinct URL — refresh on a 303 destination would lose the
    /// plaintext, so the POST handler renders directly.
    success: bool,
    /// Form-level error banner for validation failures + the
    /// `UnknownTarget` / `InactiveTarget` runtime outcomes.
    errors: Vec<String>,
    /// Field-keyed errors (currently `reason`).
    field_errors: HashMap<String, Vec<String>>,
    /// Carries the previously-submitted reason so re-renders
    /// preserve admin input on validation failure.
    reason: String,
    /// Mode the admin had selected on the failing submission;
    /// the radio buttons render `checked` based on this. Defaults
    /// to `"email"` on first GET.
    mode: String,
    // Success-state fields (populated only when `success == true`).
    /// `"email"` or `"temp_pw"` — the mode that succeeded.
    success_mode: String,
    /// For temp_pw success: the plaintext temp password to render
    /// once. Empty string in every other state. The plaintext
    /// EXISTS only in this template render — never logged, never
    /// audited, never persisted in any other column. Refreshing
    /// the page re-POSTs and re-issues a NEW temp password
    /// (rotating the previous one out); the template warns
    /// against refresh.
    temp_password: String,
    /// For email success: `"sent"` or `"failed"`. Empty otherwise.
    email_status: String,
    /// For temp_pw success: how many sibling sessions were
    /// revoked. 0 in every other state.
    revoked_session_count: usize,
}

impl AdminResetPasswordCtx {
    fn form(
        base: BaseContext,
        target_user_id: i64,
        target_email: String,
        reason: String,
        mode: String,
        errors: Vec<String>,
        field_errors: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            base,
            page_title: format!("Reset password — {target_email}"),
            target_user_id,
            target_email,
            success: false,
            errors,
            field_errors,
            reason,
            mode,
            success_mode: String::new(),
            temp_password: String::new(),
            email_status: String::new(),
            revoked_session_count: 0,
        }
    }
}

/// Look up the target user. Returns `(email, is_active, role)` or
/// `None` for missing. Sibling of `recovery_admin::load_admin_reset_target`
/// extended with `role` because the cross-rank guard needs it; the
/// handler's pre-validation does its own DB hit so the runtime fn
/// can stay role-free.
async fn load_target(
    db: &crate::orm::Db,
    target_user_id: i64,
) -> Result<Option<(String, bool, Role)>> {
    let row = sqlx::query("SELECT email, is_active, role FROM rustio_users WHERE id = $1")
        .bind(target_user_id)
        .fetch_optional(db.pool())
        .await?;
    match row {
        Some(r) => {
            let email: String = r.try_get("email")?;
            let is_active: bool = r.try_get("is_active")?;
            let role_str: String = r.try_get("role")?;
            let role = Role::parse(&role_str)?;
            Ok(Some((email, is_active, role)))
        }
        None => Ok(None),
    }
}

/// Compute the re-auth redirect URL for `/admin/users/:id/reset-password`.
/// Bounce to `/admin/reauth?return_to=<encoded path>` so the user re-auths
/// and lands back here after promotion.
fn reauth_redirect_for(target_user_id: i64) -> Response {
    let path = format!("/admin/users/{target_user_id}/reset-password");
    Response::redirect(format!(
        "/admin/reauth?return_to={}",
        urlencoding::encode(&path)
    ))
}

/// Resolve the actor's session id from the request's cookie. Returns
/// None on any of: missing cookie header, malformed token, no current
/// session row. Caller treats None as "not elevated".
async fn current_session_id_for(ctx: &AdminCtx, req: &Request) -> Result<Option<i64>> {
    let cookie = match req.header("cookie") {
        Some(c) => c,
        None => return Ok(None),
    };
    let token = match auth::session_token_from_cookie(cookie) {
        Some(t) => t,
        None => return Ok(None),
    };
    auth::current_session_id(&ctx.db, &token).await
}

/// `GET /admin/users/:id/reset-password` — render the reset form.
///
/// Order of checks:
/// 1. Target user exists (404 if not).
/// 2. Cross-rank: actor's role must dominate target's
///    (`Error::Forbidden` propagates → 403 page).
/// 3. Re-auth wall: actor's session must be elevated. If not, 303
///    redirect to `/admin/reauth?return_to=<this path>`.
/// 4. Render the form with `mode='email'` selected by default and
///    an empty reason field.
pub(crate) async fn show_admin_reset_password(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: &Request,
) -> Result<Response> {
    let (target_email, _is_active, target_role) = match load_target(&ctx.db, target_user_id).await?
    {
        Some(t) => t,
        None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
    };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    let session_id = current_session_id_for(ctx, req).await?;
    let elevated = match session_id {
        Some(id) => check_session_elevated(&ctx.db, id).await?,
        None => false,
    };
    if !elevated {
        return Ok(reauth_redirect_for(target_user_id));
    }

    let view = AdminResetPasswordCtx::form(
        BaseContext::new(Some(&actor_identity), csrf_token(req), &ctx.admin),
        target_user_id,
        target_email,
        String::new(),
        "email".to_string(),
        Vec::new(),
        HashMap::new(),
    );
    let body = ctx
        .templates
        .render("admin/admin_reset_password.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/users/:id/reset-password` — apply the reset.
///
/// Same authority + re-auth checks as the GET. On a successful
/// dispatch:
///
/// - **email mode** → 303 redirect to `/admin/users` (PRG; the
///   user list shows the target row and the admin can verify
///   the reset). Audit row already emitted by the runtime; the
///   admin's success signal is the redirect destination
///   re-rendering normally, plus the audit log entry an operator
///   can confirm in `/admin/history`.
/// - **temp_pw mode** → 200 render of the success card with the
///   plaintext temp password. NOT a redirect: the temp must
///   survive the response, and a 303 destination GET cannot
///   carry plaintext. Refreshing the success page re-POSTs and
///   re-issues a NEW temp password — the template warns the
///   admin not to refresh.
///
/// Failures collapse to a re-render of the form with field /
/// banner errors. The runtime's `UnknownTarget` and
/// `InactiveTarget` outcomes both surface as a form-level banner.
/// Cross-rank failures bubble through `Error::Forbidden` and
/// render the framework's 403 page.
pub(crate) async fn do_admin_reset_password(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: Request,
) -> Result<Response> {
    let (target_email, target_is_active, target_role) =
        match load_target(&ctx.db, target_user_id).await? {
            Some(t) => t,
            None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
        };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    let session_id = current_session_id_for(ctx, &req).await?;
    let elevated = match session_id {
        Some(id) => check_session_elevated(&ctx.db, id).await?,
        None => false,
    };
    if !elevated {
        return Ok(reauth_redirect_for(target_user_id));
    }

    let form = req.form()?;
    let mode = form.get("mode").unwrap_or("email").trim().to_string();
    let reason = form.get("reason").unwrap_or("").trim().to_string();

    // Validate. Reason floor 8 chars per §12 locked-decision.
    let mut errors: Vec<String> = Vec::new();
    let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

    if reason.chars().count() < 8 {
        let msg = "Reason must be at least 8 characters.".to_string();
        errors.push(msg.clone());
        field_errors.entry("reason".into()).or_default().push(msg);
    }
    if mode != "email" && mode != "temp_pw" {
        errors.push(format!("Unknown mode: {mode:?}."));
    }
    if !target_is_active {
        errors.push(format!(
            "{target_email} is currently disabled. Re-enable the account before resetting."
        ));
    }

    let render_form = |ctx: &AdminCtx,
                       errors: Vec<String>,
                       field_errors: HashMap<String, Vec<String>>|
     -> Result<Response> {
        let view = AdminResetPasswordCtx::form(
            BaseContext::new(Some(&actor_identity), csrf_token(&req), &ctx.admin),
            target_user_id,
            target_email.clone(),
            reason.clone(),
            mode.clone(),
            errors,
            field_errors,
        );
        let body = ctx
            .templates
            .render("admin/admin_reset_password.html", &view)?;
        Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST))
    };

    if !errors.is_empty() {
        return render_form(ctx, errors, field_errors);
    }

    // Dispatch to the runtime. The actor's identity carries email +
    // user_id; bundle into AdminActor for the runtime call.
    let actor = AdminActor {
        user_id: actor_identity.user_id,
        email: &actor_identity.email,
    };
    let cid = correlation_id_from(&req);

    match mode.as_str() {
        "email" => {
            let outcome = issue_admin_reset_token(
                &ctx.db,
                &ctx.admin,
                &req,
                target_user_id,
                actor,
                &reason,
                cid.as_deref(),
            )
            .await?;
            match outcome {
                AdminIssueOutcome::Issued { .. } => {
                    // PRG: 303 to the user list. Audit row already
                    // written by the runtime; the admin can pivot
                    // via /admin/history.
                    Ok(Response::redirect("/admin/users"))
                }
                AdminIssueOutcome::UnknownTarget => {
                    Err(Error::NotFound(format!("user #{target_user_id}")))
                }
                AdminIssueOutcome::InactiveTarget => render_form(
                    ctx,
                    vec![format!(
                        "{target_email} is currently disabled. Re-enable before resetting."
                    )],
                    HashMap::new(),
                ),
            }
        }
        "temp_pw" => {
            let outcome = admin_set_temp_password(
                &ctx.db,
                &req,
                target_user_id,
                actor,
                &reason,
                cid.as_deref(),
            )
            .await?;
            match outcome {
                AdminTempPwOutcome::Set {
                    temp_password,
                    revoked_session_count,
                    ..
                } => {
                    // 200 render with the plaintext temp password.
                    // PRG-violation accepted — the temp value cannot
                    // survive a 303 → GET redirect. Template warns
                    // against refresh.
                    let view = AdminResetPasswordCtx {
                        base: BaseContext::new(Some(&actor_identity), csrf_token(&req), &ctx.admin),
                        page_title: format!("Password reset for {target_email}"),
                        target_user_id,
                        target_email: target_email.clone(),
                        success: true,
                        errors: Vec::new(),
                        field_errors: HashMap::new(),
                        reason: String::new(),
                        mode: String::new(),
                        success_mode: "temp_pw".to_string(),
                        temp_password,
                        email_status: String::new(),
                        revoked_session_count,
                    };
                    let body = ctx
                        .templates
                        .render("admin/admin_reset_password.html", &view)?;
                    Ok(Response::html(body))
                }
                AdminTempPwOutcome::UnknownTarget => {
                    Err(Error::NotFound(format!("user #{target_user_id}")))
                }
                AdminTempPwOutcome::InactiveTarget => render_form(
                    ctx,
                    vec![format!(
                        "{target_email} is currently disabled. Re-enable before resetting."
                    )],
                    HashMap::new(),
                ),
            }
        }
        _ => unreachable!("mode validated above"),
    }
}

// ---- Lock / unlock / revoke (R2 commit #16) --------------------------------
//
// Three sibling action surfaces, all gated by Role::Administrator +
// cross-rank-safe + re-auth (registered in commit #17). Each pairs
// a GET that renders a confirmation form with a POST that performs
// the action via the runtime fns from §3.2.
//
// Reason floor: 8 chars (§12 locked).

#[derive(Serialize)]
struct LockUserCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    target_user_id: i64,
    target_email: String,
    errors: Vec<String>,
    field_errors: HashMap<String, Vec<String>>,
    /// Carries the previously-submitted reason on re-render.
    reason: String,
    /// Selected duration radio value (`"15min"`, `"1h"`, `"24h"`,
    /// `"7d"`, `"indefinite"`, or `"freeform"`); defaults to `"1h"`
    /// on first GET.
    duration: String,
    /// Freeform-minutes input value (preserved on re-render).
    freeform_minutes: String,
}

#[derive(Serialize)]
struct ConfirmActionCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: String,
    target_user_id: i64,
    target_email: String,
    errors: Vec<String>,
    field_errors: HashMap<String, Vec<String>>,
    reason: String,
    /// Title displayed at the top of the confirmation card.
    action_title: &'static str,
    /// One-sentence description shown above the reason field.
    action_description: &'static str,
    /// Submit-button label.
    submit_label: &'static str,
    /// Form action path (e.g. `/admin/users/42/unlock`).
    form_action: String,
}

/// Pure helper: parse the lock-form's duration + freeform_minutes
/// inputs into a [`LockDuration`]. Returns the typed value or a
/// user-facing error message.
fn parse_lock_duration(
    duration: &str,
    freeform_minutes: &str,
) -> std::result::Result<LockDuration, String> {
    match duration {
        "15min" => Ok(LockDuration::FifteenMinutes),
        "1h" => Ok(LockDuration::OneHour),
        "24h" => Ok(LockDuration::TwentyFourHours),
        "7d" => Ok(LockDuration::SevenDays),
        "indefinite" => Ok(LockDuration::Indefinite),
        "freeform" => match freeform_minutes.trim().parse::<u32>() {
            Ok(m) if m >= 1 => Ok(LockDuration::Minutes(m)),
            Ok(_) => Err("Freeform duration must be at least 1 minute.".to_string()),
            Err(_) => Err("Enter a whole number of minutes.".to_string()),
        },
        _ => Err(format!("Unknown duration: {duration:?}.")),
    }
}

/// `GET /admin/users/:id/lock` — render the lock confirmation form.
pub(crate) async fn show_lock_user(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: &Request,
) -> Result<Response> {
    let (target_email, _is_active, target_role) = match load_target(&ctx.db, target_user_id).await?
    {
        Some(t) => t,
        None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
    };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    let session_id = current_session_id_for(ctx, req).await?;
    if !matches!(session_id, Some(id) if check_session_elevated(&ctx.db, id).await?) {
        return Ok(reauth_redirect_for_path(&format!(
            "/admin/users/{target_user_id}/lock"
        )));
    }

    let view = LockUserCtx {
        base: BaseContext::new(Some(&actor_identity), csrf_token(req), &ctx.admin),
        page_title: format!("Lock account — {target_email}"),
        target_user_id,
        target_email,
        errors: Vec::new(),
        field_errors: HashMap::new(),
        reason: String::new(),
        duration: "1h".to_string(),
        freeform_minutes: String::new(),
    };
    let body = ctx.templates.render("admin/lock_user.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/users/:id/lock` — apply the lock.
pub(crate) async fn do_lock_user(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: Request,
) -> Result<Response> {
    let (target_email, _is_active, target_role) = match load_target(&ctx.db, target_user_id).await?
    {
        Some(t) => t,
        None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
    };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    let session_id = current_session_id_for(ctx, &req).await?;
    if !matches!(session_id, Some(id) if check_session_elevated(&ctx.db, id).await?) {
        return Ok(reauth_redirect_for_path(&format!(
            "/admin/users/{target_user_id}/lock"
        )));
    }

    let form = req.form()?;
    let duration_raw = form.get("duration").unwrap_or("1h").trim().to_string();
    let freeform_minutes = form
        .get("freeform_minutes")
        .unwrap_or("")
        .trim()
        .to_string();
    let reason = form.get("reason").unwrap_or("").trim().to_string();

    let mut errors: Vec<String> = Vec::new();
    let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

    if reason.chars().count() < 8 {
        let msg = "Reason must be at least 8 characters.".to_string();
        errors.push(msg.clone());
        field_errors.entry("reason".into()).or_default().push(msg);
    }

    let parsed_duration = match parse_lock_duration(&duration_raw, &freeform_minutes) {
        Ok(d) => Some(d),
        Err(msg) => {
            errors.push(msg.clone());
            field_errors.entry("duration".into()).or_default().push(msg);
            None
        }
    };

    if !errors.is_empty() {
        let view = LockUserCtx {
            base: BaseContext::new(Some(&actor_identity), csrf_token(&req), &ctx.admin),
            page_title: format!("Lock account — {target_email}"),
            target_user_id,
            target_email,
            errors,
            field_errors,
            reason,
            duration: duration_raw,
            freeform_minutes,
        };
        let body = ctx.templates.render("admin/lock_user.html", &view)?;
        return Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST));
    }

    let actor = AdminActor {
        user_id: actor_identity.user_id,
        email: &actor_identity.email,
    };
    let cid = correlation_id_from(&req);

    let outcome = lock_user_account(
        &ctx.db,
        &req,
        target_user_id,
        actor,
        parsed_duration.expect("validated above"),
        &reason,
        cid.as_deref(),
    )
    .await?;
    match outcome {
        LockOutcome::Locked { .. } => Ok(Response::redirect("/admin/users")),
        LockOutcome::UnknownTarget => Err(Error::NotFound(format!("user #{target_user_id}"))),
    }
}

/// `GET /admin/users/:id/unlock` — render the unlock confirmation form.
pub(crate) async fn show_unlock_user(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: &Request,
) -> Result<Response> {
    let (target_email, _is_active, target_role) = match load_target(&ctx.db, target_user_id).await?
    {
        Some(t) => t,
        None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
    };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    let session_id = current_session_id_for(ctx, req).await?;
    if !matches!(session_id, Some(id) if check_session_elevated(&ctx.db, id).await?) {
        return Ok(reauth_redirect_for_path(&format!(
            "/admin/users/{target_user_id}/unlock"
        )));
    }

    let view = ConfirmActionCtx {
        base: BaseContext::new(Some(&actor_identity), csrf_token(req), &ctx.admin),
        page_title: format!("Unlock account — {target_email}"),
        target_user_id,
        target_email,
        errors: Vec::new(),
        field_errors: HashMap::new(),
        reason: String::new(),
        action_title: "Unlock account",
        action_description: "Clear the account lock and reset the failed-login counter. The user can sign in immediately.",
        submit_label: "Unlock account",
        form_action: format!("/admin/users/{target_user_id}/unlock"),
    };
    let body = ctx
        .templates
        .render("admin/confirm_admin_action.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/users/:id/unlock` — clear the lock.
pub(crate) async fn do_unlock_user(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: Request,
) -> Result<Response> {
    do_confirm_action(ctx, actor_identity, target_user_id, req, ActionKind::Unlock).await
}

/// `GET /admin/users/:id/revoke-sessions` — render the revoke confirmation form.
pub(crate) async fn show_admin_revoke_sessions(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: &Request,
) -> Result<Response> {
    let (target_email, _is_active, target_role) = match load_target(&ctx.db, target_user_id).await?
    {
        Some(t) => t,
        None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
    };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    let session_id = current_session_id_for(ctx, req).await?;
    if !matches!(session_id, Some(id) if check_session_elevated(&ctx.db, id).await?) {
        return Ok(reauth_redirect_for_path(&format!(
            "/admin/users/{target_user_id}/revoke-sessions"
        )));
    }

    let view = ConfirmActionCtx {
        base: BaseContext::new(Some(&actor_identity), csrf_token(req), &ctx.admin),
        page_title: format!("Revoke sessions — {target_email}"),
        target_user_id,
        target_email,
        errors: Vec::new(),
        field_errors: HashMap::new(),
        reason: String::new(),
        action_title: "Revoke all sessions",
        action_description: "Sign the user out of every device immediately. No lock is applied; they can sign in again on a fresh session.",
        submit_label: "Revoke all sessions",
        form_action: format!("/admin/users/{target_user_id}/revoke-sessions"),
    };
    let body = ctx
        .templates
        .render("admin/confirm_admin_action.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/users/:id/revoke-sessions` — revoke all sessions.
pub(crate) async fn do_admin_revoke_sessions(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: Request,
) -> Result<Response> {
    do_confirm_action(ctx, actor_identity, target_user_id, req, ActionKind::Revoke).await
}

/// `POST /admin/users/:target_user_id/sessions/:session_id/revoke` —
/// revoke ONE specific active session of `target_user_id`. Backs
/// the per-row "Revoke" button on the user_view sessions tab.
///
/// Distinguished from [`do_admin_revoke_sessions`] (the bulk
/// counterpart, which sits behind the user-view "Revoke all
/// sessions" admin action): no reason field, no re-auth wall, no
/// confirmation step. The narrower scope of "revoke one suspect
/// device" doesn't justify the friction of the bulk surface, and
/// admins already have the bulk button next to it for the heavier
/// case.
///
/// Order of checks:
/// 1. Target user exists (404 if not).
/// 2. Cross-rank: actor's role must dominate target's (same guard
///    every admin-user action shares).
/// 3. Session ownership: the session row exists, is active
///    (`revoked_at IS NULL AND expires_at > NOW()`), and belongs to
///    `target_user_id`. Mismatch / gone → 404.
/// 4. Defense-in-depth: refuse if the session is the actor's
///    *own* current session. The user_view template hides the
///    button on that row; this is the security boundary.
/// 5. Invalidate via the centralized API; emit one
///    `SessionsRevokedByOther` audit row carrying the actor /
///    target / correlation_id metadata.
/// 6. Redirect back to the sessions tab.
pub(crate) async fn do_admin_revoke_one_session(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    session_id: i64,
    req: Request,
) -> Result<Response> {
    let (target_email, _is_active, target_role) = match load_target(&ctx.db, target_user_id).await?
    {
        Some(t) => t,
        None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
    };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    // Ownership + liveness check in a single query. A mismatched
    // user_id, a revoked row, or an expired row all collapse to the
    // same uniform 404 — no information leak about whether the
    // session existed at all.
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM rustio_sessions
         WHERE session_id = $1 AND revoked_at IS NULL AND expires_at > NOW()",
    )
    .bind(session_id)
    .fetch_optional(ctx.db.pool())
    .await
    .map_err(|e| Error::Internal(format!("session ownership lookup: {e}")))?;
    match owner {
        Some(uid) if uid == target_user_id => {}
        _ => {
            return Err(Error::NotFound(format!(
                "session #{session_id} for user #{target_user_id}"
            )))
        }
    }

    // The viewing admin's own session is excluded from this surface
    // — the template hides the button on that row, and this check
    // prevents URL-crafting around the template. Admins sign
    // themselves out via `/admin/account/sessions`.
    if let Some(actor_session_id) = current_session_id_for(ctx, &req).await? {
        if actor_session_id == session_id {
            return Err(Error::BadRequest(
                "You can't revoke your own session from another user's profile. \
                 Use Account → Sessions to manage your own devices."
                    .into(),
            ));
        }
    }

    let outcome = auth::invalidate_sessions(
        &ctx.db,
        auth::SessionTarget::Single { session_id },
        auth::SessionInvalidationReason::AdministrativeRevoke,
    )
    .await?;

    let ip = client_ip(&req);
    let cid = correlation_id_from(&req);
    for revoked_id in &outcome.revoked_session_ids {
        let metadata = serde_json::json!({
            "session_id": revoked_id,
            "reason": "administrative_revoke",
            "via": "user_view_per_session",
            "target_user_email": target_email,
        });
        let mut entry = audit::LogEntry::new(
            target_user_id,
            audit::ActionType::Update,
            "user",
            target_user_id,
        )
        .with_event(audit::AuditEvent::SessionsRevokedByOther);
        entry.actor_user_id = Some(actor_identity.user_id);
        entry.correlation_id = cid.as_deref();
        entry.ip_address = ip.as_deref();
        entry.metadata = Some(metadata);
        entry.summary = format!(
            "session {revoked_id} revoked by {} for {target_email}",
            actor_identity.email,
        );
        if let Err(e) = audit::record(&ctx.db, entry).await {
            log::error!(
                target: "rustio_admin::sessions::admin_revoke_one",
                "audit::record failed for session_id={revoked_id} target_user_id={target_user_id}: {e}",
            );
        }
    }

    Ok(Response::redirect(format!(
        "/admin/users/{target_user_id}?tab=sessions"
    )))
}

/// Discriminator for the shared confirm-action POST handler. Lock
/// has its own POST (`do_lock_user`) because of the duration field.
#[derive(Debug, Clone, Copy)]
enum ActionKind {
    Unlock,
    Revoke,
}

async fn do_confirm_action(
    ctx: &AdminCtx,
    actor_identity: Identity,
    target_user_id: i64,
    req: Request,
    kind: ActionKind,
) -> Result<Response> {
    let (target_email, _is_active, target_role) = match load_target(&ctx.db, target_user_id).await?
    {
        Some(t) => t,
        None => return Err(Error::NotFound(format!("user #{target_user_id}"))),
    };
    enforce_cross_rank_safe(&actor_identity, target_user_id, target_role)?;

    let path_suffix = match kind {
        ActionKind::Unlock => "unlock",
        ActionKind::Revoke => "revoke-sessions",
    };
    let session_id = current_session_id_for(ctx, &req).await?;
    if !matches!(session_id, Some(id) if check_session_elevated(&ctx.db, id).await?) {
        return Ok(reauth_redirect_for_path(&format!(
            "/admin/users/{target_user_id}/{path_suffix}"
        )));
    }

    let form = req.form()?;
    let reason = form.get("reason").unwrap_or("").trim().to_string();

    if reason.chars().count() < 8 {
        let (action_title, action_description, submit_label) = match kind {
            ActionKind::Unlock => (
                "Unlock account",
                "Clear the account lock and reset the failed-login counter. The user can sign in immediately.",
                "Unlock account",
            ),
            ActionKind::Revoke => (
                "Revoke all sessions",
                "Sign the user out of every device immediately. No lock is applied; they can sign in again on a fresh session.",
                "Revoke all sessions",
            ),
        };
        let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();
        let msg = "Reason must be at least 8 characters.".to_string();
        field_errors
            .entry("reason".into())
            .or_default()
            .push(msg.clone());
        let view = ConfirmActionCtx {
            base: BaseContext::new(Some(&actor_identity), csrf_token(&req), &ctx.admin),
            page_title: format!("{action_title} — {target_email}"),
            target_user_id,
            target_email,
            errors: vec![msg],
            field_errors,
            reason,
            action_title,
            action_description,
            submit_label,
            form_action: format!("/admin/users/{target_user_id}/{path_suffix}"),
        };
        let body = ctx
            .templates
            .render("admin/confirm_admin_action.html", &view)?;
        return Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST));
    }

    let actor = AdminActor {
        user_id: actor_identity.user_id,
        email: &actor_identity.email,
    };
    let cid = correlation_id_from(&req);

    match kind {
        ActionKind::Unlock => {
            let outcome = unlock_user_account(
                &ctx.db,
                &req,
                target_user_id,
                actor,
                &reason,
                cid.as_deref(),
            )
            .await?;
            match outcome {
                UnlockOutcome::Unlocked { .. } => Ok(Response::redirect("/admin/users")),
                UnlockOutcome::UnknownTarget => {
                    Err(Error::NotFound(format!("user #{target_user_id}")))
                }
            }
        }
        ActionKind::Revoke => {
            let outcome = admin_revoke_sessions(
                &ctx.db,
                &req,
                target_user_id,
                actor,
                &reason,
                cid.as_deref(),
            )
            .await?;
            match outcome {
                AdminRevokeOutcome::Revoked { .. } => Ok(Response::redirect("/admin/users")),
                AdminRevokeOutcome::UnknownTarget => {
                    Err(Error::NotFound(format!("user #{target_user_id}")))
                }
            }
        }
    }
}

/// Path-keyed sibling of [`reauth_redirect_for`]. Wraps an arbitrary
/// `/admin/...` path into the re-auth bounce URL with proper
/// percent-encoding.
fn reauth_redirect_for_path(path: &str) -> Response {
    Response::redirect(format!(
        "/admin/reauth?return_to={}",
        urlencoding::encode(path)
    ))
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

    // ---- parse_lock_duration (R2 commit #16) ------------------------------

    #[test]
    fn parse_lock_duration_accepts_locked_presets() {
        assert!(matches!(
            parse_lock_duration("15min", ""),
            Ok(LockDuration::FifteenMinutes)
        ));
        assert!(matches!(
            parse_lock_duration("1h", ""),
            Ok(LockDuration::OneHour)
        ));
        assert!(matches!(
            parse_lock_duration("24h", ""),
            Ok(LockDuration::TwentyFourHours)
        ));
        assert!(matches!(
            parse_lock_duration("7d", ""),
            Ok(LockDuration::SevenDays)
        ));
        assert!(matches!(
            parse_lock_duration("indefinite", ""),
            Ok(LockDuration::Indefinite)
        ));
    }

    #[test]
    fn parse_lock_duration_freeform_accepts_positive_minutes() {
        assert!(matches!(
            parse_lock_duration("freeform", "30"),
            Ok(LockDuration::Minutes(30))
        ));
        assert!(matches!(
            parse_lock_duration("freeform", "  120  "),
            Ok(LockDuration::Minutes(120))
        ));
    }

    #[test]
    fn parse_lock_duration_freeform_rejects_zero_negative_garbage() {
        assert!(parse_lock_duration("freeform", "0").is_err());
        assert!(parse_lock_duration("freeform", "").is_err());
        assert!(parse_lock_duration("freeform", "abc").is_err());
        assert!(parse_lock_duration("freeform", "-1").is_err());
        assert!(parse_lock_duration("freeform", "1.5").is_err());
    }

    #[test]
    fn parse_lock_duration_rejects_unknown_preset() {
        assert!(parse_lock_duration("forever", "").is_err());
        assert!(parse_lock_duration("", "").is_err());
        assert!(parse_lock_duration("1H", "").is_err()); // case-sensitive
    }

    #[test]
    fn parse_lock_duration_freeform_overflow_rejected() {
        // u32::MAX + 1 doesn't parse as u32; the helper bubbles the
        // error to a user-facing string.
        let very_big = "4294967296"; // 2^32
        assert!(parse_lock_duration("freeform", very_big).is_err());
    }
}
