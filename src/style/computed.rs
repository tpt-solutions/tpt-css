// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Computed styles and the style map (spec 8.3, 14, 16.1).
//!
//! Storage is property-id based: one `Value` per registry row, plus the
//! element's resolved custom properties. Typed accessors are conveniences over
//! that storage.

use std::collections::BTreeMap;

use crate::properties::registry::{ids, Grammar, PropertyId, PROPERTIES};
use crate::selectors::PseudoElement;
use crate::style::custom::CustomPropertyValue;
use crate::tree::NodeId;
use crate::values::calc::{self, CalcNode};
use crate::values::{Color, LengthUnit, Value};

/// Computed values of one element.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    values: Vec<Value>,
    custom: BTreeMap<String, CustomPropertyValue>,
}

impl ComputedStyle {
    /// Builds a style from one value per property, in registry order, and the
    /// resolved custom properties.
    pub(crate) fn from_values(
        values: Vec<Value>,
        custom: BTreeMap<String, CustomPropertyValue>,
    ) -> ComputedStyle {
        debug_assert_eq!(values.len(), PROPERTIES.len());
        ComputedStyle { values, custom }
    }

    /// The computed value of a property.
    pub fn raw(&self, id: PropertyId) -> &Value {
        &self.values[id.index()]
    }

    /// The resolved custom property `name` (for example `--gap`), if it is
    /// defined and valid.
    pub fn custom_property(&self, name: &str) -> Option<&CustomPropertyValue> {
        self.custom.get(name)
    }

