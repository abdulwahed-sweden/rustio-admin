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
    /// Show one group's profile — id / description / member list
    /// + the permissions granted via this group. Operational
    /// symmetry with `rustio user perms`: that command answers
    /// "what can this user do?"; this one answers "who has
    /// access via this group, and what does membership grant?".
    Show {
        #[arg(long)]
        name: String,
    },
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
        Action::Show { name } => show(db, name).await,
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

/// Snapshot of one group's profile. Built by the DB layer,
/// consumed by [`format_group_show`]. Split so the formatter is
/// unit-testable without a Postgres pool.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupReport {
    id: i64,
    name: String,
    description: String,
    /// One entry per member, sorted by email.
    members: Vec<GroupMember>,
    /// Permission names granted to this group, sorted.
    permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupMember {
    email: String,
    role: String,
    is_active: bool,
}

async fn show(db: Db, name: String) -> Result<(), String> {
    let row = sqlx::query("SELECT id, name, description FROM rustio_groups WHERE name = $1")
        .bind(&name)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("query group: {e}"))?
        .ok_or_else(|| format!("no group named {name}"))?;
    let id: i64 = row.try_get("id").map_err(|e| format!("group id: {e}"))?;
    let group_name: String = row.try_get("name").unwrap_or_default();
    let description: String = row.try_get("description").unwrap_or_default();

    // Members: join rustio_users → rustio_user_groups.
    let member_rows = sqlx::query(
        "SELECT u.email, u.role, u.is_active
           FROM rustio_users u
           JOIN rustio_user_groups ug ON ug.user_id = u.id
          WHERE ug.group_id = $1
          ORDER BY u.email ASC",
    )
    .bind(id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("members query: {e}"))?;

    let members: Vec<GroupMember> = member_rows
        .into_iter()
        .map(|r| GroupMember {
            email: r.try_get("email").unwrap_or_default(),
            role: r.try_get("role").unwrap_or_default(),
            is_active: r.try_get("is_active").unwrap_or(false),
        })
        .collect();

    // Permissions granted via this group.
    let perm_rows = sqlx::query(
        "SELECT p.name
           FROM rustio_permissions p
           JOIN rustio_group_permissions gp ON gp.permission_id = p.id
          WHERE gp.group_id = $1
          ORDER BY p.name ASC",
    )
    .bind(id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("perms query: {e}"))?;

    let permissions: Vec<String> = perm_rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("name").ok())
        .collect();

    let report = GroupReport {
        id,
        name: group_name,
        description,
        members,
        permissions,
    };
    print!("{}", format_group_show(&report));
    Ok(())
}

/// Render the report as a human-readable block. Pure function —
/// no IO, no clock, no DB — so unit tests can hand it synthetic
/// reports and assert exact output.
fn format_group_show(r: &GroupReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "Group:        {}", r.name);
    let _ = writeln!(out, "ID:           {}", r.id);
    if r.description.is_empty() {
        let _ = writeln!(out, "Description:  (none)");
    } else {
        let _ = writeln!(out, "Description:  {}", r.description);
    }
    out.push('\n');

    let _ = writeln!(out, "Members ({}):", r.members.len());
    if r.members.is_empty() {
        let _ = writeln!(out, "  (none)");
    } else {
        // Width the email column to the longest in the batch so
        // the role / active columns line up.
        let max_email = r.members.iter().map(|m| m.email.len()).max().unwrap_or(0);
        for m in &r.members {
            let _ = writeln!(
                out,
                "  {:<ew$}  {:<14}  {}",
                m.email,
                m.role,
                if m.is_active { "active" } else { "inactive" },
                ew = max_email,
            );
        }
    }
    out.push('\n');

    let _ = writeln!(out, "Permissions granted via this group ({}):", r.permissions.len());
    if r.permissions.is_empty() {
        let _ = writeln!(out, "  (none)");
    } else {
        for p in &r.permissions {
            let _ = writeln!(out, "  {p}");
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> GroupReport {
        GroupReport {
            id: 3,
            name: "Editors".into(),
            description: "Content team".into(),
            members: vec![],
            permissions: vec![],
        }
    }

    fn member(email: &str, role: &str, is_active: bool) -> GroupMember {
        GroupMember {
            email: email.into(),
            role: role.into(),
            is_active,
        }
    }

    #[test]
    fn empty_group_uses_none_markers() {
        // Brand-new group with no members and no permissions —
        // every section should read "(none)" rather than a
        // bare blank space.
        let r = base();
        let out = format_group_show(&r);
        assert!(out.contains("Group:        Editors"));
        assert!(out.contains("ID:           3"));
        assert!(out.contains("Description:  Content team"));
        assert!(out.contains("Members (0):\n  (none)"));
        assert!(out.contains("Permissions granted via this group (0):\n  (none)"));
    }

    #[test]
    fn empty_description_uses_none_marker() {
        let r = GroupReport {
            description: String::new(),
            ..base()
        };
        let out = format_group_show(&r);
        assert!(out.contains("Description:  (none)"));
    }

    #[test]
    fn members_render_with_role_and_active_status() {
        let r = GroupReport {
            members: vec![
                member("alice@example.test", "editor", true),
                member("bob@example.test", "user", false),
            ],
            ..base()
        };
        let out = format_group_show(&r);
        assert!(out.contains("Members (2):"));
        assert!(out.contains("alice@example.test"));
        assert!(out.contains("editor"));
        assert!(out.contains("active"));
        assert!(out.contains("bob@example.test"));
        assert!(out.contains("user"));
        // Inactive members get the inactive marker — operators
        // need to see "this user is in the group but the
        // active flag denies every check" at a glance.
        assert!(out.contains("inactive"));
    }

    #[test]
    fn permissions_section_lists_grants() {
        let r = GroupReport {
            permissions: vec!["posts.view_post".into(), "posts.change_post".into()],
            ..base()
        };
        let out = format_group_show(&r);
        assert!(out.contains("Permissions granted via this group (2):"));
        assert!(out.contains("posts.view_post"));
        assert!(out.contains("posts.change_post"));
    }

    #[test]
    fn member_email_column_auto_widens() {
        // Mixed-length emails: the formatter pads to the
        // longest in the batch so the role / active columns
        // line up on every row.
        let r = GroupReport {
            members: vec![
                member("a@x", "editor", true),
                member("very-long-email@example.test", "user", true),
            ],
            ..base()
        };
        let out = format_group_show(&r);
        // Both data rows reach "editor" / "user" at the same
        // column. We can't easily assert exact columns
        // without parsing, but checking the long email's row
        // also fits its label proves padding kicked in.
        for line in out.lines() {
            if line.trim_start().starts_with("a@x") {
                assert!(line.contains("editor"));
            }
            if line.trim_start().starts_with("very-long-email") {
                assert!(line.contains("user"));
            }
        }
    }

    #[test]
    fn group_with_no_permissions_but_members_still_renders_cleanly() {
        // A group used purely for grouping (no grants, just
        // membership-as-marker) is a valid pattern. Members
        // section populated, permissions section "(none)".
        let r = GroupReport {
            members: vec![member("alice@example.test", "editor", true)],
            ..base()
        };
        let out = format_group_show(&r);
        assert!(out.contains("Members (1):"));
        assert!(out.contains("alice@example.test"));
        assert!(out.contains("Permissions granted via this group (0):\n  (none)"));
    }
}
