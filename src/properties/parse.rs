// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property grammar parsing (spec 14.1, 15.1 `parse`).
//!
//! Turns component values into a typed [`SpecifiedValue`], or rejects them.
//! Rejected values are dropped at parse time, as CSS requires for values that
//! do not match the grammar. Values that contain `var()` are not parsed here:
//! the style layer substitutes them first and parses the result later.

use super::registry::{Grammar, LayerKind, PropertySpec, FONT_STRETCH_KEYWORDS};
use crate::syntax::{trim_whitespace, ComponentValue, NumericType, TokenKind};
use crate::values::color::parse_color;
use crate::values::length::parse_length_percentage;
use crate::values::{AngleUnit, ContentItem, Shadow, TimeUnit, TransformFunction, Value};

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
        Grammar::TimeList { non_negative } => parse_time_list(values, non_negative),
        Grammar::Shadow => parse_shadow_list(values),
        Grammar::Transform => parse_transform_list(values),
        Grammar::Filter => parse_filter_list(values),
        Grammar::BackgroundLayers { kind } => parse_background_layers(values, kind),
        Grammar::Number { non_negative } => {
            number(values, |v| !non_negative || v >= 0.0).map(Value::Number)
        }
        Grammar::TransitionProperty => parse_transition_property(values),
        Grammar::TimingFunctionList => parse_timing_function_list(values),
        Grammar::KeywordList { keywords } => parse_keyword_list(values, keywords),
        Grammar::IterationCountList => parse_iteration_count_list(values),
        Grammar::AnimationName => parse_animation_name(values),
        Grammar::Alternates => parse_alternates(values),
        Grammar::KeywordSet {
            keywords,
            exclusive,
        } => parse_keyword_set(values, keywords, exclusive),
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

/// Splits a comma-separated list, trimming each part. An empty part is kept, so
/// the caller rejects it.
fn comma_parts(values: &[ComponentValue]) -> Vec<&[ComponentValue]> {
    values
        .split(|v| v.is_token(&TokenKind::Comma))
        .map(trim_whitespace)
        .collect()
}

/// A time, `s` or `ms` (CSS Values 4).
fn parse_time(values: &[ComponentValue], non_negative: bool) -> Option<Value> {
    let [ComponentValue::Token(t)] = trim_whitespace(values) else {
        return None;
    };
    let TokenKind::Dimension { value, unit, .. } = &t.kind else {
        return None;
    };
    if non_negative && *value < 0.0 {
        return None;
    }
    let unit = match unit.to_ascii_lowercase().as_str() {
        "s" => TimeUnit::S,
        "ms" => TimeUnit::Ms,
        _ => return None,
    };
    Some(Value::Time {
        value: *value,
        unit,
    })
}

/// A comma-separated list of times. The result is always a list, so one time
/// and several times have the same shape.
fn parse_time_list(values: &[ComponentValue], non_negative: bool) -> Option<Value> {
    comma_parts(values)
        .into_iter()
        .map(|part| parse_time(part, non_negative))
        .collect::<Option<Vec<_>>>()
        .map(Value::List)
}

/// An angle, `deg`, `grad`, `rad`, or `turn` (CSS Values 4). A bare zero is not
/// an angle here.
fn parse_angle(values: &[ComponentValue]) -> Option<Value> {
    let [ComponentValue::Token(t)] = trim_whitespace(values) else {
        return None;
    };
    let TokenKind::Dimension { value, unit, .. } = &t.kind else {
        return None;
    };
    let unit = match unit.to_ascii_lowercase().as_str() {
        "deg" => AngleUnit::Deg,
        "grad" => AngleUnit::Grad,
        "rad" => AngleUnit::Rad,
        "turn" => AngleUnit::Turn,
        _ => return None,
    };
    Some(Value::Angle {
        value: *value,
        unit,
    })
}

/// `box-shadow`: `none`, or a comma-separated list of shadows.
fn parse_shadow_list(values: &[ComponentValue]) -> Option<Value> {
    if let Some(none) = keyword(values, &["none"]) {
        return Some(none);
    }
    comma_parts(values)
        .into_iter()
        .map(parse_shadow)
        .collect::<Option<Vec<_>>>()
        .map(Value::Shadows)
}

