// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Shorthand expansion (spec 13.6, 15.3).
//!
//! A shorthand is expanded into its longhands before the cascade, so the
//! cascade compares longhands only. Values with `var()` are expanded later,
//! after substitution, by calling [`expand`] again.
//!
//! M1 shorthands: `margin`, `padding`, `border`, `border-width`,
//! `border-style`, `border-color`, the four `border-<side>` shorthands, and
//! `background`, which covers only `background-color`. `font` and `list-style`
//! are implemented, without `font-variant`, `font-stretch`, system fonts, or
//! list-style images other than `none`. `text-decoration` is not implemented,
//! and neither is `background` with images; a
//! declaration that uses them is dropped.

use super::parse::{parse_property_value, CssWide, InvalidValue, SpecifiedValue};
use super::registry::{lookup, spec, Initial};
use crate::syntax::{ComponentValue, TokenKind};
use crate::values::Value;

/// One longhand value produced by an expansion.
pub type Expanded = Vec<(&'static str, SpecifiedValue)>;

/// A shorthand property.
#[derive(Clone, Copy)]
pub struct ShorthandSpec {
    /// Shorthand name, lowercase ASCII.
    pub name: &'static str,
    /// Longhands it sets, for documentation and CSSOM later.
    pub longhands: &'static [&'static str],
    /// Expands a value (without CSS-wide keywords, which are handled by
    /// [`expand`]).
    pub expand: fn(&[ComponentValue]) -> Result<Expanded, InvalidValue>,
}

const MARGIN: [&str; 4] = ["margin-top", "margin-right", "margin-bottom", "margin-left"];
const PADDING: [&str; 4] = [
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
];
const BORDER_WIDTH: [&str; 4] = [
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];
const BORDER_STYLE: [&str; 4] = [
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
];
const BORDER_COLOR: [&str; 4] = [
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
];

/// Every shorthand supported in M1. A shorthand's index in this table is the
/// id stored in [`SpecifiedValue::Unparsed`].
pub const SHORTHANDS: &[ShorthandSpec] = &[
    ShorthandSpec {
        name: "margin",
        longhands: &MARGIN,
        expand: margin_expand,
    },
    ShorthandSpec {
        name: "padding",
        longhands: &PADDING,
        expand: padding_expand,
    },
    ShorthandSpec {
        name: "border-width",
        longhands: &BORDER_WIDTH,
        expand: border_width_expand,
    },
    ShorthandSpec {
        name: "border-style",
        longhands: &BORDER_STYLE,
        expand: border_style_expand,
    },
    ShorthandSpec {
        name: "border-color",
        longhands: &BORDER_COLOR,
        expand: border_color_expand,
    },
    ShorthandSpec {
        name: "border",
        longhands: &[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
        expand: border_all,
    },
    ShorthandSpec {
        name: "border-top",
        longhands: &["border-top-width", "border-top-style", "border-top-color"],
        expand: border_top,
    },
    ShorthandSpec {
        name: "border-right",
        longhands: &[
            "border-right-width",
            "border-right-style",
            "border-right-color",
        ],
        expand: border_right,
    },
    ShorthandSpec {
        name: "border-bottom",
        longhands: &[
            "border-bottom-width",
            "border-bottom-style",
            "border-bottom-color",
        ],
        expand: border_bottom,
    },
    ShorthandSpec {
        name: "border-left",
        longhands: &[
            "border-left-width",
            "border-left-style",
            "border-left-color",
        ],
        expand: border_left,
    },
    ShorthandSpec {
        name: "background",
        longhands: &["background-color"],
        expand: background,
    },
    ShorthandSpec {
        name: "font",
        longhands: &[
            "font-style",
            "font-weight",
            "font-size",
            "line-height",
            "font-family",
        ],
        expand: font,
    },
    ShorthandSpec {
        name: "list-style",
        longhands: &["list-style-type", "list-style-position", "list-style-image"],
        expand: list_style,
    },
    ShorthandSpec {
        name: "text-decoration",
        longhands: &[
            "text-decoration-line",
            "text-decoration-style",
            "text-decoration-color",
        ],
        expand: text_decoration,
    },
];

/// `font: [style] [weight] size [/ line-height] family` (CSS Fonts 4 section 4.3).
///
/// Only `font-style`, `font-weight`, `font-size`, `line-height`, and
/// `font-family` are registered, so `font-variant` and `font-stretch` are not
/// accepted here, and the system font keywords (`caption`, `menu`, and the
/// others) are not supported. Omitted style and weight take their initial
/// value, and an omitted line height resets to `normal`, as the shorthand requires.
fn font(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let values = crate::syntax::trim_whitespace(values);
    let items: Vec<(usize, &ComponentValue)> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| !v.is_whitespace())
        .collect();
    let mut style = None;
    let mut weight = None;
    let mut k = 0;
    // Style and weight come before the size, in either order.
    let size = loop {
        let item = items.get(k).ok_or(InvalidValue)?.1;
        if let Ok(size) = parse_as("font-size", std::slice::from_ref(item)) {
            if matches!(size, SpecifiedValue::CssWide(_)) {
                return Err(InvalidValue);
            }
            break size;
        }
        if style.is_none() {
            if let Ok(v) = parse_as("font-style", std::slice::from_ref(item)) {
                if !matches!(v, SpecifiedValue::CssWide(_)) {
                    style = Some(v);
                    k += 1;
                    continue;
                }
            }
        }
        if weight.is_none() {
            if let Ok(v) = parse_as("font-weight", std::slice::from_ref(item)) {
                if !matches!(v, SpecifiedValue::CssWide(_)) {
                    weight = Some(v);
                    k += 1;
                    continue;
                }
            }
        }
        return Err(InvalidValue);
    };
    k += 1;
    let mut line_height = SpecifiedValue::Value(Initial::Keyword("normal").to_value());
    if let Some((_, item)) = items.get(k) {
        if item.is_token(&TokenKind::Delim('/')) {
            let (_, lh) = *items.get(k + 1).ok_or(InvalidValue)?;
            line_height = parse_as("line-height", std::slice::from_ref(lh))?;
            if matches!(line_height, SpecifiedValue::CssWide(_)) {
                return Err(InvalidValue);
            }
            k += 2;
        }
    }
    let (start, _) = *items.get(k).ok_or(InvalidValue)?;
    let family = parse_as("font-family", &values[start..])?;
    if matches!(family, SpecifiedValue::CssWide(_)) {
        return Err(InvalidValue);
    }
    Ok(vec![
        (
            "font-style",
            style.unwrap_or(SpecifiedValue::Value(Initial::Keyword("normal").to_value())),
        ),
        (
            "font-weight",
            weight.unwrap_or(SpecifiedValue::Value(Initial::Keyword("normal").to_value())),
        ),
        ("font-size", size),
        ("line-height", line_height),
        ("font-family", family),
    ])
}

