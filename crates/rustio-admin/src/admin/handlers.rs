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

use std::collections::{HashMap, HashSet};
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
    // internal:
    pub(crate) fn new(admin: Arc<Admin>, db: Db, templates: Arc<Templates>) -> Self {
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
    // The login page surfaces two non-error flashes:
    //
    //  - `?logout=1`             — the user just signed out (`do_logout`
    //                              redirects here).
    //  - `?password_reset=success` — the user just consumed a recovery
    //                              link in `do_reset_password`
    //                              (R1 commit #8).
    //
    // Logic factored into the pure helper [`login_flash_for_query`] so
    // the precedence + wording can be unit-tested without constructing
    // a Request.
    let q = req.query();
    let flash = login_flash_for_query(q.get("logout").is_some(), q.get("password_reset"));
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

/// Pure verdict for the login page's flash banner.
///
/// Precedence: `logout` wins over `password_reset` when both query
/// flags are present (matches user mental model — the most recent
/// action is logout, since password-reset success ALSO routes through
/// /admin/login but doesn't carry a logout flag). Unknown
/// `password_reset` values fall through silently.
pub(super) fn login_flash_for_query(
    logout_present: bool,
    password_reset: Option<&str>,
) -> Option<render::FlashCtx> {
    if logout_present {
        return Some(render::FlashCtx {
            kind: "success",
            message: "You've been signed out.".to_string(),
        });
    }
    if password_reset == Some("success") {
        return Some(render::FlashCtx {
            kind: "success",
            message: "Your password has been updated. Sign in with your new password.".to_string(),
        });
    }
    None
}

#[cfg(test)]
mod login_flash_tests {
    use super::login_flash_for_query;

    #[test]
    fn no_query_params_produces_no_flash() {
        assert!(login_flash_for_query(false, None).is_none());
    }

    #[test]
    fn logout_flag_produces_signed_out_flash() {
        let f = login_flash_for_query(true, None).expect("logout produces flash");
        assert_eq!(f.kind, "success");
        assert!(
            f.message.contains("signed out"),
            "logout flash missing expected wording: {}",
            f.message
        );
    }

    #[test]
    fn password_reset_success_produces_locked_flash_copy() {
        let f = login_flash_for_query(false, Some("success")).expect("reset success → flash");
        assert_eq!(f.kind, "success");
        // Locked copy from DESIGN_RECOVERY.md commit #9 spec.
        assert_eq!(
            f.message,
            "Your password has been updated. Sign in with your new password."
        );
    }

    #[test]
    fn unknown_password_reset_value_falls_through_silently() {
        // Defensive: only the literal "success" triggers the banner.
        // "garbage" and arbitrary user input must not produce a flash.
        assert!(login_flash_for_query(false, Some("garbage")).is_none());
        assert!(login_flash_for_query(false, Some("")).is_none());
        assert!(login_flash_for_query(false, Some("Success")).is_none()); // case-sensitive
    }

    #[test]
    fn logout_takes_precedence_over_password_reset_when_both_present() {
        // Edge case: query carries both flags. The most recent action
        // is logout (post-reset success ALSO routes through login but
        // doesn't itself carry a logout flag), so logout wording wins.
        let f = login_flash_for_query(true, Some("success")).expect("flash present");
        assert!(
            f.message.contains("signed out"),
            "expected logout wording when both flags present: {}",
            f.message
        );
    }
}

/// Fire-and-forget audit emission for a single login attempt. Wraps
/// `audit::record` with the conventional metadata shape the History
/// page expects (ip, correlation_id, optional `reason` discriminator).
/// Errors during the audit write itself are logged but never block
/// the login flow — same posture as every other auth-path audit
/// emission (e.g. the auto-lock branch).
async fn record_login_audit(
    ctx: &AdminCtx,
    req: &Request,
    user_id: i64,
    user_email: &str,
    event: audit::AuditEvent,
    reason: Option<&'static str>,
    mfa_pending: bool,
) {
    let ip = super::builtin::client_ip(req);
    let cid = super::builtin::correlation_id_from(req);
    let mut metadata = serde_json::Map::new();
    if let Some(r) = reason {
        metadata.insert("reason".to_string(), serde_json::Value::String(r.into()));
    }
    if mfa_pending {
        metadata.insert("mfa_pending".to_string(), serde_json::Value::Bool(true));
    }
    let summary = match event {
        audit::AuditEvent::LoginSucceeded => format!("signed in: {user_email}"),
        audit::AuditEvent::LoginFailed => match reason {
            Some(r) => format!("failed sign-in ({r}): {user_email}"),
            None => format!("failed sign-in: {user_email}"),
        },
        _ => format!("login audit: {user_email}"),
    };
    let res = audit::record(
        &ctx.db,
        audit::LogEntry {
            user_id,
            action_type: audit::ActionType::Update,
            model_name: "users",
            object_id: user_id,
            ip_address: ip.as_deref(),
            summary,
            correlation_id: cid.as_deref(),
            session_id: None,
            metadata: if metadata.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(metadata))
            },
            // Login is self-driven: the actor IS the subject. Same
            // convention as `PasswordChangedSelf` etc.
            actor_user_id: None,
            event: Some(event),
        },
    )
    .await;
    if let Err(e) = res {
        log::warn!("audit::record (login event) failed user_id={user_id}: {e}");
    }
}

pub(crate) async fn do_login(ctx: &AdminCtx, req: Request) -> Result<Response> {
    let form = req.form()?;
    let email = form.required("email")?;
    let password = form.required("password")?;

    // Per `DESIGN_R2_ORGANISATIONAL.md` §3.3 + §9.1: every failure
    // mode (no such user, inactive account, currently locked, wrong
    // password) returns the same uniform 401 — no enumeration leak.
    // The locked branch must respond identically to the wrong-password
    // branch so an attacker can't tell whether they hit the throttle.
    let uniform_unauthorized = || -> Result<Response> {
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
    };

    // 1. Look up the user (don't verify password yet).
    let user = match auth::find_user_by_email(&ctx.db, email).await? {
        Some(u) => u,
        None => return uniform_unauthorized(),
    };

    // 2. Inactive accounts are indistinguishable from wrong-password.
    //    No throttle work — inactive is an admin-set flag, not a
    //    failure-rate signal.
    if !user.is_active {
        record_login_audit(
            ctx,
            &req,
            user.id,
            &user.email,
            audit::AuditEvent::LoginFailed,
            Some("inactive"),
            false,
        )
        .await;
        return uniform_unauthorized();
    }

    // 3. Lockout check FIRST (before password verify) so a locked
    //    account doesn't pay Argon2 cost and so the response time is
    //    uniform with the wrong-password branch.
    use crate::auth::recovery_admin::{
        check_account_lockout, record_failed_login, record_successful_login, LockState,
        ThrottleOutcome,
    };
    if let LockState::Locked { .. } = check_account_lockout(&ctx.db, user.id).await? {
        record_login_audit(
            ctx,
            &req,
            user.id,
            &user.email,
            audit::AuditEvent::LoginFailed,
            Some("locked"),
            false,
        )
        .await;
        return uniform_unauthorized();
    }

    // 4. Verify password. On failure: bump the throttle counter; emit
    //    AccountLocked audit if THIS failure tripped the threshold.
    if !auth::verify_password(password, &user.password_hash) {
        let throttle = ctx.admin.active_recovery_policy().login_throttle();
        let outcome = record_failed_login(&ctx.db, user.id, throttle).await?;
        if let ThrottleOutcome::JustLocked { count, until } = outcome {
            // Auto-throttle: actor_user_id = None, LogEntry::user_id =
            // affected user (closest reasonable subject per §5.2).
            // Doctrine 22: AccountLocked auto does NOT revoke
            // sessions — the lockout check at the next request handles
            // refusal. See §13 locked-decision.
            let ip = super::builtin::client_ip(&req);
            let cid = super::builtin::correlation_id_from(&req);
            let metadata = serde_json::json!({
                "via": "auto_throttle",
                "failed_count": count,
                "until": until,
                "reason": format!(
                    "auto-throttle: {count} failed logins within {} minutes",
                    throttle.window_minutes,
                ),
            });
            let _ = audit::record(
                &ctx.db,
                audit::LogEntry {
                    user_id: user.id,
                    action_type: audit::ActionType::Update,
                    model_name: "users",
                    object_id: user.id,
                    ip_address: ip.as_deref(),
                    summary: format!(
                        "auto-throttle locked user {} after {count} failed logins",
                        user.email,
                    ),
                    correlation_id: cid.as_deref(),
                    session_id: None,
                    metadata: Some(metadata),
                    actor_user_id: None,
                    event: Some(audit::AuditEvent::AccountLocked),
                },
            )
            .await;
        }
        // Audit the individual failed attempt regardless of whether
        // it tripped the throttle. The AccountLocked row (when
        // emitted) captures the cumulative state; the LoginFailed
        // row captures *this* attempt for the timeline view.
        record_login_audit(
            ctx,
            &req,
            user.id,
            &user.email,
            audit::AuditEvent::LoginFailed,
            Some("wrong_password"),
            false,
        )
        .await;
        return uniform_unauthorized();
    }

    // 5. Success — reset throttle counter, mint session, set cookie.
    //    The forced-rotation gate (must_change_password) runs at the
    //    next request inside `login_guard` (R2 commit #13), so the
    //    cookie is set unconditionally here.
    record_successful_login(&ctx.db, user.id).await?;
    let token = auth::create_session(&ctx.db, user.id).await?;
    let cookie = format!(
        "{}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=1209600",
        auth::SESSION_COOKIE
    );

    // 6. R3 — MFA branch. If the user has MFA enrolled, redirect
    //    to `/admin/mfa/verify` instead of `/admin`. The session
    //    row is already minted with `trust_level = 'authenticated'`;
    //    R3's `verify_totp_for_user` (commit #7) +
    //    `promote_session_to_mfa_verified` (commit #11) rotate it
    //    to `mfa_verified` once the second factor lands. The
    //    `login_guard` extension in commit #18 enforces that
    //    `mfa_enabled && trust_level != mfa_verified` sessions
    //    can only reach a tiny whitelist
    //    (`/admin/mfa/verify`, `/admin/logout`,
    //    `/admin/account/sessions`).
    //
    //    Per `DESIGN_R3_MFA.md` §4.2, the framework does NOT add a
    //    `pending_mfa` trust-level variant or a `metadata.awaiting_mfa`
    //    column. The combination of `identity.mfa_enabled = TRUE`
    //    AND `trust_level = 'authenticated'` IS the pending-MFA
    //    signal; no extra schema needed.
    let redirect_to = if user.mfa_enabled {
        "/admin/mfa/verify"
    } else {
        "/admin"
    };
    // Audit AFTER the session row + cookie are committed so a
    // crash between the two leaves no orphan audit row claiming a
    // session that doesn't exist. `mfa_pending = true` distinguishes
    // "first factor cleared, MFA challenge pending" from "fully
    // authenticated" in the History view.
    record_login_audit(
        ctx,
        &req,
        user.id,
        &user.email,
        audit::AuditEvent::LoginSucceeded,
        None,
        user.mfa_enabled,
    )
    .await;
    Ok(Response::redirect(redirect_to).with_header("set-cookie", cookie))
}

pub(crate) async fn do_logout(ctx: &AdminCtx, req: Request) -> Result<Response> {
    // Logout routes through the centralized invalidate_sessions API
    // (via `logout_session`) so the row is soft-revoked with
    // `revoked_reason = 'logout'` rather than hard-deleted. This keeps
    // the audit trail intact and uses the single legitimate writer of
    // `revoked_at` (doctrine 22).
    if let Some(cookie) = req.header("cookie") {
        if let Some(token) = auth::session_token_from_cookie(cookie) {
            auth::logout_session(&ctx.db, &token).await?;
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
    let recent_actions = audit::recent(&ctx.db, 10, None, None, None)
        .await
        .unwrap_or_default();
    // Approximate row counts via `pg_class.reltuples` — one batched
    // query for every registered model. Failures are non-fatal: the
    // dashboard renders with `~ 0` placeholders rather than 500ing.
    let row_estimates = fetch_dashboard_row_estimates(ctx).await;
    // Per-model "new this week" KPI — exact count, not an estimate.
    // Only emitted for models that declare a `created_at` field;
    // skipped silently for everything else. One query per model in
    // sequence — fast on indexed `created_at` columns, and the
    // dashboard renders with the row already cached by the row-
    // estimate pass above so the round-trips are cheap.
    let new_this_week = fetch_dashboard_new_this_week(ctx).await;
    // 7-day audit-activity sparkline. Pure time-series, no FK
    // joins — fast enough to run inline on every dashboard render
    // even on large `rustio_admin_actions` tables (the date range
    // is selective). Failure-soft: empty vec on error so the
    // dashboard still renders.
    let activity_sparkline = fetch_activity_last_7_days(ctx).await;
    let dash = render::dashboard_ctx(
        &identity,
        &ctx.admin,
        recent_actions,
        csrf_token(req),
        &row_estimates,
        &new_this_week,
        activity_sparkline,
    );
    let body = ctx.templates.render("admin/index.html", &dash)?;
    Ok(Response::html(body))
}

/// Fetch `pg_class.reltuples` for every registered project model's
/// table in a single round-trip. Returns a `HashMap<&'static str,
/// i64>` keyed by table name. Failures (DB hiccup, schema not yet
/// migrated, etc.) log a warning and return an empty map — the
/// dashboard then renders zero placeholders rather than failing
/// the whole page.
async fn fetch_dashboard_row_estimates(ctx: &AdminCtx) -> HashMap<&'static str, i64> {
    let project_tables: Vec<&'static str> = ctx
        .admin
        .entries()
        .iter()
        .filter(|e| !e.core)
        .map(|e| e.table)
        .collect();
    if project_tables.is_empty() {
        return HashMap::new();
    }
    let table_strs: Vec<String> = project_tables.iter().map(|t| t.to_string()).collect();
    let rows = match sqlx::query_as::<_, (String, i64)>(
        "SELECT c.relname, c.reltuples::bigint
         FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = 'public'
           AND c.relkind = 'r'
           AND c.relname = ANY($1)",
    )
    .bind(&table_strs)
    .fetch_all(ctx.db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("dashboard row estimates: {e}");
            return HashMap::new();
        }
    };
    let mut out: HashMap<&'static str, i64> = HashMap::with_capacity(rows.len());
    for (name, estimate) in rows {
        // Map the dynamic string back to the static `&str` from the
        // entries slice so the helper output keys match the caller's
        // expectations.
        if let Some(static_name) = project_tables.iter().find(|t| **t == name) {
            out.insert(*static_name, estimate.max(0));
        }
    }
    out
}

