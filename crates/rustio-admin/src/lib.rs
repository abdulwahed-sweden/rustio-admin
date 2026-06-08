//! rustio-admin — Django Admin, but for Rust.
//!
//! This crate is the public face of the framework. Phase 2 ships the
//! HTTP / router / server / ORM / migrations / templates core; later
//! phases populate `auth` and `admin`.

#![forbid(unsafe_code)]

// public:
pub mod admin;
// public:
pub mod auth;
// public:
pub mod background;
// public:
pub mod email;
// public:
pub mod error;
// public:
pub mod http;
// public:
pub mod middleware;
// public:
pub(crate) mod meta;
pub mod migrations;
pub(crate) mod multipart;
// public:
pub mod orm;
// public:
pub mod router;
// public:
pub mod server;
// public:
pub mod templates;

// public:
pub use crate::admin::{
    register_admin_routes, Admin, AdminField, AdminModel, BulkAction, BulkActionContext,
    BulkActionFailure, BulkActionResult, FieldType, FieldValidationError, Fieldset, Inline,
    ModelAdmin,
};
// public:
pub use crate::auth::{Identity, Role};
// public:
pub use crate::error::{Error, Result};
// public:
pub use crate::http::{FormData, Request, Response};
// public:
pub use crate::orm::{Db, DbOptions, Model, Row, Value};
// public:
pub use crate::router::{Next, Router};
// public:
pub use crate::server::Server;
// public:
pub use crate::templates::{embedded_template_names, embedded_template_source};

// public:
pub use rustio_admin_macros::RustioAdmin;

// public: re-exported runtime dependencies.
//
// A downstream model crate depends on `rustio-admin` alone: the
// `RustioAdmin` derive emits paths through these re-exports
// (`::rustio_admin::chrono::…`, `::rustio_admin::rust_decimal::…`,
// `::rustio_admin::uuid::…`), and a model's own field types resolve via
// the bare type aliases below (`use rustio_admin::{Decimal, DateTime,
// Utc, Uuid};`). This is what lets a scaffolded `Cargo.toml` drop its
// direct `chrono` / `uuid` / `rust_decimal` / `sqlx` dependencies — the
// framework already owns those versions, so re-exporting them keeps the
// whole tree on one resolved copy and removes a class of version-skew
// bugs. The crates are re-exported as modules (for the macro's
// fully-qualified paths and for advanced handlers, e.g.
// `rustio_admin::sqlx::query`); the type aliases are the ergonomic
// surface model files actually name.
pub use chrono;
pub use rust_decimal;
pub use sqlx;
pub use uuid;
// public: the field types a model declares, without a direct dep.
pub use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
pub use rust_decimal::Decimal;
pub use uuid::Uuid;

// `RustioAdmin` emits `::rustio_admin::*` paths in its expansion. That
// resolves cleanly for downstream consumers, but inside this crate's
// own compilation unit `rustio_admin` isn't a known extern. Aliasing
// the crate to itself under `cfg(test)` lets the macro be exercised
// from this crate's own tests without changing any non-test build.
#[cfg(test)]
extern crate self as rustio_admin;

// internal: test-only re-export surface, gated by integration-test feature
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
    // internal: test-only re-export
    pub use crate::auth::recovery_admin::{
        admin_revoke_sessions, admin_set_temp_password, check_account_lockout,
        check_session_elevated, issue_admin_reset_token, lock_user_account,
        promote_session_elevated, record_failed_login, record_successful_login,
        unlock_user_account, AdminActor, AdminIssueOutcome, AdminRevokeOutcome, AdminTempPwOutcome,
        LockDuration, LockOutcome, LockState, ThrottleOutcome, UnlockOutcome,
    };
    // R3 MFA — runtime fns + outcomes + key type + pure helpers the
    // integration suite needs to drive enrolment / verify /
    // consume / disable / regenerate flows. The `auth::mfa` module
    // is `pub(crate)`, so this is the only door under the
    // `integration-test` feature. See `DESIGN_R3_MFA.md` §13.3.
    // internal: test-only re-export
    pub use crate::auth::mfa::{
        confirm_enrolment, consume_backup_code, current_step, disable_mfa, generate_totp,
        promote_session_to_mfa_verified, provision_secret, regenerate_backup_codes,
        verify_totp_for_user, BackupConsumeOutcome, DisableOutcome, EnrolOutcome, MfaKey,
        ProvisionedSecret, RegenOutcome, VerifyOutcome, BACKUP_CODE_COUNT,
    };
    // internal: test-only wrapper for the pub(crate) hash helper
    /// R4 emergency-recovery test-only helper — forwards to the
    /// `pub(crate)` `auth::sessions::hash_token_for_storage` so
    /// the integration suite can verify that an
    /// `emergency_access`-issued URL stores its token in the
    /// exact same format R1's consume path will look up later.
    /// Catches the hex-vs-base64 drift surfaced during commit #8
    /// (which the unit-test gate could not have seen — only a
    /// live-DB or testcontainers run can). Wrapped rather than
    /// `pub use`'d because the inner helper is `pub(crate)` and
    /// cannot be re-exported under a stricter visibility.
    /// See `DESIGN_R4_EMERGENCY.md` §9.2.
    pub fn hash_token_for_storage(token: &str) -> String {
        crate::auth::sessions::hash_token_for_storage(token)
    }

    // internal: test-only request builder
    /// Construct a minimal [`crate::http::Request`] for integration
    /// tests. POST to `/test`, no headers, no body, no params.
    /// Adequate for runtime fns that read only `client_ip` (None)
    /// and `correlation_id_from` (None) from the request.
    pub fn fake_request() -> crate::http::Request {
        use std::collections::HashMap;
        crate::http::Request::__integration_test_fake("/test".to_string(), HashMap::new())
    }
}
