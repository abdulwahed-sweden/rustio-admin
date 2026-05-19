//! Minimal hand-rolled `multipart/form-data` parser.
//!
//! Scope is intentionally narrow: the admin's file-upload widget
//! posts one form at a time, files are read into memory before
//! disk-write (no streaming), and per-part headers we care about
//! are limited to `Content-Disposition` (for `name` + `filename`)
//! and `Content-Type` (for the saved file's MIME). Anything beyond
//! that — nested multiparts, chunked transfer encoding, BOM-
//! prefixed plaintext — is out of scope; the framework's surface
//! is "small forms + file inputs," not arbitrary multipart relay.
//!
//! Reference: RFC 7578 §4. Boundaries are ASCII per the RFC; the
//! parser splits the body by `--<boundary>` and parses each part's
//! `\r\n\r\n`-separated header block ahead of the binary content.

#[cfg(test)]
use std::collections::HashMap;

/// One field parsed out of a multipart body. String parts and file
/// parts arrive in submission order via [`MultipartForm`]; the
/// caller picks them up by `name`.
#[derive(Debug, Clone)]
pub(crate) struct MultipartPart {
    pub name: String,
    /// `Some` when the part declared `filename=`; treat it as an
    /// uploaded file. `None` means a regular text field.
    pub filename: Option<String>,
    /// `Content-Type` declared on the part. Empty when absent.
    /// Reserved for the future "select on extension when none
    /// survives in filename" path; not consumed yet by the
    /// upload writer.
    #[allow(dead_code)]
    pub content_type: String,
    /// Raw bytes between the part headers and the trailing
    /// boundary. Includes neither the leading `\r\n` after the
    /// headers nor the trailing `\r\n` before the next boundary.
    pub body: Vec<u8>,
}

/// Result of parsing a full multipart body. `parts` retains
/// submission order; `text_fields` is a flat lookup for the
/// non-file parts (used by the upload code path to merge text
/// fields into the framework's existing `FormData`).
#[derive(Debug, Default)]
pub(crate) struct MultipartForm {
    pub parts: Vec<MultipartPart>,
}

impl MultipartForm {
    #[cfg(test)]
    pub fn text_fields(&self) -> HashMap<String, String> {
        let mut out = HashMap::with_capacity(self.parts.len());
        for p in &self.parts {
            if p.filename.is_none() {
                if let Ok(s) = std::str::from_utf8(&p.body) {
                    // Last value wins for duplicate names — same
                    // contract as `FormData::from_urlencoded`'s
                    // `fields` view. The multi-value view isn't
                    // wired through here; the admin's multipart
                    // surface doesn't (yet) need it.
                    out.insert(p.name.clone(), s.to_string());
                }
            }
        }
        out
    }
}

/// Errors the parser raises. Coarse — the route handler
/// translates these to a 400 with a non-specific message; the
/// detail goes to the log.
#[derive(Debug)]
pub(crate) enum MultipartError {
    MissingBoundary,
    NoOpeningBoundary,
    PartHeadersTooLong,
    MalformedPart(String),
}

impl std::fmt::Display for MultipartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBoundary => write!(f, "Content-Type missing boundary parameter"),
            Self::NoOpeningBoundary => write!(f, "body does not start with the declared boundary"),
            Self::PartHeadersTooLong => write!(f, "part headers exceeded cap"),
            Self::MalformedPart(msg) => write!(f, "malformed part: {msg}"),
        }
    }
}

impl std::error::Error for MultipartError {}

/// Extract the `boundary=<value>` parameter from a
/// `multipart/form-data; boundary=…` content-type header.
/// Tolerates surrounding whitespace and an optional `"`-quoted
/// boundary value. Returns `None` when the header isn't
/// multipart or the boundary parameter is absent / empty.
pub(crate) fn boundary_from_content_type(ct: &str) -> Option<String> {
    let lower = ct.to_ascii_lowercase();
    if !lower.starts_with("multipart/form-data") {
        return None;
    }
    for piece in ct.split(';').skip(1) {
        let piece = piece.trim();
        if let Some(rest) = piece.strip_prefix("boundary=") {
            let b = rest.trim().trim_matches('"').to_string();
            if !b.is_empty() {
                return Some(b);
            }
        }
    }
    None
}

/// Cap on the per-part header block before the parser refuses
/// — defends against pathological inputs that never terminate the
/// header section. Real browsers stay under 1 KB.
const HEADERS_MAX: usize = 16 * 1024;

