// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property grammar parsing (spec 14.1, 15.1 `parse`).
//!
//! Turns component values into a typed [`SpecifiedValue`], or rejects them.
//! Rejected values are dropped at parse time, as CSS requires for values that
//! do not match the grammar. Values that contain `var()` are not parsed here:
//! the style layer substitutes them first and parses the result later.

use super::registry::{Grammar, PropertySpec, FONT_STRETCH_KEYWORDS};
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
        Grammar::FontStretch => {
            keyword(values, FONT_STRETCH_KEYWORDS).or(font_stretch_percentage(values))
        }
        Grammar::Image => parse_image_list(values),
        Grammar::Counters { default } => parse_counter_list(values, default),
        Grammar::PassThrough => {
            if crate::values::raw::is_passable(values) {
                Some(Value::Raw(crate::values::raw::serialize_components(values)))
            } else {
                None
            }
        }
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
/// A single non-negative percentage, for `font-stretch`.
fn font_stretch_percentage(values: &[ComponentValue]) -> Option<Value> {
    match values {
        [ComponentValue::Token(t)] => match t.kind {
            TokenKind::Percentage(p) if p >= 0.0 => Some(Value::Percentage(p)),
            _ => None,
        },
        _ => None,
    }
}

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

/// Image functions accepted in `background-image`. Their arguments are not
/// checked: an image is never loaded here.
const IMAGE_FUNCTIONS: &[&str] = &[
    "url",
    "linear-gradient",
    "radial-gradient",
    "conic-gradient",
    "repeating-linear-gradient",
    "repeating-radial-gradient",
    "repeating-conic-gradient",
    "image-set",
    "cross-fade",
];

/// `background-image`: `none`, or one or more images separated by commas.
fn parse_image_list(values: &[ComponentValue]) -> Option<Value> {
    if let [ComponentValue::Token(t)] = values {
        if let TokenKind::Ident(name) = &t.kind {
            return name
                .eq_ignore_ascii_case("none")
                .then_some(Value::Keyword("none"));
        }
    }
    let mut images = Vec::new();
    for part in values.split(|v| v.is_token(&TokenKind::Comma)) {
        let part = trim_whitespace(part);
        let is_image = match part {
            [ComponentValue::Token(t)] => matches!(t.kind, TokenKind::Url(_)),
            [ComponentValue::Function(f)] => IMAGE_FUNCTIONS
                .iter()
                .any(|n| f.name.eq_ignore_ascii_case(n)),
            _ => false,
        };
        if !is_image || !crate::values::raw::is_passable(part) {
            return None;
        }
        images.push(crate::values::raw::serialize_components(part));
    }
    Some(Value::Raw(images.join(", ")))
}

/// `counter-reset`, `counter-set`, `counter-increment`: `none`, or one or more
/// counter names, each with an optional integer (CSS Lists 3).
fn parse_counter_list(values: &[ComponentValue], default: i64) -> Option<Value> {
    if let [ComponentValue::Token(t)] = values {
        if matches!(&t.kind, TokenKind::Ident(name) if name.eq_ignore_ascii_case("none")) {
            return Some(Value::Keyword("none"));
        }
    }
    let mut out: Vec<(String, i64)> = Vec::new();
    let mut i = 0;
    let values = trim_whitespace(values);
    while i < values.len() {
        let ComponentValue::Token(t) = &values[i] else {
            return None;
        };
        let TokenKind::Ident(name) = &t.kind else {
            return None;
        };
        // `none` and the CSS-wide keywords are not counter names.
        if name.eq_ignore_ascii_case("none") || css_wide(name).is_some() {
            return None;
        }
        i += 1;
        while i < values.len() && values[i].is_whitespace() {
            i += 1;
        }
        let mut amount = default;
        if let Some(ComponentValue::Token(n)) = values.get(i) {
            if let TokenKind::Number {
                value,
                ty: NumericType::Integer,
            } = n.kind
            {
                amount = value as i64;
                i += 1;
                while i < values.len() && values[i].is_whitespace() {
                    i += 1;
                }
            }
        }
        out.push((name.clone(), amount));
    }
    (!out.is_empty()).then_some(Value::Counters(out))
}

/// Counter list styles accepted in `counter()` and `counters()`.
const COUNTER_STYLES: &[&str] = &[
    "decimal",
    "none",
    "lower-alpha",
    "upper-alpha",
    "lower-latin",
    "upper-latin",
    "lower-roman",
    "upper-roman",
];

