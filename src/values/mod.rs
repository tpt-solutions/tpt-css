// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Typed values (spec 14). M0 covers lengths in px, em, and rem, percentages,
//! numbers, sRGB colors, keywords, and font family lists.
//!
//! Numbers use `f64` (spec 25.1, decided for M0; see DESIGN.md). Serialization
//! is deterministic but is not yet spec-exact (M3).

pub mod color;
pub mod length;
pub mod serialize;

pub use color::{Color, ColorSpace};

/// A length unit supported in M0 (spec 14.3, Tier 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    /// `px`
    Px,
    /// `em`: relative to the element's font size (or the parent's, for font-size).
    Em,
    /// `rem`: relative to the root element's font size.
    Rem,
}

/// A typed property value, used for both specified and computed values.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A keyword. The string is always one of the keywords listed in the
    /// property's grammar, so it is `'static`.
    Keyword(&'static str),
    /// A length. Relative units are converted to px during computation.
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
