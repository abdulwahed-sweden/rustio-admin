//! Onboarding-facing error formatting.
//!
//! PR 1.3 of the FTUX redesign. Implements the four-part error shape
//! mandated by `DESIGN_ONBOARDING.md` §8:
//!
//! ```text
//! Problem:  <what failed, in plain English>
//! Why:      <the most likely cause, one sentence>
//! Fix:      <the exact command or file edit needed>
//! Retry:    <the exact command to run again>
//! ```
//!
//! The first wave of rewrites (DATABASE_URL missing, connection
//! refused, database does not exist, migration SQL failure, invalid
//! `--role`) flows through [`OnboardingError`]. Other CLI errors
//! continue to use plain `String` and are unaffected.
//!
//! Discipline:
//!
//! - The raw backend error is preserved verbatim in an optional
//!   `Details:` section so senior engineers still see what actually
//!   failed (`DESIGN_ONBOARDING.md` §8 -- "do not hide real errors").
//! - No giant abstraction. One struct, one classifier function per
//!   error path, plain `String` everywhere else.
//! - No emoji, no hype, no `Oops!` / `Something went wrong!`
//!   (`DESIGN_ONBOARDING.md` §12).

/// The four-part onboarding error shape, plus an optional `Details:`
/// block carrying the verbatim backend error.
///
/// Built once at the call site that classifies a backend failure
/// (`classify_db_error`, `classify_migration_error`, etc.), then
/// turned into a `String` via [`OnboardingError::format`] so it
/// flows through the existing `Result<(), String>` plumbing
/// unchanged. The main exit trampoline detects the four-part shape
/// by its `"Problem:"` prefix and drops the redundant `"error: "`
/// label that wraps plain-`String` errors.
#[derive(Debug)]
pub(crate) struct OnboardingError {
    pub problem: String,
    pub why: String,
    pub fix: String,
    pub retry: String,
    /// Verbatim backend error preserved for senior engineers. Multi-
    /// line strings are indented two spaces for readability.
    pub details: Option<String>,
}

impl OnboardingError {
    pub(crate) fn format(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("Problem:  ");
        out.push_str(&self.problem);
        out.push('\n');
        out.push_str("Why:      ");
        out.push_str(&self.why);
        out.push('\n');
        out.push_str("Fix:      ");
        out.push_str(&self.fix);
        out.push('\n');
        out.push_str("Retry:    ");
        out.push_str(&self.retry);
        if let Some(d) = &self.details {
            out.push_str("\n\nDetails:\n  ");
            out.push_str(&d.trim_end().replace('\n', "\n  "));
        }
        out
    }
}

impl std::fmt::Display for OnboardingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.format())
    }
}

/// Main's exit trampoline strips the `"error: "` label when the
/// error string already starts with this sentinel. Keeping it as
/// a constant ensures the trampoline and the formatter stay in
/// lock-step.
pub(crate) const ONBOARDING_SENTINEL: &str = "Problem:";

// ------------------------------
// Classifiers for the five errors PR 1.3 rewrites.
// ------------------------------

/// `DATABASE_URL` is not set in the environment (or `.env`).
pub(crate) fn database_url_missing() -> OnboardingError {
    OnboardingError {
        problem: "DATABASE_URL is not set.".into(),
        why: "The CLI reads DATABASE_URL from your environment (and from `.env` in the current directory if present); neither contained it.".into(),
        fix: "Run `rustio-admin new <name>` in an interactive terminal to have the wizard generate `.env` for you, or add a line like `DATABASE_URL=postgres://postgres:postgres@localhost:5432/<db>_dev` to `.env`.".into(),
        retry: "rustio-admin migrate apply".into(),
        details: None,
    }
}

