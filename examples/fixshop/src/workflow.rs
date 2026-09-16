//! The shop's status ladder, and the one rule that matters.
//!
//! ```text
//!   booked in ─▶ diagnosed ─▶ quoted ─┬─▶ approved ─▶ in progress ─▶ ready ─▶ collected
//!                                     └─▶ declined
//! ```
//!
//! Every step is a framework **bulk action** on `Job`, not a free-text
//! edit of the `status` column. That buys four things without a line
//! of framework code:
//!
//! 1. `status` sits in `Job::readonly_fields()`, so nobody drags a job
//!    across the ladder by typing in the edit form.
//! 2. Each step declares `BulkAction.permission`, so the route refuses
//!    the wrong person before this module runs — the technician cannot
//!    record a customer's approval, the front desk cannot start a
//!    repair. (The owner signs in as `administrator` and bypasses
//!    group checks entirely, so every button is theirs.)
//! 3. `rustio-admin` writes one row to `rustio_admin_actions` per
//!    submission on its own. We add one per job so each ticket's own
//!    **History** tab tells the story.
//! 4. Wrong-order clicks fail per row with a readable reason instead
//!    of failing the whole batch.
//!
//! **The rule:** a job cannot reach `in_progress` until the customer
//! has approved a quote. It is enforced twice, on purpose — here
//! against `jobs.quote_approved` (see [`Step::requires_approved_quote`])
//! and again in `Job::validate`, which catches the ordinary edit form.
//! Both read the same mirror column, which a trigger on `quotes` keeps
//! equal to "this job has an approved quote".

use std::future::Future;
use std::pin::Pin;

use rustio_admin::admin::audit::{self, ActionType, LogEntry};
use rustio_admin::sqlx::Row as SqlxRow;
use rustio_admin::{
    BulkAction, BulkActionContext, BulkActionFailure, BulkActionResult, Db, Error, Result,
};

/// Every value the `jobs.status` column may hold, in ladder order.
/// Mirrored by `#[rustio(choices = …)]` on `Job::status` (which draws
/// the dropdown) and by a CHECK constraint in migration 0002 (which
/// makes Postgres the backstop).
pub const STATUSES: &[&str] = &[
    "booked_in",
    "diagnosed",
    "quoted",
    "approved",
    "in_progress",
    "ready",
    "collected",
    "declined",
];

/// Statuses a job can only be in once a quote has been approved.
/// Read by `Job::validate` as well as by this module.
pub const AFTER_APPROVAL: &[&str] = &["in_progress", "ready", "collected"];

/// One rung of the ladder.
pub struct Step {
    /// URL slug — must equal the matching [`JOB_ACTIONS`] entry's `name`.
    pub action: &'static str,
    /// Status a job must already be in.
    pub from: &'static str,
    /// Status it moves to.
    pub to: &'static str,
    /// Also flip the job's newest quote to approved. This is what
    /// "front desk records the customer's yes" means in one click;
    /// the trigger on `quotes` mirrors it onto `jobs.quote_approved`.
    pub approves_quote: bool,
    /// Refuse unless the job already has an approved quote.
    pub requires_approved_quote: bool,
}

/// The ladder itself.
pub const STEPS: &[Step] = &[
    Step {
        action: "diagnose",
        from: "booked_in",
        to: "diagnosed",
        approves_quote: false,
        requires_approved_quote: false,
    },
    Step {
        action: "mark_quoted",
        from: "diagnosed",
        to: "quoted",
        approves_quote: false,
        requires_approved_quote: false,
    },
    Step {
        action: "record_approval",
        from: "quoted",
        to: "approved",
        approves_quote: true,
        requires_approved_quote: false,
    },
    Step {
        action: "record_decline",
        from: "quoted",
        to: "declined",
        approves_quote: false,
        requires_approved_quote: false,
    },
    Step {
        action: "start_repair",
        from: "approved",
        to: "in_progress",
        approves_quote: false,
        // The one rule that matters.
        requires_approved_quote: true,
    },
    Step {
        action: "mark_ready",
        from: "in_progress",
        to: "ready",
        approves_quote: false,
        requires_approved_quote: false,
    },
    Step {
        action: "mark_collected",
        from: "ready",
        to: "collected",
        approves_quote: false,
        requires_approved_quote: false,
    },
];

/// The same ladder as the framework sees it: the buttons on the Jobs
/// list page, each with the permission that gates it. `permission:
/// Some("repair")` resolves to `jobs.repair_job`, which migration 0005
/// grants to the `technician` group and nobody else.
///
/// Kept in lockstep with [`STEPS`] by `steps_and_buttons_match` below.
pub const JOB_ACTIONS: &[BulkAction] = &[
    BulkAction {
        name: "diagnose",
        label: "Diagnosed",
        destructive: false,
        confirm: false,
        permission: Some("diagnose"),
    },
    BulkAction {
        name: "mark_quoted",
        label: "Quote sent",
        destructive: false,
        confirm: false,
        permission: Some("quote"),
    },
    BulkAction {
        name: "record_approval",
        label: "Customer approved",
        destructive: false,
        confirm: false,
        permission: Some("approve"),
    },
    BulkAction {
        name: "record_decline",
        label: "Customer declined",
        // Ends the job — red button, and ask twice.
        destructive: true,
        confirm: true,
        permission: Some("approve"),
    },
    BulkAction {
        name: "start_repair",
        label: "Start repair",
        destructive: false,
        confirm: false,
        permission: Some("repair"),
    },
    BulkAction {
        name: "mark_ready",
        label: "Ready for collection",
        destructive: false,
        confirm: false,
        permission: Some("repair"),
    },
    BulkAction {
        name: "mark_collected",
        label: "Collected",
        destructive: false,
        confirm: false,
        permission: Some("collect"),
    },
];

