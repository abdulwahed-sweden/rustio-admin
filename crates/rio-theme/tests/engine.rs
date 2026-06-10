//! Golden-file tests for the end-to-end pipeline.
//!
//! Each fixture is a `(name, ThemeInput)` pair; the test runs the
//! engine, emits CSS, and asserts byte-equality against a checked-in
//! file under `tests/golden/<name>.css`.
//!
//! To re-bless the golden files (after an intentional engine change),
//! run with `RIO_THEME_BLESS=1`:
//!
//! ```sh
//! RIO_THEME_BLESS=1 cargo test -p rio-theme --test engine
//! ```
//!
//! Determinism is required: `resolve_theme` is pure, so unchanged
//! input MUST produce byte-identical output across runs and platforms.

use std::fs;
use std::path::PathBuf;

use rio_theme::{emit, resolve_theme, Color, DarkPolicy, ThemeInput};

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(format!("{name}.css"))
}

fn check(name: &str, input: ThemeInput) {
    let tokens = resolve_theme(input);
    let actual = emit::emit(&tokens);
    let path = golden_path(name);

    if std::env::var("RIO_THEME_BLESS").is_ok() {
        fs::write(&path, &actual)
            .unwrap_or_else(|e| panic!("could not bless {}: {e}", path.display()));
        return;
    }

    let expected = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "could not read golden {}: {e}\n\n\
             If this is a new fixture, bless it with:\n\
             RIO_THEME_BLESS=1 cargo test -p rio-theme --test engine\n\n\
             --- engine output ---\n{actual}",
            path.display()
        )
    });

    assert_eq!(
        actual, expected,
        "\ngolden {} drifted.\n\nIf this change is intentional, re-bless with:\n  RIO_THEME_BLESS=1 cargo test -p rio-theme --test engine\n\n--- expected ---\n{expected}\n--- actual ---\n{actual}",
        path.display()
    );
}

/// Golden check for an explicit [`DarkPolicy`] (the dark-block cases).
fn check_with(name: &str, input: ThemeInput, dark: DarkPolicy) {
    let tokens = resolve_theme(input);
    let actual = emit::emit_with(&tokens, &dark);
    let path = golden_path(name);

    if std::env::var("RIO_THEME_BLESS").is_ok() {
        fs::write(&path, &actual)
            .unwrap_or_else(|e| panic!("could not bless {}: {e}", path.display()));
        return;
    }

    let expected = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "could not read golden {}: {e}\n\n\
             If this is a new fixture, bless it with:\n\
             RIO_THEME_BLESS=1 cargo test -p rio-theme --test engine\n\n\
             --- engine output ---\n{actual}",
            path.display()
        )
    });

    assert_eq!(
        actual, expected,
        "\ngolden {} drifted.\n\nIf this change is intentional, re-bless with:\n  RIO_THEME_BLESS=1 cargo test -p rio-theme --test engine\n\n--- expected ---\n{expected}\n--- actual ---\n{actual}",
        path.display()
    );
}

fn hex(s: &str) -> Color {
    Color::from_hex(s).expect("test fixture should parse")
}

// --- Dark-block golden cases (the three DarkPolicy derivations) ---
// All use the shop's Patina brand so the goldens double as a reference
// for what the rustio-design repo must emit (docs/design/TOKENS-EMIT-SPEC.md).

#[test]
fn dark_auto_derived() {
    check_with(
        "dark_auto",
        ThemeInput {
            brand_colors: vec![hex("#0E6B5B")],
        },
        DarkPolicy::Auto,
    );
}

#[test]
fn dark_light_only() {
    check_with(
        "dark_light_only",
        ThemeInput {
            brand_colors: vec![hex("#0E6B5B")],
        },
        DarkPolicy::LightOnly,
    );
}

#[test]
fn dark_explicit() {
    check_with(
        "dark_explicit",
        ThemeInput {
            brand_colors: vec![hex("#0E6B5B")],
        },
        DarkPolicy::Explicit {
            accent: hex("#14b8a6"),
        },
    );
}

#[test]
fn empty_input_uses_default_brand() {
    check("empty", ThemeInput::empty());
}

#[test]
fn single_calm_brand() {
    check(
        "calm_single",
        ThemeInput {
            brand_colors: vec![hex("#0d9488")],
        },
    );
}

#[test]
fn single_neon_brand_is_split_and_tamed() {
    check(
        "neon_single",
        ThemeInput {
            brand_colors: vec![hex("#39ff14")],
        },
    );
}

#[test]
fn two_near_identical_darks() {
    check(
        "two_dark",
        ThemeInput {
            brand_colors: vec![hex("#0a1a2e"), hex("#101a2c")],
        },
    );
}

#[test]
fn five_color_palette() {
    check(
        "five_palette",
        ThemeInput {
            brand_colors: vec![
                hex("#3f6089"),
                hex("#c9572e"),
                hex("#2e7d5b"),
                hex("#8a4cb4"),
                hex("#d4a017"),
            ],
        },
    );
}

#[test]
fn pipeline_is_deterministic() {
    // Pure-function check: running the same input twice must produce
    // byte-identical CSS. If this ever fails, something inside the
    // pipeline picked up nondeterminism (HashMap iteration, time,
    // RNG) and the golden tests above would silently flap.
    let input = ThemeInput {
        brand_colors: vec![hex("#3f6089"), hex("#c9572e")],
    };
    let a = emit::emit(&resolve_theme(input.clone()));
    let b = emit::emit(&resolve_theme(input));
    assert_eq!(a, b);
}
