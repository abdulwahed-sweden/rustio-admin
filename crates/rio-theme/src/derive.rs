//! Case 2 — derive a full palette from one brand color.
//!
//! Neutrals (`muted`) carry a hint of the brand temperature so the
//! whole UI reads as one family. Semantic state colors (success /
//! warning / danger) are *not* derived here — they are universal
//! conventions handled by `semantic.rs`.

use crate::color::Color;

/// The full set of brand-derived functional shades.
#[derive(Debug, Clone, Copy)]
pub struct DerivedPalette {
    /// The brand itself, unchanged. Carried through so callers do not
    /// need to thread two values.
    pub brand: Color,
    /// 10% brand into white — light fills, hover backgrounds.
    pub brand_tint: Color,
    /// Slightly darker brand — hover state on solid brand buttons.
    pub brand_hover: Color,
    /// Darker still — active / pressed state.
    pub brand_active: Color,
    /// Dark brand variant suitable for text on light surfaces.
    pub brand_text: Color,
    /// Page background tinted with a hint of brand.
    pub bg: Color,
    /// Hairline border in the brand family.
    pub border: Color,
    /// Brand-temperatured neutral gray. Built by mixing 35% brand into
    /// `#6b7280` (Tailwind's gray-500) so muted text never looks
    /// "unrelated" to the brand.
    pub muted: Color,
}

/// Expand one brand color into the full derived palette (§5).
pub fn derive_palette(brand: &Color) -> DerivedPalette {
    let white = Color::from_hex("#ffffff").expect("constant");
    let near_black = Color::from_hex("#111111").expect("constant");
    let neutral_gray = Color::from_hex("#6b7280").expect("constant");

    DerivedPalette {
        brand: *brand,
        // Per §5 the wording is "brand mixed N% into white": N% of the
        // result is brand, the rest is white. With `a.mix(b, x)`
        // returning (1-x)·a + x·b, "10% brand into white" is
        // `brand.mix(white, 0.90)`.
        brand_tint: brand.mix(&white, 0.90),
        // "88% with black" → 88% brand, 12% black.
        brand_hover: brand.mix(&near_black, 0.12),
        // "75% with black" → 75% brand, 25% black.
        brand_active: brand.mix(&near_black, 0.25),
        // "72% with black" → 72% brand, 28% black. The final pass in
        // engine.rs runs this through Case 1 against the surface, so
        // if it still fails AA after darkening the fallback kicks in.
        brand_text: brand.mix(&near_black, 0.28),
        // "3% into white" → 97% white, 3% brand.
        bg: brand.mix(&white, 0.97),
        // "14% into white" → 86% white, 14% brand.
        border: brand.mix(&white, 0.86),
        // "35% brand into neutral gray" → 65% gray, 35% brand.
        muted: neutral_gray.mix(brand, 0.35),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> Color {
        Color::from_hex(hex).unwrap()
    }

    #[test]
    fn tint_is_lighter_than_brand() {
        let p = derive_palette(&c("#0d9488"));
        assert!(p.brand_tint.l > p.brand.l);
    }

    #[test]
    fn hover_is_darker_than_brand_and_active_darker_still() {
        let p = derive_palette(&c("#0d9488"));
        assert!(p.brand_hover.l < p.brand.l);
        assert!(p.brand_active.l < p.brand_hover.l);
    }

    #[test]
    fn bg_is_almost_white_and_border_is_lighter_than_brand() {
        let p = derive_palette(&c("#0d9488"));
        assert!(p.bg.l > 0.95);
        assert!(p.border.l > p.brand.l);
    }

    #[test]
    fn muted_has_brand_temperature_not_pure_gray() {
        let brand = c("#0d9488"); // teal hue ~190
        let p = derive_palette(&brand);
        // Muted should still have nonzero chroma — a pure gray would
        // give c ≈ 0 and lose the brand family.
        assert!(p.muted.c > 0.0);
    }
}