/// One shadow. Its parts may come in any order, except that the two to four
/// lengths must be next to each other.
fn parse_shadow(part: &[ComponentValue]) -> Option<Shadow> {
    let mut inset = false;
    let mut color: Option<Value> = None;
    let mut lengths: Vec<Value> = Vec::new();
    // Set once a non-length part follows the lengths, which ends their run.
    let mut lengths_closed = false;
    for item in part.iter().filter(|v| !v.is_whitespace()) {
        if let ComponentValue::Token(t) = item {
            if let TokenKind::Ident(name) = &t.kind {
                if name.eq_ignore_ascii_case("inset") {
                    if inset {
                        return None;
                    }
                    inset = true;
                    if !lengths.is_empty() {
                        lengths_closed = true;
                    }
                    continue;
                }
            }
        }
        if let Some(length) = parse_length_percentage(std::slice::from_ref(item), false, true)
            .filter(|v| matches!(v, Value::Length { .. }))
        {
            if lengths_closed || lengths.len() == 4 {
                return None;
            }
            lengths.push(length);
            continue;
        }
        if color.is_some() {
            return None;
        }
        if !lengths.is_empty() {
            lengths_closed = true;
        }
        color = Some(match item {
            ComponentValue::Token(t) if matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case("currentcolor")) => {
                Value::Keyword("currentcolor")
            }
            _ => Value::Color(parse_color(std::slice::from_ref(item))?),
        });
    }
    if lengths.len() < 2 {
        return None;
    }
    // A blur radius cannot be negative. A spread can.
    if let Some(Value::Length { value, .. }) = lengths.get(2) {
        if *value < 0.0 {
            return None;
        }
    }
    let mut lengths = lengths.into_iter();
    let x = lengths.next()?;
    let y = lengths.next()?;
    Some(Shadow {
        inset,
        x,
        y,
        blur: lengths.next(),
        spread: lengths.next(),
        color,
    })
}

/// The kinds of argument a transform function takes.
#[derive(Debug, Clone, Copy)]
enum ArgKind {
    /// A plain number.
    Number,
    /// An angle.
    Angle,
    /// A length without percentages.
    Length,
    /// A length or percentage.
    LengthPercentage,
}

const NUMBERS6: &[ArgKind] = &[ArgKind::Number; 6];
const NUMBERS16: &[ArgKind] = &[ArgKind::Number; 16];

/// The transform functions (CSS Transforms 2): name, argument kinds, and the
/// fewest arguments that are required. Names keep the grammar's capitals.
const TRANSFORM_FUNCTIONS: &[(&str, &[ArgKind], usize)] = &[
    ("matrix", NUMBERS6, 6),
    ("matrix3d", NUMBERS16, 16),
    (
        "translate",
        &[ArgKind::LengthPercentage, ArgKind::LengthPercentage],
        1,
    ),
    ("translateX", &[ArgKind::LengthPercentage], 1),
    ("translateY", &[ArgKind::LengthPercentage], 1),
    ("translateZ", &[ArgKind::Length], 1),
    (
        "translate3d",
        &[
            ArgKind::LengthPercentage,
            ArgKind::LengthPercentage,
            ArgKind::Length,
        ],
        3,
    ),
    ("scale", &[ArgKind::Number, ArgKind::Number], 1),
    ("scaleX", &[ArgKind::Number], 1),
    ("scaleY", &[ArgKind::Number], 1),
    ("scaleZ", &[ArgKind::Number], 1),
    (
        "scale3d",
        &[ArgKind::Number, ArgKind::Number, ArgKind::Number],
        3,
    ),
    ("rotate", &[ArgKind::Angle], 1),
    ("rotateX", &[ArgKind::Angle], 1),
    ("rotateY", &[ArgKind::Angle], 1),
    ("rotateZ", &[ArgKind::Angle], 1),
    (
        "rotate3d",
        &[
            ArgKind::Number,
            ArgKind::Number,
            ArgKind::Number,
            ArgKind::Angle,
        ],
        4,
    ),
    ("skew", &[ArgKind::Angle, ArgKind::Angle], 1),
    ("skewX", &[ArgKind::Angle], 1),
    ("skewY", &[ArgKind::Angle], 1),
    ("perspective", &[ArgKind::Length], 1),
];

