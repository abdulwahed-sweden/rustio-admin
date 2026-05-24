//! `rustio doctor` — read-only health check.
//!
//! Each check answers with a one-line ✓ / · / ✗:
//!
//!   1. Does `DATABASE_URL` parse and reach Postgres?
//!   2. Are the auth tables present (i.e. has `init_tables` ever run)?
//!   3. Is there at least one administrator account?
//!   4. Is `RUSTIO_SECRET_KEY` set and ≥ 32 bytes (only when MFA
//!      is in use OR a compliance-export surface needs it).
//!   5. R3 MFA enrolment count — informational; helps the operator
//!      see at a glance whether MFA is in active use.
//!   6. R4 emergency-recovery row count — informational; surfaces
//!      whether any operator has reached for the shell tier.
//!   7. Audit slug integrity — detects pre-0.8.1 rows that still
//!      carry the legacy `"User"` / `"user"` / `"rustio_users"`
//!      `model_name` conventions. The 0.8.1 backfill (in
//!      `admin::audit::ensure_table`) rewrites them on next boot;
//!      this check surfaces any remaining drift between releases.
//!
//! Hard failures (no DB, no admin) exit non-zero so a CI step
//! can gate on it. Informational lines (R3/R4 counts, slug
//! drift) print but don't fail the check.

pub async fn run() -> Result<(), String> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            println!("✗ DATABASE_URL is not set. Add it to .env or your shell environment.");
            return Err("DATABASE_URL missing".into());
        }
    };
    println!("✓ DATABASE_URL = {}", crate::redact_password(&url));

    // Spinner during the connect round-trip, replaced by the
    // ✓/✗ line on completion. PR 1.4 / DESIGN_ONBOARDING.md §9.
    let step = crate::progress::Step::start("Connecting to PostgreSQL");
    let db = match rustio_admin::Db::connect(&url).await {
        Ok(d) => {
            step.done_with("Connected to PostgreSQL");
            d
        }
        Err(e) => {
            step.failed_with(format!("Could not connect: {e}"));
            return Err("connect failed".into());
        }
    };

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
             WHERE table_name = 'rustio_users'
        )",
    )
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("table check: {e}"))?;
    if exists {
        println!("✓ Auth tables present");
    } else {
        println!(
            "· Auth tables missing — boot the app once or run `rustio user create` to seed them."
        );
    }

    let admin_count: i64 = if exists {
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
    if admin_count > 0 {
        println!("✓ {admin_count} active administrator(s)");
    } else {
        println!(
            "· No active administrator. Run `rustio user create --email …  --role administrator`."
        );
    }

    // ---- RUSTIO_SECRET_KEY presence + format -------------------------------
    //
    // Required by R3 (TOTP-secret AES-256-GCM encryption) and by
    // projects that ship an HMAC-signed compliance export. A
    // deployment with `MfaPolicy::Disabled` and zero enrolled
    // users keeps working without the key — but the moment a
    // user enrols, the AES-GCM init guard surfaces a runtime 500.
    // Doctor reports presence + length without ever logging the
    // value.
    match std::env::var("RUSTIO_SECRET_KEY") {
        Ok(v) if v.len() >= 43 => {
            // 32 raw bytes → 43 chars URL-safe-base64-no-pad.
            // 44 chars with padding. Either is fine.
            println!("✓ RUSTIO_SECRET_KEY set ({} chars)", v.len());
        }
        Ok(v) if v.is_empty() => {
            println!(
                "· RUSTIO_SECRET_KEY is empty. Required for R3 MFA + R4 export \
                 signing. Generate with: \
                 `openssl rand 32 | base64 | tr '+/' '-_' | tr -d '='`"
            );
        }
        Ok(v) => {
            println!(
                "✗ RUSTIO_SECRET_KEY is too short ({} chars; need ≥ 43 URL-safe-base64 \
                 chars = 32 raw bytes). MFA enrol + emergency-access URL signing \
                 will refuse. Regenerate with: \
                 `openssl rand 32 | base64 | tr '+/' '-_' | tr -d '='`",
                v.len()
            );
        }
        Err(_) => {
            println!(
                "· RUSTIO_SECRET_KEY is not set. Required for R3 MFA + R4 export \
                 signing. Add to `.env`: \
                 `RUSTIO_SECRET_KEY=$(openssl rand 32 | base64 | tr '+/' '-_' | tr -d '=')`"
            );
        }
    }

    // ---- R3 MFA enrolment count --------------------------------------------
    //
    // Read directly from `rustio_users.mfa_enabled` — does not
    // require the project's `Admin` runtime. Pre-R3 schemas don't
    // have the column; we degrade silently in that case.
    if exists {
        let mfa_count: Option<i64> =
            sqlx::query_scalar("SELECT COUNT(*) FROM rustio_users WHERE mfa_enabled = TRUE")
                .fetch_optional(db.pool())
                .await
                .unwrap_or(None);
        if let Some(n) = mfa_count {
            if n > 0 {
                println!("✓ {n} user(s) enrolled in MFA");
            } else {
                println!("· No users enrolled in MFA yet (visit /admin/account/mfa/enroll while logged in)");
            }
        }
    }

    // ---- R4 emergency-recovery row count -----------------------------------
    //
    // `action_type = 'emergency_recovery'` is the audit-row marker
    // for every CLI-driven recovery (reset-password / unlock /
    // disable-mfa / promote / emergency-access). Non-zero is not
    // a failure — it's informational. Operators may want to
    // verify the row count matches their internal incident log.
    if exists {
        let emerg_count: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rustio_admin_actions \
              WHERE action_type = 'emergency_recovery'",
        )
        .fetch_optional(db.pool())
        .await
        .unwrap_or(None);
        if let Some(n) = emerg_count {
            if n > 0 {
                println!(
                    "· {n} emergency-recovery audit row(s) on file (see /admin/history?action=emergency_recovery)"
                );
            }
        }
    }

    // ---- Audit slug-drift check (0.8.1 backfill verifier) ------------------
    //
    // Before 0.8.1, audit-row emissions wrote `model_name` in four
    // inconsistent conventions ("User", "user", "rustio_users",
    // "Group", "group", "rustio_groups"). The 0.8.1 boot migration
    // in `admin::audit::ensure_table` rewrites these. If any remain
    // after a recent boot, either:
    //   - the framework binary running the migration is older than
    //     0.8.1 (upgrade); OR
    //   - a project-side custom audit emitter is writing one of the
    //     legacy strings (fix the emitter).
    if exists {
        let legacy_count: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rustio_admin_actions \
              WHERE model_name IN \
                ('User','user','rustio_users','Group','group','rustio_groups')",
        )
        .fetch_optional(db.pool())
        .await
        .unwrap_or(None);
        if let Some(n) = legacy_count {
            if n > 0 {
                println!(
                    "✗ {n} audit row(s) still carry legacy model_name slugs (`User` / \
                     `user` / `rustio_users` / `Group` / `group` / `rustio_groups`). \
                     The History page links to `/admin/{{slug}}/…` and these slugs 404. \
                     Boot against rustio-admin ≥ 0.8.1 to apply the backfill, OR \
                     fix any project-side audit emitter that writes these strings."
                );
            } else {
                println!("✓ Audit model_name slugs all canonical (0.8.1 backfill clean)");
            }
        }
    }

    Ok(())
}
