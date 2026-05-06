//! rustio-admin — Django Admin, but for Rust.
//!
//! This crate is the public face of the framework. Phase 2 ships the
//! HTTP / router / server / ORM / migrations / templates core; later
//! phases populate `auth` and `admin`.

#![forbid(unsafe_code)]

pub mod error;
pub mod http;
pub mod middleware;
pub mod migrations;
pub mod orm;
pub mod router;
pub mod server;
pub mod templates;

pub use crate::error::{Error, Result};
pub use crate::http::{FormData, Request, Response};
pub use crate::orm::{Db, DbOptions, Model, Row, Value};
pub use crate::router::{Next, Router};
pub use crate::server::Server;
