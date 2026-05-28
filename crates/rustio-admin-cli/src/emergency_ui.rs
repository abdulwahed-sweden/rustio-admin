//! Banner + confirmation surface shared by every `rustio-admin user <op>`
//! emergency command.
//!
//! This module owns the CLI's UX concerns for emergency operations:
//! reason validation, the red ANSI confirmation banner, TTY
//! detection, and the interactive `yes` / `--yes` prompt path. The
//! actual DB mutations live in `rustio_admin::auth::emergency`.
//!
//! ## Doctrinal anchors
//!
//! - **D10 -- confirmation banner is irreducible.** Every emergency
//!   command renders the banner. `--yes` skips the interactive
//!   prompt but NEVER the banner -- the banner is the visible
//!   artefact for over-the-shoulder review and for CI / scrollback
//!   forensics.
//! - **D9 -- CLI-actor identity is OS-level.** `os_actor()` reads
//!   `$USER` (or `$USERNAME` on Windows-style envs) and shells
//!   `hostname`. No synthetic-user invention; the audit row's
//!   `metadata.os_actor` is literally what the operator's shell
//!   reports.
//!
//! Reason validation per `DESIGN_R4_EMERGENCY.md` §3: trimmed
//! length must be ≥ 8 chars. Empty / whitespace-only / too-short
//! inputs surface a distinct error message and the operator-side
//! command exits with status 2 (input error).

use std::io::{IsTerminal, Write};

use chrono::{DateTime, Utc};

/// Minimum trimmed length for `--reason`. Per
/// `DESIGN_R4_EMERGENCY.md` §3 every R4 command's reason is the
/// regulatory artefact answering "why was an emergency tier used?"
/// A handful of characters is rejected by the floor.
pub const REASON_MIN_LEN: usize = 8;

// ---- Locked-properties type -------------------

/// Everything the banner needs about the operation in flight.
/// Constructed by the per-command CLI handler BEFORE rendering so
/// operator typos on `--email` surface at lookup, not inside the
/// mutation.
#[derive(Debug, Clone)]
pub struct OperationContext {
    /// User-facing operation slug, e.g. `"reset-password"`.
    /// Matches the `rustio-admin user <op>` subcommand name verbatim.
    pub operation: &'static str,
    /// The target's email, loaded fresh from
    /// `find_user_by_email` immediately before the banner renders.
    pub target_email: String,
    /// Target's `rustio_users.id`.
    pub target_user_id: i64,
    /// Target's current role, as a display string (e.g.
    /// `"administrator"`).
    pub target_role: String,
    /// The validated `--reason` text. Already passed
    /// [`validate_reason`].
    pub reason: String,
    /// OS-actor identity (`<whoami>@<hostname>`). Stamped by
    /// [`os_actor`].
    pub os_actor: String,
    /// Render-time UTC timestamp. Stamped by [`now_iso`].
    pub when: DateTime<Utc>,
}

// ---- Reason validation ---------------------

/// Validate the `--reason` flag value. Returns the trimmed reason
/// on success, or a one-line error message on rejection.
///
/// Rules per `DESIGN_R4_EMERGENCY.md` §3:
/// - Required (caller routes the missing-flag case to clap).
/// - Trimmed length ≥ [`REASON_MIN_LEN`] (8 chars).
/// - All-whitespace rejected (trims to empty).
pub fn validate_reason(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "--reason must not be empty (and must be at least {REASON_MIN_LEN} characters)"
        ));
    }
    if trimmed.chars().count() < REASON_MIN_LEN {
        return Err(format!(
            "--reason is too short: {} characters, minimum {REASON_MIN_LEN}",
            trimmed.chars().count()
        ));
    }
    Ok(trimmed.to_string())
}

// ---- OS-level actor + time --------------------

/// `<whoami>@<hostname>`, as the audit row will record it.
/// Reads `$USER` (or `$USERNAME`); falls back to `"unknown"`.
/// Shells out to `hostname` (universal on Unix); falls back to
/// `"unknown"`.
pub fn os_actor() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let host = hostname_via_command().unwrap_or_else(|| "unknown".to_string());
    format!("{user}@{host}")
}

