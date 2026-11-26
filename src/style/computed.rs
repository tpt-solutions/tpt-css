// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Computed styles and the style map (spec 8.3, 14, 16.1).
//!
//! Storage is property-id based: one `Value` per registry row. Typed accessors
//! are conveniences over that storage.

use std::collections::BTreeMap;

use crate::properties::registry::{ids, Grammar, PropertyId, PROPERTIES};
use crate::tree::NodeId;
use crate::values::{Color, LengthUnit, Value};

/// Computed values of one element, indexed by property id.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    values: Vec<Value>,
}

impl ComputedStyle {
    /// Builds a style from one value per property, in registry order.
    pub(crate) fn from_values(values: Vec<Value>) -> ComputedStyle {
        debug_assert_eq!(values.len(), PROPERTIES.len());
        ComputedStyle { values }
    }

    /// The computed value of a property.
    pub fn raw(&self, id: PropertyId) -> &Value {
        &self.values[id.index()]
    }

    /// The `display` keyword.
    pub fn display(&self) -> &'static str {
        match self.raw(ids::DISPLAY) {
            Value::Keyword(k) => k,
            _ => "inline",
        }
    }

    /// The computed `color`.
    pub fn color(&self) -> Color {
        match self.raw(ids::COLOR) {
            Value::Color(c) => *c,
            _ => Color::BLACK,
        }
    }

    /// The computed `font-size`, in px.
    pub fn font_size(&self) -> f64 {
        match self.raw(ids::FONT_SIZE) {
            Value::Length {
                value,
                unit: LengthUnit::Px,
            } => *value,
            _ => 16.0,
        }
    }

    /// The computed `font-weight` as a number (400 is normal).
    pub fn font_weight(&self) -> f64 {
        match self.raw(ids::FONT_WEIGHT) {
            Value::Number(n) => *n,
            _ => 400.0,
        }
    }
}

/// Context needed to compute one property (spec 16.1, step 10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ComputeContext {
    /// Font size of the parent, in px. Base for `em` in `font-size`.
    pub parent_font_size: f64,
    /// Font size of the root element, in px. Base for `rem`.
    pub root_font_size: f64,
    /// This element's computed font size, in px. Base for `em` elsewhere.
    pub font_size: f64,
    /// Parent's computed color. Used for `currentcolor` in `color`.
    pub parent_color: Color,
    /// This element's computed color. Used for `currentcolor` elsewhere.
    pub color: Color,
    /// Parent's computed font weight. Base for `bolder` and `lighter`.
    pub parent_weight: f64,
}

/// Computes one value (spec 16.1 step 10, CSS Values 4 section 2 for the
/// absolute-length rules).
///
/// - `em` and `rem` become px. `font-size` percentages and `em` use the parent
///   size, other properties use this element's font size.
/// - Other percentages are kept for layout to resolve (spec 4.4).
/// - `currentcolor` becomes a color. In `color` itself it means the inherited color.
/// - Font weight keywords become numbers.
pub(crate) fn compute_value(
    id: PropertyId,
    grammar: Grammar,
    value: Value,
    ctx: &ComputeContext,
) -> Value {
    let is_font_size = id == ids::FONT_SIZE;
    match value {
        Value::Length { value, unit } => {
            let px = match unit {
                LengthUnit::Px => value,
                LengthUnit::Em => {
                    let base = if is_font_size {
                        ctx.parent_font_size
                    } else {
                        ctx.font_size
                    };
                    value * base
                }
                LengthUnit::Rem => value * ctx.root_font_size,
            };
            Value::px(finite(px))
        }
        Value::Percentage(p) if is_font_size => Value::px(finite(ctx.parent_font_size * p / 100.0)),
        Value::Keyword("currentcolor") if matches!(grammar, Grammar::Color) => {
            let c = if id == ids::COLOR {
                ctx.parent_color
            } else {
                ctx.color
            };
            Value::Color(c)
        }
        Value::Keyword(k) if id == ids::FONT_WEIGHT => Value::Number(match k {
            "normal" => 400.0,
            "bold" => 700.0,
            "bolder" => bolder(ctx.parent_weight),
            "lighter" => lighter(ctx.parent_weight),
            _ => ctx.parent_weight,
        }),
        other => other,
    }
}

/// Keeps computed absolute lengths finite. Inheritance can compound `em`
/// across a deep tree (`2em` at every level overflows to infinity), and an
/// infinite length would leak into layout. NaN becomes 0 and infinities clamp
/// to the largest finite value. This is a deliberate deviation for hostile
/// input (spec 19.1, 20.3).
fn finite(x: f64) -> f64 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(f64::MIN, f64::MAX)
    }
}

/// `bolder` relative to the parent weight (CSS Fonts 4 section 2.2).
/// The thresholds follow the spec table; see SPEC_TRACE.md for the verification status.
fn bolder(parent: f64) -> f64 {
    if parent < 350.0 {
        400.0
    } else if parent < 550.0 {
        700.0
    } else {
        900.0
    }
}

/// `lighter` relative to the parent weight (CSS Fonts 4 section 2.2).
fn lighter(parent: f64) -> f64 {
    if parent < 550.0 {
        100.0
    } else if parent < 750.0 {
        400.0
    } else {
        700.0
    }
}

/// Computed styles for every element in a resolved tree.
///
/// Backed by a `BTreeMap` so iteration order depends only on node ids (spec 21.3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleMap {
    styles: BTreeMap<NodeId, ComputedStyle>,
}

impl StyleMap {
    /// The computed style of an element, if it was resolved.
    pub fn get(&self, node: NodeId) -> Option<&ComputedStyle> {
        self.styles.get(&node)
    }

    /// Number of styled elements.
    pub fn len(&self) -> usize {
        self.styles.len()
    }

    /// True if no element was styled.
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    pub(crate) fn insert(&mut self, node: NodeId, style: ComputedStyle) {
        self.styles.insert(node, style);
    }
}
