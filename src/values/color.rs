// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Colors (spec 14.5). Supported: `transparent`, the 148 CSS named colors,
//! `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`, `hsla()`,
//! `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color()` in `srgb` and
//! `srgb-linear`, and `color-mix()` in `srgb`. Every space is converted to sRGB at
//! parse time. Relative color syntax (`rgb(from ...)`) works for rgb, hsl, hwb, lab, lch,
//! oklab, and oklch; `color()` does not take it.
//! `currentcolor` is resolved by the property layer, not here.

use crate::syntax::{trim_whitespace, BlockKind, ComponentValue, TokenKind};

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
    ("aliceblue", [240, 248, 255]),
    ("antiquewhite", [250, 235, 215]),
    ("aqua", [0, 255, 255]),
    ("aquamarine", [127, 255, 212]),
    ("azure", [240, 255, 255]),
    ("beige", [245, 245, 220]),
    ("bisque", [255, 228, 196]),
    ("black", [0, 0, 0]),
    ("blanchedalmond", [255, 235, 205]),
    ("blue", [0, 0, 255]),
    ("blueviolet", [138, 43, 226]),
    ("brown", [165, 42, 42]),
    ("burlywood", [222, 184, 135]),
    ("cadetblue", [95, 158, 160]),
    ("chartreuse", [127, 255, 0]),
    ("chocolate", [210, 105, 30]),
    ("coral", [255, 127, 80]),
    ("cornflowerblue", [100, 149, 237]),
    ("cornsilk", [255, 248, 220]),
    ("crimson", [220, 20, 60]),
    ("cyan", [0, 255, 255]),
    ("darkblue", [0, 0, 139]),
    ("darkcyan", [0, 139, 139]),
    ("darkgoldenrod", [184, 134, 11]),
    ("darkgray", [169, 169, 169]),
    ("darkgreen", [0, 100, 0]),
    ("darkgrey", [169, 169, 169]),
    ("darkkhaki", [189, 183, 107]),
    ("darkmagenta", [139, 0, 139]),
    ("darkolivegreen", [85, 107, 47]),
    ("darkorange", [255, 140, 0]),
    ("darkorchid", [153, 50, 204]),
    ("darkred", [139, 0, 0]),
    ("darksalmon", [233, 150, 122]),
    ("darkseagreen", [143, 188, 143]),
    ("darkslateblue", [72, 61, 139]),
    ("darkslategray", [47, 79, 79]),
    ("darkslategrey", [47, 79, 79]),
    ("darkturquoise", [0, 206, 209]),
    ("darkviolet", [148, 0, 211]),
    ("deeppink", [255, 20, 147]),
    ("deepskyblue", [0, 191, 255]),
    ("dimgray", [105, 105, 105]),
    ("dimgrey", [105, 105, 105]),
    ("dodgerblue", [30, 144, 255]),
    ("firebrick", [178, 34, 34]),
    ("floralwhite", [255, 250, 240]),
    ("forestgreen", [34, 139, 34]),
    ("fuchsia", [255, 0, 255]),
    ("gainsboro", [220, 220, 220]),
    ("ghostwhite", [248, 248, 255]),
    ("gold", [255, 215, 0]),
    ("goldenrod", [218, 165, 32]),
    ("gray", [128, 128, 128]),
    ("green", [0, 128, 0]),
    ("greenyellow", [173, 255, 47]),
    ("grey", [128, 128, 128]),
    ("honeydew", [240, 255, 240]),
    ("hotpink", [255, 105, 180]),
    ("indianred", [205, 92, 92]),
    ("indigo", [75, 0, 130]),
    ("ivory", [255, 255, 240]),
    ("khaki", [240, 230, 140]),
    ("lavender", [230, 230, 250]),
    ("lavenderblush", [255, 240, 245]),
    ("lawngreen", [124, 252, 0]),
    ("lemonchiffon", [255, 250, 205]),
    ("lightblue", [173, 216, 230]),
    ("lightcoral", [240, 128, 128]),
    ("lightcyan", [224, 255, 255]),
    ("lightgoldenrodyellow", [250, 250, 210]),
    ("lightgray", [211, 211, 211]),
    ("lightgreen", [144, 238, 144]),
    ("lightgrey", [211, 211, 211]),
    ("lightpink", [255, 182, 193]),
    ("lightsalmon", [255, 160, 122]),
    ("lightseagreen", [32, 178, 170]),
    ("lightskyblue", [135, 206, 250]),
    ("lightslategray", [119, 136, 153]),
    ("lightslategrey", [119, 136, 153]),
    ("lightsteelblue", [176, 196, 222]),
    ("lightyellow", [255, 255, 224]),
    ("lime", [0, 255, 0]),
    ("limegreen", [50, 205, 50]),
    ("linen", [250, 240, 230]),
    ("magenta", [255, 0, 255]),
    ("maroon", [128, 0, 0]),
    ("mediumaquamarine", [102, 205, 170]),
    ("mediumblue", [0, 0, 205]),
    ("mediumorchid", [186, 85, 211]),
    ("mediumpurple", [147, 112, 219]),
    ("mediumseagreen", [60, 179, 113]),
    ("mediumslateblue", [123, 104, 238]),
    ("mediumspringgreen", [0, 250, 154]),
    ("mediumturquoise", [72, 209, 204]),
    ("mediumvioletred", [199, 21, 133]),
    ("midnightblue", [25, 25, 112]),
    ("mintcream", [245, 255, 250]),
    ("mistyrose", [255, 228, 225]),
    ("moccasin", [255, 228, 181]),
    ("navajowhite", [255, 222, 173]),
    ("navy", [0, 0, 128]),
    ("oldlace", [253, 245, 230]),
    ("olive", [128, 128, 0]),
    ("olivedrab", [107, 142, 35]),
    ("orange", [255, 165, 0]),
    ("orangered", [255, 69, 0]),
    ("orchid", [218, 112, 214]),
    ("palegoldenrod", [238, 232, 170]),
    ("palegreen", [152, 251, 152]),
    ("paleturquoise", [175, 238, 238]),
    ("palevioletred", [219, 112, 147]),
    ("papayawhip", [255, 239, 213]),
    ("peachpuff", [255, 218, 185]),
    ("peru", [205, 133, 63]),
    ("pink", [255, 192, 203]),
    ("plum", [221, 160, 221]),
    ("powderblue", [176, 224, 230]),
    ("purple", [128, 0, 128]),
    ("rebeccapurple", [102, 51, 153]),
    ("red", [255, 0, 0]),
    ("rosybrown", [188, 143, 143]),
    ("royalblue", [65, 105, 225]),
    ("saddlebrown", [139, 69, 19]),
    ("salmon", [250, 128, 114]),
    ("sandybrown", [244, 164, 96]),
    ("seagreen", [46, 139, 87]),
    ("seashell", [255, 245, 238]),
    ("sienna", [160, 82, 45]),
    ("silver", [192, 192, 192]),
    ("skyblue", [135, 206, 235]),
    ("slateblue", [106, 90, 205]),
    ("slategray", [112, 128, 144]),
    ("slategrey", [112, 128, 144]),
    ("snow", [255, 250, 250]),
    ("springgreen", [0, 255, 127]),
    ("steelblue", [70, 130, 180]),
    ("tan", [210, 180, 140]),
    ("teal", [0, 128, 128]),
    ("thistle", [216, 191, 216]),
    ("tomato", [255, 99, 71]),
    ("turquoise", [64, 224, 208]),
    ("violet", [238, 130, 238]),
    ("wheat", [245, 222, 179]),
    ("white", [255, 255, 255]),
    ("whitesmoke", [245, 245, 245]),
    ("yellow", [255, 255, 0]),
    ("yellowgreen", [154, 205, 50]),
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
        ComponentValue::Function(f)
            if starts_with_from(&f.value) && Space::of(&f.name.to_ascii_lowercase()).is_some() =>
        {
            parse_relative(&f.name.to_ascii_lowercase(), &f.value)
        }
        ComponentValue::Function(f) => match f.name.to_ascii_lowercase().as_str() {
            "rgb" | "rgba" => parse_rgb_arguments(&f.value),
            "hsl" | "hsla" => parse_hsl(&f.value),
            "hwb" => parse_hwb(&f.value),
            "lab" => parse_lab(&f.value, false),
            "lch" => parse_lab(&f.value, true),
            "oklab" => parse_oklab(&f.value, false),
            "oklch" => parse_oklab(&f.value, true),
            "color" => parse_color_function(&f.value),
            "color-mix" => parse_color_mix(&f.value),
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

/// Splits a color function's arguments into components and an optional alpha.
/// Both the comma form and the space form (with `/` for alpha) are accepted.
fn components_and_alpha(
    args: &[ComponentValue],
) -> Option<(Vec<&ComponentValue>, Option<&ComponentValue>)> {
    let args = trim_whitespace(args);
    let has_comma = args.iter().any(|v| v.is_token(&TokenKind::Comma));
    let items: Vec<&ComponentValue> = if has_comma {
        let mut out = Vec::new();
        for part in crate::parse::split_on(args, |v| v.is_token(&TokenKind::Comma)) {
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
        [a, b, c] => Some((vec![*a, *b, *c], None)),
        [a, b, c, alpha] if has_comma => Some((vec![*a, *b, *c], Some(*alpha))),
        [a, b, c, slash, alpha] if slash.is_token(&TokenKind::Delim('/')) => {
            Some((vec![*a, *b, *c], Some(*alpha)))
        }
        _ => None,
    }
}

/// A component value as a number, where a percentage maps `100%` to `full`.
fn scaled(v: &ComponentValue, full: f64) -> Option<f64> {
    match as_arg(v)? {
        Arg::Number(n) => Some(n),
        Arg::Percent(p) => Some(p / 100.0 * full),
    }
}

/// `lab(L a b [/ alpha])` and `lch(L C h [/ alpha])` (CSS Color 4 section 9).
fn parse_lab(args: &[ComponentValue], polar: bool) -> Option<Color> {
    let (c, alpha) = components_and_alpha(args)?;
    let l = scaled(c[0], 100.0)?.clamp(0.0, 100.0);
    let (a, b) = if polar {
        let chroma = scaled(c[1], 150.0)?.max(0.0);
        let hue = hue_degrees(c[2])?.to_radians();
        (chroma * hue.cos(), chroma * hue.sin())
    } else {
        (scaled(c[1], 125.0)?, scaled(c[2], 125.0)?)
    };
    Some(from_linear_rgb_clamped(lab_to_srgb(l, a, b), alpha))
}

/// `oklab(L a b [/ alpha])` and `oklch(L C h [/ alpha])` (CSS Color 4 section 9).
fn parse_oklab(args: &[ComponentValue], polar: bool) -> Option<Color> {
    let (c, alpha) = components_and_alpha(args)?;
    let l = scaled(c[0], 1.0)?.clamp(0.0, 1.0);
    let (a, b) = if polar {
        let chroma = scaled(c[1], 0.4)?.max(0.0);
        let hue = hue_degrees(c[2])?.to_radians();
        (chroma * hue.cos(), chroma * hue.sin())
    } else {
        (scaled(c[1], 0.4)?, scaled(c[2], 0.4)?)
    };
    Some(from_linear_rgb_clamped(
        oklab_to_linear_srgb(l, a, b),
        alpha,
    ))
}

/// `color(srgb r g b [/ a])` and `color(srgb-linear ...)` (CSS Color 4 section 10).
fn parse_color_function(args: &[ComponentValue]) -> Option<Color> {
    if starts_with_from(args) {
        return parse_relative_color_function(args);
    }
    let args = trim_whitespace(args);
    let (space, rest) = match args.split_first() {
        Some((ComponentValue::Token(t), rest)) => match &t.kind {
            TokenKind::Ident(s) => (s.to_ascii_lowercase(), rest),
            _ => return None,
        },
        _ => return None,
    };
    let (c, alpha) = components_and_alpha(rest)?;
    if is_xyz_space(&space) {
        let xyz = [scaled(c[0], 1.0)?, scaled(c[1], 1.0)?, scaled(c[2], 1.0)?];
        let linear = xyz_to_linear(xyz, space == "xyz-d50");
        return Some(color_from_linear(linear, alpha_of(alpha.and_then(as_arg))));
    }
    let channels = [
        scaled(c[0], 1.0)?.clamp(0.0, 1.0),
        scaled(c[1], 1.0)?.clamp(0.0, 1.0),
        scaled(c[2], 1.0)?.clamp(0.0, 1.0),
    ];
    if let Some(w) = WideSpace::of(&space) {
        let alpha = alpha_of(alpha.and_then(as_arg));
        return Some(color_from_linear(w.to_srgb_linear(channels), alpha));
    }
    let srgb = match space.as_str() {
        "srgb" => channels,
        "srgb-linear" => channels.map(encode_srgb),
        _ => return None,
    };
    Some(Color {
        space: ColorSpace::Srgb,
        components: [srgb[0] as f32, srgb[1] as f32, srgb[2] as f32],
        alpha: alpha_of(alpha.and_then(as_arg)),
    })
}

/// A color followed by an optional percentage weight.
fn color_with_percent(values: &[ComponentValue]) -> Option<(Color, Option<f64>)> {
    let values = trim_whitespace(values);
    match values.last() {
        Some(ComponentValue::Token(t)) => match t.kind {
            TokenKind::Percentage(p) => {
                let color = parse_color(&values[..values.len() - 1])?;
                Some((color, Some(p)))
            }
            _ => Some((parse_color(values)?, None)),
        },
        _ => Some((parse_color(values)?, None)),
    }
}

/// sRGB transfer function: linear light to gamma-encoded (CSS Color 4 section 4).
fn encode_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// CIE Lab (D50) to linear sRGB, via XYZ (D50) and the Bradford-adapted matrix
/// used by CSS Color 4.
fn lab_to_srgb(l: f64, a: f64, b: f64) -> [f64; 3] {
    const KAPPA: f64 = 24389.0 / 27.0;
    const EPSILON: f64 = 216.0 / 24389.0;
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let finv = |f: f64| {
        let cube = f * f * f;
        if cube > EPSILON {
            cube
        } else {
            (116.0 * f - 16.0) / KAPPA
        }
    };
    // Reference white D50.
    let x = finv(fx) * 0.96422;
    let y = if l > KAPPA * EPSILON {
        fy * fy * fy
    } else {
        l / KAPPA
    };
    let z = finv(fz) * 0.82521;
    [
        3.1338561 * x - 1.6168667 * y - 0.4906146 * z,
        -0.9787684 * x + 1.9161415 * y + 0.0334540 * z,
        0.0719453 * x - 0.2289914 * y + 1.4052427 * z,
    ]
}

/// OKLab to linear sRGB, with the matrices from the published OKLab definition
/// (the same ones CSS Color 4 uses).
fn oklab_to_linear_srgb(l: f64, a: f64, b: f64) -> [f64; 3] {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3,
        -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3,
        -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701_0 * s3,
    ]
}

/// Linear sRGB to a gamma-encoded, gamut-clamped [`Color`]. `lab_to_srgb` returns
/// linear light, so both spaces go through the same encoding.
fn from_linear_rgb_clamped(linear: [f64; 3], alpha: Option<&ComponentValue>) -> Color {
    let srgb = linear.map(|c| encode_srgb(c.clamp(0.0, 1.0)).clamp(0.0, 1.0));
    Color {
        space: ColorSpace::Srgb,
        components: [srgb[0] as f32, srgb[1] as f32, srgb[2] as f32],
        alpha: alpha_of(alpha.and_then(as_arg)),
    }
}

// ---- relative color syntax (CSS Color 5) ------------------------------------------

/// The color functions that accept relative syntax (`rgb(from <color> ...)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Space {
    Rgb,
    Hsl,
    Hwb,
    Lab,
    Lch,
    Oklab,
    Oklch,
}

