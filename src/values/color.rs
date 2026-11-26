// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Colors (spec 14.5). Supported: `transparent`, a subset of named colors,
//! `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`,
//! `hsla()`, and `hwb()`.
//!
//! Gaps (recorded in SPEC_TRACE.md): the named color table has only the 16
//! basic colors and `orange`, and the full 148-name table is pending.
//! `lab()`, `lch()`, `oklab()`, `oklch()`, and `color-mix()` are not
//! implemented. `currentcolor` is resolved by the property layer, not here.

use crate::syntax::{trim_whitespace, ComponentValue, TokenKind};

/// Color spaces. M0 supports sRGB only; the others arrive with the M1 color
/// functions (spec 14.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// Gamma-encoded sRGB.
    Srgb,
}

/// A color in one color space (spec 14.5).
///
/// `components` are the three color channels in `[0, 1]`. `alpha` is in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Color space.
    pub space: ColorSpace,
    /// Channels in `[0, 1]`.
    pub components: [f32; 3],
    /// Alpha in `[0, 1]`.
    pub alpha: f32,
}

impl Color {
    /// Opaque black, the M0 initial value of `color`.
    pub const BLACK: Color = Color {
        space: ColorSpace::Srgb,
        components: [0.0, 0.0, 0.0],
        alpha: 1.0,
    };

    /// Fully transparent black.
    pub const TRANSPARENT: Color = Color {
        space: ColorSpace::Srgb,
        components: [0.0, 0.0, 0.0],
        alpha: 0.0,
    };

    /// Creates an sRGB color from 8-bit channels and an opaque alpha.
    pub fn rgb8(r: u8, g: u8, b: u8) -> Color {
        Color {
            space: ColorSpace::Srgb,
            components: [
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
            ],
            alpha: 1.0,
        }
    }
}

/// Basic named colors (CSS Color 4, basic named colors) plus
/// `orange`. Values are the standard sRGB hex values.
const NAMED: &[(&str, [u8; 3])] = &[
    ("black", [0, 0, 0]),
    ("silver", [192, 192, 192]),
    ("gray", [128, 128, 128]),
    ("grey", [128, 128, 128]),
    ("white", [255, 255, 255]),
    ("maroon", [128, 0, 0]),
    ("red", [255, 0, 0]),
    ("purple", [128, 0, 128]),
    ("fuchsia", [255, 0, 255]),
    ("green", [0, 128, 0]),
    ("lime", [0, 255, 0]),
    ("olive", [128, 128, 0]),
    ("yellow", [255, 255, 0]),
    ("navy", [0, 0, 128]),
    ("blue", [0, 0, 255]),
    ("teal", [0, 128, 128]),
    ("aqua", [0, 255, 255]),
    ("orange", [255, 165, 0]),
];

/// Parses a single color value. Returns `None` if the value is not a color.
pub fn parse_color(values: &[ComponentValue]) -> Option<Color> {
    let values = trim_whitespace(values);
    let [single] = values else {
        return None;
    };
    match single {
        ComponentValue::Token(t) => match &t.kind {
            TokenKind::Ident(name) => {
                let lower = name.to_ascii_lowercase();
                if lower == "transparent" {
                    return Some(Color::TRANSPARENT);
                }
                NAMED
                    .iter()
                    .find(|(n, _)| *n == lower)
                    .map(|(_, [r, g, b])| Color::rgb8(*r, *g, *b))
            }
            TokenKind::Hash { value, .. } => parse_hex(value),
            _ => None,
        },
        ComponentValue::Function(f) => match f.name.to_ascii_lowercase().as_str() {
            "rgb" | "rgba" => parse_rgb_arguments(&f.value),
            "hsl" | "hsla" => parse_hsl(&f.value),
            "hwb" => parse_hwb(&f.value),
            _ => None,
        },
        _ => None,
    }
}

