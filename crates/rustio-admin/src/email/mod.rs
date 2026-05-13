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

// public:
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
    // public:
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

    // public:
    /// Builder: attach a rendered HTML body alongside the existing
    /// plaintext. The mailer transport sends both as a
    /// `multipart/alternative` MIME tree; clients pick whichever
    /// part they prefer. Plaintext stays the source of truth — the
    /// HTML is a polished alternative, not a replacement.
    pub fn with_html(mut self, html: impl Into<String>) -> Self {
        self.html_body = Some(html.into());
        self
    }
}

// public:
/// Render the framework's polished HTML body for a recovery /
/// magic-link email. The visual treatment matches DESIGN_CHROME.md:
/// calm typography, a single brand-accent point of emphasis (the
/// CTA button), hairline separation, table-based layout for email-
/// client compatibility, inlined CSS for clients that strip
/// `<style>` blocks, and a `@media` query for mobile readability.
///
/// Inputs are pre-validated by the caller. The function does no
/// HTML-escaping on `intro` or `fine_print` — those are framework-
/// owned strings, never user-supplied. `cta_url` is escaped because
/// it contains the reset token which is base64 (safe but
/// belt-and-braces).
pub fn render_recovery_html(parts: RecoveryEmailParts<'_>) -> String {
    let RecoveryEmailParts {
        site_header,
        title,
        intro,
        cta_label,
        cta_url,
        fine_print,
        when,
        request_ip,
        ua_summary,
    } = parts;

    let cta_url_safe = html_attr_escape(cta_url);
    let cta_url_text = html_text_escape(cta_url);
    let site_header_text = html_text_escape(site_header);
    let title_text = html_text_escape(title);
    let intro_text = html_text_escape(intro);
    let fine_print_text = html_text_escape(fine_print);
    let cta_label_text = html_text_escape(cta_label);

    let when_str = when.format("%Y-%m-%d %H:%M UTC").to_string();
    let ip_row = match request_ip {
        Some(ip) => format!(
            "<tr><td style=\"padding:6px 0;color:#6B7280;font-size:13px;\
             width:90px;vertical-align:top;\">From IP</td>\
             <td style=\"padding:6px 0;color:#1F2937;font-size:13px;\
             font-variant-numeric:tabular-nums;\">{}</td></tr>",
            html_text_escape(ip)
        ),
        None => String::new(),
    };
    let ua_row = match ua_summary {
        Some(ua) => format!(
            "<tr><td style=\"padding:6px 0;color:#6B7280;font-size:13px;\
             width:90px;vertical-align:top;\">Device</td>\
             <td style=\"padding:6px 0;color:#1F2937;font-size:13px;\
             word-break:break-word;\">{}</td></tr>",
            html_text_escape(ua)
        ),
        None => String::new(),
    };

    // Preheader: shown in inbox preview rows; hidden in the body.
    let preheader = format!("{title_text} — {fine_print_text}");
    let preheader_safe = html_text_escape(&preheader);

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light">
<meta name="supported-color-schemes" content="light">
<title>{title_text}</title>
<style>
  /* Mobile readability bump — Apple Mail, Gmail mobile, Outlook iOS */
  @media only screen and (max-width: 600px) {{
    .rio-mail-shell {{ padding: 24px 16px !important; }}
    .rio-mail-card {{ padding: 32px 24px !important; }}
    .rio-mail-title {{ font-size: 22px !important; }}
    .rio-mail-cta a {{ padding: 14px 24px !important; }}
  }}
</style>
</head>
<body style="margin:0;padding:0;background:#F7F9FC;color:#1F2937;
  font-family:-apple-system,BlinkMacSystemFont,'Inter','Segoe UI',Roboto,
  Helvetica,Arial,sans-serif;-webkit-font-smoothing:antialiased;
  -webkit-text-size-adjust:100%;">

<!-- Preheader: inbox preview text, hidden in the body itself. -->
<div style="display:none;font-size:1px;color:#F7F9FC;line-height:1px;
  max-height:0;max-width:0;opacity:0;overflow:hidden;">{preheader_safe}</div>

<table role="presentation" width="100%" cellpadding="0" cellspacing="0"
  border="0" style="background:#F7F9FC;">
<tr>
<td align="center" class="rio-mail-shell" style="padding:48px 24px;">

  <table role="presentation" width="100%" cellpadding="0" cellspacing="0"
    border="0" style="max-width:560px;width:100%;
    background:#FFFFFF;border:1px solid #DEE3EC;border-radius:8px;
    box-shadow:0 1px 2px rgba(17,24,39,0.04);">
  <tr>
  <td class="rio-mail-card" style="padding:40px 40px 32px 40px;">

    <!-- Wordmark — small, calm; not oversized branding -->
    <div style="font-size:12px;font-weight:600;letter-spacing:0.08em;
      color:#6B7280;text-transform:uppercase;margin:0 0 24px 0;">
      {site_header_text}
    </div>

    <!-- Title -->
    <h1 class="rio-mail-title" style="margin:0 0 16px 0;color:#111827;
      font-size:26px;line-height:1.25;font-weight:600;
      letter-spacing:-0.012em;">
      {title_text}
    </h1>

    <!-- Intro -->
    <p style="margin:0 0 32px 0;color:#374151;font-size:15px;
      line-height:1.6;">
      {intro_text}
    </p>

    <!-- CTA Button: single point of emphasis -->
    <table role="presentation" class="rio-mail-cta" cellpadding="0"
      cellspacing="0" border="0" style="margin:0 0 24px 0;">
    <tr>
    <td style="border-radius:6px;background:#0F8C7E;">
      <a href="{cta_url_safe}"
        style="display:inline-block;padding:14px 28px;
        font-family:-apple-system,BlinkMacSystemFont,'Inter','Segoe UI',Roboto,
        Helvetica,Arial,sans-serif;font-size:15px;font-weight:600;
        color:#FFFFFF;text-decoration:none;letter-spacing:-0.005em;
        border-radius:6px;">
        {cta_label_text}
      </a>
    </td>
    </tr>
    </table>

    <!-- URL fallback for clients that strip buttons -->
    <p style="margin:0 0 8px 0;color:#6B7280;font-size:13px;line-height:1.5;">
      Or copy and paste this link into your browser:
    </p>
    <p style="margin:0 0 32px 0;font-size:13px;line-height:1.5;
      word-break:break-all;font-family:'SFMono-Regular',Menlo,Consolas,
      'Liberation Mono',monospace;">
      <a href="{cta_url_safe}" style="color:#0F8C7E;text-decoration:none;">{cta_url_text}</a>
    </p>

    <!-- Fine print: TTL -->
    <p style="margin:0 0 32px 0;color:#6B7280;font-size:13px;line-height:1.5;">
      {fine_print_text}
    </p>

    <!-- Divider -->
    <hr style="border:none;border-top:1px solid #ECEFF4;margin:0 0 24px 0;">

    <!-- Security envelope — system / when / IP / device -->
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0"
      border="0" style="margin:0 0 28px 0;">
    <tr><td style="padding:6px 0;color:#6B7280;font-size:13px;
      width:90px;vertical-align:top;">System</td>
      <td style="padding:6px 0;color:#1F2937;font-size:13px;">{site_header_text}</td></tr>
    <tr><td style="padding:6px 0;color:#6B7280;font-size:13px;
      width:90px;vertical-align:top;">When</td>
      <td style="padding:6px 0;color:#1F2937;font-size:13px;
      font-variant-numeric:tabular-nums;">{when_str}</td></tr>
    {ip_row}
    {ua_row}
    </table>

    <!-- Warning panel: if not you -->
    <div style="padding:16px 18px;background:#FFF8EB;border:1px solid #F2D9A7;
      border-radius:6px;margin:0 0 8px 0;">
      <p style="margin:0;color:#6B4F12;font-size:13px;line-height:1.5;">
        <strong style="color:#4F3B0A;font-weight:600;">If this wasn't you</strong>
        — ignore this email. Your password stays unchanged. You can also
        sign in and visit your sessions page to revoke any open sessions.
      </p>
    </div>

  </td>
  </tr>
  </table>

  <!-- Footer -->
  <p style="margin:20px 0 0 0;color:#6B7280;font-size:12px;line-height:1.5;
    text-align:center;">
    Sent by {site_header_text} at <span style="font-variant-numeric:tabular-nums;">{when_str}</span>.
  </p>

</td>
</tr>
</table>

</body>
</html>"##,
    )
}

