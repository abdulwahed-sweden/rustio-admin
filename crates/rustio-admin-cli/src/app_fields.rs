//! `rustio-admin startapp --field` parser, validator, and renderer.
//!
//! PR 2.1 of the FTUX redesign. `rustio-admin startapp` accepts a closed
//! vocabulary of field declarations via repeatable `--field
//! <name>:<type>` flags so the scaffold can produce a real model
//! file + matching migration in one CLI breath -- without growing a
//! schema DSL.
//!
//! Discipline (`DESIGN_ONBOARDING.md` §6, PR 2.1 spec §3 / §7.1):
//!
//! - The vocabulary is closed: `str / text / int / bigint / float /
//!   decimal / bool / timestamp / date / time / uuid / email / phone /
//!   json / fk:<Model>`. Fifteen tokens, nothing more. Unknown tokens
//!   fail with the four-part error shape from PR 1.3.
//! - No nullability, no defaults beyond the table below, no
//!   indexes / uniqueness / constraints in the flag syntax. Users
//!   edit the generated migration to add those.
//! - `timestamp` does NOT default to `NOW()`; `json` does NOT
//!   default to `'{}'`. Either would imply created_at / settings
//!   semantics that don't fit every use. Users add defaults by
//!   editing the migration.
//! - `bool` does default to `FALSE` -- it's the genuine neutral for
//!   a boolean and avoids forcing every insert to specify it.
//! - The verb stays a scaffold helper, not an ORM front-end.
//!
//! Defaults table -- the human-readable mirror of [`FieldKind::spec`],
//! which is the programmatic single source of truth the renderer
//! reads from. Keep the two in sync when adding a kind:
//!
//! | Token         | Rust type           | SQL column                          |
//! |---------------|---------------------|-------------------------------------|
//! | `str`         | `String`            | `TEXT NOT NULL`                     |
//! | `text`        | `String`            | `TEXT NOT NULL`                     |
//! | `int`         | `i32`               | `INTEGER NOT NULL`                  |
//! | `bigint`      | `i64`               | `BIGINT NOT NULL`                   |
//! | `float`       | `f64`               | `DOUBLE PRECISION NOT NULL`         |
//! | `decimal`     | `Decimal`           | `NUMERIC NOT NULL`                  |
//! | `bool`        | `bool`              | `BOOLEAN NOT NULL DEFAULT FALSE`    |
//! | `timestamp`   | `DateTime<Utc>`     | `TIMESTAMPTZ NOT NULL`              |
//! | `date`        | `NaiveDate`         | `DATE NOT NULL`                     |
//! | `time`        | `NaiveTime`         | `TIME NOT NULL`                     |
//! | `uuid`        | `Uuid`              | `UUID NOT NULL`                     |
//! | `email`       | `String` *(+attr)*  | `TEXT NOT NULL`                     |
//! | `phone`       | `String` *(+attr)*  | `TEXT NOT NULL`                     |
//! | `json`        | `serde_json::Value` | `JSONB NOT NULL`                    |
//! | `fk:<Model>`  | `i64`               | `BIGINT NOT NULL REFERENCES <models>(id)` |

use crate::ui::OnboardingError;

/// One parsed `--field` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) kind: FieldKind,
}

/// The closed type vocabulary. Any new variant here is a scope
/// expansion of PR 2.1 and requires its own PR + motivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Str,
    Text,
    Email,
    Phone,
    Int,
    Bigint,
    Float,
    Decimal,
    Bool,
    Timestamp,
    Date,
    Time,
    Uuid,
    Json,
    /// Target model name in CamelCase (e.g. `"Doctor"`).
    Fk(String),
}

impl FieldKind {
    /// Comma-separated list for error messages.
    fn vocabulary_list() -> &'static str {
        "str, text, int, bigint, bool, timestamp, json, float, date, time, decimal, uuid, email, phone, fk:<Model>"
    }
}

/// Parse a single `name:type` (or `name:fk:Model`) declaration into
/// a [`Field`]. Returns the four-part [`OnboardingError`] shape from
/// PR 1.3 on any validation failure -- bad name, bad type token,
/// missing FK target, etc.
pub(crate) fn parse_field(input: &str) -> Result<Field, OnboardingError> {
    let (name, type_str) = match input.split_once(':') {
        Some(parts) => parts,
        None => {
            return Err(OnboardingError {
                problem: format!("`{input}` is not a valid `--field` value."),
                why: "Expected format: `<name>:<type>`. Example: `email:str`.".into(),
                fix: "Re-run with the colon-separated form.".into(),
                retry: format!("rustio-admin startapp <name> --field {input}:str"),
                details: None,
            });
        }
    };

    validate_field_name(name).map_err(|e| field_name_error(name, e))?;
    let kind = parse_kind(type_str).map_err(|e| field_type_error(type_str, e))?;

    Ok(Field {
        name: name.to_string(),
        kind,
    })
}

