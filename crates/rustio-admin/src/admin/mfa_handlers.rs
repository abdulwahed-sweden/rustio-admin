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
// The verify GET/POST routes were registered in R3 commit #19;
// the handlers below are live as of that commit. The dead-code
// allow that gated this block until routing landed has been
// removed.

#[derive(Serialize)]
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

// ---- /admin/account/mfa/regenerate-codes (R3 commit #14) -------------------
//
// Two-step regenerate flow per DESIGN_R3_MFA.md §4.5:
//
//   GET  /admin/account/mfa/regenerate-codes → show_regenerate
//        - Re-auth gate (D5). If not elevated, redirect to
//          /admin/reauth?return_to=<this page>.
//        - Render the confirmation page explaining that the
//          current batch becomes unrecoverable (D3 atomicity).
//
//   POST /admin/account/mfa/regenerate-codes → do_regenerate
//        - Re-auth gate (D5).
//        - Call regenerate_backup_codes (commit #10). The
//          runtime runs the DELETE + INSERT batch inside a
//          single transaction; the old codes are unrecoverable
//          from the moment that commit lands.
//        - RegenOutcome cases:
//          * Regenerated { plain_backup_codes,
//                          previous_codes_invalidated }
//            → render the success page with the 8 new codes
//              shown ONCE.
//          * NotEnrolled → 303 redirect to
//            /admin/account/sessions. Defensive — a regenerate
//            attempt on a not-enrolled account collapses to
//            the same outcome as visiting the wrong URL.
//
// **Future re-auth tightening.** When `/admin/reauth` gains
// the both-factors-when-enrolled enforcement (separate commit),
// the elevated_until window that gates this route will already
// have been earned by a TOTP-plus-password re-auth. The
// regenerate handler does not need to know — it just checks
// elevated_until > NOW() — but the substrate guarantees the
// elevation came from both factors when MFA is enrolled.

use crate::auth::mfa::{regenerate_backup_codes, RegenOutcome};

#[derive(Serialize)]
struct MfaRegenerateCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    /// Form-level error banner (empty on first render;
    /// populated only on transient errors that re-render the
    /// confirm page — none today).
    error: Option<String>,
}

#[derive(Serialize)]
struct MfaRegenerateCompleteCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    /// The 8 plaintext backup codes, rendered ONCE.
    plain_backup_codes: Vec<String>,
    /// How many old rows the DELETE removed (used + unused
    /// combined). Surfaced to the user so they can confirm the
    /// regenerate landed on the expected batch size.
    previous_codes_invalidated: u32,
}

/// `GET /admin/account/mfa/regenerate-codes` — render the
/// confirmation form.
///
/// Re-auth gate (D5) per the same shape as `show_enroll`. A
/// stolen cookie cannot reach this page without password
/// re-entry; a stolen cookie on an MFA-enrolled user cannot
/// reach it without the TOTP step once the `/admin/reauth`
/// extension lands.
pub(crate) async fn show_regenerate(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    // Re-auth gate. Same shape as the destructive admin routes
    // in admin_recovery_handlers (R2 commit #15).
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
            "/admin/reauth?return_to=/admin/account/mfa/regenerate-codes",
        ));
    }

    let view = MfaRegenerateCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Generate new backup codes",
        error: None,
    };
    let body = ctx.templates.render("admin/mfa_regenerate.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/account/mfa/regenerate-codes` — atomically
/// replace the user's backup-code batch.
///
/// Re-auth gate (D5) is enforced here too — a direct POST
/// without an elevated session must not bypass the wall.
///
/// **Doctrine 22 untouched.** The regenerate runtime does not
/// write `revoked_at` — D3 transaction wraps the DELETE +
/// INSERTs; existing sessions are unaffected (§4.5).
pub(crate) async fn do_regenerate(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    // Re-auth gate. Same as show_regenerate's check above.
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
            "/admin/reauth?return_to=/admin/account/mfa/regenerate-codes",
        ));
    }

    let correlation_id = req.header("x-correlation-id");
    let outcome = regenerate_backup_codes(&ctx.db, &req, identity.user_id, correlation_id).await?;

    match outcome {
        RegenOutcome::Regenerated {
            plain_backup_codes,
            previous_codes_invalidated,
        } => {
            let view = MfaRegenerateCompleteCtx {
                base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
                page_title: "New backup codes generated",
                plain_backup_codes,
                previous_codes_invalidated,
            };
            let body = ctx
                .templates
                .render("admin/mfa_regenerate_complete.html", &view)?;
            Ok(Response::html(body))
        }
        RegenOutcome::NotEnrolled => {
            // Defensive: a regenerate attempt on a not-enrolled
            // user collapses to the same outcome as visiting
            // the wrong URL — redirect to the account-sessions
            // page where the user can see their MFA state.
            Ok(Response::redirect("/admin/account/sessions"))
        }
    }
}

