//! The admin panel — Django Admin, Rust-flavoured.

pub(crate) mod admin_recovery_handlers;
// public:
pub mod audit;
pub(crate) mod builtin;
// public:
pub mod filters;
mod handlers;
pub(crate) mod icons;
pub(crate) mod mfa_handlers;
// public:
pub mod modeladmin;
mod ops;
pub(crate) mod recovery_handlers;
// public:
pub mod redact;
// public:
pub mod relations;
mod render;
mod routes;
mod types;

// public:
pub use audit::{
    ensure_table, for_object, recent, record, ActionType, AdminAction, AuditEvent, LogEntry,
};
// public:
pub use filters::{
    classify_field, field_ui_metadata, field_ui_metadata_with_relation, format_relation_cell,
    infer_filters, infer_filters_with_relations, mask_pii, FieldRole, FieldUI, FilterDef,
    FilterKind,
};
// public:
pub use modeladmin::{BulkAction, Fieldset, ModelAdmin, SortDir};
// public:
pub use redact::{redact_backup_code, redact_mfa_secret, redact_password, redact_token};
// public:
pub use relations::{
    InverseRelation, RegistryError, RelationRegistry, ResolvedRelation,
    RELATION_FILTER_DROPDOWN_CAP,
};
// public:
pub use routes::register_admin_routes;
// public:
pub use types::{
    Admin, AdminEntry, AdminField, AdminModel, AdminRelation, AdminTheme, CellLink, EditRow,
    FieldType, ListOpts, ListPage, ListRow, SiteBranding, UserProfileRow, UserProfileSection,
};