/// `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa`. Digits are hexadecimal;
/// anything else is rejected.
fn parse_hex(digits: &str) -> Option<Color> {
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |c: char| c.to_digit(16).map(|d| d as u8);
    let chars: Vec<char> = digits.chars().collect();
    let byte = |i: usize| Some(nibble(chars[i])? * 16 + nibble(chars[i + 1])?);
    match chars.len() {
        3 | 4 => {
            let r = nibble(chars[0])? * 17;
            let g = nibble(chars[1])? * 17;
            let b = nibble(chars[2])? * 17;
            let a = if chars.len() == 4 {
                f32::from(nibble(chars[3])? * 17) / 255.0
            } else {
                1.0
            };
            Some(with_alpha(Color::rgb8(r, g, b), a))
        }
        6 | 8 => {
            let a = if chars.len() == 8 {
                f32::from(byte(6)?) / 255.0
            } else {
                1.0
            };
            Some(with_alpha(Color::rgb8(byte(0)?, byte(2)?, byte(4)?), a))
        }
        _ => None,
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { alpha, ..color }
}

/// A hue angle in degrees: a number (degrees), or `deg`, `grad`, `rad`, or `turn`.
fn hue_degrees(v: &ComponentValue) -> Option<f64> {
    match v {
        ComponentValue::Token(t) => match &t.kind {
            TokenKind::Number { value, .. } => Some(*value),
            TokenKind::Dimension { value, unit, .. } => match unit.to_ascii_lowercase().as_str() {
                "deg" => Some(*value),
                "grad" => Some(*value * 0.9),
                "rad" => Some(*value * 180.0 / std::f64::consts::PI),
                "turn" => Some(*value * 360.0),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// A saturation, lightness, or whiteness/blackness value in `[0, 1]`:
/// a percentage, or a number read as a percentage (CSS Color 4 legacy form).
fn unit_interval(v: &ComponentValue) -> Option<f64> {
    match as_arg(v)? {
        Arg::Percent(p) => Some((p / 100.0).clamp(0.0, 1.0)),
        Arg::Number(n) => Some((n / 100.0).clamp(0.0, 1.0)),
    }
}

/// Splits `hsl()`/`hwb()` arguments into three components and an optional alpha.
fn hsl_parts(
    args: &[ComponentValue],
) -> Option<(
    &ComponentValue,
    &ComponentValue,
    &ComponentValue,
    Option<Arg>,
)> {
    let args = trim_whitespace(args);
    let has_comma = args.iter().any(|v| v.is_token(&TokenKind::Comma));
    let items: Vec<&ComponentValue> = if has_comma {
        let parts = crate::parse::split_on(args, |v| v.is_token(&TokenKind::Comma));
        let mut out = Vec::new();
        for part in parts {
            let part = trim_whitespace(part);
            let [only] = part else {
                return None;
            };
            out.push(only);
        }
        out
    } else {
        args.iter().filter(|v| !v.is_whitespace()).collect()
    };
    match items.as_slice() {
        [h, s, l] => Some((h, s, l, None)),
        [h, s, l, a] if has_comma => Some((h, s, l, Some(as_arg(a)?))),
        [h, s, l, slash, a] if slash.is_token(&TokenKind::Delim('/')) => {
            Some((h, s, l, Some(as_arg(a)?)))
        }
        _ => None,
    }
}

fn alpha_of(alpha: Option<Arg>) -> f32 {
    match alpha {
        None => 1.0,
        Some(Arg::Number(n)) => n.clamp(0.0, 1.0) as f32,
        Some(Arg::Percent(p)) => (p / 100.0).clamp(0.0, 1.0) as f32,
    }
}

/// `hsl(h s l [/ a])` and `hsla(h, s, l[, a])` (CSS Color 4 section 7.1).
fn parse_hsl(args: &[ComponentValue]) -> Option<Color> {
    let (h, s, l, alpha) = hsl_parts(args)?;
    let hue = hue_degrees(h)?.rem_euclid(360.0);
    let sat = unit_interval(s)?;
    let light = unit_interval(l)?;
    let [r, g, b] = hsl_to_rgb(hue, sat, light);
    Some(with_alpha(
        Color {
            space: ColorSpace::Srgb,
            components: [r as f32, g as f32, b as f32],
            alpha: 1.0,
        },
        alpha_of(alpha),
    ))
}

/// `hwb(h w b [/ a])` (CSS Color 4 section 7.2).
fn parse_hwb(args: &[ComponentValue]) -> Option<Color> {
    let (h, w, bl, alpha) = hsl_parts(args)?;
    let hue = hue_degrees(h)?.rem_euclid(360.0);
    let white = unit_interval(w)?;
    let black = unit_interval(bl)?;
    if white + black >= 1.0 {
        // Whiteness and blackness fill the whole range: the result is gray.
        let gray = (white / (white + black)) as f32;
        return Some(with_alpha(
            Color {
                space: ColorSpace::Srgb,
                components: [gray; 3],
                alpha: 1.0,
            },
            alpha_of(alpha),
        ));
    }
    let [r, g, b] = hsl_to_rgb(hue, 1.0, 0.5);
    let mix = |c: f64| c * (1.0 - white - black) + white;
    Some(with_alpha(
        Color {
            space: ColorSpace::Srgb,
            components: [mix(r) as f32, mix(g) as f32, mix(b) as f32],
            alpha: 1.0,
        },
        alpha_of(alpha),
    ))
}

/// HSL to sRGB in `[0, 1]`. `hue` in degrees, `sat` and `light` in `[0, 1]`.
fn hsl_to_rgb(hue: f64, sat: f64, light: f64) -> [f64; 3] {
    let c = (1.0 - (2.0 * light - 1.0).abs()) * sat;
    let hp = hue / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = light - c / 2.0;
    [r1 + m, g1 + m, b1 + m]
}

/// A channel or alpha argument: a number or a percentage.
#[derive(Clone, Copy)]
enum Arg {
    Number(f64),
    Percent(f64),
}

fn as_arg(v: &ComponentValue) -> Option<Arg> {
    match v {
        ComponentValue::Token(t) => match &t.kind {
            TokenKind::Number { value, .. } => Some(Arg::Number(*value)),
            TokenKind::Percentage(p) => Some(Arg::Percent(*p)),
            _ => None,
        },
        _ => None,
    }
}

/// Parses the arguments of `rgb()` or `rgba()`.
///
/// Two syntaxes are accepted and not mixed:
/// - legacy comma form: `rgb(r, g, b)` or `rgba(r, g, b, a)`
/// - space form: `rgb(r g b)` or `rgb(r g b / a)`
fn parse_rgb_arguments(args: &[ComponentValue]) -> Option<Color> {
    let args = trim_whitespace(args);
    let has_comma = args.iter().any(|v| v.is_token(&TokenKind::Comma));

    let (channels, alpha) = if has_comma {
        let parts: Vec<&[ComponentValue]> =
            crate::parse::split_on(args, |v| v.is_token(&TokenKind::Comma));
        if parts.len() != 3 && parts.len() != 4 {
            return None;
        }
        let mut items = Vec::with_capacity(parts.len());
        for part in parts {
            let part = trim_whitespace(part);
            let [only] = part else {
                return None;
            };
            items.push(as_arg(only)?);
        }
        let alpha = items.get(3).copied();
        (items[..3].to_vec(), alpha)
    } else {
        let items: Vec<&ComponentValue> = args.iter().filter(|v| !v.is_whitespace()).collect();
        match items.as_slice() {
            [r, g, b] => (vec![as_arg(r)?, as_arg(g)?, as_arg(b)?], None),
            [r, g, b, slash, a] if slash.is_token(&TokenKind::Delim('/')) => {
                (vec![as_arg(r)?, as_arg(g)?, as_arg(b)?], Some(as_arg(a)?))
            }
            _ => return None,
        }
    };

    let channel = |a: Arg| match a {
        Arg::Number(n) => (n / 255.0).clamp(0.0, 1.0),
        Arg::Percent(p) => (p / 100.0).clamp(0.0, 1.0),
    };
    let alpha_value = match alpha {
        None => 1.0,
        Some(Arg::Number(n)) => n.clamp(0.0, 1.0),
        Some(Arg::Percent(p)) => (p / 100.0).clamp(0.0, 1.0),
    };

    Some(Color {
        space: ColorSpace::Srgb,
        components: [
            channel(channels[0]) as f32,
            channel(channels[1]) as f32,
            channel(channels[2]) as f32,
        ],
        alpha: alpha_value as f32,
    })
}
