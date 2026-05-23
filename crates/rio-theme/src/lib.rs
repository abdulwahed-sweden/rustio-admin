//! Theme engine for `rustio-admin`.
//!
//! The crate takes a client's raw brand color(s) as input and emits
//! a safe, computed `tokens.css` as output. The pipeline measures,
//! repairs, derives, and adapts the raw input — the color a client
//! hands you is *not* the color the UI ends up using.
//!
//! Build-time only: the engine runs inside `rustio-admin-cli` when a
//! developer calls `rustio theme generate`. Runtime CSS is the
//! generated static file; the runtime library never depends on this
//! crate.
//!
//! Read order: `color` → `contrast` → the six decision cases
//! (`guard`, `derive`, `vivid`, `semantic`, `adaptive`, `hierarchy`)
//! → `engine` (the orchestrator) → `emit` (CSS serialisation).

pub mod adaptive;
pub mod color;
pub mod contrast;
pub mod derive;
pub mod emit;
pub mod engine;
pub mod guard;
pub mod hierarchy;
pub mod semantic;
pub mod vivid;

pub use color::{Color, ColorError};
pub use engine::{
    resolve_theme, resolve_theme_with_report, ResolveReport, ThemeInput, ThemeTokens,
};
