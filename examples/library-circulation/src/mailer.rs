//! Real SMTP `Mailer` implementation for the library-circulation
//! example.
//!
//! The framework keeps `lettre` out of its dependency tree on
//! principle — projects own their email transport choice. This file
//! is the canonical reference for "how to wire a real mailer," and
//! the rest of this crate consumes it via `Admin::mailer(...)` in
//! `main.rs`.
//!
//! ## Configuration
//!
//! Reads SMTP credentials from the process environment:
//!
//!   | env var          | default       | required? | example                    |
//!   |------------------|---------------|-----------|----------------------------|
//!   | `SMTP_HOST`      | (none)        | yes       | `smtp.gmail.com`           |
//!   | `SMTP_PORT`      | `465`         | no        | `465` (SMTPS) or `587` (STARTTLS) |
//!   | `SMTP_USER`      | (none)        | yes       | `you@gmail.com`            |
//!   | `SMTP_PASSWORD`  | (none)        | yes       | 16-char Gmail App Password |
//!   | `SMTP_TLS`       | `implicit`    | no        | `implicit` (465) or `starttls` (587) |
//!   | `MAIL_FROM`      | (= `SMTP_USER`) | no      | `RustIO Admin <you@gmail.com>` |
//!
//! If `SMTP_HOST` is unset, `main.rs` falls back to the framework's
//! `LogMailer` (writes the would-be email to stdout). That fallback
//! is the right default for `cargo test`, CI, and any environment
//! where outbound SMTP is undesirable.
//!
//! ## Failure model
//!
//! Each `send` returns a typed `rustio_admin::email::MailerError`:
//!   - `ConfigurationMissing` — recoverable, project misconfiguration.
//!   - `Transient` — network / SMTP 4xx / TLS handshake retries.
//!   - `Permanent` — SMTP 5xx, malformed address, auth rejected.
//!
//! The framework's recovery dispatcher logs the failure, marks the
//! reset-token row's `mail_status = 'failed'`, and still returns the
//! generic "if that email exists" response to the user
//! (anti-enumeration). The user never learns whether their email was
//! reachable.

use std::env;
use std::time::Duration;

use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use rustio_admin::email::{Mail, Mailer, MailerError};

/// SMTP transport configuration resolved from the environment at
/// process start. Cloned into the `LettreSmtpMailer` so the
/// per-send path doesn't re-read env vars on every email.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: Mailbox,
    /// `true` → implicit TLS on connect (port 465 SMTPS).
    /// `false` → STARTTLS upgrade after EHLO (port 587).
    pub implicit_tls: bool,
}

/// Pulls `SMTP_*` + `MAIL_FROM` from the environment.
///
/// Returns:
///   - `Ok(Some(cfg))` — all required vars present and parseable.
///   - `Ok(None)`      — `SMTP_HOST` unset; the caller falls back
///                       to `LogMailer`.
///   - `Err(...)`      — `SMTP_HOST` set but required companions
///                       missing / malformed. Project misconfigured;
///                       refusing to boot is louder than silent
///                       fallback.
pub fn smtp_config_from_env() -> Result<Option<SmtpConfig>, String> {
    let host = match env::var("SMTP_HOST") {
        Ok(h) if !h.trim().is_empty() => h.trim().to_string(),
        _ => return Ok(None),
    };

    let port: u16 = match env::var("SMTP_PORT") {
        Ok(p) => p
            .trim()
            .parse()
            .map_err(|e| format!("SMTP_PORT is not a valid port number: {e}"))?,
        Err(_) => 465,
    };

    let username = env::var("SMTP_USER")
        .map_err(|_| "SMTP_HOST is set but SMTP_USER is missing".to_string())?
        .trim()
        .to_string();
    if username.is_empty() {
        return Err("SMTP_USER must not be empty".into());
    }

    let password = env::var("SMTP_PASSWORD")
        .map_err(|_| "SMTP_HOST is set but SMTP_PASSWORD is missing".to_string())?;
    if password.is_empty() {
        return Err("SMTP_PASSWORD must not be empty".into());
    }

    let tls_mode = env::var("SMTP_TLS").unwrap_or_else(|_| "implicit".into());
    let implicit_tls = match tls_mode.to_ascii_lowercase().as_str() {
        "implicit" | "smtps" => true,
        "starttls" => false,
        other => return Err(format!("SMTP_TLS must be 'implicit' or 'starttls' (got {other:?})")),
    };

    let from_raw = env::var("MAIL_FROM").unwrap_or_else(|_| username.clone());
    let from: Mailbox = from_raw
        .parse()
        .map_err(|e| format!("MAIL_FROM is not a valid mailbox: {e}"))?;

    Ok(Some(SmtpConfig {
        host,
        port,
        username,
        password,
        from,
        implicit_tls,
    }))
}