/// `list-style: [position || image || type]` (CSS Lists 3 section 3). A lone
/// `none` sets both the type and the image to `none`.
fn list_style(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    if parts.is_empty() || parts.len() > 3 {
        return Err(InvalidValue);
    }
    let mut position = None;
    let mut image = None;
    let mut kind = None;
    for part in &parts {
        let mut placed = false;
        if position.is_none() {
            if let Ok(v) = parse_as("list-style-position", part) {
                position = Some(v);
                placed = true;
            }
        }
        if !placed && kind.is_none() {
            if let Ok(v) = parse_as("list-style-type", part) {
                kind = Some(v);
                placed = true;
            }
        }
        if !placed && image.is_none() {
            if let Ok(v) = parse_as("list-style-image", part) {
                image = Some(v);
                placed = true;
            }
        }
        if !placed {
            return Err(InvalidValue);
        }
    }
    let initial = |name: &str| {
        let id = lookup(name).expect("registered");
        match spec(id).initial {
            Initial::Keyword(k) => SpecifiedValue::Value(Initial::Keyword(k).to_value()),
            _ => unreachable!("list-style initial values are keywords"),
        }
    };
    if let [only] = parts.as_slice() {
        if chunks_is_none(only) {
            return Ok(vec![
                (
                    "list-style-type",
                    SpecifiedValue::Value(Initial::Keyword("none").to_value()),
                ),
                ("list-style-position", initial("list-style-position")),
                (
                    "list-style-image",
                    SpecifiedValue::Value(Initial::Keyword("none").to_value()),
                ),
            ]);
        }
    }
    Ok(vec![
        (
            "list-style-type",
            kind.unwrap_or_else(|| initial("list-style-type")),
        ),
        (
            "list-style-position",
            position.unwrap_or_else(|| initial("list-style-position")),
        ),
        (
            "list-style-image",
            image.unwrap_or_else(|| initial("list-style-image")),
        ),
    ])
}

fn chunks_is_none(part: &[ComponentValue]) -> bool {
    matches!(part, [ComponentValue::Token(t)] if matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case("none")))
}

