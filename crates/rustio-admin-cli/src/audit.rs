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
        /// Only print rows from the last `<duration>` window —
        /// e.g. `30s`, `15m`, `2h`, `7d`. Useful for incident
        /// response: "what happened in the last hour?" without
        /// having to pick a row count. Composes with `--user`
        /// and `--model`.
        #[arg(long)]
        since: Option<String>,
    },
}

pub async fn run(action: Action) -> Result<(), String> {
    let db = crate::db().await?;
    match action {
        Action::Tail {
            limit,
            user,
            model,
            since,
        } => tail(db, limit, user, model, since).await,
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
    since: Option<String>,
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

    // Resolve the optional --since duration into a concrete
    // cut-off timestamp. Parsing errors hard-fail with a helpful
    // message; the alternative (silently ignoring a malformed
    // duration) would mask typos like `--since 1hr` and surface
    // confusing "no rows" results.
    let since_ts = match since {
        Some(s) => Some(parse_since_cutoff(&s, chrono::Utc::now())?),
        None => None,
    };

    // Build the WHERE clause iteratively so each filter binds
    // exactly the $N placeholder for the position in which it
    // gets appended. Scales cleanly to N optional filters; the
    // previous fixed-combination match was already at the limit
    // of what fits with three filters.
    let mut conds: Vec<String> = Vec::new();
    let mut idx: u8 = 1;
    if user_id.is_some() {
        conds.push(format!("a.user_id = ${idx}"));
        idx += 1;
    }
    if model.is_some() {
        conds.push(format!("a.model_name = ${idx}"));
        idx += 1;
    }
    if since_ts.is_some() {
        conds.push(format!("a.timestamp >= ${idx}"));
        idx += 1;
    }
    let where_clause = if conds.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conds.join(" AND "))
    };
    let limit_pos = format!("${idx}");

    let sql = format!(
        "SELECT a.timestamp, a.action_type, a.model_name, a.object_id,
                a.user_id, a.summary, u.email AS user_email
           FROM rustio_admin_actions a
           LEFT JOIN rustio_users u ON u.id = a.user_id
           {where_clause}
           ORDER BY a.timestamp DESC, a.id DESC
           LIMIT {limit_pos}"
    );

    // Bind in the same order that the WHERE-clause builder
    // walked the filters. The builder and the binder MUST stay
    // in lock-step; if you add a new filter, add to both.
    let mut q = sqlx::query(&sql);
    if let Some(uid) = user_id {
        q = q.bind(uid);
    }
    if let Some(m) = model.as_deref() {
        q = q.bind(m);
    }
    if let Some(ts) = since_ts {
        q = q.bind(ts);
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

/// Parse a `--since` duration like `30s` / `15m` / `2h` / `7d`
/// into a concrete UTC cutoff timestamp computed by subtracting
/// the parsed window from `now`. Pure function — `now` is
/// injected so the unit tests pin a fixed clock.
///
/// Accepted format: one or more ASCII digits followed by exactly
/// one of `s`, `m`, `h`, `d`. Reject anything else with a
/// human-readable message: silently treating a typo (`1hr`) as
/// "no filter" would mask incident-response queries.
fn parse_since_cutoff(
    raw: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<chrono::DateTime<chrono::Utc>, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("--since cannot be empty (expected e.g. 30s, 15m, 2h, 7d)".into());
    }
    let (num_part, unit) = s.split_at(s.len() - 1);
    let unit_ch = unit.chars().next().unwrap_or(' ');
    let n: i64 = num_part.parse().map_err(|_| {
        format!(
            "--since: cannot parse leading number from {raw:?} \
             (expected e.g. 30s, 15m, 2h, 7d)"
        )
    })?;
    if n < 0 {
        return Err("--since: duration must be non-negative".into());
    }
    let seconds = match unit_ch {
        's' => n,
        'm' => n.checked_mul(60).ok_or("--since: minutes overflow")?,
        'h' => n.checked_mul(3600).ok_or("--since: hours overflow")?,
        'd' => n.checked_mul(86_400).ok_or("--since: days overflow")?,
        other => {
            return Err(format!(
                "--since: unknown unit {other:?} (expected one of s/m/h/d)"
            ));
        }
    };
    Ok(now - chrono::Duration::seconds(seconds))
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
    let max_action = pres
        .iter()
        .map(|p| p.action.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let max_target = pres
        .iter()
        .map(|p| p.target.len())
        .max()
        .unwrap_or(8)
        .max(8);
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

    fn fixed_now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-05-20T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn parse_since_accepts_seconds_minutes_hours_days() {
        let now = fixed_now();
        assert_eq!(
            parse_since_cutoff("30s", now).unwrap(),
            now - chrono::Duration::seconds(30),
        );
        assert_eq!(
            parse_since_cutoff("15m", now).unwrap(),
            now - chrono::Duration::minutes(15),
        );
        assert_eq!(
            parse_since_cutoff("2h", now).unwrap(),
            now - chrono::Duration::hours(2),
        );
        assert_eq!(
            parse_since_cutoff("7d", now).unwrap(),
            now - chrono::Duration::days(7),
        );
    }

    #[test]
    fn parse_since_rejects_empty_and_malformed() {
        let now = fixed_now();
        // Empty / whitespace.
        assert!(parse_since_cutoff("", now).is_err());
        assert!(parse_since_cutoff("   ", now).is_err());
        // Bad unit. Surfacing the typo is the WHOLE POINT —
        // silently dropping the filter would mask the operator's
        // intent on an incident-response query.
        assert!(parse_since_cutoff("1hr", now).is_err());
        assert!(parse_since_cutoff("5x", now).is_err());
        // Non-numeric leading part.
        assert!(parse_since_cutoff("abc", now).is_err());
        // Missing leading number.
        assert!(parse_since_cutoff("h", now).is_err());
    }

    #[test]
    fn parse_since_rejects_negative_numbers() {
        // `-5h` parses as a number with a unit, but a negative
        // window is operationally meaningless (would skip ahead
        // of `now`). Reject loudly.
        let now = fixed_now();
        assert!(parse_since_cutoff("-5h", now).is_err());
    }

    #[test]
    fn parse_since_trims_whitespace() {
        let now = fixed_now();
        assert_eq!(
            parse_since_cutoff("  30s  ", now).unwrap(),
            now - chrono::Duration::seconds(30),
        );
    }

    #[test]
    fn parse_since_zero_is_valid_and_equals_now() {
        // 0s / 0m / 0h / 0d are all valid edge cases — they
        // mean "no time window applied" (cutoff == now). Useful
        // to keep the cutoff-binding path uniform without a
        // special "0 means skip" rule.
        let now = fixed_now();
        assert_eq!(parse_since_cutoff("0s", now).unwrap(), now);
        assert_eq!(parse_since_cutoff("0d", now).unwrap(), now);
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
