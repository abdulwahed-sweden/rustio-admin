//! `rustio user` — auth-table CRUD without the admin UI.
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
use rustio_admin::auth::emergency::{self as fw_emergency, ResetOutcome};
use rustio_admin::auth::{DefaultPasswordPolicy, PasswordPolicy};
use rustio_admin::{auth, Db, Role};

use crate::emergency_ui::{self, ConfirmOutcome, OperationContext};

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
        /// still renders to stdout — D10 makes that irreducible.
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
        } => create(db, email, role.into(), password).await,
        Action::List => list(db).await,
        Action::Role { email, role } => set_role(db, email, role.into()).await,
        Action::Delete { email } => delete(db, email).await,
        Action::ResetPassword {
            email,
            reason,
            temp_password,
            yes,
        } => reset_password(db, email, reason, temp_password, yes).await,
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

// ---- R4: rustio user reset-password --------------------------------------

/// Length of the auto-generated temp password when `--temp-password`
/// is not supplied. 20 alphanumeric chars from a 54-character
/// ambiguity-stripped alphabet ≈ 115 bits of entropy — well above
/// any plausible online attack envelope, and short enough for the
/// target to type accurately on the next login.
const DEFAULT_TEMP_PASSWORD_LEN: usize = 20;

async fn reset_password(
    db: Db,
    email: String,
    reason_arg: String,
    temp_password: Option<String>,
    yes: bool,
) -> Result<(), String> {
    // Step 1 — validate the reason. Surfaces typos / empty / too-short
    // BEFORE the DB roundtrip so the operator's first error is fast.
    let reason = emergency_ui::validate_reason(&reason_arg)?;

    // Step 2 — resolve target. Loading the user here lets the banner
    // echo the persisted email + id + role, so a misspelled --email
    // surfaces before the operator confirms.
    let target = auth::find_user_by_email(&db, &email)
        .await
        .map_err(|e| format!("lookup target: {e}"))?
        .ok_or_else(|| format!("no user with email {email}"))?;

    // Step 3 — build the banner context. `os_actor` + `now` are
    // stamped exactly once, here, and then re-used in both the
    // banner render and the audit metadata so the two surfaces
    // agree.
    let ctx = OperationContext {
        operation: "reset-password",
        target_email: target.email.clone(),
        target_user_id: target.id,
        target_role: target.role.to_string(),
        reason: reason.clone(),
        os_actor: emergency_ui::os_actor(),
        when: emergency_ui::now(),
    };

    // Step 4 — render the banner (D10 — irreducible). ANSI / colour
    // is auto-detected: enabled only when stdout is a TTY and
    // `NO_COLOR` is unset.
    emergency_ui::print_banner(&ctx);

    // Step 5 — confirm (or honour --yes).
    match emergency_ui::require_confirm(yes) {
        ConfirmOutcome::Confirmed => {}
        ConfirmOutcome::Aborted => {
            println!("Aborted.");
            return Err("user did not confirm".into());
        }
        ConfirmOutcome::NeedsTtyOrYesFlag => {
            return Err("Refusing to run without a TTY (or pass --yes for scripting)".to_string());
        }
    }

    // Step 6 — generate or accept the temp password. The CLI owns
    // the plaintext; the framework only ever sees the value as an
    // argument to `set_password` and stores the Argon2 hash. Avoid
    // logging this anywhere.
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

    // Step 7 — call the framework's atomic mutation.
    let outcome = fw_emergency::reset_password(&db, target.id, &temp_password)
        .await
        .map_err(|e| format!("reset_password: {e}"))?;
    let revoked = match outcome {
        ResetOutcome::Ok {
            revoked_session_count,
        } => revoked_session_count,
        ResetOutcome::UnknownTarget => {
            // Race: target existed at step 2, gone now. The audit
            // row is intentionally NOT written — there was nothing
            // to record.
            return Err(format!(
                "User vanished between lookup and reset; no rows changed (email={email})"
            ));
        }
    };

    // Step 8 — write the audit row. D12 anchor: this is the ONLY
    // place in the codebase that emits AuditEvent::EmergencyRecovery
    // for the reset-password operation; the cross-crate test in
    // `admin::audit::tests::emergency_recovery_is_cli_only` enforces
    // the framework can't.
    let correlation_id = fw_emergency::fresh_correlation_id();
    let argv: Vec<String> = std::env::args().collect();
    let metadata = serde_json::json!({
        "cli_operation": "reset_password",
        "reason": reason,
        "os_actor": ctx.os_actor,
        "cli_invocation": emergency_ui::redact_reason_in_argv(&argv),
        "revoked_session_count": revoked,
        "must_change_password_set": true,
    });
    let summary = build_summary("reset-password", &reason);
    let entry = LogEntry {
        user_id: target.id,
        action_type: ActionType::Update,
        model_name: "rustio_users",
        object_id: target.id,
        ip_address: None,
        summary,
        correlation_id: Some(&correlation_id),
        session_id: None,
        metadata: Some(metadata),
        actor_user_id: None,
        event: Some(AuditEvent::EmergencyRecovery),
    };
    record(&db, entry)
        .await
        .map_err(|e| format!("audit record: {e}"))?;

    // Step 9 — operator-facing summary + ONE-TIME password echo.
    println!();
    println!("✓ Password reset for {email} (user_id={})", target.id);
    println!("  Sessions revoked: {revoked}");
    println!("  must_change_password set; user must rotate on next login.");
    println!();
    println!("  Temporary password (shown once — record now):");
    println!();
    println!("      {temp_password}");
    println!();
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

// ---- Tests ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::build_summary;

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
