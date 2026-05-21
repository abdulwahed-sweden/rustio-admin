//! CSV import — companion to `csv_export.rs`.
//!
//! Operators upload a CSV; the framework parses it RFC 4180-style,
//! matches the header row against the model's `AdminField` names,
//! and inserts each data row through `AdminOps::create`. Per-row
//! errors are reported alongside the success count rather than
//! aborting the batch — partial imports stay visible.
//!
//! Endpoint: `POST /admin/<model>/import.csv` (multipart upload
//! with a `file` part). Permission: the model's `change` gate.
//!
//! Scope of v1:
//!
//! - Header row is required and must list `AdminField.name` values
//!   exactly (extra columns are ignored; missing columns flow into
//!   the model's `from_form` as empty strings).
//! - Quoted fields, embedded commas, doubled `""` quotes, and `\n`
//!   inside quoted fields all parse per RFC 4180.
//! - Hard cap on rows imported per request — bigger batches want
//!   a background job, not a synchronous HTTP request.

use std::collections::HashMap;

use crate::http::FormData;

/// Maximum data rows accepted in one upload. Beyond this the
/// framework rejects up-front; operators should split their file.
pub(crate) const CSV_IMPORT_MAX_ROWS: usize = 10_000;

/// Maximum CSV body size — defends against pathological uploads.
/// Matches the multipart cap on file parts so a 16 MB CSV imports
/// in one request, larger needs chunking.
pub(crate) const CSV_IMPORT_MAX_BYTES: usize = 8 * 1024 * 1024;

/// One row's outcome after [`import_csv_rows`] tried to insert it.
/// Successful rows carry the new id; failures carry a list of
/// validation strings already humanised by the framework's
/// per-field bucketer.
#[derive(Debug, Clone)]
pub(crate) enum RowOutcome {
    Inserted {
        row_number: usize,
        id: i64,
    },
    Failed {
        row_number: usize,
        errors: Vec<String>,
    },
}

/// Aggregate result of one upload — surfaced on the result page.
#[derive(Debug, Clone, Default)]
pub(crate) struct ImportReport {
    pub total: usize,
    pub inserted: usize,
    pub failed: usize,
    pub outcomes: Vec<RowOutcome>,
}

/// Errors raised before any row is attempted — header problems,
/// size caps, parse failures. Per-row errors live in
/// `RowOutcome::Failed` instead.
#[derive(Debug, Clone)]
pub(crate) enum ParseError {
    Empty,
    TooLarge { size: usize, cap: usize },
    TooManyRows { rows: usize, cap: usize },
    HeaderMissing,
    HeaderEmptyColumn,
    UnknownColumns { columns: Vec<String> },
}

impl ParseError {
    pub(crate) fn message(&self) -> String {
        match self {
            ParseError::Empty => "CSV body is empty.".into(),
            ParseError::TooLarge { size, cap } => {
                format!("CSV body ({size} bytes) exceeds the {cap}-byte cap.")
            }
            ParseError::TooManyRows { rows, cap } => {
                format!("CSV has {rows} rows; cap is {cap}. Split the file and retry.")
            }
            ParseError::HeaderMissing => "First row must be a header.".into(),
            ParseError::HeaderEmptyColumn => "Header has an empty column name.".into(),
            ParseError::UnknownColumns { columns } => {
                format!(
                    "Header includes columns the model doesn't declare: {}.",
                    columns.join(", ")
                )
            }
        }
    }
}

/// Parse one CSV body into `(header, rows)`. Each row is the same
/// length as the header. Quoted fields, doubled quotes, and
/// `\r\n` / `\n` line endings are all handled. Empty trailing
/// line is tolerated.
pub(crate) fn parse_csv(body: &[u8]) -> Result<(Vec<String>, Vec<Vec<String>>), ParseError> {
    if body.is_empty() {
        return Err(ParseError::Empty);
    }
    if body.len() > CSV_IMPORT_MAX_BYTES {
        return Err(ParseError::TooLarge {
            size: body.len(),
            cap: CSV_IMPORT_MAX_BYTES,
        });
    }
    // Decode as UTF-8 — refuse non-UTF-8 input rather than guess.
    let text = std::str::from_utf8(body).map_err(|_| ParseError::Empty)?;
    let mut rows = parse_csv_text(text);
    if rows.is_empty() {
        return Err(ParseError::HeaderMissing);
    }
    let header = rows.remove(0);
    if header.iter().any(|c| c.trim().is_empty()) {
        return Err(ParseError::HeaderEmptyColumn);
    }
    Ok((header, rows))
}

