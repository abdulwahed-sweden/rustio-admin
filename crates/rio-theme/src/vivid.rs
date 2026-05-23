//! Case 3 — vivid color taming and role split.
//!
//! A neon brand is fine as a small accent but painful as a large
//! surface and hostile to text contrast. Above a chroma threshold the
//! engine produces two siblings: the original (for small touches) and
//! a tamed surface (for large fills). Hue never moves — only chroma
//! and lightness adjust.

use crate::color::Color;

/// Chroma above this (in OKLCH) is "too vivid for large surfaces".
/// 0.16 sits comfortably below electric primaries (red ≈ 0.25, lime
/// ≈ 0.24, cyan ≈ 0.18) while leaving room for healthy brand teals
/// (~0.13–0.14) to pass through untamed.
pub const VIVID_CHROMA_THRESHOLD: f64 = 0.16;

/// Output of [`split_vivid_roles`].
#[derive(Debug, Clone, Copy)]
pub struct VividSplit {
    /// Original brand, intended for small elements: dots, icons,
    /// borders, links.
    pub accent: Color,
    /// Tamed sibling, intended for large fills: topbar background,
    /// primary-button background.
    pub surface: Color,
    /// True when `surface` was modified from `accent`.
    pub was_tamed: bool,
}

/// Split a brand into accent (small) and surface (large) roles.
pub fn split_vivid_roles(brand: &Color) -> VividSplit {
    if brand.c <= VIVID_CHROMA_THRESHOLD {
        return VividSplit {
            accent: *brand,
            surface: *brand,
            was_tamed: false,
        };
    }

    // Target a calm chroma well under the threshold so the surface
    // truly reads as "dialed back", not "barely tamed".
    let target_c = (VIVID_CHROMA_THRESHOLD * 0.75).min(brand.c);

    // Clamp lightness into a mid range so the surface can carry both
    // light and dark text. Outside [0.35, 0.65] the surface starts to
    // bias contrast strongly toward one text color or the other.
    let target_l = brand.l.clamp(0.35, 0.65);

    // Hue is held fixed — a tamed lime must still read as lime.
    let surface = Color::from_oklch(target_l, target_c, brand.h);

    VividSplit {
        accent: *brand,
        surface,
        was_tamed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::hue_distance;

    fn c(hex: &str) -> Color {
        Color::from_hex(hex).unwrap()
    }

    #[test]
    fn neon_lime_is_tamed_and_hue_is_preserved() {
        let lime = c("#39ff14");
        let split = split_vivid_roles(&lime);
        assert!(split.was_tamed);
        assert!(split.surface.c < split.accent.c);
        // Hue must drift less than ~2 degrees.
        assert!(
            hue_distance(split.surface.h, split.accent.h) < 2.0,
            "hue drifted {} -> {}",
            split.accent.h,
            split.surface.h
        );
    }

    #[test]
    fn calm_blue_passes_through_untamed() {
        let blue = c("#3f6089");
        let split = split_vivid_roles(&blue);
        assert!(!split.was_tamed);
        assert_eq!(split.accent.to_hex(), split.surface.to_hex());
    }
}