    /// All resolved custom properties, in name order.
    pub fn custom_properties(&self) -> impl Iterator<Item = (&str, &CustomPropertyValue)> {
        self.custom.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// The resolved custom properties, for inheritance by children.
    pub(crate) fn customs(&self) -> &BTreeMap<String, CustomPropertyValue> {
        &self.custom
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
    /// Font size of the parent, in px. Base for `em` and `%` in `font-size`.
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
    /// Viewport width in px, for `vw`, `vmin`, and `vmax`.
    pub viewport_width: f64,
    /// Viewport height in px, for `vh`, `vmin`, and `vmax`.
    pub viewport_height: f64,
}

impl ComputeContext {
    /// Converts a length to px in this context.
    fn to_px(self, value: f64, unit: LengthUnit) -> f64 {
        let (w, h) = (self.viewport_width, self.viewport_height);
        match unit {
            LengthUnit::Px => value,
            LengthUnit::Em => value * self.font_size,
            LengthUnit::Rem => value * self.root_font_size,
            LengthUnit::Vw => value * w / 100.0,
            LengthUnit::Vh => value * h / 100.0,
            LengthUnit::Vmin => value * w.min(h) / 100.0,
            LengthUnit::Vmax => value * w.max(h) / 100.0,
            // 96 CSS px per inch (CSS Values 3, absolute lengths).
            LengthUnit::In => value * 96.0,
            LengthUnit::Cm => value * 96.0 / 2.54,
            LengthUnit::Mm => value * 96.0 / 25.4,
            LengthUnit::Q => value * 96.0 / 101.6,
            LengthUnit::Pt => value * 96.0 / 72.0,
            LengthUnit::Pc => value * 16.0,
        }
    }
}

/// Whether a property's grammar rejects negative values.
fn non_negative(grammar: Grammar) -> bool {
    match grammar {
        Grammar::LengthPercentage { non_negative, .. } => non_negative,
        Grammar::BorderWidth | Grammar::LineHeight => true,
        _ => false,
    }
}

/// Computes one value (spec 16.1 step 10, CSS Values 4 for the absolute-length
/// rules).
///
/// - Lengths and math functions become px. `font-size` percentages and `em`
///   use the parent size, other properties use this element's font size.
/// - Other percentages are kept for layout to resolve (spec 4.4). Math
///   functions that mix px and percent stay as trees for the same reason.
/// - `currentcolor` becomes a color. In `color` itself it means the inherited color.
/// - Font weight keywords become numbers.
pub(crate) fn compute_value(
    id: PropertyId,
    grammar: Grammar,
    value: Value,
    ctx: &ComputeContext,
) -> Value {
    let is_font_size = id == ids::FONT_SIZE;
    let clamp_px = |px: f64| {
        if non_negative(grammar) && px < 0.0 {
            0.0
        } else {
            finite(px)
        }
    };
    match value {
        Value::Length { value, unit } => Value::px(clamp_px(ctx.to_px(value, unit))),
        Value::Percentage(p) if is_font_size => {
            Value::px(clamp_px(ctx.parent_font_size * p / 100.0))
        }
        Value::Calc(node) => compute_calc(&node, ctx, is_font_size, non_negative(grammar)),
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

/// Simplifies a math expression at computed-value time (spec 14.4).
fn compute_calc(node: &CalcNode, ctx: &ComputeContext, is_font_size: bool, non_neg: bool) -> Value {
    let to_px = |v: f64, u: LengthUnit| ctx.to_px(v, u);
    let percent_base = is_font_size.then_some(ctx.parent_font_size);
    match calc::simplify(node, &to_px, percent_base) {
        CalcNode::Linear { px, percent } => {
            // The non-negative clamp applies to the final value. A percentage is
            // resolved later by layout, so the clamp waits for that step.
            let px = if non_neg && percent == 0.0 && px < 0.0 {
                0.0
            } else {
                finite(px)
            };
            match (px == 0.0, percent == 0.0) {
                (_, true) => Value::px(px),
                (true, false) => Value::Percentage(percent),
                (false, false) => Value::Calc(Box::new(CalcNode::Linear { px, percent })),
            }
        }
        // The parser only accepts length-typed expressions, so a bare number
        // cannot reach here. Treat it as an invalid zero length if it does.
        CalcNode::Number(_) => Value::px(0.0),
        tree => Value::Calc(Box::new(tree)),
    }
}

/// Keeps computed absolute lengths finite. Inheritance can compound `em` across
/// a deep tree (`2em` at every level overflows to infinity), and an infinite
/// length would leak into layout. NaN becomes 0 and infinities clamp to
/// `f64::MAX`. This is a deliberate deviation for hostile input (spec 19.1,
/// 20.3).
fn finite(x: f64) -> f64 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(f64::MIN, f64::MAX)
    }
}

/// `bolder` relative to the parent weight (CSS Fonts 4, relative font weights).
/// Thresholds were written from CSS Fonts 4 and are not yet verified against
/// the text (see SPEC_TRACE.md).
fn bolder(parent: f64) -> f64 {
    if parent < 350.0 {
        400.0
    } else if parent < 550.0 {
        700.0
    } else {
        900.0
    }
}

/// `lighter` relative to the parent weight (CSS Fonts 4, relative font weights).
fn lighter(parent: f64) -> f64 {
    if parent < 550.0 {
        100.0
    } else if parent < 750.0 {
        400.0
    } else {
        700.0
    }
}

/// Computed styles for every element in a resolved tree, plus the styles of
/// generated pseudo-elements.
///
/// Backed by `BTreeMap`s so iteration order depends only on node ids and
/// pseudo-element names (spec 21.3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleMap {
    styles: BTreeMap<NodeId, ComputedStyle>,
    pseudo: BTreeMap<(NodeId, PseudoElement), ComputedStyle>,
}

impl StyleMap {
    /// The computed style of an element, if it was resolved.
    pub fn get(&self, node: NodeId) -> Option<&ComputedStyle> {
        self.styles.get(&node)
    }

    /// The computed style of a generated pseudo-element of `node`. Present only
    /// when the pseudo-element is generated, that is, when its `content` is not
    /// `none` or `normal`.
    pub fn get_pseudo(&self, node: NodeId, pseudo: &PseudoElement) -> Option<&ComputedStyle> {
        self.pseudo.get(&(node, pseudo.clone()))
    }

    /// The resolved custom property `name` on `node`, if defined and valid.
    pub fn custom_property(&self, node: NodeId, name: &str) -> Option<&CustomPropertyValue> {
        self.get(node)?.custom_property(name)
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

    pub(crate) fn insert_pseudo(
        &mut self,
        node: NodeId,
        pseudo: PseudoElement,
        style: ComputedStyle,
    ) {
        self.pseudo.insert((node, pseudo), style);
    }

    /// Removes the pseudo-element styles of `node`, before it is restyled.
    pub(crate) fn clear_pseudo(&mut self, node: NodeId) {
        self.pseudo.retain(|(n, _), _| *n != node);
    }
}