/// `text-decoration: <line> || <style> || <color>` (CSS Text Decoration 3). The
/// line keywords may be spread over several chunks. `none` is a line on its
/// own, so it cannot be combined with another line keyword.
fn text_decoration(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    if parts.is_empty() {
        return Err(InvalidValue);
    }
    let mut bits = 0u8;
    let mut saw_none = false;
    let mut style = None;
    let mut color = None;
    for part in &parts {
        if let [ComponentValue::Token(t)] = part {
            if let TokenKind::Ident(word) = &t.kind {
                if word.eq_ignore_ascii_case("none") {
                    if saw_none {
                        return Err(InvalidValue);
                    }
                    saw_none = true;
                    continue;
                }
                if let Some(bit) = super::parse::decoration_line_bit(word) {
                    if bits & bit != 0 {
                        return Err(InvalidValue);
                    }
                    bits |= bit;
                    continue;
                }
            }
        }
        if style.is_none() {
            if let Ok(v) = parse_as("text-decoration-style", part) {
                style = Some(v);
                continue;
            }
        }
        if color.is_none() {
            if let Ok(v) = parse_as("text-decoration-color", part) {
                color = Some(v);
                continue;
            }
        }
        return Err(InvalidValue);
    }
    if saw_none && bits != 0 {
        return Err(InvalidValue);
    }
    let line = if bits == 0 {
        "none"
    } else {
        crate::properties::registry::TEXT_DECORATION_LINE_VALUES[usize::from(bits)]
    };
    Ok(vec![
        (
            "text-decoration-line",
            SpecifiedValue::Value(Value::Keyword(line)),
        ),
        (
            "text-decoration-style",
            style.unwrap_or_else(|| initial_value("text-decoration-style")),
        ),
        (
            "text-decoration-color",
            color.unwrap_or_else(|| initial_value("text-decoration-color")),
        ),
    ])
}

/// The initial value of a longhand, as a specified value.
fn initial_value(name: &str) -> SpecifiedValue {
    let id = lookup(name).expect("registered");
    SpecifiedValue::Value(spec(id).initial.to_value())
}

/// Looks up a shorthand by name (already lowercased).
pub fn shorthand_index(name: &str) -> Option<usize> {
    SHORTHANDS.iter().position(|s| s.name == name)
}

/// Expands shorthand `index` for a value. Handles CSS-wide keywords, which apply
/// to every longhand (spec 13.8).
pub fn expand(index: usize, values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let shorthand = SHORTHANDS.get(index).ok_or(InvalidValue)?;
    if let Some(wide) = single_css_wide(values) {
        return Ok(shorthand
            .longhands
            .iter()
            .map(|name| (*name, SpecifiedValue::CssWide(wide)))
            .collect());
    }
    (shorthand.expand)(values)
}