/// `Job::execute_bulk_action` forwards here. Runs one step against
/// every selected job, collecting per-row refusals rather than
/// aborting the batch: selecting the whole list and pressing
/// "Start repair" moves exactly the jobs that are ready to move and
/// tells you why each of the others stayed put.
pub fn run<'a>(
    action: &'a str,
    ids: &'a [i64],
    db: &'a Db,
    ctx: &'a BulkActionContext<'a>,
) -> Pin<Box<dyn Future<Output = Result<BulkActionResult>> + Send + 'a>> {
    Box::pin(async move {
        let step = STEPS.iter().find(|s| s.action == action).ok_or_else(|| {
            Error::BadRequest(format!("`{action}` is not a step on the job ladder"))
        })?;

        let mut moved = 0usize;
        let mut failed = Vec::new();
        for &id in ids {
            match advance(db, step, id, ctx).await? {
                Ok(()) => moved += 1,
                Err(reason) => failed.push(BulkActionFailure::new(id, reason)),
            }
        }

        let label = label_for(action);
        let message = summary_line(label, moved, ids.len(), &failed);
        Ok(BulkActionResult::partial(moved, failed).with_message(message))
    })
}

/// How many refusal reasons to spell out in the audit summary before
/// collapsing the rest into a count. The full list always survives in
/// `metadata.failure_reasons`; this cap only keeps the one-line
/// summary on `/admin/history` readable when someone selects the whole
/// list and presses a button that most rows cannot take.
const REASONS_IN_SUMMARY: usize = 3;

