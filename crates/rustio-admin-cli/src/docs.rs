//! `rustio-admin docs` — running-server detection + browser opener.
//!
//! PR 2.4 of the Stage 2 onboarding work. Three small affordances
//! on top of the existing `rustio-admin docs` placeholder (PR 1.1):
//!
//! 1. Probe `http://127.0.0.1:8000/admin/health` with a short
//!    timeout so the printed output reflects whether the local
//!    docs are actually reachable right now.
//! 2. `--open` flag that shells out to `open` / `xdg-open` /
//!    `cmd /c start` to launch the local docs in the default
//!    browser. Default off — auto-opening a browser is the kind
//!    of magic `DESIGN_ONBOARDING.md` §4 forbids.
//! 3. Structured output (paths cyan, URLs cyan-underlined, hints
//!    dim) via `console::style`, NO_COLOR-safe via the crate's
//!    own detection.
//!
//! Discipline:
//!
//! - Raw `std::net::TcpStream` for the health probe; no HTTP
//!   client dep. The probe is one well-formed GET to localhost;
//!   classifying 2xx/3xx from the status line is enough.
//! - 500 ms timeout. Failure-closed: any error / refused / hung
//!   connection returns "not running".
//! - `--open` exits 1 on failure (browser launch failed or
//!   server unreachable while `--open` was set) so scripts can
//!   detect it.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use console::style;

/// Where the framework's default `cargo run` binds. The probe and
/// the printed URLs all reference the same canonical host:port;
/// changing this is a coordinated edit across the scaffold's
/// `main.rs.tmpl`, the homepage, and this module.
const HOST: &str = "127.0.0.1";
const PORT: u16 = 8000;
const HEALTH_PATH: &str = "/admin/health";
const DOCS_PATH: &str = "/admin/docs";

/// Online surfaces. Stable enough to hardcode.
const ONLINE_DOCS: &str = "https://docs.rs/rustio-admin";
const REPO_URL: &str = "https://github.com/abdulwahed-sweden/rustio-admin";

/// 500 ms total budget for the probe (connect + write + read of
/// the status line). Tight enough that the CLI never hangs;
/// generous enough that a healthy local server always wins.
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Entry point called from `main`. Returns the desired process
/// exit code (0 success, 1 if `--open` failed to open).
pub(crate) fn print_docs(open: bool) -> Result<(), String> {
    let local_url = format!("http://{HOST}:{PORT}{DOCS_PATH}");
    let running = health_probe();

    if open {
        if running {
            println!(
                "Opening {} in your default browser …",
                style(&local_url).cyan().underlined()
            );
            return open_url_in_browser(&local_url);
        }
        // --open requested but no server reachable. Print the
        // "cannot open" block + the online URLs, then exit 1.
        eprintln!(
            "Cannot open the local docs: no server reachable on {}.",
            style(&format!("http://{HOST}:{PORT}")).cyan()
        );
        eprintln!(
            "Start the server with {}, then re-run {}.",
            style("`cargo run`").green(),
            style("`rustio-admin docs --open`").green(),
        );
        eprintln!();
        eprintln!(
            "  {}  {}",
            style("online").dim(),
            style(ONLINE_DOCS).cyan().underlined()
        );
        eprintln!(
            "  {}    {}",
            style("repo").dim(),
            style(REPO_URL).cyan().underlined()
        );
        return Err("server unreachable; --open could not be honoured".into());
    }

    // No --open: print the URL block. Header differs by whether
    // the local server is reachable.
    if running {
        println!("RustIO docs are running on this project.");
        println!();
        println!(
            "  {}   {}",
            style("local").dim(),
            style(&local_url).cyan().underlined()
        );
        println!(
            "  {}  {}",
            style("online").dim(),
            style(ONLINE_DOCS).cyan().underlined()
        );
        println!();
        println!(
            "({} to open the local docs in your browser)",
            style("pass --open").dim()
        );
    } else {
        println!("RustIO documentation");
        println!();
        println!(
            "  {}   {}  ({} to enable)",
            style("local").dim(),
            style(&local_url).cyan().underlined(),
            style("start the server with `cargo run`").dim()
        );
        println!(
            "  {}  {}",
            style("online").dim(),
            style(ONLINE_DOCS).cyan().underlined()
        );
        println!(
            "  {}    {}",
            style("repo").dim(),
            style(REPO_URL).cyan().underlined()
        );
        println!();
        println!(
            "({} to open the local docs in your browser, once the server is running)",
            style("pass --open").dim()
        );
    }
    Ok(())
}