/// Fetch the per-model "new this week" count for every registered
/// project model that declares a `created_at` field. One sequential
/// query per qualifying model — `SELECT COUNT(*) FROM <table> WHERE
/// created_at > NOW() - INTERVAL '7 days'`. The table name comes
/// from `entry.table` (static identifier baked at compile time) so
/// the query is format-interpolation-safe.
///
/// Models without `created_at` are skipped — the map carries no
/// entry for them, and the template falls back to showing only the
/// row total. Failures on individual models are logged and skipped
/// so one broken table doesn't take down the whole dashboard.
async fn fetch_dashboard_new_this_week(ctx: &AdminCtx) -> HashMap<&'static str, i64> {
    let mut out: HashMap<&'static str, i64> = HashMap::new();
    for entry in ctx.admin.entries() {
        if entry.core {
            continue;
        }
        if !entry.fields.iter().any(|f| f.name == "created_at") {
            continue;
        }
        let sql = format!(
            "SELECT COUNT(*) FROM {} WHERE created_at > NOW() - INTERVAL '7 days'",
            entry.table
        );
        match sqlx::query_scalar::<_, i64>(&sql)
            .fetch_one(ctx.db.pool())
            .await
        {
            Ok(n) => {
                out.insert(entry.table, n.max(0));
            }
            Err(e) => {
                log::warn!(
                    "dashboard new-this-week count failed for {}: {e}",
                    entry.table
                );
            }
        }
    }
    out
}

/// Fetch the audit-action count for each of the last 7 days
/// (today minus 6 days through today, UTC), padded so the returned
/// vec always has exactly 7 entries in chronological order. Days
/// with no activity get a `0` count. Powers the dashboard's
/// 7-day activity sparkline.
///
/// Failure-soft: a DB hiccup logs and returns 7 zero-padded
/// entries, so the dashboard always renders a chart shape even
/// when the audit table is unreachable.
async fn fetch_activity_last_7_days(ctx: &AdminCtx) -> Vec<render::DaySparkPoint> {
    use chrono::{Datelike, NaiveDate};

    let rows = sqlx::query_as::<_, (NaiveDate, i64)>(
        "SELECT DATE(timestamp) AS day, COUNT(*)
         FROM rustio_admin_actions
         WHERE timestamp > NOW() - INTERVAL '7 days'
         GROUP BY day
         ORDER BY day",
    )
    .fetch_all(ctx.db.pool())
    .await
    .unwrap_or_else(|e| {
        log::warn!("dashboard 7-day sparkline: {e}");
        Vec::new()
    });

    let by_day: HashMap<NaiveDate, i64> = rows.into_iter().collect();
    let today = chrono::Utc::now().date_naive();
    let mut out: Vec<render::DaySparkPoint> = Vec::with_capacity(7);
    for offset in (0..7).rev() {
        let day = today - chrono::Duration::days(offset);
        let count = by_day.get(&day).copied().unwrap_or(0).max(0);
        out.push(render::DaySparkPoint {
            date_iso: day.format("%Y-%m-%d").to_string(),
            // Short weekday label ("Mon", "Tue", …) for the
            // x-axis legend — `chrono`'s `Weekday::short_name`
            // returns 3-char ASCII, locale-neutral.
            label: short_weekday(day.weekday()),
            count,
        });
    }
    out
}

/// Locale-neutral 3-letter weekday label for the sparkline x-axis.
/// Chrono's `Weekday::Display` impl yields long names; this
/// projection keeps the chart compact.
fn short_weekday(w: chrono::Weekday) -> &'static str {
    use chrono::Weekday::*;
    match w {
        Mon => "Mon",
        Tue => "Tue",
        Wed => "Wed",
        Thu => "Thu",
        Fri => "Fri",
        Sat => "Sat",
        Sun => "Sun",
    }
}

// ---- Saved filters -------------------------------------------------------

/// `POST /admin/:admin_name/saved_filters` — persist the current
/// list-page query string under a user-chosen name.
///
/// Form payload:
///   `_name`  — operator-facing label, trimmed + capped via
///              `saved_filters::sanitise_name`. Empty / whitespace
///              rejected with a 400; over the cap, truncated.
///   `_query` — raw query string to bookmark
///              (e.g. `q=anna&status=active&sort=name&dir=asc`).
///              May be empty (bookmarks "the default view").
///
/// Re-saving with the same name for the same user + model is an
/// upsert — the row's `query_string` and `created_at` get refreshed,
/// no duplicate is created. Redirects back to the list page with
/// the saved query applied so the operator immediately sees what
/// they just bookmarked.
pub(crate) async fn do_save_filter(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    req: Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let form = req.form()?;
    let raw_name = form.get("_name").unwrap_or_default();
    let name = match super::saved_filters::sanitise_name(raw_name) {
        Some(n) => n,
        None => {
            return Err(Error::BadRequest(
                "Saved-filter name can't be empty.".into(),
            ))
        }
    };
    let query = super::saved_filters::sanitise_query(form.get("_query").unwrap_or_default());

    super::saved_filters::ensure_table(&ctx.db).await?;
    super::saved_filters::save(&ctx.db, identity.user_id, entry.admin_name, &name, &query).await?;

    // Redirect to the list with the saved query applied so the
    // operator visually confirms what got bookmarked.
    let target = if query.is_empty() {
        format!("/admin/{}", entry.admin_name)
    } else {
        format!("/admin/{}?{}", entry.admin_name, query)
    };
    Ok(Response::redirect(target))
}

/// `POST /admin/:admin_name/saved_filters/:id/delete` — remove one
/// of the operator's own saved filters by id. Scope-locked to
/// `identity.user_id` inside the SQL so one operator can't drop
/// another's bookmark by id-typing.
pub(crate) async fn do_delete_filter(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    id: i64,
    _req: Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    super::saved_filters::ensure_table(&ctx.db).await?;
    // The delete is scoped to `user_id` in the SQL; a missing row
    // (either nonexistent or another user's) collapses to a no-op
    // redirect — no information leak about whether the id exists
    // for someone else.
    let _ = super::saved_filters::delete(&ctx.db, identity.user_id, id).await?;
    Ok(Response::redirect(format!("/admin/{}", entry.admin_name)))
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
    // FK filters need the registry to know the target's admin slug
    // (the lookup endpoint URL bakes it in). Building the registry
    // once per list render is cheap — pure-functional walk over the
    // entries, no DB. The same registry hydrates FK cells below.
    let registry = super::relations::RelationRegistry::from_admin_entries(ctx.admin.entries());
    let inferred =
        super::filters::infer_filters_with_registry(entry.fields, entry.singular_name, &registry);
    let mut filter_groups: Vec<render::FilterGroupCtx> = Vec::new();
    let mut sql_filters: Vec<(String, String)> = Vec::new();
    let mut sql_date_ranges: Vec<(String, Option<String>, Option<String>)> = Vec::new();
    let mut sql_multi_filters: Vec<(String, Vec<String>)> = Vec::new();
    for f in inferred {
        match f.kind {
            super::filters::FilterKind::BoolYesNo => {
                let current = qs
                    .get(&f.field)
                    .map(str::to_string)
                    .filter(|s| !s.is_empty());
                if let Some(ref val) = current {
                    sql_filters.push((f.field.clone(), val.clone()));
                }
                let options = vec![
                    render::FilterOptionCtx {
                        value: "true".into(),
                        label: "Yes".into(),
                        selected: current.as_deref() == Some("true"),
                        link: String::new(),
                    },
                    render::FilterOptionCtx {
                        value: "false".into(),
                        label: "No".into(),
                        selected: current.as_deref() == Some("false"),
                        link: String::new(),
                    },
                ];
                filter_groups.push(render::FilterGroupCtx {
                    field: f.field,
                    label: f.label,
                    kind: "chips",
                    options,
                    current,
                    all_link: String::new(),
                    date_from_name: String::new(),
                    date_from_value: String::new(),
                    date_to_name: String::new(),
                    date_to_value: String::new(),
                    hidden_pairs: Vec::new(),
                    has_active_range: false,
                    multi_selected: Vec::new(),
                    fk_selected_id: String::new(),
                    fk_selected_label: String::new(),
                    fk_lookup_url: String::new(),
                    fk_target_label: String::new(),
                });
            }
            super::filters::FilterKind::MultiSelect { values } => {
                // URL convention: one repeated `?<col>=v` per
                // checked option. Filter through the declared
                // `values` slice so a hand-crafted URL can't smuggle
                // a free-text comparison into the SQL — the runtime
                // emits parameterised placeholders, but defense in
                // depth keeps the value domain closed too.
                let allowed: HashSet<&str> = values.iter().copied().collect();
                let selected: Vec<String> = qs
                    .get_all(&f.field)
                    .iter()
                    .filter(|s| !s.is_empty() && allowed.contains(s.as_str()))
                    .cloned()
                    .collect();
                if !selected.is_empty() {
                    sql_multi_filters.push((f.field.clone(), selected.clone()));
                }
                let options: Vec<render::FilterOptionCtx> = values
                    .iter()
                    .map(|v| render::FilterOptionCtx {
                        value: (*v).to_string(),
                        label: (*v).to_string(),
                        selected: selected.iter().any(|s| s == v),
                        link: String::new(),
                    })
                    .collect();
                filter_groups.push(render::FilterGroupCtx {
                    field: f.field.clone(),
                    label: f.label,
                    kind: "multi_select",
                    options,
                    current: None,
                    all_link: String::new(),
                    date_from_name: String::new(),
                    date_from_value: String::new(),
                    date_to_name: String::new(),
                    date_to_value: String::new(),
                    hidden_pairs: Vec::new(),
                    has_active_range: false,
                    multi_selected: selected,
                    fk_selected_id: String::new(),
                    fk_selected_label: String::new(),
                    fk_lookup_url: String::new(),
                    fk_target_label: String::new(),
                });
            }
            super::filters::FilterKind::DateRange => {
                // URL convention: `?<col>__gte=YYYY-MM-DD&<col>__lte=YYYY-MM-DD`.
                // Both bounds optional. Anything that doesn't parse
                // as a date is dropped silently — the input itself
                // is a `<input type="date">`, so the only way to
                // get garbage in is hand-edited URLs.
                let gte_name = format!("{}__gte", f.field);
                let lte_name = format!("{}__lte", f.field);
                let gte = qs.get(&gte_name).and_then(parse_date_yyyy_mm_dd);
                let lte = qs.get(&lte_name).and_then(parse_date_yyyy_mm_dd);
                let has_active = gte.is_some() || lte.is_some();
                if has_active {
                    sql_date_ranges.push((f.field.clone(), gte.clone(), lte.clone()));
                }
                filter_groups.push(render::FilterGroupCtx {
                    field: f.field.clone(),
                    label: f.label,
                    kind: "date_range",
                    options: Vec::new(),
                    current: None,
                    all_link: String::new(),
                    date_from_name: gte_name,
                    date_from_value: gte.unwrap_or_default(),
                    date_to_name: lte_name,
                    date_to_value: lte.unwrap_or_default(),
                    hidden_pairs: Vec::new(),
                    has_active_range: has_active,
                    multi_selected: Vec::new(),
                    fk_selected_id: String::new(),
                    fk_selected_label: String::new(),
                    fk_lookup_url: String::new(),
                    fk_target_label: String::new(),
                });
            }
            super::filters::FilterKind::FkAutocomplete {
                target_admin_name,
                target_model,
            } => {
                // The user-submitted FK id parses as i64; anything
                // else (empty, garbage, negative) drops silently.
                let selected_id: Option<i64> = qs
                    .get(&f.field)
                    .and_then(|s| s.trim().parse::<i64>().ok())
                    .filter(|n| *n > 0);
                if let Some(id) = selected_id {
                    // Reuse the equality path — `WHERE col::text =
                    // $N` is the right shape for a single FK match.
                    sql_filters.push((f.field.clone(), id.to_string()));
                }
                // Hydrate the chosen id into a human label so the
                // active-filter pill and the typeahead input both
                // render the row name, not just `#42`. A missing /
                // deleted target row falls back to `#<id>` so the
                // page never 500s on a stale URL.
                let (selected_id_str, selected_label) = if let Some(id) = selected_id {
                    let target = ctx.admin.entries().iter().find(|e| {
                        e.singular_name == target_model
                            || e.admin_name == target_model
                            || e.display_name == target_model
                            || e.admin_name == target_admin_name
                    });
                    let label = if let Some(t) = target {
                        t.ops
                            .object_label(&ctx.db, id)
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| format!("#{id}"))
                    } else {
                        format!("#{id}")
                    };
                    (id.to_string(), label)
                } else {
                    (String::new(), String::new())
                };
                let lookup_url = format!("/admin/_lookup/{target_admin_name}");
                // `current` carries the FK id so active_filter_pairs
                // round-trips it as `?<col>=<id>` automatically.
                // Empty string means "nothing selected" and is
                // filtered out one level up.
                let current = if !selected_id_str.is_empty() {
                    Some(selected_id_str.clone())
                } else {
                    None
                };
                filter_groups.push(render::FilterGroupCtx {
                    field: f.field.clone(),
                    label: f.label,
                    kind: "fk_autocomplete",
                    options: Vec::new(),
                    current,
                    all_link: String::new(),
                    date_from_name: String::new(),
                    date_from_value: String::new(),
                    date_to_name: String::new(),
                    date_to_value: String::new(),
                    hidden_pairs: Vec::new(),
                    has_active_range: false,
                    multi_selected: Vec::new(),
                    fk_selected_id: selected_id_str,
                    fk_selected_label: selected_label,
                    fk_lookup_url: lookup_url,
                    fk_target_label: target_model.clone(),
                });
            }
            super::filters::FilterKind::DropdownText => {
                // Status-shape columns: populate the dropdown from
                // `SELECT DISTINCT col::text FROM table WHERE col IS
                // NOT NULL ORDER BY col::text LIMIT N`. The column
                // name is validated against `entry.fields` first so a
                // hand-crafted URL can't smuggle SQL through the
                // format!() interpolation. `::text` cast keeps the
                // query general — works for varchar / text / enum /
                // any column type with a sensible text projection.
                if !entry.fields.iter().any(|af| af.name == f.field) {
                    continue;
                }
                let current = qs
                    .get(&f.field)
                    .map(str::to_string)
                    .filter(|s| !s.is_empty());
                if let Some(ref val) = current {
                    sql_filters.push((f.field.clone(), val.clone()));
                }
                let distinct_sql = format!(
                    "SELECT DISTINCT {col}::text AS v FROM {tbl} \
                     WHERE {col} IS NOT NULL \
                     ORDER BY v LIMIT {lim}",
                    col = f.field,
                    tbl = entry.table,
                    lim = DROPDOWN_TEXT_OPTION_CAP,
                );
                let values: Vec<String> = sqlx::query_scalar::<_, String>(&distinct_sql)
                    .fetch_all(ctx.db.pool())
                    .await
                    .unwrap_or_else(|e| {
                        log::warn!(
                            "dropdown_text distinct query failed for {}.{}: {e}",
                            entry.admin_name,
                            f.field
                        );
                        Vec::new()
                    });
                let options: Vec<render::FilterOptionCtx> = values
                    .into_iter()
                    .map(|v| render::FilterOptionCtx {
                        selected: current.as_deref() == Some(v.as_str()),
                        value: v.clone(),
                        label: v,
                        link: String::new(),
                    })
                    .collect();
                filter_groups.push(render::FilterGroupCtx {
                    field: f.field,
                    label: f.label,
                    kind: "chips",
                    options,
                    current,
                    all_link: String::new(),
                    date_from_name: String::new(),
                    date_from_value: String::new(),
                    date_to_name: String::new(),
                    date_to_value: String::new(),
                    hidden_pairs: Vec::new(),
                    has_active_range: false,
                    multi_selected: Vec::new(),
                    fk_selected_id: String::new(),
                    fk_selected_label: String::new(),
                    fk_lookup_url: String::new(),
                    fk_target_label: String::new(),
                });
            }
            // Other filter kinds need richer widgets — later phases.
            _ => {}
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
    //
    // `per_page_override` is preserved separately so URL-composing
    // links downstream can decide whether the value is "default"
    // (skip from URL) or "user-chosen" (carry it through). Without
    // this distinction every state-preserving link would either drop
    // the user's choice or pollute every URL with the default.
    let per_page_override: Option<i64> = qs
        .get("per_page")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|n| matches!(*n, 25 | 50 | 100 | 200));
    let per_page: i64 = per_page_override
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
                date_ranges: sql_date_ranges.clone(),
                multi_filters: sql_multi_filters.clone(),
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
                    date_ranges: sql_date_ranges,
                    multi_filters: sql_multi_filters,
                    search: search_opt,
                    limit: Some(per_page),
                    offset: Some(clamped_offset),
                },
            )
            .await?;
    }

    // CSV content negotiation — checked first so a request that
    // names both `text/csv` and `application/json` in `Accept`
    // wins for whichever the client listed first (handled inside
    // `wants_csv` / `wants_json`). Same short-circuit rationale
    // as JSON: skip FK hydration; the export carries raw cells.
    if super::csv_export::wants_csv(req) {
        return super::csv_export::list_response(entry, page_result);
    }

    // JSON content negotiation. When the client wants JSON (via
    // `Accept` or `?format=json`), short-circuit ahead of the
    // HTML rendering — including the FK cell hydration, which is
    // a presentation concern (the JSON shape stores raw FK ids,
    // not human labels).
    if super::json_api::wants_json(req) {
        let envelope =
            super::json_api::list_envelope(entry, page_result, page as usize, per_page as usize);
        return super::json_api::json_response(envelope);
    }

    // Resolve every FK cell on this page from raw id to the target
    // row's display label, and stash a click-through link for the
    // template. One batched query per FK column on the model — N+1-safe.
    hydrate_fk_cells(&ctx.db, &ctx.admin, entry, &mut page_result.rows).await?;

    // Saved filters for this operator on this model. Lazy CREATE on
    // first hit (parallels `ensure_audit_ready`); a fetch failure
    // logs and falls through to an empty list — the page still
    // renders, just without the bookmarks dropdown.
    let _ = super::saved_filters::ensure_table(&ctx.db).await;
    let saved_filters =
        super::saved_filters::list_for_user(&ctx.db, identity.user_id, entry.admin_name)
            .await
            .unwrap_or_else(|e| {
                log::warn!(
                    "saved_filters fetch failed for user={}: {e}",
                    identity.user_id
                );
                Vec::new()
            });

    let list = render::list_ctx(
        &identity,
        &ctx.admin,
        entry,
        page_result.rows,
        search,
        filter_groups,
        page as usize,
        per_page as usize,
        per_page_override.map(|n| n as usize),
        total_rows as usize,
        active_sort.as_ref().map(|(c, d)| (c.clone(), *d)),
        csrf_token(req),
        saved_filters,
        req.query_string().to_string(),
    );
    let body = ctx
        .templates
        .render_for_model(entry.admin_name, "admin/list.html", &list)?;
    Ok(Response::html(body))
}

