//! Self-service password recovery (R1).
//!
//! See `DESIGN_RECOVERY.md` for the canonical contract this module
//! implements. R1 ships in 0.5.0; this commit lands the schema only —
//! the trait surface (`PasswordPolicy`, `RecoveryPolicy`), the issue +
//! consume flow, the mailer wiring, the routes, and the templates land
//! in subsequent atomic commits per `DESIGN_RECOVERY.md` §16.
//!
//! ## What lives here today
//!
//! - [`init_recovery_tables`] — creates `rustio_password_reset_tokens`
//!   with the partial unique index that makes the consume path's atomic
//!   `UPDATE … RETURNING` an index seek (`DESIGN_RECOVERY.md` §9.1).
//! - [`migrate_user_recovery_schema`] — adds the additive
//!   `must_change_password` and `password_changed_at` columns on
//!   `rustio_users` (§9.2). R1's `set_password` populates
//!   `password_changed_at`; R2 enforces `must_change_password`.
//!
//! Both functions are idempotent and safe to call on every boot.
//! `auth::init_tables` invokes them after the existing user / session
//! migrations.

use crate::error::Result;
use crate::orm::Db;

/// Create the `rustio_password_reset_tokens` table and its indexes.
///
/// Schema (see `DESIGN_RECOVERY.md` §9.1 for the contract):
///
/// - `token_hash` is `sha256(token)` URL-safe-base64 — the plaintext
///   token never lands in this row.
/// - `mail_status` is one of `'pending' | 'sent' | 'failed'`; the state
///   evolves in the issue handler (one row per request).
/// - `correlation_id` mirrors the request's audit `correlation_id` so
///   an operator can pivot from token row → audit chain.
/// - The partial unique index `WHERE consumed_at IS NULL` is the index
///   the atomic consume statement seeks on.
///
/// Idempotent. Safe to call on every boot. Depends on `rustio_users`
/// existing first.
pub(crate) async fn init_recovery_tables(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_password_reset_tokens (
            id                    BIGSERIAL   PRIMARY KEY,
            user_id               BIGINT      NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
            token_hash            TEXT        NOT NULL,
            requested_ip          TEXT,
            requested_user_agent  TEXT,
            requested_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            expires_at            TIMESTAMPTZ NOT NULL,
            consumed_at           TIMESTAMPTZ,
            mail_status           TEXT        NOT NULL DEFAULT 'pending'
                                  CHECK (mail_status IN ('pending', 'sent', 'failed')),
            correlation_id        TEXT
        )",
    )
    .execute(db.pool())
    .await?;

    // Partial unique on the active-token lookup. Guarantees the
    // consume statement (`UPDATE … WHERE token_hash = $1 AND
    // consumed_at IS NULL RETURNING …`) is an index seek even after
    // the table accumulates consumed/expired rows for forensic
    // retention.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS rustio_password_reset_tokens_active_uq \
         ON rustio_password_reset_tokens (token_hash) \
         WHERE consumed_at IS NULL",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_user_idx \
         ON rustio_password_reset_tokens (user_id)",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_expires_idx \
         ON rustio_password_reset_tokens (expires_at) \
         WHERE consumed_at IS NULL",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

/// Add the additive recovery columns on `rustio_users`.
///
/// - `must_change_password BOOLEAN NOT NULL DEFAULT FALSE` — R2 will
///   read this on login to force a password reset on the next sign-in.
///   R1 introduces the column because R2's commit set stays narrower
///   when the column already exists.
/// - `password_changed_at TIMESTAMPTZ` (nullable) — populated by
///   `auth::set_password` from R1 onwards. NULL for users created
///   before the upgrade; the active-sessions UI renders "(unknown)" or
///   omits the row when NULL.
///
/// Idempotent. Safe to call on every boot. Depends on `rustio_users`
/// existing first.
pub(crate) async fn migrate_user_recovery_schema(db: &Db) -> Result<()> {
    sqlx::query(
        "ALTER TABLE rustio_users \
         ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS password_changed_at TIMESTAMPTZ")
        .execute(db.pool())
        .await?;

    Ok(())
}
