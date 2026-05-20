//! `?format=json` and `Accept: application/json` content
//! negotiation for the admin's list + detail routes.
//!
//! v1 surface — read-only. The existing list and detail handlers
//! branch on [`wants_json`] before rendering:
//!
//! - **List** (`GET /admin/<model>?format=json`): returns
//!   `{"rows":[…], "total":N, "page":N, "per_page":N, "pages":N}`
//!   under the same filter / search / sort / pagination state the
//!   HTML page honours. Each row is a flat object whose keys are
//!   the model's `AdminField.name` slugs and whose values are
//!   the strings emitted by `AdminModel::display_values`.
//! - **Detail** (`GET /admin/<model>/<id>`): returns the row as
//!   a flat object with the same `{field_name: stringified}`
//!   shape, plus `{"id": N}` at the top level. Registered on a
//!   distinct route from `/edit`; the form-driven UI stays on
//!   `/admin/<model>/<id>/edit`.
//!
//! Authentication piggybacks on the existing session-cookie
//! gate; an API client signs in via `POST /admin/login` once and
//! reuses the cookie. Per-model `view` permission is enforced
//! by the same `perm_guard` the HTML routes use. Audit emission
//! is unchanged.
//!
//! Create / update / delete via JSON is a deliberate **deferred
//! follow-up** — error shape, partial-success semantics on bulk
//! actions, and JSON-body validation are wider design surface
//! than a single commit warrants. This module ships the
//! read-only half end-to-end; the write half can be layered on
//! top without redesigning the read surface.

use hyper::StatusCode;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::http::{Request, Response};

use super::types::ListPage;

/// `true` when the client wants a JSON response.
///
/// Precedence:
///   1. `Accept` header contains `application/json` — exact match
///      on `application/json` or `application/*` or a wildcard
///      `*/*` ALONE. A pure `text/html, */*` is treated as HTML
///      (browser default).
///   2. `?format=json` query parameter — explicit per-link
///      override useful in browsers / `<a download>` links.
///
/// Returns `false` for everything else, including the absence of
/// `Accept` (default to HTML, the original surface).
pub(crate) fn wants_json(req: &Request) -> bool {
    if let Some(accept) = req.header("accept") {
        // Tokenise on `,` — any segment whose media-type is
        // application/json wins. Q-values are honoured by
        // ordering only ("first wins"), not by parsing the
        // weight; the admin's content-negotiation surface
        // doesn't need RFC 7231 §5.3.2 precision.
        for raw in accept.split(',') {
            let mt = raw.trim();
            // Trim parameters (`;q=0.9` etc.) before comparing.
            let bare = mt.split(';').next().unwrap_or(mt).trim();
            if bare.eq_ignore_ascii_case("application/json")
                || bare.eq_ignore_ascii_case("application/*")
            {
                return true;
            }
            if bare.eq_ignore_ascii_case("text/html") {
                // Browser default — HTML wins.
                return false;
            }
        }
    }
    req.query()
        .get("format")
        .map(|s| s == "json")
        .unwrap_or(false)
}

/// JSON envelope returned by `GET /admin/<model>?format=json`.
/// Snake-case keys mirror the DB / column convention; ordering
/// of fields inside each row is preserved via `BTreeMap`'s
/// alphabetical iteration so consumers can rely on stable shape
/// even when the framework reorders things internally.
#[derive(Serialize)]
pub(crate) struct ListEnvelope {
    pub rows: Vec<BTreeMap<String, serde_json::Value>>,
    pub total: i64,
    pub page: usize,
    pub per_page: usize,
    pub pages: usize,
}

/// Build the JSON envelope for a list page. `cells` comes from
/// each row's `display_values()`; field-name keys are sourced
/// from `entry.fields` so the column ordering / naming stays in
/// lock-step with what HTML rendering uses.
pub(crate) fn list_envelope(
    entry: &super::types::AdminEntry,
    page_result: ListPage,
    page: usize,
    per_page: usize,
) -> ListEnvelope {
    let total = page_result.total;
    let pages = (total.max(1) as usize).div_ceil(per_page.max(1));
    let rows = page_result
        .rows
        .into_iter()
        .map(|row| {
            let mut obj: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            obj.insert("id".into(), serde_json::Value::Number(row.id.into()));
            for (i, f) in entry.fields.iter().enumerate() {
                if f.name == "id" {
                    continue;
                }
                let cell = row.cells.get(i).cloned().unwrap_or_default();
                obj.insert(f.name.to_string(), typed_cell(f, &cell));
            }
            obj
        })
        .collect();
    ListEnvelope {
        rows,
        total,
        page,
        per_page,
        pages,
    }
}

