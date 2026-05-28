//! `rustio-admin user` -- auth-table CRUD without the admin UI.
//!
//! ## R4 emergency-recovery surface
//!
//! In addition to the day-to-day CRUD (`create`, `list`, `role`,
//! `delete`), this module exposes the emergency-tier subcommands
//! from `DESIGN_R4_EMERGENCY.md` §3:
//! `reset-password / unlock / disable-mfa / promote /
//! emergency-access`. Each emergency command renders the locked
//! confirmation banner (D10), reads `--reason "<text>"` (validated
//! ≥ 8 chars), demands interactive `yes` confirm (unless `--yes`
//! is set), calls into `rustio_admin::auth::emergency` for the
//! atomic DB mutation, and writes a single
//! `AuditEvent::EmergencyRecovery` row with the §5 metadata
//! schema.

use clap::{Subcommand, ValueEnum};
use sqlx::Row as _;

use rustio_admin::admin::audit::{record, ActionType, AuditEvent, LogEntry};
use rustio_admin::auth::emergency::{
    self as fw_emergency, DisableMfaOutcome, EmergencyAccessOutcome, PromoteOutcome, ResetOutcome,
    UnlockOutcome,
};
use rustio_admin::auth::{DefaultPasswordPolicy, PasswordPolicy};
use rustio_admin::{auth, Db, Role};

use crate::emergency_ui::{self, ConfirmOutcome, OperationContext};

/// CLI surface for `Role`. clap's derive needs `ValueEnum` and we
/// deliberately keep the labels lowercase to match the SQL column.
///
/// PR 2.2 (0.21.0) added `Editor` and `Viewer`. They are NOT new
/// framework `Role` variants — both map to `Role::User`. Their
/// actual permissions come from the matching seeded group
/// (`auth::DEFAULT_GROUP_NAMES`). The CLI's `Create` action assigns
/// group membership on user-create so the role choice and the
/// group the user lands in are the same string.
///
/// The three values `administrator` / `editor` / `viewer` MUST stay
/// in exact lockstep with `auth::DEFAULT_GROUP_NAMES` — the
/// `lockstep_default_groups_match_cli_role_names` test below fails
/// CI if either side drifts.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum CliRole {
    User,
    Staff,
    Supervisor,
    Administrator,
    Developer,
    /// PR 2.2. Maps to `Role::User`; assigned to the `editor` group.
    Editor,
    /// PR 2.2. Maps to `Role::User`; assigned to the `viewer` group.
    Viewer,
}

impl From<CliRole> for Role {
    fn from(r: CliRole) -> Self {
        match r {
            CliRole::User => Role::User,
            CliRole::Staff => Role::Staff,
            CliRole::Supervisor => Role::Supervisor,
            CliRole::Administrator => Role::Administrator,
            CliRole::Developer => Role::Developer,
            // PR 2.2 — Editor / Viewer ride at the base User tier;
            // their real authority comes from group membership.
            CliRole::Editor | CliRole::Viewer => Role::User,
        }
    }
}

