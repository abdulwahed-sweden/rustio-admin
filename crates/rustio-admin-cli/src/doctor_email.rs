//! `rustio doctor email` — SMTP self-validation.
//!
//! Reads the same `SMTP_*` + `MAIL_FROM` envelope the framework
//! example reads (so a `.env` that boots the app also passes
//! the doctor) and runs four checks:
//!
//!   1. Env-var presence — every required key is set + non-empty.
//!   2. TLS handshake — open the socket, complete TLS / STARTTLS.
//!   3. Authentication — EHLO + AUTH LOGIN (or AUTH PLAIN).
//!   4. Test send — optional, only when `--to <address>` is
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

use std::env;
use std::time::Duration;

use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Run the email-doctor checks. `send_to`: when `Some(addr)`,
/// dispatches a tiny test message after the handshake validates;
/// when `None`, the handshake is the deepest check (no email
/// goes out).
pub async fn run(send_to: Option<String>) -> Result<(), String> {
    println!("rustio doctor email — validating SMTP configuration");
    println!();

    // ---- 1. Env-var presence ---------------------------------
    let host = require_env("SMTP_HOST")?;
    let user = require_env("SMTP_USER")?;
    let pass = require_env("SMTP_PASSWORD")?;
    let port_raw = env::var("SMTP_PORT").unwrap_or_else(|_| "465".into());
    let port: u16 = port_raw
        .trim()
        .parse()
        .map_err(|e| format!("✗ SMTP_PORT is not a valid port number ({e})"))?;
    let tls_mode = env::var("SMTP_TLS").unwrap_or_else(|_| "implicit".into());
    let implicit_tls = match tls_mode.to_ascii_lowercase().as_str() {
        "implicit" | "smtps" => true,
        "starttls" => false,
        other => {
            println!("✗ SMTP_TLS must be 'implicit' or 'starttls' (got {other:?})");
            return Err("bad SMTP_TLS".into());
        }
    };
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
            println!("    (Gmail: must be a 16-char App Password — no spaces)");
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
            println!("rustio doctor email — all checks passed.");
            Ok(())
        }
        Some(to_raw) => {
            let to: Mailbox = to_raw
                .parse()
                .map_err(|e| format!("✗ --to is not a valid mailbox: {e}"))?;
            let plain = format!(
                "This is a rustio-admin doctor smoke test.\n\n\
                 If you can read this in your inbox, your SMTP \
                 configuration is correct end-to-end.\n\n\
                 — — —\n\
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
                .subject("rustio-admin doctor — SMTP smoke test")
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
            println!("OK");
            println!("✓ Test message accepted by remote (delivery in transit)");
            println!();
            println!("rustio doctor email — all checks passed.");
            println!();
            println!("Check the inbox of {to}; the message will arrive within seconds.");
            println!("Gmail may route the first message from a new SMTP sender to Spam — ");
            println!("if you don't see it, look there.");
            Ok(())
        }
    }
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
