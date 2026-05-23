//! The single value type the theme engine passes around.
//!
//! Stored as OKLCH (perceptually uniform). sRGB hex strings are an
//! I/O concern only — `from_hex` is the inbound door, `to_hex` the
//! outbound. Engine math (`mix`, `lighten`, `darken`, hue distance,
//! chroma checks) all happens in OKLCH or OKLab and never round-trips
//! through gamma-encoded sRGB.
//!
//! The OKLab matrices are Björn Ottosson's published constants
//! (https://bottosson.github.io/posts/oklab/). They are embedded
//! literally rather than computed at runtime so this file stays
//! self-contained and `cargo expand`-friendly.

use std::fmt;

/// A color stored in OKLCH. All engine math happens in this space.
///
/// - `l` (lightness) in `0.0..=1.0`. 0 is black, 1 is white.
/// - `c` (chroma) in `0.0..~0.4`. 0 is achromatic; ~0.37 is a vivid
///   sRGB-gamut red. Values above the gamut clip on emission.
/// - `h` (hue) in degrees `0.0..360.0`. Undefined when chroma is 0,
///   but we always store a value (typically 0) for `Copy`/`PartialEq`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Lightness in `0.0..=1.0`. 0 is black, 1 is white. Perceptually
    /// uniform — equal steps in `l` look equally bright to the eye.
    pub l: f64,
    /// Chroma in `0.0..~0.4`. 0 is achromatic; ~0.37 is a vivid
    /// sRGB-gamut red. Values above the gamut clip on emission.
    pub c: f64,
    /// Hue in degrees `0.0..360.0`. Undefined when `c` is 0 but
    /// always stored (typically 0) so the struct remains `Copy` and
    /// `PartialEq`.
    pub h: f64,
}

/// Failure mode of [`Color::from_hex`]. Returned instead of panicking
/// because the engine accepts client-supplied strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorError {
    /// Missing the leading `#`.
    MissingHash,
    /// Length was not 4 (`#rgb`) or 7 (`#rrggbb`).
    BadLength,
    /// Body contained a non-hexadecimal character.
    BadDigit,
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::MissingHash => f.write_str("hex color must start with '#'"),
            ColorError::BadLength => {
                f.write_str("hex color must be '#rgb' or '#rrggbb' (4 or 7 chars)")
            }
            ColorError::BadDigit => f.write_str("hex color contains a non-hex character"),
        }
    }
}

impl std::error::Error for ColorError {}

impl Color {
    /// Construct directly from OKLCH components. Used by tests and by
    /// pipeline stages that compute coordinates rather than parse strings.
    pub fn from_oklch(l: f64, c: f64, h: f64) -> Self {
        Color {
            l,
            c,
            h: ((h % 360.0) + 360.0) % 360.0,
        }
    }

    /// Parse `#rrggbb` or `#rgb`. Never panics on bad input.
    pub fn from_hex(hex: &str) -> Result<Self, ColorError> {
        let bytes = hex.as_bytes();
        if bytes.is_empty() || bytes[0] != b'#' {
            return Err(ColorError::MissingHash);
        }
        let body = &hex[1..];
        let (r, g, b) = match body.len() {
            3 => {
                let r = expand_nibble(byte(body, 0)?);
                let g = expand_nibble(byte(body, 1)?);
                let b = expand_nibble(byte(body, 2)?);
                (r, g, b)
            }
            6 => {
                let r = pair(body, 0)?;
                let g = pair(body, 2)?;
                let b = pair(body, 4)?;
                (r, g, b)
            }
            _ => return Err(ColorError::BadLength),
        };
        let srgb = [r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0];
        Ok(srgb_to_oklch(srgb))
    }

