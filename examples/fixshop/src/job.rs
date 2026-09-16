//! One broken item on the bench. This is the model the whole example
//! turns around.
//!
//! Two columns exist so that facts which live in *other* tables are
//! visible where they are needed:
//!
//! - `quote_approved` mirrors "this job has an approved quote". A
//!   trigger on `quotes` (migration 0003) keeps it equal to the real
//!   thing on every write, which is what lets [`ModelAdmin::validate`]
//!   — synchronous, no database access — enforce the shop's rule on
//!   the ordinary edit form.
//! - `overdue` mirrors "promised date has passed and the item is still
//!   here". No write to this table can keep that current, because it
//!   goes stale on its own as the clock moves; `main::refresh_overdue_flags`
//!   recomputes it at boot and once a minute after that.

use std::future::Future;
use std::pin::Pin;

use rustio_admin::{
    BulkAction, BulkActionContext, BulkActionResult, DateTime, Db, FieldValidationError, Inline,
    ModelAdmin, NaiveDate, Result, RustioAdmin, Utc,
};

use crate::workflow;

#[derive(RustioAdmin)]
#[rustio(table = "jobs")]
pub struct Job {
    pub id: i64,
    /// `belongs_to` makes the admin render this cell as a link to the
    /// customer and turns the sidebar filter into a customer picker,
    /// rather than showing a bare integer.
    #[rustio(belongs_to = "Customer", display = "name")]
    pub customer_id: i64,
    pub ticket_no: String,
    pub item: String,
    pub problem: String,
    /// `choices` draws the dropdown; the CHECK constraint in migration
    /// 0002 is the backstop. In practice nobody picks from this
    /// dropdown — `status` is a readonly field on the edit form and
    /// moves through the bulk-action ladder in `workflow.rs` instead.
    #[rustio(choices = [
        "booked_in", "diagnosed", "quoted", "approved",
        "in_progress", "ready", "collected", "declined"
    ])]
    pub status: String,
    pub promised_date: NaiveDate,
    /// Mirror of `quotes.approved`, maintained by a trigger. Read-only
    /// on the form — approval is recorded by the front desk pressing
    /// "Customer approved", never by ticking a box here.
    pub quote_approved: bool,
    /// Recomputed on a timer. Also read-only on the form.
    pub overdue: bool,
    pub created_at: DateTime<Utc>,
}

impl ModelAdmin for Job {
    /// `overdue` and `quote_approved` are booleans, so the list page
    /// renders them as pills — the two things you want to see at a
    /// glance on a shop floor.
    fn list_display() -> &'static [&'static str] {
        &[
            "ticket_no",
            "customer_id",
            "item",
            "status",
            "promised_date",
            "overdue",
            "quote_approved",
        ]
    }

    /// Overdue first, then the oldest promise. A job that is past its
    /// promised date and still on the bench is the first row on the
    /// first page, every time — and "Overdue: yes" is one click in the
    /// sidebar.
    fn ordering() -> &'static [&'static str] {
        &["-overdue", "promised_date"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["overdue", "status", "customer_id"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["ticket_no", "item", "problem"]
    }

    /// None of these three are the operator's to type. `status` moves
    /// through the ladder; the other two are mirrors of facts owned
    /// elsewhere. The framework re-injects the stored value on save,
    /// so nothing is lost by disabling the inputs.
    fn readonly_fields() -> &'static [&'static str] {
        &["status", "quote_approved", "overdue"]
    }

    /// The quotes and the history, on the job's own page.
    fn inlines() -> &'static [Inline] {
        &[
            Inline {
                target_model: "Quote",
                fk_field: "job_id",
                label: Some("Quotes"),
                max_rows: 10,
                display_field: Some("amount"),
            },
            Inline {
                target_model: "JobEvent",
                fk_field: "job_id",
                label: Some("History"),
                max_rows: 50,
                display_field: Some("to_status"),
            },
        ]
    }

    /// The shop's rule, on the edit form.
    ///
    /// `validate` runs after the form parses and before anything
    /// reaches Postgres. It is synchronous and cannot query the
    /// database — which is exactly why `quote_approved` is a column on
    /// this row rather than a join away.
    ///
    /// The ladder in `workflow.rs` checks the same thing before it
    /// moves a job. Two checks, one fact: if someone adds a job
    /// straight into `in_progress` on the add form, this is what stops
    /// them.
    fn validate(job: &Self) -> std::result::Result<(), Vec<FieldValidationError>> {
        // Postgres would refuse an unknown status anyway (the CHECK
        // constraint in migration 0002), but a 409 conflict page is a
        // poor way to learn you mistyped a status. Say it on the form.
        if !workflow::STATUSES.contains(&job.status.as_str()) {
            return Err(vec![FieldValidationError::field(
                "status",
                "is not one of the shop's statuses.",
            )]);
        }
        if workflow::AFTER_APPROVAL.contains(&job.status.as_str()) && !job.quote_approved {
            return Err(vec![FieldValidationError::field(
                "status",
                format!(
                    "cannot be \"{}\" until the customer has approved a quote.",
                    workflow::human(&job.status)
                ),
            )]);
        }
        Ok(())
    }

    /// The seven buttons in the bulk bar, each gated on its own
    /// permission. See `workflow.rs`.
    fn bulk_actions() -> &'static [BulkAction] {
        workflow::JOB_ACTIONS
    }

    fn execute_bulk_action<'a>(
        action: &'a str,
        ids: &'a [i64],
        db: &'a Db,
        ctx: &'a BulkActionContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<BulkActionResult>> + Send + 'a>> {
        workflow::run(action, ids, db, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(status: &str, quote_approved: bool) -> Job {
        Job {
            id: 1,
            customer_id: 1,
            ticket_no: "FS-1001".into(),
            item: "iPhone 12".into(),
            problem: "Screen cracked".into(),
            status: status.into(),
            promised_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            quote_approved,
            overdue: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn in_progress_without_an_approved_quote_is_refused() {
        let errs = Job::validate(&job("in_progress", false)).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].field, Some("status"));
    }

    #[test]
    fn in_progress_with_an_approved_quote_is_allowed() {
        assert!(Job::validate(&job("in_progress", true)).is_ok());
    }

    #[test]
    fn everything_up_to_approval_is_allowed_without_a_quote() {
        for status in ["booked_in", "diagnosed", "quoted", "approved", "declined"] {
            assert!(
                Job::validate(&job(status, false)).is_ok(),
                "{status} should not need an approved quote"
            );
        }
    }

    #[test]
    fn an_unknown_status_is_refused() {
        let errs = Job::validate(&job("on_fire", true)).unwrap_err();
        assert_eq!(errs[0].field, Some("status"));
    }

    /// The dropdown the derive draws and the ladder `workflow.rs`
    /// walks have to offer the same set. The `#[rustio(choices = …)]`
    /// attribute needs string literals, so the two lists are written
    /// out separately; this is what keeps them equal.
    #[test]
    fn the_status_dropdown_matches_the_ladder() {
        use rustio_admin::AdminModel;
        let choices = Job::FIELDS
            .iter()
            .find(|f| f.name == "status")
            .and_then(|f| f.choices)
            .expect("status is a choices field");
        assert_eq!(choices, workflow::STATUSES);
    }

    #[test]
    fn readonly_fields_cover_every_mirrored_column() {
        for name in ["status", "quote_approved", "overdue"] {
            assert!(Job::readonly_fields().contains(&name));
        }
    }
}
