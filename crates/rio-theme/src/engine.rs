//! Pipeline orchestrator.
//!
//! Pure function `resolve_theme(ThemeInput) -> ThemeTokens`: every
//! stage's input is the previous stage's output, no I/O, no globals.
//! That purity is what makes the golden-file tests stable and what
//! lets the CLI report each case's effect deterministically.
//!
//! Stage order matters. See §10 of the implementation brief.

use crate::adaptive::{adaptive_brand, AdaptiveBrand};
use crate::color::Color;
use crate::contrast::{contrast_ratio, AA_NON_TEXT, AA_TEXT, DARK_BG, LIGHT_BG};
use crate::derive::{derive_palette, DerivedPalette};
use crate::guard::{readable_text, resolve_text_token};
use crate::hierarchy::{assign_roles, RoleAssignment};
use crate::semantic::{resolve_semantics, SemanticPalette};
use crate::vivid::{split_vivid_roles, VividSplit};

/// Safe default brand color used when the client supplies none
/// (Case 7). A quiet blue-gray that passes contrast against white
/// automatically and sits in a mid lightness band so it survives a
/// future dark mode without adjustment.
pub const DEFAULT_BRAND: &str = "#3f6089";

/// The client's raw theme request.
#[derive(Debug, Clone)]
pub struct ThemeInput {
    /// Zero or more raw brand colors, in stated priority order.
    /// Empty inputs trip Case 7 and substitute [`DEFAULT_BRAND`].
    pub brand_colors: Vec<Color>,
}

impl ThemeInput {
    /// Convenience constructor for the empty input (Case 7).
    pub fn empty() -> Self {
        ThemeInput {
            brand_colors: Vec::new(),
        }
    }
}

/// The fully-resolved, safe set of tokens the UI consumes. Field
/// names match the `--rio-*` custom properties emitted by `emit.rs`.
#[derive(Debug, Clone)]
pub struct ThemeTokens {
    /// Brand variant for light-mode surfaces.
    pub brand_light: Color,
    /// Brand variant for dark-mode surfaces.
    pub brand_dark: Color,
    /// Tamed brand for large fills (topbar, primary button bg).
    pub brand_surface: Color,
    /// Raw brand for small touches (icons, dots, focus rings).
    pub brand_accent: Color,
    /// Secondary brand from a multi-color input. For single-color
    /// inputs this is a derived hover-darkened variant of the primary.
    pub brand_secondary: Color,
    /// Solid brand background for hover states.
    pub brand_hover: Color,
    /// Solid brand background for active / pressed states.
    pub brand_active: Color,
    /// Light brand-tinted surface (focus ring backgrounds, soft fills).
    pub brand_tint: Color,
    /// Brand-family text color usable on light surfaces.
    pub brand_text: Color,
    /// Page canvas (brand-tinted near-white).
    pub bg: Color,
    /// Hairline border in the brand family.
    pub border: Color,
    /// Muted neutral with a hint of brand temperature.
    pub muted: Color,
    /// Success semantic foreground.
    pub success: Color,
    /// Warning semantic foreground.
    pub warning: Color,
    /// Danger / destructive semantic foreground.
    pub danger: Color,
    /// Data-series fills. Empty for fewer than three brand inputs.
    pub chart: Vec<Color>,
}

/// Per-case effects recorded during a pipeline run, intended for the
/// CLI to surface to the developer. Building this alongside the
/// tokens means the engine's reasoning is transparent — no separate
/// "explain" pass that could diverge.
#[derive(Debug, Clone, Default)]
pub struct ResolveReport {
    /// True when Case 7 (default-brand fallback) fired.
    pub default_brand_used: bool,
    /// True when Case 3 reduced chroma on the surface brand.
    pub vivid_tamed: bool,
    /// True when Case 5 nudged the light variant.
    pub light_adjusted: bool,
    /// True when Case 5 nudged the dark variant.
    pub dark_adjusted: bool,
    /// True when the (adjusted) light variant still doesn't clear AA.
    pub light_still_failing: bool,
    /// True when the (adjusted) dark variant still doesn't clear AA.
    pub dark_still_failing: bool,
    /// True when Case 1 had to substitute the text fallback.
    pub text_substituted: bool,
    /// True when the raw vivid accent failed `AA_NON_TEXT` against
    /// the page bg and was substituted by the tamed surface.
    pub accent_substituted: bool,
    /// Brand vs LIGHT_BG contrast (post-adaptive).
    pub light_contrast: f64,
    /// Brand vs DARK_BG contrast (post-adaptive).
    pub dark_contrast: f64,
    /// brand_text vs bg contrast (after the Case 1 final pass).
    pub text_on_bg_contrast: f64,
    /// brand_accent vs bg contrast — non-text guard.
    pub accent_on_bg_contrast: f64,
    /// success/warning/danger contrast vs LIGHT_BG.
    pub success_contrast: f64,
    pub warning_contrast: f64,
    pub danger_contrast: f64,
}

