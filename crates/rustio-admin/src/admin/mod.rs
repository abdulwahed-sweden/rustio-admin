//! The admin panel — Django Admin, Rust-flavoured.

pub mod audit;
pub mod filters;
mod handlers;
pub(crate) mod icons;
mod ops;
pub mod relations;
mod render;
mod routes;
mod types;

pub use audit::{ensure_table, for_object, recent, record, ActionType, AdminAction, LogEntry};
pub use filters::{
    classify_field, field_ui_metadata, field_ui_metadata_with_relation, format_relation_cell,
    infer_filters, infer_filters_with_relations, mask_pii, FieldRole, FieldUI, FilterDef,
    FilterKind,
};
pub use relations::{
    InverseRelation, RegistryError, RelationRegistry, ResolvedRelation,
    RELATION_FILTER_DROPDOWN_CAP,
};
pub use routes::register_admin_routes;
pub use types::{
    Admin, AdminEntry, AdminField, AdminModel, AdminRelation, AdminTheme, EditRow, FieldType,
    ListRow, SiteBranding, UserProfileRow, UserProfileSection,
};
