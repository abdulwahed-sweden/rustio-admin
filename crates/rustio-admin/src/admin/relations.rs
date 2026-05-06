//! Relation Intelligence Layer — runtime registry.
//!
//! The admin renders, filters, and guards deletes on foreign keys by
//! consulting a `RelationRegistry` built pure-functionally from the
//! current `[AdminEntry]` list. The registry itself is data; it does
//! no I/O and holds no connections.
//!
//! Tier 1 supports only `BelongsTo` relations (declared via
//! `#[rustio(belongs_to = "Target")]` on the struct field). The
//! legacy schema-driven variant (built from `crate::schema::Schema`)
//! has been replaced with the AdminEntry-driven build below — Tier 1
//! has no separate schema layer.
//!
//! ## Two lookup tables, computed once per Admin reload
//!
//! - `belongs_to[(model, field)] → ResolvedRelation` — forward direction.
//! - `has_many[model] → Vec<InverseRelation>` — every incoming edge
//!   into `model`, used by the inverse-panel renderer and the delete
//!   guard.

use std::collections::HashMap;

use super::types::AdminEntry;

/// Soft cap on the number of rows a relation filter will expose as a
/// `<select>` dropdown. Above this threshold the admin renders a
/// numeric-id input instead. 500 was chosen to fit comfortably in
/// one HTTP round-trip.
pub const RELATION_FILTER_DROPDOWN_CAP: usize = 500;

/// One forward (`BelongsTo`) relation resolved against the current
/// admin registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRelation {
    /// Model name holding the FK column, e.g. `"Appointment"`.
    pub source_model: String,
    /// Field on the source carrying the id, e.g. `"patient_id"`.
    pub source_field: String,
    /// Target model name, e.g. `"Patient"`.
    pub target_model: String,
    /// Target model's SQL table.
    pub target_table: String,
    /// Target model's admin slug (`/admin/<slug>/<id>`).
    pub target_admin_name: String,
    /// Column on the target whose value is rendered as the human
    /// label. `None` means the admin renders `#<id>` and does NOT
    /// infer a column.
    pub target_display_field: Option<String>,
}

/// One reverse (`HasMany`) relation — an incoming edge pointing at a
/// given target model. Produced by inverting every stored `BelongsTo`
/// at registry-build time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InverseRelation {
    /// Source model holding the FK, e.g. `"Appointment"`.
    pub source_model: String,
    /// Source model's SQL table.
    pub source_table: String,
    /// Source model's admin slug for filter links.
    pub source_admin_name: String,
    /// Source model's display name (plural) — used as the panel heading.
    pub source_display_name: String,
    /// Field on the source pointing at `target_model.id`.
    pub source_field: String,
    /// Target model name — supplied for symmetry with [`ResolvedRelation`].
    pub target_model: String,
}