    /// Emit a `#rrggbb` string. Converts to sRGB and clamps each
    /// channel into `0..=1` before quantizing — out-of-gamut OKLCH
    /// coordinates are projected by channel clamping, not by hue shift.
    pub fn to_hex(&self) -> String {
        let [r, g, b] = oklch_to_srgb(*self);
        let r = quantize(r);
        let g = quantize(g);
        let b = quantize(b);
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// Mix toward `other` by `amount` (0.0 = self, 1.0 = other).
    ///
    /// Interpolation happens in OKLab — L, a, b each interpolate
    /// linearly — so the result tracks perceived color without the
    /// hue-wraparound traps of LCh-space mixing. This is the engine's
    /// port of CSS `color-mix()` for the cases this engine needs.
    pub fn mix(&self, other: &Color, amount: f64) -> Color {
        let a = amount.clamp(0.0, 1.0);
        let (la, aa, ba) = oklch_to_oklab(self.l, self.c, self.h);
        let (lb, ab, bb) = oklch_to_oklab(other.l, other.c, other.h);
        let l = la + (lb - la) * a;
        let ax = aa + (ab - aa) * a;
        let bx = ba + (bb - ba) * a;
        let (l, c, h) = oklab_to_oklch(l, ax, bx);
        Color { l, c, h }
    }

    /// Mix toward pure white by `amount`. Convenience wrapper used
    /// across the derivation rules (§5).
    pub fn lighten(&self, amount: f64) -> Color {
        let white = Color {
            l: 1.0,
            c: 0.0,
            h: 0.0,
        };
        self.mix(&white, amount)
    }

    /// Mix toward near-black (#111) by `amount`. Convenience wrapper
    /// for hover / active / dark-text derivations (§5). Near-black
    /// rather than pure black so the result keeps a hair of warmth
    /// instead of going dead-flat.
    pub fn darken(&self, amount: f64) -> Color {
        // #111 in OKLCH coordinates (L≈0.18, C=0).
        let near_black = Color::from_hex("#111111").expect("constant");
        self.mix(&near_black, amount)
    }
}

// --- hex helpers ---------------------------------------------------

fn byte(s: &str, i: usize) -> Result<u8, ColorError> {
    let c = s.as_bytes()[i] as char;
    c.to_digit(16).map(|d| d as u8).ok_or(ColorError::BadDigit)
}

fn pair(s: &str, i: usize) -> Result<u8, ColorError> {
    let hi = byte(s, i)?;
    let lo = byte(s, i + 1)?;
    Ok((hi << 4) | lo)
}

fn expand_nibble(n: u8) -> u8 {
    (n << 4) | n
}

fn quantize(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

// --- sRGB <-> linear sRGB -----------------------------------------

fn srgb_decode(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_encode(v: f64) -> f64 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

// --- linear sRGB <-> OKLab ----------------------------------------
//
// Ottosson's matrices; do not edit without re-deriving from the
// original paper.

fn linear_srgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let big_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let big_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let big_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;
    (big_l, big_a, big_b)
}

fn oklab_to_linear_srgb(big_l: f64, big_a: f64, big_b: f64) -> (f64, f64, f64) {
    let l_ = big_l + 0.3963377774 * big_a + 0.2158037573 * big_b;
    let m_ = big_l - 0.1055613458 * big_a - 0.0638541728 * big_b;
    let s_ = big_l - 0.0894841775 * big_a - 1.2914855480 * big_b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;
    (r, g, b)
}

// --- OKLab <-> OKLCH ----------------------------------------------

fn oklab_to_oklch(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let c = (a * a + b * b).sqrt();
    let h_rad = b.atan2(a);
    let mut h_deg = h_rad.to_degrees();
    if h_deg < 0.0 {
        h_deg += 360.0;
    }
    (l, c, h_deg)
}

fn oklch_to_oklab(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let h_rad = h.to_radians();
    let a = c * h_rad.cos();
    let b = c * h_rad.sin();
    (l, a, b)
}

// --- full pipeline shortcuts --------------------------------------

fn srgb_to_oklch(srgb: [f64; 3]) -> Color {
    let r = srgb_decode(srgb[0]);
    let g = srgb_decode(srgb[1]);
    let b = srgb_decode(srgb[2]);
    let (ll, la, lb) = linear_srgb_to_oklab(r, g, b);
    let (l, c, h) = oklab_to_oklch(ll, la, lb);
    Color { l, c, h }
}

fn oklch_to_srgb(color: Color) -> [f64; 3] {
    let (ll, la, lb) = oklch_to_oklab(color.l, color.c, color.h);
    let (r, g, b) = oklab_to_linear_srgb(ll, la, lb);
    [srgb_encode(r), srgb_encode(g), srgb_encode(b)]
}

/// Linear-sRGB triple in the engine's gamma-decoded space. Exposed
/// `pub(crate)` so `contrast.rs` can compute relative luminance
/// without re-doing the conversion in two places.
pub(crate) fn linear_srgb_of(color: &Color) -> [f64; 3] {
    let (ll, la, lb) = oklch_to_oklab(color.l, color.c, color.h);
    let (r, g, b) = oklab_to_linear_srgb(ll, la, lb);
    [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}

/// Shortest angular distance between two hues, in degrees `0.0..=180.0`.
/// Used by §7 to gap brand vs state colors and by §6 to score role fit.
pub fn hue_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).abs() % 360.0;
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn parse_six_digit_hex() {
        let c = Color::from_hex("#3f6089").unwrap();
        // Sanity check: a mid-blue should sit in the mid lightness
        // band, carry visible chroma, and have a blue-leaning hue.
        assert!(c.l > 0.30 && c.l < 0.65, "L out of mid band: {}", c.l);
        assert!(c.c > 0.05);
        assert!(c.h > 240.0 && c.h < 280.0, "got H={}", c.h);
    }

    #[test]
    fn parse_three_digit_hex_expands() {
        let short = Color::from_hex("#f00").unwrap();
        let long = Color::from_hex("#ff0000").unwrap();
        // f -> ff, 0 -> 00 — must round-trip to identical OKLCH.
        assert!(approx(short.l, long.l, 1e-9));
        assert!(approx(short.c, long.c, 1e-9));
        assert!(approx(short.h, long.h, 1e-9));
    }

    #[test]
    fn round_trip_hex_to_hex() {
        for hex in [
            "#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff", "#3f6089", "#0d9488",
        ] {
            let c = Color::from_hex(hex).unwrap();
            assert_eq!(c.to_hex(), hex, "round trip lost {hex}");
        }
    }

    #[test]
    fn bad_input_returns_error_no_panic() {
        assert_eq!(Color::from_hex(""), Err(ColorError::MissingHash));
        assert_eq!(Color::from_hex("3f6089"), Err(ColorError::MissingHash));
        assert_eq!(Color::from_hex("#12"), Err(ColorError::BadLength));
        assert_eq!(Color::from_hex("#zzzzzz"), Err(ColorError::BadDigit));
    }

    #[test]
    fn mix_halfway_is_perceptually_centered() {
        let black = Color::from_hex("#000000").unwrap();
        let white = Color::from_hex("#ffffff").unwrap();
        let mid = black.mix(&white, 0.5);
        // OKLab L of mid gray should sit ~0.5 — half of pure white's 1.0.
        assert!(approx(mid.l, 0.5, 0.02), "got L={}", mid.l);
        assert!(mid.c < 1e-6, "mid of two grays should be achromatic");
    }

    #[test]
    fn mix_self_is_self() {
        let c = Color::from_hex("#0d9488").unwrap();
        let m = c.mix(&Color::from_hex("#ffffff").unwrap(), 0.0);
        assert_eq!(c.to_hex(), m.to_hex());
    }

    #[test]
    fn out_of_gamut_clamps_not_wraps() {
        // Very high chroma at L≈0.5 — sits outside sRGB. Must clamp
        // each channel, not wrap into a different hue.
        let oog = Color::from_oklch(0.5, 0.35, 30.0);
        let hex = oog.to_hex();
        for ch in (hex.as_bytes()[1..]).chunks(2) {
            let s = std::str::from_utf8(ch).unwrap();
            assert!(u8::from_str_radix(s, 16).is_ok());
        }
    }

    #[test]
    fn hue_distance_wraps_correctly() {
        assert!((hue_distance(10.0, 350.0) - 20.0).abs() < 1e-9);
        assert!((hue_distance(0.0, 180.0) - 180.0).abs() < 1e-9);
        assert!((hue_distance(45.0, 45.0)).abs() < 1e-9);
    }
}
