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
//! Slimmed for Tier 1: the legacy file's developer stub routes
//! (`__schema__`, `__logs__`, `__sql_console__`) and the FK remote-
//! search endpoint have been dropped. Everything else — `/static/admin.css`
//! and `/static/admin.js` (P8), login/logout, dashboard,
//! /admin/users/*, /admin/groups/*, /admin/history,
//! /admin/password_change, /admin/:model/* CRUD,
//! /admin/:model/:id/history — is wired below.

use std::sync::Arc;

use crate::auth::{self, Identity, Role};
use crate::error::{Error, Result};
use crate::http::{Request, Response};
use crate::orm::Db;
use crate::router::Router;
use crate::templates::Templates;

/// Embedded stylesheet baked into the binary.
///
/// The admin stylesheet is authored as a multi-file architecture
/// under `assets/static/admin/` (tokens → base → layout →
/// components → pages → responsive → print), modelled on GitHub
/// Primer / IBM Carbon. The browser still receives one concatenated
/// bundle so we keep our "self-hosted, single round-trip, no FOUT"
/// doctrine.
///
/// Order below MUST mirror the `@import` manifest in
/// `assets/static/admin/admin.css` exactly — the two lists are the
/// source of cascade order, and they must stay in lock-step or the
/// served bundle will silently drift from what contributors author.
///
/// Project overrides happen via `Admin::theme(...)` (CSS custom
/// properties) rather than an asset override, so we don't expose a
/// disk path here.
const ADMIN_CSS: &str = concat!(
    // ---- tokens -----------------------------------------------
    include_str!("../../assets/static/admin/tokens/colors.css"),
    "\n",
    include_str!("../../assets/static/admin/tokens/spacing.css"),
    "\n",
    include_str!("../../assets/static/admin/tokens/radius.css"),
    "\n",
    include_str!("../../assets/static/admin/tokens/shadows.css"),
    "\n",
    include_str!("../../assets/static/admin/tokens/typography.css"),
    "\n",
    // ---- base -------------------------------------------------
    include_str!("../../assets/static/admin/base/reset.css"),
    "\n",
    include_str!("../../assets/static/admin/base/base.css"),
    "\n",
    include_str!("../../assets/static/admin/base/typography.css"),
    "\n",
    include_str!("../../assets/static/admin/base/typography-i18n.css"),
    "\n",
    include_str!("../../assets/static/admin/base/utilities.css"),
    "\n",
    // ---- layout -----------------------------------------------
    include_str!("../../assets/static/admin/layout/shell.css"),
    "\n",
    include_str!("../../assets/static/admin/layout/topbar.css"),
    "\n",
    include_str!("../../assets/static/admin/layout/sidebar.css"),
    "\n",
    include_str!("../../assets/static/admin/layout/footer.css"),
    "\n",
    // ---- components -------------------------------------------
    include_str!("../../assets/static/admin/components/cards.css"),
    "\n",
    include_str!("../../assets/static/admin/components/buttons.css"),
    "\n",
    include_str!("../../assets/static/admin/components/forms.css"),
    "\n",
    include_str!("../../assets/static/admin/components/tables.css"),
    "\n",
    include_str!("../../assets/static/admin/components/filters.css"),
    "\n",
    include_str!("../../assets/static/admin/components/dropdowns.css"),
    "\n",
    include_str!("../../assets/static/admin/components/search_palette.css"),
    "\n",
    include_str!("../../assets/static/admin/components/pagination.css"),
    "\n",
    include_str!("../../assets/static/admin/components/pills.css"),
    "\n",
    include_str!("../../assets/static/admin/components/flashes.css"),
    "\n",
    include_str!("../../assets/static/admin/components/timeline.css"),
    "\n",
    include_str!("../../assets/static/admin/components/tabs.css"),
    "\n",
    // ---- pages ------------------------------------------------
    include_str!("../../assets/static/admin/pages/auth.css"),
    "\n",
    include_str!("../../assets/static/admin/pages/dashboard.css"),
    "\n",
    include_str!("../../assets/static/admin/pages/db_browser.css"),
    "\n",
    include_str!("../../assets/static/admin/pages/permissions.css"),
    "\n",
    include_str!("../../assets/static/admin/pages/sessions.css"),
    "\n",
    include_str!("../../assets/static/admin/pages/errors.css"),
    "\n",
    // ---- responsive — mobile-first overrides, last so they win.
    include_str!("../../assets/static/admin/layout/responsive.css"),
    "\n",
    // ---- print ------------------------------------------------
    include_str!("../../assets/static/admin/print/print.css"),
);

/// Embedded admin JS (sidebar drawer + dropdowns + bulk select +
/// FK autocomplete). ≤200 LOC, no build step.
const ADMIN_JS: &str = include_str!("../../assets/static/admin.js");

/// Self-hosted fonts (SIL OFL-1.1, see assets/static/fonts/LICENSE.txt).
/// Bundling them as bytes keeps the single-binary deploy story intact
/// and avoids the FOUT/CDN round-trip every consuming app would
/// otherwise inherit from a Google Fonts <link>.
///
/// Latin (identity): Geist Variable wght 100..900. Latin fallback:
/// Inter Variable (Latin-ext + Cyrillic + Greek + Vietnamese under
/// one variable file). Mono: Geist Mono Variable. Arabic: Noto
/// Naskh Arabic Variable wght 400..700 (default reading face) +
/// Tajawal 400/500/700 (selective geometric accent). International
/// scripts: Noto Sans Thai + Devanagari (variable, auto-loaded via
/// unicode-range) + Noto Sans JP/KR/SC (static Regular, lang-gated
/// to avoid Han Unification shape collisions).
const FONT_GEIST: &[u8] = include_bytes!("../../assets/static/fonts/Geist-Variable.woff2");
const FONT_GEIST_MONO: &[u8] = include_bytes!("../../assets/static/fonts/GeistMono-Variable.woff2");
const FONT_INTER: &[u8] = include_bytes!("../../assets/static/fonts/InterVariable.woff2");
const FONT_TAJAWAL_REG: &[u8] = include_bytes!("../../assets/static/fonts/Tajawal-Regular.woff2");
const FONT_TAJAWAL_MED: &[u8] = include_bytes!("../../assets/static/fonts/Tajawal-Medium.woff2");
const FONT_TAJAWAL_BOLD: &[u8] = include_bytes!("../../assets/static/fonts/Tajawal-Bold.woff2");
const FONT_NOTO_NASKH_AR: &[u8] =
    include_bytes!("../../assets/static/fonts/NotoNaskhArabic-Variable.woff2");
const FONT_NOTO_THAI: &[u8] =
    include_bytes!("../../assets/static/fonts/NotoSansThai-Variable.woff2");
const FONT_NOTO_DEVA: &[u8] =
    include_bytes!("../../assets/static/fonts/NotoSansDevanagari-Variable.woff2");
