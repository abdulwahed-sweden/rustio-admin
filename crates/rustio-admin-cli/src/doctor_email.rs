//! `rustio doctor email` ----- SMTP self-validation.
//!
//! Reads the same `SMTP_*` + `MAIL_FROM` envelope the framework
//! example reads (so a `.env` that boots the app also passes
//! the doctor) and runs four checks:
//!
//!   1. Env-var presence ----- every required key is set + non-empty.
//!   2. TLS handshake ----- open the socket, complete TLS / STARTTLS.
//!   3. Authentication ----- EHLO + AUTH LOGIN (or AUTH PLAIN).
//!   4. Test send ----- optional, only when `--to <address>` is
//!      passed. Builds a tiny multipart message ("rustio-admin
//!      doctor smoke test") and ships it through the same
//!      transport the recovery flow uses.
//!
//! Each check answers with a ✓ / ⚠ / ✗ line. The output matches
//! `rustio doctor`'s aesthetic so an operator's eye can scan
//! both surfaces the same way.
//!
//! No credentials are echoed; SMTP_PASSWORD is reported as
//! `(set, N chars)` only.
//!
//! Modes:
//!   - default (handshake only)
//!   - `--to <address>` (handshake + real send)
//!   - `--html-preview` (renders the recovery email body to
//!     `/tmp/rustio-email-preview.html` and opens it ----- no SMTP
//!     traffic at all; useful for visual iteration without
//!     burning a real send).
//!
//! Rate limiting: when `--to` is passed, the doctor writes a
//! cooldown stamp to `/tmp/rustio-doctor-email-last-send` and
//! refuses to repeat within 30 seconds. Cheap accidental-spam
//! safety net for developers who hit ↑↑Enter in a loop.

use std::env;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

const SEND_COOLDOWN_SECS: u64 = 30;
const COOLDOWN_PATH: &str = "/tmp/rustio-doctor-email-last-send";
const PREVIEW_PATH: &str = "/tmp/rustio-email-preview.html";

/// Provider preset table. Returns `(host, port, implicit_tls,
/// default_user_hint)` for known keys. Operators set
/// `MAIL_PROVIDER=gmail` (or `resend`, `postmark`, `mailgun`,
/// `sendgrid`, `ethereal`) and the doctor + framework example
/// fill the corresponding host / port / TLS fields automatically.
/// Explicit `SMTP_HOST` / `SMTP_PORT` / `SMTP_TLS` always
/// override the preset.
fn provider_preset(name: &str) -> Option<(&'static str, u16, bool, Option<&'static str>)> {
    match name.trim().to_ascii_lowercase().as_str() {
        "gmail" => Some(("smtp.gmail.com", 465, true, None)),
        "resend" => Some(("smtp.resend.com", 465, true, Some("resend"))),
        "postmark" => Some(("smtp.postmarkapp.com", 587, false, None)),
        "mailgun" => Some(("smtp.mailgun.org", 587, false, None)),
        "sendgrid" => Some(("smtp.sendgrid.net", 587, false, Some("apikey"))),
        "ethereal" => Some(("smtp.ethereal.email", 587, false, None)),
        _ => None,
    }
}