/// Strict `YYYY-MM-DD` parse for the `DateRange` filter's URL
/// parameters. Empty / whitespace / malformed input returns `None`
/// so the filter falls back to "unbounded on this side" rather than
/// throwing a 4xx — the `<input type="date">` widget itself prevents
/// most bad input, and hand-edited URLs that don't parse simply lose
/// the bound. Returns the canonical `YYYY-MM-DD` re-emission so
/// Postgres' `::date` cast accepts it without ambiguity.
fn parse_date_yyyy_mm_dd(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .ok()
        .map(|d| d.format("%Y-%m-%d").to_string())
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

// ---- CSV export ----------------------------------------------------------

/// Parsed shape of `?q=&sort=&dir=&<col>=&<col>__gte=…` etc.,
/// reduced to just the SQL-side pieces needed for the export query.
///
/// Mirrors the filter-match arms inside `list_model` but skips the
/// UI-context construction (chip options, hydrated pill labels,
/// hidden-input plumbing) — none of which apply to a CSV. If a new
/// `FilterKind` ever lights up over there, mirror its SQL arm
/// here so the export keeps parity. The two surfaces are not yet
/// unified behind a single walker because the UI side carries
/// significantly more state per group; promoting one source of
/// truth is a future refactor, called out on the
/// `keep-in-lockstep` comment below.
struct SqlFilterQuery {
    filters: Vec<(String, String)>,
    date_ranges: Vec<(String, Option<String>, Option<String>)>,
    multi_filters: Vec<(String, Vec<String>)>,
}

/// keep-in-lockstep: any change to the filter-kind branches inside
/// `list_model::list_ctx`'s assembly loop must mirror here.
fn parse_sql_filter_query(
    entry: &super::types::AdminEntry,
    qs: &crate::http::FormData,
    registry: &super::relations::RelationRegistry,
) -> SqlFilterQuery {
    let inferred =
        super::filters::infer_filters_with_registry(entry.fields, entry.singular_name, registry);
    let mut filters: Vec<(String, String)> = Vec::new();
    let mut date_ranges: Vec<(String, Option<String>, Option<String>)> = Vec::new();
    let mut multi_filters: Vec<(String, Vec<String>)> = Vec::new();
    for f in inferred {
        match f.kind {
            super::filters::FilterKind::BoolYesNo => {
                if let Some(val) = qs.get(&f.field).filter(|s| !s.is_empty()) {
                    filters.push((f.field.clone(), val.to_string()));
                }
            }
            super::filters::FilterKind::MultiSelect { values } => {
                let allowed: HashSet<&str> = values.iter().copied().collect();
                let selected: Vec<String> = qs
                    .get_all(&f.field)
                    .iter()
                    .filter(|s| !s.is_empty() && allowed.contains(s.as_str()))
                    .cloned()
                    .collect();
                if !selected.is_empty() {
                    multi_filters.push((f.field.clone(), selected));
                }
            }
            super::filters::FilterKind::DateRange => {
                let gte_name = format!("{}__gte", f.field);
                let lte_name = format!("{}__lte", f.field);
                let gte = qs.get(&gte_name).and_then(parse_date_yyyy_mm_dd);
                let lte = qs.get(&lte_name).and_then(parse_date_yyyy_mm_dd);
                if gte.is_some() || lte.is_some() {
                    date_ranges.push((f.field, gte, lte));
                }
            }
            super::filters::FilterKind::FkAutocomplete { .. } => {
                if let Some(id) = qs
                    .get(&f.field)
                    .and_then(|s| s.trim().parse::<i64>().ok())
                    .filter(|n| *n > 0)
                {
                    filters.push((f.field, id.to_string()));
                }
            }
            super::filters::FilterKind::DropdownText => {
                if let Some(val) = qs.get(&f.field).filter(|s| !s.is_empty()) {
                    filters.push((f.field.clone(), val.to_string()));
                }
            }
            _ => {}
        }
    }
    SqlFilterQuery {
        filters,
        date_ranges,
        multi_filters,
    }
}

/// Cap on the number of options surfaced in a `DropdownText`
/// filter widget. Distinct-value queries that exceed this are
/// truncated — operators with high-cardinality columns should fall
/// back to the search box. 50 keeps the dropdown legible without
/// pretending to be a search-replacement.
const DROPDOWN_TEXT_OPTION_CAP: i64 = 50;

/// Maximum rows a single CSV download will emit. Caps the memory
/// footprint of the bulk `entry.ops.list(...)` call — the runtime
/// fetches all matching rows into a `Vec<ListRow>` before iterating.
/// Operators with bigger tables should pre-filter via the list-page
/// widgets (search / filters / sort) before downloading.
const CSV_EXPORT_MAX_ROWS: i64 = 10_000;

/// `GET /admin/:admin_name/export.csv?<same filters as list page>` —
/// stream a CSV of every row matching the current list-page query
/// (search + filters + sort), capped at `CSV_EXPORT_MAX_ROWS`.
///
/// Header row uses each field's humanised label (same source as the
/// list-page column header). Cell content comes from
/// `AdminModel::display_values()` — same strings the list view
/// shows. Each cell is RFC 4180-quoted via `csv_escape_field` so
/// commas / quotes / newlines inside the data don't break the
/// parser on the operator's spreadsheet.
///
/// Permission: same as the list page (`view`). Core entries (the
/// synthetic `User`) are blocked by `find_project_entry`.
pub(crate) async fn export_model_csv(
    ctx: &AdminCtx,
    _identity: Identity,
    admin_name: &str,
    req: Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let qs = req.query();

    let active_sort = parse_active_sort(entry, qs.get("sort"), qs.get("dir"));
    let ordering = match &active_sort {
        Some((col, dir)) => vec![(col.clone(), *dir)],
        None => entry
            .ordering
            .iter()
            .map(|s| super::modeladmin::parse_order_spec(s))
            .collect(),
    };

    let registry = super::relations::RelationRegistry::from_admin_entries(ctx.admin.entries());
    let parsed = parse_sql_filter_query(entry, &qs, &registry);

    let search = qs.get("q").unwrap_or_default().to_string();
    let search_opt: Option<(String, Vec<String>)> =
        if search.is_empty() || entry.search_fields.is_empty() {
            None
        } else {
            Some((
                search,
                entry.search_fields.iter().map(|s| s.to_string()).collect(),
            ))
        };

    let page = entry
        .ops
        .list(
            &ctx.db,
            super::types::ListOpts {
                ordering,
                filters: parsed.filters,
                date_ranges: parsed.date_ranges,
                multi_filters: parsed.multi_filters,
                search: search_opt,
                limit: Some(CSV_EXPORT_MAX_ROWS),
                offset: None,
            },
        )
        .await?;

    // Header row: per-field humanised label, in the model's
    // declared field order. We export every column on the model
    // (including those omitted from `list_display`) — CSV is a
    // data-fidelity surface, not a styled list, so the export
    // surface is consciously wider than the on-screen view.
    let mut body = String::with_capacity(1024);
    for (i, f) in entry.fields.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&csv_escape_field(f.label));
    }
    body.push('\n');

    for row in page.rows {
        for (i, cell) in row.cells.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&csv_escape_field(cell));
        }
        body.push('\n');
    }

    let filename = format!("{}.csv", entry.admin_name);
    Ok(Response::ok(body)
        .with_header("content-type", "text/csv; charset=utf-8")
        .with_header(
            "content-disposition",
            format!("attachment; filename=\"{filename}\""),
        ))
}

