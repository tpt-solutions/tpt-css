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
    /// A `calc()`, `min()`, `max()`, or `clamp()` expression. Specified values
    /// keep their units; computed values have lengths in px.
    Calc(Box<CalcNode>),
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