/// Reject duplicate field names across a `--field` list. The
/// generated Rust struct refuses duplicate fields anyway, but
/// surfacing it at parse time produces a calmer error than a
/// compilation failure later.
pub(crate) fn validate_unique_names(fields: &[Field]) -> Result<(), OnboardingError> {
    for i in 0..fields.len() {
        for j in (i + 1)..fields.len() {
            if fields[i].name == fields[j].name {
                return Err(OnboardingError {
                    problem: format!("Field `{}` declared twice.", fields[i].name),
                    why: "Each field on a model must have a unique name.".into(),
                    fix: "Rename one of the duplicates.".into(),
                    retry: "(re-run the command with distinct field names)".into(),
                    details: None,
                });
            }
        }
    }
    Ok(())
}

fn parse_kind(type_str: &str) -> Result<FieldKind, &'static str> {
    if let Some(rest) = type_str.strip_prefix("fk:") {
        if rest.is_empty() {
            return Err("fk_missing_target");
        }
        if rest.contains(':') {
            return Err("fk_extra_segment");
        }
        validate_camel_case(rest).map_err(|_| "fk_bad_target")?;
        return Ok(FieldKind::Fk(rest.to_string()));
    }
    if type_str == "fk" {
        return Err("fk_missing_target");
    }
    match type_str {
        "str" => Ok(FieldKind::Str),
        "text" => Ok(FieldKind::Text),
        "email" => Ok(FieldKind::Email),
        "phone" => Ok(FieldKind::Phone),
        "int" => Ok(FieldKind::Int),
        "bigint" => Ok(FieldKind::Bigint),
        "float" => Ok(FieldKind::Float),
        "decimal" => Ok(FieldKind::Decimal),
        "bool" => Ok(FieldKind::Bool),
        "timestamp" => Ok(FieldKind::Timestamp),
        "date" => Ok(FieldKind::Date),
        "time" => Ok(FieldKind::Time),
        "uuid" => Ok(FieldKind::Uuid),
        "json" => Ok(FieldKind::Json),
        _ => Err("unknown_type"),
    }
}

fn field_name_error(name: &str, code: &str) -> OnboardingError {
    match code {
        "empty" => OnboardingError {
            problem: "Field name is empty.".into(),
            why: "Expected format: `<name>:<type>`. The portion before the colon is the field name.".into(),
            fix: "Re-run with a non-empty name, e.g. `--field email:str`.".into(),
            retry: "(re-run with a valid `--field`)".into(),
            details: None,
        },
        "starts_with_digit" => OnboardingError {
            problem: format!("Field name `{name}` starts with a digit."),
            why: "Rust struct field names cannot begin with a digit.".into(),
            fix: format!("Pick a name that starts with a letter or underscore, e.g. `{name}_field` or `count_{name}`."),
            retry: "(re-run with a valid field name)".into(),
            details: None,
        },
        "bad_chars" => OnboardingError {
            problem: format!("Field name `{name}` contains invalid characters."),
            why: "Field names use ASCII lowercase letters, digits, and `_` only (snake_case).".into(),
            fix: "Re-run with a snake_case name, e.g. `full_name`, `is_active`, `user_id`.".into(),
            retry: "(re-run with a valid field name)".into(),
            details: None,
        },
        "rust_keyword" => OnboardingError {
            problem: format!("`{name}` is a reserved Rust identifier and cannot be a field name."),
            why: "Rust forbids using keywords as struct field names.".into(),
            fix: format!("Pick a non-reserved name, e.g. `{name}_` or a synonym (e.g. `class` -> `classroom`)."),
            retry: format!("(re-run with `--field {name}_:<type>` or a renamed alternative)"),
            details: None,
        },
        _ => OnboardingError {
            problem: format!("Field name `{name}` is invalid."),
            why: "Field names use ASCII lowercase letters, digits, and `_` only (snake_case), must not start with a digit, and cannot be a Rust keyword.".into(),
            fix: "Re-run with a valid snake_case name.".into(),
            retry: "(re-run with a valid field name)".into(),
            details: None,
        },
    }
}