/// `transform`: `none`, or one or more transform functions in order. The
/// functions are separated by spaces, and each function's arguments by commas.
fn parse_transform_list(values: &[ComponentValue]) -> Option<Value> {
    if let Some(none) = keyword(values, &["none"]) {
        return Some(none);
    }
    let mut functions = Vec::new();
    for item in trim_whitespace(values)
        .iter()
        .filter(|v| !v.is_whitespace())
    {
        let ComponentValue::Function(f) = item else {
            return None;
        };
        let &(name, kinds, min) = TRANSFORM_FUNCTIONS
            .iter()
            .find(|(name, _, _)| f.name.eq_ignore_ascii_case(name))?;
        let args = transform_args(&f.value, kinds, min)?;
        functions.push(TransformFunction { name, args });
    }
    (!functions.is_empty()).then_some(Value::Transforms(functions))
}

/// The filter functions whose amount is a non-negative number or percentage. An
/// omitted amount is 1 (Filter Effects 1 section 10).
const FILTER_AMOUNT_FUNCTIONS: &[&str] = &[
    "brightness",
    "contrast",
    "grayscale",
    "invert",
    "opacity",
    "saturate",
    "sepia",
];

/// The whitespace-separated parts of a value. Functions are single parts.
fn space_parts(values: &[ComponentValue]) -> Vec<&[ComponentValue]> {
    values
        .split(|v| v.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect()
}

/// `filter`: `none`, or one or more filter functions separated by spaces (Filter
/// Effects 1 section 10). A list that names a `url()` is kept as its canonical
/// text, because a reference is not resolved here.
fn parse_filter_list(values: &[ComponentValue]) -> Option<Value> {
    if keyword(values, &["none"]).is_some() {
        return Some(Value::Keyword("none"));
    }
    let parts = space_parts(values);
    if parts.is_empty() {
        return None;
    }
    let mut functions = Vec::new();
    let mut has_url = false;
    for part in parts {
        match trim_whitespace(part) {
            [ComponentValue::Token(t)] if matches!(t.kind, TokenKind::Url(_)) => has_url = true,
            [ComponentValue::Function(f)] => functions.push(filter_function(f)?),
            _ => return None,
        }
    }
    if has_url {
        return Some(Value::Raw(crate::values::raw::serialize_components(values)));
    }
    Some(Value::Transforms(functions))
}

/// One filter function with its arguments, or its defaults when they are omitted.
fn filter_function(f: &crate::syntax::Function) -> Option<TransformFunction> {
    let args = trim_whitespace(&f.value);
    let name = f.name.to_ascii_lowercase();
    if name == "blur" {
        let blur = if args.is_empty() {
            Value::px(0.0)
        } else {
            let length = parse_length_percentage(args, true, true)?;
            if !matches!(length, Value::Length { .. }) {
                return None;
            }
            length
        };
        return Some(TransformFunction {
            name: "blur",
            args: vec![blur],
        });
    }
    if let Some(&name) = FILTER_AMOUNT_FUNCTIONS.iter().find(|n| **n == name) {
        let amount = if args.is_empty() {
            1.0
        } else {
            number_or_percent(args).filter(|v| *v >= 0.0)?
        };
        return Some(TransformFunction {
            name,
            args: vec![Value::Number(amount)],
        });
    }
    if name == "hue-rotate" {
        let angle = if args.is_empty() {
            Value::Angle {
                value: 0.0,
                unit: AngleUnit::Deg,
            }
        } else {
            parse_angle(args)?
        };
        return Some(TransformFunction {
            name: "hue-rotate",
            args: vec![angle],
        });
    }
    if name == "drop-shadow" {
        return drop_shadow(&f.value).map(|args| TransformFunction {
            name: "drop-shadow",
            args,
        });
    }
    None
}

/// `drop-shadow()`: two lengths for the offsets, an optional third for the blur
/// (never negative), and an optional color, in any position among the parts.
/// The result is the offsets, the blur if given, and the color if given.
fn drop_shadow(values: &[ComponentValue]) -> Option<Vec<Value>> {
    let mut lengths = Vec::new();
    let mut color = None;
    for part in space_parts(values) {
        if let Some(c) = parse_color(part) {
            if color.replace(Value::Color(c)).is_some() {
                return None;
            }
        } else {
            let length = parse_length_percentage(part, false, true)?;
            if !matches!(length, Value::Length { .. }) {
                return None;
            }
            lengths.push(length);
        }
    }
    if !(2..=3).contains(&lengths.len()) {
        return None;
    }
    if let Some(Value::Length { value, .. }) = lengths.get(2) {
        if *value < 0.0 {
            return None;
        }
    }
    let mut args = lengths;
    args.extend(color);
    Some(args)
}

/// A background longhand: a comma-separated list with one item for each layer.
fn parse_background_layers(values: &[ComponentValue], kind: LayerKind) -> Option<Value> {
    comma_parts(values)
        .into_iter()
        .map(|layer| background_layer_item(layer, kind))
        .collect::<Option<Vec<_>>>()
        .map(Value::List)
}

/// One layer's item for a background longhand (CSS Backgrounds 3). Items that
/// are forms of several words are kept as their canonical text.
pub(crate) fn background_layer_item(layer: &[ComponentValue], kind: LayerKind) -> Option<Value> {
    let parts = space_parts(layer);
    if parts.is_empty() {
        return None;
    }
    match kind {
        LayerKind::Position => valid_position(&parts).then(|| words_text(&parts)),
        LayerKind::Size => size_item(&parts),
        LayerKind::Repeat => repeat_item(&parts),
        LayerKind::Attachment => single_keyword(&parts, &["scroll", "fixed", "local"]),
        LayerKind::Box => single_keyword(&parts, &["padding-box", "border-box", "content-box"]),
    }
}

/// The canonical text of a sequence of words, separated by single spaces.
fn words_text(parts: &[&[ComponentValue]]) -> Value {
    Value::Raw(
        parts
            .iter()
            .map(|part| crate::values::raw::serialize_components(part))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn single_keyword(parts: &[&[ComponentValue]], list: &'static [&'static str]) -> Option<Value> {
    match parts {
        [part] => keyword(part, list),
        _ => None,
    }
}

/// A keyword of `background-position` that names one side or the middle.
fn position_keyword(part: &[ComponentValue]) -> Option<&'static str> {
    match keyword(part, &["left", "center", "right", "top", "bottom"])? {
        Value::Keyword(k) => Some(k),
        _ => None,
    }
}

fn is_offset(part: &[ComponentValue]) -> bool {
    parse_length_percentage(part, false, true).is_some()
}

fn is_horizontal(k: &str) -> bool {
    k == "left" || k == "right"
}

fn is_vertical(k: &str) -> bool {
    k == "top" || k == "bottom"
}

/// `background-position`: one to four values (CSS Backgrounds 3 section 3.6). One
/// or two values are keywords or offsets, with the axes in order unless both are
/// keywords. Three or four values are edge keywords, each with an optional offset
/// after it, for two different axes.
fn valid_position(parts: &[&[ComponentValue]]) -> bool {
    match parts {
        [a] => position_keyword(a).is_some() || is_offset(a),
        [a, b] => match (position_keyword(a), position_keyword(b)) {
            (Some(x), Some(y)) => {
                !(is_horizontal(x) && is_horizontal(y)) && !(is_vertical(x) && is_vertical(y))
            }
            (Some(x), None) => is_offset(b) && !is_vertical(x),
            (None, Some(y)) => is_offset(a) && !is_horizontal(y),
            (None, None) => is_offset(a) && is_offset(b),
        },
        _ => edge_position(parts),
    }
}

/// The edge form of `background-position`: two edge keywords on different axes,
/// each with an optional offset after it.
fn edge_position(parts: &[&[ComponentValue]]) -> bool {
    let mut edges: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let Some(edge) = keyword(parts[i], &["left", "right", "top", "bottom"]) else {
            return false;
        };
        let Value::Keyword(edge) = edge else {
            return false;
        };
        edges.push(edge);
        i += 1;
        if i < parts.len() && is_offset(parts[i]) {
            i += 1;
        }
    }
    matches!(edges.as_slice(), [a, b] if is_horizontal(a) != is_horizontal(b))
}