impl CliRole {
    /// Name of the seeded default group this role assigns to on
    /// `rustio-admin user create`, if any. Returns `None` for the legacy
    /// 5-tier roles which don't have an associated default group
    /// (their authority comes from the tier-level `check_permission`
    /// path: `Administrator` / `Developer` bypass; `Staff` /
    /// `Supervisor` / `User` rely on direct grants).
    pub fn default_group_name(&self) -> Option<&'static str> {
        match self {
            CliRole::Administrator => Some("administrator"),
            CliRole::Editor => Some("editor"),
            CliRole::Viewer => Some("viewer"),
            CliRole::User | CliRole::Staff | CliRole::Supervisor | CliRole::Developer => None,
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
        /// Optional given name. Surfaces in recovery-email greetings
        /// when display_name is unset.
        #[arg(long)]
        first_name: Option<String>,
        /// Optional family name. Pairs with first_name in the
        /// recovery-email signature block.
        #[arg(long)]
        last_name: Option<String>,
        /// Preferred display name. Takes precedence over first_name
        /// in greetings. Falls back to first_name → email-local-part.
        #[arg(long)]
        display_name: Option<String>,
        /// Job title -- second line of the email signature block.
        #[arg(long)]
        job_title: Option<String>,
    },
    /// List every user with id / email / role / active flag.
    List,
    /// Report a user's effective permissions: role, group
    /// memberships, direct grants, group-inherited grants, and the
    /// effective set the runtime's `check_permission` honours.
    /// Operational: useful for debugging "why can't user X see
    /// model Y" without reading the `rustio_user_permissions` /
    /// `rustio_group_permissions` tables by hand.
    Perms {
        #[arg(long)]
        email: String,
    },
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
    /// EMERGENCY: set a new password for a user, force password
    /// rotation on next login, revoke every session. Prints the
    /// temp password to stdout exactly once. Renders the locked
    /// banner and demands `--reason "<text>"` (≥ 8 chars). See
    /// `DESIGN_R4_EMERGENCY.md` §3.1.
    ResetPassword {
        #[arg(long)]
        email: String,
        /// Why this emergency was needed. Lands verbatim in the
        /// `EmergencyRecovery` audit row and the banner.
        /// Must be ≥ 8 trimmed characters.
        #[arg(long)]
        reason: String,
        /// Use this password instead of a generated 20-char random
        /// alphanumeric. Useful for scripted runs that pin the
        /// value out-of-band. The user must still change it on
        /// next login (`must_change_password = TRUE`).
        #[arg(long)]
        temp_password: Option<String>,
        /// Skip the interactive `yes` confirm prompt. The banner
        /// still renders to stdout -- D10 makes that irreducible.
        #[arg(long)]
        yes: bool,
    },
    /// EMERGENCY: clear an auto-throttle lock and zero out the
    /// failed-login counter. Renders the locked banner and demands
    /// `--reason "<text>"` (≥ 8 chars). Does NOT revoke sessions --
    /// an unlock is not a session event. See
    /// `DESIGN_R4_EMERGENCY.md` §3.2.
    Unlock {
        #[arg(long)]
        email: String,
        /// Why this emergency was needed. Lands verbatim in the
        /// `EmergencyRecovery` audit row and the banner.
        /// Must be ≥ 8 trimmed characters.
        #[arg(long)]
        reason: String,
        /// Skip the interactive `yes` confirm prompt. The banner
        /// still renders to stdout -- D10 makes that irreducible.
        #[arg(long)]
        yes: bool,
    },
    /// EMERGENCY: clear MFA on a user -- drop the TOTP secret + key
    /// id + replay-step + every backup code, then revoke every
    /// session for the user. Renders the locked banner and demands
    /// `--reason "<text>"` (≥ 8 chars). See
    /// `DESIGN_R4_EMERGENCY.md` §3.3.
    ///
    /// If the deployment's `MfaPolicy` is `Required`, the target
    /// user will be redirected to MFA enrolment on next login --
    /// the disable clears the state but does not exempt them from
    /// the policy. The summary line warns the operator.
    DisableMfa {
        #[arg(long)]
        email: String,
        /// Why this emergency was needed. Lands verbatim in the
        /// `EmergencyRecovery` audit row and the banner.
        /// Must be ≥ 8 trimmed characters.
        #[arg(long)]
        reason: String,
        /// Skip the interactive `yes` confirm prompt. The banner
        /// still renders to stdout -- D10 makes that irreducible.
        #[arg(long)]
        yes: bool,
    },
    /// EMERGENCY: change a user's role. Refuses to demote the sole
    /// active Administrator. Revokes the target's sessions so the
    /// new tier takes effect on the next login. Renders the locked
    /// banner and demands `--reason "<text>"` (≥ 8 chars). See
    /// `DESIGN_R4_EMERGENCY.md` §3.4.
    Promote {
        #[arg(long)]
        email: String,
        /// The new role. Persisted verbatim in
        /// `rustio_users.role`. Must be one of the five framework
        /// roles.
        #[arg(long = "to-role", value_enum)]
        to_role: CliRole,
        /// Why this emergency was needed. Lands verbatim in the
        /// `EmergencyRecovery` audit row and the banner.
        /// Must be ≥ 8 trimmed characters.
        #[arg(long)]
        reason: String,
        /// Skip the interactive `yes` confirm prompt. The banner
        /// still renders to stdout -- D10 makes that irreducible.
        #[arg(long)]
        yes: bool,
    },
    /// EMERGENCY: issue a single-use password-reset URL bypassing
    /// the email mailer. The URL plaintext prints to stdout once --
    /// hand it to the target out-of-band. Renders the locked banner
    /// and demands `--reason "<text>"` (≥ 8 chars). Refuses inactive
    /// targets (issuing a URL to a deactivated account has no
    /// recovery semantic). See `DESIGN_R4_EMERGENCY.md` §3.5.
    EmergencyAccess {
        #[arg(long)]
        email: String,
        /// Why this emergency was needed. Lands verbatim in the
        /// `EmergencyRecovery` audit row and the banner.
        /// Must be ≥ 8 trimmed characters.
        #[arg(long)]
        reason: String,
        /// URL validity in minutes. Default 15; clamped to
        /// `[1, 60]` inside the framework. Beyond 60 use
        /// `reset-password` instead -- wider TTLs widen the URL
        /// interception window for diminishing operational benefit.
        #[arg(long = "ttl-minutes", default_value_t = 15)]
        ttl_minutes: i64,
        /// Skip the interactive `yes` confirm prompt. The banner
        /// still renders to stdout -- D10 makes that irreducible.
        #[arg(long)]
        yes: bool,
    },
}

pub async fn run(action: Action) -> Result<(), String> {
    let db = crate::db().await?;
    match action {
        Action::Create {
            email,
            role,
            password,
            first_name,
            last_name,
            display_name,
            job_title,
        } => {
            create(
                db,
                email,
                role,
                password,
                first_name,
                last_name,
                display_name,
                job_title,
            )
            .await
        }
        Action::List => list(db).await,
        Action::Perms { email } => perms(db, email).await,
        Action::Role { email, role } => set_role(db, email, role.into()).await,
        Action::Delete { email } => delete(db, email).await,
        Action::ResetPassword {
            email,
            reason,
            temp_password,
            yes,
        } => reset_password(db, email, reason, temp_password, yes).await,
        Action::Unlock { email, reason, yes } => unlock(db, email, reason, yes).await,
        Action::DisableMfa { email, reason, yes } => disable_mfa(db, email, reason, yes).await,
        Action::Promote {
            email,
            to_role,
            reason,
            yes,
        } => promote(db, email, to_role.into(), reason, yes).await,
        Action::EmergencyAccess {
            email,
            reason,
            ttl_minutes,
            yes,
        } => emergency_access(db, email, reason, ttl_minutes, yes).await,
    }
}