/// Parse a complete multipart body. `body` is the full request
/// payload; `boundary` is the value pulled from the
/// `Content-Type` header. The parser does not stream — the caller
/// is expected to have already capped request size at a level
/// appropriate for the project.
pub(crate) fn parse_multipart(
    body: &[u8],
    boundary: &str,
) -> Result<MultipartForm, MultipartError> {
    if boundary.is_empty() {
        return Err(MultipartError::MissingBoundary);
    }
    let delim = format!("--{boundary}");
    let delim_bytes = delim.as_bytes();

    let mut cursor = find_subsequence(body, delim_bytes).ok_or(MultipartError::NoOpeningBoundary)?;
    // Skip the opening boundary itself.
    cursor += delim_bytes.len();

    let mut parts: Vec<MultipartPart> = Vec::new();
    loop {
        // After a boundary line: either "--" → end of body, or
        // "\r\n" → next part. Tolerate "\n" as well for
        // non-conformant browsers.
        if body.get(cursor..cursor + 2) == Some(b"--") {
            // Terminal boundary.
            break;
        }
        if body.get(cursor..cursor + 2) == Some(b"\r\n") {
            cursor += 2;
        } else if body.get(cursor..cursor + 1) == Some(b"\n") {
            cursor += 1;
        } else if cursor >= body.len() {
            // Body ran out without a closing `--` — accept.
            break;
        } else {
            return Err(MultipartError::MalformedPart(format!(
                "expected CRLF after boundary at byte {cursor}"
            )));
        }

        // Headers: read until "\r\n\r\n".
        let headers_end_rel = find_subsequence(&body[cursor..], b"\r\n\r\n")
            .or_else(|| find_subsequence(&body[cursor..], b"\n\n"))
            .ok_or_else(|| {
                MultipartError::MalformedPart("part headers unterminated".into())
            })?;
        if headers_end_rel > HEADERS_MAX {
            return Err(MultipartError::PartHeadersTooLong);
        }
        let header_block = &body[cursor..cursor + headers_end_rel];
        let header_terminator_len =
            if body.get(cursor + headers_end_rel..cursor + headers_end_rel + 4) == Some(b"\r\n\r\n") {
                4
            } else {
                2
            };
        cursor += headers_end_rel + header_terminator_len;

        let (name, filename, content_type) = parse_part_headers(header_block)?;

        // Body: read until the next boundary occurrence.
        let next_boundary_rel = find_subsequence(&body[cursor..], delim_bytes)
            .ok_or_else(|| MultipartError::MalformedPart("part body unterminated".into()))?;
        // Strip the trailing `\r\n` that precedes the boundary
        // (always present in well-formed multipart).
        let body_end = if next_boundary_rel >= 2
            && &body[cursor + next_boundary_rel - 2..cursor + next_boundary_rel] == b"\r\n"
        {
            next_boundary_rel - 2
        } else if next_boundary_rel >= 1
            && body[cursor + next_boundary_rel - 1] == b'\n'
        {
            next_boundary_rel - 1
        } else {
            next_boundary_rel
        };
        let part_body = body[cursor..cursor + body_end].to_vec();
        cursor += next_boundary_rel + delim_bytes.len();

        parts.push(MultipartPart {
            name,
            filename,
            content_type,
            body: part_body,
        });
    }

    Ok(MultipartForm { parts })
}

/// Parse one part's header block — `Content-Disposition`'s `name`
/// and `filename` parameters plus the optional `Content-Type`. The
/// header block is case-insensitive per RFC 7578 §4.2.
fn parse_part_headers(block: &[u8]) -> Result<(String, Option<String>, String), MultipartError> {
    let text = std::str::from_utf8(block)
        .map_err(|e| MultipartError::MalformedPart(format!("non-utf8 header: {e}")))?;
    let mut name: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut content_type = String::new();
    for line in text.split("\r\n").flat_map(|l| l.split('\n')) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-disposition:") {
            // Pull `name="…"` and (optionally) `filename="…"`.
            let raw = &line[line.find(':').unwrap_or(0) + 1..];
            name = extract_quoted_param(raw, "name=").or(name);
            filename = extract_quoted_param(raw, "filename=").or(filename);
            // `lower` was used only to bound where the header
            // name ended; the raw `line` carries the real
            // (case-preserving) parameter values.
            let _ = rest;
        } else if let Some(_rest) = lower.strip_prefix("content-type:") {
            content_type = line[line.find(':').unwrap_or(0) + 1..].trim().to_string();
        }
    }
    let name = name
        .ok_or_else(|| MultipartError::MalformedPart("part missing Content-Disposition name".into()))?;
    Ok((name, filename, content_type))
}

