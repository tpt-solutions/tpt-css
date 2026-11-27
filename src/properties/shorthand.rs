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
//! `background` (one image and one color; no layer lists), `font`,
//! `list-style`, `text-decoration` (line, style, color, thickness, skip-ink),
//! and `border-radius` (including the `/` form) are implemented. `font` has no
//! `font-variant`, no system fonts, and `list-style` has no images other than
//! `none`. A `background` declaration with a layer list is dropped.

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

const FLEX: [&str; 3] = ["flex-grow", "flex-shrink", "flex-basis"];

const BACKGROUND: [&str; 8] = [
    "background-color",
    "background-image",
    "background-position",
    "background-size",
    "background-repeat",
    "background-attachment",
    "background-origin",
    "background-clip",
];

const ANIMATION: [&str; 8] = [
    "animation-name",
    "animation-duration",
    "animation-timing-function",
    "animation-delay",
    "animation-iteration-count",
    "animation-direction",
    "animation-fill-mode",
    "animation-play-state",
];

const TRANSITION: [&str; 4] = [
    "transition-property",
    "transition-duration",
    "transition-timing-function",
    "transition-delay",
];

const RADIUS: [&str; 4] = [
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
];

/// `border-radius: <horizontal>{1,4} [ / <vertical>{1,4} ]?` (CSS Backgrounds 3
/// section 5.1). Without a slash, the vertical radii equal the horizontal ones.
/// Each corner whose two radii differ becomes an elliptical value.
fn radius_expand(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let Some(slash) = values
        .iter()
        .position(|v| v.is_token(&TokenKind::Delim('/')))
    else {
        return sides(values, RADIUS);
    };
    let horizontal = sides(&values[..slash], RADIUS)?;
    let vertical = sides(&values[slash + 1..], RADIUS)?;
    horizontal
        .into_iter()
        .zip(vertical)
        .map(|((name, h), (_, v))| {
            let (SpecifiedValue::Value(h), SpecifiedValue::Value(v)) = (h, v) else {
                return Err(InvalidValue);
            };
            let value = if h == v {
                h
            } else {
                Value::Elliptical {
                    horizontal: Box::new(h),
                    vertical: Box::new(v),
                }
            };
            Ok((name, SpecifiedValue::Value(value)))
        })
        .collect()
}

/// Every shorthand supported in M1. A shorthand's index in this table is the
/// id stored in [`SpecifiedValue::Unparsed`].
pub const SHORTHANDS: &[ShorthandSpec] = &[
    ShorthandSpec {
        name: "flex",
        longhands: &FLEX,
        expand: flex_expand,
    },
    ShorthandSpec {
        name: "transition",
        longhands: &TRANSITION,
        expand: transition_expand,
    },
    ShorthandSpec {
        name: "animation",
        longhands: &ANIMATION,
        expand: animation_expand,
    },
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
        longhands: &BACKGROUND,
        expand: background,
    },
    ShorthandSpec {
        name: "border-radius",
        longhands: &RADIUS,
        expand: radius_expand,
    },
    ShorthandSpec {
        name: "font",
        longhands: &[
            "font-style",
            "font-weight",
            "font-stretch",
            "font-variant-caps",
            "font-variant-ligatures",
            "font-variant-numeric",
            "font-variant-east-asian",
            "font-variant-position",
            "font-variant-emoji",
            "font-variant-alternates",
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
            "text-decoration-thickness",
            "text-decoration-skip-ink",
        ],
        expand: text_decoration,
    },
    ShorthandSpec {
        name: "font-variant",
        longhands: &VARIANT_LONGHANDS,
        expand: font_variant,
    },
];

/// The longhands of `font-variant`, in the order the shorthand lists them.
const VARIANT_LONGHANDS: [&str; 7] = [
    "font-variant-caps",
    "font-variant-ligatures",
    "font-variant-numeric",
    "font-variant-east-asian",
    "font-variant-position",
    "font-variant-emoji",
    "font-variant-alternates",
];

