//! Spinner and status-feedback helpers for onboarding-facing CLI
//! operations.
//!
//! PR 1.4 of the FTUX redesign. Goal: prevent the "did it freeze?"
//! anxiety on long-running operations (DB connect, migration apply,
//! doctor checks) without becoming theatrical.
//!
//! Discipline (`DESIGN_ONBOARDING.md` §9):
//!
//! - One spinner style. One success marker (`✓`). One failure
//!   marker (`✗`). No emoji, no animated banners, no celebration.
//! - Spinners draw to stderr; command output stays on stdout
//!   undisturbed.
//! - Spinners are suppressed entirely when stderr is not a
//!   terminal (pipes, redirection, CI), when `NO_COLOR` is set,
//!   under CI (`CI` env var), or when the user passed `--quiet` /
//!   `--no-progress`. The result line still prints where it
//!   carries meaning (doctor checks); silent paths just clear.
//! - No global TUI state. One `Step` per operation, RAII-shaped.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

/// Set once by `main()` before dispatch, based on the global
/// `--quiet` / `--no-progress` flags on [`crate::Cli`]. Read by
/// [`animation_allowed`] alongside the standard env-based checks.
static SUPPRESS_ANIMATION: AtomicBool = AtomicBool::new(false);

/// Wire the global CLI flags into the progress layer. Must be
/// called before any `Step::start(...)`.
pub(crate) fn configure(quiet: bool, no_progress: bool) {
    if quiet || no_progress {
        SUPPRESS_ANIMATION.store(true, Ordering::Relaxed);
    }
}

/// True when an animated spinner is appropriate. False under
/// non-TTY stderr, `NO_COLOR`, `CI`, `--quiet`, or `--no-progress`.
fn animation_allowed() -> bool {
    if SUPPRESS_ANIMATION.load(Ordering::Relaxed) {
        return false;
    }
    if std::env::var_os("CI").is_some() {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stderr().is_terminal()
}

/// One in-progress step. Created via [`Step::start`]; the caller
/// must invoke exactly one of [`Step::done_with`],
/// [`Step::failed_with`], or [`Step::clear`] when the underlying
/// operation completes.
///
/// Stream convention: the spinner ticks on stderr (progress is
/// feedback, not data). The `✓ <summary>` result line is always
/// emitted on stdout, the `✗ <summary>` failure line on stderr —
/// matching the rest of the CLI, so scripts that grep stdout for
/// `✓` keep working in both animated and non-animated modes.
pub(crate) struct Step {
    spinner: Option<ProgressBar>,
}

impl Step {
    /// Start a step. In animated mode the spinner ticks on stderr;
    /// otherwise this is a no-op until the caller finishes it.
    pub(crate) fn start(label: impl AsRef<str>) -> Self {
        let label = label.as_ref();
        let spinner = if animation_allowed() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template("{spinner:.cyan} {msg}")
                    .expect("static template")
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.set_message(format!("{label}…"));
            pb.enable_steady_tick(Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };
        Self { spinner }
    }

    /// Clear the spinner; print `✓ <summary>` to stdout.
    pub(crate) fn done_with(self, summary: impl AsRef<str>) {
        self.clear_spinner();
        println!("✓ {}", summary.as_ref());
    }

    /// Clear the spinner; print `✗ <summary>` to stderr.
    pub(crate) fn failed_with(self, summary: impl AsRef<str>) {
        self.clear_spinner();
        eprintln!("✗ {}", summary.as_ref());
    }

    /// Clear the spinner without printing anything. Used on the
    /// happy path of `main::db()` and `migrate::apply` where the
    /// surrounding command produces its own output and an extra
    /// `✓ X` line would just add noise.
    pub(crate) fn clear(self) {
        self.clear_spinner();
    }

    fn clear_spinner(&self) {
        if let Some(pb) = &self.spinner {
            pb.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Step::start` followed immediately by `clear` is the silent
    /// happy-path shape used by `main::db()`. Must not panic in any
    /// environment, including when no spinner is created.
    #[test]
    fn start_then_clear_is_safe_in_non_tty_mode() {
        // Tests run with stderr captured (non-TTY) → no spinner.
        let step = Step::start("test");
        step.clear();
    }

    #[test]
    fn start_then_done_with_in_non_tty_does_not_panic() {
        // In non-animated mode `done_with` falls through to a
        // bare println!; constructing and finishing the step must
        // not panic in any environment.
        let step = Step::start("inner step");
        step.done_with("inner step ok");
    }

    #[test]
    fn start_then_failed_with_in_non_tty_does_not_panic() {
        let step = Step::start("inner step");
        step.failed_with("inner step broke");
    }

    #[test]
    fn configure_short_circuits_animation() {
        // Setting --quiet ought to suppress animation even when
        // every other env signal says "go". Idempotent: setting
        // again is fine.
        configure(true, false);
        assert!(!animation_allowed());
        configure(false, true);
        assert!(!animation_allowed());
        // We deliberately don't try to flip the flag back to false
        // here — the static is set-once for the process lifetime.
    }
}