fn hostname_via_command() -> Option<String> {
    let output = std::process::Command::new("hostname").output().ok()?;
    let s = String::from_utf8(output.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Convenience: render-time `Utc::now()`. Wrapped so callers can
/// stamp the banner's timestamp deterministically in tests.
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

// ---- Banner rendering ----------------------

/// Render the banner string. `with_ansi = true` wraps the header
/// line in red ANSI escapes; `false` produces plain text for tests
/// and for `NO_COLOR`-aware terminals.
///
/// Lock points from `DESIGN_R4_EMERGENCY.md` §4:
/// - Operation / Target / Reason / Operator / Time labels appear in
///   that exact order.
/// - The header line is `⚠  EMERGENCY OPERATION -- RUSTIO ADMIN CLI`.
/// - The closing line is `This action is audited and irreversible.`
/// - Box width is 64 chars.
pub fn format_banner(ctx: &OperationContext, with_ansi: bool) -> String {
    let (red_on, red_off) = if with_ansi {
        ("\x1b[31m", "\x1b[0m")
    } else {
        ("", "")
    };

    let target_line = format!(
        "{} (user_id={}, role={})",
        ctx.target_email, ctx.target_user_id, ctx.target_role
    );
    let when_iso = ctx.when.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let mut out = String::new();
    out.push_str(&format!(
        "{red_on}┌──────────────────────────────────────────────────────────────┐{red_off}\n"
    ));
    out.push_str(&format!(
        "{red_on}│  ⚠  EMERGENCY OPERATION -- RUSTIO ADMIN CLI                  │{red_off}\n"
    ));
    out.push_str(&format!(
        "{red_on}├──────────────────────────────────────────────────────────────┤{red_off}\n"
    ));
    out.push_str(&body_row("Operation:", ctx.operation));
    out.push_str(&body_row("Target:", &target_line));
    out.push_str(&body_row("Reason:", &ctx.reason));
    out.push_str(&body_row("Operator:", &ctx.os_actor));
    out.push_str(&body_row("Time:", &when_iso));
    out.push_str(&format!(
        "{red_on}├──────────────────────────────────────────────────────────────┤{red_off}\n"
    ));
    out.push_str(&body_row("", "This action is audited and irreversible."));
    out.push_str(&format!(
        "{red_on}└──────────────────────────────────────────────────────────────┘{red_off}\n"
    ));
    out
}

/// Print the banner to stdout. Colour is automatic: enabled when
/// stdout is a TTY and `NO_COLOR` is not set.
pub fn print_banner(ctx: &OperationContext) {
    let with_ansi = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    print!("{}", format_banner(ctx, with_ansi));
    let _ = std::io::stdout().flush();
}

/// Format one body row of the banner. Pads labels to 11 chars so
/// the value column lines up across rows.
///
/// The 64-char box is tight; values longer than ~46 chars wrap by
/// extending the box on that row (no truncation). The longer
/// banner is uglier but truthful -- truncating a reason field
/// would defeat the forensic point.
fn body_row(label: &str, value: &str) -> String {
    let inside = if label.is_empty() {
        format!("  {value}")
    } else {
        format!("  {label:<11}{value}")
    };
    // Pad to 60 inside chars (62 outside the `│ │` walls) when
    // possible, otherwise extend the right wall outward.
    let pad_target = 60_usize;
    let inside_width = inside.chars().count();
    if inside_width <= pad_target {
        let pad = " ".repeat(pad_target - inside_width);
        format!("│{inside}{pad}│\n")
    } else {
        format!("│{inside}│\n")
    }
}

// ---- Confirmation prompt ----------------------

/// Outcome of [`require_confirm`]. Distinct variants so the CLI
/// handler can map each to a stable exit code (Aborted=1,
/// NeedsTty=2, Confirmed=0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// Operator confirmed via typing `yes` (or `--yes` was set and
    /// the banner-only flow ran). Proceed with the mutation.
    Confirmed,
    /// Operator typed something other than `yes`. Caller should
    /// print a "Aborted." line and exit non-zero.
    Aborted,
    /// stdin is not a TTY and `--yes` was not set. Caller should
    /// print the no-TTY message and exit non-zero.
    NeedsTtyOrYesFlag,
}

/// Demand confirmation. With `yes = true`, skips the interactive
/// prompt -- but the BANNER must already have been rendered by
/// the caller (D10). With `yes = false`, prints the "Type 'yes' to
/// confirm" prompt to stdout, reads one line from stdin, and
/// matches it against the literal `"yes"` (case-sensitive, trimmed).
///
/// If stdin is not a TTY and `yes = false`, refuses to proceed
/// rather than blocking on `read_line`.
pub fn require_confirm(yes: bool) -> ConfirmOutcome {
    if yes {
        return ConfirmOutcome::Confirmed;
    }
    let is_tty = std::io::stdin().is_terminal();
    if !is_tty {
        return ConfirmOutcome::NeedsTtyOrYesFlag;
    }
    print!("Type 'yes' to confirm, anything else to abort: ");
    let _ = std::io::stdout().flush();
    let mut buf = String::new();
    if std::io::stdin().read_line(&mut buf).is_err() {
        return ConfirmOutcome::Aborted;
    }
    classify_response(&buf)
}

/// Pure-function half of [`require_confirm`] -- what to return for
/// a given user-typed line. Split out so tests can exercise the
/// classification without touching stdin.
pub fn classify_response(line: &str) -> ConfirmOutcome {
    if line.trim() == "yes" {
        ConfirmOutcome::Confirmed
    } else {
        ConfirmOutcome::Aborted
    }
}

// ---- argv redaction ------------------------

/// Redact the `--reason` value from `argv`-style input. The
/// reason text lives in `metadata.reason` of the audit row as a
/// dedicated typed field; echoing it again inside
/// `metadata.cli_invocation` would double-store the same string
/// and risks leaking through logging surfaces that summarise the
/// argv shape. The dedicated metadata field is the authoritative
/// copy.
///
/// Handles both space-separated (`--reason TEXT`) and
/// equals-form (`--reason=TEXT`). Unrecognised flags pass
/// through verbatim.
pub fn redact_reason_in_argv(args: &[String]) -> String {
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--reason" && i + 1 < args.len() {
            out.push("--reason".to_string());
            out.push("<redacted>".to_string());
            i += 2;
        } else if a.starts_with("--reason=") {
            out.push("--reason=<redacted>".to_string());
            i += 1;
        } else {
            out.push(a.clone());
            i += 1;
        }
    }
    out.join(" ")
}