#[allow(clippy::too_many_arguments)]
async fn create(
    db: Db,
    email: String,
    cli_role: CliRole,
    password: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    display_name: Option<String>,
    job_title: Option<String>,
) -> Result<(), String> {
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

    let role: Role = cli_role.into();
    let id = auth::create_user(&db, &email, &pw, role)
        .await
        .map_err(|e| format!("create_user: {e}"))?;

    // Profile identity columns -- populated post-create so the
    // existing `create_user` signature doesn't grow. All four are
    // optional; UPDATE sets only the columns the operator supplied.
    if first_name.is_some() || last_name.is_some() || display_name.is_some() || job_title.is_some()
    {
        sqlx::query(
            "UPDATE rustio_users SET \
                first_name   = COALESCE($1, first_name), \
                last_name    = COALESCE($2, last_name), \
                display_name = COALESCE($3, display_name), \
                job_title    = COALESCE($4, job_title) \
              WHERE id = $5",
        )
        .bind(first_name.as_deref())
        .bind(last_name.as_deref())
        .bind(display_name.as_deref())
        .bind(job_title.as_deref())
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(|e| format!("profile fields: {e}"))?;
    }

    println!("Created user id={id} email={email} role={role}");

    // PR 2.2 -- structural permission defaults. If the chosen role
    // maps to a seeded default group (`administrator` / `editor` /
    // `viewer`), add the new user to it so their authority comes
    // from the group's permission grants (and shows up in the
    // group's member list for UI / audit clarity). Looks up the
    // group by name; a missing group means `seed_default_groups`
    // was skipped on this project (user-defined-groups guard), in
    // which case we don't try to invent membership -- the operator
    // can wire it via the admin permission-matrix page.
    if let Some(group_name) = cli_role.default_group_name() {
        let group_id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM rustio_groups WHERE name = $1")
                .bind(group_name)
                .fetch_optional(db.pool())
                .await
                .map_err(|e| format!("lookup group `{group_name}`: {e}"))?;
        if let Some(gid) = group_id {
            auth::add_user_to_group(&db, id, gid)
                .await
                .map_err(|e| format!("add to group `{group_name}`: {e}"))?;
            println!("Added to group: {group_name}");
        }
    }
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

/// Structured snapshot of a user's permission state, returned by the
/// DB-fetching layer and consumed by the pure formatter. The two
/// layers are split so [`format_perms_report`] is unit-testable
/// without spinning up a Postgres pool.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PermsReport {
    email: String,
    id: i64,
    role_label: String,
    is_active: bool,
    is_demo: bool,
    bypasses_group_checks: bool,
    /// Group names the user belongs to, sorted alphabetically.
    groups: Vec<String>,
    /// Permissions granted directly to the user via
    /// `rustio_user_permissions`. Sorted.
    direct_perms: Vec<String>,
    /// Per-group permission grants. `(group_name, [perm, …])`.
    /// Groups appear in the same alphabetical order as `groups`.
    group_perms_by_group: Vec<(String, Vec<String>)>,
}

impl PermsReport {
    /// Union of direct + group-inherited permissions -- the set the
    /// runtime's `check_permission` honours for non-bypassing roles.
    fn effective_perms(&self) -> Vec<String> {
        use std::collections::BTreeSet;
        let mut set: BTreeSet<String> = BTreeSet::new();
        set.extend(self.direct_perms.iter().cloned());
        for (_, perms) in &self.group_perms_by_group {
            set.extend(perms.iter().cloned());
        }
        set.into_iter().collect()
    }
}

async fn perms(db: Db, email: String) -> Result<(), String> {
    let user = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;

    let groups = sqlx::query(
        "SELECT g.name
           FROM rustio_groups g
           JOIN rustio_user_groups ug ON ug.group_id = g.id
          WHERE ug.user_id = $1
          ORDER BY g.name",
    )
    .bind(user.id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("groups query: {e}"))?;

    let direct = sqlx::query(
        "SELECT p.name
           FROM rustio_permissions p
           JOIN rustio_user_permissions up ON up.permission_id = p.id
          WHERE up.user_id = $1
          ORDER BY p.name",
    )
    .bind(user.id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("direct-perms query: {e}"))?;

    let group_pairs = sqlx::query(
        "SELECT g.name AS group_name, p.name AS perm_name
           FROM rustio_permissions p
           JOIN rustio_group_permissions gp ON gp.permission_id = p.id
           JOIN rustio_groups g ON g.id = gp.group_id
           JOIN rustio_user_groups ug ON ug.group_id = gp.group_id
          WHERE ug.user_id = $1
          ORDER BY g.name, p.name",
    )
    .bind(user.id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("group-perms query: {e}"))?;

    let groups: Vec<String> = groups
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("name").ok())
        .collect();
    let direct_perms: Vec<String> = direct
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("name").ok())
        .collect();

    // Bucket group_pairs into Vec<(group, Vec<perm>)>. The SQL is
    // already sorted by group then perm so we can fold in O(n)
    // without re-sorting.
    let mut group_perms_by_group: Vec<(String, Vec<String>)> = Vec::new();
    for row in group_pairs {
        let g: String = row.try_get("group_name").unwrap_or_default();
        let p: String = row.try_get("perm_name").unwrap_or_default();
        match group_perms_by_group.last_mut() {
            Some((last, perms)) if last == &g => perms.push(p),
            _ => group_perms_by_group.push((g, vec![p])),
        }
    }

    let report = PermsReport {
        email: user.email,
        id: user.id,
        role_label: user.role.label().to_string(),
        is_active: user.is_active,
        is_demo: user.is_demo,
        bypasses_group_checks: user.role.bypasses_group_checks(),
        groups,
        direct_perms,
        group_perms_by_group,
    };
    print!("{}", format_perms_report(&report));
    Ok(())
}

