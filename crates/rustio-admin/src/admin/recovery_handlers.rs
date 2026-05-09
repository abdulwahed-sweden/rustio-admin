//! HTTP handlers for the unauthenticated password-recovery surface
//! (R1 commit #8). See `DESIGN_RECOVERY.md` §3 + §8.
//!
//! Routes wired in commit #9; this module owns:
//!
//! - `GET  /admin/forgot-password`            → [`show_forgot_password`]
//! - `POST /admin/forgot-password`            → [`do_forgot_password`]
//! - `GET  /admin/forgot-password/sent`       → [`show_forgot_password_sent`]
//! - `GET  /admin/reset-password/<token>`     → [`show_reset_password`]
//! - `POST /admin/reset-password/<token>`     → [`do_reset_password`]
//!
//! ## Uniform-response invariant (LOCKED — `DESIGN_RECOVERY.md` §2.3)
//!
//! Outward-facing pages MUST NOT reveal:
//!
//! - whether an email is registered (`do_forgot_password` redirects
//!   to `/admin/forgot-password/sent` regardless of the underlying
//!   [`IssueOutcome`] — known, unknown, inactive, rate-limited all
//!   collapse to one response);
//! - whether a reset token is malformed, expired, or already
//!   consumed (`show_reset_password` and `do_reset_password` render
//!   the same "this link is no longer valid" card across all three
//!   sub-cases);
//! - whether the consume path was rate-limited (renders identically
//!   to "invalid", per the disclosure rule).
//!
//! `IssueOutcome` and `ConsumeOutcome` distinctions exist only for
//! audit/log/observability; the handler maps them to indistinguish-
//! able pages.
//!
//! ## What this module does NOT do (commits that follow)
//!
//! - **Routes** are registered in commit #9.
//! - **Login-page touch-ups** ("Forgot your password?" link,
//!   `?password_reset=success` banner) are commit #9.
//! - **Active-sessions revoke buttons** are commit #10.
//! - **`do_password_change` drift correction** is commit #11.
//! - **Token sweeper** is commit #12.

use std::sync::Arc;

use serde::Serialize;

use crate::auth::recovery::{
    check_reset_token_valid, consume_reset_token, issue_reset_token, ConsumeOutcome,
    PasswordPolicyError,
};
use crate::error::Result;
use crate::http::{Request, Response};
use crate::middleware::{CorrelationId, RateLimiter};

use super::handlers::{csrf_token, AdminCtx};
use super::render::{BaseContext, FlashCtx, FormField, FormSection};
use super::types::Admin;

// ---- RecoveryState --------------------------------------------------------

/// Per-process recovery state. The two rate-limit buckets live here
/// because they're stateful (token-bucket counters per IP) and the
/// runtime functions need them on every request.
///
/// Built once at route-registration time (commit #9) via
/// [`RecoveryState::from_admin`]; the `Arc<RateLimiter>` fields are
/// then cloned into each route closure for the process lifetime.
/// No global / static / `OnceLock` state — the `Arc<RecoveryState>`
/// is owned by the route handler closures.
#[derive(Clone)]
pub(crate) struct RecoveryState {
    pub request_limiter: Arc<RateLimiter>,
    pub consume_limiter: Arc<RateLimiter>,
}

impl RecoveryState {
    /// Build from `Admin`, sizing the buckets from
    /// `RecoveryPolicy::request_rate_limit()` and
    /// `consume_rate_limit()`. Called once at registration time.
    pub fn from_admin(admin: &Admin) -> Self {
        let policy = admin.active_recovery_policy();
        let (req_cap, req_window) = policy.request_rate_limit();
        let (cons_cap, cons_window) = policy.consume_rate_limit();
        Self {
            request_limiter: Arc::new(RateLimiter::new(req_cap, req_window)),
            consume_limiter: Arc::new(RateLimiter::new(cons_cap, cons_window)),
        }
    }
}

// ---- Helpers --------------------------------------------------------------

fn correlation_id(req: &Request) -> Option<String> {
    req.ctx().get::<CorrelationId>().map(|c| c.0.clone())
}

// ---- Render contexts ------------------------------------------------------

/// Context for `admin/forgot_password.html`. Identity is always
/// `None` (unauthenticated flow).
#[derive(Serialize)]
struct ForgotPasswordCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    sections: Vec<FormSection>,
    /// Optional uniform error banner — used only for form-level
    /// validation feedback that doesn't reveal account existence.
    /// Set to `None` for the success path; we never render an error
    /// distinguishing "account exists" from "doesn't exist".
    error: Option<String>,
    /// Per-request UUID v7 from the correlation_id middleware.
    /// Rendered in a `<meta>` tag for support-ticket diagnostics.
    correlation_id: Option<String>,
}

