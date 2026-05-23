//! Case 4 — brand-vs-state collision resolver.
//!
//! Success / warning / danger are universal conventions. They are
//! *not* derived from the brand. The only adjustment the engine makes
//! is to push a state color out of the brand's way when the hues
//! collide, so a red brand doesn't reduce the "delete" button to noise.

use crate::color::{hue_distance, Color};
use crate::contrast::{contrast_ratio, AA_NON_TEXT, LIGHT_BG};

/// Minimum OKLCH hue gap (degrees) a state color must keep from the
/// brand. 45° is the smallest distance at which a hue-shifted variant
/// still reads as "moved", not "kinda the same color".
pub const MIN_HUE_GAP: f64 = 45.0;

/// Fixed anchor hue for "success". Green is the universal convention.
pub const SUCCESS_ANCHOR: &str = "#16a34a";
/// Fixed anchor hue for "warning". Amber is the universal convention.
pub const WARNING_ANCHOR: &str = "#d97706";
/// Fixed anchor hue for "danger". Red is the universal convention.
pub const DANGER_ANCHOR: &str = "#dc2626";

/// Resolved state colors after collision avoidance.
#[derive(Debug, Clone, Copy)]
pub struct SemanticPalette {
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
}

/// Per-state preferred rotation direction. Picked so each state keeps
/// reading as itself after the shift: danger stays in the
/// crimson/orange band, success in the emerald band, warning in the
/// amber/orange band.
fn rotate_direction(anchor_hue: f64) -> f64 {
    // Hand-tuned per state. The engine never crosses into a
    // categorically wrong band (no green "danger", no red "success").
    match anchor_hue.round() as i32 {
        h if (130..=160).contains(&h) => 1.0, // success: emerald → mint
        h if (60..=90).contains(&h) => -1.0,  // warning: amber → orange
        _ => -1.0,                            // danger: red → crimson
    }
}

fn shift_away(anchor: Color, brand_hue: f64) -> Color {
    let gap = hue_distance(anchor.h, brand_hue);
    if gap >= MIN_HUE_GAP {
        return anchor;
    }
    let needed = MIN_HUE_GAP - gap;
    let dir = rotate_direction(anchor.h);
    let candidate_a = Color::from_oklch(anchor.l, anchor.c, anchor.h + dir * needed);
    // Verify the shift actually increased distance (not always
    // guaranteed when brand sits on the side we rotated toward).
    let candidate_b = Color::from_oklch(anchor.l, anchor.c, anchor.h - dir * needed);
    if hue_distance(candidate_a.h, brand_hue) >= hue_distance(candidate_b.h, brand_hue) {
        candidate_a
    } else {
        candidate_b
    }
}

fn ensure_visible_on_white(color: Color) -> Color {
    let bg = Color::from_hex(LIGHT_BG).expect("constant");
    if contrast_ratio(&bg, &color) >= AA_NON_TEXT {
        return color;
    }
    // Step the color darker in OKLCH lightness until it clears AA
    // against white. Five steps is plenty — a single 0.05 nudge
    // usually fixes mid-tone pastels, and the anchors start well
    // above the threshold to begin with.
    let mut c = color;
    for _ in 0..10 {
        c = Color::from_oklch((c.l - 0.05).max(0.0), c.c, c.h);
        if contrast_ratio(&bg, &c) >= AA_NON_TEXT {
            break;
        }
    }
    c
}

/// Pick state colors, pushing each one at least [`MIN_HUE_GAP`]
/// degrees away from the brand hue when needed.
pub fn resolve_semantics(brand: &Color) -> SemanticPalette {
    let success = Color::from_hex(SUCCESS_ANCHOR).expect("constant");
    let warning = Color::from_hex(WARNING_ANCHOR).expect("constant");
    let danger = Color::from_hex(DANGER_ANCHOR).expect("constant");

    SemanticPalette {
        success: ensure_visible_on_white(shift_away(success, brand.h)),
        warning: ensure_visible_on_white(shift_away(warning, brand.h)),
        danger: ensure_visible_on_white(shift_away(danger, brand.h)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> Color {
        Color::from_hex(hex).unwrap()
    }

    #[test]
    fn red_brand_shifts_danger_out_of_the_way() {
        let red_brand = c("#cc2222");
        let p = resolve_semantics(&red_brand);
        assert!(
            hue_distance(p.danger.h, red_brand.h) >= MIN_HUE_GAP - 1.0,
            "danger {} did not clear gap from brand {}",
            p.danger.h,
            red_brand.h
        );
    }

    #[test]
    fn purple_brand_leaves_all_three_unshifted() {
        let purple = c("#8a4cb4");
        let p = resolve_semantics(&purple);
        assert_eq!(p.success.to_hex(), c(SUCCESS_ANCHOR).to_hex());
        assert_eq!(p.warning.to_hex(), c(WARNING_ANCHOR).to_hex());
        assert_eq!(p.danger.to_hex(), c(DANGER_ANCHOR).to_hex());
    }
}