// public:
/// Structured inputs to [`render_recovery_html`]. Kept separate
/// from `Mail` so the framework's HTML email surface stays
/// declarative — call-sites pass labelled fields rather than a
/// long positional argument list.
///
/// `#[non_exhaustive]` so future additions (e.g. a project-
/// supplied accent override, a footer-link tuple) are SemVer-safe.
#[non_exhaustive]
pub struct RecoveryEmailParts<'a> {
    pub site_header: &'a str,
    pub title: &'a str,
    pub intro: &'a str,
    pub cta_label: &'a str,
    pub cta_url: &'a str,
    pub fine_print: &'a str,
    pub when: DateTime<Utc>,
    pub request_ip: Option<&'a str>,
    pub ua_summary: Option<&'a str>,
}

/// Minimal HTML-text escape for the visible-text positions in the
/// recovery template. Covers `&`, `<`, `>`, `"`, `'` — the five
/// canonical entities.
fn html_text_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Slightly stricter escape for the `href="..."` attribute
/// position — drops control bytes that browsers sometimes
/// interpret. The reset token is base64-url-safe so this is
/// belt-and-braces.
fn html_attr_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // ASCII control bytes — never legitimate in a URL
            c if (c as u32) < 0x20 || (c as u32) == 0x7f => { /* drop */ }
            c => out.push(c),
        }
    }
    out
}