/// Context for `admin/forgot_password_sent.html` (the post-submit
/// "check your inbox" card). No form, no errors, no policy details —
/// pure static copy plus the diagnostics meta.
#[derive(Serialize)]
struct ForgotPasswordSentCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    correlation_id: Option<String>,
}

/// Context for `admin/reset_password.html`. Two render modes
/// distinguished by the `invalid` flag — when `true`, the template
/// shows the "no longer valid" card and ignores the form fields;
/// when `false`, the form is rendered with the policy minimum text.
#[derive(Serialize)]
struct ResetPasswordCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    /// `true` ⇒ render the generic "this link is no longer valid"
    /// card. Used for unknown / expired / consumed tokens
    /// indistinguishably (§2.3) and for the rate-limited POST path.
    invalid: bool,
    /// Form-action target (the same opaque URL parameter the user
    /// arrived with). Rendered ONLY into the `<form action>` and
    /// the `Request a new link` href; never echoed back as visible
    /// content.
    token: String,
    /// Live policy minimum length, derived from
    /// `admin.active_password_policy().min_length()` so users see
    /// the floor before submitting. Default 10; projects may
    /// override.
    min_length: usize,
    sections: Vec<FormSection>,
    /// Form-level errors (e.g. "passwords didn't match"). NEVER the
    /// raw policy enum debug; only user-safe `Display` strings.
    errors: Vec<String>,
    flash: Option<FlashCtx>,
    correlation_id: Option<String>,
}

// ---- Form-section builders -----------------------------------------------

fn forgot_password_form_sections(prefilled_email: &str) -> Vec<FormSection> {
    vec![FormSection {
        title: None,
        fields: vec![FormField {
            name: "email",
            label: "Email".to_string(),
            widget: "input",
            input_type: "email",
            value: prefilled_email.to_string(),
            hint: None,
            placeholder: None,
            required: true,
            options: None,
            multiple: false,
            span: 2,
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
        }],
    }]
}