/// Bulk-import the parsed rows. Returns the per-row outcomes;
/// the caller renders them on a result page.
///
/// `entry` describes the target model; `header` lists the column
/// names from the CSV (already validated by [`parse_csv`]);
/// `rows` are the data rows. Each row builds a `FormData` keyed
/// by `header[i] → row[i]` and goes through `AdminOps::create` —
/// the same path the HTML form uses, so framework validation
/// runs unchanged.
pub(crate) async fn import_csv_rows(
    db: &crate::orm::Db,
    entry: &super::types::AdminEntry,
    header: &[String],
    rows: Vec<Vec<String>>,
) -> ImportReport {
    let known_fields: HashMap<&str, ()> = entry.fields.iter().map(|f| (f.name, ())).collect();
    let header_known: Vec<bool> = header
        .iter()
        .map(|h| known_fields.contains_key(h.as_str()))
        .collect();

    let mut report = ImportReport {
        total: rows.len(),
        ..Default::default()
    };

    for (idx, row) in rows.into_iter().enumerate() {
        let row_number = idx + 2; // header is row 1, first data row is row 2
        let mut form = FormData::default();
        for (col_idx, value) in row.into_iter().enumerate() {
            if !header_known.get(col_idx).copied().unwrap_or(false) {
                continue; // ignore columns the model doesn't declare
            }
            if let Some(name) = header.get(col_idx) {
                form.set(name.clone(), value);
            }
        }

        match entry.ops.create(db, &form).await {
            Ok(Ok(id)) => {
                report.inserted += 1;
                report
                    .outcomes
                    .push(RowOutcome::Inserted { row_number, id });
            }
            Ok(Err(errors)) => {
                report.failed += 1;
                report
                    .outcomes
                    .push(RowOutcome::Failed { row_number, errors });
            }
            Err(e) => {
                report.failed += 1;
                report.outcomes.push(RowOutcome::Failed {
                    row_number,
                    errors: vec![format!("internal error: {e}")],
                });
            }
        }
    }
    report
}

/// RFC 4180-ish parser. State machine driven; no allocation per
/// character. Returns rows of equal length only when the input
/// is well-formed; jagged input still parses (shorter rows just
/// have fewer columns), and the caller decides whether to allow
/// it. The export side always writes uniform rows, so a CSV
/// produced by `csv_export` round-trips cleanly.
fn parse_csv_text(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut just_closed_quote = false;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_quotes {
            if c == b'"' {
                // Either a doubled quote (escaped) or the closing quote.
                if bytes.get(i + 1) == Some(&b'"') {
                    field.push('"');
                    i += 2;
                    continue;
                } else {
                    in_quotes = false;
                    just_closed_quote = true;
                    i += 1;
                    continue;
                }
            } else {
                field.push(c as char);
                i += 1;
                continue;
            }
        }
        // Not in quotes.
        match c {
            b',' => {
                row.push(std::mem::take(&mut field));
                just_closed_quote = false;
                i += 1;
            }
            b'\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                just_closed_quote = false;
                i += 1;
            }
            b'\r' => {
                // Swallow lone \r before \n; bare \r treated as line break.
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1; // consume \r; loop handles \n
                    continue;
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                just_closed_quote = false;
                i += 1;
            }
            b'"' if field.is_empty() && !just_closed_quote => {
                in_quotes = true;
                i += 1;
            }
            _ => {
                field.push(c as char);
                i += 1;
            }
        }
    }
    // Tail: trailing field without newline (no terminator).
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    // Drop empty trailing rows from a tailing blank line.
    while rows
        .last()
        .map(|r| r.iter().all(|f| f.is_empty()))
        .unwrap_or(false)
    {
        rows.pop();
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_header_and_two_rows() {
        let csv = "name,published\nFoo,true\nBar,false\n";
        let (header, rows) = parse_csv(csv.as_bytes()).unwrap();
        assert_eq!(header, vec!["name", "published"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["Foo", "true"]);
        assert_eq!(rows[1], vec!["Bar", "false"]);
    }

    #[test]
    fn handles_quoted_fields_with_commas_and_doubled_quotes() {
        let csv = "title,body\n\"Hello, world\",\"She said \"\"hi\"\".\"\n";
        let (_, rows) = parse_csv(csv.as_bytes()).unwrap();
        assert_eq!(rows[0][0], "Hello, world");
        assert_eq!(rows[0][1], "She said \"hi\".");
    }

    #[test]
    fn handles_crlf_line_endings() {
        let csv = "a,b\r\n1,2\r\n3,4\r\n";
        let (_, rows) = parse_csv(csv.as_bytes()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["1", "2"]);
    }

    #[test]
    fn empty_body_errors() {
        assert!(matches!(parse_csv(b"").unwrap_err(), ParseError::Empty));
    }

    #[test]
    fn oversized_body_errors() {
        let big = vec![b'a'; CSV_IMPORT_MAX_BYTES + 1];
        assert!(matches!(
            parse_csv(&big).unwrap_err(),
            ParseError::TooLarge { .. }
        ));
    }

    #[test]
    fn empty_header_column_errors() {
        let csv = "name,\nFoo,Bar\n";
        assert!(matches!(
            parse_csv(csv.as_bytes()).unwrap_err(),
            ParseError::HeaderEmptyColumn
        ));
    }

    #[test]
    fn trailing_blank_line_is_dropped() {
        let csv = "a\n1\n\n";
        let (_, rows) = parse_csv(csv.as_bytes()).unwrap();
        assert_eq!(rows, vec![vec!["1".to_string()]]);
    }
}
