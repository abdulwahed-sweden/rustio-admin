//! `rustio doctor` — read-only health check.
//!
//! Three questions, each answered with a one-line ✓ / · / ✗:
//!   1. Does `DATABASE_URL` parse and reach Postgres?
//!   2. Are the auth tables present (i.e. has `init_tables` ever run)?
//!   3. Is there at least one administrator account?
//!
//! Anything red exits non-zero so a CI step can gate on it.

pub async fn run() -> Result<(), String> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            println!("✗ DATABASE_URL is not set. Add it to .env or your shell environment.");
            return Err("DATABASE_URL missing".into());
        }
    };
    println!("✓ DATABASE_URL = {}", crate::redact_password(&url));

    let db = match rustio_admin::Db::connect(&url).await {
        Ok(d) => {
            println!("✓ Connected to Postgres");
            d
        }
        Err(e) => {
            println!("✗ Could not connect: {e}");
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

    Ok(())
}