/// Render the report as a human-readable block. Pure function -- no
/// IO, no clock, no DB -- so the unit tests can hand it synthetic
/// reports and assert exact output. The DB layer above builds the
/// report and prints what this returns.
fn format_perms_report(r: &PermsReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "User:      {}", r.email);
    let _ = writeln!(out, "ID:        {}", r.id);
    let role_line = if r.bypasses_group_checks {
        format!("{} (bypasses group checks)", r.role_label)
    } else {
        r.role_label.clone()
    };
    let _ = writeln!(out, "Role:      {role_line}");
    let _ = writeln!(
        out,
        "Active:    {}",
        if r.is_active {
            "yes"
        } else {
            "no -- every check_permission call denies"
        }
    );
    if r.is_demo {
        let _ = writeln!(out, "Demo:      yes");
    }
    out.push('\n');

    let _ = writeln!(out, "Groups:");
    if r.groups.is_empty() {
        let _ = writeln!(out, "  (none)");
    } else {
        for g in &r.groups {
            let _ = writeln!(out, "  {g}");
        }
    }
    out.push('\n');

    let _ = writeln!(out, "Direct permissions (rustio_user_permissions):");
    if r.direct_perms.is_empty() {
        let _ = writeln!(out, "  (none)");
    } else {
        for p in &r.direct_perms {
            let _ = writeln!(out, "  {p}");
        }
    }
    out.push('\n');

    let _ = writeln!(out, "Group permissions (inherited via memberships):");
    if r.group_perms_by_group.is_empty() {
        let _ = writeln!(out, "  (none)");
    } else {
        for (g, perms) in &r.group_perms_by_group {
            let _ = writeln!(out, "  via \"{g}\":");
            for p in perms {
                let _ = writeln!(out, "    {p}");
            }
        }
    }
    out.push('\n');

    let _ = writeln!(
        out,
        "Effective permissions (what check_permission honours):"
    );
    if r.bypasses_group_checks {
        // The role bypasses the M2M lookup entirely -- direct and
        // group grants are still listed above for transparency,
        // but the runtime treats this user as having every
        // permission regardless of the lookup result.
        let _ = writeln!(out, "  ★ all permissions (role bypasses group checks) ★");
    } else if !r.is_active {
        // Inactive users always deny; surface this prominently
        // since direct + group grants above might otherwise
        // mislead the operator.
        let _ = writeln!(out, "  (none -- user is inactive, every check denies)");
    } else {
        let eff = r.effective_perms();
        if eff.is_empty() {
            let _ = writeln!(out, "  (none)");
        } else {
            for p in &eff {
                let _ = writeln!(out, "  {p}");
            }
            let _ = writeln!(out, "  ({} total)", eff.len());
        }
    }
    out
}

