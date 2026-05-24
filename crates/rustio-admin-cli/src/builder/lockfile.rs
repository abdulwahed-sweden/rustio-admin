//! `.rustio/builder.lock` ----- Builder version pin (Doctrine B11).
//!
//! `DESIGN_BUILDER.md` §4.3: *"Every Builder command except
//! `rustio upgrade` reads `builder.lock` first and refuses to
//! proceed if the executing Builder's semver does not match."*
//!
//! Reproducibility invariant §4.4 #7 binds on this file: the
//! Builder version is part of the canonical environment. A
//! mismatch is not a soft warning; it is a refusal.

use toml_edit::{value, DocumentMut, Item};

use crate::builder::toml_canon::{emit_canonical, parse, TomlError};

/// Schema version of the lockfile format itself. Distinct from
/// `Draft::schema_version`.
pub(crate) const LOCK_SCHEMA_V: u32 = 1;

/// The canonical emitter identifier baked into every generated
/// file's `SPDX-EmitterVersion` header. Bumps require a doctrine
/// amendment and a corresponding `schema_upgraded` event.
pub(crate) const EMITTER_VERSION: &str = "rio-canon-1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuilderLock {
    pub schema_version: u32,
    /// Exact Builder semver ----- sourced from `CARGO_PKG_VERSION` at
    /// `rustio new` time.
    pub builder: String,
    /// Canonical TOML emitter identifier. Encoded in every
    /// generated file's header so a future emitter bump is
    /// traceable.
    pub toml_emitter: String,
    /// Per §4.2.5; off by default in MVP.
    pub chain_mode: bool,
}

#[derive(Debug)]
pub(crate) enum LockError {
    Toml(TomlError),
    UnsupportedSchemaVersion(u32),
    Missing(&'static str),
    /// The lockfile pins a Builder version different from the one
    /// currently executing. Doctrine B11 refusal point.
    VersionMismatch {
        pinned: String,
        running: String,
    },
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Toml(e) => write!(f, "{e}"),
            LockError::UnsupportedSchemaVersion(v) => write!(
                f,
                "builder.lock schema_version {v} is not supported by this Builder \
                 (this Builder understands schema_version {LOCK_SCHEMA_V})"
            ),
            LockError::Missing(k) => write!(f, "builder.lock is missing required key '{k}'"),
            LockError::VersionMismatch { pinned, running } => write!(
                f,
                "builder.lock pins Builder {pinned} but this is Builder {running}. \
                 Doctrine B11 forbids running mismatched versions. \
                 Run `rustio upgrade` to migrate the project."
            ),
        }
    }
}

impl std::error::Error for LockError {}

impl From<TomlError> for LockError {
    fn from(e: TomlError) -> Self {
        LockError::Toml(e)
    }
}

impl BuilderLock {
    /// Construct a fresh lockfile pinning the executing Builder.
    pub(crate) fn current() -> Self {
        Self {
            schema_version: LOCK_SCHEMA_V,
            builder: env!("CARGO_PKG_VERSION").to_string(),
            toml_emitter: EMITTER_VERSION.to_string(),
            chain_mode: false,
        }
    }

    pub(crate) fn to_toml(&self) -> String {
        let mut doc = DocumentMut::new();
        doc["builder"] = value(self.builder.clone());
        doc["chain_mode"] = value(self.chain_mode);
        doc["schema_version"] = value(i64::from(self.schema_version));
        doc["toml_emitter"] = value(self.toml_emitter.clone());
        emit_canonical(&doc)
    }

    pub(crate) fn from_toml(input: &str) -> Result<Self, LockError> {
        let doc = parse(input)?;
        let schema_version = doc
            .get("schema_version")
            .and_then(Item::as_integer)
            .ok_or(LockError::Missing("schema_version"))? as u32;
        if schema_version != LOCK_SCHEMA_V {
            return Err(LockError::UnsupportedSchemaVersion(schema_version));
        }
        Ok(Self {
            schema_version,
            builder: doc
                .get("builder")
                .and_then(Item::as_str)
                .ok_or(LockError::Missing("builder"))?
                .to_string(),
            toml_emitter: doc
                .get("toml_emitter")
                .and_then(Item::as_str)
                .ok_or(LockError::Missing("toml_emitter"))?
                .to_string(),
            chain_mode: doc
                .get("chain_mode")
                .and_then(Item::as_bool)
                .ok_or(LockError::Missing("chain_mode"))?,
        })
    }

    /// Doctrine B11 enforcement point. Refuses the operation if the
    /// pinned Builder is not the one currently running.
    pub(crate) fn verify_against_running(&self) -> Result<(), LockError> {
        let running = env!("CARGO_PKG_VERSION");
        if self.builder != running {
            return Err(LockError::VersionMismatch {
                pinned: self.builder.clone(),
                running: running.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let lock = BuilderLock::current();
        let s = lock.to_toml();
        let back = BuilderLock::from_toml(&s).unwrap();
        assert_eq!(back, lock);
    }

    #[test]
    fn current_pins_running_version() {
        let lock = BuilderLock::current();
        assert_eq!(lock.builder, env!("CARGO_PKG_VERSION"));
        assert_eq!(lock.toml_emitter, EMITTER_VERSION);
        assert!(!lock.chain_mode);
    }

    #[test]
    fn verify_passes_on_match() {
        let lock = BuilderLock::current();
        assert!(lock.verify_against_running().is_ok());
    }

    #[test]
    fn verify_refuses_on_mismatch() {
        let mut lock = BuilderLock::current();
        lock.builder = "99.99.99".into();
        let err = lock.verify_against_running().expect_err("must refuse");
        assert!(matches!(err, LockError::VersionMismatch { .. }));
        // Error message names both versions and points at the
        // upgrade command.
        let msg = format!("{err}");
        assert!(msg.contains("99.99.99"));
        assert!(msg.contains("rustio upgrade"));
    }

    #[test]
    fn unknown_schema_version_refused() {
        let mut doc = DocumentMut::new();
        doc["builder"] = value("0.0.1");
        doc["chain_mode"] = value(false);
        doc["schema_version"] = value(99_i64);
        doc["toml_emitter"] = value("rio-canon-1");
        let s = emit_canonical(&doc);
        assert!(matches!(
            BuilderLock::from_toml(&s),
            Err(LockError::UnsupportedSchemaVersion(99))
        ));
    }
}
