//! `rustio user` — auth-table CRUD without the admin UI.

use clap::{Subcommand, ValueEnum};
use sqlx::Row as _;

use rustio_admin::auth::{DefaultPasswordPolicy, PasswordPolicy};
use rustio_admin::{auth, Db, Role};

/// CLI surface for `Role`. clap's derive needs `ValueEnum` and we
/// deliberately keep the labels lowercase to match the SQL column.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum CliRole {
    User,
    Staff,
    Supervisor,
    Administrator,
    Developer,
}

impl From<CliRole> for Role {
    fn from(r: CliRole) -> Self {
        match r {
            CliRole::User => Role::User,
            CliRole::Staff => Role::Staff,
            CliRole::Supervisor => Role::Supervisor,
            CliRole::Administrator => Role::Administrator,
            CliRole::Developer => Role::Developer,
        }
    }
}

#[derive(Subcommand)]
pub enum Action {
    /// Create a new user. Prompts for the password unless --password
    /// is provided. Fresh databases get auth tables created on the
    /// first call so this works as a bootstrap step.
    Create {
        #[arg(long)]
        email: String,
        #[arg(long, value_enum, default_value_t = CliRole::User)]
        role: CliRole,
        /// Provide the password inline (CI / scripting). Skips the
        /// interactive confirm-twice prompt.
        #[arg(long)]
        password: Option<String>,
    },
    /// List every user with id / email / role / active flag.
    List,
    /// Set the role on an existing user.
    Role {
        #[arg(long)]
        email: String,
        #[arg(value_enum)]
        role: CliRole,
    },
    /// Delete a user (cascades sessions, group memberships, direct grants).
    Delete {
        #[arg(long)]
        email: String,
    },
}

pub async fn run(action: Action) -> Result<(), String> {
    let db = crate::db().await?;
    match action {
        Action::Create {
            email,
            role,
            password,
        } => create(db, email, role.into(), password).await,
        Action::List => list(db).await,
        Action::Role { email, role } => set_role(db, email, role.into()).await,
        Action::Delete { email } => delete(db, email).await,
    }
}

async fn create(db: Db, email: String, role: Role, password: Option<String>) -> Result<(), String> {
    auth::init_tables(&db)
        .await
        .map_err(|e| format!("init auth tables: {e}"))?;

    if auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup: {e}"))?
        .is_some()
    {
        return Err(format!("a user with email {email} already exists"));
    }

    let pw = match password {
        Some(p) => p,
        None => prompt_new_password()?,
    };
    // Delegate to the framework's `DefaultPasswordPolicy` so the CLI
    // floor stays in lockstep with admin-create-user and self-service
    // password recovery (`DESIGN_R2_ORGANISATIONAL.md` §11). Projects
    // that override `Admin::password_policy(...)` get their stronger
    // policy on the web surfaces; the CLI is a bootstrap tool with
    // no `Admin` instance available, so it uses the default floor
    // (currently 10 chars).
    let policy = DefaultPasswordPolicy::new();
    policy.validate(&pw).map_err(|e| e.to_string())?;

    let id = auth::create_user(&db, &email, &pw, role)
        .await
        .map_err(|e| format!("create_user: {e}"))?;
    println!("Created user id={id} email={email} role={role}");
    Ok(())
}

async fn list(db: Db) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT id, email, role, is_active, created_at
           FROM rustio_users
          ORDER BY id ASC",
    )
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("query: {e}"))?;

    if rows.is_empty() {
        println!("No users.");
        return Ok(());
    }

    println!(
        "{:>4}  {:<32}  {:<14}  {:<6}  CREATED",
        "ID", "EMAIL", "ROLE", "ACTIVE"
    );
    for r in rows {
        let id: i64 = r.try_get("id").unwrap_or(0);
        let email: String = r.try_get("email").unwrap_or_default();
        let role: String = r.try_get("role").unwrap_or_default();
        let active: bool = r.try_get("is_active").unwrap_or(false);
        let created: chrono::DateTime<chrono::Utc> = r
            .try_get("created_at")
            .unwrap_or_else(|_| chrono::Utc::now());
        println!(
            "{:>4}  {:<32}  {:<14}  {:<6}  {}",
            id,
            email,
            role,
            if active { "yes" } else { "no" },
            created.format("%Y-%m-%d %H:%M UTC")
        );
    }
    Ok(())
}

async fn set_role(db: Db, email: String, role: Role) -> Result<(), String> {
    let user = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;

    // Last-protected-role guard mirrors the framework's user-edit
    // path. `would_orphan_protected` covers every protected role
    // (Administrator + Developer), not just Developer — so a CLI-driven
    // role change can't orphan an Administrator either.
    if let Some(orphaned) = auth::would_orphan_protected(&db, user.id, role, true)
        .await
        .map_err(|e| format!("orphan check: {e}"))?
    {
        return Err(format!(
            "Refusing — this change would leave the system with zero active {}s.",
            orphaned.label()
        ));
    }

    auth::update_user_role(&db, user.id, role)
        .await
        .map_err(|e| format!("update_user_role: {e}"))?;
    println!("Set role of {email} to {role}");
    Ok(())
}

async fn delete(db: Db, email: String) -> Result<(), String> {
    let user = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;

    if let Some(orphaned) = auth::would_orphan_protected(&db, user.id, Role::User, false)
        .await
        .map_err(|e| format!("orphan check: {e}"))?
    {
        return Err(format!(
            "Refusing — deleting this user would leave zero active {}s.",
            orphaned.label()
        ));
    }

    sqlx::query("DELETE FROM rustio_users WHERE id = $1")
        .bind(user.id)
        .execute(db.pool())
        .await
        .map_err(|e| format!("delete: {e}"))?;
    println!("Deleted user id={} email={email}", user.id);
    Ok(())
}

/// Confirm-twice password prompt. Both reads are echo-suppressed.
fn prompt_new_password() -> Result<String, String> {
    let pw1 =
        rpassword::prompt_password("Password: ").map_err(|e| format!("read password: {e}"))?;
    let pw2 = rpassword::prompt_password("Confirm password: ")
        .map_err(|e| format!("read password: {e}"))?;
    if pw1 != pw2 {
        return Err("Passwords don't match.".into());
    }
    Ok(pw1)
}
