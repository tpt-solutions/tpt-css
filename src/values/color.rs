// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Colors (spec 14.5, Tier 0): a subset of named colors, `transparent`,
//! `#rgb`, `#rrggbb`, `rgb()`, and `rgba()`.
//!
//! Gaps for M0 (recorded in SPEC_TRACE.md): the named color table has only the
//! 16 basic colors and `orange`, the full 148-name table is pending; `#rgba`
//! and `#rrggbbaa` are M1; `currentcolor` is resolved by the property layer,
//! not here.

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

/// Basic named colors (CSS Color 4 section 6.1, the 16 basic keywords) plus
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
        ComponentValue::Function(f) => {
            let name = f.name.to_ascii_lowercase();
            if name == "rgb" || name == "rgba" {
                parse_rgb_arguments(&f.value)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// `#rgb` or `#rrggbb`. Digits are hexadecimal; anything else is rejected.
fn parse_hex(digits: &str) -> Option<Color> {
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |c: char| c.to_digit(16).map(|d| d as u8);
    let chars: Vec<char> = digits.chars().collect();
    match chars.len() {
        3 => {
            let r = nibble(chars[0])? * 17;
            let g = nibble(chars[1])? * 17;
            let b = nibble(chars[2])? * 17;
            Some(Color::rgb8(r, g, b))
        }
        6 => {
            let byte = |i: usize| Some(nibble(chars[i])? * 16 + nibble(chars[i + 1])?);
            Some(Color::rgb8(byte(0)?, byte(2)?, byte(4)?))
        }
        _ => None,
    }
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
