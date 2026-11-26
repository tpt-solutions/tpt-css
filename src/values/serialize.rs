// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Deterministic value serialization (spec 7 `serialize`, 22.1).
//!
//! This is the canonical form used for debugging and golden tests. It is not
//! yet spec-exact CSSOM serialization (M3, spec 25.8). Colors are written in
//! the legacy `rgb()` and `rgba()` forms with 8-bit channels.

use super::calc;
use super::{color::Color, Value};

/// Serializes a value in canonical form.
pub fn serialize_value(value: &Value) -> String {
    match value {
        Value::Keyword(k) => (*k).to_string(),
        Value::Length { value, unit } => {
            format!("{}{}", format_number(*value), calc::unit_name(*unit))
        }
        Value::Percentage(p) => format!("{}%", format_number(*p)),
        Value::Number(n) => format_number(*n),
        Value::Color(c) => serialize_color(c),
        Value::FontFamily(names) => names
            .iter()
            .map(|n| {
                if n.contains(' ') {
                    format!("\"{n}\"")
                } else {
                    n.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
        Value::LightDark(light, dark) => format!(
            "light-dark({}, {})",
            serialize_color(light),
            serialize_color(dark)
        ),
        Value::Strings(list) => list
            .iter()
            .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(" "),
        Value::Calc(node) => calc::serialize(node),
    }
}

/// Serializes a color as `rgb(r, g, b)` or `rgba(r, g, b, a)`.
pub fn serialize_color(color: &Color) -> String {
    let r = to_byte(color.components[0]);
    let g = to_byte(color.components[1]);
    let b = to_byte(color.components[2]);
    if color.alpha >= 1.0 {
        format!("rgb({r}, {g}, {b})")
    } else {
        format!("rgba({r}, {g}, {b}, {})", color.alpha)
    }
}

fn to_byte(channel: f32) -> u8 {
    (channel * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Formats a number without a trailing `.0` and without `-0`.
pub fn format_number(n: f64) -> String {
    if n == 0.0 {
        "0".to_string()
    } else {
        format!("{n}")
    }
}