/// Build the operator-facing line for one bulk submission.
///
/// The framework uses `BulkActionResult.message` verbatim as the audit
/// row's `summary`, which is the text `/admin/history` renders — so
/// folding the refusal reasons in here is what makes them readable
/// there. `metadata.failure_reasons` is built by the framework from
/// `BulkActionResult.failed`, which this does not touch, so the
/// structured list stays exactly as it was.
///
/// Reasons name the **ticket number** and nothing else. Every one is
/// built in [`advance`] from `ticket_no` plus status names — customer
/// names and phone numbers never reach a reason string, and so never
/// reach an audit summary that a wider set of operators can read than
/// can read the customer record itself.
fn summary_line(label: &str, moved: usize, total: usize, failed: &[BulkActionFailure]) -> String {
    let mut line = format!("{label}: {moved} of {total} jobs moved");
    if failed.is_empty() {
        return line;
    }
    let shown = failed.len().min(REASONS_IN_SUMMARY);
    line.push_str(" — ");
    line.push_str(
        &failed[..shown]
            .iter()
            .map(|f| f.reason.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    );
    if let Some(rest) = failed.len().checked_sub(shown).filter(|n| *n > 0) {
        line.push_str(&format!("; and {rest} more"));
    }
    line
}

/// Move one job one rung. `Ok(Ok(()))` moved it; `Ok(Err(reason))` is
/// a refusal the operator should read; `Err(_)` is a database problem
/// and aborts the batch.
async fn advance(
    db: &Db,
    step: &Step,
    job_id: i64,
    ctx: &BulkActionContext<'_>,
) -> Result<std::result::Result<(), String>> {
    let Some(row) = rustio_admin::sqlx::query(
        "SELECT ticket_no, status, quote_approved FROM jobs WHERE id = $1",
    )
    .bind(job_id)
    .fetch_optional(db.pool())
    .await?
    else {
        return Ok(Err("job no longer exists".into()));
    };
    let ticket: String = row.try_get("ticket_no")?;
    let status: String = row.try_get("status")?;
    let quote_approved: bool = row.try_get("quote_approved")?;

    if status != step.from {
        return Ok(Err(format!(
            "{ticket} is {}, and this step starts from {}",
            human(&status),
            human(step.from)
        )));
    }

    // The rule. `quote_approved` is the trigger-maintained mirror of
    // "this job has an approved quote" — see migration 0003.
    if step.requires_approved_quote && !quote_approved {
        return Ok(Err(format!(
            "{ticket} has no approved quote — the customer has to say yes first"
        )));
    }

    // Front desk recording the customer's yes: approve the newest
    // quote, then move the job. Quote first, so a job with no quote at
    // all never changes status.
    if step.approves_quote {
        let hits = rustio_admin::sqlx::query(
            "UPDATE quotes SET approved = TRUE
              WHERE id = (SELECT id FROM quotes
                           WHERE job_id = $1
                           ORDER BY created_at DESC, id DESC
                           LIMIT 1)",
        )
        .bind(job_id)
        .execute(db.pool())
        .await?
        .rows_affected();
        if hits == 0 {
            return Ok(Err(format!(
                "{ticket} has no quote yet — write one before recording an answer"
            )));
        }
    }

    // `AND status = $3` makes this a compare-and-set: if someone else
    // moved the job between our read and this write, we refuse rather
    // than overwrite their change.
    let hits =
        rustio_admin::sqlx::query("UPDATE jobs SET status = $1 WHERE id = $2 AND status = $3")
            .bind(step.to)
            .bind(job_id)
            .bind(step.from)
            .execute(db.pool())
            .await?
            .rows_affected();
    if hits == 0 {
        return Ok(Err(format!(
            "{ticket} was changed by someone else just now — reload and try again"
        )));
    }

    // FixShop's own trail, in the shop's words.
    rustio_admin::sqlx::query(
        "INSERT INTO job_events (job_id, from_status, to_status, actor) VALUES ($1, $2, $3, $4)",
    )
    .bind(job_id)
    .bind(step.from)
    .bind(step.to)
    .bind(&ctx.actor.email)
    .execute(db.pool())
    .await?;

    // And the framework's, so the ticket's own History tab shows the
    // move. `correlation_id` is the per-request id the framework's
    // middleware minted, so every job moved by one click is one chain.
    let mut entry = LogEntry::new(ctx.actor.user_id, ActionType::Update, "jobs", job_id);
    entry.summary = format!("{ticket}: {} → {}", human(step.from), human(step.to));
    entry.ip_address = ctx.ip_address;
    entry.correlation_id = ctx.correlation_id;
    audit::record(db, entry).await?;

    Ok(Ok(()))
}

/// `in_progress` → `in progress`. Used in operator-facing messages so
/// refusals read like English rather than like column values.
pub fn human(status: &str) -> String {
    status.replace('_', " ")
}

fn label_for(action: &str) -> &'static str {
    JOB_ACTIONS
        .iter()
        .find(|a| a.name == action)
        .map(|a| a.label)
        .unwrap_or("Job update")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two tables are written out by hand — one for the framework
    /// (buttons + permissions), one for us (from / to / guards). This
    /// is what stops them drifting apart.
    #[test]
    fn steps_and_buttons_match() {
        assert_eq!(STEPS.len(), JOB_ACTIONS.len());
        for (step, button) in STEPS.iter().zip(JOB_ACTIONS) {
            assert_eq!(step.action, button.name);
            assert!(
                button.permission.is_some(),
                "`{}` must name the permission that gates it",
                step.action
            );
        }
    }

    #[test]
    fn every_step_moves_between_real_statuses() {
        for step in STEPS {
            assert!(STATUSES.contains(&step.from), "{} from", step.action);
            assert!(STATUSES.contains(&step.to), "{} to", step.action);
        }
    }

    /// The ladder must not offer a second way into `in_progress` that
    /// skips the quote check.
    #[test]
    fn the_only_road_into_in_progress_needs_an_approved_quote() {
        let ways_in: Vec<&Step> = STEPS.iter().filter(|s| s.to == "in_progress").collect();
        assert_eq!(ways_in.len(), 1);
        assert!(ways_in[0].requires_approved_quote);
    }

    /// The reason a row was refused has to survive into the audit
    /// summary, because that is the text `/admin/history` shows — the
    /// structured `metadata.failure_reasons` is not rendered there.
    /// Long batches collapse to a count so the line stays readable,
    /// and the ticket number is the only identifier that appears.
    #[test]
    fn refusal_reasons_reach_the_audit_summary() {
        let failed = vec![
            BulkActionFailure::new(
                9,
                "FS-1009 has no approved quote — the customer has to say yes first",
            ),
            BulkActionFailure::new(
                6,
                "FS-1004 is booked in, and this step starts from approved",
            ),
        ];

        let line = summary_line("Start repair", 1, 3, &failed);
        assert!(line.starts_with("Start repair: 1 of 3 jobs moved"));
        assert!(line.contains("FS-1009 has no approved quote"));
        assert!(line.contains("FS-1004 is booked in"));

        // A clean run stays a plain count — no trailing separator.
        assert_eq!(
            summary_line("Start repair", 3, 3, &[]),
            "Start repair: 3 of 3 jobs moved"
        );

        // Past the cap, the overflow collapses instead of running on.
        let many: Vec<BulkActionFailure> = (0..7)
            .map(|i| BulkActionFailure::new(i, format!("FS-10{i:02} is booked in")))
            .collect();
        let long = summary_line("Start repair", 0, 7, &many);
        assert!(long.ends_with("; and 4 more"), "got: {long}");
    }

    #[test]
    fn statuses_past_approval_are_all_real_statuses() {
        for status in AFTER_APPROVAL {
            assert!(STATUSES.contains(status));
        }
    }
}