/// `background-size`: `auto`, `cover`, or `contain` alone, or one or two
/// non-negative sizes, where `auto` is allowed (CSS Backgrounds 3 section 3.9).
fn size_item(parts: &[&[ComponentValue]]) -> Option<Value> {
    let is_size = |part: &[ComponentValue]| {
        keyword(part, &["auto"]).is_some() || parse_length_percentage(part, true, true).is_some()
    };
    match parts {
        [part] => keyword(part, &["auto", "cover", "contain"])
            .or_else(|| is_size(part).then(|| words_text(parts))),
        [a, b] => (is_size(a) && is_size(b)).then(|| words_text(parts)),
        _ => None,
    }
}

/// `background-repeat`: one keyword, or two of `repeat`, `space`, `round`, and
/// `no-repeat` (CSS Backgrounds 3 section 3.7).
fn repeat_item(parts: &[&[ComponentValue]]) -> Option<Value> {
    let pair_word = |part: &[ComponentValue]| {
        keyword(part, &["repeat", "space", "round", "no-repeat"]).is_some()
    };
    match parts {
        [part] => keyword(
            part,
            &[
                "repeat-x",
                "repeat-y",
                "repeat",
                "space",
                "round",
                "no-repeat",
            ],
        ),
        [a, b] => (pair_word(a) && pair_word(b)).then(|| words_text(parts)),
        _ => None,
    }
}

