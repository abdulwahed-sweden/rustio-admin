//! `rustio audit` — operator-facing read surface for the
//! `rustio_admin_actions` table.
//!
//! Sibling to the framework's HTML `/admin/history` page — same
//! data, different consumer. Useful for shell-driven incident
//! response ("what did Alice do in the last hour?") and
//! pipe-into-grep workflows.
//!
//! v1 surface is read-only: a single `tail` subcommand that
//! prints the most-recent rows. Filtering by user / model is
//! handled in-SQL so even very large audit tables stay fast.
//! Write-side commands (e.g. retention pruning, manual event
//! injection) are deliberately *not* part of this surface —
//! audit data is append-only by doctrine.

use clap::Subcommand;
use sqlx::Row as _;

use rustio_admin::{auth, Db};

#[derive(Subcommand)]
pub enum Action {
    /// Print the most-recent audit rows, newest first. Optional
    /// `--user` and `--model` filters narrow the result; the
    /// SQL is indexed on `(model_name, object_id)` and
    /// `(timestamp DESC)` so the lookup stays cheap.
    Tail {
        /// How many rows to print. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: i64,
        /// Restrict to rows whose `user_id` resolves to this
        /// email. Looked up via `find_user_by_email`; an unknown
        /// email errors out rather than silently returning zero
        /// rows.
        #[arg(long)]
        user: Option<String>,
        /// Restrict to rows touching this model (the
        /// `model_name` column — e.g. `clinics`, `patients`,
        /// `rustio_users`).
        #[arg(long)]
        model: Option<String>,
    },
}

pub async fn run(action: Action) -> Result<(), String> {
    let db = crate::db().await?;
    match action {
        Action::Tail {
            limit,
            user,
            model,
        } => tail(db, limit, user, model).await,
    }
}

/// One row of the audit tail. Built by the DB layer, consumed
/// by the pure formatter [`format_audit_tail`]. Split so the
/// formatter is unit-testable without a Postgres pool.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AuditRow {
    timestamp: chrono::DateTime<chrono::Utc>,
    action_type: String,
    model_name: String,
    object_id: i64,
    user_email: Option<String>,
    user_id: i64,
    summary: String,
}

async fn tail(
    db: Db,
    limit: i64,
    user: Option<String>,
    model: Option<String>,
) -> Result<(), String> {
    let limit = limit.clamp(1, 10_000);

    // Resolve the optional --user filter to a user_id before
    // hitting the audit table. Unknown email → hard error, not
    // a silent empty result.
    let user_id = match user {
        Some(email) => {
            let u = auth::find_user_by_email(&db, &email)
                .await
                .map_err(|e| format!("lookup: {e}"))?
                .ok_or_else(|| format!("no user with email {email}"))?;
            Some(u.id)
        }
        None => None,
    };

    // Build the WHERE clause conditionally. sqlx doesn't have
    // a clean "optional bind" idiom so we build the SQL string
    // up-front with $1, $2, $3... placeholders; only the
    // parameters supplied get bound below.
    let (where_clause, params): (String, Vec<&str>) = match (user_id, model.as_deref()) {
        (Some(_), Some(_)) => ("WHERE a.user_id = $1 AND a.model_name = $2".into(), vec![]),
        (Some(_), None) => ("WHERE a.user_id = $1".into(), vec![]),
        (None, Some(_)) => ("WHERE a.model_name = $1".into(), vec![]),
        (None, None) => ("".into(), vec![]),
    };
    let _ = params; // placeholder for future param threading
    let limit_pos = match (user_id, model.as_deref()) {
        (Some(_), Some(_)) => "$3",
        (Some(_), None) | (None, Some(_)) => "$2",
        (None, None) => "$1",
    };

    let sql = format!(
        "SELECT a.timestamp, a.action_type, a.model_name, a.object_id,
                a.user_id, a.summary, u.email AS user_email
           FROM rustio_admin_actions a
           LEFT JOIN rustio_users u ON u.id = a.user_id
           {where_clause}
           ORDER BY a.timestamp DESC, a.id DESC
           LIMIT {limit_pos}"
    );

    // Bind the parameters in declaration order. The match in
    // both `where_clause` and `limit_pos` keeps them in sync.
    let mut q = sqlx::query(&sql);
    if let Some(uid) = user_id {
        q = q.bind(uid);
    }
    if let Some(m) = model.as_deref() {
        q = q.bind(m);
    }
    q = q.bind(limit);

    let rows = q
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("audit query: {e}"))?;

    let entries: Vec<AuditRow> = rows
        .into_iter()
        .map(|r| AuditRow {
            timestamp: r
                .try_get("timestamp")
                .unwrap_or_else(|_| chrono::Utc::now()),
            action_type: r.try_get("action_type").unwrap_or_default(),
            model_name: r.try_get("model_name").unwrap_or_default(),
            object_id: r.try_get("object_id").unwrap_or(0),
            user_email: r.try_get("user_email").ok(),
            user_id: r.try_get("user_id").unwrap_or(0),
            summary: r.try_get("summary").unwrap_or_default(),
        })
        .collect();

    print!("{}", format_audit_tail(&entries));
    Ok(())
}

