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
pub struct Loan {
    pub id: i64,
    #[rustio(belongs_to = "Patron", display = "full_name")]
    pub patron_id: i64,
    #[rustio(belongs_to = "Item", display = "title")]
    pub item_id: i64,
    pub status: String,
    pub borrowed_at: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Loan {
    const TABLE: &'static str = "loans";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "patron_id",
        "item_id",
        "status",
        "borrowed_at",
        "due_at",
        "returned_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "patron_id",
        "item_id",
        "status",
        "borrowed_at",
        "due_at",
        "returned_at",
    ];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Loan {
            id: row.get_i64("id")?,
            patron_id: row.get_i64("patron_id")?,
            item_id: row.get_i64("item_id")?,
            status: row.get_string("status")?,
            borrowed_at: row.get_datetime("borrowed_at")?,
            due_at: row.get_datetime("due_at")?,
            returned_at: row.get_optional_datetime("returned_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.patron_id),
            Value::from(self.item_id),
            Value::from(self.status.clone()),
            Value::from(self.borrowed_at),
            Value::from(self.due_at),
            Value::from(self.returned_at),
        ]
    }
}

impl ModelAdmin for Loan {
    fn bulk_actions() -> &'static [BulkAction] {
        &[
            BulkAction {
                name: "mark_overdue",
                label: "Mark overdue",
                destructive: false,
                confirm: true,
            },
            BulkAction {
                name: "mark_returned",
                label: "Mark returned",
                destructive: false,
                confirm: true,
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
                "mark_overdue" => mark_overdue(db, ids).await,
                "mark_returned" => mark_returned(db, ids).await,
                other => Err(Error::BadRequest(format!(
                    "unknown loan bulk action: `{other}`"
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

async fn mark_overdue(db: &Db, ids: &[i64]) -> Result<BulkActionResult> {
    let rows = sqlx::query("SELECT id, status FROM loans WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(db.pool())
        .await?;

    let mut eligible: Vec<i64> = Vec::new();
    let mut failed: Vec<BulkActionFailure> = Vec::new();
    let mut seen: HashSet<i64> = HashSet::with_capacity(rows.len());

    for row in &rows {
        let id: i64 = row
            .try_get("id")
            .map_err(|e| Error::Internal(format!("loan row missing id: {e}")))?;
        let status: String = row
            .try_get("status")
            .map_err(|e| Error::Internal(format!("loan row missing status: {e}")))?;
        seen.insert(id);
        match status.as_str() {
            "active" => eligible.push(id),
            "overdue" => failed.push(BulkActionFailure::new(id, "already overdue")),
            "returned" => failed.push(BulkActionFailure::new(
                id,
                "already returned — cannot be marked overdue",
            )),
            other => failed.push(BulkActionFailure::new(
                id,
                format!("unexpected loan status `{other}`"),
            )),
        }
    }
    for id in ids {
        if !seen.contains(id) {
            failed.push(BulkActionFailure::new(*id, "loan no longer exists"));
        }
    }

    let succeeded = if eligible.is_empty() {
        0
    } else {
        let res = sqlx::query(
            "UPDATE loans \
             SET status = 'overdue' \
             WHERE id = ANY($1) AND status = 'active'",
        )
        .bind(&eligible)
        .execute(db.pool())
        .await?;
        res.rows_affected() as usize
    };

    let total = ids.len();
    let message = if failed.is_empty() {
        format!("Marked {succeeded} of {total} loans overdue.")
    } else {
        format!(
            "Marked {succeeded} of {total} loans overdue ({skipped} skipped).",
            skipped = failed.len()
        )
    };
    Ok(BulkActionResult::partial(succeeded, failed).with_message(message))
}

async fn mark_returned(db: &Db, ids: &[i64]) -> Result<BulkActionResult> {
    let rows = sqlx::query("SELECT id, status FROM loans WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(db.pool())
        .await?;

    let mut eligible: Vec<i64> = Vec::new();
    let mut failed: Vec<BulkActionFailure> = Vec::new();
    let mut seen: HashSet<i64> = HashSet::with_capacity(rows.len());

    for row in &rows {
        let id: i64 = row
            .try_get("id")
            .map_err(|e| Error::Internal(format!("loan row missing id: {e}")))?;
        let status: String = row
            .try_get("status")
            .map_err(|e| Error::Internal(format!("loan row missing status: {e}")))?;
        seen.insert(id);
        match status.as_str() {
            "active" | "overdue" => eligible.push(id),
            "returned" => failed.push(BulkActionFailure::new(id, "already returned")),
            other => failed.push(BulkActionFailure::new(
                id,
                format!("unexpected loan status `{other}`"),
            )),
        }
    }
    for id in ids {
        if !seen.contains(id) {
            failed.push(BulkActionFailure::new(*id, "loan no longer exists"));
        }
    }

    let succeeded = if eligible.is_empty() {
        0
    } else {
        let res = sqlx::query(
            "UPDATE loans \
             SET status = 'returned', returned_at = NOW() \
             WHERE id = ANY($1) AND status IN ('active', 'overdue')",
        )
        .bind(&eligible)
        .execute(db.pool())
        .await?;
        res.rows_affected() as usize
    };

    let total = ids.len();
    let message = if failed.is_empty() {
        format!("Marked {succeeded} of {total} loans returned.")
    } else {
        format!(
            "Marked {succeeded} of {total} loans returned ({skipped} skipped).",
            skipped = failed.len()
        )
    };
    Ok(BulkActionResult::partial(succeeded, failed).with_message(message))
}