impl Space {
    fn of(name: &str) -> Option<Space> {
        Some(match name {
            "rgb" | "rgba" => Space::Rgb,
            "hsl" | "hsla" => Space::Hsl,
            "hwb" => Space::Hwb,
            "lab" => Space::Lab,
            "lch" => Space::Lch,
            "oklab" => Space::Oklab,
            "oklch" => Space::Oklch,
            _ => return None,
        })
    }

    /// The keywords that name the three channels in relative syntax.
    fn keywords(self) -> [&'static str; 3] {
        match self {
            Space::Rgb => ["r", "g", "b"],
            Space::Hsl => ["h", "s", "l"],
            Space::Hwb => ["h", "w", "b"],
            Space::Lab | Space::Oklab => ["l", "a", "b"],
            Space::Lch | Space::Oklch => ["l", "c", "h"],
        }
    }

    /// The value that 100% stands for in each channel. Hue takes no percentage
    /// in the specification, so 360 is used only to keep the arithmetic defined.
    fn percent_refs(self) -> [f64; 3] {
        match self {
            Space::Rgb => [255.0, 255.0, 255.0],
            Space::Hsl | Space::Hwb => [360.0, 100.0, 100.0],
            Space::Lab => [100.0, 125.0, 125.0],
            Space::Lch => [100.0, 150.0, 360.0],
            Space::Oklab => [1.0, 0.4, 0.4],
            Space::Oklch => [1.0, 0.4, 360.0],
        }
    }
}

