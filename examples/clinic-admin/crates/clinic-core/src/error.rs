//! One error type for the whole workspace — the framework's own.
//!
//! Re-exported so capability crates write `clinic_core::error::Result`
//! instead of reaching into `rustio_admin` directly.

pub use rustio_admin::{Error, Result};
