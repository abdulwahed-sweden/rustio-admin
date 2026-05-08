//! Email delivery abstraction.
//!
//! Doctrine 6: email is operational infrastructure, not business
//! logic. Recovery flows compose [`Mail`] objects with a fixed
//! envelope ([`Mail::framework_envelope`]) and dispatch them through
//! a project-supplied [`Mailer`] implementation. The framework
//! refuses to lock into SMTP — projects ship whatever transport
//! their organisation already uses (SES, Mailgun, Postmark, internal
//! relay, queued background job, etc.).
//!
//! ## What the framework provides
//!
//! - The [`Mailer`] trait — one async method, `send`.
//! - [`LogMailer`] — the safe default. Writes the would-be email to
//!   `log::info!` so reset links stay visible during dev / CI without
//!   a mail server. **Not suitable for production**: in production a
//!   real mailer must be configured or recovery emails will be
//!   silently lost.
//! - [`Mail`] + [`Mail::framework_envelope`] for canonical headers
//!   (timestamp, source IP, browser/OS summary, "if this was not you"
//!   guidance) per doctrine 6.
//!
//! ## Project override
//!
//! ```ignore
//! let admin = Admin::new()
//!     .mailer(Arc::new(MyProjectMailer::new(/* SES, Mailgun, … */)));
//! ```
//!
//! The default is [`LogMailer`] — projects opt in to a real mailer
//! by registering one. R1+ recovery flows will read the configured
//! mailer from `Admin` and refuse to boot in `production` mode if
//! none is registered.

use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, Utc};

/// One outbound message. Plaintext body is required; HTML is
/// optional. Extra headers are project-controlled.
#[derive(Debug, Clone)]
pub struct Mail {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
    pub headers: Vec<(String, String)>,
}

impl Mail {
    /// Build a message with the framework's canonical security
    /// envelope appended to the plaintext body. Used by recovery
    /// flows so every framework-emitted email carries the same
    /// "where, when, what, who" context — anti-phishing parity.
    ///
    /// `system_name` should be the project's `SiteBranding::site_header`
    /// (the human label of the install); `request_ip` and `ua_summary`
    /// are best-effort context lifted from the triggering request.
    pub fn framework_envelope(
        to: impl Into<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
        system_name: &str,
        request_ip: Option<&str>,
        ua_summary: Option<&str>,
        when: DateTime<Utc>,
    ) -> Self {
        let mut text = body.into();
        text.push_str("\n\n— — —\n");
        text.push_str(&format!("System: {system_name}\n"));
        text.push_str(&format!("When:   {} UTC\n", when.format("%Y-%m-%d %H:%M")));
        if let Some(ip) = request_ip {
            text.push_str(&format!("From IP: {ip}\n"));
        }
        if let Some(ua) = ua_summary {
            text.push_str(&format!("Device:  {ua}\n"));
        }
        text.push_str(
            "\nIf this was not you, sign in and visit /admin/account/sessions to revoke \
             sessions, then ask your administrator to reset your account.\n",
        );
        Mail {
            to: to.into(),
            subject: subject.into(),
            text_body: text,
            html_body: None,
            headers: Vec::new(),
        }
    }
}

/// Errors a [`Mailer`] can return. The most important variant is
/// [`MailerError::ConfigurationMissing`] — the framework treats it
/// as a hard boot failure when production deployments forget to wire
/// up a real mailer (per the user-locked decision: "Mailer blocking
/// behaviour" → refuse to start when no mailer is configured for
/// production).
#[derive(Debug)]
#[non_exhaustive]
pub enum MailerError {
    /// The mailer is structurally missing — no transport, no API
    /// key, etc. R1+ boot guards check for this at startup.
    ConfigurationMissing(String),
    /// A transient failure (SMTP timeout, 5xx from the API, queue
    /// full). Recovery flows treat this as "log + uniform user
    /// response" per the user-locked mailer-blocking-behaviour
    /// decision: the user sees the same response as success; an
    /// audit row is written with `metadata.email_send_status =
    /// "failed"` so the operator can grep for undelivered resets.
    Transient(String),
    /// A non-recoverable failure (invalid recipient, blocked
    /// domain). Surfaces in audit logs the same way as Transient.
    Permanent(String),
}

impl fmt::Display for MailerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigurationMissing(m) => write!(f, "mailer configuration missing: {m}"),
            Self::Transient(m) => write!(f, "mailer transient failure: {m}"),
            Self::Permanent(m) => write!(f, "mailer permanent failure: {m}"),
        }
    }
}

impl std::error::Error for MailerError {}