/// The channel values of `color` in `space`, in the units that the keywords
/// use: 0 to 255 for `rgb`, degrees for hue, 0 to 100 for hsl and hwb
/// percentages, and the Lab and OKLab ranges for the rest.
fn channels_of(color: Color, space: Space) -> [f64; 3] {
    let [r, g, b] = color.components.map(f64::from);
    match space {
        Space::Rgb => [r * 255.0, g * 255.0, b * 255.0],
        Space::Hsl => {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            [h, s * 100.0, l * 100.0]
        }
        Space::Hwb => {
            let (h, _, _) = rgb_to_hsl(r, g, b);
            let white = r.min(g).min(b);
            let black = 1.0 - r.max(g).max(b);
            [h, white * 100.0, black * 100.0]
        }
        Space::Lab => srgb_to_lab(r, g, b),
        Space::Lch => {
            let [l, a, bb] = srgb_to_lab(r, g, b);
            polar(l, a, bb)
        }
        Space::Oklab => srgb_to_oklab(r, g, b),
        Space::Oklch => {
            let [l, a, bb] = srgb_to_oklab(r, g, b);
            polar(l, a, bb)
        }
    }
}

/// Lightness, chroma, and hue in degrees from a Lab-style triple.
fn polar(l: f64, a: f64, b: f64) -> [f64; 3] {
    let chroma = a.hypot(b);
    // Achromatic colors have no meaningful hue; zero keeps the value defined.
    let hue = if chroma < 1e-9 {
        0.0
    } else {
        b.atan2(a).to_degrees().rem_euclid(360.0)
    };
    [l, chroma, hue]
}

