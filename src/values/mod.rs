// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Typed values (spec 14).
//!
//! M1 covers: absolute and font-relative lengths (px, em, rem), viewport
//! lengths (vw, vh, vmin, vmax), physical absolute lengths (cm, mm, Q, in, pt,
//! pc), percentages, numbers, sRGB colors (including `hsl()`, `hwb()`, and
//! 4- and 8-digit hex), `content` strings, font family lists, and `calc()`
//! with `min()`, `max()`, and `clamp()`.
//!
//! Numbers use `f64` (spec 25.1). Serialization is deterministic but not yet
//! spec-exact (M3).

pub mod calc;
pub mod color;
pub mod length;
pub mod raw;
pub mod serialize;

pub use calc::CalcNode;
pub use color::{Color, ColorSpace};

/// A length unit (spec 14.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    /// `px`
    Px,
    /// `em`: relative to the element's font size (or the parent's, for font-size).
    Em,
    /// `rem`: relative to the root element's font size.
    Rem,
    /// `vw`: 1% of the viewport width.
    Vw,
    /// `vh`: 1% of the viewport height.
    Vh,
    /// `vmin`: 1% of the smaller viewport dimension.
    Vmin,
    /// `vmax`: 1% of the larger viewport dimension.
    Vmax,
    /// `cm`
    Cm,
    /// `mm`
    Mm,
    /// `Q`: quarter millimeter.
    Q,
    /// `in`
    In,
    /// `pt`
    Pt,
    /// `pc`
    Pc,
    /// `cqw`: 1% of the nearest container's width (the viewport if none).
    Cqw,
    /// `cqh`: 1% of the nearest container's height.
    Cqh,
    /// `cqi`: 1% of the container's inline size (its width).
    Cqi,
    /// `cqb`: 1% of the container's block size (its height).
    Cqb,
    /// `cqmin`: the smaller of `cqi` and `cqb`.
    Cqmin,
    /// `cqmax`: the larger of `cqi` and `cqb`.
    Cqmax,
}

/// One item of a `content` list that mixes strings with `attr()` (CSS
/// Generated Content 3, `attr()` with a plain name).
#[derive(Debug, Clone, PartialEq)]
pub enum ContentItem {
    /// A quoted string.
    Text(String),
    /// `attr(name)` or `attr(name, "fallback")`: the value of the element's
    /// attribute `name`, read at computed-value time. A missing attribute
    /// contributes the fallback, or the empty string without one.
    Attr {
        /// Attribute name, lowercased.
        name: String,
        /// Text used when the attribute is missing.
        fallback: Option<String>,
    },
    /// `counter(name, style)`: the innermost value of the counter, formatted in
    /// `style` (a list style name, already validated).
    Counter {
        /// Counter name.
        name: String,
        /// List style name.
        style: String,
    },
    /// `counters(name, separator, style)`: every value of the counter in scope,
    /// outermost first, joined by `separator`.
    Counters {
        /// Counter name.
        name: String,
        /// Text between the values.
        separator: String,
        /// List style name.
        style: String,
    },
}

/// A time unit (CSS Values 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    /// `s`
    S,
    /// `ms`
    Ms,
}

/// An angle unit (CSS Values 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AngleUnit {
    /// `deg`
    Deg,
    /// `grad`: 400 per turn.
    Grad,
    /// `rad`
    Rad,
    /// `turn`: 360 degrees.
    Turn,
}

/// One `box-shadow` layer (CSS Backgrounds 3). The parts are kept in a fixed
/// order, so two equal shadows serialize the same way.
#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    /// Whether the shadow is drawn inside the border box.
    pub inset: bool,
    /// Horizontal offset, a length.
    pub x: Value,
    /// Vertical offset, a length.
    pub y: Value,
    /// Blur radius, a non-negative length. `None` when omitted.
    pub blur: Option<Value>,
    /// Spread distance, a length. `None` when omitted.
    pub spread: Option<Value>,
    /// The color. `None` means `currentcolor`.
    pub color: Option<Value>,
}

/// One function of a `transform` list (CSS Transforms 2).
#[derive(Debug, Clone, PartialEq)]
pub struct TransformFunction {
    /// The function name as the grammar spells it, for example `translateX`.
    pub name: &'static str,
    /// The arguments in order: lengths, percentages, numbers, or angles.
    pub args: Vec<Value>,
}

/// A typed property value, used for both specified and computed values.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A keyword. The string is always one of the keywords listed in the
    /// property's grammar, so it is `'static`.
    Keyword(&'static str),
    /// A length. Relative and physical units are converted to px during
    /// computation.
    Length {
        /// Numeric value.
        value: f64,
        /// Unit.
        unit: LengthUnit,
    },
    /// A percentage. `50.0` means `50%`.
    Percentage(f64),
    /// A unitless number.
    Number(f64),
    /// A color.
    Color(Color),
    /// A font family list. Generic families and named families are both
    /// plain names here.
    FontFamily(Vec<String>),
    /// The string list of `content`.
    Strings(Vec<String>),
    /// A `content` list with at least one `attr()`. Computed values replace
    /// it with [`Value::Strings`].
    Content(Vec<ContentItem>),
    /// An elliptical corner radius, with different horizontal and vertical
    /// radii (`border-radius: 10px / 20px`). Equal radii are stored as one
    /// value, so this variant always has two different parts.
    Elliptical {
        /// Horizontal radius.
        horizontal: Box<Value>,
        /// Vertical radius.
        vertical: Box<Value>,
    },
    /// Counter operations for `counter-reset`, `counter-set`, or
    /// `counter-increment`: each counter name with its integer.
    Counters(Vec<(String, i64)>),
    /// The canonical text of a pass-through value (`grid`). This engine does not
    /// parse its grammar.
    Raw(String),
    /// A `light-dark(light, dark)` color pair, resolved at computed-value time
    /// from the color scheme (CSS Color 5).
    LightDark(Color, Color),
    /// A `calc()`, `min()`, `max()`, or `clamp()` expression. Specified values
    /// keep their units; computed values have lengths in px.
    Calc(Box<CalcNode>),
    /// A time. Computed values are in seconds.
    Time {
        /// Numeric value.
        value: f64,
        /// Unit.
        unit: TimeUnit,
    },
    /// An angle. Computed values are in degrees.
    Angle {
        /// Numeric value.
        value: f64,
        /// Unit.
        unit: AngleUnit,
    },
    /// A comma-separated list of values of one type, such as
    /// `transition-duration: 1s, 2s`.
    List(Vec<Value>),
    /// A `box-shadow` list of one or more shadows.
    Shadows(Vec<Shadow>),
    /// A `transform` list: one or more functions, in order.
    Transforms(Vec<TransformFunction>),
    /// Several keywords of one property, in canonical order (for example
    /// `font-variant-numeric: tabular-nums slashed-zero`).
    Keywords(Vec<&'static str>),
}

impl Value {
    /// A px length.
    pub fn px(value: f64) -> Value {
        Value::Length {
            value,
            unit: LengthUnit::Px,
        }
    }
}
