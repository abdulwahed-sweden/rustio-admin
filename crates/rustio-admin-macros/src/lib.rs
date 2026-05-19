//! Procedural macros for `rustio-admin`.
//!
//! `#[derive(RustioAdmin)]`. Given a user-written struct, the derive
//! emits `impl AdminModel for TheStruct` with `ADMIN_NAME`,
//! `DISPLAY_NAME`, `SINGULAR_NAME`, `FIELDS`, and the row/form/update
//! helpers.
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
        let editable = fname_str != "id" && kind != FieldKind::DateTimeAuto;

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
        field_metas.push(quote! {
            ::rustio_admin::admin::AdminField {
                name: #fname_str,
                label: #humanised_label,
                field_type: ::rustio_admin::admin::FieldType::#type_variant,
                editable: #editable,
                relation: #relation_tokens,
                // Derived models don't carry enum choices yet. A future
                // macro pass will accept `#[rustio(choices = [...])]`
                // and populate this; today consumers that want a
                // `<select>` backed by a static value list set this on
                // the generated AdminField directly.
                choices: ::std::option::Option::None,
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
            FieldKind::String | FieldKind::FilePath => quote! {
                out.push((#fname_str.to_string(), self.#fname.clone()));
            },
            FieldKind::OptionalString | FieldKind::OptionalFilePath => quote! {
                // `Option<String>` does not implement `Display`, so we
                // can't share the String arm. None → empty string,
                // Some(v) → v.
                out.push((#fname_str.to_string(), match &self.#fname {
                    Some(v) => v.clone(),
                    None => String::new(),
                }));
            },
            FieldKind::I32 | FieldKind::I64 => quote! {
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
                            match ::chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M") {
                                Ok(dt) => ::chrono::DateTime::<::chrono::Utc>::from_naive_utc_and_offset(dt, ::chrono::Utc),
                                Err(_) => { errors.push(#date_invalid_msg.to_string()); ::chrono::Utc::now() }
                            }
                        }
                        _ => { errors.push(#required_msg.to_string()); ::chrono::Utc::now() }
                    };
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::DateTimeAuto => {
                // created_at-style fields default to now().
                from_form_parses.push(quote! {
                    let #fname = ::chrono::Utc::now();
                });
                from_form_fields.push(quote! { #fname });
            }
            FieldKind::OptionalDateTime => {
                // Symmetric to `OptionalI64`: blank → None (legitimate),
                // garbage → validation error + None (NOT silently
                // defaulted to `Utc::now()` like the non-optional arm).
                from_form_parses.push(quote! {
                    let #fname: ::std::option::Option<::chrono::DateTime<::chrono::Utc>> =
                        match form.get(#fname_str).map(str::trim) {
                            None | Some("") => ::std::option::Option::None,
                            Some(raw) => match ::chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M") {
                                Ok(dt) => ::std::option::Option::Some(
                                    ::chrono::DateTime::<::chrono::Utc>::from_naive_utc_and_offset(dt, ::chrono::Utc),
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
    Bool,
    String,
    DateTime,
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
            FieldKind::Bool => format_ident!("Bool"),
            FieldKind::String => format_ident!("String"),
            FieldKind::DateTime | FieldKind::DateTimeAuto => format_ident!("DateTime"),
            FieldKind::OptionalString => format_ident!("OptionalString"),
            FieldKind::OptionalI64 => format_ident!("OptionalI64"),
            FieldKind::OptionalDateTime => format_ident!("OptionalDateTime"),
            FieldKind::FilePath => format_ident!("FilePath"),
            FieldKind::OptionalFilePath => format_ident!("OptionalFilePath"),
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
fn humanise_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut next_upper = true;
    for ch in s.chars() {
        if ch == '_' {
            out.push(' ');
            next_upper = true;
        } else if next_upper {
            out.push(ch.to_ascii_uppercase());
            next_upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn classify_type(ty: &syn::Type) -> syn::Result<FieldKind> {
    let as_string = quote! { #ty }.to_string().replace(' ', "");
    let kind = match as_string.as_str() {
        "i32" => FieldKind::I32,
        "i64" => FieldKind::I64,
        "bool" => FieldKind::Bool,
        "String" => FieldKind::String,
        "DateTime<Utc>" | "chrono::DateTime<chrono::Utc>" => FieldKind::DateTime,
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
            } else {
                // Field-level keys (e.g. `belongs_to`, `display`)
                // legitimately appear on `#[rustio(...)]` placed on
                // FIELDS, not the struct. When the same `rustio`
                // attribute is on the struct, those keys are
                // surprising. Reject so a misplaced field attribute
                // doesn't silently fail.
                Err(m.error(
                    "unknown rustio struct attribute; expected `admin_name` or `display_name`",
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
                // Other keys (`belongs_to = "…"`, `display = "…"`)
                // carry an `=` and a literal we must consume so the
                // parser doesn't choke on the trailing `,`. We don't
                // validate the value here — `parse_relation_attr`
                // owns the surface; this is just lexer-level skip.
                let _value = m.value()?;
                let _: Lit = _value.parse()?;
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