/// `font-variant` (CSS Fonts 4 section 6.1). Each keyword goes to the longhand that
/// accepts it, and the longhands with keywords are parsed from those keywords.
/// `none` sets ligatures to `none`, and the other longhands to `normal`. `normal`
/// sets every longhand to `normal`. Alternates (`font-variant-alternates`, which
/// takes functions) is not supported, so a value that names it is invalid.
fn font_variant(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    if parts.is_empty() {
        return Err(InvalidValue);
    }
    let all_initial = |ligatures: SpecifiedValue| -> Expanded {
        VARIANT_LONGHANDS
            .iter()
            .map(|&name| {
                if name == "font-variant-ligatures" {
                    (name, ligatures.clone())
                } else {
                    (name, initial_value(name))
                }
            })
            .collect()
    };
    if let [part] = parts.as_slice() {
        if chunks_is_none(part) {
            return Ok(all_initial(parse_as("font-variant-ligatures", part)?));
        }
        if let [ComponentValue::Token(t)] = part {
            if matches!(&t.kind, TokenKind::Ident(w) if w.eq_ignore_ascii_case("normal")) {
                return Ok(all_initial(initial_value("font-variant-ligatures")));
            }
        }
    }

    // Sort the keywords by longhand, keeping their order.
    let tokens: Vec<ComponentValue> = values
        .iter()
        .filter(|v| !v.is_whitespace())
        .cloned()
        .collect();
    let mut groups: Vec<Vec<ComponentValue>> = vec![Vec::new(); VARIANT_LONGHANDS.len()];
    for item in &tokens {
        let owner = (0..VARIANT_LONGHANDS.len())
            .find(|&i| {
                parse_property_value_for(VARIANT_LONGHANDS[i], std::slice::from_ref(item)).is_some()
            })
            .ok_or(InvalidValue)?;
        groups[owner].push(item.clone());
    }
    let mut out = Vec::new();
    for (i, name) in VARIANT_LONGHANDS.iter().enumerate() {
        let value = if groups[i].is_empty() {
            initial_value(name)
        } else {
            parse_as(name, &groups[i])?
        };
        out.push((*name, value));
    }
    Ok(out)
}

/// Whether `name` accepts `chunk`, as its value.
fn parse_property_value_for(name: &str, chunk: &[ComponentValue]) -> Option<SpecifiedValue> {
    parse_as(name, chunk).ok()
}