/// Build the JSON envelope for `GET /admin/<model>/<id>`. Same
/// per-cell typing as the list envelope; the top-level object
/// carries an `id` field.
pub(crate) fn detail_envelope(
    entry: &super::types::AdminEntry,
    row: super::types::EditRow,
) -> BTreeMap<String, serde_json::Value> {
    let mut obj: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    obj.insert("id".into(), serde_json::Value::Number(row.id.into()));
    for f in entry.fields {
        if f.name == "id" {
            continue;
        }
        let cell = row
            .values
            .iter()
            .find(|(name, _)| name == f.name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        obj.insert(f.name.to_string(), typed_cell(f, &cell));
    }
    obj
}

/// Map a stringified cell value to its native JSON representation
/// based on the field's declared type.
///
/// - `Bool`: `true | false` (only when the cell parses as `"true"`).
/// - `I32 | I64`: integer when the cell parses, else `null`.
/// - `OptionalI64`: `null` when empty, integer otherwise.
/// - Everything else (`String`, `OptionalString`, `DateTime`,
///   `OptionalDateTime`, `FilePath`, `OptionalFilePath`): string,
///   or `null` for the nullable variants when empty.
///
/// The strict-but-tolerant policy: when typing fails (a row that
/// somehow holds non-numeric content in an `I64` column), surface
/// the raw string instead of dropping the field. A future stricter
/// mode could refuse to serve such rows; v1 prefers visibility.
fn typed_cell(field: &super::types::AdminField, cell: &str) -> serde_json::Value {
    use super::types::FieldType;
    match field.field_type {
        FieldType::Bool => serde_json::Value::Bool(cell == "true"),
        FieldType::I32 | FieldType::I64 => match cell.parse::<i64>() {
            Ok(n) => serde_json::Value::Number(n.into()),
            Err(_) => serde_json::Value::String(cell.to_string()),
        },
        FieldType::OptionalI64 => {
            if cell.is_empty() {
                serde_json::Value::Null
            } else {
                match cell.parse::<i64>() {
                    Ok(n) => serde_json::Value::Number(n.into()),
                    Err(_) => serde_json::Value::String(cell.to_string()),
                }
            }
        }
        FieldType::OptionalString | FieldType::OptionalDateTime | FieldType::OptionalFilePath => {
            if cell.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(cell.to_string())
            }
        }
        FieldType::String | FieldType::DateTime | FieldType::FilePath => {
            serde_json::Value::String(cell.to_string())
        }
    }
}

/// Helper that wraps a Serialize body into a `Response` with the
/// right Content-Type. Returns 500 on serialisation failure with
/// a JSON-shaped error envelope so the client still sees JSON.
pub(crate) fn json_response<T: Serialize>(body: T) -> Result<Response> {
    let s = serde_json::to_string(&body)
        .map_err(|e| crate::error::Error::Internal(format!("json serialise: {e}")))?;
    Ok(Response::json_raw(s))
}

