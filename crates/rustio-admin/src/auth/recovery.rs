//! Self-service password recovery (R1).
//!
//! See `DESIGN_RECOVERY.md` for the canonical contract this module
//! implements. R1 ships in 0.5.0; this commit lands the schema, the
//! [`PasswordPolicy`] surface, and the [`RecoveryPolicy`] surface.
//! The issue + consume flow, the mailer wiring, the routes, and the
//! templates land in subsequent atomic commits per
//! `DESIGN_RECOVERY.md` §16.
//!
//! ## What lives here today
//!
//! - [`init_recovery_tables`] — creates `rustio_password_reset_tokens`
//!   with the partial unique index that makes the consume path's
//!   atomic `UPDATE … RETURNING` an index seek
//!   (`DESIGN_RECOVERY.md` §9.1).
//! - [`migrate_user_recovery_schema`] — adds the additive
//!   `must_change_password` and `password_changed_at` columns on
//!   `rustio_users` (§9.2). R1's `set_password` populates
//!   `password_changed_at`; R2 enforces `must_change_password`.
//! - [`PasswordPolicy`] / [`DefaultPasswordPolicy`] /
//!   [`PasswordPolicyError`] / [`SharedPasswordPolicy`] — the
//!   password-policy surface (§13).
//! - [`RecoveryPolicy`] / [`DefaultRecoveryPolicy`] /
//!   [`SharedRecoveryPolicy`] — the recovery-flow tunables (§10.2,
//!   §12.3). Reset-token TTL, rate-limit shape, strict-mailer boot
//!   guard, and the public-site-URL derivation rule.
//!
//! `Admin::password_policy(...)` and `Admin::recovery_policy(...)`
//! live in `admin::types`; the traits and default impls live here so
//! the recovery module owns its vocabulary.
//!
//! The migration functions are idempotent and safe to call on every
//! boot. `auth::init_tables` invokes them after the existing user /
//! session migrations. The policy surface is data-only at this
//! commit; no handler reads either policy yet.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::Duration as ChronoDuration;

use crate::admin::audit::{record as audit_record, ActionType, AuditEvent, LogEntry};
use crate::admin::redact::redact_token;
use crate::admin::Admin;
use crate::auth::sessions::{hash_token_for_storage, random_token};
use crate::auth::users::find_user_by_email;
use crate::auth::{invalidate_sessions, set_password, SessionInvalidationReason, SessionTarget};
use crate::email::Mail;
use crate::error::Result;
use crate::http::Request;
use crate::middleware::RateLimiter;
use crate::orm::Db;

/// Create the `rustio_password_reset_tokens` table and its indexes.
///
/// Schema (see `DESIGN_RECOVERY.md` §9.1 for the contract):
///
/// - `token_hash` is `sha256(token)` URL-safe-base64 — the plaintext
///   token never lands in this row.
/// - `mail_status` is one of `'pending' | 'sent' | 'failed'`; the state
///   evolves in the issue handler (one row per request).
/// - `correlation_id` mirrors the request's audit `correlation_id` so
///   an operator can pivot from token row → audit chain.
/// - The partial unique index `WHERE consumed_at IS NULL` is the index
///   the atomic consume statement seeks on.
///
/// Idempotent. Safe to call on every boot. Depends on `rustio_users`
/// existing first.
pub(crate) async fn init_recovery_tables(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_password_reset_tokens (
            id                    BIGSERIAL   PRIMARY KEY,
            user_id               BIGINT      NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
            token_hash            TEXT        NOT NULL,
            requested_ip          TEXT,
            requested_user_agent  TEXT,
            requested_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            expires_at            TIMESTAMPTZ NOT NULL,
            consumed_at           TIMESTAMPTZ,
            mail_status           TEXT        NOT NULL DEFAULT 'pending'
                                  CHECK (mail_status IN ('pending', 'sent', 'failed')),
            correlation_id        TEXT
        )",
    )
    .execute(db.pool())
    .await?;

    // Partial unique on the active-token lookup. Guarantees the
    // consume statement (`UPDATE … WHERE token_hash = $1 AND
    // consumed_at IS NULL RETURNING …`) is an index seek even after
    // the table accumulates consumed/expired rows for forensic
    // retention.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS rustio_password_reset_tokens_active_uq \
         ON rustio_password_reset_tokens (token_hash) \
         WHERE consumed_at IS NULL",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_user_idx \
         ON rustio_password_reset_tokens (user_id)",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_expires_idx \
         ON rustio_password_reset_tokens (expires_at) \
         WHERE consumed_at IS NULL",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

/// Add the additive recovery columns on `rustio_users`.
///
/// - `must_change_password BOOLEAN NOT NULL DEFAULT FALSE` — R2 will
///   read this on login to force a password reset on the next sign-in.
///   R1 introduces the column because R2's commit set stays narrower
///   when the column already exists.
/// - `password_changed_at TIMESTAMPTZ` (nullable) — populated by
///   `auth::set_password` from R1 onwards. NULL for users created
///   before the upgrade; the active-sessions UI renders "(unknown)" or
///   omits the row when NULL.
///
/// Idempotent. Safe to call on every boot. Depends on `rustio_users`
/// existing first.
pub(crate) async fn migrate_user_recovery_schema(db: &Db) -> Result<()> {
    sqlx::query(
        "ALTER TABLE rustio_users \
         ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS password_changed_at TIMESTAMPTZ")
        .execute(db.pool())
        .await?;

    Ok(())
}

// ---- Password policy -------------------------------------------------------

/// Validates a candidate password against project-defined rules.
///
/// The framework ships [`DefaultPasswordPolicy`] (length-only floor)
/// as the secure-by-default baseline. Projects layer a stronger
/// policy via [`crate::admin::Admin::password_policy`] when
/// regulation or risk requires it. The trait is `Send + Sync` so the
/// `Arc<dyn PasswordPolicy>` lives on `Admin` and is cheap to clone
/// into async futures.
///
/// ## Implementing a custom policy
///
/// ```ignore
/// use rustio_admin::auth::{PasswordPolicy, PasswordPolicyError};
///
/// struct OrgPolicy;
/// impl PasswordPolicy for OrgPolicy {
///     fn validate(&self, candidate: &str) -> Result<(), PasswordPolicyError> {
///         let len = candidate.chars().count();
///         if len < 16 {
///             return Err(PasswordPolicyError::TooShort { min: 16, actual: len });
///         }
///         if !candidate.chars().any(|c| c.is_ascii_digit()) {
///             return Err(PasswordPolicyError::Custom(
///                 "Password must contain at least one digit.".into(),
///             ));
///         }
///         Ok(())
///     }
///     fn min_length(&self) -> usize { 16 }
/// }
/// ```
///
/// Implementations MUST treat the borrowed candidate as a secret:
/// no logging, no panic-with-the-plaintext, no inclusion in the
/// returned error. The framework's audit + log helpers redact
/// passwords (`audit::redact_password()`); custom policies that
/// want to surface a project-specific message use
/// [`PasswordPolicyError::Custom`] with a user-safe string.
pub trait PasswordPolicy: Send + Sync {
    /// Approve or reject the candidate.
    fn validate(&self, candidate: &str) -> std::result::Result<(), PasswordPolicyError>;