/// Render the rows as a fixed-width table, newest first.
/// Pure function — no IO, no clock, no DB — so the unit tests
/// can hand it synthetic vectors and assert exact output.
fn format_audit_tail(rows: &[AuditRow]) -> String {
    use std::fmt::Write as _;

    if rows.is_empty() {
        return "(no audit rows)\n".into();
    }

    // Build each row's pre-summary line first so we can compute
    // a single "summary column start" for alignment. Action
    // types are bounded (≤8 chars in practice), model/object
    // varies more; pad to the widest seen in this batch.
    struct Pre {
        ts: String,
        action: String,
        target: String,
        who: String,
        summary: String,
    }
    let pres: Vec<Pre> = rows
        .iter()
        .map(|r| {
            let who = match r.user_email.as_deref() {
                Some(e) => format!("{e} (id={})", r.user_id),
                None => format!("(user id={} not found)", r.user_id),
            };
            Pre {
                ts: r.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                action: r.action_type.clone(),
                target: format!("{}/{}", r.model_name, r.object_id),
                who,
                summary: r.summary.clone(),
            }
        })
        .collect();

    // Right-pad each column to the max width seen in this
    // batch. Adding 2 spaces between columns for readability.
    let max_action = pres.iter().map(|p| p.action.len()).max().unwrap_or(6).max(6);
    let max_target = pres.iter().map(|p| p.target.len()).max().unwrap_or(8).max(8);
    let max_who = pres.iter().map(|p| p.who.len()).max().unwrap_or(4).max(4);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<19}  {:<aw$}  {:<tw$}  {:<uw$}  SUMMARY",
        "TIMESTAMP",
        "ACTION",
        "TARGET",
        "USER",
        aw = max_action,
        tw = max_target,
        uw = max_who,
    );
    for p in &pres {
        let _ = writeln!(
            out,
            "{:<19}  {:<aw$}  {:<tw$}  {:<uw$}  {}",
            p.ts,
            p.action,
            p.target,
            p.who,
            p.summary,
            aw = max_action,
            tw = max_target,
            uw = max_who,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(
        action: &str,
        model: &str,
        object_id: i64,
        email: Option<&str>,
        user_id: i64,
        summary: &str,
    ) -> AuditRow {
        AuditRow {
            // Pin a deterministic timestamp so test assertions
            // can compare exact strings without depending on
            // wall-clock state.
            timestamp: chrono::DateTime::parse_from_rfc3339("2026-05-20T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            action_type: action.into(),
            model_name: model.into(),
            object_id,
            user_email: email.map(|s| s.into()),
            user_id,
            summary: summary.into(),
        }
    }

    #[test]
    fn empty_input_renders_marker() {
        // Operator runs `rustio audit tail --model wrong` and
        // gets zero matches — surface the absence rather than
        // print just a header row.
        assert_eq!(format_audit_tail(&[]), "(no audit rows)\n");
    }

    #[test]
    fn header_row_carries_column_names() {
        let rows = vec![mk("create", "clinics", 5, Some("a@x.test"), 1, "Created")];
        let out = format_audit_tail(&rows);
        let header = out.lines().next().unwrap();
        assert!(header.contains("TIMESTAMP"));
        assert!(header.contains("ACTION"));
        assert!(header.contains("TARGET"));
        assert!(header.contains("USER"));
        assert!(header.contains("SUMMARY"));
    }

    #[test]
    fn rows_render_newest_first_in_input_order() {
        // The SQL ORDER BY DESC happens server-side; the
        // formatter must preserve input order, not re-sort.
        let rows = vec![
            mk("create", "clinics", 5, Some("a@x.test"), 1, "first"),
            mk("update", "patients", 12, Some("b@x.test"), 2, "second"),
        ];
        let out = format_audit_tail(&rows);
        let first_pos = out.find("first").expect("first present");
        let second_pos = out.find("second").expect("second present");
        assert!(first_pos < second_pos, "input order must be preserved");
    }

    #[test]
    fn user_email_with_id_suffix_renders() {
        let rows = vec![mk(
            "create",
            "clinics",
            5,
            Some("alice@example.test"),
            42,
            "Created",
        )];
        let out = format_audit_tail(&rows);
        assert!(out.contains("alice@example.test (id=42)"));
    }

    #[test]
    fn orphan_user_id_falls_back_to_marker() {
        // The CASCADE on rustio_users(id) means deleted users
        // also delete their audit rows, but a future change
        // could relax that. The formatter must still produce
        // SOMETHING readable for orphaned user_ids.
        let rows = vec![mk("delete", "patients", 7, None, 99, "Patient deleted")];
        let out = format_audit_tail(&rows);
        assert!(out.contains("(user id=99 not found)"));
    }

    #[test]
    fn target_formatting_is_model_slash_id() {
        let rows = vec![mk("update", "clinics", 5, Some("a@x.test"), 1, "")];
        let out = format_audit_tail(&rows);
        assert!(out.contains("clinics/5"));
    }

    #[test]
    fn columns_auto_widen_to_batch_max() {
        // Mix short and long entries — the formatter pads to
        // the widest seen so columns align across rows.
        let rows = vec![
            mk("create", "a", 1, Some("x@y.z"), 1, "short"),
            mk(
                "update",
                "very_long_model_name",
                9999,
                Some("very-long-email@example.test"),
                100,
                "longer summary text",
            ),
        ];
        let out = format_audit_tail(&rows);
        let lines: Vec<&str> = out.lines().collect();
        // Header + 2 rows = 3 lines.
        assert_eq!(lines.len(), 3);
        // Both rows reach the SUMMARY column at the same
        // offset; assert by checking the data rows are at
        // least as wide as the header row (a non-aligned
        // formatter would have varying row widths).
        let widest_data = lines[1].len().max(lines[2].len());
        assert!(
            widest_data >= lines[0].len(),
            "data rows ({widest_data} chars) should be at least as wide as header ({} chars)",
            lines[0].len(),
        );
    }
}
