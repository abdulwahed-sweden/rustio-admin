//! `GET /admin/healthz` — public liveness/readiness probe.
//!
//! Unauthenticated by design — load balancers, k8s probes, and
//! uptime checkers don't carry session cookies. Returns a small
//! JSON envelope and a meaningful HTTP status:
//!
//! - **200 OK** with `{"ok": true,  "db": "up",   "version": "<crate>"}`
//!   when the framework can round-trip a `SELECT 1` against the
//!   Postgres pool.
//! - **503 Service Unavailable** with
//!   `{"ok": false, "db": "down", "version": "<crate>"}` when the
//!   probe query fails. Operators wiring k8s readiness probes get
//!   the right "remove this pod from the LB" signal without having
//!   to parse the body.
//!
//! `version` carries `rustio-admin`'s `CARGO_PKG_VERSION` at compile
//! time — handy for matrix-debugging "which pod is on which
//! framework version" without a separate `/version` route.
//!
//! `Cache-Control: no-store` keeps intermediaries from caching the
//! probe result — a `200` cached for 60s would mask a real outage.
//!
//! The route is registered ahead of the wildcard `/admin/:admin_name`
//! so a model literally named `healthz` couldn't shadow it (the
//! Tier-2 model-name guard would refuse such a name anyway, but
//! the route order doubles as defence-in-depth).

use hyper::StatusCode;

use crate::error::Result;
use crate::http::Response;
use crate::orm::Db;

/// Public health-check handler. Probes the DB with `SELECT 1` and
/// builds the JSON envelope around the result.
pub(crate) async fn healthz(db: &Db) -> Result<Response> {
    let db_ok = sqlx::query("SELECT 1").execute(db.pool()).await.is_ok();
    Ok(build_response(db_ok))
}

/// Pure-function half of the handler, separated so unit tests
/// can assert the response shape without spinning up a Postgres
/// pool. Same body / header / status code the real handler emits.
fn build_response(db_ok: bool) -> Response {
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = serde_json::json!({
        "ok": db_ok,
        "db": if db_ok { "up" } else { "down" },
        "version": env!("CARGO_PKG_VERSION"),
    });
    let s = serde_json::to_string(&body).unwrap_or_else(|_| {
        // Two static-shaped strings, can only fail under OOM —
        // hand-build the fallback so the contract still holds.
        format!(r#"{{"ok":{db_ok},"db":"{}","version":"unknown"}}"#,
            if db_ok { "up" } else { "down" })
    });
    Response::new(status, s)
        .with_header("content-type", "application/json")
        .with_header("cache-control", "no-store")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_response_is_200_with_db_up() {
        let resp = build_response(true);
        assert_eq!(resp.status.as_u16(), 200);
        let body = std::str::from_utf8(&resp.body).expect("utf-8");
        let v: serde_json::Value = serde_json::from_str(body).expect("valid json");
        assert_eq!(v["ok"], true);
        assert_eq!(v["db"], "up");
        // Version is the crate's CARGO_PKG_VERSION at compile time;
        // assert it's present and non-empty rather than pinning it
        // to a specific string (test would break on every release).
        assert!(
            v["version"].as_str().is_some_and(|s| !s.is_empty()),
            "version field missing or empty: {body}",
        );
    }

    #[test]
    fn down_response_is_503_with_db_down() {
        // 503 is the right signal for readiness probes — k8s
        // will pull the pod out of LB rotation. 500 would mean
        // "this request errored" which is semantically wrong
        // for a liveness-style probe.
        let resp = build_response(false);
        assert_eq!(resp.status.as_u16(), 503);
        let v: serde_json::Value = serde_json::from_slice(&resp.body).expect("valid json");
        assert_eq!(v["ok"], false);
        assert_eq!(v["db"], "down");
    }

    #[test]
    fn response_headers_carry_json_and_no_store() {
        let resp = build_response(true);
        let get = |name: &str| {
            resp.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
                .unwrap_or("")
        };
        assert_eq!(get("content-type"), "application/json");
        // no-store is load-bearing — a cached "ok" response would
        // mask a real outage.
        assert_eq!(get("cache-control"), "no-store");
    }
}
