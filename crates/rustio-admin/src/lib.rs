//! rustio-admin — Django Admin, but for Rust.
//!
//! This crate is the public face of the framework. Phase 2 ships the
//! HTTP / router / server / ORM / migrations / templates core; later
//! phases populate `auth` and `admin`.

#![forbid(unsafe_code)]

pub mod admin;
pub mod auth;
pub mod background;
pub mod email;
pub mod error;
pub mod http;
pub mod middleware;
pub mod migrations;
pub mod orm;
pub mod router;
pub mod server;
pub mod templates;

pub use crate::admin::{
    register_admin_routes, Admin, AdminField, AdminModel, FieldType, Fieldset, ModelAdmin,
};
pub use crate::auth::{Identity, Role};
pub use crate::error::{Error, Result};
pub use crate::http::{FormData, Request, Response};
pub use crate::orm::{Db, DbOptions, Model, Row, Value};
pub use crate::router::{Next, Router};
pub use crate::server::Server;

pub use rustio_admin_macros::RustioAdmin;

// `RustioAdmin` emits `::rustio_admin::*` paths in its expansion. That
// resolves cleanly for downstream consumers, but inside this crate's
// own compilation unit `rustio_admin` isn't a known extern. Aliasing
// the crate to itself under `cfg(test)` lets the macro be exercised
// from this crate's own tests without changing any non-test build.
#[cfg(test)]
extern crate self as rustio_admin;

/// Test-only re-exports for the integration-test suite under
/// `tests/integration_*.rs`. NOT part of the public API — the
/// module is `#[doc(hidden)]` and gated behind the
/// `integration-test` Cargo feature, so a regular
/// `cargo build` / `cargo test --workspace` cannot reach it.
///
/// Re-exports the otherwise-`pub(crate)` runtime surface of
/// `auth::recovery_admin` so the integration tests can exercise
/// `record_failed_login`, `admin_set_temp_password`, etc. without
/// promoting the internal API to permanent `pub` visibility.
///
/// Plus a `fake_request()` builder for runtime fns that take
/// `&Request` (currently `lock_user_account`,
/// `unlock_user_account`, `admin_revoke_sessions`,
/// `issue_admin_reset_token`, `admin_set_temp_password`). The
/// fake request has no headers — `client_ip` and
/// `correlation_id_from` both return `None`, which is the
/// neutral state for the audit + logging layers.
///
/// See `DESIGN_R2_ORGANISATIONAL.md` §10.3 for the integration-
/// test plan.
#[doc(hidden)]
#[cfg(feature = "integration-test")]
pub mod __integration {
    pub use crate::auth::recovery_admin::{
        admin_revoke_sessions, admin_set_temp_password, check_account_lockout,
        check_session_elevated, issue_admin_reset_token, lock_user_account,
        promote_session_elevated, record_failed_login, record_successful_login,
        unlock_user_account, AdminActor, AdminIssueOutcome, AdminRevokeOutcome, AdminTempPwOutcome,
        LockDuration, LockOutcome, LockState, ThrottleOutcome, UnlockOutcome,
    };

    /// Construct a minimal [`crate::http::Request`] for integration
    /// tests. POST to `/test`, no headers, no body, no params.
    /// Adequate for runtime fns that read only `client_ip` (None)
    /// and `correlation_id_from` (None) from the request.
    pub fn fake_request() -> crate::http::Request {
        use std::collections::HashMap;
        crate::http::Request::__integration_test_fake("/test".to_string(), HashMap::new())
    }
}