// ---- Tests ---------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_ctx() -> OperationContext {
        OperationContext {
            operation: "reset-password",
            target_email: "alice@example.com".to_string(),
            target_user_id: 42,
            target_role: "administrator".to_string(),
            reason: "Lost MFA device + locked out, no other admins".to_string(),
            os_actor: "mansour@studio.local".to_string(),
            when: Utc.with_ymd_and_hms(2026, 5, 11, 16, 42, 0).unwrap(),
        }
    }

    // ---- validate_reason ----

    #[test]
    fn reason_rejects_empty() {
        let e = validate_reason("").unwrap_err();
        assert!(e.contains("must not be empty"), "got {e}");
    }

    #[test]
    fn reason_rejects_all_whitespace() {
        let e = validate_reason("     \t\n").unwrap_err();
        assert!(e.contains("must not be empty"), "got {e}");
    }

    #[test]
    fn reason_rejects_too_short() {
        let e = validate_reason("hello").unwrap_err();
        assert!(e.contains("too short"), "got {e}");
        assert!(e.contains("5 characters"), "got {e}");
    }

    #[test]
    fn reason_rejects_seven_chars() {
        // boundary: REASON_MIN_LEN = 8
        let e = validate_reason("seven_x").unwrap_err();
        assert!(e.contains("too short"), "got {e}");
    }

    #[test]
    fn reason_accepts_eight_chars_exactly() {
        let r = validate_reason("eightlen").unwrap();
        assert_eq!(r, "eightlen");
    }

    #[test]
    fn reason_trims_surrounding_whitespace() {
        let r = validate_reason("   hello world   ").unwrap();
        assert_eq!(r, "hello world");
    }

    // ---- format_banner ----

    #[test]
    fn banner_plain_renders_locked_layout() {
        let ctx = fixed_ctx();
        let out = format_banner(&ctx, false);
        // Header marker
        assert!(out.contains("EMERGENCY OPERATION -- RUSTIO ADMIN CLI"));
        // Every locked label appears
        assert!(out.contains("Operation:"));
        assert!(out.contains("Target:"));
        assert!(out.contains("Reason:"));
        assert!(out.contains("Operator:"));
        assert!(out.contains("Time:"));
        // Values echoed verbatim
        assert!(out.contains("reset-password"));
        assert!(out.contains("alice@example.com"));
        assert!(out.contains("user_id=42"));
        assert!(out.contains("role=administrator"));
        assert!(out.contains("Lost MFA device + locked out, no other admins"));
        assert!(out.contains("mansour@studio.local"));
        assert!(out.contains("2026-05-11T16:42:00Z"));
        // Closing line
        assert!(out.contains("This action is audited and irreversible."));
        // No ANSI escapes in plain mode
        assert!(
            !out.contains("\x1b["),
            "plain banner must not contain ANSI escapes"
        );
    }

    #[test]
    fn banner_ansi_wraps_only_box_chars() {
        let ctx = fixed_ctx();
        let out = format_banner(&ctx, true);
        assert!(out.contains("\x1b[31m"), "missing red ANSI prefix");
        assert!(out.contains("\x1b[0m"), "missing ANSI reset");
        // Strip ANSI and confirm the body content is identical to
        // the plain rendering.
        let stripped = strip_ansi(&out);
        let plain = format_banner(&ctx, false);
        assert_eq!(stripped, plain);
    }

    #[test]
    fn banner_long_reason_does_not_panic_or_truncate() {
        let mut ctx = fixed_ctx();
        ctx.reason = "x".repeat(200);
        let out = format_banner(&ctx, false);
        assert!(
            out.contains(&"x".repeat(200)),
            "reason was truncated; banner must echo verbatim"
        );
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                // Consume `[`, then digits / `;`, until a final
                // letter (m for SGR).
                chars.next();
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            out.push(c);
        }
        out
    }

    // ---- classify_response (the pure half of require_confirm) ----

    #[test]
    fn confirm_accepts_yes_exactly() {
        assert_eq!(classify_response("yes"), ConfirmOutcome::Confirmed);
        assert_eq!(classify_response("yes\n"), ConfirmOutcome::Confirmed);
        assert_eq!(classify_response("  yes  \n"), ConfirmOutcome::Confirmed);
    }

    #[test]
    fn confirm_rejects_other_responses() {
        for s in ["y", "Y", "YES", "yeah", "no", "n", "", "\n", "yes please"] {
            assert_eq!(
                classify_response(s),
                ConfirmOutcome::Aborted,
                "{s:?} should not confirm"
            );
        }
    }

    // ---- require_confirm with --yes ----

    #[test]
    fn yes_flag_returns_confirmed_without_reading_stdin() {
        // No prompt drawn; no stdin read; banner is the caller's
        // responsibility per D10.
        assert_eq!(require_confirm(true), ConfirmOutcome::Confirmed);
    }

    // ---- redact_reason_in_argv ----

    #[test]
    fn redact_space_form() {
        let argv: Vec<String> = [
            "rustio-admin",
            "user",
            "reset-password",
            "--email",
            "alice@example.com",
            "--reason",
            "lost MFA device, no other admins",
            "--yes",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = redact_reason_in_argv(&argv);
        assert!(out.contains("--reason <redacted>"));
        assert!(!out.contains("lost MFA"));
        // Other args preserved
        assert!(out.contains("--email alice@example.com"));
        assert!(out.contains("--yes"));
    }

    #[test]
    fn redact_equals_form() {
        let argv: Vec<String> = [
            "rustio-admin",
            "user",
            "reset-password",
            "--reason=lost MFA device",
            "--email=alice@example.com",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = redact_reason_in_argv(&argv);
        assert!(out.contains("--reason=<redacted>"));
        assert!(!out.contains("lost MFA"));
        assert!(out.contains("--email=alice@example.com"));
    }

    #[test]
    fn redact_no_reason_passes_through() {
        let argv: Vec<String> = ["rustio-admin", "user", "list"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let out = redact_reason_in_argv(&argv);
        assert_eq!(out, "rustio-admin user list");
    }

    #[test]
    fn redact_lone_reason_flag_at_end_passes_through() {
        // Pathological: --reason at end of argv with no value.
        // Clap would have rejected this; the redactor must not panic
        // or eat a value that doesn't exist.
        let argv: Vec<String> = ["rustio-admin", "--reason"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let out = redact_reason_in_argv(&argv);
        assert_eq!(out, "rustio-admin --reason");
    }
}
