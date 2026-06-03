//! The shared visual language for onboarding-facing CLI output.
//!
//! One palette, one set of helpers, used by the `new` wizard, the
//! scaffold summaries, and the post-command "Next" pointers so the
//! whole first-run reads as one calm, legible flow instead of a wall
//! of single-colour text.
//!
//! Doctrine (`DESIGN_ONBOARDING.md` §13, which supersedes the earlier
//! colourless-wizard rule):
//!
//! - **One accent** — a warm amber (256-colour 173, the Claude-tone
//!   orange). It marks the *active* thing: a step title, a prompt
//!   arrow, the "Next" label. Never decoration.
//! - **Roles, not rainbows.** Success is green (`✓`); paths and URLs
//!   are cyan; commands you type are bold; secondary text is dim.
//!   Nothing else gets colour.
//! - **No emoji, no hype.** The only marks are `✓` (done), `›`
//!   (prompt / pointer), and a dim divider. (`✗` for failures lives in
//!   `progress`/`ui`.)
//! - **Breathing room.** Stages are separated by blank lines; callers
//!   own the spacing, these helpers only colour fragments.
//! - **Degrades cleanly.** `console` strips every escape under
//!   `NO_COLOR`, non-TTY, or a dumb terminal, so piped/CI output stays
//!   plain text with the same words and spacing.

use std::io::IsTerminal;

use console::style;

/// The single accent colour — a warm amber in the 256-colour cube
/// (`#d7875f`), chosen to read as calm and on-brand rather than
/// alarming. Used only for the active element of a stage.
pub(crate) const ACCENT: u8 = 173;

/// A screen/section title (accent + bold). E.g. `RustIO  ›  new project`.
pub(crate) fn title(s: &str) -> String {
    style(s).color256(ACCENT).bold().to_string()
}

/// A stage / "Next" heading (accent, bold).
pub(crate) fn heading(s: &str) -> String {
    style(s).color256(ACCENT).bold().to_string()
}

/// Secondary explanatory text — the calm one-liners under a heading.
pub(crate) fn hint(s: &str) -> String {
    style(s).dim().to_string()
}

/// A command the developer types or pastes (bold so it stands out as
/// "this is the thing to run").
pub(crate) fn command(s: &str) -> String {
    style(s).bold().to_string()
}

/// A filesystem path (cyan).
pub(crate) fn path(s: &str) -> String {
    style(s).cyan().to_string()
}

/// A URL (cyan, underlined).
pub(crate) fn url(s: &str) -> String {
    style(s).cyan().underlined().to_string()
}

/// A value the user chose or that was generated (bold, no colour) —
/// e.g. the confirmed project name beside a `✓`.
pub(crate) fn value(s: &str) -> String {
    style(s).bold().to_string()
}

/// Accent-coloured fragment (no bold) — e.g. a list number or the
/// `›` prompt arrow.
pub(crate) fn accent(s: &str) -> String {
    style(s).color256(ACCENT).to_string()
}

/// The green success mark.
pub(crate) fn check() -> String {
    style("✓").green().to_string()
}

/// A gentle correction shown when input fails validation — yellow so
/// it is noticed, but not the red of a hard error.
pub(crate) fn warn(s: &str) -> String {
    style(s).yellow().to_string()
}

/// The accent prompt / pointer arrow.
pub(crate) fn arrow() -> String {
    style("›").color256(ACCENT).to_string()
}

/// A dim horizontal divider, sized to sit comfortably in an 80-col
/// terminal without reaching the edge.
pub(crate) fn divider() -> String {
    style("──────────────────────────────────────────")
        .dim()
        .to_string()
}

/// A stage header line: `Step 2 of 3 · Project type`, the counter in
/// accent and the title in bold, so the eye lands on "where am I".
pub(crate) fn step(n: usize, total: usize, title: &str) -> String {
    format!(
        "{} {}",
        accent(&format!("Step {n} of {total} ·")),
        value(title)
    )
}

/// A confirmation line: `  ✓ huntclick`.
pub(crate) fn confirm(value_text: &str) -> String {
    format!("  {} {}", check(), value(value_text))
}

/// True when output is going to an interactive human (a TTY, not CI).
/// The contextual "Next" pointers a command prints after it runs are
/// guidance for a person mid-flow — scripts and CI runs stay quiet.
pub(crate) fn is_interactive() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("CI").is_none()
}

/// Print a "Next — <what>:" pointer: a bold accent heading, then each
/// command on its own copy-clean line, with any (already-styled)
/// annotation on a dim line beneath it. The shared shape behind the
/// post-command guidance so `migrate`, `user create`, and the scaffold
/// all point forward the same way.
pub(crate) fn next_step(what: &str, steps: &[(String, String)]) {
    println!();
    println!("  {}", heading(&format!("Next — {what}:")));
    println!();
    for (cmd, annot) in steps {
        println!("    {}", command(cmd));
        if !annot.is_empty() {
            println!("        {annot}");
        }
    }
}