/// Why a [`RelationRegistry`] declaration was rejected.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// `#[rustio(belongs_to = "X")]` on `<model>.<field>` but `X`
    /// isn't a registered admin entry.
    UnknownTarget {
        model: String,
        field: String,
        target: String,
    },
    /// `display = "col"` but `col` isn't a field on the target model.
    UnknownDisplayField {
        model: String,
        field: String,
        target: String,
        display: String,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTarget {
                model,
                field,
                target,
            } => write!(
                f,
                "`{model}.{field}` declares `belongs_to = \"{target}\"`, \
                 but no admin entry named `{target}` is registered"
            ),
            Self::UnknownDisplayField {
                model,
                field,
                target,
                display,
            } => write!(
                f,
                "`{model}.{field}` declares `display = \"{display}\"` against `{target}`, \
                 but `{target}` has no field named `{display}`"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Relation lookup tables for one snapshot of the admin registration.
#[derive(Debug, Clone, Default)]
pub struct RelationRegistry {
    belongs_to: HashMap<(String, String), ResolvedRelation>,
    has_many: HashMap<String, Vec<InverseRelation>>,
    /// Forward relations indexed by source model.
    belongs_to_of: HashMap<String, Vec<ResolvedRelation>>,
}

impl RelationRegistry {
    /// Empty registry. Every lookup returns `None`.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build the registry from the current admin entries. Silent on
    /// unknown targets / display fields — call [`validate`](Self::validate)
    /// after if you want those surfaced as errors.
    pub fn from_admin_entries(entries: &[AdminEntry]) -> Self {
        let mut belongs_to: HashMap<(String, String), ResolvedRelation> = HashMap::new();
        let mut has_many: HashMap<String, Vec<InverseRelation>> = HashMap::new();
        let mut belongs_to_of: HashMap<String, Vec<ResolvedRelation>> = HashMap::new();

        // Index by singular name for O(1) target lookup.
        let by_singular: HashMap<&str, &AdminEntry> =
            entries.iter().map(|e| (e.singular_name, e)).collect();

        for source in entries {
            for field in source.fields {
                let Some(rel) = &field.relation else {
                    continue;
                };
                let Some(target) = by_singular.get(rel.target_model) else {
                    continue;
                };
                // Validate display_field against target's fields, drop
                // silently if missing; `validate` surfaces it.
                let display_field = match rel.display_field {
                    None => None,
                    Some(col) => {
                        if target.fields.iter().any(|f| f.name == col) {
                            Some(col.to_string())
                        } else {
                            None
                        }
                    }
                };

                let resolved = ResolvedRelation {
                    source_model: source.singular_name.to_string(),
                    source_field: field.name.to_string(),
                    target_model: target.singular_name.to_string(),
                    target_table: target.table.to_string(),
                    target_admin_name: target.admin_name.to_string(),
                    target_display_field: display_field,
                };

                belongs_to.insert(
                    (source.singular_name.to_string(), field.name.to_string()),
                    resolved.clone(),
                );

                belongs_to_of
                    .entry(source.singular_name.to_string())
                    .or_default()
                    .push(resolved.clone());

                has_many
                    .entry(target.singular_name.to_string())
                    .or_default()
                    .push(InverseRelation {
                        source_model: source.singular_name.to_string(),
                        source_table: source.table.to_string(),
                        source_admin_name: source.admin_name.to_string(),
                        source_display_name: source.display_name.to_string(),
                        source_field: field.name.to_string(),
                        target_model: target.singular_name.to_string(),
                    });
            }
        }

        // Deterministic order so panel rendering is stable across runs.
        for list in has_many.values_mut() {
            list.sort_by(|a, b| a.source_model.cmp(&b.source_model));
        }
        for list in belongs_to_of.values_mut() {
            list.sort_by(|a, b| a.source_field.cmp(&b.source_field));
        }

        Self {
            belongs_to,
            has_many,
            belongs_to_of,
        }
    }

    /// The `ResolvedRelation` for `(model, field)`, if any.
    pub fn belongs_to(&self, model: &str, field: &str) -> Option<&ResolvedRelation> {
        self.belongs_to.get(&(model.to_string(), field.to_string()))
    }

    /// Every forward relation owned by a source model.
    pub fn belongs_to_of(&self, model: &str) -> &[ResolvedRelation] {
        self.belongs_to_of
            .get(model)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Every incoming edge into `model`. Used by the inverse-panel
    /// renderer and the delete guard.
    pub fn has_many(&self, model: &str) -> &[InverseRelation] {
        self.has_many
            .get(model)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// `true` if the registry knows no relations at all.
    pub fn is_empty(&self) -> bool {
        self.belongs_to.is_empty()
    }

    /// Walk every stored relation and report declarations that
    /// reference models or columns not present in the current admin.
    pub fn validate(&self, entries: &[AdminEntry]) -> Vec<RegistryError> {
        let mut errors: Vec<RegistryError> = Vec::new();
        let by_singular: HashMap<&str, &AdminEntry> =
            entries.iter().map(|e| (e.singular_name, e)).collect();

        for source in entries {
            for field in source.fields {
                let Some(rel) = &field.relation else {
                    continue;
                };
                let Some(target) = by_singular.get(rel.target_model) else {
                    errors.push(RegistryError::UnknownTarget {
                        model: source.singular_name.to_string(),
                        field: field.name.to_string(),
                        target: rel.target_model.to_string(),
                    });
                    continue;
                };
                if let Some(display) = rel.display_field {
                    if !target.fields.iter().any(|f| f.name == display) {
                        errors.push(RegistryError::UnknownDisplayField {
                            model: source.singular_name.to_string(),
                            field: field.name.to_string(),
                            target: rel.target_model.to_string(),
                            display: display.to_string(),
                        });
                    }
                }
            }
        }

        errors
    }

    /// A forward iterator over every ResolvedRelation in the registry,
    /// in deterministic order.
    pub fn iter_belongs_to(&self) -> impl Iterator<Item = &ResolvedRelation> {
        let mut entries: Vec<&ResolvedRelation> = self.belongs_to.values().collect();
        entries.sort_by(|a, b| {
            a.source_model
                .cmp(&b.source_model)
                .then_with(|| a.source_field.cmp(&b.source_field))
        });
        entries.into_iter()
    }
}
