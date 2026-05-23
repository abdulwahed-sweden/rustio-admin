//! Case 5 — mode-adaptive brand.
//!
//! A single brand color faces two backgrounds (light and dark) and
//! the two checks pull in opposite directions. The engine measures
//! each independently and only edits the failing mode.
//!
//! `rustio-admin` is currently light-only, so today both outputs
//! often equal the input. The token structure still ships both so a
//! future dark-mode return is a configuration flip, not a refactor.

use crate::color::Color;
use crate::contrast::{contrast_ratio, AA_NON_TEXT, DARK_BG, LIGHT_BG};

/// The light- and dark-mode variants of one brand color, plus a flag
/// for each side indicating whether the engine had to adjust it and
/// whether that adjustment actually cleared AA. `*_adjusted` says
/// "we tried"; `*_clears_aa` says "we succeeded".
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBrand {
    /// Variant to use on light backgrounds.
    pub light: Color,
    /// Variant to use on dark backgrounds.
    pub dark: Color,
    /// True when the light variant was nudged from the input.
    pub light_adjusted: bool,
    /// True when the dark variant was nudged from the input.
    pub dark_adjusted: bool,
    /// True when the (possibly adjusted) light variant clears
    /// `AA_NON_TEXT` against `LIGHT_BG`. False means the loop ran
    /// out of headroom — the caller should know the token is still
    /// failing rather than silently trust the report.
    pub light_clears_aa: bool,
    /// Same as [`Self::light_clears_aa`] for the dark background.
    pub dark_clears_aa: bool,
}

/// Produce the light/dark pair for one brand color.
pub fn adaptive_brand(brand: &Color) -> AdaptiveBrand {
    let light_bg = Color::from_hex(LIGHT_BG).expect("constant");
    let dark_bg = Color::from_hex(DARK_BG).expect("constant");

    let (light, light_adjusted, light_clears_aa) =
        adjust_for_bg(brand, &light_bg, /*lighten=*/ false);
    let (dark, dark_adjusted, dark_clears_aa) =
        adjust_for_bg(brand, &dark_bg, /*lighten=*/ true);

    AdaptiveBrand {
        light,
        dark,
        light_adjusted,
        dark_adjusted,
        light_clears_aa,
        dark_clears_aa,
    }
}

/// Run the (color, bg) check; if it passes, pass the color through.
/// If it fails, step toward more contrast until it passes or the loop
/// runs out of headroom. Returns `(color, adjusted, cleared_aa)`.
fn adjust_for_bg(color: &Color, bg: &Color, lighten: bool) -> (Color, bool, bool) {
    if contrast_ratio(bg, color) >= AA_NON_TEXT {
        return (*color, false, true);
    }
    let mut c = *color;
    let mut cleared = false;
    for _ in 0..20 {
        let new_l = if lighten {
            (c.l + 0.05).min(1.0)
        } else {
            (c.l - 0.05).max(0.0)
        };
        if (new_l - c.l).abs() < f64::EPSILON {
            break;
        }
        c = Color::from_oklch(new_l, c.c, c.h);
        if contrast_ratio(bg, &c) >= AA_NON_TEXT {
            cleared = true;
            break;
        }
    }
    (c, true, cleared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> Color {
        Color::from_hex(hex).unwrap()
    }

    #[test]
    fn very_dark_navy_lightens_for_dark_mode_only() {
        // #0a1a2e is near-invisible on a near-black dark background.
        let navy = c("#0a1a2e");
        let a = adaptive_brand(&navy);
        assert!(!a.light_adjusted, "should pass against white");
        assert!(a.dark_adjusted, "should fail against dark bg");
        assert!(a.dark_clears_aa, "should clear AA after lightening");
        assert!(a.dark.l > navy.l, "dark variant should be lighter");
    }

    #[test]
    fn mid_tone_passes_both_modes_unchanged() {
        let mid = c("#0d9488");
        let a = adaptive_brand(&mid);
        assert!(!a.light_adjusted);
        assert!(!a.dark_adjusted);
        assert!(a.light_clears_aa);
        assert!(a.dark_clears_aa);
        assert_eq!(a.light.to_hex(), mid.to_hex());
        assert_eq!(a.dark.to_hex(), mid.to_hex());
    }

    #[test]
    fn adjustment_flag_separates_attempted_from_succeeded() {
        // For #0a1a2e against the dark bg, the loop attempts and
        // succeeds — the two flags are both true and consistent.
        let navy = c("#0a1a2e");
        let a = adaptive_brand(&navy);
        assert!(a.dark_adjusted && a.dark_clears_aa);
    }
}