/// Async outbound-mail interface. Project implementations live in
/// the project crate so the framework never imports `lettre` /
/// `aws-sdk-ses` / etc.
///
/// Implementations MUST be `Send + Sync` and cheap to clone (the
/// framework holds a single `Arc<dyn Mailer>` for the lifetime of
/// the process).
pub trait Mailer: Send + Sync {
    /// Send one message. Errors are typed; see [`MailerError`].
    fn send<'a>(
        &'a self,
        msg: Mail,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), MailerError>> + Send + 'a>,
    >;
}

/// Default mailer. Writes the message to `log::info!` instead of
/// sending it. Safe for dev / CI / testing where outbound SMTP is
/// forbidden or undesirable; not suitable for production — recovery
/// emails will be lost (the audit row will record the attempt).
///
/// Subjects and recipient addresses appear in the log output;
/// **bodies are truncated** at 200 chars and **anything that looks
/// like a token is replaced with a fingerprint** before logging
/// (doctrine 11 — never log secrets).
#[derive(Debug, Default, Clone)]
pub struct LogMailer;

impl LogMailer {
    pub fn new() -> Self {
        Self
    }
}

impl Mailer for LogMailer {
    fn send<'a>(
        &'a self,
        msg: Mail,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), MailerError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let body_preview: String = msg.text_body.chars().take(200).collect();
            // Doctrine 11: redact anything that looks like a URL with
            // a token segment before it lands in the log target.
            let redacted = redact_likely_tokens(&body_preview);
            log::info!(
                target: "rustio_admin::mailer::log",
                "[LogMailer] to={} subject={:?} body_preview={:?}",
                msg.to,
                msg.subject,
                redacted,
            );
            Ok(())
        })
    }
}

/// Replace anything that looks like a URL ending in a long
/// alphanumeric segment (a reset-token-shaped suffix) with the
/// `<redacted-link>` placeholder. Pure function; no I/O.
fn redact_likely_tokens(s: &str) -> String {
    // Heuristic: any whitespace-delimited segment ≥ 32 chars that's
    // ASCII alphanumeric + - / _ + : / . is replaced. Catches
    // "/admin/reset-password/<token>" style URLs and bare tokens.
    s.split_whitespace()
        .map(|w| {
            if w.len() >= 32 && w.chars().all(is_token_url_char) {
                "<redacted-link>"
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_token_url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | ':' | '.' | '?' | '&' | '=' | '#')
}

/// Type-erased shared mailer reference. The framework's `Admin`
/// holds one of these; defaults to `Arc::new(LogMailer)` until a
/// project overrides via `Admin::mailer(Arc::new(...))`.
pub type SharedMailer = Arc<dyn Mailer>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_envelope_appends_security_footer() {
        let m = Mail::framework_envelope(
            "user@example.com",
            "Test",
            "Body line.",
            "Bosphorus & Sham · Stockholm",
            Some("198.51.100.42"),
            Some("macOS · Safari 18"),
            Utc::now(),
        );
        assert!(m.text_body.contains("Body line."));
        assert!(m.text_body.contains("System: Bosphorus & Sham · Stockholm"));
        assert!(m.text_body.contains("From IP: 198.51.100.42"));
        assert!(m.text_body.contains("Device:  macOS · Safari 18"));
        assert!(m.text_body.contains("If this was not you"));
    }

    #[test]
    fn framework_envelope_omits_missing_fields() {
        let m = Mail::framework_envelope(
            "user@example.com",
            "Test",
            "Body.",
            "ACME",
            None,
            None,
            Utc::now(),
        );
        assert!(!m.text_body.contains("From IP:"));
        assert!(!m.text_body.contains("Device:"));
        assert!(m.text_body.contains("If this was not you"));
    }

    #[tokio::test]
    async fn log_mailer_send_is_ok() {
        let m = LogMailer::new();
        let mail = Mail {
            to: "user@example.com".into(),
            subject: "Hi".into(),
            text_body: "test body".into(),
            html_body: None,
            headers: Vec::new(),
        };
        assert!(m.send(mail).await.is_ok());
    }

    #[test]
    fn redact_likely_tokens_redacts_long_alnum_strings() {
        let s = "Click http://example.com/admin/reset-password/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa to reset.";
        let r = redact_likely_tokens(s);
        assert!(r.contains("<redacted-link>"));
        assert!(!r.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn redact_likely_tokens_keeps_short_words() {
        let s = "Hello user, your account is fine.";
        let r = redact_likely_tokens(s);
        assert_eq!(r, s);
    }

    #[test]
    fn mailer_error_display() {
        let e = MailerError::ConfigurationMissing("no SMTP host".into());
        assert!(format!("{e}").contains("configuration missing"));
        let e = MailerError::Transient("timeout".into());
        assert!(format!("{e}").contains("transient"));
        let e = MailerError::Permanent("blocked".into());
        assert!(format!("{e}").contains("permanent"));
    }
}