/// Hue in degrees, saturation, and lightness in `[0, 1]`.
fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let light = (max + min) / 2.0;
    if delta == 0.0 {
        return (0.0, 0.0, light);
    }
    let sat = delta / (1.0 - (2.0 * light - 1.0).abs());
    let hue = if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue.rem_euclid(360.0), sat, light)
}

/// The sRGB transfer function inverted: gamma-encoded to linear light.
fn linearize(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Inverse of the 3x3 matrix `m`, by cofactors.
fn invert3(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let cofactor = |r: usize, col: usize| {
        let rows: Vec<usize> = (0..3).filter(|&i| i != r).collect();
        let cols: Vec<usize> = (0..3).filter(|&j| j != col).collect();
        let minor =
            m[rows[0]][cols[0]] * m[rows[1]][cols[1]] - m[rows[0]][cols[1]] * m[rows[1]][cols[0]];
        if (r + col) % 2 == 0 {
            minor
        } else {
            -minor
        }
    };
    let mut out = [[0.0; 3]; 3];
    for (r, row) in out.iter_mut().enumerate() {
        for (col, cell) in row.iter_mut().enumerate() {
            // The inverse is the transpose of the cofactor matrix, over det.
            *cell = cofactor(col, r) / det;
        }
    }
    out
}

/// sRGB to CIE Lab (D50). This is the inverse of [`lab_to_srgb`].
fn srgb_to_lab(r: f64, g: f64, b: f64) -> [f64; 3] {
    const KAPPA: f64 = 24389.0 / 27.0;
    const EPSILON: f64 = 216.0 / 24389.0;
    // The matrix that lab_to_srgb inverts: XYZ (D50) to linear sRGB.
    const XYZ_TO_RGB: [[f64; 3]; 3] = [
        [3.1338561, -1.6168667, -0.4906146],
        [-0.9787684, 1.9161415, 0.0334540],
        [0.0719453, -0.2289914, 1.4052427],
    ];
    let linear = [linearize(r), linearize(g), linearize(b)];
    let inverse = invert3(XYZ_TO_RGB);
    let xyz: [f64; 3] = std::array::from_fn(|i| {
        inverse[i]
            .iter()
            .zip(linear)
            .map(|(m, v)| m * v)
            .sum::<f64>()
    });
    // Reference white D50, scaled out of the XYZ components.
    let reference = [xyz[0] / 0.96422, xyz[1], xyz[2] / 0.82521];
    let f = |t: f64| {
        if t > EPSILON {
            t.cbrt()
        } else {
            (KAPPA * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(reference[0]), f(reference[1]), f(reference[2]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// sRGB to OKLab. This is the inverse of [`oklab_to_linear_srgb`].
fn srgb_to_oklab(r: f64, g: f64, b: f64) -> [f64; 3] {
    let linear = [linearize(r), linearize(g), linearize(b)];
    let lms = [
        0.412_221_470_8 * linear[0] + 0.536_332_536_3 * linear[1] + 0.051_445_992_9 * linear[2],
        0.211_903_498_2 * linear[0] + 0.680_699_545_1 * linear[1] + 0.107_396_956_6 * linear[2],
        0.088_302_461_9 * linear[0] + 0.281_718_837_6 * linear[1] + 0.629_978_700_5 * linear[2],
    ];
    let cube = lms.map(f64::cbrt);
    [
        0.210_454_255_3 * cube[0] + 0.793_617_785_0 * cube[1] - 0.004_072_046_8 * cube[2],
        1.977_998_495_1 * cube[0] - 2.428_592_205_0 * cube[1] + 0.450_593_709_9 * cube[2],
        0.025_904_037_1 * cube[0] + 0.782_771_766_2 * cube[1] - 0.808_675_766_0 * cube[2],
    ]
}

/// A linear-light sRGB triple to a gamut-clamped [`Color`] with `alpha`.
fn color_from_linear(linear: [f64; 3], alpha: f32) -> Color {
    let srgb = linear.map(|c| encode_srgb(c.clamp(0.0, 1.0)).clamp(0.0, 1.0));
    Color {
        space: ColorSpace::Srgb,
        components: [srgb[0] as f32, srgb[1] as f32, srgb[2] as f32],
        alpha,
    }
}

/// Chroma and hue in degrees to the Cartesian `(a, b)` pair.
fn polar_to_rect(chroma: f64, hue: f64) -> (f64, f64) {
    let rad = hue.to_radians();
    (chroma * rad.cos(), chroma * rad.sin())
}

/// A color from channel values in `space`. This is the inverse of
/// [`channels_of`], and it clamps each channel to its range.
fn color_from_channels(space: Space, ch: [f64; 3], alpha: f64) -> Color {
    let alpha = alpha.clamp(0.0, 1.0) as f32;
    let srgb = |rgb: [f64; 3]| Color {
        space: ColorSpace::Srgb,
        components: rgb.map(|c| c.clamp(0.0, 1.0) as f32),
        alpha,
    };
    match space {
        Space::Rgb => srgb(ch.map(|c| c / 255.0)),
        Space::Hsl => srgb(hsl_to_rgb(
            ch[0].rem_euclid(360.0),
            (ch[1] / 100.0).clamp(0.0, 1.0),
            (ch[2] / 100.0).clamp(0.0, 1.0),
        )),
        Space::Hwb => {
            let white = (ch[1] / 100.0).clamp(0.0, 1.0);
            let black = (ch[2] / 100.0).clamp(0.0, 1.0);
            if white + black >= 1.0 {
                let gray = white / (white + black);
                srgb([gray; 3])
            } else {
                let base = hsl_to_rgb(ch[0].rem_euclid(360.0), 1.0, 0.5);
                srgb(base.map(|c| c * (1.0 - white - black) + white))
            }
        }
        Space::Lab => color_from_linear(lab_to_srgb(ch[0].clamp(0.0, 100.0), ch[1], ch[2]), alpha),
        Space::Lch => {
            let (a, b) = polar_to_rect(ch[1].max(0.0), ch[2]);
            color_from_linear(lab_to_srgb(ch[0].clamp(0.0, 100.0), a, b), alpha)
        }
        Space::Oklab => color_from_linear(
            oklab_to_linear_srgb(ch[0].clamp(0.0, 1.0), ch[1], ch[2]),
            alpha,
        ),
        Space::Oklch => {
            let (a, b) = polar_to_rect(ch[1].max(0.0), ch[2]);
            color_from_linear(oklab_to_linear_srgb(ch[0].clamp(0.0, 1.0), a, b), alpha)
        }
    }
}

/// Whether the arguments start with the `from` keyword (relative syntax).
pub(crate) fn starts_with_from(args: &[ComponentValue]) -> bool {
    matches!(
        trim_whitespace(args).first(),
        Some(ComponentValue::Token(t))
            if matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case("from"))
    )
}

/// The bindings for a channel expression: the channel keywords and `alpha`.
struct Bindings {
    names: [&'static str; 3],
    values: [f64; 3],
    alpha: f64,
}

impl Bindings {
    fn get(&self, name: &str) -> Option<f64> {
        if name.eq_ignore_ascii_case("alpha") {
            return Some(self.alpha);
        }
        self.names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))
            .map(|i| self.values[i])
    }
}

/// Evaluates one channel value: a keyword, `none`, a number, a percentage (of
/// `percent_ref`), a degree value, or a math function over those.
fn eval_channel(v: &ComponentValue, bind: &Bindings, percent_ref: f64) -> Option<f64> {
    match v {
        ComponentValue::Token(t) => match &t.kind {
            TokenKind::Number { value, .. } => Some(*value),
            TokenKind::Percentage(p) => Some(p / 100.0 * percent_ref),
            TokenKind::Dimension { value, unit, .. } if unit.eq_ignore_ascii_case("deg") => {
                Some(*value)
            }
            TokenKind::Ident(name) if name.eq_ignore_ascii_case("none") => Some(0.0),
            TokenKind::Ident(name) => bind.get(name),
            _ => None,
        },
        ComponentValue::Function(f) => eval_function(&f.name, &f.value, bind, percent_ref),
        ComponentValue::Block(b) if b.kind == BlockKind::Paren => {
            evaluate(&b.value, bind, percent_ref)
        }
        _ => None,
    }
}

/// Evaluates `values` as one expression, with whitespace ignored.
fn evaluate(values: &[ComponentValue], bind: &Bindings, percent_ref: f64) -> Option<f64> {
    let items: Vec<&ComponentValue> = values.iter().filter(|v| !v.is_whitespace()).collect();
    Parser {
        items,
        pos: 0,
        bind,
        percent_ref,
    }
    .finish()
}

/// `calc()`, `min()`, `max()`, and `clamp()` over channel values.
fn eval_function(
    name: &str,
    args: &[ComponentValue],
    bind: &Bindings,
    percent_ref: f64,
) -> Option<f64> {
    let parts = || -> Option<Vec<f64>> {
        crate::parse::split_on(args, |v| v.is_token(&TokenKind::Comma))
            .into_iter()
            .map(|part| evaluate(part, bind, percent_ref))
            .collect()
    };
    match name.to_ascii_lowercase().as_str() {
        "calc" => evaluate(args, bind, percent_ref),
        "min" => parts()?.into_iter().reduce(f64::min),
        "max" => parts()?.into_iter().reduce(f64::max),
        "clamp" => match parts()?.as_slice() {
            [lo, value, hi] => Some(lo.max(value.min(*hi))),
            _ => None,
        },
        _ => None,
    }
}

/// Recursive-descent evaluator for `+ - * /` over one expression's items.
struct Parser<'a> {
    items: Vec<&'a ComponentValue>,
    pos: usize,
    bind: &'a Bindings,
    percent_ref: f64,
}

impl Parser<'_> {
    fn peek_delim(&self) -> Option<char> {
        match self.items.get(self.pos) {
            Some(ComponentValue::Token(t)) => match t.kind {
                TokenKind::Delim(c) => Some(c),
                _ => None,
            },
            _ => None,
        }
    }

    /// Parses all items as one expression, failing on trailing input.
    fn finish(mut self) -> Option<f64> {
        let value = self.sum()?;
        (self.pos == self.items.len()).then_some(value)
    }

    fn sum(&mut self) -> Option<f64> {
        let mut acc = self.product()?;
        while let Some(op) = self.peek_delim().filter(|c| *c == '+' || *c == '-') {
            self.pos += 1;
            let rhs = self.product()?;
            acc = if op == '+' { acc + rhs } else { acc - rhs };
        }
        Some(acc)
    }

    fn product(&mut self) -> Option<f64> {
        let mut acc = self.unary()?;
        while let Some(op) = self.peek_delim().filter(|c| *c == '*' || *c == '/') {
            self.pos += 1;
            let rhs = self.unary()?;
            acc = if op == '*' {
                acc * rhs
            } else if rhs == 0.0 {
                return None;
            } else {
                acc / rhs
            };
        }
        Some(acc)
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek_delim() {
            Some('-') => {
                self.pos += 1;
                self.primary().map(|v| -v)
            }
            Some('+') => {
                self.pos += 1;
                self.primary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Option<f64> {
        let item = *self.items.get(self.pos)?;
        self.pos += 1;
        eval_channel(item, self.bind, self.percent_ref)
    }
}

/// `rgb(from <color> r g b [/ alpha])` and the same form for the other spaces.
/// The origin color supplies the channel values that the keywords stand for.
/// The alpha defaults to the origin's alpha (CSS Color 5).
fn parse_relative(name: &str, args: &[ComponentValue]) -> Option<Color> {
    let space = Space::of(name)?;
    let items: Vec<&ComponentValue> = trim_whitespace(args)
        .iter()
        .filter(|v| !v.is_whitespace())
        .collect();
    // items[0] is the `from` keyword and items[1] is the origin color.
    let origin = parse_color(std::slice::from_ref(*items.get(1)?))?;
    let rest = &items[2..];
    let (channel_items, alpha_item) = match rest {
        [a, b, c] => (vec![*a, *b, *c], None),
        [a, b, c, slash, alpha] if slash.is_token(&TokenKind::Delim('/')) => {
            (vec![*a, *b, *c], Some(*alpha))
        }
        _ => return None,
    };

    let bind = Bindings {
        names: space.keywords(),
        values: channels_of(origin, space),
        alpha: f64::from(origin.alpha),
    };
    let refs = space.percent_refs();
    let mut values = [0.0; 3];
    for (i, item) in channel_items.iter().enumerate() {
        values[i] = eval_channel(item, &bind, refs[i])?;
    }
    let alpha = match alpha_item {
        Some(item) => eval_channel(item, &bind, 1.0)?,
        None => f64::from(origin.alpha),
    };
    Some(color_from_channels(space, values, alpha))
}

// ---- color-mix() and relative color() ----------------------------------------------

/// A wide-gamut RGB space that CSS Color 4 defines beside sRGB (section 10).
/// Channels are gamma-encoded, and `to_xyz` maps linear light to XYZ. Display P3,
/// Rec. 2020, and A98 RGB use the D65 white point. ProPhoto RGB uses D50.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WideSpace {
    DisplayP3,
    Rec2020,
    A98Rgb,
    ProPhotoRgb,
}

/// Linear Rec. 2020 to XYZ (D65), from the primaries (CSS Color 4).
const REC2020_TO_XYZ_D65: [[f64; 3]; 3] = [
    [0.636_958_048_3, 0.144_616_903_6, 0.168_880_975_2],
    [0.262_700_212_0, 0.677_998_071_5, 0.059_301_716_5],
    [0.0, 0.028_072_693_0, 1.060_985_057_7],
];

/// Linear A98 RGB to XYZ (D65).
const A98_TO_XYZ_D65: [[f64; 3]; 3] = [
    [0.576_669_042_9, 0.185_558_237_9, 0.188_228_646_2],
    [0.297_344_925_9, 0.627_363_566_3, 0.075_291_458_5],
    [0.027_031_361_4, 0.070_688_852_5, 0.991_337_536_8],
];

/// Linear ProPhoto RGB to XYZ (D50).
const PROPHOTO_TO_XYZ_D50: [[f64; 3]; 3] = [
    [0.797_760_489_6, 0.135_191_708_2, 0.031_349_349_5],
    [0.288_071_128_2, 0.711_843_217_8, 0.000_085_654_0],
    [0.0, 0.0, 0.825_104_602_5],
];

const REC2020_ALPHA: f64 = 1.099_296_826_809_44;
const REC2020_BETA: f64 = 0.018_053_968_510_807;

/// Applies `f` to the magnitude of `v`, keeping its sign. The transfer
/// functions of these spaces are odd functions.
fn signed(v: f64, f: impl Fn(f64) -> f64) -> f64 {
    if v < 0.0 {
        -f(-v)
    } else {
        f(v)
    }
}

impl WideSpace {
    /// The space named by a `color()` or `color-mix()` identifier.
    fn of(name: &str) -> Option<WideSpace> {
        Some(match name {
            "display-p3" => WideSpace::DisplayP3,
            "rec2020" => WideSpace::Rec2020,
            "a98-rgb" => WideSpace::A98Rgb,
            "prophoto-rgb" => WideSpace::ProPhotoRgb,
            _ => return None,
        })
    }

    /// Whether the space's white point is D50, so its XYZ goes through the D50 path.
    fn d50(self) -> bool {
        self == WideSpace::ProPhotoRgb
    }

    /// Linear light to XYZ, in this space's white point.
    fn to_xyz(self) -> [[f64; 3]; 3] {
        match self {
            WideSpace::DisplayP3 => DISPLAY_P3_TO_XYZ_D65,
            WideSpace::Rec2020 => REC2020_TO_XYZ_D65,
            WideSpace::A98Rgb => A98_TO_XYZ_D65,
            WideSpace::ProPhotoRgb => PROPHOTO_TO_XYZ_D50,
        }
    }

    /// Gamma-encoded value to linear light.
    fn decode(self, v: f64) -> f64 {
        match self {
            WideSpace::DisplayP3 => linearize(v),
            WideSpace::Rec2020 => signed(v, |a| {
                if a < 4.5 * REC2020_BETA {
                    a / 4.5
                } else {
                    ((a + REC2020_ALPHA - 1.0) / REC2020_ALPHA).powf(1.0 / 0.45)
                }
            }),
            WideSpace::A98Rgb => signed(v, |a| a.powf(563.0 / 256.0)),
            WideSpace::ProPhotoRgb => signed(v, |a| {
                if a < 16.0 / 512.0 {
                    a / 16.0
                } else {
                    a.powf(1.8)
                }
            }),
        }
    }

    /// Linear light to gamma-encoded value.
    fn encode(self, l: f64) -> f64 {
        match self {
            WideSpace::DisplayP3 => encode_srgb(l),
            WideSpace::Rec2020 => signed(l, |a| {
                if a < REC2020_BETA {
                    4.5 * a
                } else {
                    REC2020_ALPHA * a.powf(0.45) - (REC2020_ALPHA - 1.0)
                }
            }),
            WideSpace::A98Rgb => signed(l, |a| a.powf(256.0 / 563.0)),
            WideSpace::ProPhotoRgb => signed(l, |a| {
                if a < 1.0 / 512.0 {
                    16.0 * a
                } else {
                    a.powf(1.0 / 1.8)
                }
            }),
        }
    }

    /// The gamma-encoded channels of an sRGB color in this space.
    fn encode_from_srgb(self, color: Color) -> [f64; 3] {
        let xyz = linear_to_xyz(
            color.components.map(|c| linearize(f64::from(c))),
            self.d50(),
        );
        mat_vec(invert3(self.to_xyz()), xyz).map(|l| self.encode(l))
    }

    /// The linear sRGB light of gamma-encoded channels in this space. It may be
    /// outside sRGB, and the caller clamps it.
    fn to_srgb_linear(self, channels: [f64; 3]) -> [f64; 3] {
        let xyz = mat_vec(self.to_xyz(), channels.map(|c| self.decode(c)));
        xyz_to_linear(xyz, self.d50())
    }
}

/// The spaces that `color-mix()` and relative `color()` can work in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MixSpace {
    Srgb,
    SrgbLinear,
    /// A wide-gamut RGB space, mixed in its gamma-encoded values (CSS Color 4).
    /// The result is converted back to sRGB, so colors outside sRGB are clipped.
    Wide(WideSpace),
    Xyz65,
    Xyz50,
    Model(Space),
}

impl MixSpace {
    fn of(name: &str) -> Option<MixSpace> {
        Some(match name {
            "srgb" => MixSpace::Srgb,
            "srgb-linear" => MixSpace::SrgbLinear,
            "xyz" | "xyz-d65" => MixSpace::Xyz65,
            "xyz-d50" => MixSpace::Xyz50,
            "hsl" => MixSpace::Model(Space::Hsl),
            "hwb" => MixSpace::Model(Space::Hwb),
            "lab" => MixSpace::Model(Space::Lab),
            "lch" => MixSpace::Model(Space::Lch),
            "oklab" => MixSpace::Model(Space::Oklab),
            "oklch" => MixSpace::Model(Space::Oklch),
            _ => return WideSpace::of(name).map(MixSpace::Wide),
        })
    }

    /// The index of the hue channel, for the polar spaces.
    fn hue_index(self) -> Option<usize> {
        match self {
            MixSpace::Model(Space::Hsl | Space::Hwb) => Some(0),
            MixSpace::Model(Space::Lch | Space::Oklch) => Some(2),
            _ => None,
        }
    }

    /// The channels of `color` in this space.
    fn channels(self, color: Color) -> [f64; 3] {
        match self {
            MixSpace::Srgb => color.components.map(f64::from),
            MixSpace::SrgbLinear => color.components.map(|c| linearize(f64::from(c))),
            MixSpace::Wide(w) => w.encode_from_srgb(color),

            MixSpace::Xyz65 => {
                linear_to_xyz(color.components.map(|c| linearize(f64::from(c))), false)
            }
            MixSpace::Xyz50 => {
                linear_to_xyz(color.components.map(|c| linearize(f64::from(c))), true)
            }
            MixSpace::Model(space) => channels_of(color, space),
        }
    }

    /// A color from channel values in this space.
    fn color(self, ch: [f64; 3], alpha: f64) -> Color {
        let alpha32 = alpha.clamp(0.0, 1.0) as f32;
        match self {
            MixSpace::Srgb => Color {
                space: ColorSpace::Srgb,
                components: ch.map(|c| c.clamp(0.0, 1.0) as f32),
                alpha: alpha32,
            },
            MixSpace::SrgbLinear => color_from_linear(ch.map(|c| c.clamp(0.0, 1.0)), alpha32),
            MixSpace::Wide(w) => color_from_linear(w.to_srgb_linear(ch), alpha32),

            MixSpace::Xyz65 => color_from_linear(xyz_to_linear(ch, false), alpha32),
            MixSpace::Xyz50 => color_from_linear(xyz_to_linear(ch, true), alpha32),
            MixSpace::Model(space) => color_from_channels(space, ch, alpha),
        }
    }
}

/// How hues are interpolated (CSS Color 4 section 12.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HueMethod {
    Shorter,
    Longer,
    /// The hue only moves upward, so the arc may be longer than 180 degrees.
    Increasing,
    /// The hue only moves downward.
    Decreasing,
}

/// Interpolates two colors in `space` with weights `wa` and `wb` (CSS Color 5).
/// Alpha is premultiplied for the non-hue channels, so transparent colors do not
/// tint the result. A hue moves along the arc that `method` selects.
fn mix_in(space: MixSpace, method: HueMethod, a: Color, b: Color, (wa, wb): (f64, f64)) -> Color {
    let (ca, cb) = (space.channels(a), space.channels(b));
    let (aa, ab) = (f64::from(a.alpha), f64::from(b.alpha));
    let alpha = aa * wa + ab * wb;
    let hue = space.hue_index();
    let mut out = [0.0; 3];
    for i in 0..3 {
        if Some(i) == hue {
            let start = ca[i];
            // The upward distance from start to end, in [0, 360).
            let up = (cb[i] - start).rem_euclid(360.0);
            // The shorter arc is within 180 degrees, in either direction.
            let short = if up > 180.0 { up - 360.0 } else { up };
            let step = match method {
                HueMethod::Shorter => short,
                HueMethod::Longer if short > 0.0 => short - 360.0,
                HueMethod::Longer if short < 0.0 => short + 360.0,
                HueMethod::Longer => 0.0,
                HueMethod::Increasing => up,
                HueMethod::Decreasing if up == 0.0 => 0.0,
                HueMethod::Decreasing => up - 360.0,
            };
            out[i] = (start + step * wb).rem_euclid(360.0);
        } else if alpha > 0.0 {
            out[i] = (ca[i] * aa * wa + cb[i] * ab * wb) / alpha;
        }
    }
    space.color(out, alpha)
}

/// The two weights of `color-mix()` from the optional percentages (spec 14.5).
fn mix_weights(pa: Option<f64>, pb: Option<f64>) -> Option<(f64, f64)> {
    let (wa, wb) = match (pa, pb) {
        (None, None) => (0.5, 0.5),
        (Some(p), None) => (p / 100.0, 1.0 - p / 100.0),
        (None, Some(q)) => (1.0 - q / 100.0, q / 100.0),
        (Some(p), Some(q)) => {
            let total = p + q;
            if total <= 0.0 {
                return None;
            }
            (p / total, q / total)
        }
    };
    ((0.0..=1.0).contains(&wa) && (0.0..=1.0).contains(&wb)).then_some((wa, wb))
}

/// `color-mix(in <space> [<hue method> hue], A [p%], B [q%])` (CSS Color 5).
fn parse_color_mix(args: &[ComponentValue]) -> Option<Color> {
    let parts = crate::parse::split_on(trim_whitespace(args), |v| v.is_token(&TokenKind::Comma));
    let [space, first, second] = parts.as_slice() else {
        return None;
    };
    let space_items: Vec<&ComponentValue> = trim_whitespace(space)
        .iter()
        .filter(|v| !v.is_whitespace())
        .collect();
    let (space_name, method) = match space_items.as_slice() {
        [in_kw, name] if is_ident(in_kw, "in") => (*name, HueMethod::Shorter),
        [in_kw, name, method, hue] if is_ident(in_kw, "in") && is_ident(hue, "hue") => {
            let method = match ident_lower(method)?.as_str() {
                "shorter" => HueMethod::Shorter,
                "longer" => HueMethod::Longer,
                "increasing" => HueMethod::Increasing,
                "decreasing" => HueMethod::Decreasing,
                _ => return None,
            };
            (*name, method)
        }
        _ => return None,
    };
    let space = MixSpace::of(&ident_lower(space_name)?)?;
    if space.hue_index().is_none() && method != HueMethod::Shorter {
        return None;
    }
    let (ca, pa) = color_with_percent(first)?;
    let (cb, pb) = color_with_percent(second)?;
    let weights = mix_weights(pa, pb)?;
    Some(mix_in(space, method, ca, cb, weights))
}

/// Whether `v` is the identifier `word`, ignoring case.
fn is_ident(v: &ComponentValue, word: &str) -> bool {
    matches!(v, ComponentValue::Token(t) if matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case(word)))
}

/// The lowercased identifier `v`, if it is one.
fn ident_lower(v: &ComponentValue) -> Option<String> {
    match v {
        ComponentValue::Token(t) => match &t.kind {
            TokenKind::Ident(s) => Some(s.to_ascii_lowercase()),
            _ => None,
        },
        _ => None,
    }
}

/// `color(from <color> srgb|srgb-linear r g b [/ alpha])` (CSS Color 5). The
/// keywords `r`, `g`, and `b` stand for the origin's channels in that space.
fn parse_relative_color_function(args: &[ComponentValue]) -> Option<Color> {
    let items: Vec<&ComponentValue> = trim_whitespace(args)
        .iter()
        .filter(|v| !v.is_whitespace())
        .collect();
    // items[0] is `from`, items[1] the origin, items[2] the color space.
    let origin = parse_color(std::slice::from_ref(*items.get(1)?))?;
    let space = match ident_lower(items.get(2)?)?.as_str() {
        "srgb" => MixSpace::Srgb,
        "srgb-linear" => MixSpace::SrgbLinear,
        _ => return None,
    };
    let rest = &items[3..];
    let (channel_items, alpha_item) = match rest {
        [a, b, c] => (vec![*a, *b, *c], None),
        [a, b, c, slash, alpha] if slash.is_token(&TokenKind::Delim('/')) => {
            (vec![*a, *b, *c], Some(*alpha))
        }
        _ => return None,
    };
    let bind = Bindings {
        names: ["r", "g", "b"],
        values: space.channels(origin),
        alpha: f64::from(origin.alpha),
    };
    let mut values = [0.0; 3];
    for (i, item) in channel_items.iter().enumerate() {
        values[i] = eval_channel(item, &bind, 1.0)?;
    }
    let alpha = match alpha_item {
        Some(item) => eval_channel(item, &bind, 1.0)?,
        None => f64::from(origin.alpha),
    };
    Some(space.color(values, alpha))
}

// ---- XYZ color spaces (CSS Color 4 section 10.3) -----------------------------------

/// Linear sRGB to XYZ with the D65 white point (the standard sRGB matrix).
/// Linear Display P3 to XYZ with the D65 white point (CSS Color 4).
const DISPLAY_P3_TO_XYZ_D65: [[f64; 3]; 3] = [
    [0.486_570_948_6, 0.265_667_693_2, 0.198_217_285_2],
    [0.228_974_564_1, 0.691_738_521_8, 0.079_286_914_1],
    [0.0, 0.045_113_381_9, 1.043_944_368_9],
];

const SRGB_TO_XYZ_D65: [[f64; 3]; 3] = [
    [0.412_390_799_3, 0.357_584_339_4, 0.180_480_788_4],
    [0.212_639_005_9, 0.715_168_678_8, 0.072_192_315_4],
    [0.019_330_818_7, 0.119_194_779_8, 0.950_532_152_2],
];

/// XYZ with the D50 white point to linear sRGB. This is the matrix that
/// [`lab_to_srgb`] uses, so the Lab and XYZ spaces agree.
const XYZ_D50_TO_LINEAR_SRGB: [[f64; 3]; 3] = [
    [3.1338561, -1.6168667, -0.4906146],
    [-0.9787684, 1.9161415, 0.0334540],
    [0.0719453, -0.2289914, 1.4052427],
];

fn mat_vec(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// Whether `space` names one of the XYZ spaces (`xyz`, `xyz-d65`, or `xyz-d50`).
fn is_xyz_space(space: &str) -> bool {
    matches!(space, "xyz" | "xyz-d65" | "xyz-d50")
}

/// Linear sRGB to XYZ, with the D65 or D50 white point.
fn linear_to_xyz(linear: [f64; 3], d50: bool) -> [f64; 3] {
    if d50 {
        mat_vec(invert3(XYZ_D50_TO_LINEAR_SRGB), linear)
    } else {
        mat_vec(SRGB_TO_XYZ_D65, linear)
    }
}

/// XYZ with the D65 or D50 white point to linear sRGB.
fn xyz_to_linear(xyz: [f64; 3], d50: bool) -> [f64; 3] {
    if d50 {
        mat_vec(XYZ_D50_TO_LINEAR_SRGB, xyz)
    } else {
        mat_vec(invert3(SRGB_TO_XYZ_D65), xyz)
    }
}