    /// The minimum length the policy enforces, in Unicode `char`s.
    /// Templates display this on the new-password form so users see
    /// the floor before submitting.
    fn min_length(&self) -> usize;
}

/// Type-erased shared password-policy reference, mirroring
/// [`crate::email::SharedMailer`]. The framework's `Admin` holds one
/// of these; defaults to `Arc::new(DefaultPasswordPolicy::new())`
/// until a project overrides via
/// `Admin::password_policy(Arc::new(...))`.
pub type SharedPasswordPolicy = Arc<dyn PasswordPolicy>;

/// Reasons a candidate password fails policy validation.
///
/// Variants intentionally omit the candidate plaintext — none of the
/// fields carry the rejected password, so a `Display` / `Debug`
/// rendering of any error value is safe to log, audit, or pass to a
/// form-field renderer. Project-supplied policies that emit
/// [`PasswordPolicyError::Custom`] are responsible for keeping their
/// message free of the plaintext as well.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PasswordPolicyError {
    /// Length floor not met. Both fields are character counts (not
    /// bytes), matching `min_length()`.
    TooShort { min: usize, actual: usize },
    /// Project-defined rejection. The string renders to the user
    /// verbatim and lands in logs verbatim — keep it free of secrets.
    Custom(String),
}

impl std::fmt::Display for PasswordPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { min, actual } => write!(
                f,
                "This password is too short. It must contain at least {min} characters \
                 (you entered {actual})."
            ),
            Self::Custom(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for PasswordPolicyError {}

/// Length-only password policy. Default `min_len` is **10** — the
/// secure-by-default baseline R1 ships with: long enough to defeat
/// trivial guessing under Argon2id + per-IP rate-limiting (NIST SP
/// 800-63B's recommended length floor is 8, with longer being
/// preferable), short enough not to drive operators toward sticky-
/// note workarounds. Production / regulated deployments are
/// encouraged to override to 12+ via
/// [`crate::admin::Admin::password_policy`]; high-sensitivity
/// deployments may want 16+ paired with an organisational
/// complexity rule or breach blocklist.
///
/// The framework deliberately ships **no complexity-class rules**
/// ("must contain a symbol", "must include uppercase") in the
/// default — they demonstrably push humans toward predictable
/// patterns without improving entropy meaningfully (NIST SP
/// 800-63B Appendix A). Projects that need them implement a
/// custom `PasswordPolicy`.
#[derive(Debug, Clone, Copy)]
pub struct DefaultPasswordPolicy {
    pub min_len: usize,
}

impl DefaultPasswordPolicy {
    /// New policy with the framework's default floor (`min_len = 10`).
    pub const fn new() -> Self {
        Self { min_len: 10 }
    }

    /// New policy with an explicit floor. Useful for projects that
    /// want a stronger length baseline without authoring a full
    /// `PasswordPolicy` impl.
    pub const fn with_min_len(min_len: usize) -> Self {
        Self { min_len }
    }
}

impl Default for DefaultPasswordPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordPolicy for DefaultPasswordPolicy {
    fn validate(&self, candidate: &str) -> std::result::Result<(), PasswordPolicyError> {
        // Count Unicode `char`s, not bytes — a 10-char password is
        // 10 user-visible characters regardless of UTF-8 byte width.
        // Grapheme-cluster counting is left to project policies that
        // need it.
        let actual = candidate.chars().count();
        if actual < self.min_len {
            return Err(PasswordPolicyError::TooShort {
                min: self.min_len,
                actual,
            });
        }
        Ok(())
    }

    fn min_length(&self) -> usize {
        self.min_len
    }
}

// ---- Recovery policy -------------------------------------------------------

/// Tunables for the R1 recovery flow: token TTL, rate-limit shape,
/// strict-mailer boot guard, and public-site-URL derivation.
///
/// `Admin::new()` seeds [`DefaultRecoveryPolicy`]; projects override
/// via [`crate::admin::Admin::recovery_policy`]. The trait is `Send +
/// Sync` so the `Arc<dyn RecoveryPolicy>` lives on `Admin` and is
/// cheap to clone into async futures.
///
/// The trait method `public_site_url` has a provided default that
/// derives the URL from request headers via [`derive_public_site_url`]
/// per `DESIGN_RECOVERY.md` §12.3. Projects whose deployment can't
/// rely on the standard Forwarded / X-Forwarded-* / Host headers
/// override this method and return their own absolute URL (e.g.
/// stamped at deployment time from a config secret).
///
/// ## Trust boundary for forwarded headers
///
/// The default `public_site_url` honours these client-supplied
/// inputs in priority order:
///
/// 1. RFC 7239 `Forwarded` header (`for / proto / host` of the first
///    hop)
/// 2. `X-Forwarded-Proto` + `X-Forwarded-Host` (first CSV entry of
///    each)
/// 3. `Host` header (assumes `http://`)
///
/// **The operator's reverse proxy MUST strip incoming versions of
/// these headers before adding its own.** The framework cannot know
/// the deployment topology; if a hostile client can reach the
/// process directly with a chosen `Forwarded: …` header set, the
/// reset link in the dispatched email will point wherever they ask.
/// `proto` is whitelisted to `{http, https}` (case-insensitive) and
/// `host` is rejected when it contains whitespace, control bytes, or
/// CRLF — so direct injection of `\r\n`-style header smuggling
/// fails — but a malicious yet shape-conformant value still needs
/// to be filtered upstream.
///
/// Projects that need a stricter trust posture: override
/// `public_site_url` to return a fixed string (e.g. read from
/// project config at startup) and the framework will use that
/// regardless of headers.
pub trait RecoveryPolicy: Send + Sync {
    /// How long a freshly-issued reset token stays valid. Default
    /// 1 hour. Locked-decision per `DESIGN_RECOVERY.md` §17.
    fn reset_token_ttl(&self) -> ChronoDuration;

    /// Per-IP rate-limit on `POST /admin/forgot-password`. Returned
    /// as `(capacity, window)`: at most `capacity` requests within
    /// `window`. Default `(5, 15min)`.
    fn request_rate_limit(&self) -> (u32, StdDuration);

    /// Per-IP rate-limit on `POST /admin/reset-password/<token>`.
    /// Tighter than the request limit since the consume path is the
    /// brute-force surface. Default `(10, 5min)`.
    fn consume_rate_limit(&self) -> (u32, StdDuration);

    /// When `true`, the framework refuses to start at boot if the
    /// registered mailer is still the default [`crate::email::LogMailer`]
    /// (production deployments must opt in to a real mailer).
    /// Default `false`. Enforcement lands when the recovery handlers
    /// ship (R1 commit #7+); this commit ships the declaration only.
    fn strict_mailer_required(&self) -> bool;

    /// Derive the absolute base URL the reset email's link should
    /// point at. Default: see [`derive_public_site_url`] +
    /// trust-boundary docs on this trait. Projects override this
    /// method to return a fixed string (e.g. read from config) when
    /// header derivation isn't appropriate for their topology.
    ///
    /// Returns `None` when nothing resolves; the caller (R1 issue
    /// handler, commit #7) treats `None` as a hard failure and
    /// records `metadata.email_send_status = "failed"` with a clear
    /// log line.
    fn public_site_url(&self, req: &Request) -> Option<String> {
        derive_public_site_url(|name| req.header(name).map(|s| s.to_string()))
    }
}

/// Type-erased shared recovery-policy reference, mirroring
/// [`SharedPasswordPolicy`] / [`crate::email::SharedMailer`].
pub type SharedRecoveryPolicy = Arc<dyn RecoveryPolicy>;

/// Length-only / rate-limit-only baseline policy. Public fields plus
/// chainable `with_*` setters so projects that want to tweak one knob
/// don't need to author a full trait impl.
#[derive(Debug, Clone)]
pub struct DefaultRecoveryPolicy {
    pub reset_token_ttl: ChronoDuration,
    pub request_rate_limit: (u32, StdDuration),
    pub consume_rate_limit: (u32, StdDuration),
    pub strict_mailer_required: bool,
}

impl DefaultRecoveryPolicy {
    /// New policy with the framework's locked defaults
    /// (`DESIGN_RECOVERY.md` §17): TTL 1h, request 5/15min, consume
    /// 10/5min, strict-mailer guard off.
    pub fn new() -> Self {
        Self {
            reset_token_ttl: ChronoDuration::hours(1),
            request_rate_limit: (5, StdDuration::from_secs(15 * 60)),
            consume_rate_limit: (10, StdDuration::from_secs(5 * 60)),
            strict_mailer_required: false,
        }
    }

    /// Override the reset-token TTL. Projects that want shorter
    /// blast-radius windows pass `Duration::minutes(30)`; projects
    /// that need user-friendlier deadlines pass `Duration::hours(2)`.
    pub fn with_reset_token_ttl(mut self, ttl: ChronoDuration) -> Self {
        self.reset_token_ttl = ttl;
        self
    }

    /// Override the request-endpoint rate-limit shape.
    pub fn with_request_rate_limit(mut self, capacity: u32, window: StdDuration) -> Self {
        self.request_rate_limit = (capacity, window);
        self
    }

    /// Override the consume-endpoint rate-limit shape.
    pub fn with_consume_rate_limit(mut self, capacity: u32, window: StdDuration) -> Self {
        self.consume_rate_limit = (capacity, window);
        self
    }

    /// Toggle the strict-mailer boot guard. When `true`, R1's boot
    /// sequence (commits #7+) refuses to start with the default
    /// `LogMailer`. Default `false`.
    pub fn with_strict_mailer_required(mut self, required: bool) -> Self {
        self.strict_mailer_required = required;
        self
    }
}

impl Default for DefaultRecoveryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RecoveryPolicy for DefaultRecoveryPolicy {
    fn reset_token_ttl(&self) -> ChronoDuration {
        self.reset_token_ttl
    }

    fn request_rate_limit(&self) -> (u32, StdDuration) {
        self.request_rate_limit
    }

    fn consume_rate_limit(&self) -> (u32, StdDuration) {
        self.consume_rate_limit
    }

    fn strict_mailer_required(&self) -> bool {
        self.strict_mailer_required
    }

    // public_site_url uses the trait's provided default.
}

/// Pure helper for the default `RecoveryPolicy::public_site_url`
/// implementation, factored out so the parser can be unit-tested
/// without constructing a full [`Request`].
///
/// `header` is a closure that returns the named header's value (case-
/// insensitive name match, owned `String` because the default
/// closure copies out of the request's borrowed buffer).
///
/// Priority order — first source that resolves to a safe
/// `(proto, host)` pair wins:
///
/// 1. RFC 7239 `Forwarded` — first comma-separated entry's
///    `proto=` + `host=` pairs.
/// 2. `X-Forwarded-Proto` + `X-Forwarded-Host` — first CSV entry of
///    each, both required to fall through if either's missing.
/// 3. `Host` header alone — assumes `http://` (no HTTPS guesswork).
///
/// Returns `None` when nothing resolves. Never panics on malformed
/// input — see the test suite's `malformed_forwarded_inputs_never_panic`
/// for the property check.
///
/// **Trust:** see the `RecoveryPolicy` trait's "Trust boundary"
/// section. The operator's reverse proxy is responsible for
/// stripping incoming versions of these headers before its own
/// hop appends them.
pub(crate) fn derive_public_site_url<F>(header: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    // 1. RFC 7239 Forwarded — first hop
    if let Some(value) = header("forwarded") {
        if let Some(url) = parse_forwarded_first_hop(&value) {
            return Some(url);
        }
    }

    // 2. X-Forwarded-Proto + X-Forwarded-Host
    let xfp = header("x-forwarded-proto").and_then(|s| first_csv(&s).map(|v| v.to_string()));
    let xfh = header("x-forwarded-host").and_then(|s| first_csv(&s).map(|v| v.to_string()));
    if let (Some(proto), Some(host)) = (xfp, xfh) {
        if is_safe_proto(&proto) && is_safe_host(&host) {
            return Some(format!("{}://{}", proto.to_ascii_lowercase(), host));
        }
    }

    // 3. Host header — assume http
    if let Some(host) = header("host") {
        if is_safe_host(&host) {
            return Some(format!("http://{host}"));
        }
    }

    None
}

/// Take the first comma-separated, trimmed, non-empty token of `s`.
fn first_csv(s: &str) -> Option<&str> {
    let trimmed = s.split(',').next()?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Whitelist: only `http` and `https` are accepted. Case-insensitive.
fn is_safe_proto(p: &str) -> bool {
    p.eq_ignore_ascii_case("http") || p.eq_ignore_ascii_case("https")
}

/// Reject empty / over-long / control-char / whitespace hosts. Allows
/// alphanumerics, the dot/dash/underscore separators, the colon for
/// the `host:port` shape, and `[` / `]` for IPv6 literals.
fn is_safe_host(h: &str) -> bool {
    if h.is_empty() || h.len() > 253 {
        return false;
    }
    h.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '_' | '[' | ']'))
}

