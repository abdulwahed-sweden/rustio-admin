//! `?format=csv` and `Accept: text/csv` content negotiation
//! for the admin's list endpoint. Sibling of
//! [`crate::admin::json_api`] — same auth gate, same column
//! ordering, same filter / search / sort / pagination state.
//!
//! Detail endpoint stays JSON-only — CSV for a single object
//! doesn't carry meaningful structure.
//!
//! Output is RFC 4180:
//!   - CRLF line terminators.
//!   - Fields containing `,`, `"`, `\r`, or `\n` are quoted.
//!   - `"` inside quoted fields doubles to `""`.
//!   - Header row uses `AdminField.label` (the human label the
//!     HTML table header shows), so the spreadsheet matches
//!     what the user saw on screen.
//!   - Values use the same per-cell stringification as the HTML
//!     list page (`row.cells[i]`) — booleans stay `true` /
//!     `false`, null `Optional*` slots become empty strings,
//!     dates render in the framework's `%Y-%m-%dT%H:%M` form.
//!
//! `Content-Disposition: attachment; filename="<admin_name>.csv"`
//! triggers a browser download rather than inline render.

use hyper::StatusCode;

use crate::error::Result;
use crate::http::{Request, Response};

use super::types::{AdminEntry, ListPage};

/// `true` when the client wants a CSV response.
///
/// Precedence:
///   1. `Accept` header lists `text/csv` ahead of `text/html` /
///      `application/json`. The first concrete media type wins.
///   2. `?format=csv` query parameter — explicit per-link
///      override useful for browsers and `<a download>` links.
///
/// Returns `false` for everything else, including the absence of
/// `Accept` (default to HTML).
pub(crate) fn wants_csv(req: &Request) -> bool {
    if let Some(accept) = req.header("accept") {
        for raw in accept.split(',') {
            let bare = raw.split(';').next().unwrap_or(raw).trim();
            if bare.eq_ignore_ascii_case("text/csv") {
                return true;
            }
            if bare.eq_ignore_ascii_case("text/html")
                || bare.eq_ignore_ascii_case("application/json")
                || bare.eq_ignore_ascii_case("application/*")
            {
                return false;
            }
        }
    }
    req.query()
        .get("format")
        .map(|s| s == "csv")
        .unwrap_or(false)
}

/// Build the CSV response body for a list page. Column order
/// mirrors `entry.fields` (with `id` pulled to the front);
/// per-cell values come straight from `row.cells`, which is the
/// same data the HTML list table renders. This keeps the export
/// in lock-step with what an operator sees on screen.
pub(crate) fn list_response(entry: &AdminEntry, page_result: ListPage) -> Result<Response> {
    let mut body = String::new();

    // Header row — `id` first, then the rest of the configured
    // fields. Use `AdminField.label` so the column header is the
    // same humanised string the HTML table shows.
    let mut header: Vec<&str> = Vec::with_capacity(entry.fields.len() + 1);
    header.push("id");
    for f in entry.fields.iter() {
        if f.name == "id" {
            continue;
        }
        header.push(f.label);
    }
    write_row(&mut body, header.iter().copied());

    for row in &page_result.rows {
        let mut cells: Vec<String> = Vec::with_capacity(entry.fields.len() + 1);
        cells.push(row.id.to_string());
        for (i, f) in entry.fields.iter().enumerate() {
            if f.name == "id" {
                continue;
            }
            cells.push(row.cells.get(i).cloned().unwrap_or_default());
        }
        write_row(&mut body, cells.iter().map(String::as_str));
    }

    let filename = format!("{}.csv", entry.admin_name);
    Ok(Response::new(StatusCode::OK, body)
        .with_header("content-type", "text/csv; charset=utf-8")
        .with_header(
            "content-disposition",
            format!("attachment; filename=\"{filename}\""),
        ))
}

fn write_row<'a, I: IntoIterator<Item = &'a str>>(out: &mut String, cells: I) {
    let mut first = true;
    for c in cells {
        if !first {
            out.push(',');
        }
        first = false;
        write_cell(out, c);
    }
    out.push_str("\r\n");
}

fn write_cell(out: &mut String, cell: &str) {
    let needs_quoting =
        cell.contains(',') || cell.contains('"') || cell.contains('\n') || cell.contains('\r');
    if !needs_quoting {
        out.push_str(cell);
        return;
    }
    out.push('"');
    for ch in cell.chars() {
        if ch == '"' {
            out.push_str("\"\"");
        } else {
            out.push(ch);
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_row(cells: &[&str]) -> String {
        let mut out = String::new();
        write_row(&mut out, cells.iter().copied());
        out
    }

    fn cell_only(s: &str) -> String {
        let mut out = String::new();
        write_cell(&mut out, s);
        out
    }

    #[test]
    fn simple_cells_are_unquoted() {
        assert_eq!(cell_only("abc"), "abc");
        assert_eq!(cell_only("123"), "123");
        assert_eq!(cell_only(""), "");
    }

    #[test]
    fn cells_with_comma_are_quoted() {
        assert_eq!(cell_only("a,b"), "\"a,b\"");
    }

    #[test]
    fn cells_with_double_quote_double_the_quote() {
        // RFC 4180: a quote inside a quoted field is escaped by
        // doubling it. `she said "hi"` becomes `"she said ""hi"""`.
        assert_eq!(cell_only(r#"she said "hi""#), r#""she said ""hi""""#);
    }

    #[test]
    fn cells_with_newlines_are_quoted() {
        assert_eq!(cell_only("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(cell_only("line1\r\nline2"), "\"line1\r\nline2\"");
    }

    #[test]
    fn row_uses_crlf_and_comma() {
        assert_eq!(render_row(&["a", "b", "c"]), "a,b,c\r\n");
        // Empty trailing cell still serialises.
        assert_eq!(render_row(&["a", "", "c"]), "a,,c\r\n");
    }

    #[test]
    fn row_quotes_only_offending_cells() {
        // Per-cell decision: mixing quoted and unquoted cells in
        // one row is the whole point of RFC 4180's grammar.
        assert_eq!(
            render_row(&["plain", "has,comma", "plain again"]),
            "plain,\"has,comma\",plain again\r\n",
        );
    }

    #[test]
    fn wants_csv_accept_header_precedence() {
        // The full test machinery requires a real Request; we
        // unit-test the precedence rules through a thin shim
        // that mimics `req.header("accept")` lookup.
        fn pick(accept: &str) -> bool {
            for raw in accept.split(',') {
                let bare = raw.split(';').next().unwrap_or(raw).trim();
                if bare.eq_ignore_ascii_case("text/csv") {
                    return true;
                }
                if bare.eq_ignore_ascii_case("text/html")
                    || bare.eq_ignore_ascii_case("application/json")
                    || bare.eq_ignore_ascii_case("application/*")
                {
                    return false;
                }
            }
            false
        }

        assert!(pick("text/csv"));
        assert!(pick("text/csv, text/html"));
        assert!(pick("text/csv;q=0.9, */*;q=0.1"));
        assert!(!pick("text/html, text/csv"));
        assert!(!pick("application/json, text/csv"));
        assert!(!pick("text/html"));
        assert!(!pick("*/*"));
    }
}