fn field_type_error(type_str: &str, code: &str) -> OnboardingError {
    match code {
        "unknown_type" => OnboardingError {
            problem: format!("`{type_str}` is not a valid field type."),
            why: format!("Accepted types are: {}.", FieldKind::vocabulary_list()),
            fix: format!("Re-run with one of those, e.g. `--field <name>:str`. (You passed `{type_str}`.)"),
            retry: "(re-run with a valid type token)".into(),
            details: None,
        },
        "fk_missing_target" => OnboardingError {
            problem: "`fk` requires a target model name.".into(),
            why: "Expected format: `fk:<Model>` where `<Model>` is the CamelCase name of an existing model (e.g. `fk:Doctor`).".into(),
            fix: "Re-run with the model name, e.g. `--field patient:fk:Patient`.".into(),
            retry: "(re-run with `fk:<Model>`)".into(),
            details: None,
        },
        "fk_extra_segment" => OnboardingError {
            problem: format!("`{type_str}` has too many colon-separated segments."),
            why: "Expected exactly: `fk:<Model>`. No further qualifiers are supported (no `fk:Model:cascade`, `fk:Model:nullable`, etc.).".into(),
            fix: "Re-run with just `fk:<Model>`; edit the generated migration if you need `ON DELETE`/`ON UPDATE` clauses.".into(),
            retry: "(re-run with `fk:<Model>`)".into(),
            details: None,
        },
        "fk_bad_target" => OnboardingError {
            problem: "FK target model name must be in CamelCase.".into(),
            why: "Rust struct names are CamelCase; the FK references the struct's table by its name (e.g. `Patient` -> `patients`).".into(),
            fix: "Re-run with the CamelCase form, e.g. `fk:Patient` (not `fk:patient` or `fk:my_patient`).".into(),
            retry: "(re-run with a CamelCase model name)".into(),
            details: None,
        },
        _ => OnboardingError {
            problem: format!("`{type_str}` is not a valid field type."),
            why: format!("Accepted types are: {}.", FieldKind::vocabulary_list()),
            fix: format!("Re-run with one of those, e.g. `--field <name>:str`. (You passed `{type_str}`.)"),
            retry: "(re-run with a valid type token)".into(),
            details: None,
        },
    }
}

fn validate_field_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("empty");
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("starts_with_digit");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err("bad_chars");
    }
    if is_rust_keyword(name) {
        return Err("rust_keyword");
    }
    Ok(())
}

fn validate_camel_case(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("empty");
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_uppercase() {
        return Err("must_start_upper");
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("bad_chars");
    }
    Ok(())
}

/// Comprehensive Rust strict-keyword list (covers Rust 2015, 2018,
/// and 2021 editions). Reserved-for-future keywords are deliberately
/// excluded (those produce warnings, not errors, and rejecting them
/// would surprise users using a valid identifier).
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "yield"
            // Reserved-but-effectively-illegal-as-field-names:
            | "_"
    )
}

// -----------------------------------------------------------------
// Renderer -- turns Vec<Field> into the placeholder substrings the
// `model_with_fields.rs.tmpl` and `migration_with_fields.sql.tmpl`
// templates consume.
// -----------------------------------------------------------------

/// All template substitution values for the with-fields path,
/// rendered once from the parsed field list. Each member ends up
/// inserted verbatim into its template placeholder.
pub(crate) struct Render {
    /// `use chrono::{DateTime, Utc};` lines, joined with `\n` and
    /// only present when any field needs them. Always includes the
    /// core `rustio_admin` import.
    pub(crate) imports: String,
    /// One line per field, indented 4 spaces, e.g.
    /// `    pub name: String,`. Joined with `\n`.
    pub(crate) struct_fields: String,
    /// SQL `, <col> <DDL>` segments, each prefixed with `,\n    `,
    /// to splice after `id BIGSERIAL PRIMARY KEY` in the CREATE
    /// TABLE.
    pub(crate) column_decls_sql: String,
    /// Quoted column list for `COLUMNS` -- includes `"id"` first.
    pub(crate) columns_literal: String,
    /// Quoted column list for `INSERT_COLUMNS` -- excludes `id`.
    pub(crate) insert_columns_literal: String,
    /// Per-field `name: row.get_*("name")?,` lines, indented 12
    /// spaces (to match the `Ok(Self { ... })` block).
    pub(crate) from_row_assignments: String,
    /// Comma-separated `.into()` expressions for `insert_values`.
    pub(crate) insert_values_expr: String,
    /// Quoted list for `ModelAdmin::list_display`.
    pub(crate) list_display_literal: String,
    /// Quoted list for `ModelAdmin::search_fields` -- text fields
    /// (`str`, `text`) only; `&[]` when there are none.
    pub(crate) search_fields_literal: String,
}

