//! Public surface for project-defined bulk-action dispatch.
//!
//! The framework owns the metadata declaration ([`super::BulkAction`])
//! and the rendering / confirmation flow; *projects* own what each
//! action actually does. This module carries the three types that
//! cross the public boundary between framework and project:
//!
//!   - [`BulkActionContext`] — what the framework hands to the
//!     project's handler (actor, correlation-id, client IP). Narrow
//!     by design; `#[non_exhaustive]` keeps it SemVer-safe.
//!   - [`BulkActionResult`] — what the project hands back. Carries
//!     a succeeded count, an optional per-id failure list, and an
//!     operator-facing summary line.
//!   - [`BulkActionFailure`] — one row that failed inside an
//!     otherwise-successful batch.
//!
//! The dispatcher itself is [`super::ModelAdmin::execute_bulk_action`].
//! See its docstring for the contract; see `DESIGN_CHROME.md` for the
//! bulk-bar's visual conventions.
//!
//! The framework emits **one** `audit::record` row per submission
//! after the project's handler returns. Projects don't need to call
//! `audit::record` themselves for the dispatch envelope; any
//! business-level audit emissions inside the action body are still
//! the project's call.

use crate::auth::Identity;

// public:
/// Per-request context the framework passes into project-side
/// [`super::ModelAdmin::execute_bulk_action`] implementations.
///
/// Narrower than a generic "request context" on purpose. The
/// framework promises three facts at the dispatch boundary:
///
///   - `actor` — who initiated this action. Used by projects for
///     per-actor authorisation refinements, per-row audit
///     emission, and "edited_by"-shaped column writes.
///   - `correlation_id` — the per-request UUID. Projects that emit
///     their own audit rows should reuse it so the history chain
///     stays linked through their work.
///   - `ip_address` — the client IP resolved from `x-forwarded-for`
///     / `x-real-ip` headers, when present.
///
/// `#[non_exhaustive]` so future additions (e.g. a per-request
/// transaction handle, a `ProgressHandle` for streaming progress)
/// stay SemVer-safe.
#[non_exhaustive]
pub struct BulkActionContext<'a> {
    /// The signed-in operator who initiated the bulk dispatch.
    pub actor: &'a Identity,
    /// Per-request correlation id from the R0 middleware. `None`
    /// only on request paths that don't run the correlation
    /// middleware (none in the framework today; reserved for
    /// future custom mounts).
    pub correlation_id: Option<&'a str>,
    /// Client IP resolved from request headers. `None` when the
    /// framework couldn't determine it (no proxy headers, no
    /// connecting peer info available).
    pub ip_address: Option<&'a str>,
}

impl<'a> BulkActionContext<'a> {
    /// Construct a minimal context — actor-only. Future fields
    /// default to `None`. Useful for project-side unit tests of
    /// `execute_bulk_action` that don't want to fabricate a
    /// correlation id or IP.
    pub fn new(actor: &'a Identity) -> Self {
        Self {
            actor,
            correlation_id: None,
            ip_address: None,
        }
    }
}

// public:
/// Outcome of a project-defined bulk action.
///
/// Two channels:
///
///   - The action *itself* failed (unknown action name, project
///     code panicked into a `Result::Err`, DB connection lost).
///     The dispatcher returns `Err(...)` from
///     [`super::ModelAdmin::execute_bulk_action`]; the framework
///     surfaces it as a 4xx / 5xx page.
///   - The action ran but *some rows* failed (one of three loans
///     was already in the target state, an FK constraint
///     rejected one of the writes). The dispatcher returns
///     `Ok(BulkActionResult)` with `failed` populated; the
///     framework emits a partial-success audit row and the
///     operator sees a per-id failure summary.
///
/// `#[non_exhaustive]` so future additions (e.g. a `warnings`
/// channel, a `progress_handle` for streaming) stay SemVer-safe.
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct BulkActionResult {
    /// Number of rows the project successfully applied the action
    /// to.
    pub succeeded: usize,
    /// Per-row failure list. Empty for a clean run; one entry per
    /// row the project tried and couldn't complete.
    pub failed: Vec<BulkActionFailure>,
    /// Operator-facing summary line. `None` lets the framework
    /// fall back to its default rendering
    /// (`"<label>: <succeeded> of <total>"`).
    pub message: Option<String>,
}

impl BulkActionResult {
    /// All `succeeded` rows applied cleanly; no failures.
    pub fn ok(succeeded: usize) -> Self {
        Self {
            succeeded,
            failed: Vec::new(),
            message: None,
        }
    }

    /// Mixed outcome — pair the survivor count with the per-id
    /// failure list.
    pub fn partial(succeeded: usize, failed: Vec<BulkActionFailure>) -> Self {
        Self {
            succeeded,
            failed,
            message: None,
        }
    }

    /// Total rows the project attempted: `succeeded + failed.len()`.
    pub fn total(&self) -> usize {
        self.succeeded + self.failed.len()
    }

    /// Attach an operator-facing summary line. Builder-style for
    /// terse construction:
    ///
    /// ```ignore
    /// BulkActionResult::ok(12).with_message("Marked 12 loans overdue")
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

// public:
/// One row that failed inside an otherwise-successful bulk batch.
///
/// `#[non_exhaustive]` so future additions (e.g. a structured
/// `ErrorKind` enum, a retryable hint) stay SemVer-safe.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BulkActionFailure {
    /// The row id the project couldn't apply the action to.
    pub id: i64,
    /// Operator-facing reason. Shown as-is on the bulk result
    /// flash / page; redact sensitive details before constructing.
    pub reason: String,
}

impl BulkActionFailure {
    /// Construct a failure entry with a reason string.
    pub fn new(id: i64, reason: impl Into<String>) -> Self {
        Self {
            id,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_constructs_clean_result() {
        let r = BulkActionResult::ok(7);
        assert_eq!(r.succeeded, 7);
        assert!(r.failed.is_empty());
        assert!(r.message.is_none());
        assert_eq!(r.total(), 7);
    }

    #[test]
    fn partial_carries_failures_into_total() {
        let r = BulkActionResult::partial(
            5,
            vec![
                BulkActionFailure::new(42, "already overdue"),
                BulkActionFailure::new(51, "row not active"),
            ],
        );
        assert_eq!(r.succeeded, 5);
        assert_eq!(r.failed.len(), 2);
        assert_eq!(r.total(), 7);
        assert_eq!(r.failed[0].id, 42);
        assert_eq!(r.failed[0].reason, "already overdue");
    }

    #[test]
    fn with_message_attaches_summary() {
        let r = BulkActionResult::ok(3).with_message("Marked 3 loans overdue");
        assert_eq!(r.message.as_deref(), Some("Marked 3 loans overdue"));
    }

    #[test]
    fn default_is_zero_result() {
        let r = BulkActionResult::default();
        assert_eq!(r.succeeded, 0);
        assert!(r.failed.is_empty());
        assert_eq!(r.total(), 0);
    }

    #[test]
    fn failure_new_owns_reason() {
        // String, &str, and String::from all flow through Into<String>.
        let f1 = BulkActionFailure::new(1, "static");
        let f2 = BulkActionFailure::new(2, String::from("owned"));
        let f3 = BulkActionFailure::new(3, format!("{}-{}", "fmt", "string"));
        assert_eq!(f1.reason, "static");
        assert_eq!(f2.reason, "owned");
        assert_eq!(f3.reason, "fmt-string");
    }
}