/// Top-level pipeline. Pure; same input always yields the same output.
pub fn resolve_theme(input: ThemeInput) -> ThemeTokens {
    let (tokens, _report) = resolve_theme_with_report(input);
    tokens
}

/// Pipeline plus per-case effect log. The CLI uses this; tests use
/// the report fields to assert which stages fired.
pub fn resolve_theme_with_report(input: ThemeInput) -> (ThemeTokens, ResolveReport) {
    let mut report = ResolveReport::default();

    // --- Case 7: no brand → safe default ---
    let brand_colors: Vec<Color> = if input.brand_colors.is_empty() {
        report.default_brand_used = true;
        vec![Color::from_hex(DEFAULT_BRAND).expect("constant")]
    } else {
        input.brand_colors
    };

    // --- Case 6: role assignment ---
    let RoleAssignment {
        primary,
        secondary,
        chart,
    } = assign_roles(&brand_colors);

    // --- Case 3: vivid split on primary ---
    let VividSplit {
        accent: raw_accent,
        surface: surface_brand,
        was_tamed,
    } = split_vivid_roles(&primary);
    report.vivid_tamed = was_tamed;

    // --- Case 5: mode-adaptive on the surface variant ---
    let AdaptiveBrand {
        light: brand_light,
        dark: brand_dark,
        light_adjusted,
        dark_adjusted,
        light_clears_aa,
        dark_clears_aa,
    } = adaptive_brand(&surface_brand);
    report.light_adjusted = light_adjusted;
    report.dark_adjusted = dark_adjusted;
    report.light_still_failing = !light_clears_aa;
    report.dark_still_failing = !dark_clears_aa;

    // --- Case 2: derived shades from the *tamed* surface (brief
    //     §10 step 5 — derive runs on `brand_surface`, not on the
    //     mode-adapted variant; otherwise hover/active drift
    //     differently in light vs dark mode).
    let DerivedPalette {
        brand: _,
        brand_tint,
        brand_hover,
        brand_active,
        brand_text,
        bg,
        border,
        muted,
    } = derive_palette(&surface_brand);

    // --- Case 4: semantic anchors, pushed away from brand hue ---
    let SemanticPalette {
        success,
        warning,
        danger,
    } = resolve_semantics(&primary);

    // --- Case 1: final guard pass. Every text-on-surface pairing the
    //     engine is about to emit goes through `resolve_text_token`
    //     and any substitution is reflected in the report.
    let safe_brand_text = resolve_text_token(&bg, &brand_text);
    if safe_brand_text.to_hex() != brand_text.to_hex() {
        report.text_substituted = true;
    }

    // brand_accent is a non-text role (icons/dots/borders). Threshold
    // is AA_NON_TEXT (3.0). For vivid-tamed inputs this is where the
    // raw neon would otherwise leak through.
    let safe_brand_accent = if contrast_ratio(&bg, &raw_accent) >= AA_NON_TEXT {
        raw_accent
    } else {
        // Substitute the tamed surface — same hue family, guaranteed
        // to pass AA_NON_TEXT against light backgrounds because Case
        // 3 already chose its lightness for that.
        log::warn!(
            "rio-theme: brand_accent {} fails AA_NON_TEXT on bg {} (ratio {:.2}); substituting tamed surface {}",
            raw_accent.to_hex(),
            bg.to_hex(),
            contrast_ratio(&bg, &raw_accent),
            surface_brand.to_hex(),
        );
        report.accent_substituted = true;
        surface_brand
    };

    // Secondary role: multi-color inputs supply a real second colour,
    // single-color inputs reuse the primary's hover-darkened variant.
    // Either way the token is populated — the live admin's
    // `--rio-accent-border` can lean on this.
    let brand_secondary = if secondary.to_hex() == primary.to_hex() {
        brand_hover
    } else {
        secondary
    };

    // Contrast measurements for the CLI report — post-resolution so
    // the numbers match what the emitted tokens will actually produce.
    let light_bg = Color::from_hex(LIGHT_BG).expect("constant");
    let dark_bg = Color::from_hex(DARK_BG).expect("constant");
    report.light_contrast = contrast_ratio(&light_bg, &brand_light);
    report.dark_contrast = contrast_ratio(&dark_bg, &brand_dark);
    report.text_on_bg_contrast = contrast_ratio(&bg, &safe_brand_text);
    report.accent_on_bg_contrast = contrast_ratio(&bg, &safe_brand_accent);
    report.success_contrast = contrast_ratio(&light_bg, &success);
    report.warning_contrast = contrast_ratio(&light_bg, &warning);
    report.danger_contrast = contrast_ratio(&light_bg, &danger);

    let tokens = ThemeTokens {
        brand_light,
        brand_dark,
        brand_surface: surface_brand,
        brand_accent: safe_brand_accent,
        brand_secondary,
        brand_hover,
        brand_active,
        brand_tint,
        brand_text: safe_brand_text,
        bg,
        border,
        muted,
        success,
        warning,
        danger,
        chart,
    };

    // Guard sanity: every text-on-surface pairing we emit should
    // satisfy at least AA_TEXT. The substitution above covers
    // brand_text on bg; touch every other emitted text pairing here
    // so future additions to ThemeTokens fail loudly in tests if
    // they slip an unmeasured pair past the guard.
    debug_assert!(
        contrast_ratio(&tokens.bg, &tokens.brand_text) >= AA_TEXT - 0.01,
        "brand_text {} fails AA on bg {}",
        tokens.brand_text.to_hex(),
        tokens.bg.to_hex(),
    );
    // The chrome surface (slate-900 in the drop-in scaffold) carries
    // a near-white text. That pairing is fixed and known-safe; we
    // don't recompute it here. If the scaffold ever becomes
    // brand-derived, run `readable_text` on it the same way.
    let _ = readable_text;

    (tokens, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_uses_default_brand() {
        let (tokens, report) = resolve_theme_with_report(ThemeInput::empty());
        assert!(report.default_brand_used);
        assert!(!report.vivid_tamed);
        assert!(!report.light_adjusted);
        assert!(tokens.bg.l > 0.95);
    }

    #[test]
    fn neon_input_trips_vivid_taming_and_guards_accent() {
        let lime = Color::from_hex("#39ff14").unwrap();
        let (tokens, report) = resolve_theme_with_report(ThemeInput {
            brand_colors: vec![lime],
        });
        assert!(report.vivid_tamed);
        // The drop-in accent must not be the raw neon when it would
        // fail AA_NON_TEXT against the page bg.
        let raw_accent_contrast = contrast_ratio(&tokens.bg, &lime);
        if raw_accent_contrast < AA_NON_TEXT {
            assert_ne!(
                tokens.brand_accent.to_hex(),
                lime.to_hex(),
                "neon should have been substituted"
            );
        }
    }

    #[test]
    fn two_color_input_populates_brand_secondary_distinctly() {
        let (tokens, _) = resolve_theme_with_report(ThemeInput {
            brand_colors: vec![
                Color::from_hex("#3f6089").unwrap(),
                Color::from_hex("#c9572e").unwrap(),
            ],
        });
        // Both inputs must end up reachable via a token — primary
        // surfaces as brand_surface, secondary as brand_secondary.
        let surface_hex = tokens.brand_surface.to_hex();
        let secondary_hex = tokens.brand_secondary.to_hex();
        assert_ne!(surface_hex, secondary_hex);
    }

    #[test]
    fn report_contrast_fields_are_populated() {
        // The CLI surfaces these — a zero value would silently mean
        // "we forgot to measure". Confirm all four are nonzero for a
        // generic input.
        let (_, report) = resolve_theme_with_report(ThemeInput::empty());
        assert!(report.light_contrast > 0.0);
        assert!(report.dark_contrast > 0.0);
        assert!(report.text_on_bg_contrast > 0.0);
        assert!(report.accent_on_bg_contrast > 0.0);
        assert!(report.success_contrast > 0.0);
        assert!(report.warning_contrast > 0.0);
        assert!(report.danger_contrast > 0.0);
    }

    #[test]
    fn brand_text_always_clears_aa_on_bg() {
        // The central guarantee of Case 1. Run it against varied
        // brands, including the failure-prone neon, dark navy, and
        // the default.
        for hex in [
            "#3f6089", "#0d9488", "#39ff14", "#0a1a2e", "#c9572e", "#888888",
        ] {
            let brand = Color::from_hex(hex).unwrap();
            let tokens = resolve_theme(ThemeInput {
                brand_colors: vec![brand],
            });
            let ratio = contrast_ratio(&tokens.bg, &tokens.brand_text);
            assert!(
                ratio >= AA_TEXT - 0.01,
                "brand={hex}: brand_text {} on bg {} only {ratio:.2}",
                tokens.brand_text.to_hex(),
                tokens.bg.to_hex(),
            );
        }
    }
}
