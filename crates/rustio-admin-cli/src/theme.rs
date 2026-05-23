//! `rustio theme` — print curated `AdminTheme` snippets for
//! copy-paste into a project's `Admin::new()` builder chain.
//!
//! No source mutation. The verb is a transparent printer — it lists
//! the available presets, then on demand emits a complete, ready-to-
//! paste `.theme(...)` clause. Operators do the one-line edit
//! themselves; the framework never reaches into `main.rs` and risks
//! corrupting in-progress work. Same posture as `rustio override`,
//! which copies a template to disk but never edits the project's
//! existing files.
//!
//! Palette values respect `docs/design/DESIGN_DOCTRINE.md`:
//! comfortable saturation for ten-hour sessions, no neon, no
//! oled-black. Each preset is a six-field hex tuple; project authors
//! can tweak any single field after pasting.

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Action {
    /// List every available preset name + one-line description.
    List,
    /// Print the Rust `.theme(...)` clause for a single preset.
    Show {
        /// Preset identifier (e.g. `ocean`, `forest`, `sunset`,
        /// `monochrome`). Case-insensitive.
        name: String,
    },
    /// Generate a `tokens.css` from raw brand colors. The engine
    /// measures contrast, tames vivid colors, derives shades, and
    /// pushes state colors out of the brand's way before writing.
    /// Zero `--brand` flags emits the safe default (Case 7).
    Generate {
        /// Brand color in `#rrggbb` form. Repeatable; order is the
        /// client's priority order. Engine ranks by fitness rather
        /// than trusting the order blindly — see DESIGN_THEME §9.
        #[arg(long = "brand")]
        brand: Vec<String>,
        /// Destination path for the emitted CSS. Existing files are
        /// overwritten without prompting — the engine is deterministic
        /// and regenerable.
        #[arg(long = "out", default_value = "tokens.css")]
        out: PathBuf,
    },
}

/// One curated preset. Field comments mirror `AdminTheme`'s field
/// purposes so the printed code is self-documenting.
struct Preset {
    name: &'static str,
    description: &'static str,
    accent: &'static str,
    bg: &'static str,
    surface: &'static str,
    text: &'static str,
    text_muted: &'static str,
    border: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "ocean",
        description: "Cool teal-blue accent on calm slate-tinted neutrals. Calm for long sessions.",
        accent: "#0F6890",
        bg: "#EEF2F5",
        surface: "#FFFFFF",
        text: "#0F1B25",
        text_muted: "#536976",
        border: "#D8E1E8",
    },
    Preset {
        name: "forest",
        description:
            "Muted moss-green accent on a warm paper neutral. Reads as editorial, not corporate.",
        accent: "#2E7D5B",
        bg: "#F4F1EA",
        surface: "#FFFFFF",
        text: "#1F2B25",
        text_muted: "#5C6B62",
        border: "#DCD4C6",
    },
    Preset {
        name: "sunset",
        description:
            "Warm terracotta accent on cream. Sits in the same family as the default crimson.",
        accent: "#C9572E",
        bg: "#FAF5EE",
        surface: "#FFFFFF",
        text: "#231B12",
        text_muted: "#6B5A4A",
        border: "#E7DCC8",
    },
    Preset {
        name: "monochrome",
        description: "Zero saturation — paper-white, ink-black. Print-shop minimalism.",
        accent: "#1A1A1A",
        bg: "#F5F5F5",
        surface: "#FFFFFF",
        text: "#0A0A0A",
        text_muted: "#666666",
        border: "#D9D9D9",
    },
];

pub(crate) fn run(action: Action) -> Result<(), String> {
    match action {
        Action::List => {
            print_list();
            Ok(())
        }
        Action::Show { name } => print_show(&name),
        Action::Generate { brand, out } => run_generate(&brand, &out),
    }
}

fn run_generate(brand: &[String], out: &std::path::Path) -> Result<(), String> {
    let mut colors = Vec::with_capacity(brand.len());
    for raw in brand {
        let parsed =
            rio_theme::Color::from_hex(raw).map_err(|e| format!("invalid --brand {raw:?}: {e}"))?;
        colors.push(parsed);
    }

    let input = rio_theme::ThemeInput {
        brand_colors: colors.clone(),
    };
    let (tokens, report) = rio_theme::resolve_theme_with_report(input);
    let css = rio_theme::emit::emit(&tokens);

    std::fs::write(out, &css).map_err(|e| format!("could not write {}: {e}", out.display()))?;

    print_generate_report(&colors, &tokens, &report, out);
    Ok(())
}