/// RFC 4180-style field escape. Quotes the field when it contains
/// any of `,`, `"`, `\r`, `\n`; doubles every internal `"`. Plain
/// alphanumeric content is passed through unchanged.
fn csv_escape_field(s: &str) -> String {
    let needs_quote = s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !needs_quote {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' {
            out.push_str("\"\"");
        } else {
            out.push(c);
        }
    }
    out.push('"');
    out
}

// ---- FK autocomplete lookup ----------------------------------------------

/// JSON shape returned by [`lookup_model`]. Stable contract for the
/// admin's bundled JS — keep these field names in sync with
/// `admin.js::initFkAutocomplete`.
#[derive(serde::Serialize)]
struct LookupItem {
    id: i64,
    label: String,
}

/// `GET /admin/_lookup/:admin_name?q=<term>&limit=<n>` —
/// foreign-key autocomplete backend. Returns up to `limit` rows of
/// the named admin model (1..=50) as JSON, search filtered through
/// the target's `ModelAdmin::search_fields()`. The label per row
/// resolves via the same ladder as the form view's
/// `resolve_relation_options`. Identity is required (every other
/// `/admin/*` route goes through the same guard), but the endpoint
/// itself has no model-level permission gate beyond "the user can
/// reach the admin" — the data leaked is the same the user can
/// already see on the target's list page.
pub(crate) async fn lookup_model(
    ctx: &AdminCtx,
    _identity: Identity,
    admin_name: &str,
    req: Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let qs = req.query();
    let term = qs.get("q").unwrap_or_default().trim().to_string();
    let limit: i64 = qs
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
        .clamp(1, 50);

    // Empty term + no search_fields → just return the first N rows.
    // Empty term + search_fields configured → still return first N
    // (the ILIKE on `%%` is a full scan we'd rather avoid).
    let search = if term.is_empty() || entry.search_fields.is_empty() {
        None
    } else {
        Some((
            term,
            entry.search_fields.iter().map(|s| s.to_string()).collect(),
        ))
    };

    let page = entry
        .ops
        .list(
            &ctx.db,
            super::types::ListOpts {
                search,
                limit: Some(limit),
                ..Default::default()
            },
        )
        .await?;

    // The target model decides its own label column: a column named
    // `name` or `title` if present, otherwise the row id. We don't
    // honour `BelongsTo.display = "col"` here because the lookup is
    // target-rooted (no source relation in play) — same convention
    // the target's own list page uses for the first display column.
    let display_idx = render::pick_display_index(entry.fields, None);
    let items: Vec<LookupItem> = page
        .rows
        .into_iter()
        .map(|r| {
            let label = display_idx
                .and_then(|i| r.cells.get(i).cloned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("#{}", r.id));
            LookupItem { id: r.id, label }
        })
        .collect();

    let body = serde_json::to_string(&items)
        .map_err(|e| Error::Internal(format!("fk lookup serialize: {e}")))?;
    Ok(Response::json_raw(body))
}

// ---- Global search palette (⌘K) ------------------------------------------

/// Maximum results returned per model by the global search palette.
const SEARCH_PER_MODEL_LIMIT: i64 = 5;

/// Hard cap on total results returned by the global search palette.
/// The palette is meant for "I know what I'm looking for"; longer
/// hit-lists belong on the per-model list page with its full filter
/// set.
const SEARCH_TOTAL_LIMIT: usize = 20;

/// Minimum query length the global search will run. Sub-threshold
/// queries return an empty result set without touching the DB — a
/// single character against N models is enough load multiplier to
/// matter on a busy admin.
const SEARCH_MIN_QUERY_LEN: usize = 2;

#[derive(serde::Serialize)]
struct SearchResult {
    admin_name: String,
    model_label: String,
    label: String,
    url: String,
}

#[derive(serde::Serialize)]
struct SearchEnvelope {
    results: Vec<SearchResult>,
}

/// Cross-model search endpoint backing the `⌘K` topbar palette.
///
/// `GET /admin/_search?q=<term>` returns JSON
/// `{"results": [{"admin_name", "model_label", "label", "url"}, ...]}`,
/// capped at [`SEARCH_TOTAL_LIMIT`] entries overall and
/// [`SEARCH_PER_MODEL_LIMIT`] per model. Only project models with
/// non-empty `search_fields` are queried; each is gated by the
/// operator's `<admin_name>.change_<singular>` permission. The
/// permission matches the URL the palette ships — every result links
/// to `/admin/<model>/<id>/edit`, which is itself `change`-gated, so
/// surfacing only `change`-permitted rows keeps the palette honest:
/// every result is a row the operator can directly act on. Auditors /
/// view-only operators still have the per-list-page search at
/// `/admin/<model>?q=...` (gated on `view`).
///
/// Core (framework-owned) entries like the synthetic User are skipped
/// to mirror `find_project_entry`'s policy — searching users is
/// reachable from `/admin/users` with its bespoke gate.
///
/// Empty term or a term shorter than [`SEARCH_MIN_QUERY_LEN`] short-
/// circuits to an empty result set without touching the DB.
pub(crate) async fn search_models(
    ctx: &AdminCtx,
    identity: Identity,
    req: Request,
) -> Result<Response> {
    let term = req.query().get("q").unwrap_or_default().trim().to_string();
    let mut envelope = SearchEnvelope {
        results: Vec::new(),
    };

    if term.chars().count() < SEARCH_MIN_QUERY_LEN {
        let body = serde_json::to_string(&envelope)
            .map_err(|e| Error::Internal(format!("search serialize: {e}")))?;
        return Ok(Response::json_raw(body));
    }

    for entry in ctx.admin.entries() {
        if entry.core || entry.search_fields.is_empty() {
            continue;
        }
        // Gate on `change`, not `view`: the URL we ship below is
        // `/admin/<model>/<id>/edit`, which the framework routes
        // through a `change`-gated handler. Surfacing rows the
        // operator can't open creates a trap UX.
        let perm = format!(
            "{}.change_{}",
            entry.admin_name,
            entry.singular_name.to_ascii_lowercase()
        );
        if !auth::check_permission(&ctx.db, &identity, &perm).await? {
            continue;
        }
        let page = entry
            .ops
            .list(
                &ctx.db,
                super::types::ListOpts {
                    search: Some((
                        term.clone(),
                        entry.search_fields.iter().map(|s| s.to_string()).collect(),
                    )),
                    limit: Some(SEARCH_PER_MODEL_LIMIT),
                    ..Default::default()
                },
            )
            .await?;

        let display_idx = render::pick_display_index(entry.fields, None);
        for row in page.rows {
            if envelope.results.len() >= SEARCH_TOTAL_LIMIT {
                break;
            }
            let label = display_idx
                .and_then(|i| row.cells.get(i).cloned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("#{}", row.id));
            envelope.results.push(SearchResult {
                admin_name: entry.admin_name.to_string(),
                model_label: entry.display_name.to_string(),
                label,
                url: format!("/admin/{}/{}/edit", entry.admin_name, row.id),
            });
        }
        if envelope.results.len() >= SEARCH_TOTAL_LIMIT {
            break;
        }
    }

    let body = serde_json::to_string(&envelope)
        .map_err(|e| Error::Internal(format!("search serialize: {e}")))?;
    Ok(Response::json_raw(body))
}

// ---- CRUD audit emission -------------------------------------------------

/// Fire-and-forget audit emission for a project-model CRUD event.
/// Mirrors the conventional metadata shape used elsewhere (ip,
/// correlation_id, summary, optional structured metadata). Audit-
/// write failures log a warning but never block the user-facing
/// redirect — same posture as every other CRUD-adjacent audit
/// emission in the framework (e.g. the auto-lock branch).
#[allow(clippy::too_many_arguments)]
async fn record_crud_audit(
    ctx: &AdminCtx,
    identity: &Identity,
    req: &Request,
    entry: &super::types::AdminEntry,
    action_type: audit::ActionType,
    object_id: i64,
    summary: String,
    metadata: Option<serde_json::Value>,
) {
    let ip = super::builtin::client_ip(req);
    let cid = super::builtin::correlation_id_from(req);
    let mut log_entry =
        audit::LogEntry::new(identity.user_id, action_type, entry.admin_name, object_id);
    log_entry.summary = summary;
    log_entry.ip_address = ip.as_deref();
    log_entry.correlation_id = cid.as_deref();
    log_entry.metadata = metadata;
    if let Err(e) = audit::record(&ctx.db, log_entry).await {
        log::warn!(
            "audit::record (CRUD {at}) failed model={model} id={object_id}: {e}",
            at = action_type.as_str(),
            model = entry.admin_name,
        );
    }
}

/// One column's value transition between two row snapshots. Emitted
/// as a `{field, label, from, to}` object inside the audit metadata's
/// `changes` array.
#[derive(serde::Serialize)]
struct FieldChange {
    field: String,
    label: String,
    from: String,
    to: String,
}

/// Compute the per-column diff between two `EditRow` snapshots.
/// Restricted to fields the model declares as editable — non-
/// editable timestamps and hashes are excluded so the diff reads
/// as "what did the human change," not "what side-effects did the
/// SQL update have."
fn compute_row_diff(
    entry: &super::types::AdminEntry,
    before: &super::types::EditRow,
    after: &super::types::EditRow,
) -> Vec<FieldChange> {
    let before_map: HashMap<&str, &str> = before
        .values
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let after_map: HashMap<&str, &str> = after
        .values
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let mut changes: Vec<FieldChange> = Vec::new();
    for f in entry.fields {
        if !f.editable {
            continue;
        }
        let before_val = before_map.get(f.name).copied().unwrap_or("");
        let after_val = after_map.get(f.name).copied().unwrap_or("");
        if before_val != after_val {
            changes.push(FieldChange {
                field: f.name.to_string(),
                label: f.label.to_string(),
                from: before_val.to_string(),
                to: after_val.to_string(),
            });
        }
    }
    changes
}

// ---- JSON detail endpoint ------------------------------------------------

/// `GET /admin/<model>/<id>` — JSON-only detail endpoint. Mirrors
/// the row that `/admin/<model>/<id>/edit` renders as a form;
/// returns the row as `{"id":N, "<field>":…, …}`. Refuses with a
/// 406 if the client didn't request JSON — the HTML edit page is
/// the dedicated UI surface at `.../edit`.
pub(crate) async fn show_object_json(
    ctx: &AdminCtx,
    _identity: Identity,
    admin_name: &str,
    id: i64,
    req: &Request,
) -> Result<Response> {
    // Capture the negotiation decision once, up front. Both the
    // "unknown model" lookup below and the row fetch can fail —
    // when the client asked for JSON, those failures should
    // also serialise as JSON envelopes, not as the framework's
    // default HTML error page.
    let wants_json = super::json_api::wants_json(req);

    let entry = match find_project_entry(&ctx.admin, admin_name) {
        Ok(e) => e,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };

    if !wants_json {
        // Send the caller to the form view. Browsers visiting
        // `/admin/<model>/<id>` directly land where they expect.
        return Ok(Response::redirect(format!(
            "/admin/{}/{}/edit",
            entry.admin_name, id
        )));
    }

    let row = match entry.ops.find_row(&ctx.db, id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Ok(super::json_api::json_error(Error::NotFound(format!(
                "{admin_name}/{id}"
            ))));
        }
        Err(e) => return Ok(super::json_api::json_error(e)),
    };
    let obj = super::json_api::detail_envelope(entry, row);
    super::json_api::json_response(obj)
}

// ---- Multipart form parsing (file uploads) -------------------------------

/// Maximum bytes the framework holds in memory while parsing a
/// multipart body. Above this the request is refused. Sized
/// conservatively for admin-form workloads — image uploads up to
/// ~8 MB land cleanly; larger files want a streaming surface that
/// v1 doesn't ship.
const MULTIPART_MAX_BODY: usize = 16 * 1024 * 1024;

/// Maximum per-file size after multipart parsing. Mirrors the
/// whole-body cap above; refusing earlier than the round-trip
/// catches the bad input one indirection sooner.
const UPLOAD_FILE_MAX: usize = 16 * 1024 * 1024;

/// Pick the right form parser based on the request's
/// `Content-Type`. Multipart bodies → parse via
/// `crate::multipart`, write file parts to `<uploads_dir>/<…>`,
/// inject the resulting relative path back into the form's
/// string slot under the part's `name`. Urlencoded bodies fall
/// through to the existing `Request::form()` path.
///
/// `uploads_dir` is the canonical root from `Admin::uploads_dir`.
/// When `None`, file parts are silently dropped — the framework
/// renders the file input but won't write anything, which is the
/// right default for projects that haven't opted into the upload
/// surface yet.
async fn parse_form_with_uploads(
    req: &Request,
    uploads_dir: Option<&std::path::Path>,
) -> Result<crate::http::FormData> {
    let ct = req.header("content-type").unwrap_or("");
    let Some(boundary) = crate::multipart::boundary_from_content_type(ct) else {
        return req.form();
    };
    if req.body().len() > MULTIPART_MAX_BODY {
        return Err(Error::BadRequest(format!(
            "multipart body exceeds the {MULTIPART_MAX_BODY}-byte cap"
        )));
    }
    let parsed = crate::multipart::parse_multipart(req.body(), &boundary)
        .map_err(|e| Error::BadRequest(format!("multipart: {e}")))?;
    let mut form = crate::http::FormData::default();
    for part in parsed.parts {
        match (&part.filename, uploads_dir) {
            (Some(filename), _) if filename.is_empty() || part.body.is_empty() => {
                // File input left blank: preserve the existing
                // column value (do nothing). The
                // readonly-field-injection path inside `do_update`
                // already re-injects existing values for any field
                // the form omitted, so an empty upload leaves the
                // column unchanged across both create + update.
            }
            (Some(filename), Some(root)) => {
                if part.body.len() > UPLOAD_FILE_MAX {
                    return Err(Error::BadRequest(format!(
                        "uploaded file `{filename}` exceeds the {UPLOAD_FILE_MAX}-byte cap"
                    )));
                }
                let rel = persist_upload(root, filename, &part.body).await?;
                form.set(part.name, rel);
            }
            (Some(_), None) => {
                // File received but Admin::uploads_dir is unset.
                // Drop silently rather than 500ing — the project
                // hasn't opted into the upload surface yet.
                log::warn!(
                    "received multipart file part `{}` but Admin::uploads_dir is unset; \
                     dropping the upload",
                    part.name,
                );
            }
            (None, _) => {
                let text = String::from_utf8_lossy(&part.body).into_owned();
                form.set(part.name, text);
            }
        }
    }
    Ok(form)
}

