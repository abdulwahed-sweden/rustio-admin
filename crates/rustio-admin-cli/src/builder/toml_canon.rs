//! The Builder's sole TOML emitter (Doctrine B1, §4.4 #4).
//!
//! `DESIGN_BUILDER.md` §4.1: *"Canonical serialization is enforced
//! by `cli::toml::emit_canonical`; the function is the sole emitter
//! for any Builder-written TOML."*
//!
//! The grep proof at §10.4 forbids any other file under
//! `crates/rustio-admin-cli/src/` from calling `toml::to_string`,
//! `toml_edit::*::to_string`, or otherwise constructing a TOML byte
//! stream.
//!
//! ## Canonical-form rules
//!
//! Per `DESIGN_BUILDER.md` §4.1 and §4.4:
//!
//! - Top-level keys: scalars first in **doctrine-fixed order**
//!   (`schema_version`), then tables alphabetically, then arrays of
//!   tables alphabetically.
//! - Inside any table: keys sorted alphabetically.
//! - Arrays of tables: elements emitted in their TOML-edit insertion
//!   order. Sorting *within* an array of tables is the caller's
//!   responsibility (e.g. `[[models]]` sorted by `name`).
//! - 2-space indent (TOML default).
//! - LF line endings, no trailing whitespace, single trailing LF --
//!   enforced by piping the emitter output through
//!   [`crate::builder::canonical::canonicalize`].
//! - No comments emitted. Builder-managed TOML carries no comments
//!   because they would not round-trip through replay.
//!
//! ## Round-trip property
//!
//! `cli::toml_canon::tests::canonical_round_trip` asserts that
//! parsing a canonical document and re-emitting it produces
//! byte-identical output. This is the load-bearing reproducibility
//! drift test of §10.8.

use toml_edit::{DocumentMut, Item, Table};

use crate::builder::canonical::canonicalize;

/// Errors the canonical emitter / parser can surface.
#[derive(Debug)]
pub(crate) enum TomlError {
    Parse(toml_edit::TomlError),
}

impl std::fmt::Display for TomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TomlError::Parse(e) => write!(f, "TOML parse error: {e}"),
        }
    }
}

impl std::error::Error for TomlError {}

/// Parse a TOML byte sequence into an editable document. The caller
/// then mutates it and passes the result to [`emit_canonical`].
pub(crate) fn parse(input: &str) -> Result<DocumentMut, TomlError> {
    input.parse::<DocumentMut>().map_err(TomlError::Parse)
}

/// Emit a TOML document in canonical form. Output is guaranteed
/// byte-stable across runs given identical input. Doctrine B1 +
/// §4.4 #4 enforcement primitive.
///
/// The sole TOML emitter in the CLI crate.
pub(crate) fn emit_canonical(doc: &DocumentMut) -> String {
    let mut copy = doc.clone();
    sort_recursively(copy.as_table_mut());
    let raw = copy.to_string();
    canonicalize(&raw)
}

/// Recursively sort every `Table` value within a `Table`. Arrays of
/// tables preserve element order (semantic); each element's keys
/// are sorted in turn.
fn sort_recursively(table: &mut Table) {
    table.sort_values();
    for (_, item) in table.iter_mut() {
        match item {
            Item::Table(t) => sort_recursively(t),
            Item::ArrayOfTables(aot) => {
                for entry in aot.iter_mut() {
                    sort_recursively(entry);
                }
            }
            Item::Value(_) | Item::None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml_edit::value;

    /// Build a representative document with scalars, a regular
    /// table, and an array of tables. Used to exercise the
    /// canonicalizer.
    fn fixture() -> DocumentMut {
        let mut doc = DocumentMut::new();
        // Intentionally inserted out of alphabetical order.
        doc["zeta"] = value("z");
        doc["alpha"] = value(1_i64);
        let mut project = Table::new();
        project["name"] = value("demo");
        project["created_at"] = value("2026-05-15T10:30:00Z");
        doc["project"] = Item::Table(project);
        // Array of tables -- order is semantic, NOT sorted by the
        // emitter. Caller is responsible for sorting before passing.
        let mut aot = toml_edit::ArrayOfTables::new();
        {
            let mut m = Table::new();
            m["name"] = value("Patient");
            m["table"] = value("patients");
            aot.push(m);
        }
        {
            let mut m = Table::new();
            m["name"] = value("Doctor");
            m["table"] = value("doctors");
            aot.push(m);
        }
        doc["models"] = Item::ArrayOfTables(aot);
        doc
    }

    #[test]
    fn canonical_round_trip() {
        // The load-bearing test from §10.8: emit → parse → emit
        // must produce byte-identical output.
        let first = emit_canonical(&fixture());
        let parsed = parse(&first).expect("emitter output must parse");
        let second = emit_canonical(&parsed);
        assert_eq!(first, second, "canonical form is not byte-stable");
    }

    #[test]
    fn keys_sorted_within_table() {
        let out = emit_canonical(&fixture());
        // Top-level scalars in alphabetical order.
        let alpha_pos = out.find("alpha = ").expect("alpha emitted");
        let zeta_pos = out.find("zeta = ").expect("zeta emitted");
        assert!(
            alpha_pos < zeta_pos,
            "alpha must precede zeta in canonical output:\n{out}"
        );
        // Within [project]: created_at before name (alphabetical).
        let created_at_pos = out.find("created_at = ").expect("created_at emitted");
        let name_pos = out.find("name = \"demo\"").expect("project.name emitted");
        assert!(
            created_at_pos < name_pos,
            "created_at must precede name in [project]:\n{out}"
        );
    }

    #[test]
    fn array_of_tables_preserves_insertion_order() {
        // The emitter must NOT reorder `[[models]]` entries. The
        // caller decides the canonical ordering (by name, etc.)
        // before passing the doc in.
        let out = emit_canonical(&fixture());
        let patient_pos = out.find("name = \"Patient\"").expect("Patient emitted");
        let doctor_pos = out.find("name = \"Doctor\"").expect("Doctor emitted");
        assert!(
            patient_pos < doctor_pos,
            "AOT element order must be preserved:\n{out}"
        );
    }

    #[test]
    fn output_ends_with_single_lf() {
        let out = emit_canonical(&fixture());
        assert!(out.ends_with('\n'), "output must end with LF");
        assert!(
            !out.ends_with("\n\n"),
            "output must not have trailing blank lines"
        );
    }

    #[test]
    fn output_uses_lf_line_endings_only() {
        let out = emit_canonical(&fixture());
        assert!(!out.contains('\r'), "canonical output must not contain CR");
    }

    #[test]
    fn empty_document_round_trips() {
        let doc = DocumentMut::new();
        let out = emit_canonical(&doc);
        let parsed = parse(&out).expect("empty doc parses");
        let again = emit_canonical(&parsed);
        assert_eq!(out, again);
    }
}
