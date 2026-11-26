// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property grammar parsing (spec 14.1, 15.1 `parse`).
//!
//! Turns component values into a typed [`SpecifiedValue`], or rejects them.
//! Rejected values are dropped at parse time, as CSS requires for values that
//! do not match the grammar. Values that contain `var()` are not parsed here:
//! the style layer substitutes them first and parses the result later.

use super::registry::{Grammar, PropertySpec};
use crate::syntax::{trim_whitespace, ComponentValue, NumericType, TokenKind};
use crate::values::color::parse_color;
use crate::values::length::parse_length_percentage;
use crate::values::{ContentItem, Value};

/// CSS-wide keywords (spec 13.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssWide {
    /// `initial`
    Initial,
    /// `inherit`
    Inherit,
    /// `unset`
    Unset,
    /// `revert`: roll back to the cascaded value from the origin below (spec 13.8).
    Revert,
    /// `revert-layer`: roll back to the value from the layer below (spec 13.8).
    RevertLayer,
}

/// The value does not match the property grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidValue;

/// A parsed declaration value before computation.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecifiedValue {
    /// A CSS-wide keyword, resolved during computation.
    CssWide(CssWide),
    /// A typed value.
    Value(Value),
    /// A value that contains `var()`, kept as tokens until computed-value time
    /// (spec 13.7). `shorthand` names the shorthand it came from, if any, so the
    /// expansion can run after substitution.
    Unparsed {
        /// The declaration's component values.
        tokens: Vec<ComponentValue>,
        /// Index into [`crate::properties::shorthand::SHORTHANDS`], if any.
        shorthand: Option<usize>,
    },
}

/// Parses a declaration value against a property's grammar.
///
/// Returns `Err(InvalidValue)` when the value is invalid. The caller reports the
/// diagnostic and drops the declaration.
pub fn parse_property_value(
    spec: &PropertySpec,
    values: &[ComponentValue],
) -> Result<SpecifiedValue, InvalidValue> {
    let values = trim_whitespace(values);
    if values.is_empty() {
        return Err(InvalidValue);
    }

    if let [ComponentValue::Token(t)] = values {
        if let TokenKind::Ident(name) = &t.kind {
            if let Some(wide) = css_wide(name) {
                return Ok(SpecifiedValue::CssWide(wide));
            }
        }
    }

    let value = match spec.grammar {
        Grammar::Keywords(list) => keyword(values, list),
        Grammar::Color => keyword(values, &["currentcolor"])
            .or_else(|| parse_light_dark(values))
            .or_else(|| parse_color(values).map(Value::Color)),
        Grammar::LengthPercentage {
            non_negative,
            keywords,
        } => keyword(values, keywords)
            .or_else(|| parse_length_percentage(values, non_negative, true)),
        Grammar::BorderWidth => keyword(values, &["thin", "medium", "thick"])
            .map(|k| match k {
                Value::Keyword("thin") => Value::px(1.0),
                Value::Keyword("thick") => Value::px(5.0),
                _ => Value::px(3.0),
            })
            .or_else(|| parse_length_percentage(values, true, true)),
        Grammar::LineHeight => keyword(values, &["normal"])
            .or_else(|| number(values, |v| v >= 0.0).map(Value::Number))
            .or_else(|| parse_length_percentage(values, true, true)),
        Grammar::FontWeight => keyword(values, &["normal", "bold", "bolder", "lighter"])
            .or_else(|| number(values, |v| (1.0..=1000.0).contains(&v)).map(Value::Number)),
        Grammar::FontFamily => parse_font_family(values),
        Grammar::TextDecorationLine => parse_decoration_line(values),
        Grammar::Length { keywords } => keyword(values, keywords).or_else(|| {
            parse_length_percentage(values, false, true)
                .filter(|v| matches!(v, Value::Length { .. }))
        }),
        Grammar::Content => keyword(values, &["normal", "none"]).or_else(|| parse_content(values)),
        Grammar::Opacity => number_or_percent(values).map(|v| Value::Number(v.clamp(0.0, 1.0))),
        Grammar::BorderRadius => parse_border_radius(values),
        Grammar::Integer { keywords } => {
            keyword(values, keywords).or_else(|| integer(values).map(|n| Value::Number(n as f64)))
        }
    };
    value.map(SpecifiedValue::Value).ok_or(InvalidValue)
}

fn css_wide(name: &str) -> Option<CssWide> {
    match name.to_ascii_lowercase().as_str() {
        "initial" => Some(CssWide::Initial),
        "inherit" => Some(CssWide::Inherit),
        "unset" => Some(CssWide::Unset),
        "revert" => Some(CssWide::Revert),
        "revert-layer" => Some(CssWide::RevertLayer),
        _ => None,
    }
}

/// Matches a single identifier against `list`, ASCII case-insensitively.
/// Returns the matching entry as a `'static` keyword value.
fn keyword(values: &[ComponentValue], list: &'static [&'static str]) -> Option<Value> {
    let [ComponentValue::Token(t)] = values else {
        return None;
    };
    let TokenKind::Ident(name) = &t.kind else {
        return None;
    };
    list.iter()
        .copied()
        .find(|k| k.eq_ignore_ascii_case(name))
        .map(Value::Keyword)
}

/// A single unitless number that satisfies `accept`.
fn number(values: &[ComponentValue], accept: impl Fn(f64) -> bool) -> Option<f64> {
    match values {
        [ComponentValue::Token(t)] => match t.kind {
            TokenKind::Number { value, .. } if accept(value) => Some(value),
            _ => None,
        },
        _ => None,
    }
}