/// `Mailer` implementation backed by `lettre`'s async SMTP
/// transport. One transport is built at construction and reused for
/// every `send` — lettre handles connection pooling internally.
pub struct LettreSmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl LettreSmtpMailer {
    /// Build the transport from `cfg`. Performs no I/O; the first
    /// network call happens inside `send`.
    pub fn new(cfg: SmtpConfig) -> Result<Self, String> {
        let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());
        let tls_params = TlsParameters::new(cfg.host.clone())
            .map_err(|e| format!("SMTP TLS parameters: {e}"))?;

        let builder = if cfg.implicit_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
                .map_err(|e| format!("SMTP relay setup: {e}"))?
                .port(cfg.port)
                .tls(Tls::Wrapper(tls_params))
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
                .map_err(|e| format!("SMTP starttls setup: {e}"))?
                .port(cfg.port)
                .tls(Tls::Required(tls_params))
        };

        let transport = builder
            .credentials(creds)
            .timeout(Some(Duration::from_secs(15)))
            .build();

        Ok(Self {
            transport,
            from: cfg.from,
        })
    }
}

impl Mailer for LettreSmtpMailer {
    fn send<'a>(
        &'a self,
        msg: Mail,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), MailerError>> + Send + 'a>,
    > {
        Box::pin(async move {
            // Address parsing. Malformed recipient → permanent error.
            let to: Mailbox = msg
                .to
                .parse()
                .map_err(|e| MailerError::Permanent(format!("recipient address: {e}")))?;

            // Build the multipart message: text fallback + optional
            // HTML alternative. lettre sends them as
            // `multipart/alternative`; the client picks whichever
            // representation it prefers.
            let builder = Message::builder()
                .from(self.from.clone())
                .to(to)
                .subject(&msg.subject);

            // Custom headers from `Mail::headers` are reserved for a
            // future framework emission (e.g. `Reply-To`,
            // `List-Unsubscribe`) and are not yet routed through the
            // lettre builder. The recovery flow doesn't set any, so
            // this is a deliberate gap, not a regression.
            if !msg.headers.is_empty() {
                log::warn!(
                    target: "library_circulation::mailer",
                    "Mail.headers carried {} entries — custom header dispatch \
                     not yet wired in LettreSmtpMailer; dropping",
                    msg.headers.len()
                );
            }

            let message = match &msg.html_body {
                Some(html) => builder
                    .multipart(
                        MultiPart::alternative()
                            .singlepart(
                                SinglePart::builder()
                                    .header(ContentType::TEXT_PLAIN)
                                    .body(msg.text_body.clone()),
                            )
                            .singlepart(
                                SinglePart::builder()
                                    .header(ContentType::TEXT_HTML)
                                    .body(html.clone()),
                            ),
                    )
                    .map_err(|e| MailerError::Permanent(format!("MIME build: {e}")))?,
                None => builder
                    .header(ContentType::TEXT_PLAIN)
                    .body(msg.text_body.clone())
                    .map_err(|e| MailerError::Permanent(format!("MIME build: {e}")))?,
            };

            self.transport.send(message).await.map_err(classify_error)?;
            Ok(())
        })
    }
}

/// Map lettre's transport errors to the framework's `MailerError`
/// taxonomy.
///
/// Coarse classification by inspecting the error's `Display`
/// representation. lettre's error type doesn't expose a stable
/// programmatic accessor for "response code" across all 0.11.x
/// patch versions, so we string-sniff for the obvious 5xx
/// signatures (Gmail returns these on auth failure / disallowed
/// sender). Everything else is treated as Transient — the framework
/// already marks the reset-token row's mail_status = 'failed' on
/// any send error, so the misclassification cost is only in the
/// `MailerError` enum-tag visible to a future retry pump.
fn classify_error(e: lettre::transport::smtp::Error) -> MailerError {
    let msg = e.to_string();
    let lower = msg.to_ascii_lowercase();
    // Gmail / standard SMTP 5xx auth + sender-policy failures.
    if lower.contains("530")            // not authenticated
        || lower.contains("535")        // bad credentials
        || lower.contains("550")        // mailbox / policy reject
        || lower.contains("553")        // mailbox name not allowed
        || lower.contains("authentication")
        || lower.contains("permanent")
    {
        return MailerError::Permanent(format!("SMTP transport: {msg}"));
    }
    MailerError::Transient(format!("SMTP transport: {msg}"))
}