async fn set_role(db: Db, email: String, role: Role) -> Result<(), String> {
    let user = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;

    // Last-protected-role guard mirrors the framework's user-edit
    // path. `would_orphan_protected` covers every protected role
    // (Administrator + Developer), not just Developer -- so a CLI-driven
    // role change can't orphan an Administrator either.
    if let Some(orphaned) = auth::would_orphan_protected(&db, user.id, role, true)
        .await
        .map_err(|e| format!("orphan check: {e}"))?
    {
        return Err(format!(
            "Refusing -- this change would leave the system with zero active {}s.",
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
            "Refusing -- deleting this user would leave zero active {}s.",
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

// ---- R4: shared preflight + audit-emission helpers -----------

/// Length of the auto-generated temp password when
/// `--temp-password` is not supplied. 20 alphanumeric chars from a
/// 54-character ambiguity-stripped alphabet ≈ 115 bits of entropy --
/// well above any plausible online attack envelope, and short
/// enough for the target to type accurately on next login.
const DEFAULT_TEMP_PASSWORD_LEN: usize = 20;

/// Steps 1-5 of every emergency-recovery handler: validate the
/// reason, resolve the target user, build the [`OperationContext`],
/// render the banner (D10), and demand confirm. Returns the
/// constructed context on success; returns the operator-facing
/// error message on rejection.
///
/// Per-operation handlers call this, then add their own steps 6+
/// (the framework call, audit emission, operator summary). The
/// extraction is identical across all five R4 commands; future
/// audits of the emergency surface walk this function rather than
/// per-handler copies.
async fn preflight(
    db: &Db,
    operation: &'static str,
    email: &str,
    reason_arg: &str,
    yes: bool,
) -> Result<OperationContext, String> {
    // Step 1 -- validate the reason. Surfaces typos / empty /
    // too-short BEFORE the DB roundtrip so the operator's first
    // error is fast.
    let reason = emergency_ui::validate_reason(reason_arg)?;

    // Step 2 -- resolve target. Loading the user here lets the
    // banner echo the persisted email + id + role, so a misspelled
    // --email surfaces before the operator confirms.
    let target = auth::find_user_by_email(db, email)
        .await
        .map_err(|e| format!("lookup target: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;

    // Step 3 -- build the banner context. `os_actor` + `now` are
    // stamped exactly once, here, and then re-used in both the
    // banner render and the audit metadata so the two surfaces
    // agree.
    let ctx = OperationContext {
        operation,
        target_email: target.email.clone(),
        target_user_id: target.id,
        target_role: target.role.to_string(),
        reason,
        os_actor: emergency_ui::os_actor(),
        when: emergency_ui::now(),
    };

    // Step 4 -- render the banner (D10 -- irreducible). ANSI /
    // colour is auto-detected: enabled only when stdout is a TTY
    // and `NO_COLOR` is unset.
    emergency_ui::print_banner(&ctx);

    // Step 5 -- confirm (or honour --yes).
    match emergency_ui::require_confirm(yes) {
        ConfirmOutcome::Confirmed => Ok(ctx),
        ConfirmOutcome::Aborted => {
            println!("Aborted.");
            Err("user did not confirm".into())
        }
        ConfirmOutcome::NeedsTtyOrYesFlag => {
            Err("Refusing to run without a TTY (or pass --yes for scripting)".to_string())
        }
    }
}

/// Write the `EmergencyRecovery` audit row for a completed
/// operation. Returns the freshly-stamped correlation_id so the
/// per-handler summary line can echo it for the operator's
/// records.
///
/// `cli_op` is the audit slug (`"reset_password" | "unlock" |
/// "disable_mfa" | "promote" | "emergency_access"`) -- distinct
/// from `ctx.operation` which is the kebab-case banner display
/// slug.
///
/// `per_op_metadata` MUST be a JSON object. Its top-level keys
/// are merged into the base metadata (cli_operation, reason,
/// os_actor, cli_invocation). Per-op keys can shadow base keys --
/// that's deliberate so a handler can override e.g. cli_invocation
/// for an unusual call site if ever needed.
///
/// D12 anchor: this is the ONLY function in the codebase that
/// emits `AuditEvent::EmergencyRecovery`. The cross-crate
/// visibility test in
/// `admin::audit::tests::emergency_recovery_is_cli_only` keeps it
/// that way.
async fn write_emergency_audit(
    db: &Db,
    ctx: &OperationContext,
    cli_op: &str,
    per_op_metadata: serde_json::Value,
) -> Result<String, String> {
    let correlation_id = fw_emergency::fresh_correlation_id();
    let argv: Vec<String> = std::env::args().collect();

    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "cli_operation".into(),
        serde_json::Value::String(cli_op.into()),
    );
    metadata.insert(
        "reason".into(),
        serde_json::Value::String(ctx.reason.clone()),
    );
    metadata.insert(
        "os_actor".into(),
        serde_json::Value::String(ctx.os_actor.clone()),
    );
    metadata.insert(
        "cli_invocation".into(),
        serde_json::Value::String(emergency_ui::redact_reason_in_argv(&argv)),
    );
    if let serde_json::Value::Object(extra) = per_op_metadata {
        for (k, v) in extra {
            metadata.insert(k, v);
        }
    }

    let summary = build_summary(ctx.operation, &ctx.reason);
    let entry = LogEntry {
        user_id: ctx.target_user_id,
        action_type: ActionType::Update,
        // The admin slug for the built-in User admin is `"users"` --
        // matches the dispatcher's `admin.find("users")` lookup so
        // the History page can render a working `/admin/users/:id`
        // link to this row. See `VISIBILITY_AUDIT.md` finding F1.
        model_name: "users",
        object_id: ctx.target_user_id,
        ip_address: None,
        summary,
        correlation_id: Some(&correlation_id),
        session_id: None,
        metadata: Some(serde_json::Value::Object(metadata)),
        actor_user_id: None,
        event: Some(AuditEvent::EmergencyRecovery),
    };
    record(db, entry)
        .await
        .map_err(|e| format!("audit record: {e}"))?;
    Ok(correlation_id)
}

// ---- R4: rustio-admin user reset-password -----------------

async fn reset_password(
    db: Db,
    email: String,
    reason_arg: String,
    temp_password: Option<String>,
    yes: bool,
) -> Result<(), String> {
    let ctx = preflight(&db, "reset-password", &email, &reason_arg, yes).await?;

    // Generate or accept the temp password. The CLI owns the
    // plaintext; the framework only ever sees the value as an
    // argument to `set_password` and stores the Argon2 hash.
    let temp_password = match temp_password {
        Some(p) => {
            // Even when operator-supplied, validate against the
            // password policy so an emergency reset can't bypass
            // length / complexity floors.
            let policy = DefaultPasswordPolicy::new();
            policy
                .validate(&p)
                .map_err(|e| format!("--temp-password rejected by policy: {e}"))?;
            p
        }
        None => fw_emergency::generate_temp_password(DEFAULT_TEMP_PASSWORD_LEN),
    };

    let outcome = fw_emergency::reset_password(&db, ctx.target_user_id, &temp_password)
        .await
        .map_err(|e| format!("reset_password: {e}"))?;
    let revoked = match outcome {
        ResetOutcome::Ok {
            revoked_session_count,
        } => revoked_session_count,
        ResetOutcome::UnknownTarget => {
            // Race: target existed at preflight, gone now. The
            // audit row is intentionally NOT written -- there was
            // nothing to record.
            return Err(format!(
                "User vanished between lookup and reset; no rows changed (email={email})"
            ));
        }
    };

    let correlation_id = write_emergency_audit(
        &db,
        &ctx,
        "reset_password",
        serde_json::json!({
            "revoked_session_count": revoked,
            "must_change_password_set": true,
        }),
    )
    .await?;

    println!();
    println!(
        "✓ Password reset for {email} (user_id={})",
        ctx.target_user_id
    );
    println!("  Sessions revoked: {revoked}");
    println!("  must_change_password set; user must rotate on next login.");
    println!();
    println!("  Temporary password (shown once -- record now):");
    println!();
    println!("      {temp_password}");
    println!();
    println!("  Audit correlation: {correlation_id}");

    Ok(())
}

// ---- R4: rustio-admin user unlock -------------------

async fn unlock(db: Db, email: String, reason_arg: String, yes: bool) -> Result<(), String> {
    let ctx = preflight(&db, "unlock", &email, &reason_arg, yes).await?;

    let outcome = fw_emergency::unlock(&db, ctx.target_user_id)
        .await
        .map_err(|e| format!("unlock: {e}"))?;
    let previously_locked = match outcome {
        UnlockOutcome::Ok { previously_locked } => previously_locked,
        UnlockOutcome::UnknownTarget => {
            return Err(format!(
                "User vanished between lookup and unlock; no rows changed (email={email})"
            ));
        }
    };

    let correlation_id = write_emergency_audit(
        &db,
        &ctx,
        "unlock",
        serde_json::json!({
            "previously_locked": previously_locked,
        }),
    )
    .await?;

    println!();
    println!(
        "✓ Unlock applied to {email} (user_id={})",
        ctx.target_user_id
    );
    if previously_locked {
        println!("  Account was actively locked; locked_until + failed_login_count cleared.");
    } else {
        println!("  Note: account was not locked at run time; no functional change.");
        println!("  (The audit row still landed -- the action remains forensically visible.)");
    }
    println!("  Audit correlation: {correlation_id}");

    Ok(())
}

// ---- R4: rustio-admin user disable-mfa -----------------

async fn disable_mfa(db: Db, email: String, reason_arg: String, yes: bool) -> Result<(), String> {
    let ctx = preflight(&db, "disable-mfa", &email, &reason_arg, yes).await?;

    let outcome = fw_emergency::disable_mfa(&db, ctx.target_user_id)
        .await
        .map_err(|e| format!("disable_mfa: {e}"))?;
    let (was_enabled, deleted_backup_codes, revoked) = match outcome {
        DisableMfaOutcome::Ok {
            was_enabled,
            deleted_backup_codes,
            revoked_session_count,
        } => (was_enabled, deleted_backup_codes, revoked_session_count),
        DisableMfaOutcome::UnknownTarget => {
            return Err(format!(
                "User vanished between lookup and disable_mfa; no rows changed (email={email})"
            ));
        }
    };

    let correlation_id = write_emergency_audit(
        &db,
        &ctx,
        "disable_mfa",
        serde_json::json!({
            "was_enabled": was_enabled,
            "deleted_backup_codes": deleted_backup_codes,
            "revoked_session_count": revoked,
        }),
    )
    .await?;

    println!();
    println!("✓ MFA disabled on {email} (user_id={})", ctx.target_user_id);
    if was_enabled {
        println!(
            "  MFA secret cleared. {deleted_backup_codes} backup code(s) deleted. {revoked} session(s) revoked."
        );
    } else {
        println!("  Note: MFA was not enabled at run time; no functional change.");
        println!("  (The audit row still landed -- the action remains forensically visible.)");
    }
    println!();
    println!("  If the deployment's MfaPolicy is `Required` (or `RequiredForRoles`),");
    println!("  the user will be redirected to MFA enrolment on their next login.");
    println!("  Audit correlation: {correlation_id}");

    Ok(())
}

// ---- R4: rustio-admin user promote ------------------

async fn promote(
    db: Db,
    email: String,
    new_role: Role,
    reason_arg: String,
    yes: bool,
) -> Result<(), String> {
    let ctx = preflight(&db, "promote", &email, &reason_arg, yes).await?;

    let outcome = fw_emergency::promote(&db, ctx.target_user_id, new_role)
        .await
        .map_err(|e| format!("promote: {e}"))?;

    match outcome {
        PromoteOutcome::UnknownTarget => Err(format!(
            "User vanished between lookup and promote; no rows changed (email={email})"
        )),
        PromoteOutcome::SoleAdministratorDemoteRefused => {
            // Refused inside the framework. No audit row is
            // written -- refusing isn't a state change.
            Err(format!(
                "Refused: {email} is the sole active administrator; demoting them would leave \
                 the deployment with zero administrators. Promote another user to administrator \
                 first, then re-run."
            ))
        }
        PromoteOutcome::NoChange { current_role } => {
            // The target already carries `new_role`. The framework
            // skipped the UPDATE; we still write an audit row so
            // the forensic log shows the operator ran the command.
            let correlation_id = write_emergency_audit(
                &db,
                &ctx,
                "promote",
                serde_json::json!({
                    "previous_role": current_role.to_string(),
                    "new_role": new_role.to_string(),
                    "no_change": true,
                }),
            )
            .await?;

            println!();
            println!(
                "✓ Promote applied to {email} (user_id={})",
                ctx.target_user_id
            );
            println!("  Note: user already carried role={current_role}; no functional change.");
            println!("  (The audit row still landed -- the action remains forensically visible.)");
            println!("  Audit correlation: {correlation_id}");
            Ok(())
        }
        PromoteOutcome::Ok {
            previous_role,
            new_role,
            revoked_session_count,
        } => {
            let correlation_id = write_emergency_audit(
                &db,
                &ctx,
                "promote",
                serde_json::json!({
                    "previous_role": previous_role.to_string(),
                    "new_role": new_role.to_string(),
                    "revoked_session_count": revoked_session_count,
                }),
            )
            .await?;

            println!();
            println!(
                "✓ Promoted {email} (user_id={}) {previous_role} → {new_role}",
                ctx.target_user_id
            );
            println!("  Sessions revoked: {revoked_session_count}");
            println!("  The user must re-authenticate to pick up the new tier.");
            println!("  Audit correlation: {correlation_id}");
            Ok(())
        }
    }
}

// ---- R4: rustio-admin user emergency-access ---------------

async fn emergency_access(
    db: Db,
    email: String,
    reason_arg: String,
    ttl_minutes: i64,
    yes: bool,
) -> Result<(), String> {
    let ctx = preflight(&db, "emergency-access", &email, &reason_arg, yes).await?;

    let outcome = fw_emergency::emergency_access(&db, ctx.target_user_id, ttl_minutes)
        .await
        .map_err(|e| format!("emergency_access: {e}"))?;

    let (token_id, url_path, expires_at, effective_ttl) = match outcome {
        EmergencyAccessOutcome::Ok {
            token_id,
            url_path,
            expires_at,
        } => (token_id, url_path, expires_at, ttl_minutes.clamp(1, 60)),
        EmergencyAccessOutcome::UnknownTarget => {
            return Err(format!(
                "User vanished between lookup and emergency_access; no token issued (email={email})"
            ));
        }
        EmergencyAccessOutcome::InactiveTarget => {
            return Err(format!(
                "Refused: {email} is deactivated. Emergency-access only issues URLs to active \
                 accounts (a URL into a deactivated account has no recovery semantic). \
                 Reactivate the user first via `rustio-admin user role` or update `is_active` \
                 directly, then re-run."
            ));
        }
    };

    // The audit row carries `token_id` (linkable to
    // `rustio_password_reset_tokens.id`) and `expires_at`, but
    // NEVER the URL plaintext -- the URL embeds the single-use
    // token. Persisting it in audit metadata would defeat the
    // single-use property by giving an audit-log reader a
    // working credential.
    let correlation_id = write_emergency_audit(
        &db,
        &ctx,
        "emergency_access",
        serde_json::json!({
            "token_id": token_id,
            "ttl_minutes": effective_ttl,
            "expires_at": expires_at.to_rfc3339(),
        }),
    )
    .await?;

    println!();
    println!(
        "✓ Emergency-access URL issued for {email} (user_id={})",
        ctx.target_user_id
    );
    println!("  Token id: {token_id}");
    println!(
        "  Expires:  {} (in {effective_ttl} minute(s))",
        expires_at.to_rfc3339()
    );
    println!();
    println!("  URL (shown once -- hand to target out-of-band):");
    println!();
    println!("      <BASE_URL>{url_path}");
    println!();
    println!("  Prefix <BASE_URL> with your deployment's admin URL");
    println!("  (e.g., https://admin.example.com → full URL would be");
    println!("  https://admin.example.com{url_path}).");
    println!("  Single-use: consuming the token writes consumed_at=NOW().");
    println!("  Audit correlation: {correlation_id}");

    Ok(())
}

/// Truncate the reason to ≤ 200 chars for the audit row's
/// `summary` column. The full reason is also in
/// `metadata.reason`; the summary is the human-readable hook on
/// `/admin/history`.
fn build_summary(op: &str, reason: &str) -> String {
    let mut preview = String::with_capacity(op.len() + 2 + 200);
    preview.push_str(op);
    preview.push_str(": ");
    let limit = 200;
    let total = reason.chars().count();
    for c in reason.chars().take(limit) {
        preview.push(c);
    }
    if total > limit {
        preview.push('…');
    }
    preview
}

// ---- Tests ---------------------------

#[cfg(test)]
mod tests {
    use super::{build_summary, format_perms_report, CliRole, PermsReport};

    /// PR 2.2 lockstep guard. The three names in
    /// `auth::DEFAULT_GROUP_NAMES` MUST exactly match the set of
    /// `CliRole` variants that return `Some(name)` from
    /// `default_group_name()`. Renaming either side without the
    /// other -- or adding a new default role/group on only one
    /// side -- fails this test and blocks CI.
    ///
    /// Required behaviour, not a coincidence. Reviewers who hit
    /// this failure should NOT relax the assertion; they should
    /// either revert the asymmetric change or extend both sides.
    #[test]
    fn lockstep_default_groups_match_cli_role_names() {
        use rustio_admin::auth::DEFAULT_GROUP_NAMES;
        use std::collections::BTreeSet;

        let groups: BTreeSet<&'static str> = DEFAULT_GROUP_NAMES.iter().copied().collect();

        // Every CliRole variant that claims a default group.
        let cli_roles: BTreeSet<&'static str> = [
            CliRole::User,
            CliRole::Staff,
            CliRole::Supervisor,
            CliRole::Administrator,
            CliRole::Developer,
            CliRole::Editor,
            CliRole::Viewer,
        ]
        .iter()
        .filter_map(|r| r.default_group_name())
        .collect();

        assert_eq!(
            groups, cli_roles,
            "lockstep drift: rustio_admin::auth::DEFAULT_GROUP_NAMES ({groups:?}) \
             must equal the set of CliRole values that return Some from \
             default_group_name() ({cli_roles:?})"
        );

        // Sanity: the lockstep set is non-empty and equals exactly
        // the three doctrine names. If a future PR legitimately
        // adds a fourth default role, update both lists AND the
        // expected set below.
        let expected: BTreeSet<&'static str> =
            ["administrator", "editor", "viewer"].into_iter().collect();
        assert_eq!(
            groups, expected,
            "doctrine guard: the seeded default groups should be exactly \
             {{administrator, editor, viewer}} per DESIGN_PERMISSIONS.md"
        );
    }

    fn base_report() -> PermsReport {
        PermsReport {
            email: "alice@example.test".into(),
            id: 42,
            role_label: "user".into(),
            is_active: true,
            is_demo: false,
            bypasses_group_checks: false,
            groups: vec![],
            direct_perms: vec![],
            group_perms_by_group: vec![],
        }
    }

    #[test]
    fn perms_report_admin_bypass_shows_all_permissions_marker() {
        let r = PermsReport {
            role_label: "administrator".into(),
            bypasses_group_checks: true,
            ..base_report()
        };
        let out = format_perms_report(&r);
        assert!(out.contains("administrator (bypasses group checks)"));
        assert!(out.contains("★ all permissions (role bypasses group checks) ★"));
    }

    #[test]
    fn perms_report_inactive_user_shows_deny_marker() {
        // Even if direct grants exist, an inactive user fails
        // every check_permission. The report must surface this.
        let r = PermsReport {
            is_active: false,
            direct_perms: vec!["posts.view_post".into()],
            ..base_report()
        };
        let out = format_perms_report(&r);
        assert!(out.contains("Active:    no -- every check_permission call denies"));
        assert!(out.contains("(none -- user is inactive, every check denies)"));
        // Direct grants are still printed above the effective
        // section for transparency.
        assert!(out.contains("posts.view_post"));
    }

    #[test]
    fn perms_report_empty_user_uses_none_markers() {
        // A freshly-created user with role=user, no groups,
        // no direct grants. Every section should read "(none)".
        let r = base_report();
        let out = format_perms_report(&r);
        assert!(out.contains("Groups:\n  (none)"));
        assert!(out.contains("Direct permissions (rustio_user_permissions):\n  (none)"));
        assert!(out.contains("Group permissions (inherited via memberships):\n  (none)"));
        assert!(out.contains("Effective permissions (what check_permission honours):\n  (none)"));
    }

    #[test]
    fn perms_report_effective_unions_direct_and_group() {
        let r = PermsReport {
            groups: vec!["Editors".into()],
            direct_perms: vec!["posts.view_post".into()],
            group_perms_by_group: vec![(
                "Editors".into(),
                vec!["posts.change_post".into(), "posts.view_post".into()],
            )],
            ..base_report()
        };
        let out = format_perms_report(&r);
        // Direct + group-section bookkeeping.
        assert!(out.contains("Groups:\n  Editors"));
        assert!(out.contains("Direct permissions"));
        assert!(out.contains("  posts.view_post"));
        assert!(out.contains("  via \"Editors\":"));
        assert!(out.contains("    posts.change_post"));
        // Effective is the deduped union -- view_post appears in
        // both halves; the effective listing has it ONCE.
        let effective_block = out
            .split_once("Effective permissions")
            .expect("effective section exists")
            .1;
        assert_eq!(effective_block.matches("posts.view_post").count(), 1);
        assert!(effective_block.contains("posts.change_post"));
        assert!(effective_block.contains("(2 total)"));
    }

    #[test]
    fn perms_report_admin_with_grants_still_lists_them_for_transparency() {
        // An administrator might also have direct grants (unusual
        // but legal). The grants must be visible in the report
        // even though the effective set collapses to "all
        // permissions" via bypass. Operators auditing role
        // changes need to see what's there.
        let r = PermsReport {
            role_label: "administrator".into(),
            bypasses_group_checks: true,
            direct_perms: vec!["posts.delete_post".into()],
            ..base_report()
        };
        let out = format_perms_report(&r);
        assert!(out.contains("  posts.delete_post"));
        assert!(out.contains("★ all permissions"));
    }

    #[test]
    fn perms_report_demo_user_shows_demo_marker() {
        let r = PermsReport {
            is_demo: true,
            ..base_report()
        };
        let out = format_perms_report(&r);
        assert!(out.contains("Demo:      yes"));
    }

    #[test]
    fn effective_perms_deduplicates_across_groups() {
        // Same permission granted via two different groups
        // should appear ONCE in the effective set.
        let r = PermsReport {
            group_perms_by_group: vec![
                ("Editors".into(), vec!["posts.view_post".into()]),
                ("Reviewers".into(), vec!["posts.view_post".into()]),
            ],
            ..base_report()
        };
        let eff = r.effective_perms();
        assert_eq!(eff, vec!["posts.view_post".to_string()]);
    }

    #[test]
    fn summary_short_reason_pass_through() {
        let s = build_summary("reset-password", "lost MFA device");
        assert_eq!(s, "reset-password: lost MFA device");
    }

    #[test]
    fn summary_truncates_at_200_chars_with_ellipsis() {
        let long = "x".repeat(250);
        let s = build_summary("reset-password", &long);
        // "reset-password: " is 16 chars; then 200 x's; then …
        assert!(s.ends_with('…'));
        let body = s.trim_start_matches("reset-password: ");
        let body = body.trim_end_matches('…');
        assert_eq!(body.chars().count(), 200);
    }

    #[test]
    fn summary_handles_unicode() {
        // Composed character counts as one char, not its byte length.
        let reason = "räddade en ångbåt".to_string();
        let s = build_summary("reset-password", &reason);
        assert_eq!(s, format!("reset-password: {reason}"));
    }
}
