//! rustio-admin — Django Admin, but for Rust.
//!
//! This crate is the public face of the framework. Phase 2 ships the
//! HTTP / router / server / ORM / migrations / templates core; later
//! phases populate `auth` and `admin`.

#![forbid(unsafe_code)]

pub mod admin;
pub mod auth;
pub mod background;
pub mod error;
pub mod http;
pub mod middleware;
pub mod migrations;
pub mod orm;
pub mod router;
pub mod server;
pub mod templates;

pub use crate::admin::{Admin, AdminField, AdminModel, FieldType};
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