/// Run the email-doctor checks.
///   `send_to`: when `Some`, dispatches a real test message
///   after the handshake. Triggers cooldown enforcement.
///   `html_preview`: when `true`, renders the recovery email
///   body to `/tmp` and opens it ----- runs no SMTP traffic.
pub async fn run(send_to: Option<String>, html_preview: bool) -> Result<(), String> {
    if html_preview {
        return run_html_preview().await;
    }

    println!("rustio doctor email ----- validating SMTP configuration");
    println!();

    // ---- 1. Env-var presence + provider preset resolution ----
    let provider = env::var("MAIL_PROVIDER").ok();
    let preset = provider.as_deref().and_then(provider_preset);
    if let Some(p) = provider.as_deref() {
        if preset.is_some() {
            println!("✓ MAIL_PROVIDER = {p} (preset applied)");
        } else {
            println!(
                "⚠ MAIL_PROVIDER = {p} ----- unknown preset; falling back to explicit SMTP_* vars"
            );
            println!("  known presets: gmail, resend, postmark, mailgun, sendgrid, ethereal");
        }
    }
    let host = resolve_host(preset)?;
    let user = resolve_user(preset)?;
    let pass = require_env("SMTP_PASSWORD")?;
    let port = resolve_port(preset)?;
    let (tls_mode, implicit_tls) = resolve_tls(preset)?;
    let from_raw = env::var("MAIL_FROM").unwrap_or_else(|_| user.clone());
    let from: Mailbox = from_raw
        .parse()
        .map_err(|e| format!("✗ MAIL_FROM is not a valid mailbox: {e}"))?;

    println!("✓ Env vars present");
    println!("    SMTP_HOST     = {host}");
    println!("    SMTP_PORT     = {port}");
    println!("    SMTP_USER     = {user}");
    println!("    SMTP_PASSWORD = (set, {} chars)", pass.len());
    println!("    SMTP_TLS      = {tls_mode}");
    println!("    MAIL_FROM     = {from}");
    println!();

    // ---- 2. + 3. Build transport + handshake ------------------
    let tls_params = TlsParameters::new(host.clone())
        .map_err(|e| format!("✗ TLS parameter construction: {e}"))?;

    let builder = if implicit_tls {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .map_err(|e| format!("✗ SMTP relay setup: {e}"))?
            .port(port)
            .tls(Tls::Wrapper(tls_params))
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|e| format!("✗ SMTP starttls setup: {e}"))?
            .port(port)
            .tls(Tls::Required(tls_params))
    };

    let transport = builder
        .credentials(Credentials::new(user.clone(), pass))
        .timeout(Some(Duration::from_secs(15)))
        .build::<Tokio1Executor>();

    print!("• Handshake (TCP → TLS → EHLO → AUTH → QUIT)… ");
    use std::io::Write;
    std::io::stdout().flush().ok();
    match transport.test_connection().await {
        Ok(true) => {
            println!("OK");
            println!("✓ TLS handshake succeeded");
            println!("✓ SMTP authentication succeeded");
            println!();
        }
        Ok(false) => {
            println!("FAILED (server returned negative)");
            return Err("SMTP server refused handshake".into());
        }
        Err(e) => {
            println!("FAILED");
            println!();
            println!("✗ {e}");
            println!();
            println!("Common causes:");
            println!("  • SMTP_PASSWORD is wrong");
            println!("    (Gmail: must be a 16-char App Password ----- no spaces)");
            println!("  • 2-Step Verification is not enabled on the Google account");
            println!("    (App Passwords require 2FA; enable it first then regenerate)");
            println!("  • Wrong port + TLS combination");
            println!("    (use 465 + implicit, or 587 + starttls)");
            println!("  • Network egress to {host}:{port} is blocked");
            println!("    (corporate firewall, VPN, etc.)");
            return Err("SMTP handshake failed".into());
        }
    }

    // ---- 4. Test send (optional) -----------------------------
    match send_to {
        None => {
            println!("· Test send skipped (pass `--to <address>` to dispatch a real message)");
            println!();
            println!("rustio doctor email ----- all checks passed.");
            Ok(())
        }
        Some(to_raw) => {
            // Rate-limit: refuse to repeat within the cooldown window.
            // Prevents accidental-spam from a developer hammering the
            // command. Stamp lives in /tmp; survives across CLI runs
            // but not across reboots.
            if let Some(remaining) = cooldown_remaining() {
                println!("✗ Cooldown active ----- last `--to` send was {remaining}s ago.");
                println!(
                    "  Wait {wait}s before sending another (cooldown is {window}s; \
                     prevents accidental loops).",
                    wait = SEND_COOLDOWN_SECS.saturating_sub(remaining),
                    window = SEND_COOLDOWN_SECS,
                );
                return Err("send cooldown".into());
            }
            let to: Mailbox = to_raw
                .parse()
                .map_err(|e| format!("✗ --to is not a valid mailbox: {e}"))?;
            let plain = format!(
                "This is a rustio-admin doctor smoke test.\n\n\
                 If you can read this in your inbox, your SMTP \
                 configuration is correct end-to-end.\n\n\
                 ----- ----- -----\n\
                 Sent from `rustio doctor email --to {to}`.\n"
            );
            let html = format!(
                "<!DOCTYPE html><html><body style=\"margin:0;padding:48px 24px;\
                 background:#F7F9FC;font-family:-apple-system,BlinkMacSystemFont,\
                 'Inter','Segoe UI',Roboto,sans-serif;color:#111827;\">\
                 <table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" \
                 border=\"0\" style=\"max-width:520px;margin:0 auto;\
                 background:#FFFFFF;border:1px solid #DEE3EC;border-radius:8px;\">\
                 <tr><td style=\"padding:36px;\">\
                 <div style=\"font-size:11px;font-weight:600;letter-spacing:0.10em;\
                 color:#6B7280;text-transform:uppercase;margin-bottom:18px;\">\
                 rustio-admin · doctor</div>\
                 <h1 style=\"margin:0 0 14px 0;color:#0B0F19;font-size:24px;\
                 line-height:1.25;font-weight:700;letter-spacing:-0.012em;\">\
                 SMTP smoke test</h1>\
                 <p style=\"margin:0 0 20px 0;color:#374151;font-size:15px;\
                 line-height:1.6;\">If you can read this, your SMTP \
                 configuration delivers end-to-end.</p>\
                 <div style=\"padding:14px 16px;background:#F0FDF4;border:1px solid \
                 #BBF7D0;border-radius:6px;font-size:13px;color:#166534;\">\
                 ✓ TLS handshake ✓ Authentication ✓ Delivery</div>\
                 <p style=\"margin:24px 0 0 0;color:#9CA3AF;font-size:12px;\
                 line-height:1.5;\">Sent from <code style=\"font-family:\
                 SFMono-Regular,Menlo,monospace;font-size:11px;\">rustio doctor \
                 email --to {to}</code>.</p>\
                 </td></tr></table></body></html>"
            );

            let msg = Message::builder()
                .from(from)
                .to(to.clone())
                .subject("rustio-admin doctor ----- SMTP smoke test")
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(plain),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html),
                        ),
                )
                .map_err(|e| format!("✗ MIME build: {e}"))?;

            print!("• Sending test message to {to}… ");
            std::io::stdout().flush().ok();
            transport
                .send(msg)
                .await
                .map_err(|e| format!("FAILED\n✗ {e}"))?;
            stamp_cooldown();
            println!("OK");
            println!("✓ Test message accepted by remote (delivery in transit)");
            println!();
            println!("rustio doctor email ----- all checks passed.");
            println!();
            println!("Check the inbox of {to}; the message will arrive within seconds.");
            println!("Gmail may route the first message from a new SMTP sender to Spam ----- ");
            println!("if you don't see it, look there.");
            Ok(())
        }
    }
}

