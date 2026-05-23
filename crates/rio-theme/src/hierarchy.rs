//! Case 6 — multi-color role assignment.
//!
//! Given N colors, decide which carries the UI (primary), which
//! ornaments it (secondary), and which fill data series (chart).
//! No color is dropped — extras land in `chart` where multi-color
//! variety is wanted.

use crate::color::Color;
use crate::contrast::{contrast_ratio, LIGHT_BG};

/// Output of [`assign_roles`].
#[derive(Debug, Clone)]
pub struct RoleAssignment {
    /// Leads the UI: topbar emblem, primary buttons.
    pub primary: Color,
    /// Accent only — badges, dots, focus rings.
    pub secondary: Color,
    /// Reserved for data-series fills (charts). May be empty.
    pub chart: Vec<Color>,
}

/// Higher = better fit to carry large surfaces. Rewards readable
/// contrast against the page background, penalises both excessive
/// chroma (vivid colors are tiring at scale) and extreme lightness
/// (too-light surfaces can't carry text, too-dark surfaces dominate).
pub fn surface_fitness(color: &Color) -> f64 {
    let bg = Color::from_hex(LIGHT_BG).expect("constant");
    let cr = contrast_ratio(&bg, color);

    // Contrast: more is better, up to a point. Log-shaped so a 7.0
    // doesn't dwarf a 4.5.
    let contrast_score = cr.ln().max(0.0);

    // Lightness penalty: distance from 0.55 (a comfortable mid-tone
    // that carries either text color).
    let lightness_penalty = (color.l - 0.55).powi(2);

    // Chroma penalty: above ~0.13 the color starts demanding too much
    // visual attention for a large surface.
    let chroma_penalty = (color.c - 0.10).max(0.0).powi(2) * 4.0;

    contrast_score - lightness_penalty - chroma_penalty
}

/// Assign roles to the input list. Order on input is the client's
/// stated priority, but the actual `primary` is chosen by fitness —
/// the client's preferred ordering is a tiebreaker, not the sole
/// signal (a client's first color is sometimes too vivid for the
/// primary role).
pub fn assign_roles(brand_colors: &[Color]) -> RoleAssignment {
    // Edge: zero inputs. Caller (engine.rs) should have substituted
    // the Case 7 default before reaching here, but guard anyway.
    // Returns `secondary == primary` as a sentinel — the engine then
    // substitutes its own (post-tame) hover as the secondary token,
    // which is the right thing for "I have no real second colour".
    if brand_colors.is_empty() {
        let fallback = Color::from_hex("#3f6089").expect("constant");
        return RoleAssignment {
            primary: fallback,
            secondary: fallback,
            chart: Vec::new(),
        };
    }

    // Edge: single input. Same sentinel — engine substitutes hover.
    // This avoids deriving the secondary from the *raw* (possibly
    // vivid) input; the engine's own hover is derived from the
    // *tamed* surface and is the right value to surface.
    if brand_colors.len() == 1 {
        let only = brand_colors[0];
        return RoleAssignment {
            primary: only,
            secondary: only,
            chart: Vec::new(),
        };
    }

    // Stable-sort by fitness, descending. Sort key preserves the
    // client's input order on ties (Vec::sort_by is stable).
    let mut ranked: Vec<(usize, Color)> = brand_colors.iter().copied().enumerate().collect();
    ranked.sort_by(|a, b| {
        surface_fitness(&b.1)
            .partial_cmp(&surface_fitness(&a.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let primary = ranked[0].1;
    let secondary = ranked[1].1;
    let chart: Vec<Color> = ranked[2..].iter().map(|(_, c)| *c).collect();
    RoleAssignment {
        primary,
        secondary,
        chart,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> Color {
        Color::from_hex(hex).unwrap()
    }

    #[test]
    fn five_inputs_yield_one_primary_one_secondary_three_chart() {
        let inputs = vec![
            c("#3f6089"),
            c("#c9572e"),
            c("#2e7d5b"),
            c("#8a4cb4"),
            c("#d4a017"),
        ];
        let r = assign_roles(&inputs);
        assert_eq!(r.chart.len(), 3);
        // primary should outrank secondary by fitness.
        assert!(surface_fitness(&r.primary) >= surface_fitness(&r.secondary));
    }

    #[test]
    fn one_input_uses_sentinel_secondary_equal_to_primary() {
        // The engine reads `secondary == primary` as "no real second
        // colour" and substitutes its own post-tame hover. Keeping
        // the sentinel explicit avoids deriving from the raw input
        // (which for vivid inputs would be unusable).
        let r = assign_roles(&[c("#0d9488")]);
        assert!(r.chart.is_empty());
        assert_eq!(r.primary.to_hex(), r.secondary.to_hex());
    }

    #[test]
    fn two_inputs_yield_empty_chart() {
        let r = assign_roles(&[c("#3f6089"), c("#c9572e")]);
        assert!(r.chart.is_empty());
    }
}
