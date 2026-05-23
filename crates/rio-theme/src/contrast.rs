//! WCAG 2.1 contrast primitives. Cases 1, 4, and 5 all measure
//! against these — implement once, correctly.
//!
//! `relative_luminance` works on linearized sRGB; the conversion
//! lives in `color::linear_srgb_of` so this file does not duplicate
//! the gamma curve.

use crate::color::{linear_srgb_of, Color};

/// WCAG 2.1 relative luminance of an sRGB color, range `0.0..=1.0`.
pub fn relative_luminance(color: &Color) -> f64 {
    let [r, g, b] = linear_srgb_of(color);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG 2.1 contrast ratio. Range `1.0..=21.0` — identical colors
/// give 1.0, black-on-white gives 21.0.
pub fn contrast_ratio(a: &Color, b: &Color) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (light, dark) = if la >= lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

/// AA threshold for normal-sized text.
pub const AA_TEXT: f64 = 4.5;
/// AA threshold for large text (>= 18pt regular / 14pt bold).
pub const AA_LARGE_TEXT: f64 = 3.0;
/// AA threshold for UI components, icons, borders.
pub const AA_NON_TEXT: f64 = 3.0;

/// Page background the engine measures the light-mode palette against.
pub const LIGHT_BG: &str = "#ffffff";
/// Page background for the future dark mode (§8 — emitted today even
/// though the framework is light-only).
pub const DARK_BG: &str = "#15161a";
/// Default near-black text used on light surfaces.
pub const TEXT_ON_LIGHT: &str = "#1a1a1a";
/// Default near-white text used on dark surfaces.
pub const TEXT_ON_DARK: &str = "#f5f5f5";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    fn c(hex: &str) -> Color {
        Color::from_hex(hex).unwrap()
    }

    #[test]
    fn black_on_white_is_twenty_one() {
        let r = contrast_ratio(&c("#000000"), &c("#ffffff"));
        assert!((r - 21.0).abs() < 0.01, "got {r}");
    }

    #[test]
    fn identical_colors_yield_one() {
        let r = contrast_ratio(&c("#3f6089"), &c("#3f6089"));
        assert!((r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ratio_is_symmetric() {
        let a = c("#0d9488");
        let b = c("#ffffff");
        assert!((contrast_ratio(&a, &b) - contrast_ratio(&b, &a)).abs() < 1e-9);
    }
}