/// `font: [style] [weight] [stretch] size [/ line-height] family` (CSS Fonts 4
/// section 4.3).
///
/// `font-variant` is not registered, so it is not accepted here, and the system
/// font keywords (`caption`, `menu`, and the others) are not supported. Omitted
/// style, weight, and stretch take their initial values, and an omitted line
/// height resets to `normal`, as the shorthand requires.
fn font(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let values = crate::syntax::trim_whitespace(values);
    let items: Vec<(usize, &ComponentValue)> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| !v.is_whitespace())
        .collect();
    let mut style = None;
    let mut weight = None;
    let mut stretch = None;
    let mut variant = None;
    let mut k = 0;
    // Style, weight, stretch, and small-caps come before the size, in any order.
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
        if stretch.is_none() {
            if let Ok(v) = parse_as("font-stretch", std::slice::from_ref(item)) {
                if !matches!(v, SpecifiedValue::CssWide(_)) {
                    stretch = Some(v);
                    k += 1;
                    continue;
                }
            }
        }
        // Only `small-caps` is allowed here (CSS Fonts 4 `font-variant-css2`).
        if variant.is_none() {
            if let Ok(v) = parse_as("font-variant-caps", std::slice::from_ref(item)) {
                if matches!(v, SpecifiedValue::Value(Value::Keyword("small-caps"))) {
                    variant = Some(v);
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
        (
            "font-stretch",
            stretch.unwrap_or(SpecifiedValue::Value(Initial::Keyword("normal").to_value())),
        ),
        (
            "font-variant-caps",
            variant.unwrap_or(SpecifiedValue::Value(Initial::Keyword("normal").to_value())),
        ),
        (
            "font-variant-ligatures",
            initial_value("font-variant-ligatures"),
        ),
        (
            "font-variant-numeric",
            initial_value("font-variant-numeric"),
        ),
        (
            "font-variant-east-asian",
            initial_value("font-variant-east-asian"),
        ),
        (
            "font-variant-position",
            initial_value("font-variant-position"),
        ),
        ("font-variant-emoji", initial_value("font-variant-emoji")),
        (
            "font-variant-alternates",
            initial_value("font-variant-alternates"),
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
    let mut thickness = None;
    let mut skip_ink = None;
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
        // `none` is the line value and `auto` is the thickness, so the
        // shorthand can only name skip-ink as `all`.
        if let [ComponentValue::Token(t)] = part {
            if matches!(&t.kind, TokenKind::Ident(w) if w.eq_ignore_ascii_case("all")) {
                if skip_ink.is_some() {
                    return Err(InvalidValue);
                }
                skip_ink = Some(SpecifiedValue::Value(Value::Keyword("all")));
                continue;
            }
        }
        if thickness.is_none() {
            if let Ok(v) = parse_as("text-decoration-thickness", part) {
                thickness = Some(v);
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
        (
            "text-decoration-thickness",
            thickness.unwrap_or_else(|| initial_value("text-decoration-thickness")),
        ),
        (
            "text-decoration-skip-ink",
            skip_ink.unwrap_or_else(|| initial_value("text-decoration-skip-ink")),
        ),
    ])
}

/// `flex: none | [ <grow> <shrink>? || <basis> ]` (CSS Flexbox 1 section 7.2).
///
/// A unitless number is a flex factor while fewer than two are present. After
/// that, a unitless zero is a basis, as the spec requires. An omitted shrink is
/// 1. Without factors, grow and shrink are 1 and the basis is given.
///
/// With factors and no basis, the basis is `0%`.
fn flex_expand(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let parts = chunks(values);
    if let [part] = parts.as_slice() {
        if chunks_is_none(part) {
            return Ok(vec![
                ("flex-grow", SpecifiedValue::Value(Value::Number(0.0))),
                ("flex-shrink", SpecifiedValue::Value(Value::Number(0.0))),
                ("flex-basis", initial_value("flex-basis")),
            ]);
        }
    }
    if parts.is_empty() {
        return Err(InvalidValue);
    }
    let mut factors: Vec<f64> = Vec::new();
    let mut basis: Option<SpecifiedValue> = None;
    for part in parts {
        if factors.len() < 2 {
            if let Ok(SpecifiedValue::Value(Value::Number(n))) = parse_as("flex-grow", part) {
                factors.push(n);
                continue;
            }
        }
        if basis.is_some() {
            return Err(InvalidValue);
        }
        basis = Some(parse_as("flex-basis", part)?);
    }
    let grow = factors.first().copied().unwrap_or(1.0);
    let shrink = factors.get(1).copied().unwrap_or(1.0);
    let basis = basis.unwrap_or(SpecifiedValue::Value(Value::Percentage(0.0)));
    Ok(vec![
        ("flex-grow", SpecifiedValue::Value(Value::Number(grow))),
        ("flex-shrink", SpecifiedValue::Value(Value::Number(shrink))),
        ("flex-basis", basis),
    ])
}

/// `transition: [ <property> || <duration> || <timing-function> || <delay> ]#`
/// (CSS Transitions 1 section 2).
///
/// Each comma-separated layer sets one entry in each of the four longhand lists.
/// The first time is the duration and the second is the delay. An omitted
/// property is `all`, an omitted timing function is `ease`, and an omitted
/// duration or delay is `0s`.
fn transition_expand(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let zero = || Value::Time {
        value: 0.0,
        unit: crate::values::TimeUnit::S,
    };
    let (mut properties, mut durations, mut timings, mut delays) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for layer in values.split(|v| v.is_token(&TokenKind::Comma)) {
        let layer = crate::syntax::trim_whitespace(layer);
        let parts = chunks(layer);
        if parts.is_empty() {
            return Err(InvalidValue);
        }
        let (mut property, mut duration, mut timing, mut delay) = (None, None, None, None);
        for part in parts {
            if duration.is_none() {
                if let Some(time) = single_item(parse_as("transition-duration", part)) {
                    duration = Some(time);
                    continue;
                }
            } else if delay.is_none() {
                if let Some(time) = single_item(parse_as("transition-delay", part)) {
                    delay = Some(time);
                    continue;
                }
            }
            if timing.is_none() {
                if let Some(easing) = single_item(parse_as("transition-timing-function", part)) {
                    timing = Some(easing);
                    continue;
                }
            }
            // An easing keyword is not a property name, even a second time.
            if property.is_none() && parse_as("transition-timing-function", part).is_err() {
                if let Some(name) = single_item(parse_as("transition-property", part)) {
                    property = Some(name);
                    continue;
                }
            }
            return Err(InvalidValue);
        }
        properties.push(property.unwrap_or(Value::Keyword("all")));
        durations.push(duration.unwrap_or_else(zero));
        timings.push(timing.unwrap_or(Value::Keyword("ease")));
        delays.push(delay.unwrap_or_else(zero));
    }
    Ok(vec![
        (
            "transition-property",
            SpecifiedValue::Value(Value::List(properties)),
        ),
        (
            "transition-duration",
            SpecifiedValue::Value(Value::List(durations)),
        ),
        (
            "transition-timing-function",
            SpecifiedValue::Value(Value::List(timings)),
        ),
        (
            "transition-delay",
            SpecifiedValue::Value(Value::List(delays)),
        ),
    ])
}

/// `animation: [ <name> || <duration> || <timing-function> || <delay> ||
/// <iteration-count> || <direction> || <fill-mode> || <play-state> ]#`
/// (CSS Animations 1 section 3).
///
/// Each comma-separated layer sets one entry in each of the eight longhand lists.
/// The first time is the duration and the second is the delay. `none` is the
/// name when no name has been given, and otherwise the fill mode. A name cannot
/// be a keyword of another longhand. The defaults are the initial values.
fn animation_expand(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let mut columns: Vec<Vec<Value>> = vec![Vec::new(); ANIMATION.len()];
    for layer in values.split(|v| v.is_token(&TokenKind::Comma)) {
        let parts = chunks(crate::syntax::trim_whitespace(layer));
        if parts.is_empty() {
            return Err(InvalidValue);
        }
        let mut current = AnimationLayer::default();
        for part in parts {
            if !place_animation_part(&mut current, part) {
                return Err(InvalidValue);
            }
        }
        for (column, value) in columns.iter_mut().zip(current.finish()) {
            column.push(value);
        }
    }
    Ok(ANIMATION
        .iter()
        .zip(columns)
        .map(|(name, items)| (*name, SpecifiedValue::Value(Value::List(items))))
        .collect())
}

/// The values of one `animation` layer. Each slot is filled once.
#[derive(Default)]
struct AnimationLayer {
    name: Option<Value>,
    duration: Option<Value>,
    timing: Option<Value>,
    delay: Option<Value>,
    iteration: Option<Value>,
    direction: Option<Value>,
    fill: Option<Value>,
    play: Option<Value>,
}

impl AnimationLayer {
    /// The eight values in the order of [`ANIMATION`], with the initial values for
    /// the parts that were omitted.
    fn finish(self) -> [Value; 8] {
        let zero = || Value::Time {
            value: 0.0,
            unit: crate::values::TimeUnit::S,
        };
        [
            self.name.unwrap_or(Value::Keyword("none")),
            self.duration.unwrap_or_else(zero),
            self.timing.unwrap_or(Value::Keyword("ease")),
            self.delay.unwrap_or_else(zero),
            self.iteration.unwrap_or(Value::Number(1.0)),
            self.direction.unwrap_or(Value::Keyword("normal")),
            self.fill.unwrap_or(Value::Keyword("none")),
            self.play.unwrap_or(Value::Keyword("running")),
        ]
    }
}

/// Places one chunk of an `animation` layer in the first free slot that accepts
/// it. Returns `false` when no slot takes the chunk.
fn place_animation_part(layer: &mut AnimationLayer, part: &[ComponentValue]) -> bool {
    let none_name = match single_item(parse_as("animation-fill-mode", part)) {
        Some(none @ Value::Keyword("none")) if layer.name.is_none() => Some(none),
        _ => None,
    };
    take(
        &mut layer.duration,
        single_item(parse_as("animation-duration", part)),
    ) || take(
        &mut layer.delay,
        single_item(parse_as("animation-delay", part)),
    ) || take(
        &mut layer.timing,
        single_item(parse_as("animation-timing-function", part)),
    ) || take(
        &mut layer.iteration,
        single_item(parse_as("animation-iteration-count", part)),
    ) || take(
        &mut layer.direction,
        single_item(parse_as("animation-direction", part)),
    ) || take(
        &mut layer.play,
        single_item(parse_as("animation-play-state", part)),
    ) || take(&mut layer.name, none_name)
        || take(
            &mut layer.fill,
            single_item(parse_as("animation-fill-mode", part)),
        )
        || (!is_other_animation_keyword(part)
            && take(
                &mut layer.name,
                single_item(parse_as("animation-name", part)),
            ))
}

/// Whether a chunk is a keyword of another `animation` longhand, which a name
/// cannot be.
fn is_other_animation_keyword(part: &[ComponentValue]) -> bool {
    [
        "animation-timing-function",
        "animation-iteration-count",
        "animation-direction",
        "animation-fill-mode",
        "animation-play-state",
    ]
    .iter()
    .any(|name| parse_as(name, part).is_ok())
}

/// Stores `value` in an empty slot. Returns `false` when there is no value or the
/// slot is taken.
fn take(slot: &mut Option<Value>, value: Option<Value>) -> bool {
    match value {
        Some(value) if slot.is_none() => {
            *slot = Some(value);
            true
        }
        _ => false,
    }
}

/// The one item of a list-valued longhand parsed from a single chunk. The
/// longhands are lists, so a single chunk gives a list of one.
fn single_item(result: Result<SpecifiedValue, InvalidValue>) -> Option<Value> {
    match result {
        Ok(SpecifiedValue::Value(Value::List(mut items))) if items.len() == 1 => items.pop(),
        _ => None,
    }
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

/// `background` (CSS Backgrounds 3, one layer): one image and one color, in any
/// order. Each is optional. An omitted image is `none` and an omitted color is
/// `transparent`, because the shorthand resets every longhand it covers. Layer
/// lists (commas between images) are not supported.
/// `background`: one or more comma-separated layers (CSS Backgrounds 3 section
/// 2.1). A layer takes an image, a position with an optional `/ size`, a repeat
/// style, an attachment, one or two boxes, and, in the last layer only, a color.
/// Omitted parts take their initial values.
///
/// Each part is checked by the grammar of its own longhand. The images form one
/// list. A single image is its text, and `none` alone is the keyword.
fn background(values: &[ComponentValue]) -> Result<Expanded, InvalidValue> {
    let layers: Vec<&[ComponentValue]> = values.split(|v| v.is_token(&TokenKind::Comma)).collect();
    let count = layers.len();
    let mut parsed = Vec::with_capacity(count);
    for (i, layer) in layers.into_iter().enumerate() {
        parsed.push(background_layer(layer, i + 1 == count)?);
    }

    let images: Vec<&Value> = parsed.iter().map(|l| &l.image).collect();
    let image = if images.iter().all(|v| matches!(v, Value::Keyword("none"))) {
        Value::Keyword("none")
    } else {
        Value::Raw(
            images
                .iter()
                .map(|v| match v {
                    Value::Raw(text) => text.clone(),
                    _ => "none".to_string(),
                })
                .collect::<Vec<_>>()
                .join(", "),
        )
    };
    let color = parsed
        .last()
        .and_then(|l| l.color.clone())
        .unwrap_or_else(|| Initial::Transparent.to_value());
    let per_layer = |pick: fn(&BackgroundLayer) -> &Value| {
        SpecifiedValue::Value(Value::List(
            parsed.iter().map(|l| pick(l).clone()).collect(),
        ))
    };
    Ok(vec![
        ("background-color", SpecifiedValue::Value(color)),
        ("background-image", SpecifiedValue::Value(image)),
        ("background-position", per_layer(|l| &l.position)),
        ("background-size", per_layer(|l| &l.size)),
        ("background-repeat", per_layer(|l| &l.repeat)),
        ("background-attachment", per_layer(|l| &l.attachment)),
        ("background-origin", per_layer(|l| &l.origin)),
        ("background-clip", per_layer(|l| &l.clip)),
    ])
}

/// The values of one `background` layer, one for each longhand.
struct BackgroundLayer {
    image: Value,
    position: Value,
    size: Value,
    repeat: Value,
    attachment: Value,
    origin: Value,
    clip: Value,
    color: Option<Value>,
}

/// Parses one layer of `background`. Words are split at spaces and at `/`.
fn background_layer(
    layer: &[ComponentValue],
    is_last: bool,
) -> Result<BackgroundLayer, InvalidValue> {
    let words = layer_words(layer);
    if words.is_empty() {
        return Err(InvalidValue);
    }
    let mut image = None;
    let mut color = None;
    let mut position: Vec<&[ComponentValue]> = Vec::new();
    let mut position_done = false;
    let mut size: Vec<&[ComponentValue]> = Vec::new();
    let mut in_size = false;
    let mut slash_seen = false;
    let mut repeat: Vec<&[ComponentValue]> = Vec::new();
    let mut attachment = None;
    let mut boxes: Vec<&[ComponentValue]> = Vec::new();

    for word in words {
        // A size is one or two size words after `/`. The first word that is not a
        // size ends it, and the rest of the layer follows.
        if in_size {
            if size.len() < 2
                && !is_slash_word(word)
                && single_value(parse_as("background-size", word)).is_some()
            {
                size.push(word);
                continue;
            }
            in_size = false;
        }
        if is_slash_word(word) {
            // One size, after the position and before the rest of the layer.
            if position.is_empty() || slash_seen {
                return Err(InvalidValue);
            }
            slash_seen = true;
            in_size = true;
            continue;
        }
        if single_value(parse_as("background-position", word)).is_some() {
            if position_done || slash_seen {
                return Err(InvalidValue);
            }
            position.push(word);
            continue;
        }
        if !position.is_empty() {
            position_done = true;
        }
        if image.is_none() {
            if let Ok(SpecifiedValue::Value(value)) = parse_as("background-image", word) {
                image = Some(value);
                continue;
            }
        }
        if parse_as("background-repeat", word).is_ok() {
            if repeat.len() == 2 {
                return Err(InvalidValue);
            }
            repeat.push(word);
            continue;
        }
        if attachment.is_none() {
            if let Some(value) = single_value(parse_as("background-attachment", word)) {
                attachment = Some(value);
                continue;
            }
        }
        if parse_as("background-origin", word).is_ok() {
            if boxes.len() == 2 {
                return Err(InvalidValue);
            }
            boxes.push(word);
            continue;
        }
        if color.is_none() && is_last {
            if let Ok(SpecifiedValue::Value(value)) = parse_as("background-color", word) {
                color = Some(value);
                continue;
            }
        }
        return Err(InvalidValue);
    }
    if slash_seen && size.is_empty() {
        return Err(InvalidValue);
    }

    let longhand = |name: &str, words: &[&[ComponentValue]]| -> Result<Value, InvalidValue> {
        single_value(parse_as(name, &joined(words)?)).ok_or(InvalidValue)
    };
    let position = if position.is_empty() {
        Value::Raw("0% 0%".to_string())
    } else {
        longhand("background-position", &position)?
    };
    let size = if size.is_empty() {
        Value::Keyword("auto")
    } else {
        longhand("background-size", &size)?
    };
    let repeat = if repeat.is_empty() {
        Value::Keyword("repeat")
    } else {
        longhand("background-repeat", &repeat)?
    };
    let attachment = attachment.unwrap_or(Value::Keyword("scroll"));
    let (origin, clip) = match boxes.as_slice() {
        [] => (Value::Keyword("padding-box"), Value::Keyword("border-box")),
        [one] => {
            let one = longhand("background-origin", &[*one])?;
            (one.clone(), one)
        }
        [origin, clip] => (
            longhand("background-origin", &[*origin])?,
            longhand("background-origin", &[*clip])?,
        ),
        _ => return Err(InvalidValue),
    };
    Ok(BackgroundLayer {
        image: image.unwrap_or(Value::Keyword("none")),
        position,
        size,
        repeat,
        attachment,
        origin,
        clip,
        color,
    })
}

/// The one value of a single-item longhand, or `None`.
fn single_value(result: Result<SpecifiedValue, InvalidValue>) -> Option<Value> {
    match result {
        Ok(SpecifiedValue::Value(Value::List(mut items))) if items.len() == 1 => items.pop(),
        Ok(SpecifiedValue::Value(value)) => Some(value),
        _ => None,
    }
}

fn is_slash(v: &ComponentValue) -> bool {
    v.is_token(&TokenKind::Delim('/'))
}

/// Whether a word is the single `/` of a layer.
fn is_slash_word(word: &[ComponentValue]) -> bool {
    matches!(word, [v] if is_slash(v))
}

/// The words of a layer: runs of components between spaces and `/`, with each
/// `/` as a word of its own.
fn layer_words(layer: &[ComponentValue]) -> Vec<&[ComponentValue]> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, v) in layer.iter().enumerate() {
        if v.is_whitespace() || is_slash(v) {
            if let Some(s) = start.take() {
                out.push(&layer[s..i]);
            }
            if is_slash(v) {
                out.push(&layer[i..=i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push(&layer[s..]);
    }
    out
}

/// The components of several words, joined by one space, so that a longhand can
/// read them as one value. The text is tokenized again to get the components.
fn joined(words: &[&[ComponentValue]]) -> Result<Vec<ComponentValue>, InvalidValue> {
    let text = words
        .iter()
        .map(|w| crate::values::raw::serialize_components(w))
        .collect::<Vec<_>>()
        .join(" ");
    let tokenized = crate::syntax::tokenizer::tokenize(
        crate::syntax::SourceId(0),
        &text,
        &crate::limits::ParseLimits::default(),
    );
    crate::syntax::component::build_component_values(&tokenized.tokens, 16)
        .map_err(|_| InvalidValue)
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
