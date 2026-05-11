//! HTTP handlers for the R3 MFA surface (`DESIGN_R3_MFA.md`
//! §4 + §10). Routes are registered in
//! `admin::routes::register_admin_routes` in a later R3 commit.
//! This module owns the handlers; runtime fns live in
//! `auth::mfa`.
//!
//! ## Routes wired through this module
//!
//! - `GET  /admin/mfa/verify`                       → [`show_verify`]
//! - `POST /admin/mfa/verify`                       → [`do_verify`]
//! - (later) `GET/POST /admin/account/mfa/enroll`             — R3 commit #13
//! - (later) `GET/POST /admin/account/mfa/regenerate-codes`   — R3 commit #14
//! - (later) `GET/POST /admin/account/mfa/disable`            — R3 commit #15
//!
//! ## What this module does NOT do
//!
//! - Audit rows on the verify form itself. The verify flow's
//!   audit footprint is:
//!   - TOTP success: silent (session-promotion lineage via
//!     `parent_session_id` per `DESIGN_AUDIT.md` §8.3).
//!   - Backup-code success: `AuditEvent::MfaCodeConsumed` row
//!     emitted by `auth::mfa::consume_backup_code`.
//!   - Failure (any cause): silent. A failed verify attempt is
//!     uniform-error-rendered with no log row, matching the
//!     §3.1 disclosure rules.
//!
//! - Session revocation in the verify flow. Doctrine 22
//!   untouched; the trust-escalation primitive
//!   [`crate::auth::mfa::promote_session_to_mfa_verified`]
//!   delegates revocation to `auth::sessions::invalidate_sessions`
//!   per the single-writer invariant.

use serde::Serialize;

use crate::auth::mfa::{
    base32_decode_no_pad, build_otpauth_url, confirm_enrolment, consume_backup_code,
    promote_session_to_mfa_verified, provision_secret, verify_totp_for_user, BackupConsumeOutcome,
    EnrolOutcome, MfaKey, VerifyOutcome,
};
use crate::auth::recovery_admin::check_session_elevated;
use crate::auth::{self, Identity};
use crate::error::Result;
use crate::http::{Request, Response};

use super::handlers::{csrf_token, AdminCtx};
use super::render::BaseContext;

// ---- /admin/mfa/verify (R3 commit #12) -------------------------------------
//
// The handlers and template context below are dead-code at this
// commit. R3 commit #19 wires the GET/POST routes; once mapped,
// both functions become live and the #[allow(dead_code)]
// attributes can be removed. Same pattern as recovery_admin
// runtime fns that landed before R2 commit #17's route
// registration.

#[derive(Serialize)]
#[allow(dead_code)] // call sites land at the verify route in R3 commit #19
struct MfaVerifyCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    email: String,
    error: Option<String>,
}

