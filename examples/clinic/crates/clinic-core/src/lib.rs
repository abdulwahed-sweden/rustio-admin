//! Shared foundation for every clinic capability.
//!
//! `clinic-core` holds the things more than one capability needs:
//! configuration, the database connection, the shared error type, and
//! a `prelude` of the imports every model file uses.
//!
//! Doctrine: code lives in its capability crate until a *second*
//! capability needs it — then, and only then, it sinks down to here.
//! Capabilities depend on `clinic-core`; they never depend on each other.

pub mod config;
pub mod db;
pub mod error;
pub mod prelude;

pub use config::Config;