/// Parse `proto=` and `host=` from the FIRST comma-separated entry
/// of an RFC 7239 `Forwarded` header value. Returns the canonical
/// `proto://host` URL, or `None` if either is missing or fails the
/// safety check.
fn parse_forwarded_first_hop(value: &str) -> Option<String> {
    let first = value.split(',').next()?;
    let mut proto: Option<&str> = None;
    let mut host: Option<&str> = None;

    for pair in first.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (key, val) = match pair.split_once('=') {
            Some(p) => p,
            None => continue,
        };
        let key = key.trim();
        // Strip surrounding quotes if present (RFC 7239 allows
        // quoted-string syntax for values containing special chars).
        let val = val.trim().trim_matches('"');
        if val.is_empty() {
            continue;
        }
        if key.eq_ignore_ascii_case("proto") {
            proto = Some(val);
        } else if key.eq_ignore_ascii_case("host") {
            host = Some(val);
        }
    }

    let proto = proto?;
    let host = host?;
    if !is_safe_proto(proto) || !is_safe_host(host) {
        return None;
    }
    Some(format!("{}://{}", proto.to_ascii_lowercase(), host))
}

// ---- Runtime: token issuance + consumption -------------------------------

/// Outcome of [`issue_reset_token`]. Variants exist for
/// observability and testability — the user-facing handler renders
/// the same uniform "if that email has an account, we just sent a
/// link" page across every variant per the disclosure rule
/// (`DESIGN_RECOVERY.md` §2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IssueOutcome {
    /// A token row was inserted; the mailer dispatch attempt
    /// finished (see `email_status` for whether the message
    /// actually went out). One audit row written
    /// (`AuditEvent::PasswordResetSelfRequest`).
    Issued {
        token_id: i64,
        email_status: MailerEmailStatus,
    },
    /// Email didn't match an active user — either unknown OR
    /// deactivated. The two sub-cases are deliberately
    /// indistinguishable from outside (doctrine 9, §2.3 disclosure
    /// rule). No DB row, no audit, no mail. A `log::info!` line is
    /// written for operator-side visibility, but it never carries
    /// a token, password, or anything that could be used for
    /// enumeration analysis later.
    UnknownOrInactive,
    /// Per-IP rate-limit on the request endpoint exhausted. No DB
    /// row. Renderer treats this identically to `Issued` /
    /// `UnknownOrInactive` (uniform-response invariant).
    RateLimited,
}

