//! The admin panel — Django Admin, Rust-flavoured.
//!
//! Phase 5 ships only the metadata vocabulary + the manual runtime;
//! handlers, routes, render, builtin, relations, filters, audit, and
//! icons land in P6.

mod types;

pub use types::{
    Admin, AdminEntry, AdminField, AdminModel, AdminRelation, AdminTheme, EditRow, FieldType,
    ListRow, SiteBranding, UserProfileRow, UserProfileSection,
};