// ---- /admin/account/mfa/disable (R3 commit #15) ----------------------------
//
// Two-step disable flow per DESIGN_R3_MFA.md §4.3:
//
//   GET  /admin/account/mfa/disable → show_disable
//        - Re-auth gate (D5). If not elevated, redirect to
//          /admin/reauth?return_to=<this page>.
//        - Render the confirmation page explaining that disable
//          revokes every session for this user (current device
//          included).
//
//   POST /admin/account/mfa/disable → do_disable
//        - Re-auth gate (D5).
//        - Call disable_mfa (commit #9). The runtime clears
//          the MFA columns, deletes the backup-code rows,
//          and runs invalidate_sessions(User, MfaDisabled)
//          via Doctrine 22's single writer.
//        - On Disabled: clear the session cookie and redirect
//          to /admin/login. The user signs back in with
//          password only.
//        - On NotEnrolled: 303 to /admin/account/sessions
//          (defensive — should not happen if the GET form
//          guards it correctly).
//        - PolicyRequired is currently unreachable from the
//          runtime; if it ever surfaces (when policy
//          enforcement moves into the runtime layer), the
//          handler renders a "your administrator requires MFA"
//          forbidden page. For commit #15 that branch returns
//          a generic 403 without a custom template; future
//          commits can ship a dedicated copy.

use crate::auth::mfa::{disable_mfa, DisableOutcome};

#[derive(Serialize)]
struct MfaDisableCtx {
    #[serde(flatten)]
    base: BaseContext,
    page_title: &'static str,
    /// Form-level error banner (empty on first render).
    error: Option<String>,
}

/// `GET /admin/account/mfa/disable` — render the confirmation
/// form.
///
/// Re-auth gate (D5) per the same shape as `show_enroll` and
/// `show_regenerate`.
pub(crate) async fn show_disable(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
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
            "/admin/reauth?return_to=/admin/account/mfa/disable",
        ));
    }

    let view = MfaDisableCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Disable two-factor authentication",
        error: None,
    };
    let body = ctx.templates.render("admin/mfa_disable.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/account/mfa/disable` — disable MFA and sign
/// the user out of every device.
///
/// Re-auth gate (D5) is enforced here too — a direct POST
/// without an elevated session must not bypass the wall.
///
/// **Doctrine 22 routed through.** `disable_mfa` calls
/// `auth::sessions::invalidate_sessions(User, MfaDisabled)`
/// — the framework's only writer of `revoked_at`. The
/// handler then clears the session cookie (the runtime
/// revoked the row; the cookie is a stale reference that
/// should not survive the response) and redirects to
/// `/admin/login`.
pub(crate) async fn do_disable(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    // Re-auth gate. Same as show_disable's check above.
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
            "/admin/reauth?return_to=/admin/account/mfa/disable",
        ));
    }

    let correlation_id = req.header("x-correlation-id");
    let outcome = disable_mfa(&ctx.db, &req, identity.user_id, correlation_id).await?;

    match outcome {
        DisableOutcome::Disabled {
            sessions_revoked: _,
        } => {
            // The current session was revoked along with every
            // other session for this user. Clear the cookie
            // before redirecting — leaving it set would carry
            // a stale token forward that no longer matches
            // any live row. Same pattern as the existing
            // do_logout handler in admin/handlers.rs.
            let clear = format!(
                "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
                auth::SESSION_COOKIE
            );
            Ok(Response::redirect("/admin/login?mfa_disabled=1").with_header("set-cookie", clear))
        }
        DisableOutcome::NotEnrolled => {
            // Defensive: the GET form should not have rendered
            // for a not-enrolled user. If we land here, redirect
            // to the account-sessions page where the user can
            // see their MFA state.
            Ok(Response::redirect("/admin/account/sessions"))
        }
        DisableOutcome::PolicyRequired => {
            // Reserved branch (the current runtime never
            // returns this). When MfaPolicy enforcement moves
            // into the runtime, surface a generic 403 here.
            // A dedicated template lands in a future commit.
            Ok(
                Response::html("Forbidden: your administrator requires MFA.")
                    .with_status(hyper::StatusCode::FORBIDDEN),
            )
        }
    }
}
