//! `rustio group` — group CRUD on top of the framework's `auth`
//! permission tables.

use clap::Subcommand;
use sqlx::Row as _;

use rustio_admin::{auth, Db};

#[derive(Subcommand)]
pub enum Action {
    /// Create a new group.
    Create {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    /// List every group with id / name / member count.
    List,
    /// Add a user to a group (by email + group name).
    AddUser {
        #[arg(long)]
        email: String,
        #[arg(long)]
        group: String,
    },
    /// Remove a user from a group.
    RemoveUser {
        #[arg(long)]
        email: String,
        #[arg(long)]
        group: String,
    },
}

pub async fn run(action: Action) -> Result<(), String> {
    let db = crate::db().await?;
    match action {
        Action::Create { name, description } => create(db, name, description).await,
        Action::List => list(db).await,
        Action::AddUser { email, group } => add_user(db, email, group).await,
        Action::RemoveUser { email, group } => remove_user(db, email, group).await,
    }
}

async fn create(db: Db, name: String, description: String) -> Result<(), String> {
    auth::init_permission_tables(&db)
        .await
        .map_err(|e| format!("init: {e}"))?;
    let id = auth::create_group(&db, &name, &description)
        .await
        .map_err(|e| format!("create_group: {e}"))?;
    println!("Created group id={id} name={name}");
    Ok(())
}

async fn list(db: Db) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT g.id,
                g.name,
                g.description,
                COUNT(ug.user_id) AS member_count
           FROM rustio_groups g
           LEFT JOIN rustio_user_groups ug ON ug.group_id = g.id
          GROUP BY g.id, g.name, g.description
          ORDER BY g.name ASC",
    )
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("query: {e}"))?;

    if rows.is_empty() {
        println!("No groups.");
        return Ok(());
    }
    println!("{:>4}  {:<24}  {:>7}  DESCRIPTION", "ID", "NAME", "MEMBERS");
    for r in rows {
        let id: i64 = r.try_get("id").unwrap_or(0);
        let name: String = r.try_get("name").unwrap_or_default();
        let desc: String = r.try_get("description").unwrap_or_default();
        let count: i64 = r.try_get("member_count").unwrap_or(0);
        println!("{id:>4}  {name:<24}  {count:>7}  {desc}");
    }
    Ok(())
}

async fn add_user(db: Db, email: String, group: String) -> Result<(), String> {
    let user = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup user: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;
    let gid = lookup_group(&db, &group).await?;
    auth::add_user_to_group(&db, user.id, gid)
        .await
        .map_err(|e| format!("add_user_to_group: {e}"))?;
    println!("Added {email} to group {group}");
    Ok(())
}

async fn remove_user(db: Db, email: String, group: String) -> Result<(), String> {
    let user = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup user: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;
    let gid = lookup_group(&db, &group).await?;
    auth::remove_user_from_group(&db, user.id, gid)
        .await
        .map_err(|e| format!("remove_user_from_group: {e}"))?;
    println!("Removed {email} from group {group}");
    Ok(())
}

async fn lookup_group(db: &Db, name: &str) -> Result<i64, String> {
    let row = sqlx::query("SELECT id FROM rustio_groups WHERE name = $1")
        .bind(name)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("query group: {e}"))?;
    let row = row.ok_or_else(|| format!("no group named {name}"))?;
    row.try_get::<i64, _>("id")
        .map_err(|e| format!("group id: {e}"))
}