/// The comma-separated arguments of a function, each trimmed.
fn function_arguments(values: &[ComponentValue]) -> Vec<&[ComponentValue]> {
    values
        .split(|v| v.is_token(&TokenKind::Comma))
        .map(trim_whitespace)
        .collect()
}

/// The single identifier in `part`, if it is one.
fn ident_of(part: &[ComponentValue]) -> Option<&str> {
    match part {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(s) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

/// A list style name, lowercased, if it is one this engine supports.
fn counter_style(part: &[ComponentValue]) -> Option<String> {
    let name = ident_of(part)?.to_ascii_lowercase();
    COUNTER_STYLES.contains(&name.as_str()).then_some(name)
}

/// `counter(name)` or `counter(name, style)`.
fn parse_counter_function(args: &[&[ComponentValue]]) -> Option<ContentItem> {
    let name = ident_of(args.first()?)?.to_string();
    let style = match args.get(1) {
        Some(part) => counter_style(part)?,
        None => "decimal".to_string(),
    };
    if args.len() > 2 {
        return None;
    }
    Some(ContentItem::Counter { name, style })
}

/// `counters(name, separator)` or `counters(name, separator, style)`.
fn parse_counters_function(args: &[&[ComponentValue]]) -> Option<ContentItem> {
    let name = ident_of(args.first()?)?.to_string();
    let separator = match *args.get(1)? {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::String(s) => s.clone(),
            _ => return None,
        },
        _ => return None,
    };
    let style = match args.get(2) {
        Some(part) => counter_style(part)?,
        None => "decimal".to_string(),
    };
    if args.len() > 3 {
        return None;
    }
    Some(ContentItem::Counters {
        name,
        separator,
        style,
    })
}

fn parse_content(values: &[ComponentValue]) -> Option<Value> {
    let mut out = Vec::new();
    let mut needs_computing = false;
    for v in values {
        match v {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::String(s) => out.push(ContentItem::Text(s.clone())),
                TokenKind::Whitespace => {}
                _ => return None,
            },
            ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("attr") => {
                let args = function_arguments(&f.value);
                out.push(parse_attr_function(&args)?);
                needs_computing = true;
            }
            ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("counter") => {
                let args = function_arguments(&f.value);
                out.push(parse_counter_function(&args)?);
                needs_computing = true;
            }
            ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("counters") => {
                let args = function_arguments(&f.value);
                out.push(parse_counters_function(&args)?);
                needs_computing = true;
            }
            _ => return None,
        }
    }
    if out.is_empty() {
        return None;
    }
    if needs_computing {
        return Some(Value::Content(out));
    }
    let strings = out
        .into_iter()
        .filter_map(|item| match item {
            ContentItem::Text(s) => Some(s),
            _ => None,
        })
        .collect();
    Some(Value::Strings(strings))
}

/// The name in `attr(name)`. Only a plain identifier is accepted; the type
/// and fallback forms are not supported. Names are lowercased, since HTML
/// attribute names are case-insensitive.
/// `attr(name)`, `attr(name string)`, `attr(name, "fallback")`, or both. Only the
/// `string` type is supported, because it is the default for `content`. Other
/// types would need their own value parsing.
fn parse_attr_function(args: &[&[ComponentValue]]) -> Option<ContentItem> {
    let head: Vec<&ComponentValue> = args
        .first()?
        .iter()
        .filter(|v| !v.is_whitespace())
        .collect();
    let name = match head.as_slice() {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(name) => name.to_ascii_lowercase(),
            _ => return None,
        },
        [ComponentValue::Token(n), ComponentValue::Token(ty)] => match (&n.kind, &ty.kind) {
            (TokenKind::Ident(name), TokenKind::Ident(ty)) if ty.eq_ignore_ascii_case("string") => {
                name.to_ascii_lowercase()
            }
            _ => return None,
        },
        _ => return None,
    };
    let fallback = match args.get(1) {
        None => None,
        Some(part) => match *part {
            [ComponentValue::Token(t)] => match &t.kind {
                TokenKind::String(s) => Some(s.clone()),
                _ => return None,
            },
            _ => return None,
        },
    };
    if args.len() > 2 {
        return None;
    }
    Some(ContentItem::Attr { name, fallback })
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
