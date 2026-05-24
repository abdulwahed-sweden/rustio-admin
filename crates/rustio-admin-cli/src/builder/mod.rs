//! The Builder layer.
//!
//! See `docs/design/DESIGN_BUILDER.md` for the doctrine this module
//! implements. Every submodule names the invariant it carries; the
//! doctrine grep-proofs at §10 of that document target individual
//! files in this directory.
//!
//! Foundation primitives (this commit):
//!
//! - [`canonical`] -- UTF-8 NFC + LF normalization (§4.4 #2, #3).
//! - [`toml_canon`] -- sole TOML emitter (Doctrine B1, §4.4 #4, §10.4).
//! - [`redact`] -- sole redactor (Doctrine B4, §10.5).
//! - [`ulid_gen`] -- ULID generation with stable timestamp semantics
//!   (§4.4 #6).
//! - [`history`] -- `HistoryOp` enum + `append`, the sole writer of
//!   `.rustio/history.jsonl` (Doctrine B3, §10.1).
//!
//! The [`cmd`] dispatchers translate `rustio` subcommands into
//! primitive calls. Items that surface only in tests or in unrealised
//! future verbs (e.g. `Undo`, the closed redaction category list)
//! are kept here behind a narrow `#[allow(dead_code)]` rather than
//! deleted -- they pin doctrine references and CI checks would
//! catch their loss.

#![allow(dead_code)]

pub(crate) mod canonical;
pub(crate) mod cmd;
pub(crate) mod codegen;
pub(crate) mod draft;
pub(crate) mod hash;
pub(crate) mod history;
pub(crate) mod lifecycle;
pub(crate) mod lockfile;
pub(crate) mod redact;
pub(crate) mod replay;
pub(crate) mod toml_canon;
pub(crate) mod ulid_gen;
