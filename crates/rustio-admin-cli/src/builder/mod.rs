//! The Builder layer.
//!
//! See `docs/design/DESIGN_BUILDER.md` for the doctrine this module
//! implements. Every submodule names the invariant it carries; the
//! doctrine grep-proofs at §10 of that document target individual
//! files in this directory.
//!
//! Foundation primitives (this commit):
//!
//! - [`canonical`] — UTF-8 NFC + LF normalization (§4.4 #2, #3).
//! - [`toml_canon`] — sole TOML emitter (Doctrine B1, §4.4 #4, §10.4).
//! - [`redact`] — sole redactor (Doctrine B4, §10.5).
//! - [`ulid_gen`] — ULID generation with stable timestamp semantics
//!   (§4.4 #6).
//! - [`history`] — `HistoryOp` enum + `append`, the sole writer of
//!   `.rustio/history.jsonl` (Doctrine B3, §10.1).
//!
//! Later commits will add: schema types ([`draft`]), version pin
//! ([`lockfile`]), [`hash`] projection, [`replay`], [`codegen`],
//! [`plan`], [`commit`], and the [`cmd`] verb dispatchers.

// The foundation modules below are dead code until the lifecycle
// commits wire them up. Each primitive carries its own tests; the
// dead-code allowance is removed when the verb dispatchers land
// and start calling these functions.
#![allow(dead_code)]

pub(crate) mod canonical;
pub(crate) mod draft;
pub(crate) mod hash;
pub(crate) mod history;
pub(crate) mod lockfile;
pub(crate) mod redact;
pub(crate) mod replay;
pub(crate) mod toml_canon;
pub(crate) mod ulid_gen;