fn print_generate_report(
    inputs: &[rio_theme::Color],
    tokens: &rio_theme::ThemeTokens,
    report: &rio_theme::ResolveReport,
    out: &std::path::Path,
) {
    println!("rio-theme: wrote {}", out.display());
    println!();
    println!("inputs:");
    if inputs.is_empty() {
        println!("  (none — Case 7 default brand used)");
    } else {
        for (i, c) in inputs.iter().enumerate() {
            println!("  [{}] {}", i + 1, c.to_hex());
        }
    }

    println!();
    println!("cases fired:");
    if report.default_brand_used {
        println!("  Case 7  default brand inserted (no input)");
    }
    if report.vivid_tamed {
        println!(
            "  Case 3  vivid taming: surface {} (raw input above chroma threshold)",
            tokens.brand_surface.to_hex(),
        );
    }
    if report.accent_substituted {
        println!(
            "  Case 1  brand_accent substituted to {} (raw input failed AA_NON_TEXT on bg)",
            tokens.brand_accent.to_hex(),
        );
    }
    if report.light_adjusted {
        println!(
            "  Case 5  light variant adjusted to {} for contrast on white",
            tokens.brand_light.to_hex()
        );
    }
    if report.dark_adjusted {
        println!(
            "  Case 5  dark variant adjusted to {} for contrast on dark bg",
            tokens.brand_dark.to_hex()
        );
    }
    if report.light_still_failing {
        println!("  WARNING light variant still fails AA after adjustment");
    }
    if report.dark_still_failing {
        println!("  WARNING dark variant still fails AA after adjustment");
    }
    if report.text_substituted {
        println!(
            "  Case 1  brand_text substituted to {} to clear AA on page bg",
            tokens.brand_text.to_hex()
        );
    }
    if !report.default_brand_used
        && !report.vivid_tamed
        && !report.light_adjusted
        && !report.dark_adjusted
        && !report.text_substituted
        && !report.accent_substituted
        && !report.light_still_failing
        && !report.dark_still_failing
    {
        println!("  (none — inputs needed no repair)");
    }

    println!();
    println!("contrast (post-resolution):");
    println!("  brand_light   vs #ffffff : {:.2}", report.light_contrast);
    println!("  brand_dark    vs #15161a : {:.2}", report.dark_contrast);
    println!(
        "  brand_text    vs bg      : {:.2}",
        report.text_on_bg_contrast
    );
    println!(
        "  brand_accent  vs bg      : {:.2}",
        report.accent_on_bg_contrast
    );
    println!(
        "  success       vs #ffffff : {:.2}",
        report.success_contrast
    );
    println!(
        "  warning       vs #ffffff : {:.2}",
        report.warning_contrast
    );
    println!("  danger        vs #ffffff : {:.2}", report.danger_contrast);
}

fn print_list() {
    println!("{} curated presets:", PRESETS.len());
    println!();
    // Width column lets long names align without a runtime padder.
    let max_name = PRESETS.iter().map(|p| p.name.len()).max().unwrap_or(0);
    for p in PRESETS {
        println!("  {:<width$}  {}", p.name, p.description, width = max_name);
    }
    println!();
    println!("Show a preset's full code with `rustio theme show <name>`.");
    println!("Paste the printed `.theme(...)` clause into your `Admin::new()` chain.");
}

fn print_show(name: &str) -> Result<(), String> {
    let lowered = name.to_ascii_lowercase();
    let preset = PRESETS.iter().find(|p| p.name == lowered).ok_or_else(|| {
        let names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
        format!("unknown preset `{name}`. Available: {}", names.join(", "))
    })?;

    let snippet = render_snippet(preset);
    println!(
        "// rustio theme preset: {} — {}",
        preset.name, preset.description
    );
    println!("//");
    println!("// Paste this clause into your `Admin::new()` builder chain.");
    println!("// Trailing fluent calls (e.g. `.app_name(...)`) can come after.");
    println!();
    println!("{snippet}");
    Ok(())
}

fn render_snippet(p: &Preset) -> String {
    format!(
        ".theme(\n    \
            rustio_admin::admin::AdminTheme::new()\n        \
                .accent(\"{}\")\n        \
                .bg(\"{}\")\n        \
                .surface(\"{}\")\n        \
                .text(\"{}\")\n        \
                .text_muted(\"{}\")\n        \
                .border(\"{}\"),\n\
        )",
        p.accent, p.bg, p.surface, p.text, p.text_muted, p.border,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_named_presets_ship() {
        let names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["ocean", "forest", "sunset", "monochrome"]);
    }

    #[test]
    fn every_preset_has_six_normalised_hex_fields() {
        for p in PRESETS {
            for (label, value) in [
                ("accent", p.accent),
                ("bg", p.bg),
                ("surface", p.surface),
                ("text", p.text),
                ("text_muted", p.text_muted),
                ("border", p.border),
            ] {
                assert!(
                    value.starts_with('#'),
                    "{}.{label} = {value:?} missing leading #",
                    p.name
                );
                assert_eq!(
                    value.len(),
                    7,
                    "{}.{label} = {value:?} not in #rrggbb form",
                    p.name
                );
                assert!(
                    value[1..].chars().all(|c| c.is_ascii_hexdigit()),
                    "{}.{label} = {value:?} contains non-hex chars",
                    p.name
                );
            }
        }
    }

    #[test]
    fn snippet_compiles_against_admintheme_builder_surface() {
        // The verb is only useful if the printed snippet
        // round-trips into something the library accepts. Read the
        // ocean preset, parse the chain method names back out,
        // and assert they match the AdminTheme fluent builders.
        let p = PRESETS.iter().find(|p| p.name == "ocean").unwrap();
        let s = render_snippet(p);
        for method in [
            ".accent(",
            ".bg(",
            ".surface(",
            ".text(",
            ".text_muted(",
            ".border(",
        ] {
            assert!(
                s.contains(method),
                "snippet must call {method} on AdminTheme: \n{s}",
            );
        }
        assert!(s.contains("rustio_admin::admin::AdminTheme::new()"));
        // Trailing comma + closing paren so the clause is a
        // complete expression ready for `.chain(...)` follow-ups.
        assert!(s.ends_with(",\n)"));
    }

    #[test]
    fn show_unknown_preset_returns_helpful_error() {
        let err = print_show("nonexistent").unwrap_err();
        assert!(err.contains("unknown preset"));
        // The error must list the available names so the operator
        // can correct without re-running `list`.
        assert!(err.contains("ocean"));
        assert!(err.contains("monochrome"));
    }

    #[test]
    fn show_is_case_insensitive() {
        // Operators capitalise preset names by accident; don't
        // punish them for it.
        assert!(print_show("Ocean").is_ok());
        assert!(print_show("MONOCHROME").is_ok());
    }
}
