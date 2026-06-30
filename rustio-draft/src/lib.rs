//! rustio-draft — setup-time genesis for rustio-admin.
//!
//! Turns a natural-language brief into a `schema.json` (via the Claude API),
//! which `rustio-admin import` then applies deterministically. This crate is the
//! *only* place in the ecosystem that calls an LLM, and it lives outside the
//! framework workspace so the runtime never gains a network/LLM dependency.
//!
//! See `../docs/RUSTIO_DRAFT_SCOPE.md` for the full design and boundary.

pub mod client;
pub mod schema;

pub use client::{DraftClient, DEFAULT_MODEL};
pub use schema::{validate, SchemaDoc, SchemaField, SchemaModel, FIELD_TYPES};