// public:
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

// public:
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

// public:
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
    // public:
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

// public:
/// Type-erased shared mailer reference. The framework's `Admin`
/// holds one of these; defaults to `Arc::new(LogMailer)` until a
/// project overrides via `Admin::mailer(Arc::new(...))`.
pub type SharedMailer = Arc<dyn Mailer>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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

    #[test]
    fn with_html_attaches_alternative_body() {
        let m = Mail::framework_envelope(
            "user@example.com",
            "Test",
            "Plain.",
            "ACME",
            None,
            None,
            Utc::now(),
        )
        .with_html("<p>Rich</p>");
        assert!(m.text_body.contains("Plain."));
        assert_eq!(m.html_body.as_deref(), Some("<p>Rich</p>"));
    }

    #[test]
    fn recovery_html_contains_required_markers_and_escapes() {
        let when = Utc.with_ymd_and_hms(2026, 5, 13, 14, 30, 0).unwrap();
        let html = render_recovery_html(RecoveryEmailParts {
            site_header: "RustIO Admin",
            title: "Sign back in to your account",
            intro: "We received a request to sign you back in to RustIO Admin. \
                    Click the button below to set a new password.",
            cta_label: "Set a new password",
            cta_url: "http://127.0.0.1:3000/admin/reset-password/abc123",
            fine_print: "This link expires in 30 minutes.",
            when,
            request_ip: Some("127.0.0.1"),
            ua_summary: Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Safari/605.1.15"),
        });
        // Structural markers
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("viewport"));
        assert!(html.contains("multipart")  == false); // we don't reference multipart in HTML
        // Required content
        assert!(html.contains("RustIO Admin"));
        assert!(html.contains("Sign back in to your account"));
        assert!(html.contains("Set a new password"));
        assert!(html.contains("http://127.0.0.1:3000/admin/reset-password/abc123"));
        assert!(html.contains("This link expires in 30 minutes."));
        assert!(html.contains("2026-05-13 14:30 UTC"));
        assert!(html.contains("127.0.0.1"));
        assert!(html.contains("Mozilla/5.0"));
        // Anti-phishing copy
        assert!(html.contains("If this wasn"));
        // Brand-accent CTA
        assert!(html.contains("#0F8C7E"));

        // Write a copy to /tmp so the developer can open it in a
        // browser to visually verify the rendered email. Cheap
        // dev-feedback loop; failure to write is non-fatal (CI may
        // run in read-only sandboxes).
        let _ = std::fs::write("/tmp/rustio-recovery-email-preview.html", &html);
    }

    #[test]
    fn recovery_html_escapes_html_in_inputs() {
        let html = render_recovery_html(RecoveryEmailParts {
            site_header: "<script>alert(1)</script>",
            title: "Title & co",
            intro: "Body <em>x</em>",
            cta_label: "Click >>",
            cta_url: "http://example.com/?a=1&b=2",
            fine_print: "Expires in <30> minutes",
            when: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            request_ip: Some("<bad>"),
            ua_summary: Some("\"chrome\""),
        });
        // Script tag must NOT be present unescaped anywhere
        assert!(!html.contains("<script>alert(1)</script>"));
        // Escape outputs must be present
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("Title &amp; co"));
        assert!(html.contains("Body &lt;em&gt;x&lt;/em&gt;"));
        assert!(html.contains("Click &gt;&gt;"));
        // URL attribute escaping for ampersand
        assert!(html.contains("?a=1&amp;b=2"));
        assert!(html.contains("&lt;bad&gt;"));
        assert!(html.contains("&quot;chrome&quot;"));
    }
}