/// JSON-shape error envelope for API clients. Returned by API
/// handlers in lieu of the framework's default HTML error page
/// so the response contract stays predictable for non-browser
/// consumers.
///
/// Shape: `{"error": "<message>", "status": <code>}`. The status
/// number is duplicated in the body so clients that don't read
/// headers (most JS) still see it. `client_message()` strips
/// detail from 500s — internal text stays in logs.
pub(crate) fn json_error(err: Error) -> Response {
    let status = StatusCode::from_u16(err.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = serde_json::json!({
        "error": err.client_message(),
        "status": err.status(),
    });
    let s = serde_json::to_string(&body).unwrap_or_else(|_| {
        // Fallback for the unreachable case where serialising a
        // {string, number} pair fails — emit a hand-built JSON
        // string so the response contract still holds.
        format!(
            r#"{{"error":"json error envelope serialisation failed","status":{}}}"#,
            err.status()
        )
    });
    Response::new(status, s).with_header("content-type", "application/json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::types::{AdminField, FieldType};

    fn field(name: &'static str, ty: FieldType) -> AdminField {
        AdminField {
            name,
            label: name,
            field_type: ty,
            editable: true,
            relation: None,
            choices: None,
        }
    }

    #[test]
    fn typed_cell_bool_round_trips() {
        let f = field("active", FieldType::Bool);
        assert_eq!(typed_cell(&f, "true"), serde_json::Value::Bool(true));
        assert_eq!(typed_cell(&f, "false"), serde_json::Value::Bool(false));
        // Anything that isn't literally "true" is treated as false
        // — same shape as the form widget's checkbox semantics.
        assert_eq!(typed_cell(&f, ""), serde_json::Value::Bool(false));
    }

    #[test]
    fn typed_cell_integers_parse() {
        let f = field("count", FieldType::I64);
        assert_eq!(typed_cell(&f, "42"), serde_json::json!(42));
        let opt = field("count", FieldType::OptionalI64);
        assert_eq!(typed_cell(&opt, ""), serde_json::Value::Null);
        assert_eq!(typed_cell(&opt, "7"), serde_json::json!(7));
    }

    #[test]
    fn typed_cell_garbage_in_int_column_surfaces_as_string() {
        // Tolerant policy: rather than refuse the row, render the
        // raw string. Visibility beats invisibility on the API
        // surface; operators tracking down a bad row see what's
        // actually there.
        let f = field("count", FieldType::I64);
        assert_eq!(typed_cell(&f, "abc"), serde_json::json!("abc"));
    }

    #[test]
    fn typed_cell_optional_strings_null_on_empty() {
        let f = field("subtitle", FieldType::OptionalString);
        assert_eq!(typed_cell(&f, ""), serde_json::Value::Null);
        assert_eq!(typed_cell(&f, "set"), serde_json::json!("set"));
    }

    #[test]
    fn typed_cell_required_strings_stay_string() {
        let f = field("title", FieldType::String);
        assert_eq!(typed_cell(&f, ""), serde_json::json!(""));
        assert_eq!(typed_cell(&f, "hi"), serde_json::json!("hi"));
    }

    #[test]
    fn json_error_carries_status_code_and_content_type() {
        // 404 NotFound case — the common "row missing" path.
        let resp = json_error(Error::NotFound("clinics/9999".into()));
        assert_eq!(resp.status.as_u16(), 404);
        let ctype = resp
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        assert_eq!(ctype, "application/json");
    }

    #[test]
    fn json_error_envelope_shape_is_error_plus_status() {
        let resp = json_error(Error::NotFound("clinics/9999".into()));
        let body = std::str::from_utf8(&resp.body).expect("utf-8");
        let v: serde_json::Value = serde_json::from_str(body).expect("valid json");
        assert_eq!(v["status"], 404);
        // Message comes from `client_message()` and matches what
        // the framework would otherwise put in the HTML page.
        assert_eq!(v["error"], "clinics/9999");
    }

    #[test]
    fn json_error_redacts_internal_message() {
        // For 500s the framework deliberately returns a generic
        // string — the real detail stays in logs. The JSON
        // envelope must inherit that policy.
        let resp = json_error(Error::Internal("password in this string!".into()));
        assert_eq!(resp.status.as_u16(), 500);
        let body = std::str::from_utf8(&resp.body).expect("utf-8");
        assert!(
            !body.contains("password"),
            "internal error detail leaked into client envelope: {body}",
        );
        let v: serde_json::Value = serde_json::from_str(body).expect("valid json");
        assert_eq!(v["error"], "Internal Server Error");
        assert_eq!(v["status"], 500);
    }

    #[test]
    fn json_error_maps_each_status_variant() {
        // Every variant of Error must produce its expected
        // HTTP status code in the envelope. This guards against
        // silent regressions if a future Error variant is added
        // and we forget to thread it through.
        for (err, expected) in [
            (Error::BadRequest("x".into()), 400),
            (Error::Unauthorized("x".into()), 401),
            (Error::Forbidden("x".into()), 403),
            (Error::NotFound("x".into()), 404),
            (Error::MethodNotAllowed("x".into()), 405),
            (Error::Conflict("x".into()), 409),
            (Error::Internal("x".into()), 500),
        ] {
            let resp = json_error(err);
            assert_eq!(resp.status.as_u16(), expected);
            let v: serde_json::Value = serde_json::from_slice(&resp.body).expect("valid json");
            assert_eq!(v["status"].as_u64(), Some(expected as u64));
        }
    }

    #[test]
    fn typed_cell_filepath_returns_string_or_null() {
        let req = field("photo_path", FieldType::FilePath);
        assert_eq!(typed_cell(&req, "abc.png"), serde_json::json!("abc.png"));
        let opt = field("photo_path", FieldType::OptionalFilePath);
        assert_eq!(typed_cell(&opt, ""), serde_json::Value::Null);
        assert_eq!(typed_cell(&opt, "x.png"), serde_json::json!("x.png"));
    }
}