/// Identifiers that cannot name a transitioned property (CSS-wide keywords and
/// `default`, plus `none`, which has its own meaning).
fn is_reserved_property_name(name: &str) -> bool {
    matches!(
        name,
        "none" | "initial" | "inherit" | "unset" | "default" | "revert" | "revert-layer"
    )
}

/// `transition-property`: `none`, or a comma-separated list of `all` and property
/// names (CSS Transitions 1 section 2). Always a list, so one name and several
/// names have the same shape. Names are lowercase.
fn parse_transition_property(values: &[ComponentValue]) -> Option<Value> {
    if keyword(values, &["none"]).is_some() {
        return Some(Value::List(vec![Value::Keyword("none")]));
    }
    comma_parts(values)
        .into_iter()
        .map(parse_transition_property_name)
        .collect::<Option<Vec<_>>>()
        .map(Value::List)
}

/// One item of `transition-property`: `all` or a custom ident.
fn parse_transition_property_name(values: &[ComponentValue]) -> Option<Value> {
    let [ComponentValue::Token(t)] = trim_whitespace(values) else {
        return None;
    };
    let TokenKind::Ident(name) = &t.kind else {
        return None;
    };
    let name = name.to_ascii_lowercase();
    if name == "all" {
        return Some(Value::Keyword("all"));
    }
    if is_reserved_property_name(&name) {
        return None;
    }
    Some(Value::Raw(name))
}

/// The easing keywords (CSS Easing 1).
const EASING_KEYWORDS: &[&str] = &[
    "ease",
    "linear",
    "ease-in",
    "ease-out",
    "ease-in-out",
    "step-start",
    "step-end",
];

/// The `steps()` position keywords (CSS Easing 1 section 2.2).
const STEP_POSITIONS: &[&str] = &[
    "start",
    "end",
    "jump-start",
    "jump-end",
    "jump-none",
    "jump-both",
];

/// `transition-timing-function`: a comma-separated list of easing values. Each is
/// a keyword, `cubic-bezier()`, or `steps()`, and the result is always a list.
fn parse_timing_function_list(values: &[ComponentValue]) -> Option<Value> {
    comma_parts(values)
        .into_iter()
        .map(parse_easing)
        .collect::<Option<Vec<_>>>()
        .map(Value::List)
}

