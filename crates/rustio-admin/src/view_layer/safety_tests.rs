//! Sensitive-field safety guarantees, kept in one place so a reviewer can see
//! the security contract at a glance: sensitive columns are inferred as
//! `Hidden`, never reach a rendered cell, and are always listed by
//! `redacted_fields()`. If someone weakens inference later, these break loudly.

#[cfg(test)]
mod tests {
    use crate::view_layer::infer::{infer_view_spec, FieldKind, FieldMeta};
    use crate::view_layer::render::{render_row, RenderedCell, RowData};
    use crate::view_layer::roles::FieldRole;

    /// The exact patterns the spec requires to be `Hidden` by default.
    const SENSITIVE_NAMES: &[&str] = &[
        "password",
        "password_hash",
        "hash",
        "pin",
        "pin_hash",
        "token",
        "secret",
        "api_key",
        "private_key",
        "session",
        "reset_token",
    ];

    fn text(name: &str) -> FieldMeta {
        FieldMeta {
            name: name.into(),
            kind: FieldKind::Text,
            nullable: false,
        }
    }

    #[test]
    fn all_sensitive_names_infer_as_hidden() {
        for name in SENSITIVE_NAMES {
            let spec = infer_view_spec("account", &[text("email"), text(name)]);
            let field = spec.fields.iter().find(|f| &f.field_name == name).unwrap();
            assert_eq!(
                field.role,
                FieldRole::Hidden,
                "expected `{name}` to be hidden by default"
            );
        }
    }

    #[test]
    fn sensitive_values_never_reach_rendered_cells() {
        let mut columns = vec![text("email")];
        columns.extend(SENSITIVE_NAMES.iter().map(|n| text(n)));
        let spec = infer_view_spec("account", &columns);

        let mut row = RowData::new();
        row.insert("email".into(), "user@example.com".into());
        for name in SENSITIVE_NAMES {
            row.insert((*name).into(), format!("LEAK_{name}"));
        }

        let rendered = render_row(&spec, &row);
        for cell in &rendered.cells {
            let value = match cell {
                RenderedCell::Primary { value, .. }
                | RenderedCell::Secondary { value, .. }
                | RenderedCell::Badge { value, .. }
                | RenderedCell::Timestamp { value, .. } => value.clone(),
                RenderedCell::Composed { parts, .. } => parts
                    .iter()
                    .map(|p| p.value.clone())
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            assert!(
                !value.contains("LEAK_"),
                "sensitive value reached a cell: {value}"
            );
        }
    }

    #[test]
    fn redacted_fields_lists_every_sensitive_column() {
        let mut columns = vec![text("email")];
        columns.extend(SENSITIVE_NAMES.iter().map(|n| text(n)));
        let spec = infer_view_spec("account", &columns);

        let redacted = spec.redacted_fields();
        for name in SENSITIVE_NAMES {
            assert!(
                redacted.contains(name),
                "`{name}` missing from redacted set"
            );
        }
    }
}
