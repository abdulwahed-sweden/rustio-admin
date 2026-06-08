//! Procedural macros for `rustio-admin`.
//!
//! `#[derive(RustioAdmin)]`. Given a user-written struct, the derive
//! emits two impls from the fields it already parses:
//!
//!   * `impl AdminModel` — `ADMIN_NAME`, `DISPLAY_NAME`,
//!     `SINGULAR_NAME`, `FIELDS`, and the row/form/update helpers.
//!   * `impl ::rustio_admin::orm::Model` — `TABLE`, `COLUMNS`,
//!     `INSERT_COLUMNS`, `id`, `from_row`, `insert_values`. This used to
//!     be hand-written in every model file (and had to be kept in sync
//!     with the struct by hand); the derive now generates it from the
//!     same field walk, so the struct is the single source of truth.
//!
//! Struct-level escape hatches for the rare cases the field walk can't
//! infer: `#[rustio(table = "…")]` when the SQL table name differs from
//! the auto slug (e.g. `Address` → `addresses`), and
//! `#[rustio(extra_columns = ["…"])]` for generated/virtual columns
//! that are not struct fields (e.g. a Postgres `tsvector`) but must be
//! SELECT-safe and full-text-search eligible.
//!
//! The macro deliberately stays dumb: all runtime behaviour lives in
//! `rustio_admin`. Keeping the macro small makes it easier to debug —
//! if something feels wrong, read the generated code with
//! `cargo expand`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit, Meta};