// ============================================================
// HTML preview mode ----- no SMTP traffic.
//
// Renders the framework's recovery email template with realistic
// placeholder data and writes the HTML to /tmp, then opens it in
// the operator's default browser. Useful for iterating on email
// design without burning a real send through Gmail's first-time-
// sender heuristics.
// ============================================================

async fn run_html_preview() -> Result<(), String> {
    println!("rustio doctor email ----- rendering HTML preview");
    println!();

    let app_name = env::var("APP_NAME").unwrap_or_else(|_| "Library Circulation".into());
    let app_tagline = env::var("MAIL_FOOTER_TEXT")
        .ok()
        .or_else(|| Some("Operational library management".to_string()));
    let support_email = env::var("SUPPORT_EMAIL").ok();

    let when = chrono::Utc::now();
    let intro = format!(
        "We received a request to reset the password for your \
         {app_name} account. Choose a new password to continue."
    );
    let fine_print = "This link expires in 1 hour.".to_string();
    let mut parts = rustio_admin::email::RecoveryEmailParts::new(
        &app_name,
        "Reset your password",
        "Abdulwahed",
        &intro,
        "http://127.0.0.1:3000/admin/reset-password/preview-token-not-real",
        &fine_print,
        when,
    );
    parts.app_tagline = app_tagline.as_deref();
    parts.request_ip = Some("127.0.0.1");
    parts.ua_summary = Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15");
    parts.correlation_id = Some("019e2200-0000-7000-8000-000000abc123");
    parts.signature_primary = Some("Abdulwahed Mansour");
    parts.signature_title = Some("Principal Administrator");
    parts.support_email = support_email.as_deref();
    let html = rustio_admin::email::render_recovery_html(parts);

    std::fs::write(PREVIEW_PATH, &html)
        .map_err(|e| format!("✗ Failed to write {PREVIEW_PATH}: {e}"))?;
    println!("✓ Rendered {} bytes", html.len());
    println!("✓ Written to {PREVIEW_PATH}");

    // Open in the default browser. On macOS use `open`, on Linux
    // `xdg-open` if available; both are fire-and-forget ----- the
    // doctor doesn't care if the open succeeded.
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        // Other platforms: print the path and let the operator
        // open it themselves.
        ""
    };
    if !opener.is_empty() {
        match std::process::Command::new(opener)
            .arg(PREVIEW_PATH)
            .status()
        {
            Ok(s) if s.success() => {
                println!("✓ Opened in default browser");
            }
            Ok(_) | Err(_) => {
                println!("⚠ Could not open browser automatically; open the path above manually");
            }
        }
    } else {
        println!("· Open the file path manually in your browser");
    }
    println!();
    println!("Preview uses realistic placeholder data. Re-run with");
    println!("APP_NAME=... / SUPPORT_EMAIL=... / MAIL_FOOTER_TEXT=... in your");
    println!("environment to render with your project's identity.");
    Ok(())
}