pub(crate) fn render(fields: &[Field]) -> Render {
    // One descriptor per field, resolved once. Every member below
    // reads from these specs -- `spec()` is the single source of
    // truth for the Rust type, DDL, row getter, insert form, import
    // needs, and search eligibility of each kind.
    let specs: Vec<FieldSpec> = fields.iter().map(|f| f.kind.spec()).collect();

    // The core import is always present. Any extra `use` line a
    // field's spec requires (e.g. chrono for `timestamp`) is inserted
    // *before* it, first-seen order preserved and de-duplicated.
    let mut import_lines: Vec<&'static str> =
        vec!["use rustio_admin::{Error, Model, ModelAdmin, Row, RustioAdmin, Value};"];
    let mut extra_imports: Vec<&'static str> = Vec::new();
    for spec in &specs {
        if let Some(imp) = spec.needs_import {
            if !extra_imports.contains(&imp) {
                extra_imports.push(imp);
            }
        }
    }
    for (i, imp) in extra_imports.into_iter().enumerate() {
        import_lines.insert(i, imp);
    }

    let struct_fields = fields
        .iter()
        .zip(&specs)
        .map(|(f, s)| match s.field_attr {
            Some(attr) => format!("    {attr}\n    pub {}: {},", f.name, s.rust_type),
            None => format!("    pub {}: {},", f.name, s.rust_type),
        })
        .collect::<Vec<_>>()
        .join("\n");

    let column_decls_sql = fields
        .iter()
        .zip(&specs)
        .map(|(f, s)| format!(",\n    {} {}", f.name, s.sql_decl))
        .collect::<String>();

    let mut columns: Vec<String> = vec!["\"id\"".into()];
    columns.extend(fields.iter().map(|f| format!("\"{}\"", f.name)));
    let columns_literal = columns.join(", ");

    let insert_columns_literal = fields
        .iter()
        .map(|f| format!("\"{}\"", f.name))
        .collect::<Vec<_>>()
        .join(", ");

    let from_row_assignments = fields
        .iter()
        .zip(&specs)
        .map(|(f, s)| {
            format!(
                "            {}: row.{}(\"{}\")?,",
                f.name, s.row_getter, f.name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let insert_values_expr = fields
        .iter()
        .zip(&specs)
        .map(|(f, s)| {
            if s.insert_needs_clone {
                format!("self.{}.clone().into()", f.name)
            } else {
                format!("self.{}.into()", f.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    let list_display_literal = columns.join(", ");

    let search_fields: Vec<String> = fields
        .iter()
        .zip(&specs)
        .filter(|(_, s)| s.is_text_search)
        .map(|(f, _)| format!("\"{}\"", f.name))
        .collect();
    let search_fields_literal = search_fields.join(", ");

    Render {
        imports: import_lines.join("\n"),
        struct_fields,
        column_decls_sql,
        columns_literal,
        insert_columns_literal,
        from_row_assignments,
        insert_values_expr,
        list_display_literal,
        search_fields_literal,
    }
}

/// Every codegen fact about one [`FieldKind`], resolved in a single
/// place. This is the programmatic single source of truth the
/// renderer reads from -- it replaces the former parallel
/// `rust_type` / `sql_decl` / `row_getter` / `insert_value_expr`
/// match arms, so adding a field type is one descriptor, not four
/// edits that can silently drift. The doc-comment defaults table at
/// the top of this module mirrors these values for humans.
struct FieldSpec {
    /// Rust type rendered into the generated struct field.
    rust_type: &'static str,
    /// PostgreSQL column DDL (everything after the column name) for
    /// the `CREATE TABLE`. Owned because `Fk` interpolates its target
    /// table name.
    sql_decl: String,
    /// `orm::Row` accessor the generated `from_row` calls.
    row_getter: &'static str,
    /// Whether the `insert_values` expression needs `.clone()` before
    /// `.into()`: `true` for non-`Copy` types (`String`, json
    /// `Value`), `false` for `Copy` types (numbers, `bool`, chrono
    /// `DateTime<Utc>`, FK ids).
    insert_needs_clone: bool,
    /// Whether this kind feeds `ModelAdmin::search_fields` (text-like
    /// columns only).
    is_text_search: bool,
    /// Extra `use` line the generated model needs for this kind, if
    /// any (e.g. `timestamp` pulls in chrono).
    needs_import: Option<&'static str>,
    /// Field-level attribute line emitted immediately above the struct
    /// field, if any. `email` / `phone` carry
    /// `#[rustio(format = "...")]` so the derive macro -- which only
    /// sees the `String` Rust type -- knows to apply the matching
    /// widget and validator. Indentation is added by the renderer.
    field_attr: Option<&'static str>,
}

impl FieldKind {
    /// Resolve this kind's [`FieldSpec`] -- the one match the whole
    /// renderer pivots on.
    fn spec(&self) -> FieldSpec {
        match self {
            FieldKind::Str | FieldKind::Text => FieldSpec {
                rust_type: "String",
                sql_decl: "TEXT NOT NULL".into(),
                row_getter: "get_string",
                insert_needs_clone: true,
                is_text_search: true,
                needs_import: None,
                field_attr: None,
            },
            FieldKind::Email => FieldSpec {
                rust_type: "String",
                sql_decl: "TEXT NOT NULL".into(),
                row_getter: "get_string",
                insert_needs_clone: true,
                is_text_search: true,
                needs_import: None,
                field_attr: Some("#[rustio(format = \"email\")]"),
            },
            FieldKind::Phone => FieldSpec {
                rust_type: "String",
                sql_decl: "TEXT NOT NULL".into(),
                row_getter: "get_string",
                insert_needs_clone: true,
                is_text_search: true,
                needs_import: None,
                field_attr: Some("#[rustio(format = \"phone\")]"),
            },
            FieldKind::Int => FieldSpec {
                rust_type: "i32",
                sql_decl: "INTEGER NOT NULL".into(),
                row_getter: "get_i32",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: None,
                field_attr: None,
            },
            FieldKind::Bigint => FieldSpec {
                rust_type: "i64",
                sql_decl: "BIGINT NOT NULL".into(),
                row_getter: "get_i64",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: None,
                field_attr: None,
            },
            FieldKind::Float => FieldSpec {
                rust_type: "f64",
                sql_decl: "DOUBLE PRECISION NOT NULL".into(),
                row_getter: "get_f64",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: None,
                field_attr: None,
            },
            FieldKind::Decimal => FieldSpec {
                rust_type: "Decimal",
                sql_decl: "NUMERIC NOT NULL".into(),
                row_getter: "get_decimal",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: Some("use rust_decimal::Decimal;"),
                field_attr: None,
            },
            FieldKind::Bool => FieldSpec {
                rust_type: "bool",
                sql_decl: "BOOLEAN NOT NULL DEFAULT FALSE".into(),
                row_getter: "get_bool",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: None,
                field_attr: None,
            },
            FieldKind::Timestamp => FieldSpec {
                rust_type: "DateTime<Utc>",
                sql_decl: "TIMESTAMPTZ NOT NULL".into(),
                row_getter: "get_datetime",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: Some("use chrono::{DateTime, Utc};"),
                field_attr: None,
            },
            FieldKind::Date => FieldSpec {
                rust_type: "NaiveDate",
                sql_decl: "DATE NOT NULL".into(),
                row_getter: "get_date",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: Some("use chrono::NaiveDate;"),
                field_attr: None,
            },
            FieldKind::Time => FieldSpec {
                rust_type: "NaiveTime",
                sql_decl: "TIME NOT NULL".into(),
                row_getter: "get_time",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: Some("use chrono::NaiveTime;"),
                field_attr: None,
            },
            FieldKind::Uuid => FieldSpec {
                rust_type: "Uuid",
                sql_decl: "UUID NOT NULL".into(),
                row_getter: "get_uuid",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: Some("use uuid::Uuid;"),
                field_attr: None,
            },
            FieldKind::Json => FieldSpec {
                rust_type: "serde_json::Value",
                sql_decl: "JSONB NOT NULL".into(),
                row_getter: "get_json",
                insert_needs_clone: true,
                is_text_search: false,
                needs_import: None,
                field_attr: None,
            },
            FieldKind::Fk(target) => FieldSpec {
                rust_type: "i64",
                sql_decl: format!(
                    "BIGINT NOT NULL REFERENCES {}(id)",
                    pluralise_camel_to_snake(target)
                ),
                row_getter: "get_i64",
                insert_needs_clone: false,
                is_text_search: false,
                needs_import: None,
                field_attr: None,
            },
        }
    }
}

/// Pluralise a snake_case singular noun. Three rules, evaluated
/// in order:
///
/// 1. Ends in `s` / `x` / `z` / `ch` / `sh` → add `es`
///    (`class -> classes`, `bus -> buses`, `box -> boxes`).
/// 2. Ends in consonant + `y` → drop `y`, add `ies`
///    (`category -> categories`).
/// 3. Otherwise → add `s` (`task -> tasks`).
///
/// Vowel + `y` keeps the `y` (`monkey -> monkeys`). Deliberately
/// no irregular-noun dictionary (`person -> persons`, not
/// `people` — predictable beats correct here; users rename if
/// needed). Empty input returns empty.
pub(crate) fn pluralise_snake(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if let Some(stem) = s.strip_suffix('y') {
        let prev = stem.chars().last();
        return if matches!(prev, Some(c) if "aeiou".contains(c)) {
            format!("{s}s")
        } else {
            format!("{stem}ies")
        };
    }
    if s.ends_with("ch") || s.ends_with("sh") {
        return format!("{s}es");
    }
    let last = s.chars().last().unwrap();
    if matches!(last, 's' | 'x' | 'z') {
        return format!("{s}es");
    }
    format!("{s}s")
}

/// CamelCase singular -> snake_case plural. Composes
/// [`pluralise_snake`] after a camel→snake walk.
fn pluralise_camel_to_snake(name: &str) -> String {
    let mut snake = String::with_capacity(name.len() + 2);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            snake.push(c.to_ascii_lowercase());
        } else {
            snake.push(c);
        }
    }
    pluralise_snake(&snake)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Parser ----

    #[test]
    fn parse_each_simple_type() {
        let cases = [
            ("name:str", FieldKind::Str),
            ("body:text", FieldKind::Text),
            ("count:int", FieldKind::Int),
            ("user_id:bigint", FieldKind::Bigint),
            ("price:float", FieldKind::Float),
            ("amount:decimal", FieldKind::Decimal),
            ("active:bool", FieldKind::Bool),
            ("at:timestamp", FieldKind::Timestamp),
            ("birth_date:date", FieldKind::Date),
            ("start_time:time", FieldKind::Time),
            ("public_id:uuid", FieldKind::Uuid),
            ("contact:email", FieldKind::Email),
            ("mobile:phone", FieldKind::Phone),
            ("meta:json", FieldKind::Json),
        ];
        for (input, expected) in cases {
            let f = parse_field(input).unwrap_or_else(|e| panic!("{input}: {e}"));
            assert_eq!(f.kind, expected);
        }
    }

    #[test]
    fn parse_fk_with_camelcase_target() {
        let f = parse_field("doctor:fk:Doctor").unwrap();
        assert_eq!(f.name, "doctor");
        assert_eq!(f.kind, FieldKind::Fk("Doctor".into()));
    }

    #[test]
    fn parse_fk_target_can_have_digits() {
        let f = parse_field("scan:fk:CTScan2").unwrap();
        assert_eq!(f.kind, FieldKind::Fk("CTScan2".into()));
    }

    #[test]
    fn parse_rejects_missing_colon() {
        let e = parse_field("name").unwrap_err();
        assert!(e.problem.contains("`name` is not a valid"));
        assert!(e.why.contains("Expected format"));
    }

    #[test]
    fn parse_rejects_unknown_type() {
        let e = parse_field("name:varchar").unwrap_err();
        assert!(e.problem.contains("`varchar` is not a valid field type"));
        assert!(e
            .why
            .contains("str, text, int, bigint, bool, timestamp, json"));
    }

    #[test]
    fn parse_rejects_empty_field_name() {
        let e = parse_field(":str").unwrap_err();
        assert!(e.problem.contains("empty"));
    }

    #[test]
    fn parse_rejects_field_name_starting_with_digit() {
        let e = parse_field("1count:int").unwrap_err();
        assert!(e.problem.contains("starts with a digit"));
    }

    #[test]
    fn parse_rejects_field_name_with_uppercase() {
        let e = parse_field("Name:str").unwrap_err();
        assert!(e.problem.contains("invalid characters"));
    }

    #[test]
    fn parse_rejects_field_name_with_dash() {
        let e = parse_field("full-name:str").unwrap_err();
        assert!(e.problem.contains("invalid characters"));
    }

    #[test]
    fn parse_rejects_rust_keyword() {
        let e = parse_field("type:str").unwrap_err();
        assert!(e.problem.contains("`type` is a reserved Rust identifier"));
        let e = parse_field("self:str").unwrap_err();
        assert!(e.problem.contains("`self`"));
        let e = parse_field("match:str").unwrap_err();
        assert!(e.problem.contains("`match`"));
    }

    #[test]
    fn parse_rejects_underscore_only_name() {
        let e = parse_field("_:str").unwrap_err();
        assert!(e.problem.contains("`_`"));
    }

    #[test]
    fn parse_rejects_bare_fk() {
        let e = parse_field("patient:fk").unwrap_err();
        assert!(e.problem.contains("fk` requires a target model"));
    }

    #[test]
    fn parse_rejects_fk_with_empty_target() {
        let e = parse_field("patient:fk:").unwrap_err();
        assert!(e.problem.contains("fk` requires a target model"));
    }

    #[test]
    fn parse_rejects_fk_with_lowercase_target() {
        let e = parse_field("patient:fk:patient").unwrap_err();
        assert!(e.problem.contains("CamelCase"));
    }

    #[test]
    fn parse_rejects_fk_with_extra_segment() {
        let e = parse_field("patient:fk:Patient:cascade").unwrap_err();
        assert!(e.problem.contains("too many colon"));
    }

    #[test]
    fn validate_unique_catches_duplicates() {
        let fields = vec![
            parse_field("name:str").unwrap(),
            parse_field("name:text").unwrap(),
        ];
        let e = validate_unique_names(&fields).unwrap_err();
        assert!(e.problem.contains("Field `name` declared twice"));
    }

    #[test]
    fn validate_unique_passes_for_distinct() {
        let fields = vec![
            parse_field("name:str").unwrap(),
            parse_field("body:text").unwrap(),
            parse_field("at:timestamp").unwrap(),
        ];
        assert!(validate_unique_names(&fields).is_ok());
    }

    // ---- Renderer ----

    fn fs(specs: &[&str]) -> Vec<Field> {
        specs.iter().map(|s| parse_field(s).unwrap()).collect()
    }

    #[test]
    fn render_struct_fields_in_declared_order() {
        let r = render(&fs(&["name:str", "active:bool", "at:timestamp"]));
        assert_eq!(
            r.struct_fields,
            "    pub name: String,\n    pub active: bool,\n    pub at: DateTime<Utc>,"
        );
    }

    #[test]
    fn render_imports_include_chrono_only_when_timestamp_present() {
        let r = render(&fs(&["name:str"]));
        assert!(!r.imports.contains("chrono"));
        let r = render(&fs(&["at:timestamp"]));
        assert!(r.imports.contains("use chrono::{DateTime, Utc};"));
    }

    #[test]
    fn render_sql_decls_for_every_kind() {
        let r = render(&fs(&[
            "n:str",
            "b:text",
            "c:int",
            "d:bigint",
            "e:bool",
            "f:timestamp",
            "g:json",
            "h:fk:Doctor",
        ]));
        // Per the §2.2 / module-doc defaults table.
        assert!(r.column_decls_sql.contains(",\n    n TEXT NOT NULL"));
        assert!(r.column_decls_sql.contains(",\n    b TEXT NOT NULL"));
        assert!(r.column_decls_sql.contains(",\n    c INTEGER NOT NULL"));
        assert!(r.column_decls_sql.contains(",\n    d BIGINT NOT NULL"));
        assert!(r
            .column_decls_sql
            .contains(",\n    e BOOLEAN NOT NULL DEFAULT FALSE"));
        // PR 2.1 design adjustment: NO `DEFAULT NOW()` for timestamp.
        assert!(
            r.column_decls_sql
                .contains(",\n    f TIMESTAMPTZ NOT NULL\n")
                || r.column_decls_sql.ends_with("f TIMESTAMPTZ NOT NULL")
                || r.column_decls_sql
                    .contains(",\n    f TIMESTAMPTZ NOT NULL,")
        );
        assert!(!r.column_decls_sql.contains("DEFAULT NOW()"));
        // PR 2.1 design adjustment: NO `DEFAULT '{}'::jsonb` for json.
        assert!(r.column_decls_sql.contains(",\n    g JSONB NOT NULL"));
        assert!(!r.column_decls_sql.contains("'{}'"));
        // FK uses the target's snake-pluralised table.
        assert!(r
            .column_decls_sql
            .contains(",\n    h BIGINT NOT NULL REFERENCES doctors(id)"));
    }

    #[test]
    fn render_new_scalar_types_float_date_time() {
        let r = render(&fs(&["price:float", "birth_date:date", "start_time:time"]));
        // Struct fields use the mapped Rust types.
        assert!(r.struct_fields.contains("    pub price: f64,"));
        assert!(r.struct_fields.contains("    pub birth_date: NaiveDate,"));
        assert!(r.struct_fields.contains("    pub start_time: NaiveTime,"));
        // SQL DDL per the §2.2 defaults table.
        assert!(r
            .column_decls_sql
            .contains(",\n    price DOUBLE PRECISION NOT NULL"));
        assert!(r
            .column_decls_sql
            .contains(",\n    birth_date DATE NOT NULL"));
        assert!(r
            .column_decls_sql
            .contains(",\n    start_time TIME NOT NULL"));
        // Row getters.
        assert!(r
            .from_row_assignments
            .contains("price: row.get_f64(\"price\")?,"));
        assert!(r
            .from_row_assignments
            .contains("birth_date: row.get_date(\"birth_date\")?,"));
        assert!(r
            .from_row_assignments
            .contains("start_time: row.get_time(\"start_time\")?,"));
        // date / time pull in only the chrono types they use — never
        // the `DateTime, Utc` pair (that would be an unused import and
        // fail the consumer's `-D warnings` build).
        assert!(r.imports.contains("use chrono::NaiveDate;"));
        assert!(r.imports.contains("use chrono::NaiveTime;"));
        assert!(!r.imports.contains("DateTime"));
        // All three are `Copy` → `.into()` with no `.clone()`.
        assert!(r.insert_values_expr.contains("self.price.into()"));
        assert!(r.insert_values_expr.contains("self.birth_date.into()"));
        assert!(r.insert_values_expr.contains("self.start_time.into()"));
        // None are text-searchable.
        assert_eq!(r.search_fields_literal, "");
    }

    #[test]
    fn render_decimal_and_uuid_types() {
        let r = render(&fs(&["price:decimal", "public_id:uuid"]));
        assert!(r.struct_fields.contains("    pub price: Decimal,"));
        assert!(r.struct_fields.contains("    pub public_id: Uuid,"));
        assert!(r.column_decls_sql.contains(",\n    price NUMERIC NOT NULL"));
        assert!(r
            .column_decls_sql
            .contains(",\n    public_id UUID NOT NULL"));
        assert!(r
            .from_row_assignments
            .contains("price: row.get_decimal(\"price\")?,"));
        assert!(r
            .from_row_assignments
            .contains("public_id: row.get_uuid(\"public_id\")?,"));
        // Each pulls in only its own type crate.
        assert!(r.imports.contains("use rust_decimal::Decimal;"));
        assert!(r.imports.contains("use uuid::Uuid;"));
        assert!(!r.imports.contains("chrono"));
        // Both are `Copy` → `.into()` with no `.clone()`.
        assert!(r.insert_values_expr.contains("self.price.into()"));
        assert!(r.insert_values_expr.contains("self.public_id.into()"));
        // Neither is text-searchable.
        assert_eq!(r.search_fields_literal, "");
    }

    #[test]
    fn render_email_and_phone_carry_format_attribute() {
        let r = render(&fs(&["work_email:email", "mobile:phone"]));
        // Both land as `String` with the `#[rustio(format = "...")]`
        // attribute the derive macro reads to pick the widget +
        // validator (the struct type alone can't tell them apart).
        assert!(r
            .struct_fields
            .contains("    #[rustio(format = \"email\")]\n    pub work_email: String,"));
        assert!(r
            .struct_fields
            .contains("    #[rustio(format = \"phone\")]\n    pub mobile: String,"));
        // Stored as plain TEXT, read with the string getter.
        assert!(r
            .column_decls_sql
            .contains(",\n    work_email TEXT NOT NULL"));
        assert!(r.column_decls_sql.contains(",\n    mobile TEXT NOT NULL"));
        assert!(r
            .from_row_assignments
            .contains("work_email: row.get_string(\"work_email\")?,"));
        // No extra imports — String needs none.
        assert!(!r.imports.contains("chrono"));
        // Both are text-like and therefore searchable.
        assert_eq!(r.search_fields_literal, r#""work_email", "mobile""#);
    }

    #[test]
    fn render_columns_literal_includes_id_first() {
        let r = render(&fs(&["name:str", "active:bool"]));
        assert_eq!(r.columns_literal, r#""id", "name", "active""#);
        assert_eq!(r.insert_columns_literal, r#""name", "active""#);
    }

    #[test]
    fn render_from_row_uses_correct_getter_per_type() {
        let r = render(&fs(&[
            "n:str",
            "c:int",
            "d:bigint",
            "e:bool",
            "f:timestamp",
            "g:json",
            "h:fk:Doctor",
        ]));
        assert!(r
            .from_row_assignments
            .contains("n: row.get_string(\"n\")?,"));
        assert!(r.from_row_assignments.contains("c: row.get_i32(\"c\")?,"));
        assert!(r.from_row_assignments.contains("d: row.get_i64(\"d\")?,"));
        assert!(r.from_row_assignments.contains("e: row.get_bool(\"e\")?,"));
        assert!(r
            .from_row_assignments
            .contains("f: row.get_datetime(\"f\")?,"));
        assert!(r.from_row_assignments.contains("g: row.get_json(\"g\")?,"));
        // FK is i64.
        assert!(r.from_row_assignments.contains("h: row.get_i64(\"h\")?,"));
    }

    #[test]
    fn render_insert_values_clones_string_and_json_only() {
        let r = render(&fs(&["n:str", "c:int", "e:bool", "f:timestamp", "g:json"]));
        assert!(r.insert_values_expr.contains("self.n.clone().into()"));
        assert!(r.insert_values_expr.contains("self.c.into()"));
        assert!(r.insert_values_expr.contains("self.e.into()"));
        assert!(r.insert_values_expr.contains("self.f.into()"));
        assert!(r.insert_values_expr.contains("self.g.clone().into()"));
    }

    #[test]
    fn render_search_fields_only_text_kinds() {
        let r = render(&fs(&["name:str", "body:text", "count:int", "active:bool"]));
        assert_eq!(r.search_fields_literal, r#""name", "body""#);
    }

    #[test]
    fn render_search_fields_empty_when_no_text_kinds() {
        let r = render(&fs(&["count:int", "active:bool"]));
        assert_eq!(r.search_fields_literal, "");
    }

    // ---- Pluraliser parity ----

    #[test]
    fn pluralise_handles_common_shapes() {
        assert_eq!(pluralise_camel_to_snake("Patient"), "patients");
        assert_eq!(pluralise_camel_to_snake("Doctor"), "doctors");
        assert_eq!(pluralise_camel_to_snake("Category"), "categories");
        assert_eq!(pluralise_camel_to_snake("Box"), "boxes");
        assert_eq!(pluralise_camel_to_snake("Status"), "statuses");
        assert_eq!(pluralise_camel_to_snake("Branch"), "branches");
        assert_eq!(pluralise_camel_to_snake("Dish"), "dishes");
        assert_eq!(pluralise_camel_to_snake("BookReview"), "book_reviews");
    }

    // ---- pluralise_snake (the bug-fix surface) ----

    #[test]
    fn pluralise_snake_default_adds_s() {
        assert_eq!(pluralise_snake("task"), "tasks");
        assert_eq!(pluralise_snake("patient"), "patients");
        assert_eq!(pluralise_snake("book_review"), "book_reviews");
    }

    #[test]
    fn pluralise_snake_sxz_take_es() {
        // The bug from the audit: naive +s on "class" yielded "classs".
        assert_eq!(pluralise_snake("class"), "classes");
        assert_eq!(pluralise_snake("bus"), "buses");
        assert_eq!(pluralise_snake("status"), "statuses");
        assert_eq!(pluralise_snake("box"), "boxes");
        assert_eq!(pluralise_snake("quiz"), "quizes");
    }

    #[test]
    fn pluralise_snake_ch_sh_take_es() {
        assert_eq!(pluralise_snake("branch"), "branches");
        assert_eq!(pluralise_snake("dish"), "dishes");
    }

    #[test]
    fn pluralise_snake_consonant_y_becomes_ies() {
        assert_eq!(pluralise_snake("category"), "categories");
        assert_eq!(pluralise_snake("city"), "cities");
        assert_eq!(pluralise_snake("party"), "parties");
    }

    #[test]
    fn pluralise_snake_vowel_y_keeps_y() {
        assert_eq!(pluralise_snake("monkey"), "monkeys");
        assert_eq!(pluralise_snake("survey"), "surveys");
        assert_eq!(pluralise_snake("day"), "days");
    }

    #[test]
    fn pluralise_snake_empty_returns_empty() {
        assert_eq!(pluralise_snake(""), "");
    }
}