/// Whether the mailer's `send` call returned `Ok` or a typed
/// `MailerError`. Persisted on the token row's `mail_status` column
/// and into the audit row's `metadata.email_send_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MailerEmailStatus {
    Sent,
    Failed,
}

/// Outcome of [`consume_reset_token`]. The user-facing handler
/// renders `Invalid` and `RateLimited` identically (the "this link
/// is no longer valid" page) per disclosure rule §2.3 — the variant
/// distinction exists for observability + tests, not for branching
/// the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConsumeOutcome {
    /// Token consumed atomically; password updated; every session
    /// for the affected user revoked through
    /// `invalidate_sessions(SessionTarget::User { user_id },
    /// SessionInvalidationReason::PasswordReset)`. One audit row
    /// written (`AuditEvent::PasswordResetSelfConsume`).
    Consumed {
        user_id: i64,
        revoked_session_count: usize,
    },
    /// Token unknown / expired / already consumed (the three are
    /// deliberately indistinguishable per §2.3). No password
    /// change, no session revocation, no audit row written. A
    /// `log::info!` line carries the token's redacted fingerprint
    /// for cross-row pivoting if the operator needs to investigate.
    Invalid,
    /// `PasswordPolicy::validate` rejected the candidate password.
    /// No DB mutation: the token stays valid for retry; the form
    /// re-renders with the policy error. The error itself is safe
    /// to render — `PasswordPolicyError` variants do not carry the
    /// candidate plaintext (see commit #4's leak-prevention test).
    PolicyRejected(PasswordPolicyError),
    /// Per-IP rate-limit on the consume endpoint exhausted. No DB
    /// mutation. Renderer treats this identically to `Invalid`.
    RateLimited,
}