/// Classify a `Db::connect` failure into one of three onboarding
/// errors: connection refused (service down), database missing,
/// or a generic catch-all with the raw error preserved.
///
/// The framework wraps every connect-time failure as `"db connect
/// failed: <sqlx-error>"` and goes through a connection pool, so
/// the underlying TCP error (refused, host unreachable) is usually
/// surfaced as `"pool timed out"`. We treat both of those, plus the
/// raw `"connection refused"` shape, as the same "service is not
/// reachable" case -- the fix is the same regardless.
pub(crate) fn classify_db_connect_error(url_redacted: &str, raw: &str) -> OnboardingError {
    let lower = raw.to_ascii_lowercase();
    let unreachable = lower.contains("connection refused")
        || lower.contains("could not connect to server")
        || lower.contains("pool timed out")
        || lower.contains("db connect failed");
    if unreachable && !lower.contains("does not exist") {
        return OnboardingError {
            problem: format!("Cannot connect to PostgreSQL ({url_redacted})."),
            why: "The PostgreSQL server is not running, or it is not listening on the host/port your DATABASE_URL points at.".into(),
            fix: "Start PostgreSQL. macOS:  `brew services start postgresql@16`  ·  Ubuntu:  `sudo systemctl start postgresql`  ·  Windows:  start the PostgreSQL service from the Services panel.".into(),
            retry: "rustio-admin migrate apply".into(),
            details: Some(raw.to_string()),
        };
    }
    if let Some(db_name) = parse_missing_database_name(raw) {
        return OnboardingError {
            problem: format!("Database \"{db_name}\" does not exist on the PostgreSQL server."),
            why: "PostgreSQL is running, but no database with that name has been created. Either the wizard's chosen DB_NAME has not been created yet, or `.env` and the actual database disagree.".into(),
            fix: format!("Create the database (`createdb {db_name}`) or edit `.env` so DB_NAME / DATABASE_URL match an existing database."),
            retry: "rustio-admin migrate apply".into(),
            details: Some(raw.to_string()),
        };
    }
    if lower.contains("password authentication failed") {
        return OnboardingError {
            problem: format!("PostgreSQL refused the credentials in DATABASE_URL ({url_redacted})."),
            why: "The user / password pair in DATABASE_URL does not match what the server expects.".into(),
            fix: "Check the DB_USER and DB_PASSWORD lines in `.env`, or update DATABASE_URL directly.".into(),
            retry: "rustio-admin migrate apply".into(),
            details: Some(raw.to_string()),
        };
    }
    OnboardingError {
        problem: format!("Could not connect to PostgreSQL ({url_redacted})."),
        why: "The driver returned an error that does not match the common cases (service down, missing database, bad credentials).".into(),
        fix: "Inspect the Details block below and adjust `.env` / the server accordingly.".into(),
        retry: "rustio-admin migrate apply".into(),
        details: Some(raw.to_string()),
    }
}

/// Migration SQL failed mid-apply. The library returns the failure
/// as `"migration <stem> failed: <sqlx-error>"` wrapped in
/// `Error::Internal` (so we see `"500 Internal: migration … "`)
/// and the CLI's caller adds its own `"apply: "` prefix; we pull
/// out the stem from anywhere in the string and surface the raw
/// SQL error verbatim in the Details block.
pub(crate) fn classify_migration_error(raw: &str) -> OnboardingError {
    let (stem, body) = parse_migration_failure(raw)
        .unwrap_or(("<unknown migration>".to_string(), raw.to_string()));
    OnboardingError {
        problem: format!("Migration `{stem}` failed."),
        why: "PostgreSQL rejected a statement in the migration file. The raw SQL error is preserved in the Details block below; line numbers, if present, refer to the file as PostgreSQL parsed it.".into(),
        fix: format!("Edit the migration `{stem}` in your `migrations/` directory (the file ends in `_{stem}.sql`) and correct the failing statement. Migrations are append-only -- never edit one that already ran on another database; write a new numerically prefixed migration instead."),
        retry: "rustio-admin migrate apply".into(),
        details: Some(body),
    }
}

/// `--role` (or any other clap `ValueEnum`) given an invalid value.
/// `arg` is the clap arg label (e.g. `"--role <ROLE>"`), `bad` is
/// the rejected value, `valid` is the comma-separated valid set
/// pulled from clap's `ValidValue` context.
pub(crate) fn invalid_value(arg: &str, bad: &str, valid: &[String]) -> OnboardingError {
    let valid_list = valid.join(", ");
    let is_role = arg.contains("--role") || arg.eq_ignore_ascii_case("<ROLE>");
    let example = valid.first().cloned().unwrap_or_else(|| "<value>".into());
    if is_role {
        OnboardingError {
            problem: format!("`{bad}` is not a valid role."),
            why: format!("Known roles are: {valid_list}."),
            fix: format!("Re-run with one of those values, e.g. `--role {example}`."),
            retry: format!("rustio-admin user create --email <email> --role {example}"),
            details: None,
        }
    } else {
        OnboardingError {
            problem: format!("`{bad}` is not a valid value for `{arg}`."),
            why: format!("Accepted values are: {valid_list}."),
            fix: format!("Re-run with one of those values, e.g. `{arg}={example}`."),
            retry: "(re-run the same command with a valid value)".into(),
            details: None,
        }
    }
}