/// Sanitise + UUID-prefix the user-supplied filename, write the
/// bytes under `uploads_dir`, and return the relative path the
/// model column stores.
///
/// File-name sanitisation rules:
///   - Strip any path components — only the basename survives.
///   - Replace all but `[A-Za-z0-9._-]` with `_`.
///   - Trim leading dots so `.htaccess` lands as `htaccess`.
///   - Cap the resulting basename to 120 chars.
///
/// The on-disk filename is `<uuid-v4>-<sanitised>` so two
/// operators can upload `screenshot.png` simultaneously without
/// collision. The serve route consults the same path-traversal
/// guard before reading the file back.
async fn persist_upload(
    uploads_dir: &std::path::Path,
    raw_filename: &str,
    bytes: &[u8],
) -> Result<String> {
    let sanitised = sanitise_upload_filename(raw_filename);
    let rel = format!("{}-{}", uuid::Uuid::new_v4(), sanitised);
    let dest = uploads_dir.join(&rel);
    // Create the directory lazily — projects that set
    // `Admin::uploads_dir` to a non-existent path get it
    // materialised on first upload rather than at boot.
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Internal(format!("create uploads dir: {e}")))?;
        }
    }
    std::fs::write(&dest, bytes)
        .map_err(|e| Error::Internal(format!("write upload {}: {e}", dest.display())))?;
    Ok(rel)
}

/// Strip path components, replace unsafe chars, trim leading
/// dots, cap length. Pure function; unit-tested below.
pub(super) fn sanitise_upload_filename(raw: &str) -> String {
    // Take only the last path component on either separator.
    let basename = raw.rsplit(['/', '\\']).next().unwrap_or("file");
    let trimmed = basename.trim_start_matches('.').trim();
    let mapped: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let bounded: String = mapped.chars().take(120).collect();
    if bounded.is_empty() {
        "file".into()
    } else {
        bounded
    }
}

/// `GET /admin/uploads/:filename` — serve a previously-uploaded
/// file. Identity-gated (Staff floor); the path-traversal guard
/// ensures the resolved file lives inside the canonical
/// `Admin::uploads_dir`. A rejected lookup returns 404 (no
/// information leak about whether the file could exist).
pub(crate) async fn serve_upload(
    ctx: &AdminCtx,
    _identity: Identity,
    filename: &str,
    _req: Request,
) -> Result<Response> {
    let Some(root) = ctx.admin.uploads_dir_path() else {
        return Err(Error::NotFound("uploads dir not configured".into()));
    };
    // Canonicalise the root once so the per-request guard
    // compares against a real on-disk path. Refuse if the
    // configured root doesn't resolve (project misconfiguration).
    let canonical_root = std::fs::canonicalize(root)
        .map_err(|_| Error::NotFound("uploads root unresolved".into()))?;
    // Defense-in-depth even though the route only binds one path
    // segment: refuse anything containing `..` or absolute paths.
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(Error::NotFound("upload not found".into()));
    }
    let candidate = canonical_root.join(filename);
    let resolved = std::fs::canonicalize(&candidate)
        .map_err(|_| Error::NotFound("upload not found".into()))?;
    if !resolved.starts_with(&canonical_root) {
        return Err(Error::NotFound("upload not found".into()));
    }
    let bytes = std::fs::read(&resolved).map_err(|_| Error::NotFound("upload not found".into()))?;
    let content_type = guess_content_type(filename);
    Ok(
        Response::new(hyper::StatusCode::OK, bytes::Bytes::from(bytes))
            .with_header("content-type", content_type)
            .with_header("cache-control", "private, max-age=300"),
    )
}

/// Trivial filename-extension → MIME guesser. Covers the common
/// cases admins upload (images + PDFs + plain docs); falls
/// through to `application/octet-stream` for everything else.
/// Resists the urge to grow into a real `mime_guess` clone.
fn guess_content_type(filename: &str) -> &'static str {
    let lower = filename.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" | "log" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

// ---- Inline-children fetch -----------------------------------------------

