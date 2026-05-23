//! Case 1 — contrast guard for text-on-surface pairings.
//!
//! The client's requested text color is treated as a *suggestion*.
//! When it fails AA against the surface, the engine substitutes a
//! guaranteed-readable text color rather than emit unreadable HTML.
//! The rejected color is never destroyed — callers may still place it
//! on a border or divider.

use crate::color::Color;
use crate::contrast::{contrast_ratio, AA_TEXT, TEXT_ON_DARK, TEXT_ON_LIGHT};

/// Pick whichever of the two default text colors reads best on
/// `surface`. Used both as the fallback inside [`resolve_text_token`]
/// and as a direct utility for "I just need text that works".
pub fn readable_text(surface: &Color) -> Color {
    let light = Color::from_hex(TEXT_ON_LIGHT).expect("constant");
    let dark = Color::from_hex(TEXT_ON_DARK).expect("constant");
    if contrast_ratio(surface, &light) >= contrast_ratio(surface, &dark) {
        light
    } else {
        dark
    }
}

/// If `requested` clears AA on `surface`, return it. Otherwise log a
/// warning and substitute [`readable_text`].
pub fn resolve_text_token(surface: &Color, requested: &Color) -> Color {
    let ratio = contrast_ratio(surface, requested);
    if ratio >= AA_TEXT {
        return *requested;
    }
    log::warn!(
        "rio-theme: text {} fails AA on surface {} (ratio {:.2} < {:.1}); substituting readable fallback",
        requested.to_hex(),
        surface.to_hex(),
        ratio,
        AA_TEXT,
    );
    readable_text(surface)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> Color {
        Color::from_hex(hex).unwrap()
    }

    #[test]
    fn near_black_surface_with_dark_gray_text_falls_back_to_light() {
        let surface = c("#0a0a0a");
        let bad_text = c("#222222");
        let resolved = resolve_text_token(&surface, &bad_text);
        // Fallback must NOT be the same dark gray.
        assert_ne!(resolved.to_hex(), bad_text.to_hex());
        // Must clear AA after substitution.
        assert!(contrast_ratio(&surface, &resolved) >= AA_TEXT);
    }

    #[test]
    fn white_surface_with_dark_text_passes_through() {
        let surface = c("#ffffff");
        let text = c("#1a1a1a");
        assert_eq!(resolve_text_token(&surface, &text).to_hex(), text.to_hex());
    }

    #[test]
    fn readable_text_picks_higher_ratio() {
        let surface = c("#ffffff");
        let t = readable_text(&surface);
        // On white, near-black wins.
        assert_eq!(t.to_hex(), "#1a1a1a");
    }
}
