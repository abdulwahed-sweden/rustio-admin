//! Adaptive View Layer.
//!
//! The database schema knows field names and types but not visual importance.
//! This module lets rustio-admin record that importance once, as a stable
//! [`ViewSpec`], and render generated tables/lists/cards deterministically.
//!
//! Runtime contract: rendering reads a saved `ViewSpec` and nothing else. No
//! AI, no dynamic guessing, no non-determinism. Inference ([`infer`]) and the
//! future view designer only ever *produce* a `ViewSpec`; the saved spec is
//! always the source of truth.
//!
//! The layer is purely additive and presentation-only. It owns no search,
//! filtering, sorting, pagination, actions, or permissions — when a model has
//! no saved spec, callers fall back to the existing table path unchanged.

// public:
pub mod compose;
// public:
pub mod infer;
// public:
pub mod modes;
// public:
pub mod render;
// public:
pub mod roles;
// public:
pub mod spec;

#[cfg(test)]
mod safety_tests;

// public:
pub use compose::{CellComposition, ComposeStyle};
// public:
pub use infer::{infer_view_spec, infer_view_spec_from_fields, FieldKind, FieldMeta};
// public:
pub use modes::ViewMode;
// public:
pub use render::{
    render_row, render_view, CellPart, RenderedCell, RenderedRow, RenderedView, RowData,
};
// public:
pub use roles::{FieldRole, SemanticClass};
// public:
pub use spec::{FieldViewSpec, ViewSpec, VIEW_SPEC_VERSION};