/// `key=value-with-optional-quotes` extractor. Looks for `key=`
/// case-insensitively; reads the value until the next `;`, `\r`,
/// or end-of-line. Strips surrounding `"` if present. Returns
/// `None` when the key isn't found or the value is empty.
fn extract_quoted_param(haystack: &str, key: &str) -> Option<String> {
    let needle_lower = key.to_ascii_lowercase();
    let hay_lower = haystack.to_ascii_lowercase();
    let start = hay_lower.find(&needle_lower)? + key.len();
    let rest = &haystack[start..];
    let value: String = if let Some(rest) = rest.strip_prefix('"') {
        // Quoted — read until the closing `"`.
        let end = rest.find('"')?;
        rest[..end].to_string()
    } else {
        // Unquoted — read until `;`, `\r`, `\n`, or eos.
        rest.chars()
            .take_while(|c| *c != ';' && *c != '\r' && *c != '\n')
            .collect::<String>()
            .trim()
            .to_string()
    };
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// `memchr`-style needle search without a dep. Returns the first
/// byte offset in `haystack` where `needle` matches, or `None`.
/// Sufficient for boundary scanning on typical request sizes; not
/// optimised for adversarial input.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_extracts_simple() {
        assert_eq!(
            boundary_from_content_type("multipart/form-data; boundary=abc"),
            Some("abc".into())
        );
    }

    #[test]
    fn boundary_extracts_quoted() {
        assert_eq!(
            boundary_from_content_type("multipart/form-data; boundary=\"a-b-c\""),
            Some("a-b-c".into())
        );
    }

    #[test]
    fn boundary_rejects_non_multipart() {
        assert_eq!(boundary_from_content_type("application/json"), None);
        assert_eq!(
            boundary_from_content_type("application/x-www-form-urlencoded"),
            None
        );
    }

    #[test]
    fn boundary_case_insensitive_prefix() {
        assert_eq!(
            boundary_from_content_type("Multipart/Form-Data; boundary=ZZ"),
            Some("ZZ".into())
        );
    }

    #[test]
    fn parse_simple_text_field() {
        // RFC-shaped body with one text field and the closing
        // boundary. Sample boundary is `xxx`.
        let body = b"--xxx\r\n\
Content-Disposition: form-data; name=\"title\"\r\n\r\n\
Hello world\r\n\
--xxx--\r\n";
        let parsed = parse_multipart(body, "xxx").unwrap();
        assert_eq!(parsed.parts.len(), 1);
        assert_eq!(parsed.parts[0].name, "title");
        assert!(parsed.parts[0].filename.is_none());
        assert_eq!(parsed.parts[0].body, b"Hello world");
        assert_eq!(parsed.text_fields().get("title").unwrap(), "Hello world");
    }

    #[test]
    fn parse_file_part() {
        let body = b"--xxx\r\n\
Content-Disposition: form-data; name=\"photo\"; filename=\"x.png\"\r\n\
Content-Type: image/png\r\n\r\n\
\x89PNG\r\n\
--xxx--\r\n";
        let parsed = parse_multipart(body, "xxx").unwrap();
        assert_eq!(parsed.parts.len(), 1);
        let p = &parsed.parts[0];
        assert_eq!(p.name, "photo");
        assert_eq!(p.filename.as_deref(), Some("x.png"));
        assert_eq!(p.content_type, "image/png");
        // Body is the raw bytes between `\r\n\r\n` and the
        // trailing `\r\n--xxx--`.
        assert_eq!(p.body, b"\x89PNG");
    }

    #[test]
    fn parse_mixed_text_and_file() {
        let body = b"--xxx\r\n\
Content-Disposition: form-data; name=\"title\"\r\n\r\n\
hello\r\n\
--xxx\r\n\
Content-Disposition: form-data; name=\"avatar\"; filename=\"a.txt\"\r\n\
Content-Type: text/plain\r\n\r\n\
inside\r\n\
--xxx--\r\n";
        let parsed = parse_multipart(body, "xxx").unwrap();
        assert_eq!(parsed.parts.len(), 2);
        assert_eq!(parsed.parts[0].name, "title");
        assert!(parsed.parts[0].filename.is_none());
        assert_eq!(parsed.parts[0].body, b"hello");
        assert_eq!(parsed.parts[1].name, "avatar");
        assert_eq!(parsed.parts[1].filename.as_deref(), Some("a.txt"));
        assert_eq!(parsed.parts[1].body, b"inside");
    }

    #[test]
    fn parse_rejects_no_opening_boundary() {
        let body = b"hello no boundary anywhere\r\n";
        let err = parse_multipart(body, "xxx").unwrap_err();
        assert!(matches!(err, MultipartError::NoOpeningBoundary));
    }

    #[test]
    fn parse_rejects_unterminated_headers() {
        let body = b"--xxx\r\nContent-Disposition: form-data; name=\"x\"";
        // Headers never get \r\n\r\n.
        let err = parse_multipart(body, "xxx").unwrap_err();
        assert!(matches!(err, MultipartError::MalformedPart(_)));
    }

    #[test]
    fn extract_param_handles_quotes_and_unquoted() {
        assert_eq!(
            extract_quoted_param(" form-data; name=\"x\"; filename=\"y.png\"", "name="),
            Some("x".into())
        );
        assert_eq!(
            extract_quoted_param(" form-data; name=plain; whatever", "name="),
            Some("plain".into())
        );
        assert_eq!(extract_quoted_param(" something else", "name="), None);
    }
}
