//! Persistence for adaptive-view-layer [`ViewSpec`]s.
//!
//! One saved spec per model (global, not per-user): a model's visual contract
//! is a developer-authored display config, not a personal preference. The
//! store mirrors the lazy-`ensure_table` posture of [`super::feature_flags`]
//! and [`super::saved_filters`] — no boot-time migration; the table is created
//! on first read/write.
//!
//! This store is the **only authority** for the view layer: the designer
//! ([`super::handlers`]) and the future renderer read what is saved here, and
//! the deterministic [`crate::view_layer::infer_view_spec_from_fields`] only
//! ever *produces* a draft to seed an edit — it never writes.

use sqlx::Row as _;

use crate::orm::Db;
use crate::view_layer::ViewSpec;
use crate::Result;

/// One row per model. `spec_json` is a serialized [`ViewSpec`]; keeping it as
/// opaque TEXT lets the spec's serde shape evolve (it is versioned) without a
/// schema migration.
const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS rustio_admin_view_specs (
    model      TEXT        PRIMARY KEY,
    spec_json  TEXT        NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)";

// internal:
/// Ensure the `rustio_admin_view_specs` table exists. Idempotent.
pub(crate) async fn ensure_table(db: &Db) -> Result<()> {
    sqlx::query(CREATE_TABLE_SQL).execute(db.pool()).await?;
    Ok(())
}

// internal:
/// Load the saved [`ViewSpec`] for a model, if one has been authored. Returns
/// `None` when no spec exists — callers then fall back to the legacy table
/// path or a fresh inferred draft. A stored row that fails to deserialize
/// surfaces as `Err` (a malformed save is a real problem, not a silent miss).
pub async fn load(db: &Db, model: &str) -> Result<Option<ViewSpec>> {
    ensure_table(db).await?;
    let row = sqlx::query("SELECT spec_json FROM rustio_admin_view_specs WHERE model = $1")
        .bind(model)
        .fetch_optional(db.pool())
        .await?;
    match row {
        Some(r) => {
            let json: String = r.try_get("spec_json")?;
            // serde_json::Error -> Error::BadRequest via From.
            let spec: ViewSpec = serde_json::from_str(&json)?;
            Ok(Some(spec))
        }
        None => Ok(None),
    }
}

// internal:
/// The set of model slugs that have a saved spec. Powers the designer index's
/// "saved / draft" marker without loading every spec body.
pub async fn saved_models(db: &Db) -> Result<Vec<String>> {
    ensure_table(db).await?;
    let rows = sqlx::query("SELECT model FROM rustio_admin_view_specs ORDER BY model ASC")
        .fetch_all(db.pool())
        .await?;
    Ok(rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("model").ok())
        .collect())
}

// internal:
/// Insert or replace the saved spec for a model. The `model` key is taken from
/// the argument, not `spec.model`, so the caller controls the row identity;
/// they are expected to match.
pub async fn save(db: &Db, model: &str, spec: &ViewSpec) -> Result<()> {
    ensure_table(db).await?;
    let json = serde_json::to_string(spec)?;
    sqlx::query(
        "INSERT INTO rustio_admin_view_specs (model, spec_json, updated_at) \
         VALUES ($1, $2, NOW()) \
         ON CONFLICT (model) DO UPDATE SET spec_json = EXCLUDED.spec_json, updated_at = NOW()",
    )
    .bind(model)
    .bind(json)
    .execute(db.pool())
    .await?;
    Ok(())
}