// ------------------------------
// Helpers.
// ------------------------------

/// Detect a "database does not exist" message in a sqlx-flavoured
/// error string and extract the bare name. PostgreSQL phrases it as
/// `database "X" does not exist`; sqlx surfaces it verbatim inside
/// `error returned from database: …`.
fn parse_missing_database_name(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    if !lower.contains("does not exist") {
        return None;
    }
    let idx = lower.find("database \"")?;
    let after = &raw[idx + "database \"".len()..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

/// Match the library's `"migration <stem> failed: <body>"` shape
/// anywhere in the raw error string and split it into
/// `(stem, body)`. The library wraps inside `Error::Internal` (so
/// `"500 Internal: …"` is prepended) and the CLI adds its own
/// `"apply: "` prefix, so we use substring search rather than
/// a strict prefix match. A trailing sanity check rejects matches
/// where the stem contains a space, which guards against
/// accidental substring hits like `"migration history"`.
fn parse_migration_failure(raw: &str) -> Option<(String, String)> {
    let idx = raw.find("migration ")?;
    let after = &raw[idx + "migration ".len()..];
    let (stem, body) = after.split_once(" failed: ")?;
    if stem.is_empty() || stem.contains(' ') {
        return None;
    }
    Some((stem.to_string(), body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_carries_four_parts_in_order() {
        let e = OnboardingError {
            problem: "X failed.".into(),
            why: "Y.".into(),
            fix: "Z.".into(),
            retry: "W.".into(),
            details: None,
        };
        let s = e.format();
        assert!(s.starts_with("Problem:"));
        let lines: Vec<&str> = s.lines().collect();
        assert!(lines[0].starts_with("Problem:"));
        assert!(lines[1].starts_with("Why:"));
        assert!(lines[2].starts_with("Fix:"));
        assert!(lines[3].starts_with("Retry:"));
    }

    #[test]
    fn details_block_indents_and_separates() {
        let e = OnboardingError {
            problem: "P".into(),
            why: "W".into(),
            fix: "F".into(),
            retry: "R".into(),
            details: Some("line one\nline two".into()),
        };
        let s = e.format();
        assert!(s.contains("\n\nDetails:\n  line one\n  line two"));
    }

    #[test]
    fn details_omitted_when_none() {
        let e = OnboardingError {
            problem: "P".into(),
            why: "W".into(),
            fix: "F".into(),
            retry: "R".into(),
            details: None,
        };
        assert!(!e.format().contains("Details:"));
    }

    #[test]
    fn classify_connect_recognises_refused() {
        let raw = "error connecting to server: Connection refused (os error 61)".to_string();
        let e = classify_db_connect_error("postgres://postgres:***@localhost:5432/foo", &raw);
        assert!(e.problem.contains("Cannot connect"));
        assert!(e.fix.contains("brew services start"));
        assert_eq!(e.details.as_deref(), Some(raw.as_str()));
    }

    #[test]
    fn classify_connect_recognises_pool_timeout_as_service_unreachable() {
        // The framework's `Db::connect` goes through a sqlx pool, so
        // a refused TCP connect surfaces as the pool timeout wrap
        // verified at the surface against a port nothing is listening on.
        let raw =
            "500 Internal: db connect failed: pool timed out while waiting for an open connection"
                .to_string();
        let e = classify_db_connect_error("postgres://postgres:***@127.0.0.1:5499/foo", &raw);
        assert!(
            e.problem.contains("Cannot connect"),
            "got problem: {}",
            e.problem
        );
        assert!(e.fix.contains("brew services start"));
        assert_eq!(e.details.as_deref(), Some(raw.as_str()));
    }

    #[test]
    fn classify_connect_db_missing_wins_over_pool_timeout_phrasing() {
        // If both "does not exist" and "pool timed out" appear in the
        // chained error, the db-missing classification is the more
        // specific signal and should win -- the fix is different.
        let raw = "pool timed out … database \"clinic_dev\" does not exist".to_string();
        let e = classify_db_connect_error("postgres://x:***@h/d", &raw);
        assert!(e.problem.contains("\"clinic_dev\""), "got: {}", e.problem);
    }

    #[test]
    fn classify_connect_recognises_missing_database() {
        let raw =
            "error returned from database: database \"clinic_dev\" does not exist".to_string();
        let e =
            classify_db_connect_error("postgres://postgres:***@localhost:5432/clinic_dev", &raw);
        assert!(e.problem.contains("\"clinic_dev\""));
        assert!(e.fix.contains("createdb clinic_dev"));
    }

    #[test]
    fn classify_connect_recognises_bad_credentials() {
        let raw = "FATAL: password authentication failed for user \"postgres\"".to_string();
        let e = classify_db_connect_error("postgres://postgres:***@localhost:5432/foo", &raw);
        assert!(e.problem.contains("refused the credentials"));
    }

    #[test]
    fn classify_connect_falls_through_to_generic() {
        let raw = "some unknown driver error".to_string();
        let e = classify_db_connect_error("postgres://x:***@h/d", &raw);
        assert!(e.problem.contains("Could not connect"));
        assert_eq!(e.details.as_deref(), Some(raw.as_str()));
    }

    #[test]
    fn classify_migration_extracts_stem_and_body() {
        let raw =
            "apply: migration create_comments failed: ERROR: syntax error at or near \"FROMM\"";
        let e = classify_migration_error(raw);
        assert!(e.problem.contains("`create_comments`"));
        assert!(e.fix.contains("_create_comments.sql"));
        assert!(e.details.as_deref().unwrap().contains("syntax error"));
    }

    #[test]
    fn classify_migration_handles_500_internal_wrap_layer() {
        // The framework wraps with Error::Internal, which Display-formats
        // as "500 Internal: ..."; the CLI then adds "apply: ". Verified
        // against a real Postgres with a deliberately broken migration.
        let raw = "apply: 500 Internal: migration create_broken failed: error returned from database: syntax error at or near \"TABEL\"";
        let e = classify_migration_error(raw);
        assert!(
            e.problem.contains("`create_broken`"),
            "got problem: {}",
            e.problem
        );
        assert!(e.fix.contains("_create_broken.sql"));
    }

    #[test]
    fn classify_migration_falls_through_when_shape_unknown() {
        let raw = "apply: something completely different";
        let e = classify_migration_error(raw);
        assert!(e.problem.contains("`<unknown migration>`"));
        assert_eq!(e.details.as_deref(), Some(raw));
    }

    #[test]
    fn parse_migration_rejects_multi_word_stem() {
        // The "no whitespace" guard catches the case where the runner's
        // wrap puts a phrase rather than an identifier in the stem slot.
        assert!(parse_migration_failure("migration two words failed: body").is_none());
    }

    #[test]
    fn invalid_value_for_role_includes_known_list() {
        let valid: Vec<String> = ["user", "staff", "administrator"]
            .into_iter()
            .map(String::from)
            .collect();
        let e = invalid_value("--role <ROLE>", "admin", &valid);
        assert!(e.problem.contains("`admin`"));
        assert!(e.why.contains("user, staff, administrator"));
        assert!(e.retry.contains("--role user"));
    }

    #[test]
    fn invalid_value_for_non_role_uses_generic_phrasing() {
        let valid: Vec<String> = ["minimal", "blog"].into_iter().map(String::from).collect();
        let e = invalid_value("--preset <PRESET>", "fancy", &valid);
        assert!(e.problem.contains("`fancy`"));
        assert!(e.why.contains("minimal, blog"));
        assert!(!e.problem.contains("role"));
    }

    #[test]
    fn parse_missing_database_handles_typical_shape() {
        let raw = "blah blah database \"acme_dev\" does not exist blah";
        assert_eq!(
            parse_missing_database_name(raw).as_deref(),
            Some("acme_dev")
        );
    }

    #[test]
    fn parse_missing_database_returns_none_when_no_match() {
        assert!(parse_missing_database_name("nothing of interest").is_none());
        // "does not exist" without the literal `database "X"` shape
        // should not match.
        assert!(parse_missing_database_name("table foo does not exist").is_none());
    }

    #[test]
    fn sentinel_matches_format_prefix() {
        let e = OnboardingError {
            problem: "X".into(),
            why: "Y".into(),
            fix: "Z".into(),
            retry: "W".into(),
            details: None,
        };
        assert!(e.format().starts_with(ONBOARDING_SENTINEL));
    }
}
