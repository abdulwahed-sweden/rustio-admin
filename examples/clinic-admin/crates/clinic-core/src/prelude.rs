//! The imports every model file pulls in with `use clinic_core::prelude::*;`.
//!
//! Deliberately does NOT re-export `Result`: a model's
//! `fn from_row(..) -> Result<Self, Error>` uses the std two-parameter
//! `Result`, so the alias must stay out of scope here.

pub use chrono::{DateTime, Utc};
pub use rustio_admin::{Admin, Error, Model, ModelAdmin, Row, RustioAdmin, Value};