const FONT_NOTO_JP: &[u8] = include_bytes!("../../assets/static/fonts/NotoSansJP-Regular.woff2");
const FONT_NOTO_KR: &[u8] = include_bytes!("../../assets/static/fonts/NotoSansKR-Regular.woff2");
const FONT_NOTO_SC: &[u8] = include_bytes!("../../assets/static/fonts/NotoSansSC-Regular.woff2");

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

/// Paths a user with `must_change_password = TRUE` is allowed to
/// reach without first completing the forced rotation.
/// Locked-decision per `DESIGN_R2_ORGANISATIONAL.md` §12.
///
/// Exact-path match (no prefix matching). Sub-paths of
/// `/admin/account/sessions` (e.g. `/admin/account/sessions/revoke`)
/// are intentionally NOT whitelisted — a user being forced to
/// rotate may view their active sessions but must finish the
/// rotation before revoking siblings.
const MUST_CHANGE_WHITELIST: &[&str] = &[
    "/admin/must-change-password",
    "/admin/logout",
    "/admin/account/sessions",
];

/// Whether `path` is on the must-change-password whitelist.
/// Pulled out as a free fn so the rule is unit-testable without a
/// `Request`. See [`MUST_CHANGE_WHITELIST`] for the contract.
fn is_must_change_whitelisted_path(path: &str) -> bool {
    MUST_CHANGE_WHITELIST.contains(&path)
}

/// Exact-path read-only-mode allowlist for mutating verbs
/// (POST / PUT / DELETE). When [`Admin::read_only`] is on, every
/// other mutating request under `/admin/*` is rejected with 403;
/// the entries here keep auth-flow round-trips working so an
/// operator can still sign in and out of a frozen admin.
///
/// Sub-paths under `/admin/account/sessions/`, `/admin/account/mfa/`,
/// and `/admin/reset-password/` need prefix matching (variable
/// `:id` / `:token` segments); [`is_read_only_writable_path`]
/// handles those separately.
const READ_ONLY_EXACT_ALLOW: &[&str] = &[
    "/admin/login",
    "/admin/logout",
    "/admin/reauth",
    "/admin/forgot-password",
    "/admin/mfa/verify",
    "/admin/must-change-password",
    "/admin/password_change",
];

const READ_ONLY_PREFIX_ALLOW: &[&str] = &[
    // Token-bearing self-recovery URL — operators must be able to
    // consume a reset link even when project mutations are frozen.
    "/admin/reset-password/",
    // Own-session management — sign-out-everywhere, revoke this
    // device, etc. Identity self-service is not "data mutation."
    "/admin/account/sessions/",
    // Own MFA management — enrolment, regeneration, disable. Same
    // identity-self-service reasoning.
    "/admin/account/mfa/",
];

/// `/admin/<model>/saved_filters` and its
/// `/admin/<model>/saved_filters/<id>/delete` sibling are
/// per-operator UI state (bookmarks for the list page), not
/// project-data mutations. Allowlist by suffix-match so they
/// remain usable even in read-only mode.
fn is_saved_filter_path(path: &str) -> bool {
    if let Some(rest) = path.strip_prefix("/admin/") {
        // Path shape: `<admin_name>/saved_filters` or
        // `<admin_name>/saved_filters/<id>/delete`. Find the
        // first `/saved_filters` segment after the model slug.
        if let Some((_, after)) = rest.split_once('/') {
            return after == "saved_filters" || after.starts_with("saved_filters/");
        }
    }
    false
}

/// `true` when the HTTP method writes to state — POST, PUT,
/// PATCH, DELETE. GET / HEAD / OPTIONS pass through the read-only
/// guard untouched. Idempotency isn't the discriminator (PUT/PATCH
/// are mutating despite being idempotent) — the question is "does
/// this typically mutate server state?"
pub(crate) fn is_mutating_method(method: &hyper::Method) -> bool {
    matches!(
        *method,
        hyper::Method::POST | hyper::Method::PUT | hyper::Method::PATCH | hyper::Method::DELETE
    )
}

/// Returns `true` when a mutating request to `path` should be
/// allowed despite [`Admin::read_only`] being on. The narrow set
/// of exceptions covers identity / auth flows so a read-only
/// admin is still usable as a sign-in surface.
///
/// Pulled out as a free fn so the policy is unit-testable
/// without a `Request`.
pub(crate) fn is_read_only_writable_path(path: &str) -> bool {
    if READ_ONLY_EXACT_ALLOW.contains(&path) {
        return true;
    }
    if READ_ONLY_PREFIX_ALLOW
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return true;
    }
    is_saved_filter_path(path)
}

/// Extract the `:admin_name` URL segment from a request path of the
/// shape `/admin/<admin_name>[/...]`. Returns `None` for paths that
/// don't match (root `/admin/`, framework reserved `/admin/_*`, or
/// non-admin paths). Used by the per-model read-only gate so the
/// middleware can check `Admin::is_model_read_only` without parsing
/// the router's `:admin_name` capture.
pub(crate) fn extract_admin_name(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/admin/")?;
    let slug = rest.split('/').next()?;
    if slug.is_empty() || slug.starts_with('_') {
        return None;
    }
    Some(slug)
}

/// Paths reachable when `MfaPolicy::Required` is active and the
/// user has not yet enrolled (R3 commit #18). Forward-only
/// enforcement per `DESIGN_R3_MFA.md` D6: existing sessions
/// continue to work, but every non-whitelisted request from a
/// not-yet-enrolled user redirects to the enrolment form.
/// Mirrors [`MUST_CHANGE_WHITELIST`]'s shape so the two
/// interstitial flows compose identically when both gates fire.
///
/// Exact-path match. Sub-paths of `/admin/account/sessions`
/// (e.g. `/admin/account/sessions/revoke`) are NOT whitelisted
/// — a user being forced to enrol may view their active
/// sessions but must finish enrolment before revoking siblings.
const MFA_ENROLL_WHITELIST: &[&str] = &[
    "/admin/account/mfa/enroll",
    "/admin/logout",
    "/admin/account/sessions",
];

fn is_mfa_enroll_whitelisted_path(path: &str) -> bool {
    MFA_ENROLL_WHITELIST.contains(&path)
}

/// Paths reachable when the user has MFA enrolled but the
/// current session has not yet been promoted to `mfa_verified`
/// (the post-password, pre-MFA-verify window from R3 commit
/// #16's `do_login`). The user can complete the second-factor
/// verify, log out, or inspect their active sessions — nothing
/// else.
///
/// Exact-path match. See [`MFA_ENROLL_WHITELIST`] for the
/// rationale around the sessions page.
const MFA_VERIFY_WHITELIST: &[&str] = &[
    "/admin/mfa/verify",
    "/admin/logout",
    "/admin/account/sessions",
];

