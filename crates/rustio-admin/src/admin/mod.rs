//! The admin panel — Django Admin, Rust-flavoured.

pub mod audit;
mod builtin;
pub mod filters;
mod handlers;
pub(crate) mod icons;
pub mod modeladmin;
mod ops;
pub(crate) mod recovery_handlers;
pub mod redact;
pub mod relations;
mod render;
mod routes;
mod types;

pub use audit::{
    ensure_table, for_object, recent, record, ActionType, AdminAction, AuditEvent, LogEntry,
};
pub use filters::{
    classify_field, field_ui_metadata, field_ui_metadata_with_relation, format_relation_cell,
    infer_filters, infer_filters_with_relations, mask_pii, FieldRole, FieldUI, FilterDef,
    FilterKind,
};
pub use modeladmin::{BulkAction, Fieldset, ModelAdmin, SortDir};
pub use redact::{redact_backup_code, redact_mfa_secret, redact_password, redact_token};
pub use relations::{
    InverseRelation, RegistryError, RelationRegistry, ResolvedRelation,
    RELATION_FILTER_DROPDOWN_CAP,
};
pub use routes::register_admin_routes;
pub use types::{
    Admin, AdminEntry, AdminField, AdminModel, AdminRelation, AdminTheme, CellLink, EditRow,
    FieldType, ListOpts, ListPage, ListRow, SiteBranding, UserProfileRow, UserProfileSection,
};