/// `GET /admin/mfa/verify` — render the second-factor form.
///
/// Reachable from the login flow when the user's password
/// verification succeeded and `identity.mfa_enabled = TRUE`.
/// In R3 commit #16 the login flow lands the pending-MFA
/// session-state mechanism that gates this route; until then
/// the handler is reachable for any authenticated identity
/// but unreachable in practice because no route maps to it
/// (commit #19 wires the routing).
///
/// The form accepts either a 6-digit TOTP code or an
/// `XXXX-XXXX` backup code in the same input field. The POST
/// handler distinguishes based on which verifier accepts the
/// input.
#[allow(dead_code)] // call site lands at the verify GET route in R3 commit #19
pub(crate) async fn show_verify(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    let view = MfaVerifyCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Two-factor authentication",
        email: identity.email.clone(),
        error: None,
    };
    let body = ctx.templates.render("admin/mfa_verify.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/mfa/verify` — verify the candidate and promote
/// the session.
///
/// Flow:
///
/// 1. Parse `code` from the form. Empty input → uniform 401.
/// 2. Resolve current session id from the cookie (same pattern
///    as `/admin/reauth`'s `do_reauth`).
/// 3. Try [`verify_totp_for_user`]. The function rejects:
///    - non-numeric input (collapses to `VerifyOutcome::Invalid`),
///    - codes outside the configured skew window,
///    - replayed steps (D4).
/// 4. If TOTP returns `Verified`, proceed. If TOTP returns
///    `Invalid`, fall back to [`consume_backup_code`]:
///    - The backup-code path normalises the input
///      (hyphens / casing) and Argon2id-verifies against the
///      unused-rows index.
///    - `via = "login"` is stamped into the
///      `AuditEvent::MfaCodeConsumed` metadata.
/// 5. On either success path, call
///    [`promote_session_to_mfa_verified`] (Doctrine 17 token
///    rotation), set the new cookie, redirect 303 to `/admin`.
/// 6. On all-fail (TOTP `Invalid` + backup `Invalid`, or any
///    `Replay`/`NotEnrolled`), re-render with a uniform error
///    and HTTP 401.
///
/// **Uniform failure copy per §3.1.** The handler does NOT
/// distinguish "wrong TOTP" from "wrong backup code" from
/// "replay" in the rendered error. The variant distinctions
/// exist for forensic logging, not user-facing UX.
///
/// **Doctrine 22 untouched.** The handler calls
/// `promote_session_to_mfa_verified` (which calls
/// `auth::sessions::invalidate_sessions` for the parent-row
/// revocation), `verify_totp_for_user` (UPDATE on the user
/// row's `mfa_last_used_step` only), and `consume_backup_code`
/// (UPDATE on the backup-code row's `used_at` + audit emit).
/// None write `revoked_at` directly.
#[allow(dead_code)] // call site lands at the verify POST route in R3 commit #19
pub(crate) async fn do_verify(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let code = form.get("code").unwrap_or("").trim().to_string();

    // Uniform failure renderer used by every error branch
    // below. Same status, same wording — no distinction
    // between "wrong TOTP", "replay", "no current session",
    // or "no MFA enrolled" reaches the client.
    let uniform_failure = |ctx: &AdminCtx| -> Result<Response> {
        let view = MfaVerifyCtx {
            base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
            page_title: "Two-factor authentication",
            email: identity.email.clone(),
            error: Some("Could not verify your code.".to_string()),
        };
        let body = ctx.templates.render("admin/mfa_verify.html", &view)?;
        Ok(Response::html(body).with_status(hyper::StatusCode::UNAUTHORIZED))
    };

    if code.is_empty() {
        return uniform_failure(ctx);
    }

    // Resolve current session id via the cookie token. Same
    // pattern as do_reauth — the verify-handler needs the
    // session id for the trust-escalation primitive's
    // parent_session_id field.
    let cookie = match req.header("cookie") {
        Some(c) => c,
        None => return uniform_failure(ctx),
    };
    let token = match auth::session_token_from_cookie(cookie) {
        Some(t) => t,
        None => return uniform_failure(ctx),
    };
    let current_session_id = match auth::current_session_id(&ctx.db, &token).await? {
        Some(id) => id,
        None => return uniform_failure(ctx),
    };

    let policy = ctx.admin.active_recovery_policy();
    let step_seconds = policy.mfa_step_seconds();
    let skew_steps = policy.mfa_skew_steps();

    // Read the AES-256-GCM key for decrypting the user's TOTP
    // secret. Today: read from env on every request. Future:
    // cache on Admin to avoid the env-read latency. The boot
    // guard that ties RUSTIO_SECRET_KEY's existence to
    // MfaPolicy != Disabled lands in a separate commit; until
    // then this from_env() call returns Error::Internal if
    // the env var is unset, surfacing as the framework's
    // generic 500 (acceptable since the handler is unreachable
    // until commit #19 anyway).
    let key = MfaKey::from_env()?;
    let correlation_id = req.header("x-correlation-id");

    // 1. Try TOTP first.
    let totp_outcome = verify_totp_for_user(
        &ctx.db,
        identity.user_id,
        &code,
        step_seconds,
        skew_steps,
        &key,
    )
    .await?;

    let totp_verified = matches!(totp_outcome, VerifyOutcome::Verified { .. });

    let verified = if totp_verified {
        true
    } else {
        // 2. TOTP did not match. Try backup-code fallback only
        //    for the Invalid outcome. Replay / NotEnrolled
        //    collapse to uniform failure without trying the
        //    backup code — a replayed TOTP is a security
        //    signal we should not paper over with a backup-code
        //    attempt, and NotEnrolled means the user shouldn't
        //    be on this page at all.
        match totp_outcome {
            VerifyOutcome::Invalid => {
                let backup_outcome = consume_backup_code(
                    &ctx.db,
                    &req,
                    identity.user_id,
                    &code,
                    "login",
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

    if !verified {
        return uniform_failure(ctx);
    }

    // 3. Promote the session via Doctrine 17 token rotation.
    //    Mints a fresh mfa_verified row, revokes the parent
    //    via invalidate_sessions(Single, TrustEscalation).
    let new_token =
        promote_session_to_mfa_verified(&ctx.db, current_session_id, identity.user_id).await?;
    let cookie = format!(
        "{}={new_token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=1209600",
        auth::SESSION_COOKIE
    );

    Ok(Response::redirect("/admin").with_header("set-cookie", cookie))
}

// ---- /admin/account/mfa/enroll (R3 commit #13) -----------------------------
//
// Two-step enrolment flow per DESIGN_R3_MFA.md §4.1:
//
//   GET  /admin/account/mfa/enroll → show_enroll
//        - Re-auth gate (D5). If the session is not elevated,
//          redirect to /admin/reauth?return_to=<this page>.
//        - provision_secret() — 20 random bytes + base32 form.
//        - Build otpauth:// URL via build_otpauth_url so the
//          user's authenticator app can scan or import it.
//        - Render the form: clickable otpauth URL + manual-entry
//          base32 + hidden `secret_base32` form field + 6-digit
//          code input.
//
//   POST /admin/account/mfa/enroll → do_enroll
//        - Re-auth gate (D5) — same as GET.
//        - Read hidden `secret_base32` + user-typed `code`.
//        - base32_decode_no_pad the hidden secret.
//          Decode failure → uniform "invalid code" + re-render
//          GET form with a fresh secret (the user's window
//          has invalidated).
//        - confirm_enrolment — verifies, encrypts, persists,
//          inserts hashed backup codes, emits MfaEnabled.
//        - EnrolOutcome cases:
//          * Enrolled { plain_backup_codes } → render the
//            success page with the 8 codes shown ONCE.
//          * InvalidCode → re-render GET with the SAME secret
//            (so the user can retry without re-scanning).
//          * AlreadyEnrolled → redirect to /admin/account/sessions
//            (defensive; the GET form should have not rendered).
//
// **Hidden secret rationale.** The provisioned secret lives in
// process memory at GET-time, but the framework has no
// server-side per-user session-state map. Carrying the secret
// as a hidden form field is safe because:
//   (a) The user already saw the secret in the QR / manual
//       display.
//   (b) Tampering with the field makes the verify fail; nothing
//       persists.
//   (c) The secret only becomes load-bearing after persistence,
//       which only happens on successful verify.
// Future commits may move the secret into a dedicated
// short-lived session-state row; commit #13 takes the simpler
// hidden-field path.

#[derive(Serialize)]
#[allow(dead_code)] // call site at the enrolment GET route (R3 commit #19)
struct MfaEnrollCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    /// The otpauth:// URI for the user's authenticator app.
    otpauth_url: String,
    /// The same secret in base32 form, for the manual-entry
    /// fallback when the user can't scan the QR.
    secret_base32: String,
    /// Form-level error banner (empty on first GET; populated
    /// when the POST returns InvalidCode and the GET re-renders
    /// with the same secret).
    error: Option<String>,
}

#[derive(Serialize)]
#[allow(dead_code)] // call site at the enrolment success page (R3 commit #19)
struct MfaEnrollCompleteCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    /// The 8 plaintext backup codes, rendered ONCE.
    plain_backup_codes: Vec<String>,
}

/// `GET /admin/account/mfa/enroll` — render the enrolment form.
///
/// Re-auth gate per D5: if the current session's
/// `elevated_until` is in the past (or never set), redirect to
/// `/admin/reauth?return_to=/admin/account/mfa/enroll`. A
/// stolen cookie cannot land on this page without password
/// re-entry.
#[allow(dead_code)] // call site lands at the enrolment GET route (R3 commit #19)
pub(crate) async fn show_enroll(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    // Re-auth gate (D5). Same shape as the destructive admin
    // routes in admin_recovery_handlers (R2 commit #15).
    let cookie = req.header("cookie");
    let token = cookie.and_then(auth::session_token_from_cookie);
    let session_id = if let Some(t) = token {
        auth::current_session_id(&ctx.db, &t).await?
    } else {
        None
    };
    let elevated = match session_id {
        Some(id) => check_session_elevated(&ctx.db, id).await?,
        None => false,
    };
    if !elevated {
        return Ok(Response::redirect(
            "/admin/reauth?return_to=/admin/account/mfa/enroll",
        ));
    }

    let provisioned = provision_secret();
    let issuer = ctx.admin.branding().site_title.as_str();
    let step_seconds = ctx.admin.active_recovery_policy().mfa_step_seconds();
    let otpauth_url = build_otpauth_url(issuer, &identity.email, &provisioned.base32, step_seconds);

    let view = MfaEnrollCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Set up two-factor authentication",
        otpauth_url,
        secret_base32: provisioned.base32,
        error: None,
    };
    let body = ctx.templates.render("admin/mfa_enroll.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/account/mfa/enroll` — verify the user's first
/// TOTP code and persist the enrolment.
///
/// Re-auth gate (D5) is enforced here too — a direct POST
/// without GET-first must not bypass the wall.
#[allow(dead_code)] // call site lands at the enrolment POST route (R3 commit #19)
pub(crate) async fn do_enroll(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    // Re-auth gate. Same as show_enroll's check above.
    let cookie = req.header("cookie");
    let token = cookie.and_then(auth::session_token_from_cookie);
    let session_id = if let Some(t) = token {
        auth::current_session_id(&ctx.db, &t).await?
    } else {
        None
    };
    let elevated = match session_id {
        Some(id) => check_session_elevated(&ctx.db, id).await?,
        None => false,
    };
    if !elevated {
        return Ok(Response::redirect(
            "/admin/reauth?return_to=/admin/account/mfa/enroll",
        ));
    }

    let form = req.form()?;
    let secret_base32 = form.get("secret_base32").unwrap_or("").to_string();
    let code_str = form.get("code").unwrap_or("").trim().to_string();

    // Re-render helper for the InvalidCode and decode-failure
    // branches. The GET page's secret-loss UX is unavoidable
    // if the hidden field round-trips a corrupted value;
    // commit #13 chooses re-render-with-fresh-secret rather
    // than carrying a tampered value forward.
    let render_with_fresh_secret = |ctx: &AdminCtx, error: &str| -> Result<Response> {
        let provisioned = provision_secret();
        let issuer = ctx.admin.branding().site_title.as_str();
        let step_seconds = ctx.admin.active_recovery_policy().mfa_step_seconds();
        let otpauth_url =
            build_otpauth_url(issuer, &identity.email, &provisioned.base32, step_seconds);
        let view = MfaEnrollCtx {
            base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
            page_title: "Set up two-factor authentication",
            otpauth_url,
            secret_base32: provisioned.base32,
            error: Some(error.to_string()),
        };
        let body = ctx.templates.render("admin/mfa_enroll.html", &view)?;
        Ok(Response::html(body).with_status(hyper::StatusCode::UNAUTHORIZED))
    };

    // Decode the hidden secret. Failure → uniform InvalidCode
    // response with a fresh secret (the user's window is gone).
    let Some(secret_bytes) = base32_decode_no_pad(&secret_base32) else {
        return render_with_fresh_secret(ctx, "Could not verify your code.");
    };
    if secret_bytes.len() != 20 {
        return render_with_fresh_secret(ctx, "Could not verify your code.");
    }

    // Parse the candidate as u32.
    let candidate_code = match code_str.parse::<u32>() {
        Ok(n) if n < 1_000_000 => n,
        _ => return render_with_fresh_secret(ctx, "Could not verify your code."),
    };

    let policy = ctx.admin.active_recovery_policy();
    let step_seconds = policy.mfa_step_seconds();
    let skew_steps = policy.mfa_skew_steps();
    let key = MfaKey::from_env()?;
    let correlation_id = req.header("x-correlation-id");

    // Call the runtime. The function checks already-enrolled,
    // verifies the candidate, encrypts the secret, INSERTs the
    // backup codes, and emits AuditEvent::MfaEnabled.
    let outcome = confirm_enrolment(
        &ctx.db,
        &req,
        identity.user_id,
        &secret_bytes,
        candidate_code,
        step_seconds,
        skew_steps,
        &key,
        1, // key_id = 1 until staged-rotation MfaSecretKeyResolver lands
        correlation_id,
    )
    .await?;

    match outcome {
        EnrolOutcome::Enrolled { plain_backup_codes } => {
            let view = MfaEnrollCompleteCtx {
                base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
                page_title: "Two-factor authentication enabled",
                plain_backup_codes,
            };
            let body = ctx
                .templates
                .render("admin/mfa_enroll_complete.html", &view)?;
            Ok(Response::html(body))
        }
        EnrolOutcome::InvalidCode => {
            // Re-render with the SAME secret so the user can
            // retry without re-scanning the QR. The hidden
            // field already carries the secret on the way back.
            let view = MfaEnrollCtx {
                base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
                page_title: "Set up two-factor authentication",
                otpauth_url: build_otpauth_url(
                    ctx.admin.branding().site_title.as_str(),
                    &identity.email,
                    &secret_base32,
                    step_seconds,
                ),
                secret_base32,
                error: Some("Could not verify your code.".to_string()),
            };
            let body = ctx.templates.render("admin/mfa_enroll.html", &view)?;
            Ok(Response::html(body).with_status(hyper::StatusCode::UNAUTHORIZED))
        }
        EnrolOutcome::AlreadyEnrolled => {
            // Defensive: the GET form should not have rendered
            // for an already-enrolled user. If we land here, the
            // safest move is to redirect to the account-sessions
            // page where the user can see their MFA state.
            Ok(Response::redirect("/admin/account/sessions"))
        }
    }
}
