//! One status change: what it moved from, what it moved to, who did
//! it, when. Written only by `workflow.rs`, and registered read-only
//! in `main.rs` — a log nobody can edit from the admin.
//!
//! This does not replace `rustio_admin_actions`. The framework's audit
//! log records who pressed which button across the whole panel, with a
//! correlation id per request; this table says what the press meant to
//! the shop, in the shop's vocabulary, on a page the front desk can
//! read without knowing what a correlation id is.

use rustio_admin::{DateTime, ModelAdmin, RustioAdmin, Utc};

#[derive(RustioAdmin)]
#[rustio(table = "job_events")]
pub struct JobEvent {
    pub id: i64,
    #[rustio(belongs_to = "Job", display = "ticket_no")]
    pub job_id: i64,
    pub from_status: String,
    pub to_status: String,
    /// The signed-in operator's e-mail, taken from
    /// `BulkActionContext.actor` at the moment of the change.
    pub actor: String,
    pub created_at: DateTime<Utc>,
}

impl ModelAdmin for JobEvent {
    fn list_display() -> &'static [&'static str] {
        &["created_at", "job_id", "from_status", "to_status", "actor"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["to_status", "actor"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["actor", "to_status"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-created_at"]
    }
}
