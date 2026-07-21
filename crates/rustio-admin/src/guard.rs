//! Public session guard for project-defined (custom) routes.
//!
//! Apps that mount their own routes on the admin `Router` — a bespoke operational
//! page, say — use [`require_admin_session`] to gate them with the same
//! admin-session check the framework's own routes apply. This is the supported
//! way to protect a custom route: do **not** hand-roll cookie / session parsing.

use crate::auth::{self, Identity};
use crate::error::Result;
use crate::http::{Request, Response};
use crate::orm::Db;

/// Outcome of [`require_admin_session`].
pub enum SessionGuard {
    /// A valid, active admin session — carries the signed-in identity.
    Authenticated(Identity),
    /// No valid session. Return this redirect to the admin login from the handler.
    Redirect(Response),
}

/// Authenticate a custom route against the admin session cookie.
///
/// Returns [`SessionGuard::Authenticated`] with the signed-in [`Identity`] when a
/// valid, active session is present, else [`SessionGuard::Redirect`] to the admin
/// login. It reuses the same session primitives
/// ([`session_token_from_cookie`](auth::session_token_from_cookie) /
/// [`identity_from_session`](auth::identity_from_session)) as the framework's
/// internal route guard, so a custom route gets the same base authentication.
///
/// This is the **base session guard**: it does not itself apply the MFA or
/// forced-password-rotation policy gates the framework layers on its own routes —
/// those remain the framework routes' concern. It is the right primitive for
/// gating a read-only operational page behind "is this a signed-in operator?".
///
/// # Example
///
/// ```ignore
/// use rustio_admin::{require_admin_session, SessionGuard, Response};
///
/// let router = router.get("/console/my-page", move |req| {
///     let db = db.clone();
///     async move {
///         match require_admin_session(&db, &req).await? {
///             SessionGuard::Redirect(r) => Ok(r),
///             SessionGuard::Authenticated(_ident) => Ok(Response::html("…".into())),
///         }
///     }
/// });
/// ```
pub async fn require_admin_session(db: &Db, req: &Request) -> Result<SessionGuard> {
    let to_login = || SessionGuard::Redirect(Response::redirect("/admin/login"));

    let Some(cookie) = req.header("cookie") else {
        return Ok(to_login());
    };
    let Some(token) = auth::session_token_from_cookie(cookie) else {
        return Ok(to_login());
    };
    let Some(ident) = auth::identity_from_session(db, &token).await? else {
        return Ok(to_login());
    };
    if !ident.is_active {
        return Ok(to_login());
    }
    Ok(SessionGuard::Authenticated(ident))
}