// public:
#[proc_macro_derive(RustioAdmin, attributes(rustio))]
pub fn derive_rustio_admin(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let fields = struct_fields(&input)?;

    // Struct-level overrides from `#[rustio(...)]` on the struct.
    // Project-side knobs that escape the macro's auto-deriving from
    // the struct name. `VISIBILITY_AUDIT.md` F3: pre-0.8.1 there was
    // no way to override `DISPLAY_NAME` short of renaming the struct,
    // so projects with `CaseAction` got "Case actions", `Disclosure`
    // got "Disclosures", etc. — bearable but not polishable.
    let struct_overrides = parse_struct_attr(&input.attrs)?;

    let admin_name = match struct_overrides.admin_name {
        Some(ref s) => s.clone(),
        None => plural_snake(&struct_name.to_string()),
    };
    let display_name = match struct_overrides.display_name {
        Some(ref s) => s.clone(),
        None => humanise(&plural_snake(&struct_name.to_string())),
    };
    let singular = struct_name.to_string();

    let mut field_metas = Vec::new();
    let mut display_value_arms = Vec::new();
    let mut from_form_parses = Vec::new();
    let mut from_form_fields = Vec::new();
    let mut update_tuples = Vec::new();

    // `impl Model` (orm) accumulation. `COLUMNS` / `from_row` cover
    // every struct field; `INSERT_COLUMNS` / `insert_values` skip the
    // auto `id` (it is DB-assigned via `RETURNING id`).
    let mut model_columns: Vec<String> = Vec::new();
    let mut model_insert_columns: Vec<String> = Vec::new();
    let mut from_row_inits = Vec::new();
    let mut insert_value_exprs = Vec::new();

    for f in fields {
        let fname = f.ident.as_ref().unwrap();
        let fname_str = fname.to_string();
        let kind = classify_type(&f.ty)?;
        // Fields named `created_at` / `updated_at` are
        // managed by the framework: hidden from forms, defaulted to
        // `Utc::now()` in `from_form`. The macro wires that behaviour
        // through `FieldKind::DateTimeAuto`; this promotion is the
        // missing trigger that makes the variant reachable for the
        // conventionally named timestamp columns.
        let kind = if matches!(kind, FieldKind::DateTime) && is_auto_timestamp_name(&fname_str) {
            FieldKind::DateTimeAuto
        } else {
            kind
        };
        // `#[rustio(file)]` promotes String / Option<String> to the
        // file-upload variants. Other base types reject the marker —
        // the macro emits a compile error so a typo'd attribute on
        // an i64 column doesn't silently render as a text input.
        let kind = if parse_file_attr(&f.attrs)? {
            match kind {
                FieldKind::String => FieldKind::FilePath,
                FieldKind::OptionalString => FieldKind::OptionalFilePath,
                other => {
                    return Err(syn::Error::new_spanned(
                        f,
                        format!(
                            "#[rustio(file)] is only valid on String or Option<String> fields; \
                             got {other:?} for `{fname_str}`"
                        ),
                    ));
                }
            }
        } else {
            kind
        };
        // `#[rustio(format = "email" | "phone")]` promotes a String
        // column to the validated-string variants. Same discipline as
        // the file marker: it's only valid on String, and a typo'd
        // value or a non-String target is a compile error.
        let kind = match parse_format_attr(&f.attrs)? {
            Some(fmt) => match kind {
                FieldKind::String if fmt == "email" => FieldKind::Email,
                FieldKind::String if fmt == "phone" => FieldKind::Phone,
                other => {
                    return Err(syn::Error::new_spanned(
                        f,
                        format!(
                            "#[rustio(format = \"...\")] is only valid on String fields; \
                             got {other:?} for `{fname_str}`"
                        ),
                    ));
                }
            },
            None => kind,
        };
        // `#[rustio(choices = ["a", "b"])]` promotes a String column to
        // a dropdown. The values ride in `field_choices`; the `Choice`
        // kind is just the marker the from_form / display arms and the
        // `AdminField.choices` slice switch on.
        let field_choices = parse_choices_attr(&f.attrs)?;
        let kind = match &field_choices {
            Some(values) if !values.is_empty() => match kind {
                FieldKind::String => FieldKind::Choice,
                other => {
                    return Err(syn::Error::new_spanned(
                        f,
                        format!(
                            "#[rustio(choices = [...])] is only valid on String fields; \
                             got {other:?} for `{fname_str}`"
                        ),
                    ));
                }
            },
            _ => kind,
        };
        let editable = fname_str != "id" && kind != FieldKind::DateTimeAuto;

        // `impl Model` rows. Every field is a SELECT column and a
        // `from_row` getter; every field but `id` is an INSERT column
        // and an `insert_values` entry (auto timestamps included — they
        // are written, just not editable in the form).
        model_columns.push(fname_str.clone());
        let row_getter = format_ident!("{}", kind.row_getter());
        from_row_inits.push(quote! { #fname: row.#row_getter(#fname_str)? });
        if fname_str != "id" {
            model_insert_columns.push(fname_str.clone());
            insert_value_exprs.push(quote! { self.#fname.clone().into() });
        }

        let type_variant = kind.field_type_ident();
        let relation = parse_relation_attr(&f.attrs, &fname_str)?;
        let relation_tokens = match &relation {
            Some((target, display)) => {
                let display_tok = match display {
                    Some(d) => quote! { ::std::option::Option::Some(#d) },
                    None => quote! { ::std::option::Option::None },
                };
                quote! {
                    ::std::option::Option::Some(::rustio_admin::admin::AdminRelation {
                        target_model: #target,
                        display_field: #display_tok,
                        // Single belongs_to relations default to
                        // single `<select>`. Many-to-many is opt-in via
                        // a future `#[rustio(many_to_many)]` attribute;
                        // the macro emits `false` for now so consumers
                        // that want multi-select must hand-set the
                        // field on the generated AdminRelation.
                        multi: false,
                    })
                }
            }
            None => quote! { ::std::option::Option::None },
        };

        // Humanised display label, computed once at expansion time:
        // `performed_by_technician` → `"Performed by technician"`. The
        // list page renders this through CSS uppercase+tracking as
        // `PERFORMED BY TECHNICIAN` with real word boundaries, so the
        // header can wrap on narrow rows instead of dictating a wide
        // column floor. Also reused below for validation messages.
        let humanised_label = humanise_field(&fname_str);
        // `#[rustio(choices = [...])]` → a `&'static [&'static str]`
        // the form layer renders as a `<select>`. Absent → `None`.
        let choices_tokens = match &field_choices {
            Some(values) => {
                let lits = values.iter().map(|v| v.as_str());
                quote! { ::std::option::Option::Some(&[ #(#lits),* ]) }
            }
            None => quote! { ::std::option::Option::None },
        };
        field_metas.push(quote! {
            ::rustio_admin::admin::AdminField {
                name: #fname_str,
                label: #humanised_label,
                field_type: ::rustio_admin::admin::FieldType::#type_variant,
                editable: #editable,
                relation: #relation_tokens,
                choices: #choices_tokens,
            }
        });

        // `display_values`: stringify the field for the list page.
        let display_arm = match kind {
            // FilePath / OptionalFilePath live in `String` /
            // `Option<String>` Rust types but render in the form
            // as `<input type="file">`. The display path is
            // identical to the string variants — the stored value
            // IS the relative path, surfaced as plain text on the
            // list page.
            FieldKind::String
            | FieldKind::FilePath
            | FieldKind::Email
            | FieldKind::Phone
            | FieldKind::Choice => {
                quote! {
                    out.push((#fname_str.to_string(), self.#fname.clone()));
                }
            }
            FieldKind::OptionalString | FieldKind::OptionalFilePath => quote! {
                // `Option<String>` does not implement `Display`, so we
                // can't share the String arm. None → empty string,
                // Some(v) → v.
                out.push((#fname_str.to_string(), match &self.#fname {
                    Some(v) => v.clone(),
                    None => String::new(),
                }));
            },
            FieldKind::I32
            | FieldKind::I64
            | FieldKind::F64
            | FieldKind::Decimal
            | FieldKind::Uuid => quote! {
                // Decimal and Uuid both round-trip through their
                // canonical `Display` form (`19.99`, hyphenated UUID).
                out.push((#fname_str.to_string(), self.#fname.to_string()));
            },
            FieldKind::OptionalI64 => quote! {
                out.push((#fname_str.to_string(), match &self.#fname {
                    Some(v) => v.to_string(),
                    None => String::new(),
                }));
            },
            FieldKind::Bool => quote! {
                out.push((#fname_str.to_string(), if self.#fname { "true".to_string() } else { "false".to_string() }));
            },
            FieldKind::DateTime | FieldKind::DateTimeAuto => quote! {
                // ISO-8601 form with `T` separator. This is the exact
                // wire format `<input type="datetime-local">` expects
                // (`%Y-%m-%dT%H:%M`); the form-render path puts this
                // string straight into the input's `value=` attribute.
                // The list path detects the same shape (16 chars, `T`
                // at index 10) and splits it into the two-line cell
                // layout. NOTE: `datetime-local` cannot encode timezone;
                // we surface UTC values directly.
                out.push((#fname_str.to_string(), self.#fname.format("%Y-%m-%dT%H:%M").to_string()));
            },
            FieldKind::OptionalDateTime => quote! {
                // Symmetric to `OptionalString` / `OptionalI64`: None →
                // empty string, Some(v) → same ISO-8601 form as the
                // non-optional `DateTime` arm.
                out.push((#fname_str.to_string(), match &self.#fname {
                    Some(v) => v.format("%Y-%m-%dT%H:%M").to_string(),
                    None => String::new(),
                }));
            },
            FieldKind::Date => quote! {
                // `%Y-%m-%d` is exactly what `<input type="date">`
                // round-trips, so the rendered value drops straight
                // back into the input's `value=` attribute.
                out.push((#fname_str.to_string(), self.#fname.format("%Y-%m-%d").to_string()));
            },
            FieldKind::Time => quote! {
                // `%H:%M` matches `<input type="time">` (no seconds).
                out.push((#fname_str.to_string(), self.#fname.format("%H:%M").to_string()));
            },
        };
        display_value_arms.push(display_arm);

        // `from_form`: read the HTML form body into a struct field.
        if fname_str == "id" {
            from_form_fields.push(quote! { #fname: 0 });
            continue;
        }

        // Precompute human-readable validation messages at expansion
        // time so the runtime error path doesn't repeat the same
        // `format!` work per request and so every model emits
        // identically-styled copy. `humanised_label` was already
        // computed above for `AdminField.label`.
        let required_msg = format!("{humanised_label} is required.");
        let number_msg = format!("{humanised_label} must be a number.");
        let date_invalid_msg = format!("{humanised_label} is not a valid date.");
        let time_invalid_msg = format!("{humanised_label} is not a valid time.");
        let uuid_invalid_msg = format!("{humanised_label} is not a valid UUID.");
        let email_invalid_msg = format!("{humanised_label} is not a valid email address.");
        let phone_invalid_msg = format!("{humanised_label} is not a valid phone number.");

        match kind {
            FieldKind::String | FieldKind::FilePath => {
                // Trim incoming whitespace so a `"   "` submission is
                // treated as empty (and triggers the required-field
                // error) instead of silently saving a whitespace-only
                // string. FilePath uses the same trimming path: the
                // multipart-form handler injects the saved relative
                // path string into the form before `from_form` sees
                // it, so the value lands here as a normal String.
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str).map(str::trim) {
                        Some(v) if !v.is_empty() => v.to_string(),
                        _ => { errors.push(#required_msg.to_string()); String::new() }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Email => {
                // Required, trimmed, then format-checked. The column is
                // plain TEXT so the trimmed value is stored verbatim
                // even when the format check fails (the error is what
                // blocks the save, not a value rewrite).
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str).map(str::trim) {
                        Some(v) if !v.is_empty() => {
                            if !::rustio_admin::admin::is_valid_email(v) {
                                errors.push(#email_invalid_msg.to_string());
                            }
                            v.to_string()
                        }
                        _ => { errors.push(#required_msg.to_string()); String::new() }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Phone => {
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str).map(str::trim) {
                        Some(v) if !v.is_empty() => {
                            if !::rustio_admin::admin::is_valid_phone(v) {
                                errors.push(#phone_invalid_msg.to_string());
                            }
                            v.to_string()
                        }
                        _ => { errors.push(#required_msg.to_string()); String::new() }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Choice => {
                // Required, trimmed, then checked against the declared
                // set. The DB also carries a `CHECK (... IN (...))`
                // constraint, but validating here yields a calm form
                // error instead of a 409 from a constraint violation.
                let values = field_choices
                    .as_ref()
                    .expect("Choice kind is only set when choices are present");
                let choice_lits = values.iter().map(|v| v.as_str());
                let choice_invalid_msg =
                    format!("{humanised_label} must be one of: {}.", values.join(", "));
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str).map(str::trim) {
                        Some(v) if !v.is_empty() => {
                            const CHOICES: &[&str] = &[ #(#choice_lits),* ];
                            if !CHOICES.contains(&v) {
                                errors.push(#choice_invalid_msg.to_string());
                            }
                            v.to_string()
                        }
                        _ => { errors.push(#required_msg.to_string()); String::new() }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::OptionalString | FieldKind::OptionalFilePath => {
                // Trim, then collapse trimmed-empty to None so the
                // column stores NULL instead of `""`. Optional
                // FilePath shares the same path — the file-input
                // widget can submit an empty string when the
                // operator clears the field.
                from_form_parses.push(quote! {
                    let #fname: Option<String> = form
                        .get(#fname_str)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::I32 => {
                from_form_parses.push(quote! {
                    let #fname: i32 = match form.get(#fname_str).and_then(|v| v.parse().ok()) {
                        Some(v) => v,
                        None => { errors.push(#number_msg.to_string()); 0 }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::I64 => {
                from_form_parses.push(quote! {
                    let #fname: i64 = match form.get(#fname_str).and_then(|v| v.parse().ok()) {
                        Some(v) => v,
                        None => { errors.push(#number_msg.to_string()); 0 }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::F64 => {
                from_form_parses.push(quote! {
                    let #fname: f64 = match form.get(#fname_str).and_then(|v| v.parse().ok()) {
                        Some(v) => v,
                        None => { errors.push(#number_msg.to_string()); 0.0 }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Decimal => {
                from_form_parses.push(quote! {
                    let #fname: ::rustio_admin::rust_decimal::Decimal =
                        match form.get(#fname_str).map(str::trim) {
                            Some(raw) if !raw.is_empty() => match raw.parse() {
                                Ok(v) => v,
                                Err(_) => {
                                    errors.push(#number_msg.to_string());
                                    ::rustio_admin::rust_decimal::Decimal::ZERO
                                }
                            },
                            _ => {
                                errors.push(#required_msg.to_string());
                                ::rustio_admin::rust_decimal::Decimal::ZERO
                            }
                        };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::OptionalI64 => {
                // Distinguish "user left it blank" (None, legitimate)
                // from "user typed garbage" (validation error, NOT
                // silently dropped).
                from_form_parses.push(quote! {
                    let #fname: Option<i64> = match form.get(#fname_str).map(str::trim) {
                        None | Some("") => None,
                        Some(raw) => match raw.parse::<i64>() {
                            Ok(n) => Some(n),
                            Err(_) => {
                                errors.push(#number_msg.to_string());
                                None
                            }
                        },
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Bool => {
                from_form_parses.push(quote! {
                    let #fname: bool = form.bool_flag(#fname_str);
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::DateTime => {
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str) {
                        Some(raw) if !raw.is_empty() => {
                            match ::rustio_admin::chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M") {
                                Ok(dt) => ::rustio_admin::chrono::DateTime::<::rustio_admin::chrono::Utc>::from_naive_utc_and_offset(dt, ::rustio_admin::chrono::Utc),
                                Err(_) => { errors.push(#date_invalid_msg.to_string()); ::rustio_admin::chrono::Utc::now() }
                            }
                        }
                        _ => { errors.push(#required_msg.to_string()); ::rustio_admin::chrono::Utc::now() }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Date => {
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str) {
                        Some(raw) if !raw.is_empty() => {
                            match ::rustio_admin::chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
                                Ok(d) => d,
                                Err(_) => {
                                    errors.push(#date_invalid_msg.to_string());
                                    ::rustio_admin::chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
                                }
                            }
                        }
                        _ => {
                            errors.push(#required_msg.to_string());
                            ::rustio_admin::chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
                        }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Time => {
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str) {
                        Some(raw) if !raw.is_empty() => {
                            match ::rustio_admin::chrono::NaiveTime::parse_from_str(raw, "%H:%M") {
                                Ok(t) => t,
                                Err(_) => {
                                    errors.push(#time_invalid_msg.to_string());
                                    ::rustio_admin::chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                                }
                            }
                        }
                        _ => {
                            errors.push(#required_msg.to_string());
                            ::rustio_admin::chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                        }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::Uuid => {
                from_form_parses.push(quote! {
                    let #fname = match form.get(#fname_str).map(str::trim) {
                        Some(raw) if !raw.is_empty() => match ::rustio_admin::uuid::Uuid::parse_str(raw) {
                            Ok(u) => u,
                            Err(_) => {
                                errors.push(#uuid_invalid_msg.to_string());
                                ::rustio_admin::uuid::Uuid::nil()
                            }
                        },
                        _ => {
                            errors.push(#required_msg.to_string());
                            ::rustio_admin::uuid::Uuid::nil()
                        }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::DateTimeAuto => {
                // created_at-style fields default to now().
                from_form_parses.push(quote! {
                    let #fname = ::rustio_admin::chrono::Utc::now();
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::OptionalDateTime => {
                // Symmetric to `OptionalI64`: blank → None (legitimate),
                // garbage → validation error + None (NOT silently
                // defaulted to `Utc::now()` like the non-optional arm).
                from_form_parses.push(quote! {
                    let #fname: ::std::option::Option<::rustio_admin::chrono::DateTime<::rustio_admin::chrono::Utc>> =
                        match form.get(#fname_str).map(str::trim) {
                            None | Some("") => ::std::option::Option::None,
                            Some(raw) => match ::rustio_admin::chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M") {
                                Ok(dt) => ::std::option::Option::Some(
                                    ::rustio_admin::chrono::DateTime::<::rustio_admin::chrono::Utc>::from_naive_utc_and_offset(dt, ::rustio_admin::chrono::Utc),
                                ),
                                Err(_) => {
                                    errors.push(#date_invalid_msg.to_string());
                                    ::std::option::Option::None
                                }
                            },
                        };
                });
                from_form_fields.push(quote! { #fname });
            }
        }

        update_tuples.push(quote! {
            (#fname_str, self.#fname.clone().into())
        });
    }

    let object_label_expr = find_label_field(fields)
        .map(|n| {
            let id = format_ident!("{n}");
            quote! { self.#id.clone().to_string() }
        })
        .unwrap_or_else(|| quote! { format!("#{}", self.id) });

    // SQL table name. Defaults to the admin slug (`Product` → `products`),
    // which matches the table for every conventionally-pluralised model;
    // `#[rustio(table = "…")]` overrides the cases the slug can't reach
    // (e.g. `Address` → slug `address`, table `addresses`).
    let table_name = match struct_overrides.table {
        Some(ref t) => t.clone(),
        None => admin_name.clone(),
    };
    // Virtual/generated columns appended to COLUMNS only — see the
    // crate docs. Never inserted, never read by `from_row`.
    for extra in &struct_overrides.extra_columns {
        model_columns.push(extra.clone());
    }
    let column_lits = model_columns.iter().map(|s| s.as_str());
    let insert_column_lits = model_insert_columns.iter().map(|s| s.as_str());

    Ok(quote! {
        impl ::rustio_admin::admin::AdminModel for #struct_name {
            const ADMIN_NAME: &'static str = #admin_name;
            const DISPLAY_NAME: &'static str = #display_name;
            const SINGULAR_NAME: &'static str = #singular;
            const FIELDS: &'static [::rustio_admin::admin::AdminField] = &[
                #(#field_metas),*
            ];

            fn display_values(&self) -> ::std::vec::Vec<(::std::string::String, ::std::string::String)> {
                let mut out = ::std::vec::Vec::new();
                #(#display_value_arms)*
                out
            }

            fn from_form(form: &::rustio_admin::http::FormData) -> ::std::result::Result<Self, ::std::vec::Vec<::std::string::String>>
            where
                Self: Sized,
            {
                let mut errors: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
                #(#from_form_parses)*
                if !errors.is_empty() {
                    return Err(errors);
                }
                Ok(Self { #(#from_form_fields),* })
            }

            fn object_label(&self) -> ::std::string::String {
                #object_label_expr
            }

            fn id(&self) -> i64 {
                self.id
            }

            fn values_to_update(&self) -> ::std::vec::Vec<(&'static str, ::rustio_admin::orm::Value)> {
                ::std::vec![#(#update_tuples),*]
            }
        }

        impl ::rustio_admin::orm::Model for #struct_name {
            const TABLE: &'static str = #table_name;
            const COLUMNS: &'static [&'static str] = &[ #(#column_lits),* ];
            const INSERT_COLUMNS: &'static [&'static str] = &[ #(#insert_column_lits),* ];

            fn id(&self) -> i64 {
                self.id
            }

            fn from_row(row: ::rustio_admin::orm::Row<'_>) -> ::rustio_admin::error::Result<Self> {
                ::std::result::Result::Ok(Self {
                    #(#from_row_inits),*
                })
            }

            fn insert_values(&self) -> ::std::vec::Vec<::rustio_admin::orm::Value> {
                ::std::vec![ #(#insert_value_exprs),* ]
            }
        }
    })
}

fn struct_fields(
    input: &DeriveInput,
) -> syn::Result<&syn::punctuated::Punctuated<syn::Field, syn::Token![,]>> {
    let data = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "RustioAdmin can only derive on structs",
            ))
        }
    };
    match &data.fields {
        Fields::Named(named) => Ok(&named.named),
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "RustioAdmin requires a struct with named fields",
        )),
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum FieldKind {
    I32,
    I64,
    F64,
    Decimal,
    Bool,
    String,
    /// `String` column flagged with `#[rustio(format = "email")]`.
    /// Stored as `TEXT`; renders as `<input type="email">` and runs
    /// [`is_valid_email`](rustio_admin::admin::is_valid_email) in
    /// `from_form`.
    Email,
    /// `String` column flagged with `#[rustio(format = "phone")]`.
    Phone,
    /// `String` column flagged with `#[rustio(choices = [...])]`.
    /// Stored as `TEXT`; renders as a `<select>` (driven by
    /// `AdminField.choices`, not the `FieldType`) and `from_form`
    /// rejects values outside the declared set. The values
    /// themselves ride in a side variable, not this `Copy` enum.
    Choice,
    DateTime,
    Date,
    Time,
    Uuid,
    DateTimeAuto,
    OptionalString,
    OptionalI64,
    OptionalDateTime,
    /// `String` column flagged with `#[rustio(file)]`. Renders as
    /// `<input type="file">`; the multipart-form handler writes
    /// the uploaded bytes under `Admin::uploads_dir` and injects
    /// the relative path string back into the form before
    /// `from_form` parses it as a normal String.
    FilePath,
    /// `Option<String>` counterpart.
    OptionalFilePath,
}

impl FieldKind {
    fn field_type_ident(&self) -> proc_macro2::Ident {
        match self {
            FieldKind::I32 => format_ident!("I32"),
            FieldKind::I64 => format_ident!("I64"),
            FieldKind::F64 => format_ident!("F64"),
            FieldKind::Decimal => format_ident!("Decimal"),
            FieldKind::Bool => format_ident!("Bool"),
            FieldKind::String => format_ident!("String"),
            FieldKind::Email => format_ident!("Email"),
            FieldKind::Phone => format_ident!("Phone"),
            // A choice column is a String at the type level; the
            // `<select>` comes from `AdminField.choices`.
            FieldKind::Choice => format_ident!("String"),
            FieldKind::DateTime | FieldKind::DateTimeAuto => format_ident!("DateTime"),
            FieldKind::Date => format_ident!("Date"),
            FieldKind::Time => format_ident!("Time"),
            FieldKind::Uuid => format_ident!("Uuid"),
            FieldKind::OptionalString => format_ident!("OptionalString"),
            FieldKind::OptionalI64 => format_ident!("OptionalI64"),
            FieldKind::OptionalDateTime => format_ident!("OptionalDateTime"),
            FieldKind::FilePath => format_ident!("FilePath"),
            FieldKind::OptionalFilePath => format_ident!("OptionalFilePath"),
        }
    }

    /// The `Row` accessor `Model::from_row` calls for this kind. The
    /// validated-string variants (Email / Phone / Choice) and the
    /// file-path variants are plain `TEXT` at rest, so they read back
    /// through the same `get_string` / `get_optional_string` as a
    /// String.
    fn row_getter(&self) -> &'static str {
        match self {
            FieldKind::I32 => "get_i32",
            FieldKind::I64 => "get_i64",
            FieldKind::F64 => "get_f64",
            FieldKind::Decimal => "get_decimal",
            FieldKind::Bool => "get_bool",
            FieldKind::String
            | FieldKind::Email
            | FieldKind::Phone
            | FieldKind::Choice
            | FieldKind::FilePath => "get_string",
            FieldKind::OptionalString | FieldKind::OptionalFilePath => "get_optional_string",
            FieldKind::DateTime | FieldKind::DateTimeAuto => "get_datetime",
            FieldKind::OptionalDateTime => "get_optional_datetime",
            FieldKind::Date => "get_date",
            FieldKind::Time => "get_time",
            FieldKind::Uuid => "get_uuid",
            FieldKind::OptionalI64 => "get_optional_i64",
        }
    }
}

/// Names treated as framework-managed timestamps. These fields are
/// auto-promoted to `FieldKind::DateTimeAuto` regardless of declared
/// type so the admin UI doesn't render them and `from_form` fills
/// them with `Utc::now()`. Conservative list; expand only when a real
/// model needs another conventionally-named timestamp.
fn is_auto_timestamp_name(name: &str) -> bool {
    matches!(name, "created_at" | "updated_at")
}

/// Turn a snake_case column name into a Title-Case label for human-
/// readable validation errors emitted by `from_form`. Mirrors the
/// runtime humanise helper so error labels and rendered form labels
/// use identical capitalisation.
///
/// Whole-word acronym recognition: each underscore-separated segment
/// is checked against [`HUMANISE_ACRONYMS`] before being
/// title-cased, so `id` → `ID`, `email_id` → `Email ID`,
/// `mfa_secret_key_id` → `MFA Secret Key ID`. Words *containing* but
/// not *being* an acronym (`video` is not `vIDeo`) are left to the
/// default first-letter-uppercase rule.
fn humanise_field(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(s.len());
    let mut first_segment = true;
    for segment in s.split('_') {
        if !first_segment {
            out.push(' ');
        }
        first_segment = false;
        let lower = segment.to_ascii_lowercase();
        if HUMANISE_ACRONYMS.contains(&lower.as_str()) {
            out.push_str(&lower.to_ascii_uppercase());
        } else {
            let mut chars = segment.chars();
            if let Some(first) = chars.next() {
                out.push(first.to_ascii_uppercase());
                for c in chars {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Acronyms that should be fully uppercase in humanised labels.
///
/// Byte-for-byte mirror of
/// `rustio_admin::admin::render::HUMANISE_ACRONYMS` — the macros
/// crate cannot depend on the main crate (proc-macro cycle), so
/// the two lists are intentionally duplicated. Update both
/// together.
const HUMANISE_ACRONYMS: &[&str] = &[
    "id", "ip", "url", "uri", "api", "uuid", "mfa", "csv", "sql", "html", "http", "https", "json",
    "tls", "ssl", "smtp", "xml",
];

fn classify_type(ty: &syn::Type) -> syn::Result<FieldKind> {
    let as_string = quote! { #ty }.to_string().replace(' ', "");
    let kind = match as_string.as_str() {
        "i32" => FieldKind::I32,
        "i64" => FieldKind::I64,
        "f64" => FieldKind::F64,
        "Decimal" | "rust_decimal::Decimal" => FieldKind::Decimal,
        "bool" => FieldKind::Bool,
        "String" => FieldKind::String,
        "DateTime<Utc>" | "chrono::DateTime<chrono::Utc>" => FieldKind::DateTime,
        "NaiveDate" | "chrono::NaiveDate" => FieldKind::Date,
        "NaiveTime" | "chrono::NaiveTime" => FieldKind::Time,
        "Uuid" | "uuid::Uuid" => FieldKind::Uuid,
        "Option<String>" => FieldKind::OptionalString,
        "Option<i64>" => FieldKind::OptionalI64,
        "Option<DateTime<Utc>>" | "Option<chrono::DateTime<chrono::Utc>>" => {
            FieldKind::OptionalDateTime
        }
        other => {
            return Err(syn::Error::new_spanned(
                ty,
                format!("unsupported field type for RustioAdmin: {other}"),
            ))
        }
    };
    Ok(kind)
}

/// Project-side struct-level overrides parsed from
/// `#[rustio(...)]` on the deriving struct. Adds a polish escape
/// hatch for the otherwise-correct auto-derived defaults — see
/// `VISIBILITY_AUDIT.md` F3.
///
/// Example:
///
/// ```ignore
/// #[derive(RustioAdmin)]
/// #[rustio(
///     admin_name = "case-actions",
///     display_name = "Case events"
/// )]
/// pub struct CaseAction { … }
/// ```
///
/// Both fields are optional. Unknown keys produce a compile error
/// pointing at the attribute span.
#[derive(Default)]
struct StructOverrides {
    admin_name: Option<String>,
    display_name: Option<String>,
    /// SQL table name override for `Model::TABLE`. Absent → the admin
    /// slug is used.
    table: Option<String>,
    /// Generated/virtual columns appended to `Model::COLUMNS` that are
    /// not struct fields (e.g. a Postgres `tsvector`).
    extra_columns: Vec<String>,
}

fn parse_struct_attr(attrs: &[syn::Attribute]) -> syn::Result<StructOverrides> {
    let mut out = StructOverrides::default();
    for attr in attrs {
        if !attr.path().is_ident("rustio") {
            continue;
        }
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("admin_name") {
                let value = m.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    out.admin_name = Some(s.value());
                }
                Ok(())
            } else if m.path.is_ident("display_name") {
                let value = m.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    out.display_name = Some(s.value());
                }
                Ok(())
            } else if m.path.is_ident("table") {
                let value = m.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    out.table = Some(s.value());
                }
                Ok(())
            } else if m.path.is_ident("extra_columns") {
                let array: syn::ExprArray = m.value()?.parse()?;
                for elem in &array.elems {
                    match elem {
                        syn::Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(s), ..
                        }) => out.extra_columns.push(s.value()),
                        other => {
                            return Err(syn::Error::new_spanned(
                                other,
                                "#[rustio(extra_columns = [...])] elements must be string literals",
                            ));
                        }
                    }
                }
                Ok(())
            } else {
                // Field-level keys (e.g. `belongs_to`, `display`)
                // legitimately appear on `#[rustio(...)]` placed on
                // FIELDS, not the struct. When the same `rustio`
                // attribute is on the struct, those keys are
                // surprising. Reject so a misplaced field attribute
                // doesn't silently fail.
                Err(m.error(
                    "unknown rustio struct attribute; expected `admin_name`, \
                     `display_name`, `table`, or `extra_columns`",
                ))
            }
        })?;
    }
    Ok(out)
}

fn parse_relation_attr(
    attrs: &[syn::Attribute],
    field_name: &str,
) -> syn::Result<Option<(String, Option<String>)>> {
    for attr in attrs {
        if !attr.path().is_ident("rustio") {
            continue;
        }
        let mut target: Option<String> = None;
        let mut display: Option<String> = None;
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("belongs_to") {
                let value = m.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    target = Some(s.value());
                }
                Ok(())
            } else if m.path.is_ident("display") {
                let value = m.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    display = Some(s.value());
                }
                Ok(())
            } else if m.path.is_ident("file") {
                // Marker attribute — handled by `parse_file_attr`,
                // ignored here so a field can carry both
                // `belongs_to` and `file` without one parser
                // erroring on the other's keyword.
                Ok(())
            } else if m.path.is_ident("format") || m.path.is_ident("choices") {
                // Handled by `parse_format_attr` / `parse_choices_attr`.
                // Consume the value (a string literal or an array)
                // generically so this parser doesn't trip on it,
                // mirroring the `file` marker's pass-through.
                let _: syn::Expr = m.value()?.parse()?;
                Ok(())
            } else {
                Err(m.error(format!("unknown rustio attribute for field `{field_name}`")))
            }
        })?;
        if let Some(t) = target {
            return Ok(Some((t, display)));
        }
        if display.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "`display` requires `belongs_to` alongside it",
            ));
        }
    }
    // Suppress the unused warning for `Meta`.
    let _ = std::marker::PhantomData::<Meta>;
    Ok(None)
}

/// `#[rustio(file)]` marker — promotes a `String` /
/// `Option<String>` field to `FieldKind::FilePath` /
/// `FieldKind::OptionalFilePath`. The form renderer then emits
/// `<input type="file">` and the runtime's multipart-form
/// handler writes the uploaded bytes to `Admin::uploads_dir`
/// before injecting the relative path back into the form's
/// string slot.
fn parse_file_attr(attrs: &[syn::Attribute]) -> syn::Result<bool> {
    for attr in attrs {
        if !attr.path().is_ident("rustio") {
            continue;
        }
        let mut found = false;
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("file") {
                found = true;
                Ok(())
            } else if m.input.peek(syn::Token![=]) {
                // Other keys (`belongs_to = "…"`, `choices = [...]`)
                // carry an `=` and a value we must consume so the
                // parser doesn't choke on the trailing `,`. Parse as a
                // generic `Expr` so array values (`choices`) skip
                // cleanly alongside literals. We don't validate here —
                // each key's owning parser does.
                let _: syn::Expr = m.value()?.parse()?;
                Ok(())
            } else {
                // Marker key without `=` (future flags). Just skip.
                Ok(())
            }
        })?;
        if found {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Read `#[rustio(format = "email" | "phone")]` off a field. Returns
/// the lowercase format name, or `None` when no such attribute is
/// present. Any value other than `"email"` / `"phone"` is a compile
/// error pointing at the attribute span. Other `rustio` keys
/// (`belongs_to = "…"`, `display = "…"`, the `file` marker) are
/// tolerated and skipped so this parser composes with the others.
fn parse_format_attr(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if !attr.path().is_ident("rustio") {
            continue;
        }
        let mut found: Option<String> = None;
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("format") {
                let value = m.value()?;
                let lit: syn::LitStr = value.parse()?;
                let v = lit.value();
                if v != "email" && v != "phone" {
                    return Err(m.error(format!(
                        "#[rustio(format = \"...\")] accepts only \"email\" or \"phone\"; got \"{v}\""
                    )));
                }
                found = Some(v);
                Ok(())
            } else if m.input.peek(syn::Token![=]) {
                // Some other `key = value` (incl. `choices = [...]`);
                // consume it as a generic `Expr` so arrays skip
                // cleanly without tripping the trailing `,`.
                let _: syn::Expr = m.value()?.parse()?;
                Ok(())
            } else {
                // Marker key without `=` (e.g. `file`). Skip.
                Ok(())
            }
        })?;
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

/// Read `#[rustio(choices = ["a", "b", ...])]` off a field. Returns
/// the declared values in declaration order, or `None` when absent.
/// An empty array, a non-array value, or a non-string-literal element
/// is a compile error. Other `rustio` keys are skipped so this
/// composes with the file / format / relation parsers.
fn parse_choices_attr(attrs: &[syn::Attribute]) -> syn::Result<Option<Vec<String>>> {
    for attr in attrs {
        if !attr.path().is_ident("rustio") {
            continue;
        }
        let mut found: Option<Vec<String>> = None;
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("choices") {
                let array: syn::ExprArray = m.value()?.parse()?;
                let mut values = Vec::with_capacity(array.elems.len());
                for elem in &array.elems {
                    match elem {
                        syn::Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(s), ..
                        }) => values.push(s.value()),
                        other => {
                            return Err(syn::Error::new_spanned(
                                other,
                                "#[rustio(choices = [...])] elements must be string literals",
                            ));
                        }
                    }
                }
                if values.is_empty() {
                    return Err(m.error("#[rustio(choices = [...])] needs at least one value"));
                }
                found = Some(values);
                Ok(())
            } else if m.input.peek(syn::Token![=]) {
                let _: syn::Expr = m.value()?.parse()?;
                Ok(())
            } else {
                Ok(())
            }
        })?;
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

fn plural_snake(camel: &str) -> String {
    let snake = camel_to_snake(camel);
    // Regular English pluralisation. Irregular plurals (Person →
    // People, Mouse → Mice) need `#[rustio(admin_name = "...")]`.
    if snake.ends_with('s') {
        // Already ends in 's' — leave as-is so structs named in the
        // plural (`Posts`) don't become `postss`. Edge cases like
        // `Bus` → `buses` need the F1 override.
        snake
    } else if snake.ends_with('x')
        || snake.ends_with('z')
        || snake.ends_with("ch")
        || snake.ends_with("sh")
    {
        format!("{snake}es")
    } else if let Some(stem) = snake.strip_suffix('y') {
        // consonant + y → ies (Category → Categories);
        // vowel + y → s (Toy → Toys).
        let before = stem.chars().last();
        if matches!(before, Some('a' | 'e' | 'i' | 'o' | 'u')) || stem.is_empty() {
            format!("{snake}s")
        } else {
            format!("{stem}ies")
        }
    } else {
        format!("{snake}s")
    }
}

fn camel_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn humanise(snake: &str) -> String {
    // "blog_posts" → "Blog posts"
    let mut chars = snake.chars();
    let mut out = String::new();
    if let Some(first) = chars.next() {
        out.push(first.to_ascii_uppercase());
    }
    for c in chars {
        if c == '_' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn find_label_field(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::Token![,]>,
) -> Option<String> {
    // Heuristic: prefer `name`, then `title`, then `full_name`, then
    // fall through to `#id`. Keeps `object_label()` useful without
    // forcing users to implement anything.
    let names = ["name", "title", "full_name", "label", "email"];
    for candidate in names {
        if fields
            .iter()
            .any(|f| f.ident.as_ref().map(|i| i == candidate).unwrap_or(false))
        {
            return Some(candidate.to_string());
        }
    }
    None
}

#[cfg(test)]
mod plural_snake_tests {
    use super::plural_snake;

    #[test]
    fn regular_plurals() {
        assert_eq!(plural_snake("Post"), "posts");
        assert_eq!(plural_snake("Loan"), "loans");
        assert_eq!(plural_snake("BlogPost"), "blog_posts");
        assert_eq!(plural_snake("CaseAction"), "case_actions");
    }

    #[test]
    fn ch_sh_x_z_suffixes_take_es() {
        assert_eq!(plural_snake("Branch"), "branches");
        assert_eq!(plural_snake("Box"), "boxes");
        assert_eq!(plural_snake("Dish"), "dishes");
        assert_eq!(plural_snake("Buzz"), "buzzes");
    }

    #[test]
    fn consonant_y_becomes_ies_vowel_y_keeps_s() {
        assert_eq!(plural_snake("Category"), "categories");
        assert_eq!(plural_snake("Story"), "stories");
        assert_eq!(plural_snake("Toy"), "toys");
        assert_eq!(plural_snake("Day"), "days");
    }

    #[test]
    fn trailing_s_left_alone() {
        assert_eq!(plural_snake("Posts"), "posts");
        assert_eq!(plural_snake("Status"), "status");
    }
}

#[cfg(test)]
mod humanise_field_tests {
    use super::humanise_field;

    #[test]
    fn snake_case_to_title_case() {
        assert_eq!(humanise_field("title"), "Title");
        assert_eq!(humanise_field("chart_number"), "Chart Number");
        assert_eq!(humanise_field("full_name"), "Full Name");
        assert_eq!(
            humanise_field("performed_by_technician"),
            "Performed By Technician"
        );
    }

    #[test]
    fn standalone_acronyms_are_uppercased() {
        // The shipped fix: `id` no longer humanises to `Id`.
        assert_eq!(humanise_field("id"), "ID");
        assert_eq!(humanise_field("ip"), "IP");
        assert_eq!(humanise_field("url"), "URL");
        assert_eq!(humanise_field("uuid"), "UUID");
        assert_eq!(humanise_field("mfa"), "MFA");
    }

    #[test]
    fn acronyms_inside_compound_names_are_uppercased() {
        assert_eq!(humanise_field("email_id"), "Email ID");
        assert_eq!(humanise_field("id_card"), "ID Card");
        assert_eq!(humanise_field("user_ip"), "User IP");
        assert_eq!(humanise_field("api_token"), "API Token");
        assert_eq!(humanise_field("mfa_secret_key_id"), "MFA Secret Key ID");
        assert_eq!(humanise_field("csv_export_path"), "CSV Export Path");
    }

    #[test]
    fn acronym_substrings_are_not_uppercased() {
        // `id` appears inside `video` — the WORD is the unit, not
        // any embedded substring. Without this guarantee a field
        // named `video_url` would render as `vIDeo URL`.
        assert_eq!(humanise_field("video"), "Video");
        assert_eq!(humanise_field("video_url"), "Video URL");
        assert_eq!(humanise_field("hidden_field"), "Hidden Field");
        assert_eq!(humanise_field("idle_seconds"), "Idle Seconds");
    }

    #[test]
    fn empty_and_trivial_inputs_are_safe() {
        assert_eq!(humanise_field(""), "");
        assert_eq!(humanise_field("a"), "A");
    }

    #[test]
    fn datetime_suffixes_preserved() {
        // `at` / `to` / `by` are prepositions, not acronyms —
        // they stay sentence-case.
        assert_eq!(humanise_field("created_at"), "Created At");
        assert_eq!(humanise_field("revoked_by"), "Revoked By");
        assert_eq!(humanise_field("expires_at"), "Expires At");
    }
}