/// One easing value. The functions are checked and kept as canonical text.
fn parse_easing(values: &[ComponentValue]) -> Option<Value> {
    if let Some(k) = keyword(values, EASING_KEYWORDS) {
        return Some(k);
    }
    let [ComponentValue::Function(f)] = trim_whitespace(values) else {
        return None;
    };
    let args = comma_parts(&f.value);
    let valid = if f.name.eq_ignore_ascii_case("cubic-bezier") {
        valid_cubic_bezier(&args)
    } else if f.name.eq_ignore_ascii_case("steps") {
        valid_steps(&args)
    } else {
        false
    };
    valid.then(|| Value::Raw(crate::values::raw::serialize_components(values)))
}

/// A comma-separated list of keywords from `keywords`, one per item.
fn parse_keyword_list(
    values: &[ComponentValue],
    keywords: &'static [&'static str],
) -> Option<Value> {
    comma_parts(values)
        .into_iter()
        .map(|part| keyword(part, keywords))
        .collect::<Option<Vec<_>>>()
        .map(Value::List)
}

/// `animation-iteration-count`: a comma-separated list of non-negative numbers or
/// `infinite` (CSS Animations 1 section 3.2).
fn parse_iteration_count_list(values: &[ComponentValue]) -> Option<Value> {
    comma_parts(values)
        .into_iter()
        .map(|part| {
            keyword(part, &["infinite"]).or_else(|| number(part, |v| v >= 0.0).map(Value::Number))
        })
        .collect::<Option<Vec<_>>>()
        .map(Value::List)
}

/// `animation-name`: `none`, or a comma-separated list of names. Names keep their
/// case, because keyframe names are case-sensitive (CSS Animations 1 section 3.1).
fn parse_animation_name(values: &[ComponentValue]) -> Option<Value> {
    if keyword(values, &["none"]).is_some() {
        return Some(Value::List(vec![Value::Keyword("none")]));
    }
    comma_parts(values)
        .into_iter()
        .map(|part| {
            let [ComponentValue::Token(t)] = trim_whitespace(part) else {
                return None;
            };
            let TokenKind::Ident(name) = &t.kind else {
                return None;
            };
            (!is_reserved_property_name(&name.to_ascii_lowercase()))
                .then(|| Value::Raw(name.clone()))
        })
        .collect::<Option<Vec<_>>>()
        .map(Value::List)
}

/// `cubic-bezier(x1, y1, x2, y2)`: the x values lie in `[0, 1]` (CSS Easing 1).
fn valid_cubic_bezier(args: &[&[ComponentValue]]) -> bool {
    args.len() == 4
        && number(args[0], |x| (0.0..=1.0).contains(&x)).is_some()
        && number(args[1], |_| true).is_some()
        && number(args[2], |x| (0.0..=1.0).contains(&x)).is_some()
        && number(args[3], |_| true).is_some()
}

/// `steps(n [, position])`: `n` is a positive integer. `jump-none` needs at least
/// two steps (CSS Easing 1 section 2.2).
fn valid_steps(args: &[&[ComponentValue]]) -> bool {
    let (count, position) = match args {
        [count] => (count, None),
        [count, position] => (count, Some(*position)),
        _ => return false,
    };
    let Some(n) = integer(count) else {
        return false;
    };
    if n < 1 {
        return false;
    }
    match position {
        None => true,
        Some(position) => match keyword(position, STEP_POSITIONS) {
            Some(Value::Keyword("jump-none")) => n >= 2,
            Some(_) => true,
            None => false,
        },
    }
}

/// The arguments of one transform function, checked against its kinds. It takes
/// at least `min` and at most `kinds.len()` arguments.
fn transform_args(values: &[ComponentValue], kinds: &[ArgKind], min: usize) -> Option<Vec<Value>> {
    if trim_whitespace(values).is_empty() {
        return None;
    }
    let parts = comma_parts(values);
    if parts.len() < min || parts.len() > kinds.len() {
        return None;
    }
    parts
        .into_iter()
        .zip(kinds)
        .map(|(part, kind)| transform_arg(part, *kind))
        .collect()
}