/// Resolve every declared `Inline` on `parent_entry` against the
/// current admin into a renderable list of child sections. v1 is
/// read-only: each row is a click-through to the child's own
/// edit / delete pages.
///
/// Reuses `target.ops.list(...)` with a single `(fk_field,
/// parent_id::text)` filter so the runtime's existing column-name
/// validation + parameterised binding apply automatically — no
/// new SQL surface, no hand-built WHERE clause per inline.
///
/// A typo in the project's `Inline { target_model, fk_field }`
/// silently renders an empty section (no panic, no 500) — a
/// future startup-validation pass can warn early. `None` parent
/// id (new-form mode) returns empty.
async fn fetch_inline_sections(
    ctx: &AdminCtx,
    parent_entry: &super::types::AdminEntry,
    parent_id: Option<i64>,
) -> Vec<render::FormInlineCtx> {
    let Some(pid) = parent_id else {
        return Vec::new();
    };
    if parent_entry.inlines.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<render::FormInlineCtx> = Vec::with_capacity(parent_entry.inlines.len());
    for inline in parent_entry.inlines {
        let target = match ctx
            .admin
            .entries()
            .iter()
            .find(|e| e.singular_name == inline.target_model)
        {
            Some(e) => e,
            None => {
                log::warn!(
                    "inline target `{}` not registered (declared on parent `{}`)",
                    inline.target_model,
                    parent_entry.admin_name,
                );
                continue;
            }
        };
        let label = inline
            .label
            .map(str::to_string)
            .unwrap_or_else(|| target.display_name.to_string());
        let cap = inline.max_rows.max(1) as i64;
        let opts = super::types::ListOpts {
            filters: vec![(inline.fk_field.to_string(), pid.to_string())],
            limit: Some(cap),
            ..Default::default()
        };
        let page = match target.ops.list(&ctx.db, opts).await {
            Ok(p) => p,
            Err(e) => {
                log::warn!(
                    "inline fetch for `{}.{}=${pid}` failed: {e}",
                    target.admin_name,
                    inline.fk_field,
                );
                continue;
            }
        };
        let display_idx = render::pick_display_index(target.fields, inline.display_field);
        let rows: Vec<render::FormInlineRowCtx> = page
            .rows
            .iter()
            .map(|r| {
                let row_label = display_idx
                    .and_then(|i| r.cells.get(i).cloned())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("#{}", r.id));
                render::FormInlineRowCtx {
                    id: r.id,
                    label: row_label,
                    edit_url: format!("/admin/{}/{}/edit", target.admin_name, r.id),
                    delete_url: format!("/admin/{}/{}/delete", target.admin_name, r.id),
                }
            })
            .collect();
        out.push(render::FormInlineCtx {
            label,
            target_display_name: target.display_name.to_string(),
            target_admin_name: target.admin_name.to_string(),
            has_more: page.total > cap,
            total: page.total,
            rows,
            add_url: format!("/admin/{}/new", target.admin_name),
            list_url: format!("/admin/{}?{}={}", target.admin_name, inline.fk_field, pid),
        });
    }
    out
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
    // New-form mode has no parent id yet, so inlines are always
    // empty here. The edit form is where they populate.
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
        Vec::new(),
    );
    let body = ctx
        .templates
        .render_for_model(admin_name, "admin/form.html", &form)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_create(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    req: Request,
) -> Result<Response> {
    // Mirror the read-path negotiation onto writes: when the client
    // asked for JSON, success / validation / framework errors all
    // collapse to JSON envelopes. Form-encoded body parsing stays
    // the same — only the response shape changes.
    let wants_json = super::json_api::wants_json(&req);
    let entry = match find_project_entry(&ctx.admin, admin_name) {
        Ok(e) => e,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    let form = match parse_form_with_uploads(&req, ctx.admin.uploads_dir_path()).await {
        Ok(f) => f,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    let intent = submit_intent(&form);
    let create_result = match entry.ops.create(&ctx.db, &form).await {
        Ok(r) => r,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    match create_result {
        Ok(id) => {
            // Audit the create. `object_label` resolves the label
            // via the same ladder the list view uses (display field
            // → name → title → #id); a fallback to `#<id>` keeps the
            // summary readable when no label column resolves.
            let label = entry
                .ops
                .object_label(&ctx.db, id)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| format!("#{id}"));
            let summary = format!("Created {}: {label}", entry.singular_name);
            record_crud_audit(
                ctx,
                &identity,
                &req,
                entry,
                audit::ActionType::Create,
                id,
                summary,
                None,
            )
            .await;
            if wants_json {
                return Ok(super::json_api::mutation_ok_envelope(
                    admin_name,
                    id,
                    hyper::StatusCode::CREATED,
                ));
            }
            Ok(Response::redirect(redirect_after_save(
                intent, admin_name, id,
            )))
        }
        Err(errors) => {
            if wants_json {
                return Ok(super::json_api::validation_errors_envelope(errors));
            }
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
                Vec::new(),
            );
            let body = ctx
                .templates
                .render_for_model(admin_name, "admin/form.html", &ctx_view)?;
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
    let inlines = fetch_inline_sections(ctx, entry, Some(id)).await;
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
        inlines,
    );
    let body = ctx
        .templates
        .render_for_model(admin_name, "admin/form.html", &form)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_update(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    id: i64,
    req: Request,
) -> Result<Response> {
    let wants_json = super::json_api::wants_json(&req);
    let entry = match find_project_entry(&ctx.admin, admin_name) {
        Ok(e) => e,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    let mut form = match parse_form_with_uploads(&req, ctx.admin.uploads_dir_path()).await {
        Ok(f) => f,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    // Snapshot the row BEFORE the update so the audit-emission
    // branch below can diff before/after and persist the changed-
    // columns set inside `metadata.changes`. Reused below for the
    // readonly-field injection too — same query, single fetch.
    let before_row = match entry.ops.find_row(&ctx.db, id).await {
        Ok(r) => r,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    // Readonly fields render `disabled` and are not submitted by the
    // browser. Inject the existing values so `M::from_form` parses a
    // complete row — the generated parser doesn't know which columns
    // the project marked readonly.
    if !entry.readonly_fields.is_empty() {
        if let Some(existing) = before_row.as_ref() {
            for name in entry.readonly_fields {
                if form.contains(name) {
                    continue;
                }
                if let Some((_, v)) = existing.values.iter().find(|(c, _)| c == name) {
                    form.set(*name, v.clone());
                }
            }
        }
    }
    let intent = submit_intent(&form);
    let update_result = match entry.ops.update(&ctx.db, id, &form).await {
        Ok(r) => r,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    match update_result {
        Ok(()) => {
            // Audit the update. Diff before/after editable columns
            // and persist the change set under `metadata.changes`
            // so the object-history page can render a per-field
            // before/after table. An empty diff (the form
            // submitted identical values) still emits a row — the
            // human action happened; metadata reflects "no fields
            // changed."
            let after_row = entry.ops.find_row(&ctx.db, id).await?;
            let changes = match (before_row.as_ref(), after_row.as_ref()) {
                (Some(b), Some(a)) => compute_row_diff(entry, b, a),
                _ => Vec::new(),
            };
            let summary = if changes.is_empty() {
                format!("Updated {} #{id} (no field changes)", entry.singular_name)
            } else {
                format!(
                    "Updated {} #{id} ({} {} changed)",
                    entry.singular_name,
                    changes.len(),
                    if changes.len() == 1 {
                        "field"
                    } else {
                        "fields"
                    },
                )
            };
            let metadata = if changes.is_empty() {
                None
            } else {
                Some(serde_json::json!({ "changes": changes }))
            };
            record_crud_audit(
                ctx,
                &identity,
                &req,
                entry,
                audit::ActionType::Update,
                id,
                summary,
                metadata,
            )
            .await;
            if wants_json {
                return Ok(super::json_api::mutation_ok_envelope(
                    admin_name,
                    id,
                    hyper::StatusCode::OK,
                ));
            }
            Ok(Response::redirect(redirect_after_save(
                intent, admin_name, id,
            )))
        }
        Err(errors) => {
            if wants_json {
                return Ok(super::json_api::validation_errors_envelope(errors));
            }
            let existing = entry.ops.find_row(&ctx.db, id).await?;
            let token = csrf_token(&req);
            let relation_options =
                render::resolve_relation_options(&ctx.admin, entry, &ctx.db).await?;
            let (global_errors, field_errors) = render::bucket_errors_by_label(entry, errors);
            // Re-fetch inlines on the error path so the operator
            // doesn't lose context while correcting the parent's
            // validation errors.
            let inlines = fetch_inline_sections(ctx, entry, Some(id)).await;
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
                inlines,
            );
            let body = ctx
                .templates
                .render_for_model(admin_name, "admin/form.html", &ctx_view)?;
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
    let body =
        ctx.templates
            .render_for_model(entry.admin_name, "admin/confirm_delete.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_delete(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    req: Request,
    id: i64,
) -> Result<Response> {
    let wants_json = super::json_api::wants_json(&req);
    let entry = match find_project_entry(&ctx.admin, admin_name) {
        Ok(e) => e,
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    };
    // Resolve the label BEFORE the delete so the audit summary
    // ("Deleted Post: Hello world") survives the row going away.
    // A row that's already gone falls back to `#<id>` — no 500.
    let label = entry
        .ops
        .object_label(&ctx.db, id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| format!("#{id}"));
    match entry.ops.delete(&ctx.db, id).await {
        Ok(()) => {}
        Err(e) if wants_json => return Ok(super::json_api::json_error(e)),
        Err(e) => return Err(e),
    }
    let summary = format!("Deleted {}: {label}", entry.singular_name);
    record_crud_audit(
        ctx,
        &identity,
        &req,
        entry,
        audit::ActionType::Delete,
        id,
        summary,
        None,
    )
    .await;
    if wants_json {
        return Ok(super::json_api::mutation_ok_envelope(
            admin_name,
            id,
            hyper::StatusCode::OK,
        ));
    }
    Ok(Response::redirect(format!("/admin/{admin_name}")))
}

// ---- Bulk delete --------------------------------------------------------
//
// Two-step server flow that mirrors the single-row delete:
//   1. POST /admin/:model/bulk_delete with `_ids=1,2,3` (no `_confirmed`)
//      → render `bulk_confirm_delete.html` listing each row.
//   2. POST same URL with `_confirmed=1` → execute `ops.delete` per id
//      and redirect back to the list with no flash (per existing
//      single-row delete behavior, the framework's flash channel is
//      not yet wired beyond login/logout).
//
// `_ids` arrives as a single comma-separated string because
// `FormData` is a flat HashMap (one value per key). The list-view
// JS builds the CSV from checked rows; the confirm form replays it
// verbatim. Bad / missing ids are silently skipped — a stale
// checkbox set shouldn't 500 the page.

const BULK_DELETE_MAX: usize = 1000;

fn parse_bulk_ids(raw: &str) -> Vec<i64> {
    raw.split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .filter(|id| *id > 0)
        .take(BULK_DELETE_MAX)
        .collect()
}

pub(crate) async fn handle_bulk_delete(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    req: &Request,
) -> Result<Response> {
    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let form = req.form()?;

    let raw_ids = form.get("_ids").unwrap_or_default();
    let ids = parse_bulk_ids(raw_ids);
    if ids.is_empty() {
        return Ok(Response::redirect(format!("/admin/{admin_name}")));
    }

    if form.bool_flag("_confirmed") {
        // Commit step. Loop per id so the existing per-row delete
        // semantics (audit trail, FK cascade, hooks) fire for each.
        // Errors short-circuit — if a delete fails midway the user
        // sees the framework error page; the rows already deleted
        // stay deleted (this matches the single-row delete path).
        for id in &ids {
            entry.ops.delete(&ctx.db, *id).await?;
        }
        return Ok(Response::redirect(format!("/admin/{admin_name}")));
    }

    // Confirm step. Resolve each id to a label so the user sees
    // exactly what they're about to delete. Missing rows (deleted
    // between selection and confirm) are dropped from the list.
    let mut items: Vec<render::BulkDeleteItem> = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(label) = entry.ops.object_label(&ctx.db, *id).await? {
            items.push(render::BulkDeleteItem { id: *id, label });
        }
    }
    if items.is_empty() {
        return Ok(Response::redirect(format!("/admin/{admin_name}")));
    }

    let view =
        render::bulk_confirm_delete_ctx(&identity, &ctx.admin, entry, items, csrf_token(req));
    let body = ctx
        .templates
        .render("admin/bulk_confirm_delete.html", &view)?;
    Ok(Response::html(body))
}

/// Generic dispatcher for project-defined bulk actions registered via
/// [`super::modeladmin::ModelAdmin::bulk_actions`]. Mirrors the
/// `handle_bulk_delete` two-step flow: the first POST renders a
/// generic confirmation page (when the action's `confirm == true`);
/// the second (with `_confirmed=1`) calls
/// `AdminOps::execute_bulk_action` and redirects.
///
/// `delete` is intentionally not routable here — it goes through the
/// cascade-aware `/bulk_delete` path. We reject it here so an
/// accidental registration of a `BulkAction { name: "delete", … }`
/// doesn't shadow the built-in.
pub(crate) async fn handle_bulk_action(
    ctx: &AdminCtx,
    identity: Identity,
    admin_name: &str,
    action_name: &str,
    req: &Request,
) -> Result<Response> {
    if action_name == "delete" {
        return Err(Error::BadRequest(
            "the action name `delete` is reserved — use POST /admin/:model/bulk_delete".into(),
        ));
    }

    let entry = find_project_entry(&ctx.admin, admin_name)?;
    let action = entry
        .bulk_actions
        .iter()
        .find(|a| a.name == action_name)
        .copied()
        .ok_or_else(|| {
            Error::NotFound(format!(
                "bulk action `{action_name}` is not registered on `{admin_name}`"
            ))
        })?;

    // Per-action permission gate (BulkAction.permission). The bulk
    // route already enforced the model's `change` permission; this
    // is a second, stricter gate so destructive actions can require
    // a scoped permission (e.g. `delete`) without making `change`
    // itself harder. Same role-bypass semantics as `perm_guard`.
    if let Some(action_perm) = action.permission {
        if !identity.role.bypasses_group_checks() {
            let singular = entry.singular_name.to_ascii_lowercase();
            let required = format!("{admin_name}.{action_perm}_{singular}");
            if !crate::auth::check_permission(&ctx.db, &identity, &required).await? {
                return Err(Error::Forbidden(format!(
                    "Bulk action `{action_name}` requires the `{required}` permission."
                )));
            }
        }
    }

    let form = req.form()?;
    let raw_ids = form.get("_ids").unwrap_or_default();
    let ids = parse_bulk_ids(raw_ids);
    if ids.is_empty() {
        return Ok(Response::redirect(format!("/admin/{admin_name}")));
    }

    // Commit step. `_confirmed` is required only when the action
    // declared `confirm: true`; one-click actions skip the confirm
    // page entirely.
    if form.bool_flag("_confirmed") || !action.confirm {
        let correlation_id = super::builtin::correlation_id_from(req);
        let ip = super::builtin::client_ip(req);
        let bulk_ctx = super::bulk::BulkActionContext {
            actor: &identity,
            correlation_id: correlation_id.as_deref(),
            ip_address: ip.as_deref(),
        };
        // Dispatch to the project's ModelAdmin override (or surface
        // the framework's "no project handler" BadRequest when the
        // action was declared but not implemented).
        let outcome = entry
            .ops
            .execute_bulk_action(&ctx.db, action.name, &ids, &bulk_ctx)
            .await?;

        // One audit row per submission. `object_id = 0` is the
        // framework's signal that this is a bulk dispatch, not a
        // single-object action; the affected ids live in
        // `metadata.ids`. The history page can render bulk rows
        // distinctly by checking `object_id == 0` and reading the
        // structured metadata.
        let failed_ids: Vec<i64> = outcome.failed.iter().map(|f| f.id).collect();
        let failure_reasons: Vec<&str> = outcome.failed.iter().map(|f| f.reason.as_str()).collect();
        let metadata = serde_json::json!({
            "kind": "bulk_action",
            "action": action.name,
            "model": entry.admin_name,
            "ids": ids,
            "succeeded": outcome.succeeded,
            "failed_ids": failed_ids,
            "failure_reasons": failure_reasons,
        });
        let summary = match outcome.message.as_deref() {
            Some(msg) => msg.to_string(),
            None => format!(
                "bulk action `{action}` on {model}: {ok} succeeded, {fail} failed",
                action = action.name,
                model = entry.admin_name,
                ok = outcome.succeeded,
                fail = outcome.failed.len(),
            ),
        };
        let action_type = if action.destructive {
            super::audit::ActionType::Delete
        } else {
            super::audit::ActionType::Update
        };
        let _ = super::audit::record(
            &ctx.db,
            super::audit::LogEntry {
                user_id: identity.user_id,
                action_type,
                model_name: entry.admin_name,
                object_id: 0,
                ip_address: ip.as_deref(),
                summary,
                correlation_id: correlation_id.as_deref(),
                session_id: None,
                metadata: Some(metadata),
                actor_user_id: None,
                event: None,
            },
        )
        .await;
        return Ok(Response::redirect(format!("/admin/{admin_name}")));
    }

    // Confirm step. Resolve labels for each id; rows that vanished
    // between selection and confirm drop out silently.
    let mut items: Vec<render::BulkDeleteItem> = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(label) = entry.ops.object_label(&ctx.db, *id).await? {
            items.push(render::BulkDeleteItem { id: *id, label });
        }
    }
    if items.is_empty() {
        return Ok(Response::redirect(format!("/admin/{admin_name}")));
    }

    let view = render::bulk_confirm_action_ctx(
        &identity,
        &ctx.admin,
        entry,
        action,
        items,
        csrf_token(req),
    );
    let body = ctx
        .templates
        .render("admin/bulk_confirm_action.html", &view)?;
    Ok(Response::html(body))
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
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(render::SidebarEntry::from)
            .collect(),
        history_entries: render::map_audit_actions(actions),
        flash: None,
    };
    let body = ctx
        .templates
        .render_for_model(admin_name, "admin/object_history.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn show_apis_index(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    let view = render::apis_index_ctx(&identity, &ctx.admin, csrf_token(req));
    let body = ctx.templates.render("admin/apis_index.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn show_feature_flags(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    let flags = super::feature_flags::list_flags(&ctx.db)
        .await
        .unwrap_or_default();
    let view = render::feature_flags_ctx(&identity, &ctx.admin, csrf_token(req), flags, None);
    let body = ctx.templates.render("admin/feature_flags.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn do_create_feature_flag(
    ctx: &AdminCtx,
    _identity: Identity,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    let key = form.get("key").unwrap_or_default().trim().to_string();
    let description = form
        .get("description")
        .unwrap_or_default()
        .trim()
        .to_string();
    if !key.is_empty() {
        let _ = super::feature_flags::create_flag(&ctx.db, &key, &description).await;
    }
    Ok(Response::redirect("/admin/feature_flags"))
}

pub(crate) async fn do_toggle_feature_flag(
    ctx: &AdminCtx,
    _identity: Identity,
    key: &str,
    req: Request,
) -> Result<Response> {
    let form = req.form()?;
    // Toggle direction comes from the form's `enabled` hidden
    // field (1 / 0) — the template sets it to the inverse of the
    // current state so two consecutive clicks flip back.
    let target = form.get("enabled").map(|s| s == "1").unwrap_or(false);
    super::feature_flags::set_flag(&ctx.db, key, target).await?;
    Ok(Response::redirect("/admin/feature_flags"))
}

pub(crate) async fn show_health(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    let checks = super::health_dashboard::gather_checks(&ctx.db).await;
    let view = render::health_ctx(&identity, &ctx.admin, csrf_token(req), checks);
    let body = ctx.templates.render("admin/health.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn show_apis_playground(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    let view = render::playground_ctx(&identity, &ctx.admin, csrf_token(req));
    let body = ctx.templates.render("admin/apis_playground.html", &view)?;
    Ok(Response::html(body))
}

pub(crate) async fn show_log_entries(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    ensure_audit_ready(&ctx.db).await;
    // Per-actor filter: `?user_id=N` narrows the audit feed to one
    // operator's actions. Non-numeric / non-positive values drop
    // silently — the feed renders unfiltered rather than 4xx-ing on a
    // bad URL.
    let qs = req.query();
    let user_filter: Option<i64> = qs
        .get("user_id")
        .and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|n| *n > 0);
    let actions = audit::recent(&ctx.db, 100, None, None, user_filter)
        .await
        .unwrap_or_default();
    // Resolve the filtered user's display label (email) so the
    // active-filter banner can render "Showing actions by <email>"
    // instead of a bare id. A missing user falls back to `#<id>`.
    let user_filter_label: Option<String> = if let Some(uid) = user_filter {
        let email: Option<String> =
            sqlx::query_scalar("SELECT email FROM rustio_users WHERE id = $1")
                .bind(uid)
                .fetch_optional(ctx.db.pool())
                .await
                .ok()
                .flatten();
        Some(email.unwrap_or_else(|| format!("#{uid}")))
    } else {
        None
    };
    let view = render::LogEntriesCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: "Recent admin actions",
        entries: ctx
            .admin
            .entries()
            .iter()
            .filter(|e| !e.core)
            .map(render::SidebarEntry::from)
            .collect(),
        history_entries: render::map_audit_actions(actions),
        flash: None,
        user_filter_label,
    };
    let body = ctx.templates.render("admin/log_entries.html", &view)?;
    Ok(Response::html(body))
}

// ---- Self-service password change ---------------------------------------

pub(crate) async fn show_password_change(
    ctx: &AdminCtx,
    identity: Identity,
    req: &Request,
) -> Result<Response> {
    // R1 commit #11 — PRG terminus. After a successful POST,
    // `do_password_change` redirects here with `?changed=1`; the
    // GET-time render of the success card is idempotent, so a
    // browser refresh re-renders the same confirmation without
    // re-POSTing.
    let just_changed = req.query().get("changed").is_some();
    let min_length = ctx.admin.active_password_policy().min_length();
    let view = render::PasswordChangeCtx {
        base: BaseContext::new(Some(&identity), csrf_token(req), &ctx.admin),
        page_title: if just_changed {
            "Password changed"
        } else {
            "Change password"
        },
        errors: Vec::new(),
        success: just_changed,
        sections: if just_changed {
            Vec::new()
        } else {
            render::password_change_form_sections(min_length)
        },
    };
    let body = ctx.templates.render("admin/password_change.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/password_change` — authenticated self-change flow.
///
/// R1 commit #11 brought this handler into parity with the recovery
/// doctrine (`DESIGN_RECOVERY.md` §5.2 + §14.2):
///
/// - **PasswordPolicy is the single source of truth.** The legacy
///   inline `MIN_PASSWORD_LEN` length check is gone; validation
///   now goes through `admin.active_password_policy().validate(...)`.
/// - **Doctrine 22.** After a successful change, every session
///   except the current one is revoked via
///   `invalidate_sessions(SessionTarget::UserExceptCurrent, …,
///   SessionInvalidationReason::UserRequested)`. The current
///   device stays signed in.
/// - **Audit.** One `AuditEvent::PasswordChangedSelf` row plus one
///   `AuditEvent::SessionsRevokedSelf` per revoked id, all sharing
///   the request's `correlation_id`.
/// - **PRG.** On success, returns 303 → /admin/password_change?changed=1
///   so a browser refresh doesn't replay the POST.
///
/// On any validation failure (wrong old password, mismatched
/// confirms, policy rejection), the form re-renders with field-
/// level errors and **no DB mutation** — the user's old password
/// is still in place; sessions are untouched.
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

    // ---- Validation (no DB mutation on failure). ----
    //
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
    if let Err(policy_err) = ctx.admin.active_password_policy().validate(new1) {
        // PasswordPolicyError's Display impl is plaintext-free
        // (commit #4 leak-prevention test pins this); rendering
        // it directly to the form field is safe.
        let msg = policy_err.to_string();
        errors.push(msg.clone());
        field_errors
            .entry("new_password1".into())
            .or_default()
            .push(msg);
    }

    if !errors.is_empty() {
        let min_length = ctx.admin.active_password_policy().min_length();
        let mut sections = render::password_change_form_sections(min_length);
        render::apply_field_errors(&mut sections, &field_errors);
        let view = render::PasswordChangeCtx {
            base: BaseContext::new(Some(&identity), csrf_token(&req), &ctx.admin),
            page_title: "Change password",
            errors,
            success: false,
            sections,
        };
        let body = ctx.templates.render("admin/password_change.html", &view)?;
        return Ok(Response::html(body).with_status(hyper::StatusCode::BAD_REQUEST));
    }

    // ---- Validation passed; mutate. ----

    // 1. Hash + write the new password. `set_password` (commit #2)
    //    stamps `password_changed_at` on the same UPDATE.
    auth::set_password(&ctx.db, user.id, new1).await?;

    // 2. Doctrine 22 — revoke other sessions through the centralised
    //    invalidation API. Current device stays signed in.
    //
    //    Edge case: cookie expired between page-load and this POST
    //    (`current_session_id` resolves to `None`). Fall through to
    //    revoking ALL sessions — the user is on a stale cookie
    //    anyway, and the framework's auth middleware will redirect
    //    to login on the next request.
    let cookie_token = req
        .header("cookie")
        .and_then(crate::auth::session_token_from_cookie);
    let current_session_id = match &cookie_token {
        Some(t) => crate::auth::current_session_id(&ctx.db, t).await?,
        None => None,
    };
    let target = match current_session_id {
        Some(sid) => crate::auth::SessionTarget::UserExceptCurrent {
            user_id: user.id,
            current_session_id: sid,
        },
        None => crate::auth::SessionTarget::User { user_id: user.id },
    };
    let outcome = crate::auth::invalidate_sessions(
        &ctx.db,
        target,
        crate::auth::SessionInvalidationReason::UserRequested,
    )
    .await?;
    let revoked_session_count = outcome.revoked_session_ids.len();

    // 3. Audit — one `PasswordChangedSelf` row + one
    //    `SessionsRevokedSelf` per revoked id, all sharing the
    //    request's correlation_id so a future
    //    /admin/history/<correlation_id> page reconstructs the chain.
    //
    //    Audit-write failure is non-fatal — log + continue. Same
    //    trade-off documented in `auth::recovery::consume_reset_token`
    //    (commit #7) and `do_revoke_session` (commit #10): the
    //    password is already changed and sessions already revoked
    //    when the audit insert runs; refusing to redirect would
    //    leave the user staring at a 500 page wondering whether
    //    the action took effect.
    let cid_owned = req
        .ctx()
        .get::<crate::middleware::CorrelationId>()
        .map(|c| c.0.clone());
    let ip_owned = req
        .header("x-forwarded-for")
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let metadata = serde_json::json!({
        "invalidated_session_count": revoked_session_count,
    });
    let mut entry = audit::LogEntry::new(user.id, audit::ActionType::Update, "users", user.id)
        .with_event(audit::AuditEvent::PasswordChangedSelf);
    entry.correlation_id = cid_owned.as_deref();
    entry.ip_address = ip_owned.as_deref();
    entry.metadata = Some(metadata);
    entry.summary =
        format!("password changed by user; {revoked_session_count} other session(s) revoked");
    if let Err(e) = audit::record(&ctx.db, entry).await {
        log::error!(
            target: "rustio_admin::password_change",
            "audit::record (PasswordChangedSelf) failed for user_id={}: {}",
            user.id, e,
        );
    }

    record_session_revocations(
        ctx,
        &identity,
        &outcome.revoked_session_ids,
        &req,
        "password_change",
    )
    .await;

    // 4. PRG: redirect to GET so a refresh doesn't replay the POST.
    //    `show_password_change` reads `?changed=1` and renders the
    //    success card idempotently.
    Ok(Response::redirect("/admin/password_change?changed=1"))
}

/// `GET /admin/account/sessions` — read-only listing of the current
/// user's active sessions.
///
/// Doctrine 7 (active sessions UX) treats this as a core security
/// surface. The page shows every active session row for `identity`
/// with: trust level, source IP (best-effort), short user-agent
/// summary, created-at, last-seen-at, and a marker on the current
/// session.
///
/// R0 ships **read-only**. Revoke buttons (`POST /admin/account/sessions/revoke`,
/// `POST /admin/account/sessions/revoke-others`) land in R1 once
/// the centralized invalidate_sessions API is fully exercised.
pub(crate) async fn show_account_sessions(
    ctx: &AdminCtx,
    identity: crate::auth::Identity,
    req: &Request,
) -> Result<Response> {
    // Resolve the cookie token → current session id so the template
    // can mark the current device.
    let cookie_token = req
        .header("cookie")
        .and_then(crate::auth::session_token_from_cookie);
    let current_session_id = match &cookie_token {
        Some(t) => crate::auth::current_session_id(&ctx.db, t).await?,
        None => None,
    };
    let sessions = crate::auth::list_active_for_user(&ctx.db, identity.user_id).await?;

    let view = render::account_sessions_ctx(
        &identity,
        &ctx.admin,
        sessions,
        current_session_id,
        csrf_token(req),
    );
    let body = ctx.templates.render("admin/account_sessions.html", &view)?;
    Ok(Response::html(body))
}

/// `POST /admin/account/sessions/<id>/revoke` — revoke a specific
/// non-current session (R1 commit #10; `DESIGN_RECOVERY.md` §5.1).
///
/// Rejected if `session_id` matches the current device — the user
/// should use `POST /admin/account/sessions/revoke-all` for that.
/// Defense-in-depth check; the template does not expose the button
/// for the current row.
pub(crate) async fn do_revoke_session(
    ctx: &AdminCtx,
    identity: crate::auth::Identity,
    req: Request,
    session_id: i64,
) -> Result<Response> {
    let cookie_token = req
        .header("cookie")
        .and_then(crate::auth::session_token_from_cookie);
    let current_session_id = match &cookie_token {
        Some(t) => crate::auth::current_session_id(&ctx.db, t).await?,
        None => None,
    };
    revoke_session_verdict(session_id, current_session_id)?;

    let outcome = crate::auth::invalidate_sessions(
        &ctx.db,
        crate::auth::SessionTarget::Single { session_id },
        crate::auth::SessionInvalidationReason::UserRequested,
    )
    .await?;

    record_session_revocations(ctx, &identity, &outcome.revoked_session_ids, &req, "single").await;

    Ok(Response::redirect("/admin/account/sessions"))
}

/// `POST /admin/account/sessions/revoke-others` — revoke every
/// session for the user except the current one
/// (`SessionTarget::UserExceptCurrent`). The user stays signed in
/// on this device.
///
/// Edge case: when the cookie has expired between landing on the
/// page and clicking the button (`current_session_id` resolves to
/// `None`), we fall through to `SessionTarget::User` — there's no
/// current device to keep alive, and the user is about to be
/// redirected anyway when the framework's auth middleware notices
/// the dead cookie on the next request.
pub(crate) async fn do_revoke_other_sessions(
    ctx: &AdminCtx,
    identity: crate::auth::Identity,
    req: Request,
) -> Result<Response> {
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

    record_session_revocations(ctx, &identity, &outcome.revoked_session_ids, &req, "others").await;

    Ok(Response::redirect("/admin/account/sessions"))
}

/// `POST /admin/account/sessions/revoke-all` — revoke every session
/// for the user, including the current one. The handler additionally
/// clears the session cookie and redirects to `/admin/login?logout=1`
/// so the existing logout flash banner ("You've been signed out.")
/// surfaces on the login page.
pub(crate) async fn do_revoke_all_sessions(
    ctx: &AdminCtx,
    identity: crate::auth::Identity,
    req: Request,
) -> Result<Response> {
    let outcome = crate::auth::invalidate_sessions(
        &ctx.db,
        crate::auth::SessionTarget::User {
            user_id: identity.user_id,
        },
        crate::auth::SessionInvalidationReason::UserRequested,
    )
    .await?;

    record_session_revocations(ctx, &identity, &outcome.revoked_session_ids, &req, "all").await;

    let clear_cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        crate::auth::SESSION_COOKIE
    );
    Ok(Response::redirect("/admin/login?logout=1").with_header("set-cookie", clear_cookie))
}

/// Pure verdict for the per-session revoke route. Returns
/// `Err(BadRequest(...))` when `target_id` matches the current
/// session id; the renderer maps that to a clean error page.
///
/// Pulled out so the defense-in-depth check can be unit-tested
/// without a Db. The UI-side check (template hides the button on
/// the current row) is a courtesy; THIS function is the security
/// boundary.
fn revoke_session_verdict(target_id: i64, current_session_id: Option<i64>) -> Result<()> {
    if Some(target_id) == current_session_id {
        return Err(Error::BadRequest(
            "You can't revoke your current session here. Use \"Sign out everywhere\" \
             to revoke every session including this one."
                .into(),
        ));
    }
    Ok(())
}

/// Emit one `AuditEvent::SessionsRevokedSelf` row per revoked
/// session id. All rows share the request's `correlation_id` so a
/// future `/admin/history/<correlation_id>` page reconstructs the
/// chain.
///
/// Audit-write failure is non-fatal — same trade-off documented in
/// `auth::recovery::consume_reset_token`'s commit-#7 rationale: the
/// sessions are ALREADY revoked (the immutable side effect ran
/// inside `invalidate_sessions`); refusing to redirect would leave
/// the user staring at a 500 page wondering whether the action took
/// effect. We log + continue.
pub(super) async fn record_session_revocations(
    ctx: &AdminCtx,
    identity: &crate::auth::Identity,
    revoked_ids: &[i64],
    req: &Request,
    via: &'static str,
) {
    let cid_owned = req
        .ctx()
        .get::<crate::middleware::CorrelationId>()
        .map(|c| c.0.clone());
    let ip_owned = req
        .header("x-forwarded-for")
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    for revoked_id in revoked_ids {
        let metadata = serde_json::json!({
            "session_id": revoked_id,
            "reason": "user_requested",
            "via": via,
        });
        let mut entry = audit::LogEntry::new(
            identity.user_id,
            audit::ActionType::Update,
            "user",
            identity.user_id,
        )
        .with_event(audit::AuditEvent::SessionsRevokedSelf);
        entry.correlation_id = cid_owned.as_deref();
        entry.ip_address = ip_owned.as_deref();
        entry.metadata = Some(metadata);
        entry.summary = format!("session {revoked_id} revoked by user (via {via})");
        if let Err(e) = audit::record(&ctx.db, entry).await {
            log::error!(
                target: "rustio_admin::sessions::revoke",
                "audit::record failed for revoked session_id={} user_id={} via={}: {}",
                revoked_id, identity.user_id, via, e,
            );
        }
    }
}

#[cfg(test)]
mod revoke_session_verdict_tests {
    use super::revoke_session_verdict;
    use crate::error::Error;

    #[test]
    fn blocks_revoking_current_session() {
        let r = revoke_session_verdict(42, Some(42));
        assert!(matches!(r, Err(Error::BadRequest(_))));
    }

    #[test]
    fn allows_revoking_different_session() {
        assert!(revoke_session_verdict(42, Some(99)).is_ok());
    }

    #[test]
    fn allows_when_no_current_session_resolved() {
        // Edge case: cookie missing / expired between page-load and
        // POST. Allow — there's no current session to protect.
        assert!(revoke_session_verdict(42, None).is_ok());
    }

    #[test]
    fn error_message_directs_user_to_revoke_all() {
        let err = revoke_session_verdict(42, Some(42)).unwrap_err();
        let msg = err.client_message();
        assert!(
            msg.contains("Sign out everywhere"),
            "error must direct user to the right action: {msg}"
        );
    }
}

#[cfg(test)]
mod parse_date_yyyy_mm_dd_tests {
    use super::parse_date_yyyy_mm_dd;

    #[test]
    fn accepts_canonical_form() {
        assert_eq!(
            parse_date_yyyy_mm_dd("2026-05-19"),
            Some("2026-05-19".to_string()),
        );
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(
            parse_date_yyyy_mm_dd("  2026-01-01\t"),
            Some("2026-01-01".to_string()),
        );
    }

    #[test]
    fn rejects_empty_and_garbage() {
        assert_eq!(parse_date_yyyy_mm_dd(""), None);
        assert_eq!(parse_date_yyyy_mm_dd("   "), None);
        assert_eq!(parse_date_yyyy_mm_dd("not-a-date"), None);
        // Wrong separator — the format spec is strict on hyphens.
        assert_eq!(parse_date_yyyy_mm_dd("2026/05/19"), None);
        // Impossible date — chrono rejects.
        assert_eq!(parse_date_yyyy_mm_dd("2026-02-30"), None);
        // No SQL-injection vector reaches the database — anything
        // that's not a real date is None.
        assert_eq!(parse_date_yyyy_mm_dd("2026-01-01' OR '1'='1"), None);
    }

    #[test]
    fn normalises_unpadded_input_to_canonical_form() {
        // chrono's `%Y-%m-%d` accepts unpadded month / day. We
        // re-emit with `format("%Y-%m-%d")` so what gets bound to
        // Postgres is always zero-padded.
        assert_eq!(
            parse_date_yyyy_mm_dd("2026-1-1"),
            Some("2026-01-01".to_string()),
        );
    }
}

/// Resolve every foreign-key cell on the current list page from raw
/// id (`"5"`) to the target row's display label (`"Anna Lindqvist"`)
/// and remember the target's admin URL so the renderer can wrap the
/// cell in an `<a>`.
///
/// Called by the list handler immediately after `entry.ops.list`. Runs
/// at most one SELECT per FK column on the entry — so a list of 50
/// orders with four FK columns issues exactly four extra queries
/// regardless of page size, an N+1-safe batch design.
///
/// The hydration is silent on failure: a target row that's been
/// deleted (or a relation that points at a missing model) leaves the
/// cell holding the raw id with no link, so the user still sees
/// *something* and the page never 500s on a stale FK.
async fn hydrate_fk_cells(
    db: &Db,
    admin: &Admin,
    entry: &super::types::AdminEntry,
    rows: &mut [super::types::ListRow],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let registry = super::relations::RelationRegistry::from_admin_entries(admin.entries());
    if registry.is_empty() {
        return Ok(());
    }

    // Iterate every column on the entry — relation-bearing columns are
    // the only ones we hydrate. The cell index inside `ListRow.cells`
    // is the same as the field index inside `entry.fields` (positional
    // contract upheld by `display_values()`).
    for (idx, field) in entry.fields.iter().enumerate() {
        let Some(rel) = registry.belongs_to(entry.singular_name, field.name) else {
            continue;
        };
        let Some(display_field) = &rel.target_display_field else {
            // No display field on target → leave the raw id, no link.
            continue;
        };

        // Collect distinct ids in this column on the current page.
        // `OptionalI64` cells render empty when `None`; skip those.
        let mut ids: Vec<i64> = Vec::with_capacity(rows.len());
        for row in rows.iter() {
            let Some(cell) = row.cells.get(idx) else {
                continue;
            };
            if cell.is_empty() {
                continue;
            }
            if let Ok(parsed) = cell.parse::<i64>() {
                ids.push(parsed);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() {
            continue;
        }

        // Batch fetch (id, display) pairs for every distinct FK value
        // observed on this page. One round-trip per FK column.
        let sql = format!(
            "SELECT id, {display}::text AS label FROM {table} WHERE id = ANY($1)",
            display = display_field,
            table = rel.target_table,
        );
        let fetched = match sqlx::query_as::<_, (i64, String)>(&sql)
            .bind(&ids)
            .fetch_all(db.pool())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                log::warn!(
                    "FK hydration skipped for {}.{} → {}: {e}",
                    entry.singular_name,
                    field.name,
                    rel.target_model,
                );
                continue;
            }
        };
        let labels: HashMap<i64, String> = fetched.into_iter().collect();

        // Substitute label + record link target on every matching cell.
        for row in rows.iter_mut() {
            let Some(cell) = row.cells.get_mut(idx) else {
                continue;
            };
            if cell.is_empty() {
                continue;
            }
            let Ok(parsed) = cell.parse::<i64>() else {
                continue;
            };
            if let Some(label) = labels.get(&parsed) {
                *cell = label.clone();
                if let Some(slot) = row.cell_links.get_mut(idx) {
                    *slot = Some(super::types::CellLink {
                        admin_name: rel.target_admin_name.clone(),
                        id: parsed,
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod csv_escape_tests {
    use super::csv_escape_field;

    #[test]
    fn plain_text_passes_through_unquoted() {
        assert_eq!(csv_escape_field("hello"), "hello");
        assert_eq!(csv_escape_field("42"), "42");
        assert_eq!(csv_escape_field("anna.lindqvist"), "anna.lindqvist");
        assert_eq!(csv_escape_field(""), "");
    }

    #[test]
    fn comma_triggers_quoting() {
        assert_eq!(csv_escape_field("a,b"), "\"a,b\"");
    }

    #[test]
    fn embedded_quote_is_doubled_inside_quoted_field() {
        // RFC 4180: a `"` inside a quoted field is escaped by
        // doubling it. The whole field is then wrapped in `"`.
        assert_eq!(csv_escape_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn newline_triggers_quoting() {
        assert_eq!(csv_escape_field("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(csv_escape_field("crlf\r\nhere"), "\"crlf\r\nhere\"");
    }

    #[test]
    fn no_quote_added_when_only_safe_punctuation() {
        // Dots, semicolons, dashes, colons — none trigger quoting.
        assert_eq!(csv_escape_field("a.b-c:d;e"), "a.b-c:d;e");
    }

    #[test]
    fn unicode_passes_through_unchanged_when_no_special_chars() {
        assert_eq!(csv_escape_field("Anna Wåhlin"), "Anna Wåhlin");
        assert_eq!(csv_escape_field("日本語"), "日本語");
    }
}

#[cfg(test)]
mod compute_row_diff_tests {
    use super::*;
    use crate::admin::types::{AdminEntry, AdminField, EditRow, FieldType};

    fn field(name: &'static str, label: &'static str, editable: bool) -> AdminField {
        AdminField {
            name,
            label,
            field_type: FieldType::String,
            editable,
            relation: None,
            choices: None,
        }
    }

    fn row(values: &[(&str, &str)]) -> EditRow {
        EditRow {
            id: 1,
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn entry_with(fields: &'static [AdminField]) -> AdminEntry {
        // Minimal stub — only `fields` matters for compute_row_diff.
        // Other AdminEntry slots are read elsewhere; for this test
        // we can leave them at default-shaped sentinels.
        // Construct via the same `core_user_entry`-style literal:
        // we only need a valid `fields` slice on it.
        AdminEntry {
            admin_name: "test",
            display_name: "Test",
            singular_name: "Test",
            table: "test",
            fields,
            core: false,
            list_display: &[],
            list_filter: &[],
            search_fields: &[],
            ordering: &[],
            list_per_page: 50,
            readonly_fields: &[],
            fieldsets: &[],
            bulk_actions: &[],
            inlines: &[],
            ops: std::sync::Arc::new(StubOps),
        }
    }

    /// Stub `AdminOps` used only by entry_with(); compute_row_diff
    /// never calls into it.
    struct StubOps;
    impl crate::admin::types::AdminOps for StubOps {
        fn list<'a>(
            &'a self,
            _db: &'a crate::orm::Db,
            _opts: crate::admin::types::ListOpts,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = crate::error::Result<crate::admin::types::ListPage>,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async { Ok(crate::admin::types::ListPage::default()) })
        }
        fn find_row<'a>(
            &'a self,
            _db: &'a crate::orm::Db,
            _id: i64,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = crate::error::Result<Option<EditRow>>> + Send + 'a,
            >,
        > {
            Box::pin(async { Ok(None) })
        }
        fn create<'a>(
            &'a self,
            _db: &'a crate::orm::Db,
            _form: &'a crate::http::FormData,
        ) -> crate::admin::types::CreateResult<'a> {
            Box::pin(async { Ok(Ok(0)) })
        }
        fn update<'a>(
            &'a self,
            _db: &'a crate::orm::Db,
            _id: i64,
            _form: &'a crate::http::FormData,
        ) -> crate::admin::types::UpdateResult<'a> {
            Box::pin(async { Ok(Ok(())) })
        }
        fn delete<'a>(
            &'a self,
            _db: &'a crate::orm::Db,
            _id: i64,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = crate::error::Result<()>> + Send + 'a>,
        > {
            Box::pin(async { Ok(()) })
        }
        fn object_label<'a>(
            &'a self,
            _db: &'a crate::orm::Db,
            _id: i64,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = crate::error::Result<Option<String>>> + Send + 'a>,
        > {
            Box::pin(async { Ok(None) })
        }
        fn execute_bulk_action<'a>(
            &'a self,
            _db: &'a crate::orm::Db,
            _name: &'a str,
            _ids: &'a [i64],
            _ctx: &'a crate::admin::bulk::BulkActionContext<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = crate::error::Result<crate::admin::bulk::BulkActionResult>,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async { Ok(crate::admin::bulk::BulkActionResult::default()) })
        }
    }

    #[test]
    fn diff_reports_changed_fields_only() {
        const FIELDS: &[AdminField] = &[
            AdminField {
                name: "title",
                label: "Title",
                field_type: FieldType::String,
                editable: true,
                relation: None,
                choices: None,
            },
            AdminField {
                name: "body",
                label: "Body",
                field_type: FieldType::String,
                editable: true,
                relation: None,
                choices: None,
            },
            AdminField {
                name: "slug",
                label: "Slug",
                field_type: FieldType::String,
                editable: true,
                relation: None,
                choices: None,
            },
        ];
        let entry = entry_with(FIELDS);
        let before = row(&[("title", "Old"), ("body", "Same"), ("slug", "alpha")]);
        let after = row(&[("title", "New"), ("body", "Same"), ("slug", "beta")]);
        let diff = compute_row_diff(&entry, &before, &after);
        let names: Vec<&str> = diff.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(names, vec!["title", "slug"]);
        // Labels and before/after pulled through verbatim.
        assert_eq!(diff[0].label, "Title");
        assert_eq!(diff[0].from, "Old");
        assert_eq!(diff[0].to, "New");
    }

    #[test]
    fn diff_skips_non_editable_fields() {
        // Auto-touched columns (updated_at, password_hash) shouldn't
        // surface in the diff — they aren't user-driven changes.
        const FIELDS: &[AdminField] = &[
            AdminField {
                name: "title",
                label: "Title",
                field_type: FieldType::String,
                editable: true,
                relation: None,
                choices: None,
            },
            AdminField {
                name: "updated_at",
                label: "Updated at",
                field_type: FieldType::DateTime,
                editable: false,
                relation: None,
                choices: None,
            },
        ];
        let entry = entry_with(FIELDS);
        let before = row(&[("title", "A"), ("updated_at", "2026-01-01T00:00:00Z")]);
        let after = row(&[("title", "B"), ("updated_at", "2026-05-19T12:34:56Z")]);
        let diff = compute_row_diff(&entry, &before, &after);
        let names: Vec<&str> = diff.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(names, vec!["title"]);
    }

    #[test]
    fn diff_handles_unset_to_set_transition() {
        // Missing-in-before == "" by the lookup contract. The diff
        // reports it as a real change so "previously NULL → now
        // X" surfaces clearly on the history page.
        const FIELDS: &[AdminField] = &[AdminField {
            name: "subtitle",
            label: "Subtitle",
            field_type: FieldType::OptionalString,
            editable: true,
            relation: None,
            choices: None,
        }];
        let entry = entry_with(FIELDS);
        let before = row(&[("subtitle", "")]);
        let after = row(&[("subtitle", "Now set")]);
        let diff = compute_row_diff(&entry, &before, &after);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].from, "");
        assert_eq!(diff[0].to, "Now set");
    }

    #[test]
    fn diff_empty_when_nothing_changed() {
        const FIELDS: &[AdminField] = &[AdminField {
            name: "title",
            label: "Title",
            field_type: FieldType::String,
            editable: true,
            relation: None,
            choices: None,
        }];
        let entry = entry_with(FIELDS);
        let before = row(&[("title", "Same")]);
        let after = row(&[("title", "Same")]);
        let diff = compute_row_diff(&entry, &before, &after);
        assert!(diff.is_empty());
    }
}

#[cfg(test)]
mod sanitise_upload_filename_tests {
    use super::sanitise_upload_filename;

    #[test]
    fn plain_ascii_name_passes_through() {
        assert_eq!(sanitise_upload_filename("photo.png"), "photo.png");
        assert_eq!(
            sanitise_upload_filename("Report-2026_v2.pdf"),
            "Report-2026_v2.pdf"
        );
    }

    #[test]
    fn strips_unix_and_windows_path_components() {
        assert_eq!(sanitise_upload_filename("/etc/passwd"), "passwd");
        assert_eq!(sanitise_upload_filename("../../escape.png"), "escape.png");
        assert_eq!(
            sanitise_upload_filename("C:\\Users\\admin\\evil.exe"),
            "evil.exe"
        );
    }

    #[test]
    fn replaces_unsafe_chars_with_underscore() {
        assert_eq!(
            sanitise_upload_filename("hello world.png"),
            "hello_world.png"
        );
        assert_eq!(sanitise_upload_filename("a;b&c.txt"), "a_b_c.txt");
        // Non-ASCII becomes `_` too — the goal is on-disk safety,
        // not Unicode preservation.
        assert_eq!(sanitise_upload_filename("résumé.pdf"), "r_sum_.pdf");
    }

    #[test]
    fn trims_leading_dots() {
        assert_eq!(sanitise_upload_filename(".htaccess"), "htaccess");
        assert_eq!(sanitise_upload_filename("...secret"), "secret");
    }

    #[test]
    fn caps_length() {
        let long = "a".repeat(500);
        let out = sanitise_upload_filename(&long);
        assert_eq!(out.chars().count(), 120);
    }

    #[test]
    fn empty_or_pathological_falls_back_to_file() {
        assert_eq!(sanitise_upload_filename(""), "file");
        assert_eq!(sanitise_upload_filename("///"), "file");
        assert_eq!(sanitise_upload_filename("..."), "file");
    }
}
