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
use crate::derive::{derive_palette, DerivedPalette};
use crate::guard::resolve_text_token;
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
    pub brand_light: Color,
    pub brand_dark: Color,
    pub brand_surface: Color,
    pub brand_accent: Color,
    pub brand_hover: Color,
    pub brand_active: Color,
    pub brand_tint: Color,
    pub brand_text: Color,
    pub bg: Color,
    pub border: Color,
    pub muted: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
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
    /// True when Case 5 adjusted the light variant.
    pub light_adjusted: bool,
    /// True when Case 5 adjusted the dark variant.
    pub dark_adjusted: bool,
    /// True when Case 1 had to substitute a fallback text color.
    pub text_substituted: bool,
    /// Brand vs LIGHT_BG contrast (post-adaptive).
    pub light_contrast: f64,
    /// Brand vs DARK_BG contrast (post-adaptive).
    pub dark_contrast: f64,
}

/// Top-level pipeline. Pure; same input always yields the same output.
pub fn resolve_theme(input: ThemeInput) -> ThemeTokens {
    let (tokens, _report) = resolve_theme_with_report(input);
    tokens
}

/// Pipeline plus per-case effect log. The CLI uses this; tests use
/// the report fields to assert which stages fired.
pub fn resolve_theme_with_report(input: ThemeInput) -> (ThemeTokens, ResolveReport) {
    use crate::contrast::{contrast_ratio, DARK_BG, LIGHT_BG};

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
        secondary: _secondary,
        chart,
    } = assign_roles(&brand_colors);

    // --- Case 3: vivid split on primary ---
    let VividSplit {
        accent,
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
    } = adaptive_brand(&surface_brand);
    report.light_adjusted = light_adjusted;
    report.dark_adjusted = dark_adjusted;

    // --- Case 2: derived shades from the (post-adaptive light) surface ---
    let DerivedPalette {
        brand: _,
        brand_tint,
        brand_hover,
        brand_active,
        brand_text,
        bg,
        border,
        muted,
    } = derive_palette(&brand_light);

    // --- Case 4: semantic anchors, pushed away from brand hue ---
    let SemanticPalette {
        success,
        warning,
        danger,
    } = resolve_semantics(&primary);

    // --- Case 1: final pass — every text-on-surface pairing the
    //     engine emits goes through the contrast guard. The only
    //     emitted text pairing today is `brand_text` on `bg`, but the
    //     guard runs centrally so any future text/surface pairing
    //     added to ThemeTokens inherits the same safety net.
    let safe_brand_text = resolve_text_token(&bg, &brand_text);
    if safe_brand_text.to_hex() != brand_text.to_hex() {
        report.text_substituted = true;
    }

    // Contrast measurements for the CLI report — post-adaptive so the
    // numbers match what the emitted tokens will actually produce.
    report.light_contrast = contrast_ratio(
        &Color::from_hex(LIGHT_BG).expect("constant"),
        &brand_light,
    );
    report.dark_contrast = contrast_ratio(
        &Color::from_hex(DARK_BG).expect("constant"),
        &brand_dark,
    );

    let tokens = ThemeTokens {
        brand_light,
        brand_dark,
        brand_surface: surface_brand,
        brand_accent: accent,
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

    (tokens, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_uses_default_brand() {
        let (tokens, report) = resolve_theme_with_report(ThemeInput::empty());
        assert!(report.default_brand_used);
        // Default brand is calm — no taming.
        assert!(!report.vivid_tamed);
        // Default sits comfortably against white — no light-mode shift.
        assert!(!report.light_adjusted);
        // It still gets a brand-tinted page bg derived through Case 2.
        assert!(tokens.bg.l > 0.95);
    }

    #[test]
    fn neon_input_trips_vivid_taming() {
        let lime = Color::from_hex("#39ff14").unwrap();
        let (_, report) = resolve_theme_with_report(ThemeInput {
            brand_colors: vec![lime],
        });
        assert!(report.vivid_tamed);
    }
}