fn is_mfa_verify_whitelisted_path(path: &str) -> bool {
    MFA_VERIFY_WHITELIST.contains(&path)
}

/// Whether the active `MfaPolicy` requires MFA for a given
/// role. Pulled out as a free fn so the rule is unit-testable
/// without an `Admin` context.
///
/// `MfaPolicy::Disabled` / `Optional` → never required.
/// `MfaPolicy::Required` → required for every role.
/// `MfaPolicy::RequiredForRoles(roles)` → required iff the
///   user's role appears in the slice. An empty slice reads
///   as "no role requires MFA" — equivalent to `Optional`.
fn mfa_required_for_role(policy: crate::auth::MfaPolicy, role: Role) -> bool {
    use crate::auth::MfaPolicy;
    match policy {
        MfaPolicy::Disabled | MfaPolicy::Optional => false,
        MfaPolicy::Required => true,
        MfaPolicy::RequiredForRoles(roles) => roles.contains(&role),
    }
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

    // R2 forced-rotation gate (`DESIGN_R2_ORGANISATIONAL.md` §3.4 +
    // §9.2). When the flag is set, every authenticated request EXCEPT
    // the whitelist redirects to `/admin/must-change-password`. The
    // check sits BEFORE any role gate so even Administrators /
    // Developers with the flag set are funnelled through.
    if ident.must_change_password && !is_must_change_whitelisted_path(req.path()) {
        return Ok(Guard::Redirect(Response::redirect(
            "/admin/must-change-password",
        )));
    }

    // R3 MFA-required gate — forward-only per D6
    // (`DESIGN_R3_MFA.md` §12.3). When the active MfaPolicy
    // requires MFA for this user's role AND they have not
    // enrolled, every non-whitelisted request redirects to the
    // enrolment form. Existing sessions continue to work; the
    // redirect kicks in at the NEXT request, not at the moment
    // the policy flips. This matches R2's must-change-password
    // shape — see MFA_ENROLL_WHITELIST for the reachable paths.
    let policy = ctx.admin.active_mfa_policy();
    if mfa_required_for_role(policy, ident.role)
        && !ident.mfa_enabled
        && !is_mfa_enroll_whitelisted_path(req.path())
    {
        return Ok(Guard::Redirect(Response::redirect(
            "/admin/account/mfa/enroll",
        )));
    }

    // R3 pending-MFA-verify gate (`DESIGN_R3_MFA.md` §4.2 +
    // §12.3). When the user has MFA enrolled but the current
    // session has not yet been promoted to mfa_verified (the
    // post-password, pre-MFA-verify window from commit #16's
    // do_login), restrict access to the MFA verify whitelist.
    // The verify POST handler rotates the session via
    // promote_session_to_mfa_verified once both factors land.
    use crate::auth::SessionTrust;
    if ident.mfa_enabled
        && ident.trust_level != SessionTrust::MfaVerified
        && !is_mfa_verify_whitelisted_path(req.path())
    {
        return Ok(Guard::Redirect(Response::redirect("/admin/mfa/verify")));
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

/// Pure verdict for the R1 strict-mailer boot guard
/// (`DESIGN_RECOVERY.md` §12.1). Returns an operator-facing error
/// string when the policy demands a real mailer but `Admin::new()`'s
/// default `LogMailer` is still in place; returns `Ok(())`
/// otherwise.
///
/// Detection is deterministic and structural: it reads
/// [`Admin::has_custom_mailer`] (set whenever
/// [`Admin::mailer`] has been called). No `Arc::ptr_eq` against a
/// freshly-constructed `LogMailer`; no environment heuristics; no
/// hostname checks; no "production mode" guessing — the operator
/// declares intent by calling `Admin::mailer(...)` (and opts the
/// policy in via `RecoveryPolicy::strict_mailer_required(true)`).
///
/// The framework treats an explicit `Admin::mailer(...)` call as
/// satisfying the guard even when the supplied mailer is itself a
/// `LogMailer` — this is the documented escape hatch for projects
/// that want to silence the guard during a migration window
/// without yet wiring a real transport.
/// Best-effort identity resolution for the chrome of an admin
/// error page. Mirrors what `auth::routes::login_guard` does on
/// the success path, but is non-fatal: any failure returns `None`
/// and the error page falls back to the unauthenticated chrome
/// (just like the original behaviour).
///
/// `VISIBILITY_AUDIT.md` A2: without this lookup, every 4xx/5xx
/// page rendered as if the operator were logged out, so the
/// operator lost their sidebar + top-bar account links the
/// moment they hit an erroring route — a navigational dead-end.
async fn resolve_identity_for_error_page(db: &Db, cookie_header: &str) -> Option<Identity> {
    let token = auth::session_token_from_cookie(cookie_header)?;
    let identity = auth::identity_from_session(db, token.as_str())
        .await
        .ok()
        .flatten()?;
    if !identity.is_active {
        return None;
    }
    Some(identity)
}

fn strict_mailer_guard_check(admin: &Admin) -> std::result::Result<(), String> {
    if admin.active_recovery_policy().strict_mailer_required() && !admin.has_custom_mailer() {
        Err(
            "rustio-admin: RecoveryPolicy::strict_mailer_required() = true but no mailer \
             was registered via Admin::mailer(...).\n\n\
             The framework's default LogMailer writes recovery emails to log::info! instead \
             of sending them, which is unsuitable for production. Recovery routes are NOT \
             registered with this configuration.\n\n\
             To resolve, choose one:\n\
              (a) register a real mailer before calling register_admin_routes:\n\
                  Admin::mailer(Arc::new(MyProjectMailer::new(...)))\n\
              (b) opt the policy out of strict mode (the framework default — dev / CI / \
                  testing baseline):\n\
                  RecoveryPolicy::strict_mailer_required(false)\n\n\
             See DESIGN_RECOVERY.md §12.1 for the contract."
                .to_string(),
        )
    } else {
        Ok(())
    }
}

// public:
pub fn register_admin_routes(
    router: Router,
    admin: Admin,
    db: Db,
    templates: Arc<Templates>,
) -> Router {
    // R1 commit #9 — strict-mailer boot guard. Runs BEFORE any
    // route registration so a misconfigured deployment fails
    // loudly at startup rather than registering recovery routes
    // against a production-unsafe default mailer
    // (`DESIGN_RECOVERY.md` §12.1). The check is structural: see
    // [`strict_mailer_guard_check`] for why we don't do
    // pointer-equality tricks against the default LogMailer.
    if let Err(msg) = strict_mailer_guard_check(&admin) {
        panic!("{msg}");
    }

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
    //
    // The middleware also resolves the operator's identity from the
    // session cookie BEFORE handing off, so the error page renders
    // with the same chrome (sidebar, top-bar actor, "Log out") the
    // operator was using when they hit the failing route. Without
    // this lookup the 404 page rendered as if the operator were
    // unauthenticated — a navigational dead-end documented as
    // `VISIBILITY_AUDIT.md` finding A2.
    let err_admin = ctx.admin.clone();
    let err_templates = ctx.templates.clone();
    let err_db = ctx.db.clone();
    let router = router.middleware(move |req, next| {
        let admin = err_admin.clone();
        let templates = err_templates.clone();
        let db = err_db.clone();
        Box::pin(async move {
            let is_admin_path = req.path().starts_with("/admin");
            // Capture the cookie header BEFORE moving `req` into
            // `next.run` so the error branch can re-resolve the
            // operator's identity. The auth middleware sits inside
            // this one; the request is consumed by `next.run` before
            // we get to the `Err` branch.
            let cookie_header = if is_admin_path {
                req.header("cookie").map(|s| s.to_string())
            } else {
                None
            };
            let result = next.run(req).await;
            match result {
                Ok(resp) => Ok(resp),
                Err(err) if is_admin_path => {
                    let identity = match cookie_header.as_deref() {
                        Some(cookie) => resolve_identity_for_error_page(&db, cookie).await,
                        None => None,
                    };
                    Ok(render::render_admin_error_response(
                        &admin,
                        &templates,
                        identity.as_ref(),
                        err.status(),
                        err.client_message().to_string(),
                    ))
                }
                Err(err) => Err(err),
            }
        })
    });

    // Read-only guard. When [`Admin::read_only`] is on, every
    // mutating verb (POST / PUT / PATCH / DELETE) under `/admin/*`
    // returns 403 — except a small allowlist for auth flows
    // (login / logout / reauth / MFA verify / password recovery /
    // own-session management). Mounted AFTER the error-renderer
    // above so the 403 gets the styled HTML page treatment instead
    // of plain text. Mounted BEFORE every route registration below
    // so the chain runs early and the actual handler never sees a
    // blocked mutation. Pass-through (`next.run(req)`) when the
    // admin isn't read-only — zero overhead for non-frozen
    // deployments.
    let ro_flag = ctx.admin.is_read_only();
    let ro_models = std::sync::Arc::new(ctx.admin.read_only_models.clone());
    let router = router.middleware(move |req, next| {
        let ro_models = ro_models.clone();
        Box::pin(async move {
            if req.path().starts_with("/admin")
                && is_mutating_method(req.method())
                && !is_read_only_writable_path(req.path())
            {
                // Whole-admin read-only takes precedence: every
                // project-data mutation is rejected.
                if ro_flag {
                    return Err(Error::Forbidden(
                        "This admin is currently in read-only mode. \
                         Project-data mutations are disabled until the operator \
                         turns read-only off."
                            .into(),
                    ));
                }
                // Per-model read-only: extract the `:admin_name`
                // segment and check the frozen set. A frozen slug
                // stays frozen even if the rest of the admin is
                // writable.
                if !ro_models.is_empty() {
                    if let Some(slug) = extract_admin_name(req.path()) {
                        if ro_models.contains(slug) {
                            return Err(Error::Forbidden(format!(
                                "Model `{slug}` is frozen (read-only). \
                                 Mutations on this model are disabled."
                            )));
                        }
                    }
                }
            }
            next.run(req).await
        })
    });

    // Embedded stylesheet + JS. The bytes are baked into the binary
    // so single-binary deploy is preserved. CSS/JS use `no-cache`
    // (revalidate every request) so theme + design tweaks roll out the
    // moment the binary restarts; fonts (next block) keep their long
    // immutable cache because their bytes never change per release.
    let router = router.get("/static/admin.css", |_req| async move {
        Ok(Response::new(
            hyper::StatusCode::OK,
            bytes::Bytes::from_static(ADMIN_CSS.as_bytes()),
        )
        .with_header("content-type", "text/css; charset=utf-8")
        .with_header("cache-control", "no-cache, must-revalidate"))
    });
    let router = router.get("/static/admin.js", |_req| async move {
        Ok(Response::new(
            hyper::StatusCode::OK,
            bytes::Bytes::from_static(ADMIN_JS.as_bytes()),
        )
        .with_header("content-type", "application/javascript; charset=utf-8")
        .with_header("cache-control", "no-cache, must-revalidate"))
    });

    // Self-hosted fonts. Cache aggressively: file contents are
    // immutable per build, so a 1-year cache is safe — the binary
    // ships a fresh copy on the next release.
    fn font_response(bytes: &'static [u8]) -> Response {
        Response::new(hyper::StatusCode::OK, bytes::Bytes::from_static(bytes))
            .with_header("content-type", "font/woff2")
            .with_header("cache-control", "public, max-age=31536000, immutable")
    }
    let router = router.get("/static/fonts/Geist-Variable.woff2", |_req| async move {
        Ok(font_response(FONT_GEIST))
    });
    let router = router.get(
        "/static/fonts/GeistMono-Variable.woff2",
        |_req| async move { Ok(font_response(FONT_GEIST_MONO)) },
    );
    let router = router.get("/static/fonts/Tajawal-Regular.woff2", |_req| async move {
        Ok(font_response(FONT_TAJAWAL_REG))
    });
    let router = router.get("/static/fonts/Tajawal-Medium.woff2", |_req| async move {
        Ok(font_response(FONT_TAJAWAL_MED))
    });
    let router = router.get("/static/fonts/Tajawal-Bold.woff2", |_req| async move {
        Ok(font_response(FONT_TAJAWAL_BOLD))
    });
    let router = router.get(
        "/static/fonts/NotoNaskhArabic-Variable.woff2",
        |_req| async move { Ok(font_response(FONT_NOTO_NASKH_AR)) },
    );
    let router = router.get("/static/fonts/InterVariable.woff2", |_req| async move {
        Ok(font_response(FONT_INTER))
    });
    let router = router.get(
        "/static/fonts/NotoSansThai-Variable.woff2",
        |_req| async move { Ok(font_response(FONT_NOTO_THAI)) },
    );
    let router = router.get(
        "/static/fonts/NotoSansDevanagari-Variable.woff2",
        |_req| async move { Ok(font_response(FONT_NOTO_DEVA)) },
    );
    let router = router.get(
        "/static/fonts/NotoSansJP-Regular.woff2",
        |_req| async move { Ok(font_response(FONT_NOTO_JP)) },
    );
    let router = router.get(
        "/static/fonts/NotoSansKR-Regular.woff2",
        |_req| async move { Ok(font_response(FONT_NOTO_KR)) },
    );
    let router = router.get(
        "/static/fonts/NotoSansSC-Regular.woff2",
        |_req| async move { Ok(font_response(FONT_NOTO_SC)) },
    );

    // Public: liveness / readiness probe. No auth — load
    // balancers and k8s probes don't carry session cookies.
    // Registered ahead of `/admin/:admin_name` so the route
    // can't be shadowed by a model literally named `healthz`.
    let c = ctx.clone();
    let router = router.get("/admin/healthz", move |_req| {
        let c = c.clone();
        async move { super::healthz::healthz(&c.db).await }
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

    // === R1 recovery routes ====================================
    //
    // MUST be registered BEFORE the `/admin/:admin_name` model
    // wildcards lower down — without that ordering, a request to
    // `/admin/forgot-password` would match `:admin_name =
    // "forgot-password"` and route into the model CRUD handler.
    //
    // Recovery state (the rate-limit buckets) is built once here
    // and cloned into each route closure so the buckets persist
    // for the process lifetime. No global / static / OnceLock —
    // the Arc lives in the closures.
    //
    // Strict-mailer boot guard already ran at the top of this fn
    // (would have panicked if misconfigured); reaching this block
    // means we have the operator's blessing to wire recovery.

    let recovery_state = Arc::new(super::recovery_handlers::RecoveryState::from_admin(
        &ctx.admin,
    ));

    let c = ctx.clone();
    let router = router.get("/admin/forgot-password", move |req| {
        let c = c.clone();
        async move { super::recovery_handlers::show_forgot_password(&c, &req).await }
    });

    let c = ctx.clone();
    let rs = recovery_state.clone();
    let router = router.post("/admin/forgot-password", move |req| {
        let c = c.clone();
        let rs = rs.clone();
        async move { super::recovery_handlers::do_forgot_password(&c, &rs, req).await }
    });

    let c = ctx.clone();
    let router = router.get("/admin/forgot-password/sent", move |req| {
        let c = c.clone();
        async move { super::recovery_handlers::show_forgot_password_sent(&c, &req).await }
    });

    let c = ctx.clone();
    let router = router.get("/admin/reset-password/:token", move |req| {
        let c = c.clone();
        async move {
            let token = req
                .param("token")
                .ok_or_else(|| Error::BadRequest("missing token".into()))?
                .to_string();
            super::recovery_handlers::show_reset_password(&c, &req, &token).await
        }
    });

    let c = ctx.clone();
    let rs = recovery_state.clone();
    let router = router.post("/admin/reset-password/:token", move |req| {
        let c = c.clone();
        let rs = rs.clone();
        async move {
            let token = req
                .param("token")
                .ok_or_else(|| Error::BadRequest("missing token".into()))?
                .to_string();
            super::recovery_handlers::do_reset_password(&c, &rs, req, &token).await
        }
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

    // Database browser — Developer-only schema explorer at
    // `/admin/db`. Read-only `information_schema` / `pg_catalog`
    // queries; no DDL, no row sampling. Registered before the
    // generic `/admin/:admin_name` so a model coincidentally named
    // `db` can't shadow it (also the static literal wins on tie).
    let c = ctx.clone();
    let router = router.get("/admin/db", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Developer).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::db_browser::show_db_browser(&c, ident, &req).await,
            }
        }
    });

    // Health dashboard — Administrator-only web counterpart to
    // `rustio doctor`. Runs the same DB probes the CLI does
    // (Postgres reachable, auth tables present, ≥1 active admin,
    // RUSTIO_SECRET_KEY shape). Distinct from `/admin/healthz`
    // which is the public liveness probe.
    let c = ctx.clone();
    let router = router.get("/admin/health", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::show_health(&c, ident, &req).await,
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

    // Self-service active-sessions listing (R0). Any logged-in user
    // (User-tier and above) can see their own active sessions.
    let c = ctx.clone();
    let router = router.get("/admin/account/sessions", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::show_account_sessions(&c, ident, &req).await,
            }
        }
    });

    // R1 commit #10 — active-sessions revoke buttons. All three
    // POST routes go through `auth::invalidate_sessions` (Doctrine
    // 22) and write `AuditEvent::SessionsRevokedSelf` per revoked
    // id. The `/revoke-others` and `/revoke-all` literal segments
    // sit at depth-4 while `:id/revoke` sits at depth-5, so segment
    // count alone disambiguates them — no explicit ordering
    // constraint between the three.
    let c = ctx.clone();
    let router = router.post("/admin/account/sessions/revoke-others", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::do_revoke_other_sessions(&c, ident, req).await,
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/account/sessions/revoke-all", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::do_revoke_all_sessions(&c, ident, req).await,
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/account/sessions/:id/revoke", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::do_revoke_session(&c, ident, req, id).await
                }
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

    // === R2 re-auth wall (R2 commit #11) ====================================
    //
    // Standalone wall: any authenticated user can promote their own
    // session into the elevated band by re-entering their password.
    // The handler validates `return_to` strictly (only `/admin*`
    // paths; see `admin_recovery_handlers::validate_return_to`).
    // Any role from User-tier upward.

    let c = ctx.clone();
    let router = router.get("/admin/reauth", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::admin_recovery_handlers::show_reauth(&c, ident, &req).await
                }
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/reauth", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::admin_recovery_handlers::do_reauth(&c, ident, req).await
                }
            }
        }
    });

    // === R2 forced password rotation (R2 commit #12) ========================
    //
    // The `must_change_password` interstitial is the only writeable
    // surface a user can reach while their flag is TRUE. The path is
    // on `MUST_CHANGE_WHITELIST`; the `login_guard` redirect therefore
    // skips it (otherwise the rotation would be unreachable). Role::User
    // matches: any authenticated user can be forced to rotate, even a
    // User-tier account that can't access the dashboard.

    let c = ctx.clone();
    let router = router.get("/admin/must-change-password", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::admin_recovery_handlers::show_must_change_password(&c, ident, &req).await
                }
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/must-change-password", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    super::admin_recovery_handlers::do_must_change_password(&c, ident, req).await
                }
            }
        }
    });

    // === R3 MFA surface (R3 commits #12-#15) ================================
    //
    // Eight routes:
    //   /admin/mfa/verify                        — login second factor (#12)
    //   /admin/account/mfa/enroll                — provision + confirm (#13)
    //   /admin/account/mfa/regenerate-codes      — atomic batch swap   (#14)
    //   /admin/account/mfa/disable               — self-disable        (#15)
    //
    // All gated by `Role::User` — every authenticated user can manage
    // their own MFA. The /admin/mfa/verify path is on
    // `MFA_VERIFY_WHITELIST`; the enrol path is on
    // `MFA_ENROLL_WHITELIST` — so `login_guard` does NOT redirect
    // away from these routes even when the user is in the pending-
    // verify or required-enrol state. Otherwise the interstitial
    // pages would be unreachable.

    // --- /admin/mfa/verify (R3 commit #12) ---
    let c = ctx.clone();
    let router = router.get("/admin/mfa/verify", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::show_verify(&c, ident, &req).await,
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/mfa/verify", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::do_verify(&c, ident, req).await,
            }
        }
    });

    // --- /admin/account/mfa/enroll (R3 commit #13) ---
    let c = ctx.clone();
    let router = router.get("/admin/account/mfa/enroll", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::show_enroll(&c, ident, &req).await,
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/account/mfa/enroll", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::do_enroll(&c, ident, req).await,
            }
        }
    });

    // --- /admin/account/mfa/regenerate-codes (R3 commit #14) ---
    let c = ctx.clone();
    let router = router.get("/admin/account/mfa/regenerate-codes", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::show_regenerate(&c, ident, &req).await,
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/account/mfa/regenerate-codes", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::do_regenerate(&c, ident, req).await,
            }
        }
    });

    // --- /admin/account/mfa/disable (R3 commit #15) ---
    let c = ctx.clone();
    let router = router.get("/admin/account/mfa/disable", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::show_disable(&c, ident, &req).await,
            }
        }
    });

    let c = ctx.clone();
    let router = router.post("/admin/account/mfa/disable", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::User).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => super::mfa_handlers::do_disable(&c, ident, req).await,
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

    // === R2 admin-driven recovery routes ====================================
    //
    // Registered alongside the existing `/admin/users/:id/...` cluster
    // (per `DESIGN_R2_ORGANISATIONAL.md` §7.2 — user-related cluster
    // contiguous). All gated `Role::Administrator`; the cross-rank
    // safety check + the re-auth wall are enforced INSIDE the
    // handlers (commits #15 / #16) so a Supervisor probe doesn't even
    // reach the form.
    //
    // Insertion-order note: these are 4-segment routes, so the
    // 3-segment `/admin/users/:id` read-only view further down doesn't
    // conflict regardless of order. Placing them before the 3-segment
    // view keeps the user routes lexically clustered.

    // GET /admin/users/:id/reset-password — admin reset form (R2 #15).
    let c = ctx.clone();
    let router = router.get("/admin/users/:id/reset-password", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::show_admin_reset_password(&c, ident, id, &req)
                        .await
                }
            }
        }
    });

    // POST /admin/users/:id/reset-password — apply admin reset (R2 #15).
    let c = ctx.clone();
    let router = router.post("/admin/users/:id/reset-password", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::do_admin_reset_password(&c, ident, id, req)
                        .await
                }
            }
        }
    });

    // GET /admin/users/:id/lock — lock confirmation form (R2 #16).
    let c = ctx.clone();
    let router = router.get("/admin/users/:id/lock", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::show_lock_user(&c, ident, id, &req).await
                }
            }
        }
    });

    // POST /admin/users/:id/lock — apply manual lock (R2 #16).
    let c = ctx.clone();
    let router = router.post("/admin/users/:id/lock", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::do_lock_user(&c, ident, id, req).await
                }
            }
        }
    });

    // GET /admin/users/:id/unlock — unlock confirmation form (R2 #16).
    let c = ctx.clone();
    let router = router.get("/admin/users/:id/unlock", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::show_unlock_user(&c, ident, id, &req).await
                }
            }
        }
    });

    // POST /admin/users/:id/unlock — clear lock (R2 #16).
    let c = ctx.clone();
    let router = router.post("/admin/users/:id/unlock", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::do_unlock_user(&c, ident, id, req).await
                }
            }
        }
    });

    // GET /admin/users/:id/revoke-sessions — revoke confirmation form
    // (R2 #16).
    let c = ctx.clone();
    let router = router.get("/admin/users/:id/revoke-sessions", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::show_admin_revoke_sessions(&c, ident, id, &req)
                        .await
                }
            }
        }
    });

    // POST /admin/users/:id/revoke-sessions — revoke all sessions (R2 #16).
    let c = ctx.clone();
    let router = router.post("/admin/users/:id/revoke-sessions", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    super::admin_recovery_handlers::do_admin_revoke_sessions(&c, ident, id, req)
                        .await
                }
            }
        }
    });

    // POST /admin/users/:id/sessions/:session_id/revoke — revoke
    // ONE specific session (per-row affordance on the user_view
    // sessions tab). Narrower than `revoke-sessions` above: no
    // confirmation page, no reason field, no re-auth wall. Same
    // Administrator role gate; the handler enforces cross-rank +
    // session-ownership + same-actor-session refusal in-line.
    let c = ctx.clone();
    let router = router.post("/admin/users/:id/sessions/:session_id/revoke", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Administrator).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let user_id = parse_id(req.param("id"))?;
                    let session_id = parse_id(req.param("session_id"))?;
                    super::admin_recovery_handlers::do_admin_revoke_one_session(
                        &c, ident, user_id, session_id, req,
                    )
                    .await
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
                    let viewing_session_id = match req
                        .header("cookie")
                        .and_then(crate::auth::session_token_from_cookie)
                    {
                        Some(token) => crate::auth::current_session_id(&ac.db, &token)
                            .await
                            .ok()
                            .flatten(),
                        None => None,
                    };
                    super::builtin::show_user_view(
                        &ac,
                        ident,
                        id,
                        handlers::csrf_token(&req),
                        tab,
                        page,
                        viewing_session_id,
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

    // Uploaded-file serve. `Admin::uploads_dir` is the storage
    // root; the route resolves `<rel>` under it with a canonical-
    // path guard so a hand-edited URL can't reach files outside.
    // Identity-gated at Staff (anyone with admin access can see
    // uploaded files in v1; per-row visibility checks are a
    // future iteration). Registered before any generic
    // `/admin/:admin_name/…` route so the literal `uploads`
    // segment can't be shadowed by a model named `uploads`.
    let c = ctx.clone();
    let router = router.get("/admin/uploads/:filename", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Staff).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let filename = req
                        .param("filename")
                        .map(str::to_string)
                        .unwrap_or_default();
                    handlers::serve_upload(&c, ident, &filename, req).await
                }
            }
        }
    });

    // FK autocomplete lookup. Registered before any generic
    // `/admin/:admin_name/…` route so a literal `_lookup` segment
    // can't be shadowed by a model named `_lookup`. The endpoint
    // is gated on `view` permission for the target model — same
    // surface as the user reading the target's list page.
    let c = ctx.clone();
    let router = router.get("/admin/_lookup/:admin_name", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "view")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::lookup_model(&c, ident, &name, req).await,
            }
        }
    });

    // Global cross-model search (`⌘K` palette). Registered before
    // any generic `/admin/:admin_name/…` route so a literal `_search`
    // segment can't be shadowed by a model named `_search`. Role
    // gate is `Staff` — anyone who can reach the admin can open the
    // palette; the handler itself filters results to models the
    // operator can `view`, so each operator sees only what their
    // sidebar already lets them reach.
    let c = ctx.clone();
    let router = router.get("/admin/_search", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Staff).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::search_models(&c, ident, req).await,
            }
        }
    });

    // Auto-generated OpenAPI 3.0 spec. Registered before the
    // generic `/admin/:admin_name/…` patterns so a literal `apis`
    // / `openapi.json` segment can't be shadowed by a model named
    // `apis`. Role gate: Staff. The doc itself only lists endpoint
    // shapes — clients still need per-model `view` to call the
    // endpoints it describes.
    let c = ctx.clone();
    let router = router.get("/admin/apis/openapi.json", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Staff).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(_) => {
                    let spec = super::openapi::build_spec(&c.admin);
                    super::json_api::json_response(spec)
                }
            }
        }
    });

    // Human-readable HTML index for the API surface. Sibling of
    // the openapi.json endpoint above; same Staff gate. Lists
    // every registered model's endpoint table + field shapes so
    // operators can read the surface without opening a JSON
    // viewer.
    let c = ctx.clone();
    let router = router.get("/admin/apis", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Staff).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::show_apis_index(&c, ident, &req).await,
            }
        }
    });

    // Interactive playground (read-only preview): pick a model,
    // build a list-page query, fetch the JSON envelope in-page.
    // Mounted before the generic /admin/:admin_name pattern so
    // `apis` / `playground` segments don't collide with a model
    // named `playground`. Same Staff gate as the API index;
    // operators still need per-model `view` to receive non-empty
    // results.
    let c = ctx.clone();
    let router = router.get("/admin/apis/playground", move |req| {
        let c = c.clone();
        async move {
            match role_guard(&c, &req, Role::Staff).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::show_apis_playground(&c, ident, &req).await,
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

    // CSV export — must be registered before `/admin/:admin_name/:id/…`
    // patterns so the literal `export.csv` second segment isn't
    // shadowed by a numeric `:id` route. Same `view` permission gate
    // as the list page; `find_project_entry` blocks core entries.
    let c = ctx.clone();
    let router = router.get("/admin/:admin_name/export.csv", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "view")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::export_model_csv(&c, ident, &name, req).await,
            }
        }
    });

    // Saved-filter create — POST /admin/:admin_name/saved_filters.
    // Same `view` gate as the list page: any operator who can
    // reach the list can bookmark its state. Registered before the
    // generic /:admin_name/:id/edit route so the literal
    // `saved_filters` segment isn't shadowed by a numeric `:id`.
    let c = ctx.clone();
    let router = router.post("/admin/:admin_name/saved_filters", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "view")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::do_save_filter(&c, ident, &name, req).await,
            }
        }
    });

    // Saved-filter delete — POST /admin/:admin_name/saved_filters/:id/delete.
    // SQL scope-locks to identity.user_id so the route gate can
    // stay `view`; an operator can only delete their own bookmarks.
    let c = ctx.clone();
    let router = router.post("/admin/:admin_name/saved_filters/:id/delete", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let id = parse_id(req.param("id"))?;
            let perm = perm_for(&c, &name, "view")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::do_delete_filter(&c, ident, &name, id, req).await,
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

    // JSON detail. `GET /admin/:admin_name/:id` returns the row
    // as JSON when the client asked for it (Accept header or
    // `?format=json`); otherwise redirects to the HTML edit
    // form so a browser landing on the bare URL ends up where
    // it expects. View permission, same as the list endpoint
    // which JSON-list is already gated on.
    let c = ctx.clone();
    let router = router.get("/admin/:admin_name/:id", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "view")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::show_object_json(&c, ident, &name, id, &req).await
                }
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
    let router = router.post("/admin/:admin_name/:id/delete", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "delete")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    let id = parse_id(req.param("id"))?;
                    handlers::do_delete(&c, ident, &name, req, id).await
                }
            }
        }
    });

    // Bulk delete — same permission gate as the per-row delete.
    // Two-step flow: first POST renders the confirm page, second POST
    // (with `_confirmed=1`) executes. See `handlers::handle_bulk_delete`
    // for the full contract.
    let c = ctx.clone();
    let router = router.post("/admin/:admin_name/bulk_delete", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let perm = perm_for(&c, &name, "delete")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => handlers::handle_bulk_delete(&c, ident, &name, &req).await,
            }
        }
    });

    // Project-defined bulk actions. Permission gated on `change` —
    // bulk actions modify rows but don't delete them (delete has its
    // own route). Project-side guard against further write-vs-read
    // distinctions belongs inside `execute_bulk_action`.
    let c = ctx.clone();
    router.post("/admin/:admin_name/bulk/:action", move |req| {
        let c = c.clone();
        async move {
            let name = model_name_from_req(&req)?;
            let action = req
                .param("action")
                .ok_or_else(|| Error::BadRequest("missing bulk action name".into()))?
                .to_string();
            let perm = perm_for(&c, &name, "change")?;
            match perm_guard(&c, &req, &perm).await? {
                Guard::Redirect(r) => Ok(r),
                Guard::Allow(ident) => {
                    handlers::handle_bulk_action(&c, ident, &name, &action, &req).await
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
            must_change_password: false,
            mfa_enabled: false,
            trust_level: crate::auth::SessionTrust::Authenticated,
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

    // ---- strict_mailer_guard_check ----------------------------------------

    /// Default `Admin::new()` doesn't override the mailer AND
    /// doesn't enable strict mode — the guard passes.
    #[test]
    fn strict_mailer_guard_passes_for_default_admin() {
        let admin = super::super::types::Admin::new();
        assert!(strict_mailer_guard_check(&admin).is_ok());
    }

    /// Strict-mailer mode + default LogMailer = boot guard fires.
    /// The error message is operator-actionable.
    #[test]
    fn strict_mailer_guard_fails_when_required_but_default_mailer() {
        use crate::auth::DefaultRecoveryPolicy;
        let admin = super::super::types::Admin::new().recovery_policy(std::sync::Arc::new(
            DefaultRecoveryPolicy::new().with_strict_mailer_required(true),
        ));
        let err = strict_mailer_guard_check(&admin).expect_err("guard should fail");
        assert!(
            err.contains("strict_mailer_required"),
            "error message must name the policy method: {err}"
        );
        assert!(
            err.contains("Admin::mailer"),
            "error message must direct the operator to the fix: {err}"
        );
    }

    /// Strict-mailer mode + project-supplied mailer = guard passes.
    /// Note: the explicit override flips the flag even when the
    /// supplied value happens to be another LogMailer — the
    /// operator's intent is what matters, not the concrete type.
    #[test]
    fn strict_mailer_guard_passes_when_mailer_was_explicitly_overridden() {
        use crate::auth::DefaultRecoveryPolicy;
        use crate::email::LogMailer;
        let admin = super::super::types::Admin::new()
            .recovery_policy(std::sync::Arc::new(
                DefaultRecoveryPolicy::new().with_strict_mailer_required(true),
            ))
            .mailer(std::sync::Arc::new(LogMailer));
        assert!(strict_mailer_guard_check(&admin).is_ok());
    }

    /// Project NOT in strict mode + default LogMailer = passes
    /// (dev / CI / testing baseline).
    #[test]
    fn strict_mailer_guard_passes_when_strict_mode_disabled() {
        let admin = super::super::types::Admin::new();
        assert!(strict_mailer_guard_check(&admin).is_ok());
    }

    // ---- must-change-password whitelist (R2 commit #13) --------------------

    #[test]
    fn whitelist_accepts_the_three_locked_paths() {
        // Locked-decision per DESIGN_R2_ORGANISATIONAL.md §12.
        assert!(super::is_must_change_whitelisted_path(
            "/admin/must-change-password"
        ));
        assert!(super::is_must_change_whitelisted_path("/admin/logout"));
        assert!(super::is_must_change_whitelisted_path(
            "/admin/account/sessions"
        ));
    }

    #[test]
    fn whitelist_rejects_subpaths_of_account_sessions() {
        // Sub-paths of /admin/account/sessions (revoke buttons) are
        // intentionally NOT whitelisted — a user being forced to
        // rotate may VIEW their sessions but must finish the
        // rotation before revoking siblings.
        assert!(!super::is_must_change_whitelisted_path(
            "/admin/account/sessions/revoke"
        ));
        assert!(!super::is_must_change_whitelisted_path(
            "/admin/account/sessions/revoke-others"
        ));
        assert!(!super::is_must_change_whitelisted_path(
            "/admin/account/sessions/"
        ));
    }

    #[test]
    fn whitelist_rejects_other_admin_paths() {
        for path in [
            "/admin",
            "/admin/",
            "/admin/users",
            "/admin/users/42",
            "/admin/login",
            "/admin/password_change",
            "/admin/forgot-password",
            "/admin/reauth",
            "/admin/must-change-password/", // trailing slash → not exact
        ] {
            assert!(
                !super::is_must_change_whitelisted_path(path),
                "expected reject for {path:?}"
            );
        }
    }

    #[test]
    fn whitelist_rejects_paths_outside_admin_surface() {
        for path in ["/", "/login", "/static/admin.css", "/api"] {
            assert!(
                !super::is_must_change_whitelisted_path(path),
                "expected reject for {path:?}"
            );
        }
    }

    // ---- Read-only writable-path allowlist ----------------------

    #[test]
    fn read_only_allows_auth_flow_exact_paths() {
        for path in [
            "/admin/login",
            "/admin/logout",
            "/admin/reauth",
            "/admin/forgot-password",
            "/admin/mfa/verify",
            "/admin/must-change-password",
            "/admin/password_change",
        ] {
            assert!(
                super::is_read_only_writable_path(path),
                "auth path {path:?} must be writable in read-only mode"
            );
        }
    }

    #[test]
    fn read_only_allows_prefix_paths() {
        // Token-bearing recovery URL, own-session and own-MFA
        // management — all carry dynamic segments, allowlisted by
        // prefix not by exact match.
        for path in [
            "/admin/reset-password/abc123",
            "/admin/reset-password/abc123/whatever",
            "/admin/account/sessions/42/revoke",
            "/admin/account/sessions/revoke-all",
            "/admin/account/mfa/enroll",
            "/admin/account/mfa/disable",
        ] {
            assert!(
                super::is_read_only_writable_path(path),
                "prefix-allowlisted path {path:?} must be writable"
            );
        }
    }

    #[test]
    fn read_only_blocks_project_data_mutations() {
        // Project CRUD, bulk actions, admin-driven user lifecycle
        // — none of these should slip through.
        for path in [
            "/admin/posts/new",
            "/admin/posts/42/edit",
            "/admin/posts/42/delete",
            "/admin/posts/bulk_delete",
            "/admin/posts/bulk/archive",
            "/admin/users/new",
            "/admin/users/42/edit",
            "/admin/users/42/reset-password",
            "/admin/users/42/lock",
            "/admin/users/42/sessions/99/revoke",
            "/admin/groups/new",
            "/admin/groups/42/delete",
        ] {
            assert!(
                !super::is_read_only_writable_path(path),
                "data-mutation path {path:?} must be blocked in read-only mode"
            );
        }
    }

    #[test]
    fn read_only_blocks_random_paths_outside_admin_surface() {
        // Paths outside /admin/* don't reach this helper in
        // production (the middleware checks `starts_with("/admin")`
        // first), but the helper itself must still say "not
        // writable" so the policy is self-consistent.
        for path in ["/", "/login", "/static/admin.css", "/api/v1/posts"] {
            assert!(
                !super::is_read_only_writable_path(path),
                "non-admin path {path:?} must not be writable"
            );
        }
    }

    #[test]
    fn extract_admin_name_parses_slug_segment() {
        assert_eq!(super::extract_admin_name("/admin/posts"), Some("posts"));
        assert_eq!(super::extract_admin_name("/admin/posts/"), Some("posts"));
        assert_eq!(
            super::extract_admin_name("/admin/posts/42/edit"),
            Some("posts")
        );
        assert_eq!(
            super::extract_admin_name("/admin/users/42/sessions/99/revoke"),
            Some("users")
        );
    }

    #[test]
    fn extract_admin_name_rejects_root_reserved_and_non_admin() {
        // Root /admin and trailing slash → no model slug.
        assert_eq!(super::extract_admin_name("/admin/"), None);
        assert_eq!(super::extract_admin_name("/admin"), None);
        // Underscore-prefixed slugs are framework-reserved (`_search`,
        // `_lookup`, `healthz` etc.) — never project models.
        assert_eq!(super::extract_admin_name("/admin/_search"), None);
        assert_eq!(super::extract_admin_name("/admin/_lookup/posts"), None);
        // Paths outside /admin.
        assert_eq!(super::extract_admin_name("/login"), None);
        assert_eq!(super::extract_admin_name("/static/admin.css"), None);
    }

    #[test]
    fn read_only_model_builder_and_accessor_round_trip() {
        let admin = super::super::types::Admin::new()
            .read_only_model("archive_posts")
            .read_only_model("legacy_invoices");
        assert!(admin.is_model_read_only("archive_posts"));
        assert!(admin.is_model_read_only("legacy_invoices"));
        assert!(!admin.is_model_read_only("posts"));
        // Whole-admin flag stays independent.
        assert!(!admin.is_read_only());
    }

    #[test]
    fn is_mutating_method_recognises_write_verbs() {
        use hyper::Method;
        for m in [Method::POST, Method::PUT, Method::PATCH, Method::DELETE] {
            assert!(super::is_mutating_method(&m), "{m} must be mutating");
        }
        for m in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert!(!super::is_mutating_method(&m), "{m} must not be mutating");
        }
    }
}