/// A single number or percentage, as a number (`50%` is `0.5`).
fn number_or_percent(values: &[ComponentValue]) -> Option<f64> {
    match values {
        [ComponentValue::Token(t)] => match t.kind {
            TokenKind::Number { value, .. } => Some(value),
            TokenKind::Percentage(p) => Some(p / 100.0),
            _ => None,
        },
        _ => None,
    }
}

/// A single integer.
fn integer(values: &[ComponentValue]) -> Option<i64> {
    match values {
        [ComponentValue::Token(t)] => match t.kind {
            TokenKind::Number {
                value,
                ty: NumericType::Integer,
            } => Some(value as i64),
            _ => None,
        },
        _ => None,
    }
}

/// `font-family`: a comma-separated list. Each entry is either one string or
/// a sequence of identifiers joined with single spaces.
fn parse_font_family(values: &[ComponentValue]) -> Option<Value> {
    let mut names = Vec::new();
    for part in crate::parse::split_on(values, |v| v.is_token(&TokenKind::Comma)) {
        let part = trim_whitespace(part);
        if part.is_empty() {
            return None;
        }
        if let [ComponentValue::Token(t)] = part {
            if let TokenKind::String(s) = &t.kind {
                names.push(s.clone());
                continue;
            }
        }
        let mut words = Vec::new();
        for v in part {
            match v {
                ComponentValue::Token(t) => match &t.kind {
                    TokenKind::Ident(w) => words.push(w.as_str()),
                    TokenKind::Whitespace => {}
                    _ => return None,
                },
                _ => return None,
            }
        }
        names.push(words.join(" "));
    }
    Some(Value::FontFamily(names))
}

/// One or more strings, separated by whitespace (the `content` grammar).
/// One or two non-negative lengths or percentages: horizontal, then vertical.
/// Equal values collapse to one, so a circular radius is a plain value.
fn parse_border_radius(values: &[ComponentValue]) -> Option<Value> {
    let parts = crate::syntax::trim_whitespace(values);
    let chunks: Vec<&[ComponentValue]> = parts
        .split(|v| v.is_whitespace())
        .filter(|c| !c.is_empty())
        .collect();
    match chunks.as_slice() {
        [one] => parse_length_percentage(one, true, true),
        [h, v] => {
            let horizontal = parse_length_percentage(h, true, true)?;
            let vertical = parse_length_percentage(v, true, true)?;
            Some(if horizontal == vertical {
                horizontal
            } else {
                Value::Elliptical {
                    horizontal: Box::new(horizontal),
                    vertical: Box::new(vertical),
                }
            })
        }
        _ => None,
    }
}

fn parse_content(values: &[ComponentValue]) -> Option<Value> {
    let mut out = Vec::new();
    let mut has_attr = false;
    for v in values {
        match v {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::String(s) => out.push(ContentItem::Text(s.clone())),
                TokenKind::Whitespace => {}
                _ => return None,
            },
            ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("attr") => {
                let name = parse_attr_name(&f.value)?;
                has_attr = true;
                out.push(ContentItem::Attr(name));
            }
            _ => return None,
        }
    }
    if out.is_empty() {
        return None;
    }
    if has_attr {
        return Some(Value::Content(out));
    }
    let strings = out
        .into_iter()
        .filter_map(|item| match item {
            ContentItem::Text(s) => Some(s),
            ContentItem::Attr(_) => None,
        })
        .collect();
    Some(Value::Strings(strings))
}

/// The name in `attr(name)`. Only a plain identifier is accepted; the type
/// and fallback forms are not supported. Names are lowercased, since HTML
/// attribute names are case-insensitive.
fn parse_attr_name(values: &[ComponentValue]) -> Option<String> {
    match trim_whitespace(values) {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(name) => Some(name.to_ascii_lowercase()),
            _ => None,
        },
        _ => None,
    }
}

/// `light-dark(light, dark)`: each argument is a color. The choice is made at
/// computed-value time from the color scheme (CSS Color 5).
fn parse_light_dark(values: &[ComponentValue]) -> Option<Value> {
    let [ComponentValue::Function(f)] = values else {
        return None;
    };
    if !f.name.eq_ignore_ascii_case("light-dark") {
        return None;
    }
    let parts = crate::parse::split_on(&f.value, |v| v.is_token(&TokenKind::Comma));
    let [light, dark] = parts.as_slice() else {
        return None;
    };
    let light = parse_color(trim_whitespace(light))?;
    let dark = parse_color(trim_whitespace(dark))?;
    Some(Value::LightDark(light, dark))
}

/// The bit for a `text-decoration-line` keyword, as used by
/// [`TEXT_DECORATION_LINE_VALUES`]. `None` for anything else, including `none`.
pub(crate) fn decoration_line_bit(word: &str) -> Option<u8> {
    match word.to_ascii_lowercase().as_str() {
        "underline" => Some(1),
        "overline" => Some(2),
        "line-through" => Some(4),
        "blink" => Some(8),
        _ => None,
    }
}

/// `text-decoration-line`: `none`, or a set of the four line keywords. The set
/// is stored as the keyword for its combination.
fn parse_decoration_line(values: &[ComponentValue]) -> Option<Value> {
    let words: Vec<&str> = values
        .iter()
        .filter(|v| !v.is_whitespace())
        .map(|v| match v {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::Ident(s) => Some(s.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Option<_>>()?;
    if let [only] = words.as_slice() {
        if only.eq_ignore_ascii_case("none") {
            return Some(Value::Keyword("none"));
        }
    }
    let mut bits = 0u8;
    for word in words {
        let bit = decoration_line_bit(word)?;
        if bits & bit != 0 {
            return None;
        }
        bits |= bit;
    }
    if bits == 0 {
        return None;
    }
    Some(Value::Keyword(
        crate::properties::registry::TEXT_DECORATION_LINE_VALUES[usize::from(bits)],
    ))
}