fn transform_arg(part: &[ComponentValue], kind: ArgKind) -> Option<Value> {
    match kind {
        ArgKind::Number => match trim_whitespace(part) {
            [ComponentValue::Token(t)] => match &t.kind {
                TokenKind::Number { value, .. } => Some(Value::Number(*value)),
                _ => None,
            },
            _ => None,
        },
        ArgKind::Angle => parse_angle(part),
        ArgKind::Length => {
            parse_length_percentage(part, false, true).filter(|v| matches!(v, Value::Length { .. }))
        }
        ArgKind::LengthPercentage => parse_length_percentage(part, false, true),
    }
}

/// A set of keywords for a `KeywordSet` grammar. `normal` and `none` stand alone.
/// The result is one keyword, or the keywords in the grammar's order.
fn parse_keyword_set(
    values: &[ComponentValue],
    keywords: &'static [&'static str],
    exclusive: &'static [&'static [&'static str]],
) -> Option<Value> {
    let mut words = Vec::new();
    for item in trim_whitespace(values)
        .iter()
        .filter(|v| !v.is_whitespace())
    {
        match item {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::Ident(s) => words.push(s.to_ascii_lowercase()),
                _ => return None,
            },
            _ => return None,
        }
    }
    if words.is_empty() {
        return None;
    }
    if words.iter().any(|w| w == "normal" || w == "none") {
        return keyword(values, keywords);
    }
    let mut found: Vec<&'static str> = Vec::new();
    for word in &words {
        let k: &'static str = keywords.iter().copied().find(|k| *k == word.as_str())?;
        if found.contains(&k) {
            return None;
        }
        for group in exclusive {
            if group.contains(&k) && found.iter().any(|f| group.contains(f)) {
                return None;
            }
        }
        found.push(k);
    }
    found.sort_by_key(|k| keywords.iter().position(|x| x == k));
    Some(if found.len() == 1 {
        Value::Keyword(found[0])
    } else {
        Value::Keywords(found)
    })
}

/// `font-variant-alternates` (CSS Fonts 4 section 6.7). See `Grammar::Alternates`.
fn parse_alternates(values: &[ComponentValue]) -> Option<Value> {
    if let Some(normal) = keyword(values, &["normal"]) {
        return Some(normal);
    }
    let mut seen: Vec<String> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    for item in trim_whitespace(values)
        .iter()
        .filter(|v| !v.is_whitespace())
    {
        match item {
            ComponentValue::Token(t) => {
                let TokenKind::Ident(word) = &t.kind else {
                    return None;
                };
                if !word.eq_ignore_ascii_case("historical-forms") {
                    return None;
                }
                let name = "historical-forms".to_string();
                if seen.contains(&name) {
                    return None;
                }
                seen.push(name.clone());
                parts.push(name);
            }
            ComponentValue::Function(f) => {
                let name = f.name.to_ascii_lowercase();
                let args = comma_parts(&f.value);
                let text_args: Vec<String> = match name.as_str() {
                    "stylistic" | "swash" | "ornaments" | "annotation" => {
                        let [arg] = args.as_slice() else {
                            return None;
                        };
                        vec![ident_arg(arg)?]
                    }
                    "styleset" => {
                        if args.is_empty() || args.len() > 20 {
                            return None;
                        }
                        args.iter()
                            .map(|a| ident_arg(a))
                            .collect::<Option<Vec<_>>>()?
                    }
                    "character-variant" => {
                        if args.is_empty() || args.len() > 2 {
                            return None;
                        }
                        args.iter()
                            .map(|a| match trim_whitespace(a) {
                                [ComponentValue::Token(t)] => match &t.kind {
                                    TokenKind::Number {
                                        value,
                                        ty: NumericType::Integer,
                                    } if *value >= 0.0 => Some(format!("{}", *value as i64)),
                                    _ => None,
                                },
                                _ => None,
                            })
                            .collect::<Option<Vec<_>>>()?
                    }
                    _ => return None,
                };
                if seen.contains(&name) {
                    return None;
                }
                seen.push(name.clone());
                parts.push(format!("{name}({})", text_args.join(", ")));
            }
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| Value::Raw(parts.join(" ")))
}

/// One identifier argument of a function, as written.
fn ident_arg(part: &[ComponentValue]) -> Option<String> {
    match trim_whitespace(part) {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(s) => Some(s.clone()),
            _ => None,
        },
        _ => None,
    }
}
