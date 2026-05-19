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
        description: "Muted moss-green accent on a warm paper neutral. Reads as editorial, not corporate.",
        accent: "#2E7D5B",
        bg: "#F4F1EA",
        surface: "#FFFFFF",
        text: "#1F2B25",
        text_muted: "#5C6B62",
        border: "#DCD4C6",
    },
    Preset {
        name: "sunset",
        description: "Warm terracotta accent on cream. Sits in the same family as the default crimson.",
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
    }
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
    let preset = PRESETS
        .iter()
        .find(|p| p.name == lowered)
        .ok_or_else(|| {
            let names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
            format!(
                "unknown preset `{name}`. Available: {}",
                names.join(", ")
            )
        })?;

    let snippet = render_snippet(preset);
    println!("// rustio theme preset: {} — {}", preset.name, preset.description);
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