fn single_css_wide(values: &[ComponentValue]) -> Option<CssWide> {
    let values = crate::syntax::trim_whitespace(values);
    match values {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(s) => match s.to_ascii_lowercase().as_str() {
                "initial" => Some(CssWide::Initial),
                "inherit" => Some(CssWide::Inherit),
                "unset" => Some(CssWide::Unset),
                "revert" => Some(CssWide::Revert),
                "revert-layer" => Some(CssWide::RevertLayer),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// Splits a value at top-level whitespace. Functions and blocks are single
/// values, so `calc(1px + 2px) 3px` has two chunks.
fn chunks(values: &[ComponentValue]) -> Vec<&[ComponentValue]> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, v) in values.iter().enumerate() {
        if v.is_whitespace() {
            if let Some(s) = start.take() {
                out.push(&values[s..i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push(&values[s..]);
    }
    out
}

/// Parses one chunk against the grammar of longhand `name`.
fn parse_as(name: &str, chunk: &[ComponentValue]) -> Result<SpecifiedValue, InvalidValue> {
    let id = lookup(name).ok_or(InvalidValue)?;
    parse_property_value(spec(id), chunk)
}

/// The 1–4 value box shorthand: top, right, bottom, left (spec 15.3 pattern).
fn sides(values: &[ComponentValue], names: [&'static str; 4]) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    if parts.is_empty() || parts.len() > 4 {
        return Err(InvalidValue);
    }
    let parsed: Vec<SpecifiedValue> = parts
        .iter()
        .map(|c| parse_as(names[0], c))
        .collect::<Result<_, _>>()?;
    if parsed
        .iter()
        .any(|p| matches!(p, SpecifiedValue::CssWide(_)))
    {
        return Err(InvalidValue);
    }
    let order = match parsed.len() {
        1 => [0, 0, 0, 0],
        2 => [0, 1, 0, 1],
        3 => [0, 1, 2, 1],
        _ => [0, 1, 2, 3],
    };
    Ok(names
        .iter()
        .zip(order)
        .map(|(name, i)| (*name, parsed[i].clone()))
        .collect())
}

/// A component of a border shorthand, classified by trying each grammar.
enum BorderPart {
    Width(SpecifiedValue),
    Style(SpecifiedValue),
    Color(SpecifiedValue),
}

fn classify_border(chunk: &[ComponentValue], side: &str) -> Option<BorderPart> {
    let style_name = format!("border-{side}-style");
    let color_name = format!("border-{side}-color");
    let width_name = format!("border-{side}-width");
    if let Ok(v) = parse_as(&style_name, chunk) {
        if !matches!(v, SpecifiedValue::CssWide(_)) {
            return Some(BorderPart::Style(v));
        }
    }
    if let Ok(v) = parse_as(&color_name, chunk) {
        if !matches!(v, SpecifiedValue::CssWide(_)) {
            return Some(BorderPart::Color(v));
        }
    }
    if let Ok(v) = parse_as(&width_name, chunk) {
        if !matches!(v, SpecifiedValue::CssWide(_)) {
            return Some(BorderPart::Width(v));
        }
    }
    None
}

/// Sets one side from the parts given, resetting missing parts to their initial
/// values (spec 15.3: shorthands reset every longhand they cover).
fn border_parts(
    parts: &[&[ComponentValue]],
    side: &str,
) -> Result<[SpecifiedValue; 3], InvalidValue> {
    let (mut width, mut style, mut color) = (None, None, None);
    for chunk in parts {
        match classify_border(chunk, side).ok_or(InvalidValue)? {
            BorderPart::Width(v) if width.is_none() => width = Some(v),
            BorderPart::Style(v) if style.is_none() => style = Some(v),
            BorderPart::Color(v) if color.is_none() => color = Some(v),
            _ => return Err(InvalidValue),
        }
    }
    Ok([
        width.unwrap_or(SpecifiedValue::Value(Initial::Px(3.0).to_value())),
        style.unwrap_or(SpecifiedValue::Value(Initial::Keyword("none").to_value())),
        color.unwrap_or(SpecifiedValue::Value(Initial::CurrentColor.to_value())),
    ])
}

fn border_side(values: &[ComponentValue], side: &str) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    let [width, style, color] = border_parts(&parts, side)?;
    Ok(vec![
        (side_name("width", side), width),
        (side_name("style", side), style),
        (side_name("color", side), color),
    ])
}

fn border_all(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    let [width, style, color] = border_parts(&parts, "top")?;
    let mut out = Vec::new();
    for side in ["top", "right", "bottom", "left"] {
        out.push((side_name("width", side), width.clone()));
        out.push((side_name("style", side), style.clone()));
        out.push((side_name("color", side), color.clone()));
    }
    Ok(out)
}

/// The longhand name for `border-<side>-<part>`, as a `'static` string.
fn side_name(part: &str, side: &str) -> &'static str {
    match (part, side) {
        ("width", "top") => "border-top-width",
        ("width", "right") => "border-right-width",
        ("width", "bottom") => "border-bottom-width",
        ("width", _) => "border-left-width",
        ("style", "top") => "border-top-style",
        ("style", "right") => "border-right-style",
        ("style", "bottom") => "border-bottom-style",
        ("style", _) => "border-left-style",
        ("color", "top") => "border-top-color",
        ("color", "right") => "border-right-color",
        ("color", "bottom") => "border-bottom-color",
        _ => "border-left-color",
    }
}

/// `background`: M1 covers only a color.
fn background(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    match parts.as_slice() {
        [only] => match parse_as("background-color", only)? {
            SpecifiedValue::CssWide(_) => Err(InvalidValue),
            v => Ok(vec![("background-color", v)]),
        },
        _ => Err(InvalidValue),
    }
}

fn margin_expand(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    sides(v, MARGIN)
}

fn padding_expand(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    sides(v, PADDING)
}

fn border_width_expand(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    sides(v, BORDER_WIDTH)
}

fn border_style_expand(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    sides(v, BORDER_STYLE)
}

fn border_color_expand(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    sides(v, BORDER_COLOR)
}

fn border_top(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    border_side(v, "top")
}

fn border_right(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    border_side(v, "right")
}

fn border_bottom(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    border_side(v, "bottom")
}

fn border_left(v: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    border_side(v, "left")
}