// ============================================================
// Env-var resolvers ----- preset-aware.
// Explicit SMTP_* env vars always override the preset values.
// ============================================================

type Preset = (&'static str, u16, bool, Option<&'static str>);

fn resolve_host(preset: Option<Preset>) -> Result<String, String> {
    if let Ok(v) = env::var("SMTP_HOST") {
        if !v.trim().is_empty() {
            return Ok(v);
        }
    }
    if let Some((h, _, _, _)) = preset {
        return Ok(h.to_string());
    }
    println!("✗ SMTP_HOST is not set and no MAIL_PROVIDER preset matches.");
    println!("  Either set MAIL_PROVIDER=gmail|resend|postmark|mailgun|sendgrid|ethereal");
    println!("  or set SMTP_HOST explicitly.");
    Err("SMTP_HOST missing".into())
}

fn resolve_port(preset: Option<Preset>) -> Result<u16, String> {
    if let Ok(v) = env::var("SMTP_PORT") {
        if !v.trim().is_empty() {
            return v
                .trim()
                .parse()
                .map_err(|e| format!("✗ SMTP_PORT is not a valid port number ({e})"));
        }
    }
    if let Some((_, p, _, _)) = preset {
        return Ok(p);
    }
    Ok(465)
}

fn resolve_tls(preset: Option<Preset>) -> Result<(String, bool), String> {
    if let Ok(v) = env::var("SMTP_TLS") {
        if !v.trim().is_empty() {
            let implicit = match v.to_ascii_lowercase().as_str() {
                "implicit" | "smtps" => true,
                "starttls" => false,
                other => {
                    println!("✗ SMTP_TLS must be 'implicit' or 'starttls' (got {other:?})");
                    return Err("bad SMTP_TLS".into());
                }
            };
            return Ok((v, implicit));
        }
    }
    if let Some((_, _, tls, _)) = preset {
        let label = if tls { "implicit" } else { "starttls" };
        return Ok((label.to_string(), tls));
    }
    Ok(("implicit".to_string(), true))
}

fn resolve_user(preset: Option<Preset>) -> Result<String, String> {
    if let Ok(v) = env::var("SMTP_USER") {
        if !v.trim().is_empty() {
            return Ok(v);
        }
    }
    if let Some((_, _, _, Some(hint))) = preset {
        return Ok(hint.to_string());
    }
    println!("✗ SMTP_USER is not set. Add it to .env or your shell environment.");
    Err("SMTP_USER missing".into())
}

fn require_env(name: &str) -> Result<String, String> {
    match env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => {
            println!("✗ {name} is not set (or empty). Add it to .env or your shell environment.");
            Err(format!("{name} missing"))
        }
    }
}

// ============================================================
// Cooldown ----- minimal accidental-spam safety rail on `--to`.
// Stamp lives in /tmp; the file mtime is the timestamp.
// ============================================================

fn cooldown_remaining() -> Option<u64> {
    let p = PathBuf::from(COOLDOWN_PATH);
    let meta = std::fs::metadata(&p).ok()?;
    let modified = meta.modified().ok()?;
    let now = SystemTime::now();
    let elapsed = now
        .duration_since(modified)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    if elapsed < SEND_COOLDOWN_SECS {
        Some(elapsed)
    } else {
        None
    }
}

fn stamp_cooldown() {
    let _ = std::fs::write(
        COOLDOWN_PATH,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default(),
    );
}