fn reset_password_form_sections(
    min_length: usize,
    field_errors: &[(&'static str, String)],
) -> Vec<FormSection> {
    let err_for = |name: &str| -> Vec<String> {
        field_errors
            .iter()
            .filter(|(n, _)| *n == name)
            .map(|(_, m)| m.clone())
            .collect()
    };
    let hint = format!("At least {min_length} characters.");
    vec![FormSection {
        title: None,
        fields: vec![
            FormField {
                name: "new_password1",
                label: "New password".to_string(),
                widget: "input",
                input_type: "password",
                value: String::new(),
                hint: Some(hint.clone()),
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
                errors: err_for("new_password1"),
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
                errors: err_for("new_password2"),
                target_model: None,
                checked: false,
            },
        ],
    }]
}

// ---- Handlers: forgot-password -------------------------------------------

pub(crate) async fn show_forgot_password(ctx: &AdminCtx, req: &Request) -> Result<Response> {
    let view = ForgotPasswordCtx {
        base: BaseContext::new(None, csrf_token(req), &ctx.admin),
        page_title: "Reset your password",
        sections: forgot_password_form_sections(""),
        error: None,
        correlation_id: correlation_id(req),
    };
    let body = ctx.templates.render("admin/forgot_password.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/forgot-password`. Always redirects to the uniform
/// "check your inbox" page regardless of whether the email matched
/// an active user, was unknown, was inactive, or was rate-limited.
/// Disclosure rule §2.3.
pub(crate) async fn do_forgot_password(
    ctx: &AdminCtx,
    state: &RecoveryState,
    req: Request,
) -> Result<Response> {
    let cid = correlation_id(&req);
    let form = req.form()?;
    let email = form.get("email").unwrap_or("").to_string();

    // Outcome variants are observed only for log/audit purposes;
    // the user-facing response is the same across all of them.
    let _outcome = issue_reset_token(
        &ctx.db,
        &ctx.admin,
        &state.request_limiter,
        &req,
        &email,
        cid.as_deref(),
    )
    .await?;

    Ok(Response::redirect("/admin/forgot-password/sent"))
}

pub(crate) async fn show_forgot_password_sent(ctx: &AdminCtx, req: &Request) -> Result<Response> {
    let view = ForgotPasswordSentCtx {
        base: BaseContext::new(None, csrf_token(req), &ctx.admin),
        page_title: "Check your email",
        correlation_id: correlation_id(req),
    };
    let body = ctx
        .templates
        .render("admin/forgot_password_sent.html", &view)?;
    Ok(Response::html(body))
}

// ---- Handlers: reset-password --------------------------------------------

/// `GET /admin/reset-password/<token>`. Renders the new-password
/// form when the token is currently valid; renders the generic "no
/// longer valid" card otherwise (unknown / expired / consumed all
/// fall through to the same card).
///
/// The check is non-mutating; the POST path's atomic UPDATE is the
/// authoritative consume. This GET-time check is purely a UX
/// courtesy so users clicking a stale link don't fill in the form
/// before being told it's invalid.
pub(crate) async fn show_reset_password(
    ctx: &AdminCtx,
    req: &Request,
    token: &str,
) -> Result<Response> {
    let valid = check_reset_token_valid(&ctx.db, token).await?;
    let min_length = ctx.admin.active_password_policy().min_length();
    let view = ResetPasswordCtx {
        base: BaseContext::new(None, csrf_token(req), &ctx.admin),
        page_title: if valid {
            "Set a new password"
        } else {
            "This link is no longer valid"
        },
        invalid: !valid,
        token: token.to_string(),
        min_length,
        sections: if valid {
            reset_password_form_sections(min_length, &[])
        } else {
            Vec::new()
        },
        errors: Vec::new(),
        flash: None,
        correlation_id: correlation_id(req),
    };
    let body = ctx.templates.render("admin/reset_password.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/reset-password/<token>`. Validates the form
/// (passwords match) before delegating to
/// [`consume_reset_token`]. Maps the typed [`ConsumeOutcome`]
/// variants to user-facing pages per the disclosure rule:
///
/// - `Consumed` → 303 redirect to `/admin/login?password_reset=success`.
/// - `Invalid` / `RateLimited` → render the "no longer valid" card.
/// - `PolicyRejected(_)` → re-render the form with the policy's
///   `Display` string mapped onto `new_password1`.
pub(crate) async fn do_reset_password(
    ctx: &AdminCtx,
    state: &RecoveryState,
    req: Request,
    token: &str,
) -> Result<Response> {
    let cid = correlation_id(&req);
    let form = req.form()?;
    let pw1 = form.get("new_password1").unwrap_or("").to_string();
    let pw2 = form.get("new_password2").unwrap_or("").to_string();
    let min_length = ctx.admin.active_password_policy().min_length();

    // UX-level mismatch check happens BEFORE the runtime call so a
    // user mistyping doesn't trigger a rate-limit hit + audit
    // breadcrumb. The mismatch is a re-render with a field error;
    // the token stays valid.
    if pw1 != pw2 {
        return render_reset_password_form_error(
            ctx,
            &req,
            token,
            min_length,
            &[(
                "new_password2",
                "The two password fields didn't match.".into(),
            )],
            cid,
        );
    }

    let outcome = consume_reset_token(
        &ctx.db,
        &ctx.admin,
        &state.consume_limiter,
        &req,
        token,
        &pw1,
        cid.as_deref(),
    )
    .await?;

    match outcome {
        ConsumeOutcome::Consumed { .. } => {
            // Successful consume: redirect to login with a success
            // flag the login page reads from `?password_reset=success`
            // (commit #9 wires the banner). Do NOT mint a session —
            // the user signs in afresh so MFA (R3+) gets exercised.
            Ok(Response::redirect("/admin/login?password_reset=success"))
        }
        ConsumeOutcome::Invalid | ConsumeOutcome::RateLimited => {
            // Indistinguishable to the user per §2.3.
            render_reset_password_invalid(ctx, &req, token, min_length, cid)
        }
        ConsumeOutcome::PolicyRejected(err) => {
            // Map the policy error to a user-safe Display string
            // attached to `new_password1`. Never render the Debug
            // form; never include the candidate plaintext (variants
            // structurally don't carry it — pinned by commit #7
            // tests, but defence-in-depth: we only render Display).
            let user_message = render_password_policy_error(&err);
            render_reset_password_form_error(
                ctx,
                &req,
                token,
                min_length,
                &[("new_password1", user_message)],
                cid,
            )
        }
    }
}

/// Render the new-password form with one or more field errors —
/// the user can retry without burning the token.
fn render_reset_password_form_error(
    ctx: &AdminCtx,
    req: &Request,
    token: &str,
    min_length: usize,
    field_errors: &[(&'static str, String)],
    cid: Option<String>,
) -> Result<Response> {
    let view = ResetPasswordCtx {
        base: BaseContext::new(None, csrf_token(req), &ctx.admin),
        page_title: "Set a new password",
        invalid: false,
        token: token.to_string(),
        min_length,
        sections: reset_password_form_sections(min_length, field_errors),
        errors: field_errors.iter().map(|(_, m)| m.clone()).collect(),
        flash: None,
        correlation_id: cid,
    };
    let body = ctx.templates.render("admin/reset_password.html", &view)?;
    Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST))
}

/// Render the generic "this link is no longer valid" card. Used
/// for the `Invalid` and `RateLimited` consume outcomes — the
/// distinction exists in audit/logs, never in this rendered HTML.
fn render_reset_password_invalid(
    ctx: &AdminCtx,
    req: &Request,
    token: &str,
    min_length: usize,
    cid: Option<String>,
) -> Result<Response> {
    let view = ResetPasswordCtx {
        base: BaseContext::new(None, csrf_token(req), &ctx.admin),
        page_title: "This link is no longer valid",
        invalid: true,
        token: token.to_string(),
        min_length,
        sections: Vec::new(),
        errors: Vec::new(),
        flash: None,
        correlation_id: cid,
    };
    let body = ctx.templates.render("admin/reset_password.html", &view)?;
    // 200 (not 4xx) — the user reached a valid URL; the resource
    // happens to no longer be claimable. Status code carries no
    // disclosure value.
    Ok(Response::html(body))
}

/// Map a [`PasswordPolicyError`] to the user-facing string shown on
/// the form. Always uses `Display` (which the trait's default impl
/// is responsible for keeping plaintext-free) — never `Debug`.
fn render_password_policy_error(err: &PasswordPolicyError) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{DefaultPasswordPolicy, PasswordPolicy};

    /// `RecoveryState::from_admin` reads the recovery policy and
    /// builds two limiters sized from its tuples. We can't easily
    /// inspect the internal bucket capacity from outside the
    /// limiter, but we can exercise the constructor and confirm
    /// it doesn't panic on the framework defaults.
    #[test]
    fn recovery_state_constructs_from_default_admin() {
        let admin = Admin::new();
        let state = RecoveryState::from_admin(&admin);
        // Both limiters allow at least one request from a fresh
        // bucket — defaults are 5/15min and 10/5min, both ≥ 1.
        assert!(state.request_limiter.allow("test-ip-1"));
        assert!(state.consume_limiter.allow("test-ip-1"));
    }

    /// The form-section builder wires `min_length` into the field
    /// hint so users see the live policy floor on the rendered
    /// page — even when the project overrides the default.
    #[test]
    fn reset_password_form_renders_live_policy_minimum() {
        let sections = reset_password_form_sections(10, &[]);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(section.fields.len(), 2);
        let pw1 = &section.fields[0];
        assert_eq!(pw1.name, "new_password1");
        assert_eq!(
            pw1.hint.as_deref(),
            Some("At least 10 characters."),
            "hint must reflect the live policy minimum"
        );

        // Project override: the helper must propagate, not hardcode.
        let sections = reset_password_form_sections(16, &[]);
        assert_eq!(
            sections[0].fields[0].hint.as_deref(),
            Some("At least 16 characters."),
        );
    }

    /// Field errors get routed onto the right field's `errors` Vec.
    #[test]
    fn reset_password_form_routes_field_errors_to_named_field() {
        let sections = reset_password_form_sections(
            10,
            &[
                ("new_password1", "policy err".into()),
                ("new_password2", "match err".into()),
            ],
        );
        assert_eq!(sections[0].fields[0].errors, vec!["policy err"]);
        assert_eq!(sections[0].fields[1].errors, vec!["match err"]);
    }

    /// The forgot-password form is single-field (email only).
    #[test]
    fn forgot_password_form_has_only_email_field() {
        let sections = forgot_password_form_sections("");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].fields.len(), 1);
        assert_eq!(sections[0].fields[0].name, "email");
        assert_eq!(sections[0].fields[0].input_type, "email");
        assert!(sections[0].fields[0].autofocus);
    }

    /// Property: the rendered policy-error string never carries the
    /// candidate plaintext. Pinned at the rendering boundary (the
    /// trait's Display is responsible for the property; this test
    /// catches regressions in `render_password_policy_error`).
    #[test]
    fn render_password_policy_error_does_not_leak_plaintext() {
        // Construct a policy + run a known-too-short candidate
        // through it; render the error; assert the plaintext is
        // absent.
        let policy = DefaultPasswordPolicy::new();
        let plaintext = "Pwn4Ge#zZ"; // 9 chars — fails 10-char floor
        let err = policy
            .validate(plaintext)
            .expect_err("9-char password should fail the 10-char floor");
        let rendered = render_password_policy_error(&err);
        assert!(
            !rendered.contains(plaintext),
            "rendered policy error leaked plaintext: {rendered}"
        );
    }

    /// `correlation_id` field is wrapped in `Option<String>`; when
    /// the middleware didn't run (no CorrelationId in the request
    /// context), helpers must still construct a coherent context
    /// without panicking.
    #[test]
    fn render_contexts_tolerate_absent_correlation_id() {
        let admin = Admin::new();
        let base = BaseContext::new(None, "csrf-test".into(), &admin);
        let ctx = ForgotPasswordCtx {
            base,
            page_title: "Reset your password",
            sections: forgot_password_form_sections(""),
            error: None,
            correlation_id: None,
        };
        let json = serde_json::to_string(&ctx).expect("serializes cleanly with absent cid");
        assert!(json.contains("\"correlation_id\":null"));
    }
}
