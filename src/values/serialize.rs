// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Deterministic value serialization (spec 7 `serialize`, 22.1).
//!
//! This is the canonical form used for debugging and golden tests. It is not
//! yet spec-exact CSSOM serialization (M3, spec 25.8). Colors are written in
//! the legacy `rgb()` and `rgba()` forms with 8-bit channels.

use super::calc;
use super::{color::Color, AngleUnit, ContentItem, Shadow, TimeUnit, TransformFunction, Value};

/// Serializes a value in canonical form.
pub fn serialize_value(value: &Value) -> String {
    match value {
        Value::Keyword(k) => (*k).to_string(),
        Value::Raw(text) => text.clone(),
        Value::Counters(list) => list
            .iter()
            .map(|(name, n)| format!("{name} {n}"))
            .collect::<Vec<_>>()
            .join(" "),
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
        Value::Content(items) => items
            .iter()
            .map(|item| match item {
                ContentItem::Text(s) => {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                }
                ContentItem::Attr { name, fallback } => match fallback {
                    None => format!("attr({name})"),
                    Some(text) => format!(
                        "attr({name}, \"{}\")",
                        text.replace('\\', "\\\\").replace('"', "\\\"")
                    ),
                },
                ContentItem::Counter { name, style } => format!("counter({name}, {style})"),
                ContentItem::Counters {
                    name,
                    separator,
                    style,
                } => format!(
                    "counters({name}, \"{}\", {style})",
                    separator.replace('\\', "\\\\").replace('"', "\\\"")
                ),
            })
            .collect::<Vec<_>>()
            .join(" "),
        Value::Elliptical {
            horizontal,
            vertical,
        } => format!(
            "{} {}",
            serialize_value(horizontal),
            serialize_value(vertical)
        ),
        Value::Calc(node) => calc::serialize(node),
        Value::Time { value, unit } => format!(
            "{}{}",
            format_number(*value),
            match unit {
                TimeUnit::S => "s",
                TimeUnit::Ms => "ms",
            }
        ),
        Value::Angle { value, unit } => format!(
            "{}{}",
            format_number(*value),
            match unit {
                AngleUnit::Deg => "deg",
                AngleUnit::Grad => "grad",
                AngleUnit::Rad => "rad",
                AngleUnit::Turn => "turn",
            }
        ),
        Value::List(items) => items
            .iter()
            .map(serialize_value)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Shadows(shadows) => shadows
            .iter()
            .map(serialize_shadow)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Keywords(words) => words.join(" "),
        Value::Transforms(functions) => functions
            .iter()
            .map(serialize_transform)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// One `box-shadow` layer: `inset`, the offsets, the blur and spread when they
/// are present, then the color when it is present.
fn serialize_shadow(shadow: &Shadow) -> String {
    let mut parts = Vec::new();
    if shadow.inset {
        parts.push("inset".to_string());
    }
    parts.push(serialize_value(&shadow.x));
    parts.push(serialize_value(&shadow.y));
    if let Some(blur) = &shadow.blur {
        parts.push(serialize_value(blur));
    }
    if let Some(spread) = &shadow.spread {
        parts.push(serialize_value(spread));
    }
    if let Some(color) = &shadow.color {
        parts.push(serialize_value(color));
    }
    parts.join(" ")
}

/// One `transform` function: its name, then its arguments in parentheses.
fn serialize_transform(function: &TransformFunction) -> String {
    let args = function
        .args
        .iter()
        .map(serialize_value)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({args})", function.name)
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
