//! `rustio perm` -- direct + group-mediated permission grants.
//!
//! Permission codenames follow the framework's convention:
//! `<app>.<action>_<model>`, e.g. `posts.add_post`. The framework's
//! `Admin::seed_permissions` populates them automatically on every
//! registered model; this CLI just lets operators grant or list
//! them.

use clap::Subcommand;
use sqlx::Row as _;

use rustio_admin::{auth, Db};

#[derive(Subcommand)]
pub enum Action {
    /// List every permission known to the database.
    List,
    /// Grant a permission directly to a user (bypasses group membership).
    GrantUser {
        #[arg(long)]
        email: String,
        /// Permission codename, e.g. `posts.add_post`.
        #[arg(long)]
        perm: String,
    },
    /// Grant a permission to a group (every member inherits it).
    GrantGroup {
        #[arg(long)]
        group: String,
        #[arg(long)]
        perm: String,
    },
}

pub async fn run(action: Action) -> Result<(), String> {
    let db = crate::db().await?;
    match action {
        Action::List => list(db).await,
        Action::GrantUser { email, perm } => grant_user(db, email, perm).await,
        Action::GrantGroup { group, perm } => grant_group(db, group, perm).await,
    }
}

async fn list(db: Db) -> Result<(), String> {
    let rows = sqlx::query("SELECT name, description FROM rustio_permissions ORDER BY name ASC")
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("query: {e}"))?;
    if rows.is_empty() {
        println!(
            "No permissions registered yet. Boot the app once so `Admin::seed_permissions` runs."
        );
        return Ok(());
    }
    for r in rows {
        let name: String = r.try_get("name").unwrap_or_default();
        let desc: String = r.try_get("description").unwrap_or_default();
        if desc.is_empty() {
            println!("  {name}");
        } else {
            println!("  {name}  -- {desc}");
        }
    }
    Ok(())
}

async fn grant_user(db: Db, email: String, perm: String) -> Result<(), String> {
    let user = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup user: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;
    auth::grant_to_user(&db, user.id, &perm)
        .await
        .map_err(|e| format!("grant_to_user: {e}"))?;
    println!("Granted `{perm}` to {email}");
    Ok(())
}

async fn grant_group(db: Db, group: String, perm: String) -> Result<(), String> {
    let row = sqlx::query("SELECT id FROM rustio_groups WHERE name = $1")
        .bind(&group)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("lookup group: {e}"))?;
    let row = row.ok_or_else(|| format!("no group named {group}"))?;
    let gid: i64 = row.try_get("id").map_err(|e| format!("group id: {e}"))?;
    auth::grant_to_group(&db, gid, &perm)
        .await
        .map_err(|e| format!("grant_to_group: {e}"))?;
    println!("Granted `{perm}` to group {group}");
    Ok(())
}
