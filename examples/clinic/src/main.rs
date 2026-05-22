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
//! cargo run -p clinic
//! ```
//!
//! `DATABASE_URL` is read from either the shell environment or
//! `examples/.env` (the framework's convention). The DSN's password
//! segment is redacted before logging so a clipboard or CI log never
//! captures the credential.

use std::env;
use std::time::Instant;

use sqlx::postgres::PgPoolOptions;
use sqlx::Row;

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
    // Tolerate a missing .env — operator may have DATABASE_URL set
    // via shell already. We don't error on absence; only on the
    // env var itself being unset.
    let _ = dotenvy::from_path("examples/.env");

    let url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL not set (check examples/.env or your shell env)")?;

    println!("clinic — opening {}", redact_password(&url));

    let started = Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?;

    let version: String = sqlx::query("SELECT version()")
        .fetch_one(&pool)
        .await?
        .get(0);
    println!("connected in {:.1?}", started.elapsed());
    println!("server: {}", version.lines().next().unwrap_or(&version));

    println!();
    println!("{:<32} {:>10}", "table", "rows");
    println!("{:-<32} {:->10}", "", "");

    let mut total: i64 = 0;
    let mut failures: Vec<(&str, sqlx::Error)> = Vec::new();
    for &table in TABLES {
        // Table name is from a hard-coded static list — no SQL
        // injection vector. Format-interpolation is the correct
        // choice; sqlx bind parameters can't substitute identifiers.
        let sql = format!("SELECT COUNT(*) FROM {table}");
        match sqlx::query_scalar::<_, i64>(&sql).fetch_one(&pool).await {
            Ok(n) => {
                println!("{table:<32} {n:>10}");
                total += n;
            }
            Err(e) => {
                println!("{table:<32} {:>10}", "ERR");
                failures.push((table, e));
            }
        }
    }

    println!("{:-<32} {:->10}", "", "");
    println!("{:<32} {:>10}", "total", total);

    if !failures.is_empty() {
        eprintln!();
        eprintln!("{} table(s) failed:", failures.len());
        for (table, err) in &failures {
            eprintln!("  {table}: {err}");
        }
        std::process::exit(1);
    }

    pool.close().await;
    Ok(())
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
    use super::redact_password;

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
}
