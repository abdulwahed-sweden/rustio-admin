use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};

use rustio_admin::{
    BulkAction, BulkActionContext, BulkActionFailure, BulkActionResult, Db, Error, Model,
    ModelAdmin, Result, Row, RustioAdmin, Value,
};
// Trait import for `.try_get(...)` on `sqlx::postgres::PgRow`. The
// `as _` form pulls in the trait methods without binding `Row`,
// which would shadow `rustio_admin::Row` already in scope above.
use sqlx::Row as _;

#[derive(Debug, Clone, RustioAdmin)]
pub struct Appointment {
    pub id: i64,
    #[rustio(belongs_to = "Patient", display = "full_name")]
    pub patient_id: i64,
    #[rustio(belongs_to = "Practitioner", display = "full_name")]
    pub practitioner_id: i64,
    pub status: String,
    pub scheduled_at: DateTime<Utc>,
    pub checked_in_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Appointment {
    const TABLE: &'static str = "appointments";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "patient_id",
        "practitioner_id",
        "status",
        "scheduled_at",
        "checked_in_at",
        "ended_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "patient_id",
        "practitioner_id",
        "status",
        "scheduled_at",
        "checked_in_at",
        "ended_at",
    ];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Appointment {
            id: row.get_i64("id")?,
            patient_id: row.get_i64("patient_id")?,
            practitioner_id: row.get_i64("practitioner_id")?,
            status: row.get_string("status")?,
            scheduled_at: row.get_datetime("scheduled_at")?,
            checked_in_at: row.get_optional_datetime("checked_in_at")?,
            ended_at: row.get_optional_datetime("ended_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.patient_id),
            Value::from(self.practitioner_id),
            Value::from(self.status.clone()),
            Value::from(self.scheduled_at),
            Value::from(self.checked_in_at),
            Value::from(self.ended_at),
        ]
    }
}

impl ModelAdmin for Appointment {
    fn bulk_actions() -> &'static [BulkAction] {
        &[
            BulkAction {
                name: "mark_no_show",
                label: "Mark no-show",
                destructive: false,
                confirm: true,
                permission: None,
            },
            BulkAction {
                name: "mark_completed",
                label: "Mark completed",
                destructive: false,
                confirm: true,
                permission: None,
            },
        ]
    }

    fn execute_bulk_action<'a>(
        action: &'a str,
        ids: &'a [i64],
        db: &'a Db,
        _ctx: &'a BulkActionContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<BulkActionResult>> + Send + 'a>> {
        Box::pin(async move {
            match action {
                "mark_no_show" => mark_no_show(db, ids).await,
                "mark_completed" => mark_completed(db, ids).await,
                other => Err(Error::BadRequest(format!(
                    "unknown appointment bulk action: `{other}`"
                ))),
            }
        })
    }
}

// SELECT-then-UPDATE pattern (two round-trips per dispatch):
//
//   1. Read the current `status` for every selected id.
//   2. Partition into eligible-for-this-action vs. ineligible-with-reason.
//   3. Run one UPDATE against the eligible ids; `rows_affected`
//      is the canonical succeeded count.
//   4. Build a `BulkActionResult` carrying the per-id failure
//      reasons + an operator-facing summary line.
//
// Deterministic, boring, no abstractions. The framework's audit
// emission picks up the result + the summary line.

async fn mark_no_show(db: &Db, ids: &[i64]) -> Result<BulkActionResult> {
    let rows = sqlx::query("SELECT id, status FROM appointments WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(db.pool())
        .await?;

    let mut eligible: Vec<i64> = Vec::new();
    let mut failed: Vec<BulkActionFailure> = Vec::new();
    let mut seen: HashSet<i64> = HashSet::with_capacity(rows.len());

    for row in &rows {
        let id: i64 = row
            .try_get("id")
            .map_err(|e| Error::Internal(format!("appointment row missing id: {e}")))?;
        let status: String = row
            .try_get("status")
            .map_err(|e| Error::Internal(format!("appointment row missing status: {e}")))?;
        seen.insert(id);
        match status.as_str() {
            "scheduled" => eligible.push(id),
            "no_show" => failed.push(BulkActionFailure::new(id, "already marked no-show")),
            "completed" => failed.push(BulkActionFailure::new(
                id,
                "already completed — cannot be marked no-show",
            )),
            "cancelled" => failed.push(BulkActionFailure::new(
                id,
                "cancelled appointments don't transition to no-show",
            )),
            other => failed.push(BulkActionFailure::new(
                id,
                format!("unexpected appointment status `{other}`"),
            )),
        }
    }
    for id in ids {
        if !seen.contains(id) {
            failed.push(BulkActionFailure::new(*id, "appointment no longer exists"));
        }
    }

    let succeeded = if eligible.is_empty() {
        0
    } else {
        let res = sqlx::query(
            "UPDATE appointments \
             SET status = 'no_show' \
             WHERE id = ANY($1) AND status = 'scheduled'",
        )
        .bind(&eligible)
        .execute(db.pool())
        .await?;
        res.rows_affected() as usize
    };

    let total = ids.len();
    let message = if failed.is_empty() {
        format!("Marked {succeeded} of {total} appointments as no-show.")
    } else {
        format!(
            "Marked {succeeded} of {total} appointments as no-show ({skipped} skipped).",
            skipped = failed.len()
        )
    };
    Ok(BulkActionResult::partial(succeeded, failed).with_message(message))
}

async fn mark_completed(db: &Db, ids: &[i64]) -> Result<BulkActionResult> {
    let rows = sqlx::query("SELECT id, status FROM appointments WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(db.pool())
        .await?;

    let mut eligible: Vec<i64> = Vec::new();
    let mut failed: Vec<BulkActionFailure> = Vec::new();
    let mut seen: HashSet<i64> = HashSet::with_capacity(rows.len());

    for row in &rows {
        let id: i64 = row
            .try_get("id")
            .map_err(|e| Error::Internal(format!("appointment row missing id: {e}")))?;
        let status: String = row
            .try_get("status")
            .map_err(|e| Error::Internal(format!("appointment row missing status: {e}")))?;
        seen.insert(id);
        match status.as_str() {
            "scheduled" => eligible.push(id),
            "completed" => failed.push(BulkActionFailure::new(id, "already completed")),
            "no_show" => failed.push(BulkActionFailure::new(
                id,
                "no-show appointments aren't completed retroactively",
            )),
            "cancelled" => failed.push(BulkActionFailure::new(
                id,
                "cancelled appointments can't be completed",
            )),
            other => failed.push(BulkActionFailure::new(
                id,
                format!("unexpected appointment status `{other}`"),
            )),
        }
    }
    for id in ids {
        if !seen.contains(id) {
            failed.push(BulkActionFailure::new(*id, "appointment no longer exists"));
        }
    }

    let succeeded = if eligible.is_empty() {
        0
    } else {
        let res = sqlx::query(
            "UPDATE appointments \
             SET status = 'completed', ended_at = NOW() \
             WHERE id = ANY($1) AND status = 'scheduled'",
        )
        .bind(&eligible)
        .execute(db.pool())
        .await?;
        res.rows_affected() as usize
    };

    let total = ids.len();
    let message = if failed.is_empty() {
        format!("Marked {succeeded} of {total} appointments completed.")
    } else {
        format!(
            "Marked {succeeded} of {total} appointments completed ({skipped} skipped).",
            skipped = failed.len()
        )
    };
    Ok(BulkActionResult::partial(succeeded, failed).with_message(message))
}