/// Issue a password-reset token for `email` — or pretend to,
/// preserving the uniform-response invariant.
///
/// See `DESIGN_RECOVERY.md` §4.2 for the canonical contract this
/// implements. The function is `pub(crate)` because the framework
/// owns the route shape (CSRF, rate-limit middleware, render
/// pipeline). External projects compose recovery via the trait
/// surfaces ([`PasswordPolicy`], [`RecoveryPolicy`],
/// [`crate::email::Mailer`]) rather than calling this directly.
///
/// ## Security properties (LOCKED)
///
/// - The plaintext token leaves this function only as part of the
///   email body dispatched through [`crate::email::Mailer`]. The DB
///   row stores `token_hash = sha256(token)` only.
/// - Outward result is uniform: `IssueOutcome::Issued`,
///   `UnknownOrInactive`, and `RateLimited` all map to the same
///   user-facing page in the handler (commit #8). The variant
///   distinction is for audit + tests only.
/// - No `log::info!` / `log::error!` / audit row contains the
///   plaintext token. Logs use [`redact_token`] (8-char SHA-256
///   fingerprint); audit metadata stores `token_fingerprint`.
/// - On mailer failure (transient OR permanent OR `public_site_url`
///   derivation returning None), the outward result is still
///   `IssueOutcome::Issued { email_status: Failed }` — the row
///   exists with `mail_status = 'failed'` and the audit row carries
///   `email_send_status = "failed"`. The user sees the uniform
///   response.
pub(crate) async fn issue_reset_token(
    db: &Db,
    admin: &Admin,
    request_limiter: &RateLimiter,
    request: &Request,
    email: &str,
    correlation_id: Option<&str>,
) -> Result<IssueOutcome> {
    let ip = extract_request_ip(request);

    // 1. Per-IP rate-limit — bucket exhaustion → uniform response.
    if !request_limiter.allow(&ip) {
        log::info!(
            target: "rustio_admin::recovery::issue",
            "rate-limit exhausted ip={} correlation_id={:?}",
            ip,
            correlation_id,
        );
        return Ok(IssueOutcome::RateLimited);
    }

    // 2. Normalise email input.
    let email_input = email.trim().to_ascii_lowercase();
    if email_input.is_empty() {
        log::info!(
            target: "rustio_admin::recovery::issue",
            "empty-email submission ip={} correlation_id={:?}",
            ip,
            correlation_id,
        );
        return Ok(IssueOutcome::UnknownOrInactive);
    }

    // 3. User lookup. Both unknown-email and inactive-user collapse
    //    into UnknownOrInactive — leaking either creates an
    //    enumeration channel.
    let user = match find_user_by_email(db, &email_input).await? {
        Some(u) if u.is_active => u,
        Some(u) => {
            log::info!(
                target: "rustio_admin::recovery::issue",
                "inactive-user submission user_id={} ip={} correlation_id={:?}",
                u.id,
                ip,
                correlation_id,
            );
            return Ok(IssueOutcome::UnknownOrInactive);
        }
        None => {
            log::info!(
                target: "rustio_admin::recovery::issue",
                "unknown-email submission ip={} correlation_id={:?}",
                ip,
                correlation_id,
            );
            return Ok(IssueOutcome::UnknownOrInactive);
        }
    };

    // 4. Generate token. 256-bit URL-safe-base64. Plaintext lives
    //    only here, in the email body, and in the user's mailbox —
    //    NEVER in the DB, NEVER in any log line.
    let token = random_token();
    let token_hash = hash_token_for_storage(&token);

    // 5. Insert the token row with mail_status = 'pending'.
    let policy = admin.active_recovery_policy();
    let ttl = policy.reset_token_ttl();
    let expires_at = chrono::Utc::now() + ttl;
    let user_agent_owned = request.header("user-agent").map(|s| s.to_string());

    let token_id: i64 = sqlx::query_scalar(
        "INSERT INTO rustio_password_reset_tokens
            (user_id, token_hash, requested_ip, requested_user_agent,
             expires_at, mail_status, correlation_id)
         VALUES ($1, $2, $3, $4, $5, 'pending', $6)
       RETURNING id",
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(&ip)
    .bind(user_agent_owned.as_deref())
    .bind(expires_at)
    .bind(correlation_id)
    .fetch_one(db.pool())
    .await?;

    // 6. Compose + dispatch mail. If site-URL derivation fails or
    //    the mailer returns an error, mark mail_status = 'failed'
    //    and continue — the user-facing response stays uniform.
    let mail_status = match policy.public_site_url(request) {
        Some(public_site_url) => {
            let reset_link = format!(
                "{}/admin/reset-password/{}",
                public_site_url.trim_end_matches('/'),
                token,
            );
            let when = chrono::Utc::now();
            let body = format!(
                "We received a request to sign you back in to {site_header}.\n\n\
                 Click the link below to set a new password:\n\n\
                 {reset_link}\n\n\
                 The link expires {ttl_human}. If you didn't request this, you can \
                 safely ignore this email.\n",
                site_header = admin.branding().site_header,
                reset_link = reset_link,
                ttl_human = humanize_ttl(ttl),
            );
            let mail = Mail::framework_envelope(
                user.email.clone(),
                format!("{} — sign-in link", admin.branding().site_header),
                body,
                &admin.branding().site_header,
                Some(&ip),
                user_agent_owned.as_deref(),
                when,
            );
            match admin.active_mailer().send(mail).await {
                Ok(()) => {
                    set_token_mail_status(db, token_id, "sent").await?;
                    MailerEmailStatus::Sent
                }
                Err(e) => {
                    log::error!(
                        target: "rustio_admin::recovery::issue",
                        "mailer send failed user_id={} fingerprint={} correlation_id={:?}: {}",
                        user.id,
                        redact_token(&token),
                        correlation_id,
                        e,
                    );
                    set_token_mail_status(db, token_id, "failed").await?;
                    MailerEmailStatus::Failed
                }
            }
        }
        None => {
            log::error!(
                target: "rustio_admin::recovery::issue",
                "public_site_url derivation returned None — reset link cannot be built. \
                 user_id={} fingerprint={} correlation_id={:?}",
                user.id,
                redact_token(&token),
                correlation_id,
            );
            set_token_mail_status(db, token_id, "failed").await?;
            MailerEmailStatus::Failed
        }
    };

    // 7. Audit row. Token fingerprint, NEVER the plaintext.
    let metadata = serde_json::json!({
        "token_fingerprint": redact_token(&token),
        "email_send_status": match mail_status {
            MailerEmailStatus::Sent => "sent",
            MailerEmailStatus::Failed => "failed",
        },
        "requested_ip": ip,
        "requested_user_agent": user_agent_owned,
        "expires_at": expires_at.to_rfc3339(),
    });
    let mut entry = LogEntry::new(user.id, ActionType::Update, "user", user.id)
        .with_event(AuditEvent::PasswordResetSelfRequest);
    entry.correlation_id = correlation_id;
    entry.ip_address = Some(&ip);
    entry.metadata = Some(metadata);
    entry.summary = format!(
        "password reset requested; mail {}",
        match mail_status {
            MailerEmailStatus::Sent => "sent",
            MailerEmailStatus::Failed => "failed",
        }
    );
    audit_record(db, entry).await?;

    Ok(IssueOutcome::Issued {
        token_id,
        email_status: mail_status,
    })
}

/// Consume a reset token, set the new password, revoke every
/// session for the affected user.
///
/// See `DESIGN_RECOVERY.md` §4.3 for the canonical contract this
/// implements. The function is `pub(crate)` for the same reason
/// [`issue_reset_token`] is.
///
/// ## Security properties (LOCKED)
///
/// - **Atomic consume.** The single SQL statement
///   `UPDATE … SET consumed_at = NOW() WHERE token_hash = $1 AND
///    consumed_at IS NULL AND expires_at > NOW() RETURNING user_id`
///   is the only place a token's `consumed_at` flips. The partial
///   unique index `WHERE consumed_at IS NULL` (commit #1) makes
///   concurrent consumes resolve as one Consumed + one Invalid —
///   never two of either.
/// - **Policy first, consume second.** A bad password fails
///   validation BEFORE the atomic UPDATE, so the user can fix the
///   form and retry without burning a token.
/// - **Doctrine 22.** Session revocation goes through
///   `invalidate_sessions(SessionTarget::User, …PasswordReset)` —
///   the framework's only `revoked_at` writer.
/// - No log / audit row contains the plaintext token. Token
///   fingerprints (8-char SHA-256) are used for cross-row pivoting
///   when an operator needs to trace activity.
/// - The handler MUST NOT auto-log-in the user on success — they
///   go through `/admin/login` so MFA (R3+) gets exercised.
pub(crate) async fn consume_reset_token(
    db: &Db,
    admin: &Admin,
    consume_limiter: &RateLimiter,
    request: &Request,
    token: &str,
    new_password: &str,
    correlation_id: Option<&str>,
) -> Result<ConsumeOutcome> {
    let ip = extract_request_ip(request);

    // 1. Per-IP rate-limit — bucket exhaustion → render Invalid.
    if !consume_limiter.allow(&ip) {
        log::info!(
            target: "rustio_admin::recovery::consume",
            "rate-limit exhausted ip={} correlation_id={:?}",
            ip,
            correlation_id,
        );
        return Ok(ConsumeOutcome::RateLimited);
    }

    // 2. Validate password against policy. A bad password does NOT
    //    burn the token; the user re-tries the form.
    if let Err(e) = admin.active_password_policy().validate(new_password) {
        return Ok(ConsumeOutcome::PolicyRejected(e));
    }

    // 3. Atomic consume — see "Atomic consume" doctrine in the
    //    function-level docs above.
    let token_hash = hash_token_for_storage(token);
    let user_id: Option<i64> = sqlx::query_scalar(
        "UPDATE rustio_password_reset_tokens
            SET consumed_at = NOW()
          WHERE token_hash = $1
            AND consumed_at IS NULL
            AND expires_at > NOW()
        RETURNING user_id",
    )
    .bind(&token_hash)
    .fetch_optional(db.pool())
    .await?;

    let user_id = match user_id {
        Some(uid) => uid,
        None => {
            log::info!(
                target: "rustio_admin::recovery::consume",
                "consume on invalid/expired/consumed token ip={} fingerprint={} correlation_id={:?}",
                ip,
                redact_token(token),
                correlation_id,
            );
            return Ok(ConsumeOutcome::Invalid);
        }
    };

    // 4. Set new password. `set_password` stamps
    //    `password_changed_at` (commit #2). If this fails the
    //    token is consumed but password unchanged — rare DB-error
    //    mode, surfaces in logs; the user re-runs the request flow.
    set_password(db, user_id, new_password).await?;

    // 5. Doctrine 22: every session for the user goes through
    //    `invalidate_sessions`. Single writer of `revoked_at`.
    let outcome = invalidate_sessions(
        db,
        SessionTarget::User { user_id },
        SessionInvalidationReason::PasswordReset,
    )
    .await?;
    let revoked_session_count = outcome.revoked_session_ids.len();

    // 6. Audit row. Token fingerprint only.
    let user_agent_owned = request.header("user-agent").map(|s| s.to_string());
    let metadata = serde_json::json!({
        "token_fingerprint": redact_token(token),
        "invalidated_session_count": revoked_session_count,
        "ip": ip,
        "user_agent": user_agent_owned,
    });
    let mut entry = LogEntry::new(user_id, ActionType::Update, "user", user_id)
        .with_event(AuditEvent::PasswordResetSelfConsume);
    entry.correlation_id = correlation_id;
    entry.ip_address = Some(&ip);
    entry.metadata = Some(metadata);
    entry.summary = format!(
        "password reset self-consumed; {revoked_session_count} session(s) revoked"
    );
    audit_record(db, entry).await?;

    Ok(ConsumeOutcome::Consumed {
        user_id,
        revoked_session_count,
    })
}

/// Non-mutating check used by the `GET /admin/reset-password/<token>`
/// handler (R1 commit #8) to decide whether to render the new-
/// password form or the "this link is no longer valid" card. The
/// `POST` path still performs the atomic consume regardless — the
/// GET-time check is purely a UX courtesy so a user clicking a
/// stale link doesn't fill in the form before being told it's
/// invalid.
///
/// Disclosure-equivalent to the consume path: returns `false` for
/// unknown / expired / already-consumed tokens. The three sub-cases
/// are deliberately indistinguishable to the caller so the renderer
/// can't accidentally branch on them (`DESIGN_RECOVERY.md` §2.3).
pub(crate) async fn check_reset_token_valid(db: &Db, token: &str) -> Result<bool> {
    let token_hash = hash_token_for_storage(token);
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM rustio_password_reset_tokens
          WHERE token_hash = $1
            AND consumed_at IS NULL
            AND expires_at > NOW()
          LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(db.pool())
    .await?;
    Ok(exists.is_some())
}

/// Update an issued token's `mail_status` column. Only the values
/// `'pending' | 'sent' | 'failed'` are valid (CHECK constraint
/// added in commit #1).
async fn set_token_mail_status(db: &Db, token_id: i64, status: &str) -> Result<()> {
    sqlx::query(
        "UPDATE rustio_password_reset_tokens
            SET mail_status = $1
          WHERE id = $2",
    )
    .bind(status)
    .bind(token_id)
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Best-effort client-IP extraction from the `X-Forwarded-For`
/// header — first comma-separated entry, trimmed. Falls back to
/// `"anon"` when no proxy header is present; rate-limit buckets
/// all anonymous requests under one key in that case (acceptable
/// for single-tenant deployments; multi-tenant deployments behind
/// an unconfigured proxy get noisy and should set the header
/// upstream).
fn extract_request_ip(request: &Request) -> String {
    request
        .header("x-forwarded-for")
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "anon".to_string())
}

/// Render a `chrono::Duration` as a human-readable email-body
/// string (e.g. `"in 1 hour"`, `"in 30 minutes"`). Boundary cases
/// fall back gracefully — never returns an empty / grammatically
/// broken string.
fn humanize_ttl(ttl: ChronoDuration) -> String {
    let secs = ttl.num_seconds();
    if secs <= 0 {
        return "very soon".to_string();
    }
    if ttl.num_hours() >= 1 {
        let h = ttl.num_hours();
        return if h == 1 {
            "in 1 hour".to_string()
        } else {
            format!("in {h} hours")
        };
    }
    if ttl.num_minutes() >= 1 {
        let m = ttl.num_minutes();
        return if m == 1 {
            "in 1 minute".to_string()
        } else {
            format!("in {m} minutes")
        };
    }
    if secs == 1 {
        "in 1 second".to_string()
    } else {
        format!("in {secs} seconds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_floor_is_ten() {
        assert_eq!(DefaultPasswordPolicy::new().min_length(), 10);
        assert_eq!(DefaultPasswordPolicy::default().min_length(), 10);
    }

    #[test]
    fn default_policy_accepts_password_at_floor() {
        let p = DefaultPasswordPolicy::new();
        // Exactly 10 chars — the doctrine-locked default floor.
        assert!(p.validate("aaaaaaaaaa").is_ok());
        // Comfortable margin.
        assert!(p.validate("correct horse battery staple").is_ok());
    }

    #[test]
    fn default_policy_rejects_short_password() {
        let p = DefaultPasswordPolicy::new();
        let err = p.validate("nine_char").unwrap_err();
        assert_eq!(err, PasswordPolicyError::TooShort { min: 10, actual: 9 });
    }

    #[test]
    fn default_policy_rejects_empty_password() {
        let p = DefaultPasswordPolicy::new();
        let err = p.validate("").unwrap_err();
        assert_eq!(err, PasswordPolicyError::TooShort { min: 10, actual: 0 });
    }

    #[test]
    fn default_policy_with_min_len_overrides_floor() {
        let p = DefaultPasswordPolicy::with_min_len(16);
        assert_eq!(p.min_length(), 16);
        assert!(p.validate("fifteen_chars__").is_err()); // 15 chars
        assert!(p.validate("sixteen_chars___").is_ok()); //  16 chars
    }

    #[test]
    fn default_policy_counts_chars_not_bytes() {
        let p = DefaultPasswordPolicy::new();
        // 10 Cyrillic chars = 20 bytes. Char count passes the floor.
        let pw = "пароль1234";
        assert_eq!(pw.chars().count(), 10);
        assert!(pw.len() > 10);
        assert!(p.validate(pw).is_ok());

        // 9 Cyrillic chars must fail with the char count, not the
        // byte count.
        let pw = "пароль123";
        let err = p.validate(pw).unwrap_err();
        assert_eq!(err, PasswordPolicyError::TooShort { min: 10, actual: 9 });
    }

    #[test]
    fn error_renderings_do_not_leak_plaintext() {
        // Property: neither Display nor Debug formatting of a
        // policy error rendered for a rejected candidate leaks the
        // candidate string. Picked plaintext is unlikely to collide
        // with English words in the default error message.
        let p = DefaultPasswordPolicy::new();
        let plaintext = "Pwn4Ge#xy"; // 9 chars — fails the 10-char floor
        let err = p.validate(plaintext).unwrap_err();
        let display = format!("{err}");
        let debug = format!("{err:?}");
        assert!(
            !display.contains(plaintext),
            "Display leaked plaintext: {display}"
        );
        assert!(
            !debug.contains(plaintext),
            "Debug leaked plaintext: {debug}"
        );
    }

    #[test]
    fn custom_error_renders_message_verbatim() {
        let err = PasswordPolicyError::Custom("breached password rejected".into());
        assert_eq!(format!("{err}"), "breached password rejected");
    }

    #[test]
    fn shared_password_policy_is_send_sync() {
        // Compile-time guarantee that the trait-object alias retains
        // the bounds the framework relies on.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SharedPasswordPolicy>();
    }

    // ---- recovery policy ---------------------------------------------------

    #[test]
    fn default_recovery_policy_ttl_is_one_hour() {
        let p = DefaultRecoveryPolicy::new();
        assert_eq!(p.reset_token_ttl(), ChronoDuration::hours(1));
    }

    #[test]
    fn default_recovery_policy_request_rate_limit_is_five_per_fifteen_min() {
        let p = DefaultRecoveryPolicy::new();
        assert_eq!(p.request_rate_limit(), (5, StdDuration::from_secs(15 * 60)));
    }

    #[test]
    fn default_recovery_policy_consume_rate_limit_is_ten_per_five_min() {
        let p = DefaultRecoveryPolicy::new();
        assert_eq!(p.consume_rate_limit(), (10, StdDuration::from_secs(5 * 60)));
    }

    #[test]
    fn default_recovery_policy_strict_mailer_required_is_false() {
        // Locked-decision: project opts in via with_strict_mailer_required(true).
        // R1 commit #5 ships the field; enforcement is deferred to commit #7+.
        let p = DefaultRecoveryPolicy::new();
        assert!(!p.strict_mailer_required());
    }

    #[test]
    fn default_recovery_policy_with_overrides_apply_field_by_field() {
        let p = DefaultRecoveryPolicy::new()
            .with_reset_token_ttl(ChronoDuration::hours(2))
            .with_request_rate_limit(3, StdDuration::from_secs(60))
            .with_consume_rate_limit(20, StdDuration::from_secs(30))
            .with_strict_mailer_required(true);
        assert_eq!(p.reset_token_ttl(), ChronoDuration::hours(2));
        assert_eq!(p.request_rate_limit(), (3, StdDuration::from_secs(60)));
        assert_eq!(p.consume_rate_limit(), (20, StdDuration::from_secs(30)));
        assert!(p.strict_mailer_required());
    }

    #[test]
    fn shared_recovery_policy_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SharedRecoveryPolicy>();
    }

    // ---- public_site_url derivation ----------------------------------------

    fn header_lookup(
        pairs: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Option<String> + 'static {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn site_url_prefers_rfc7239_forwarded_first_hop() {
        let h = header_lookup(&[
            ("forwarded", "for=1.2.3.4;proto=https;host=admin.example.com"),
            ("x-forwarded-proto", "http"),
            ("x-forwarded-host", "wrong.example.com"),
            ("host", "internal.local"),
        ]);
        assert_eq!(
            derive_public_site_url(&h),
            Some("https://admin.example.com".to_string())
        );
    }

    #[test]
    fn site_url_falls_through_to_x_forwarded_pair() {
        let h = header_lookup(&[
            ("x-forwarded-proto", "https"),
            ("x-forwarded-host", "admin.example.com"),
            ("host", "internal.local"),
        ]);
        assert_eq!(
            derive_public_site_url(&h),
            Some("https://admin.example.com".to_string())
        );
    }

    #[test]
    fn site_url_x_forwarded_takes_first_csv_entry() {
        // Multiple proxy hops — outermost (closest to client) is first.
        let h = header_lookup(&[
            ("x-forwarded-proto", "https, http"),
            ("x-forwarded-host", "admin.example.com, internal.local"),
        ]);
        assert_eq!(
            derive_public_site_url(&h),
            Some("https://admin.example.com".to_string())
        );
    }

    #[test]
    fn site_url_falls_back_to_host_header_with_http() {
        let h = header_lookup(&[("host", "admin.example.com")]);
        assert_eq!(
            derive_public_site_url(&h),
            Some("http://admin.example.com".to_string())
        );
    }

    #[test]
    fn site_url_returns_none_when_no_headers_resolve() {
        let h = header_lookup(&[]);
        assert_eq!(derive_public_site_url(&h), None);
    }

    #[test]
    fn site_url_rejects_non_http_proto() {
        // A malicious client setting `Forwarded: proto=javascript`
        // must NOT poison the reset link. We refuse anything outside
        // {http, https} and fall through to the next source.
        let h = header_lookup(&[
            ("forwarded", "for=1.2.3.4;proto=javascript;host=evil.example.com"),
            ("host", "fallback.example.com"),
        ]);
        assert_eq!(
            derive_public_site_url(&h),
            Some("http://fallback.example.com".to_string())
        );
    }

    #[test]
    fn site_url_rejects_host_with_whitespace_or_control() {
        let h = header_lookup(&[("host", "example.com\r\nX-Injected: yes")]);
        assert_eq!(derive_public_site_url(&h), None);
    }

    #[test]
    fn site_url_handles_quoted_forwarded_values() {
        let h = header_lookup(&[(
            "forwarded",
            "for=\"_obfuscated\";proto=\"https\";host=\"admin.example.com\"",
        )]);
        assert_eq!(
            derive_public_site_url(&h),
            Some("https://admin.example.com".to_string())
        );
    }

    #[test]
    fn site_url_handles_ipv6_bracketed_host() {
        let h = header_lookup(&[
            ("x-forwarded-proto", "https"),
            ("x-forwarded-host", "[2001:db8::1]:8443"),
        ]);
        assert_eq!(
            derive_public_site_url(&h),
            Some("https://[2001:db8::1]:8443".to_string())
        );
    }

    // ---- humanize_ttl ----

    #[test]
    fn humanize_ttl_one_hour_default() {
        assert_eq!(humanize_ttl(ChronoDuration::hours(1)), "in 1 hour");
    }

    #[test]
    fn humanize_ttl_two_hours_pluralises() {
        assert_eq!(humanize_ttl(ChronoDuration::hours(2)), "in 2 hours");
    }

    #[test]
    fn humanize_ttl_minutes() {
        assert_eq!(humanize_ttl(ChronoDuration::minutes(30)), "in 30 minutes");
        assert_eq!(humanize_ttl(ChronoDuration::minutes(1)), "in 1 minute");
    }

    #[test]
    fn humanize_ttl_seconds_for_short_windows() {
        assert_eq!(humanize_ttl(ChronoDuration::seconds(45)), "in 45 seconds");
        assert_eq!(humanize_ttl(ChronoDuration::seconds(1)), "in 1 second");
    }

    #[test]
    fn humanize_ttl_zero_or_negative_returns_safe_string() {
        // Boundary: a TTL that's already in the past renders as a
        // grammatically safe placeholder. Never empty, never broken.
        assert_eq!(humanize_ttl(ChronoDuration::zero()), "very soon");
        assert_eq!(humanize_ttl(ChronoDuration::seconds(-30)), "very soon");
    }

    // ---- IssueOutcome / ConsumeOutcome leak prevention ----

    #[test]
    fn issue_outcome_debug_never_carries_plaintext_token() {
        // Variants are designed without a token field; this test
        // pins that property — a future change that adds one would
        // fail this. Synthetic plaintext is unlikely to collide
        // with the structural form-fields ("Issued", "token_id",
        // numbers, etc.).
        let synthetic = "Pwn4Ge_ZZ_token_plaintext_1234567890";
        for outcome in [
            IssueOutcome::Issued {
                token_id: 42,
                email_status: MailerEmailStatus::Sent,
            },
            IssueOutcome::Issued {
                token_id: 7,
                email_status: MailerEmailStatus::Failed,
            },
            IssueOutcome::UnknownOrInactive,
            IssueOutcome::RateLimited,
        ] {
            let debug = format!("{outcome:?}");
            assert!(
                !debug.contains(synthetic),
                "IssueOutcome Debug leaked plaintext: {debug}",
            );
        }
    }

    #[test]
    fn consume_outcome_debug_never_carries_plaintext_token() {
        let synthetic = "Pwn4Ge_ZZ_token_plaintext_1234567890";
        for outcome in [
            ConsumeOutcome::Consumed {
                user_id: 1,
                revoked_session_count: 3,
            },
            ConsumeOutcome::Invalid,
            ConsumeOutcome::PolicyRejected(PasswordPolicyError::TooShort {
                min: 10,
                actual: 4,
            }),
            ConsumeOutcome::PolicyRejected(PasswordPolicyError::Custom(
                "stub rejected".into(),
            )),
            ConsumeOutcome::RateLimited,
        ] {
            let debug = format!("{outcome:?}");
            assert!(
                !debug.contains(synthetic),
                "ConsumeOutcome Debug leaked plaintext: {debug}",
            );
        }
    }

    #[test]
    fn mailer_email_status_round_trip_strings() {
        // Locked-in for the audit metadata field
        // `email_send_status` — values are 'sent' / 'failed'.
        assert_eq!(format!("{:?}", MailerEmailStatus::Sent), "Sent");
        assert_eq!(format!("{:?}", MailerEmailStatus::Failed), "Failed");
    }

    #[test]
    fn malformed_forwarded_inputs_never_panic() {
        for input in &[
            "",
            "garbage",
            "for=",
            "proto=;host=",
            "proto=javascript:alert(1);host=evil",
            "host=example com",
            "proto=https;host=",
            ";;;",
            ",,,",
            "proto=https",
            "host=example.com",
            "for=\"unterminated",
            "=value",
            "key=",
            "key==value=",
        ] {
            let value = (*input).to_string();
            // The lookup returns the test input for "forwarded" and
            // a safe host fallback so we exercise the "fall through"
            // path too.
            let h = move |name: &str| match name {
                "forwarded" => Some(value.clone()),
                "host" => Some("fallback.example.com".to_string()),
                _ => None,
            };
            // Property: never panics. The result is acceptable as
            // long as the fall-through landed somewhere safe.
            let result = derive_public_site_url(h);
            assert!(
                result.is_none()
                    || result.as_deref() == Some("http://fallback.example.com")
                    || result.as_deref().map(|s| s.starts_with("https://")) == Some(true)
                    || result.as_deref().map(|s| s.starts_with("http://")) == Some(true),
                "input {input:?} produced unexpected url {result:?}"
            );
        }
    }
}
