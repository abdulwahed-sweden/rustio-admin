//! Web counterpart to the `rustio doctor` CLI verb.
//!
//! Exposed at `GET /admin/health` (mounted in `routes.rs`,
//! Administrator-gated). The CLI's `doctor.rs` writes ✓ / · / ✗
//! lines to stdout; this module produces a structured
//! [`Vec<HealthCheck>`] the handler renders as a table.
//!
//! Duplication-by-design: the SQL probes mirror the CLI's
//! shape (one short SELECT per check). Keeping them in this
//! module rather than refactoring the working CLI surface
//! avoids regressing the operationally-critical `rustio
//! doctor` path. Stable checks; mirror any new probe in both
//! places.
//!
//! v1 scope: the four checks that gate "is this admin
//! deployable?" — DB reachable, auth tables, ≥1 active admin,
//! and `RUSTIO_SECRET_KEY` shape. MFA-enrolment and audit-slug
//! integrity readouts stay CLI-only for now; they're
//! informational and noisier than a glanceable web view wants.

use crate::orm::Db;

// public:
/// Structured outcome of one health check. Three states, ordered
/// by severity (Ok < Warn < Error) so the renderer can colour
/// each row predictably.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Ok,
    Warn,
    Error,
}

impl HealthStatus {
    /// Stable lowercase pill class so the template binds via
    /// `rio-pill--<status>` (or any project override). Snake-
    /// case ASCII; SIEM-friendly if a project ever scrapes the
    /// HTML.
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Ok => "ok",
            HealthStatus::Warn => "warn",
            HealthStatus::Error => "error",
        }
    }
}

// public:
/// One row in the health dashboard. `label` is a short
/// human-readable name; `message` is a one-line explanation
/// (or fallback for an empty Ok row). Mirrors the CLI's
/// per-check shape.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub label: &'static str,
    pub status: HealthStatus,
    pub message: String,
}

/// Run every check against `db` (plus a couple of env-derived
/// probes that don't touch DB). Returns the results in the
/// declared check order so the dashboard reads top-to-bottom
/// like the CLI doctor's output.
pub(crate) async fn gather_checks(db: &Db) -> Vec<HealthCheck> {
    let mut out: Vec<HealthCheck> = Vec::new();

    // ---- 1. DB reachable -----------------------------------------------
    //
    // The handler runs inside an already-bound `Db`, so reaching this
    // module means at least one prior query succeeded. Probe once more
    // with a trivial SELECT to detect a connection that's drifted
    // (idle timeout, network blip).
    let db_ok: bool = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(db.pool())
        .await
        .map(|n| n == 1)
        .unwrap_or(false);
    out.push(HealthCheck {
        label: "Postgres reachable",
        status: if db_ok {
            HealthStatus::Ok
        } else {
            HealthStatus::Error
        },
        message: if db_ok {
            "Pool accepted `SELECT 1`.".to_string()
        } else {
            "Pool query failed. Check `DATABASE_URL` + network.".to_string()
        },
    });

    // ---- 2. Auth tables present ----------------------------------------
    let auth_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
             WHERE table_name = 'rustio_users'
        )",
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(false);
    out.push(HealthCheck {
        label: "Auth tables present",
        status: if auth_exists {
            HealthStatus::Ok
        } else {
            HealthStatus::Error
        },
        message: if auth_exists {
            "rustio_users + rustio_sessions are in place.".to_string()
        } else {
            "Boot the app once or run `rustio user create` to seed them.".to_string()
        },
    });

    // ---- 3. At least one active administrator --------------------------
    let admin_count: i64 = if auth_exists {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM rustio_users \
             WHERE role IN ('administrator', 'developer') AND is_active = TRUE",
        )
        .fetch_one(db.pool())
        .await
        .unwrap_or(0)
    } else {
        0
    };
    out.push(HealthCheck {
        label: "Active administrator",
        status: if admin_count > 0 {
            HealthStatus::Ok
        } else {
            HealthStatus::Error
        },
        message: if admin_count > 0 {
            format!("{admin_count} active administrator(s) on file.")
        } else {
            "No active administrator. Run `rustio user create --email … --role administrator`."
                .to_string()
        },
    });

    // ---- 4. RUSTIO_SECRET_KEY shape ------------------------------------
    //
    // Required by R3 MFA + R4 emergency-access URL signing.
    // Permissive: a missing key is Warn (the runtime works
    // without it until someone enrols in MFA); a too-short key
    // is Error (MFA enrol will refuse).
    match std::env::var("RUSTIO_SECRET_KEY") {
        Ok(v) if v.len() >= 43 => out.push(HealthCheck {
            label: "RUSTIO_SECRET_KEY",
            status: HealthStatus::Ok,
            message: format!("Set ({} chars).", v.len()),
        }),
        Ok(v) if v.is_empty() => out.push(HealthCheck {
            label: "RUSTIO_SECRET_KEY",
            status: HealthStatus::Warn,
            message: "Empty. MFA enrol + emergency-access URL signing will refuse.".to_string(),
        }),
        Ok(v) => out.push(HealthCheck {
            label: "RUSTIO_SECRET_KEY",
            status: HealthStatus::Error,
            message: format!(
                "Too short ({} chars; need ≥ 43 URL-safe-base64 chars = 32 raw bytes).",
                v.len()
            ),
        }),
        Err(_) => out.push(HealthCheck {
            label: "RUSTIO_SECRET_KEY",
            status: HealthStatus::Warn,
            message: "Not set. MFA enrol + emergency-access URL signing will refuse.".to_string(),
        }),
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_as_str_is_snake_case_and_stable() {
        // SIEM-friendly stability — strings are part of the
        // template contract.
        assert_eq!(HealthStatus::Ok.as_str(), "ok");
        assert_eq!(HealthStatus::Warn.as_str(), "warn");
        assert_eq!(HealthStatus::Error.as_str(), "error");
    }
}
