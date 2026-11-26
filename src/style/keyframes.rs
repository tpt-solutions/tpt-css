// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Keyframe value interpolation (CSS Animations 1, `@keyframes`).
//!
//! This module computes the value of one property at a point along an
//! animation. It does not drive time: the host supplies the progress, and runs
//! the animation timeline (spec 3). Numbers, lengths, percentages, and colors
//! interpolate linearly. Any other value changes discretely, at the midpoint.
//!
//! Only keyframes that set the property take part. A progress before the first
//! or after the last such keyframe gives that keyframe's value.

use crate::ast::KeyframesRule;
use crate::properties::{lookup, parse_property_value, spec, SpecifiedValue};
use crate::values::{Color, LengthUnit, Value};

/// The value of `property` at `progress` (0.0 is the start, 1.0 the end) of the
/// keyframes `rule`. `None` if no keyframe sets the property, or the property is
/// unknown or its value does not parse.
pub fn value_at(rule: &KeyframesRule, property: &str, progress: f64) -> Option<Value> {
    let id = lookup(property)?;
    let mut points: Vec<(f64, Value)> = Vec::new();
    for keyframe in &rule.keyframes {
        let Some(decl) = keyframe
            .declarations
            .declarations
            .iter()
            .rev()
            .find(|d| d.property == property)
        else {
            continue;
        };
        let Ok(SpecifiedValue::Value(value)) = parse_property_value(spec(id), &decl.value) else {
            continue;
        };
        for selector in &keyframe.selectors {
            if let Some(offset) = offset_of(selector) {
                // A later keyframe at the same offset replaces an earlier one.
                points.retain(|(o, _)| (o - offset).abs() > f64::EPSILON);
                points.push((offset, value.clone()));
            }
        }
    }
    if points.is_empty() {
        return None;
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let p = progress.clamp(0.0, 1.0);
    let (first, last) = (&points[0], &points[points.len() - 1]);
    if p <= first.0 {
        return Some(first.1.clone());
    }
    if p >= last.0 {
        return Some(last.1.clone());
    }
    for pair in points.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if p >= a.0 && p <= b.0 {
            let t = if b.0 > a.0 {
                (p - a.0) / (b.0 - a.0)
            } else {
                1.0
            };
            return Some(interpolate(&a.1, &b.1, t));
        }
    }
    Some(last.1.clone())
}

/// The offset of a keyframe selector: `from` is 0, `to` is 1, and `N%` is N/100.
fn offset_of(selector: &str) -> Option<f64> {
    match selector {
        "from" => Some(0.0),
        "to" => Some(1.0),
        other => other
            .strip_suffix('%')
            .and_then(|n| n.parse::<f64>().ok())
            .map(|p| p / 100.0),
    }
}

/// Interpolates two values at `t` in `[0, 1]`. Values of different kinds change
/// discretely.
fn interpolate(a: &Value, b: &Value, t: f64) -> Value {
    let lerp = |x: f64, y: f64| x + (y - x) * t;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(lerp(*x, *y)),
        (Value::Percentage(x), Value::Percentage(y)) => Value::Percentage(lerp(*x, *y)),
        (
            Value::Length {
                value: x,
                unit: LengthUnit::Px,
            },
            Value::Length {
                value: y,
                unit: LengthUnit::Px,
            },
        ) => Value::px(lerp(*x, *y)),
        (Value::Color(x), Value::Color(y)) => Value::Color(lerp_color(x, y, t)),
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// Interpolates two colors in sRGB, component by component, with alpha.
fn lerp_color(a: &Color, b: &Color, t: f64) -> Color {
    let t = t as f32;
    let mut components = [0.0f32; 3];
    for (i, out) in components.iter_mut().enumerate() {
        *out = a.components[i] + (b.components[i] - a.components[i]) * t;
    }
    Color {
        space: a.space,
        components,
        alpha: a.alpha + (b.alpha - a.alpha) * t,
    }
}
