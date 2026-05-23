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
/// for each side indicating whether the engine had to adjust it.
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBrand {
    pub light: Color,
    pub dark: Color,
    pub light_adjusted: bool,
    pub dark_adjusted: bool,
}

/// Produce the light/dark pair for one brand color.
pub fn adaptive_brand(brand: &Color) -> AdaptiveBrand {
    let light_bg = Color::from_hex(LIGHT_BG).expect("constant");
    let dark_bg = Color::from_hex(DARK_BG).expect("constant");

    let (light, light_adjusted) = if contrast_ratio(&light_bg, brand) >= AA_NON_TEXT {
        (*brand, false)
    } else {
        (push_until_visible(brand, &light_bg, /*lighten=*/ false), true)
    };

    let (dark, dark_adjusted) = if contrast_ratio(&dark_bg, brand) >= AA_NON_TEXT {
        (*brand, false)
    } else {
        (push_until_visible(brand, &dark_bg, /*lighten=*/ true), true)
    };

    AdaptiveBrand {
        light,
        dark,
        light_adjusted,
        dark_adjusted,
    }
}

/// Step `color` toward more contrast against `bg` until it clears
/// AA_NON_TEXT (or runs out of headroom). `lighten` chooses the
/// direction — true for dark backgrounds, false for light.
fn push_until_visible(color: &Color, bg: &Color, lighten: bool) -> Color {
    let mut c = *color;
    for _ in 0..20 {
        if contrast_ratio(bg, &c) >= AA_NON_TEXT {
            return c;
        }
        let new_l = if lighten {
            (c.l + 0.05).min(1.0)
        } else {
            (c.l - 0.05).max(0.0)
        };
        if (new_l - c.l).abs() < 1e-9 {
            return c;
        }
        c = Color::from_oklch(new_l, c.c, c.h);
    }
    c
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
        assert!(a.dark.l > navy.l, "dark variant should be lighter");
    }

    #[test]
    fn mid_tone_passes_both_modes_unchanged() {
        let mid = c("#0d9488");
        let a = adaptive_brand(&mid);
        assert!(!a.light_adjusted);
        assert!(!a.dark_adjusted);
        assert_eq!(a.light.to_hex(), mid.to_hex());
        assert_eq!(a.dark.to_hex(), mid.to_hex());
    }
}