/// Probe the local rustio-admin server on `127.0.0.1:8000` by
/// issuing a short-timeout HTTP GET on `/admin/health`. Returns
/// `true` only when the response is a 2xx / 3xx **AND** carries
/// the `x-correlation-id` response header that the framework's
/// `middleware::correlation_id` sets on every response. The
/// header check is the specificity signal -- without it, any
/// random HTTP service bound to port 8000 (a translation tool,
/// a Python `http.server`, etc.) would falsely fool the probe.
///
/// `x-correlation-id` is set by the framework before any auth
/// gate runs, so the check works on the typical
/// unauthenticated-redirect path (`/admin/health` -> 303 ->
/// `/admin/login`) and on the authenticated 200 path.
///
/// Raw `TcpStream`; no HTTP client dependency. We only ever talk
/// to localhost, only ever send one well-formed request, only
/// ever care about the status digit + one header presence --
/// a hand-rolled probe is the right tradeoff vs. pulling in
/// `ureq` / `hyper` for one endpoint.
fn health_probe() -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    let mut stream = match TcpStream::connect_timeout(&addr, PROBE_TIMEOUT) {
        Ok(s) => s,
        Err(_) => return false,
    };
    if stream.set_read_timeout(Some(PROBE_TIMEOUT)).is_err() {
        return false;
    }
    if stream.set_write_timeout(Some(PROBE_TIMEOUT)).is_err() {
        return false;
    }
    let req = format!(
        "GET {HEALTH_PATH} HTTP/1.1\r\n\
         Host: {HOST}:{PORT}\r\n\
         User-Agent: rustio-cli-health-probe\r\n\
         Connection: close\r\n\r\n"
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    // 1 KiB is comfortably past "HTTP/1.1 NNN <reason>\r\n" plus
    // the framework's standard response headers (correlation_id,
    // security_headers, set-cookie, etc.).
    let mut buf = vec![0u8; 1024];
    let mut total = 0;
    while total < buf.len() {
        match stream.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        // Stop reading once we've seen the end of the header
        // block; the body is irrelevant to the probe.
        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    if total < 12 || !buf.starts_with(b"HTTP/1.") {
        return false;
    }
    // Status line: "HTTP/1.x NNN ..."; byte 9 is the first digit
    // of the 3-digit status code.
    if !matches!(buf[9], b'2' | b'3') {
        return false;
    }
    // Specificity: require the framework's correlation_id
    // response header. Case-insensitive search across the
    // captured header bytes.
    let lower: Vec<u8> = buf[..total]
        .iter()
        .map(|b| b.to_ascii_lowercase())
        .collect();
    lower
        .windows(b"x-correlation-id:".len())
        .any(|w| w == b"x-correlation-id:")
}

/// Shell out to the platform's URL-opener. Errors are surfaced
/// to the caller (which prints + exits 1).
fn open_url_in_browser(url: &str) -> Result<(), String> {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        // `cmd /c start "" "<url>"` — the empty `""` is the
        // window-title argument; required because `start` treats
        // the first quoted argument as the title.
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .status()
    } else {
        // Linux / BSD / etc.
        std::process::Command::new("xdg-open").arg(url).status()
    };
    let status = result.map_err(|e| format!("failed to launch browser opener: {e}"))?;
    if !status.success() {
        return Err(format!(
            "browser opener exited with status {status} (URL: {url})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// The probe must return `false` quickly when nothing is
    /// listening on the canonical port. Failure-closed is the
    /// contract.
    #[test]
    fn health_probe_returns_false_when_nothing_listening() {
        // We can't guarantee port 8000 is free on every test host,
        // so this test is best-effort -- if a real rustio-admin server
        // happens to be running on 8000, the assertion below is
        // skipped. CI runs on a clean box where 8000 is free.
        if TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], PORT)),
            Duration::from_millis(50),
        )
        .is_ok()
        {
            eprintln!("[skip] port {PORT} appears to be in use; cannot verify failure-closed");
            return;
        }
        assert!(
            !health_probe(),
            "probe must return false on refused connect"
        );
    }

    /// Helper: bind a stub on PORT and serve one canned response.
    /// Skips the calling test if PORT is in use.
    fn spawn_stub(canned: &'static [u8]) -> Option<thread::JoinHandle<()>> {
        let listener = TcpListener::bind(("127.0.0.1", PORT)).ok()?;
        Some(thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 256];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(canned);
            }
        }))
    }

    /// The probe must classify a 2xx response carrying the
    /// rustio-specific `x-correlation-id` header as "running".
    #[test]
    fn health_probe_recognises_rustio_200_with_correlation_id() {
        let Some(handle) = spawn_stub(
            b"HTTP/1.1 200 OK\r\n\
              Content-Type: text/plain\r\n\
              x-correlation-id: 019e0000-0000-7000-0000-000000000001\r\n\
              Content-Length: 2\r\n\
              Connection: close\r\n\r\nok",
        ) else {
            eprintln!("[skip] port {PORT} in use; cannot bind stub");
            return;
        };
        let result = health_probe();
        let _ = handle.join();
        assert!(
            result,
            "probe must recognise a rustio 200 (with correlation_id) as 'running'"
        );
    }

    /// The typical unauthenticated `/admin/health` shape on a real
    /// rustio-admin server: 303 redirect to `/admin/login`. Must be
    /// classified as "running" because the framework's
    /// correlation_id middleware fires before any auth gate.
    #[test]
    fn health_probe_recognises_303_with_correlation_id() {
        let Some(handle) = spawn_stub(
            b"HTTP/1.1 303 See Other\r\n\
              Location: /admin/login\r\n\
              x-correlation-id: 019e0000-0000-7000-0000-000000000002\r\n\
              Content-Length: 0\r\n\
              Connection: close\r\n\r\n",
        ) else {
            eprintln!("[skip] port {PORT} in use");
            return;
        };
        let result = health_probe();
        let _ = handle.join();
        assert!(
            result,
            "probe must classify rustio 303 (with correlation_id) as 'running'"
        );
    }

    /// Audit-driven regression test: a foreign HTTP server bound
    /// to PORT that responds 2xx/3xx but does NOT carry
    /// `x-correlation-id` (a translation tool, a Python
    /// http.server, jenkins, etc.) must NOT be classified as
    /// rustio. Caught when a `translati` service on
    /// 127.0.0.1:8000 returned 303 and the original probe
    /// falsely classified it as running.
    #[test]
    fn health_probe_rejects_foreign_server_without_correlation_id() {
        let Some(handle) = spawn_stub(
            b"HTTP/1.1 303 See Other\r\n\
              Location: /login\r\n\
              Content-Length: 0\r\n\
              Connection: close\r\n\r\n",
        ) else {
            eprintln!("[skip] port {PORT} in use");
            return;
        };
        let result = health_probe();
        let _ = handle.join();
        assert!(!result, "probe must reject a 303 without x-correlation-id");
    }

    /// A 5xx response means the server is up but unhealthy --
    /// not the "running" we want to signal. The probe must
    /// return false even when the header IS present.
    #[test]
    fn health_probe_classifies_500_as_not_running() {
        let Some(handle) = spawn_stub(
            b"HTTP/1.1 500 Internal Server Error\r\n\
              x-correlation-id: 019e0000-0000-7000-0000-000000000003\r\n\
              Content-Length: 0\r\n\
              Connection: close\r\n\r\n",
        ) else {
            eprintln!("[skip] port {PORT} in use");
            return;
        };
        let result = health_probe();
        let _ = handle.join();
        assert!(!result, "probe must NOT classify a 500 as 'running'");
    }

    /// Garbage on the socket must NOT be classified as a healthy
    /// HTTP response.
    #[test]
    fn health_probe_rejects_non_http_response() {
        let Some(handle) = spawn_stub(b"GARBLE_GARBLE_NOT_HTTP\r\n") else {
            eprintln!("[skip] port {PORT} in use");
            return;
        };
        let result = health_probe();
        let _ = handle.join();
        assert!(
            !result,
            "non-HTTP response must be classified as not running"
        );
    }
}
