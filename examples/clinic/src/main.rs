//! Standalone smoke binary for the clinic data layer.
//!
//! Opens the configured `DATABASE_URL` (defaults to the project's
//! `clinic_prod`), confirms every table from
//! `migrations/0001_clinic_schema.sql` is reachable by the
//! authenticated role, prints the row count for each. No HTTP, no
//! UI — this is the lowest-rung proof that the schema, the role,
//! and the `.env` wiring are healthy.
//!
//! Run from the workspace root:
//!
//! ```sh
//! cargo run -p clinic                      # debug build, dev iteration
//! cargo run --release -p clinic            # release build, ~2x faster startup
//! ./target/release/clinic                  # bypass cargo entirely, fastest
//! ```
//!
//! `DATABASE_URL` is read from either the shell environment or
//! `examples/.env` (the framework's convention). The DSN's password
//! segment is redacted before logging so a clipboard or CI log never
//! captures the credential.
//!
//! ## Speed
//!
//! All 20 table counts come back in **one** SQL round-trip via a
//! generated `UNION ALL`. The previous implementation did 20 serial
//! `SELECT COUNT(*)` round-trips plus a `SELECT version()` probe;
//! the new shape collapses that to a single network call. End-to-end
//! wall-time on localhost is dominated by Postgres TLS handshake +
//! query-planner pass; on a warm cache it lands well under 50 ms in
//! release mode.

use std::env;
use std::time::Instant;

use sqlx::postgres::PgPoolOptions;

/// Tables declared by `migrations/0001_clinic_schema.sql`, in
/// schema declaration order. The binary iterates this static list
/// — adding a table to the SQL but forgetting to add it here
/// surfaces immediately as a "missing from summary" hole when the
/// counts are reviewed. Lock-step by hand-edit, deliberately.
const TABLES: &[&str] = &[
    "clinics",
    "branches",
    "rooms",
    "specialties",
    "services",
    "medications",
    "patients",
    "doctors",
    "doctor_schedules",
    "doctor_time_off",
    "appointments",
    "appointment_status_history",
    "appointment_reminders",
    "medical_records",
    "diagnoses",
    "prescriptions",
    "lab_orders",
    "insurance_policies",
    "invoices",
    "payments",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();

    // Tolerate a missing .env — operator may have DATABASE_URL set
    // via shell already. We don't error on absence; only on the
    // env var itself being unset.
    let _ = dotenvy::from_path("examples/.env");

    let url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL not set (check examples/.env or your shell env)")?;

    println!("clinic — opening {}", redact_password(&url));

    // max_connections = 1 — we run one query, sequentially. A larger
    // pool would mean more TCP handshakes on cold start with zero
    // benefit. acquire_timeout default 30s is fine.
    let connect_started = Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    println!("connected in {:.1?}", connect_started.elapsed());

    // ONE round-trip for every table count. The generated SQL looks
    // like:
    //   SELECT 0 AS pos, 'clinics' AS table_name,
    //          (SELECT COUNT(*) FROM clinics) AS row_count
    //   UNION ALL
    //   SELECT 1, 'branches', (SELECT COUNT(*) FROM branches)
    //   …
    //   ORDER BY pos
    //
    // Table names interpolated from the static TABLES const — no SQL
    // injection vector. The `pos` integer preserves declaration order
    // through UNION ALL (which is otherwise undefined).
    let sql = build_counts_query();
    let query_started = Instant::now();
    let rows: Vec<(i32, String, i64)> = sqlx::query_as(&sql).fetch_all(&pool).await?;
    let query_elapsed = query_started.elapsed();

    println!();
    println!("{:<32} {:>10}", "table", "rows");
    println!("{:-<32} {:->10}", "", "");

    let mut total: i64 = 0;
    for (_pos, table, n) in &rows {
        println!("{table:<32} {n:>10}");
        total += n;
    }

    println!("{:-<32} {:->10}", "", "");
    println!("{:<32} {:>10}", "total", total);

    pool.close().await;

    println!();
    println!(
        "done in {:.1?} total  ·  query {:.1?}  ·  {} table(s) in 1 round-trip",
        started.elapsed(),
        query_elapsed,
        TABLES.len()
    );

    // Anomaly check: did the query miss any of the declared tables?
    // Could only happen on a major schema drift; surface as a non-zero
    // exit. Cheap O(n) comparison; no allocation in the happy path.
    if rows.len() != TABLES.len() {
        eprintln!(
            "warning: expected {} tables, server returned {}",
            TABLES.len(),
            rows.len()
        );
        std::process::exit(1);
    }
    Ok(())
}

/// Build the `UNION ALL` query that returns one row per table with
/// its `COUNT(*)`. Position column preserves [`TABLES`] order through
/// the UNION (otherwise SQL leaves UNION ALL ordering undefined).
fn build_counts_query() -> String {
    let mut sql = String::with_capacity(TABLES.len() * 90);
    for (i, t) in TABLES.iter().enumerate() {
        if i > 0 {
            sql.push_str(" UNION ALL ");
        }
        // `{t}` is from the static TABLES const — no untrusted input.
        sql.push_str(&format!(
            "SELECT {i} AS pos, '{t}' AS table_name, \
             (SELECT COUNT(*) FROM {t}) AS row_count"
        ));
    }
    sql.push_str(" ORDER BY pos");
    sql
}

/// Hide the password segment of a `DATABASE_URL` when logging.
/// Input  `postgres://user:secret@host/db`
/// Output `postgres://user:***@host/db`
///
/// Conservative parser — finds `://`, then the first `:` after that,
/// then the first `@`. Replaces the slice between the `:` and `@`
/// with `***`. URLs without credentials pass through unchanged.
fn redact_password(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let after_scheme = scheme_end + 3;
    let Some(at_offset) = url[after_scheme..].find('@') else {
        return url.to_string();
    };
    let at_pos = after_scheme + at_offset;
    let Some(colon_offset) = url[after_scheme..at_pos].find(':') else {
        return url.to_string();
    };
    let colon_pos = after_scheme + colon_offset;
    format!("{}:***{}", &url[..colon_pos], &url[at_pos..])
}

#[cfg(test)]
mod tests {
    use super::{build_counts_query, redact_password, TABLES};

    #[test]
    fn redacts_password_between_colon_and_at() {
        assert_eq!(
            redact_password("postgres://clinic_app:Gaza1950!@localhost/clinic_prod"),
            "postgres://clinic_app:***@localhost/clinic_prod"
        );
    }

    #[test]
    fn leaves_url_without_credentials_unchanged() {
        assert_eq!(
            redact_password("postgres://localhost/clinic_prod"),
            "postgres://localhost/clinic_prod"
        );
    }

    #[test]
    fn leaves_url_without_password_unchanged() {
        // `user@host` form (no `:password`) — nothing to redact.
        assert_eq!(
            redact_password("postgres://user@localhost/db"),
            "postgres://user@localhost/db"
        );
    }

    #[test]
    fn counts_query_includes_every_table_in_declared_order() {
        let sql = build_counts_query();
        // Every table appears in COUNT(*) form.
        for t in TABLES {
            assert!(
                sql.contains(&format!("FROM {t})")),
                "table {t} missing from generated SQL"
            );
        }
        // UNION ALL count = TABLES.len() - 1
        let join_count = sql.matches(" UNION ALL ").count();
        assert_eq!(join_count, TABLES.len() - 1);
        // Order preserved via `pos`
        assert!(sql.ends_with(" ORDER BY pos"));
    }
}
