// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Math functions: `calc()`, `min()`, `max()`, `clamp()` (spec 14.4).
//!
//! Parsing builds a [`CalcNode`] tree with units kept. Every node has a kind:
//! a plain `Number`, or a `Dimension` (a length or percentage). Operators are
//! type-checked when the tree is built, so an expression such as `10px + 2`
//! is invalid and the declaration is dropped.
//!
//! Simplification runs at computed-value time (`simplify`). Lengths become px,
//! and linear forms collapse to `Linear { px, percent }`. Percentages stay
//! symbolic for layout, except in `font-size`, where the parent's size is
//! known. Expressions that mix px and percent inside `min()`, `max()`, or
//! `clamp()` stay as trees for layout to resolve (spec 4.4).

use super::serialize::format_number;
use super::LengthUnit;
use crate::syntax::{ComponentValue, TokenKind};

/// Maximum nesting of math functions and parentheses accepted by the parser.
/// Not yet configurable through `ParseLimits`.
pub const DEFAULT_CALC_DEPTH: usize = 32;

/// A node of a math expression tree.
#[derive(Debug, Clone, PartialEq)]
pub enum CalcNode {
    /// A unitless number.
    Number(f64),
    /// A length with its unit.
    Length {
        /// Numeric value.
        value: f64,
        /// Unit.
        unit: LengthUnit,
    },
    /// A percentage. `50.0` means `50%`.
    Percent(f64),
    /// A simplified linear combination of px and percent (computed form only).
    Linear {
        /// Absolute part, in px.
        px: f64,
        /// Percentage part.
        percent: f64,
    },
    /// `a + b`
    Add(Box<CalcNode>, Box<CalcNode>),
    /// `a - b`
    Sub(Box<CalcNode>, Box<CalcNode>),
    /// `a * b`
    Mul(Box<CalcNode>, Box<CalcNode>),
    /// `a / b`
    Div(Box<CalcNode>, Box<CalcNode>),
    /// `min(...)`
    Min(Vec<CalcNode>),
    /// `max(...)`
    Max(Vec<CalcNode>),
    /// `clamp(min, value, max)`
    Clamp(Box<CalcNode>, Box<CalcNode>, Box<CalcNode>),
}

/// The kind of a math expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcKind {
    /// A unitless number.
    Number,
    /// A length or percentage.
    Dimension,
}

impl CalcNode {
    /// The kind of this node, or `None` if its operands do not type-check.
    pub fn kind(&self) -> Option<CalcKind> {
        use CalcKind::{Dimension, Number};
        match self {
            CalcNode::Number(_) => Some(Number),
            CalcNode::Length { .. } | CalcNode::Percent(_) | CalcNode::Linear { .. } => {
                Some(Dimension)
            }
            CalcNode::Add(a, b) | CalcNode::Sub(a, b) => {
                let (ka, kb) = (a.kind()?, b.kind()?);
                (ka == kb).then_some(ka)
            }
            CalcNode::Mul(a, b) => match (a.kind()?, b.kind()?) {
                (Number, Number) => Some(Number),
                (Number, Dimension) | (Dimension, Number) => Some(Dimension),
                _ => None,
            },
            CalcNode::Div(a, b) => match (a.kind()?, b.kind()?) {
                (ka, Number) => Some(ka),
                _ => None,
            },
            CalcNode::Min(args) | CalcNode::Max(args) => {
                let first = args.first()?.kind()?;
                args.iter()
                    .all(|a| a.kind() == Some(first))
                    .then_some(first)
            }
            CalcNode::Clamp(a, b, c) => {
                let k = a.kind()?;
                (b.kind()? == k && c.kind()? == k).then_some(k)
            }
        }
    }
}

/// Maps a unit name (ASCII case-insensitive) to a [`LengthUnit`].
pub(crate) fn unit_from_name(name: &str) -> Option<LengthUnit> {
    match name.to_ascii_lowercase().as_str() {
        "px" => Some(LengthUnit::Px),
        "em" => Some(LengthUnit::Em),
        "rem" => Some(LengthUnit::Rem),
        "vw" => Some(LengthUnit::Vw),
        "vh" => Some(LengthUnit::Vh),
        "vmin" => Some(LengthUnit::Vmin),
        "vmax" => Some(LengthUnit::Vmax),
        "cm" => Some(LengthUnit::Cm),
        "mm" => Some(LengthUnit::Mm),
        "q" => Some(LengthUnit::Q),
        "in" => Some(LengthUnit::In),
        "pt" => Some(LengthUnit::Pt),
        "pc" => Some(LengthUnit::Pc),
        _ => None,
    }
}

/// The CSS spelling of a unit.
pub fn unit_name(unit: LengthUnit) -> &'static str {
    match unit {
        LengthUnit::Px => "px",
        LengthUnit::Em => "em",
        LengthUnit::Rem => "rem",
        LengthUnit::Vw => "vw",
        LengthUnit::Vh => "vh",
        LengthUnit::Vmin => "vmin",
        LengthUnit::Vmax => "vmax",
        LengthUnit::Cm => "cm",
        LengthUnit::Mm => "mm",
        LengthUnit::Q => "Q",
        LengthUnit::In => "in",
        LengthUnit::Pt => "pt",
        LengthUnit::Pc => "pc",
    }
}

/// Parses a math function (`calc`, `min`, `max`, `clamp`) whose result must be
/// a length or percentage. Returns `None` for anything invalid, for an unknown
/// function, or for a number-typed result.
pub fn parse_math_function(name: &str, args: &[ComponentValue]) -> Option<CalcNode> {
    let node = parse_function(name, args, 0)?;
    (node.kind()? == CalcKind::Dimension).then_some(node)
}

fn parse_function(name: &str, args: &[ComponentValue], depth: usize) -> Option<CalcNode> {
    if depth > DEFAULT_CALC_DEPTH {
        return None;
    }
    match name.to_ascii_lowercase().as_str() {
        "calc" => parse_expression(args, depth + 1),
        "min" | "max" => {
            let mut parts = Vec::new();
            for part in split_commas(args) {
                parts.push(parse_expression(part, depth + 1)?);
            }
            if parts.is_empty() {
                return None;
            }
            Some(if name.eq_ignore_ascii_case("min") {
                CalcNode::Min(parts)
            } else {
                CalcNode::Max(parts)
            })
        }
        "clamp" => {
            let parts = split_commas(args);
            if parts.len() != 3 {
                return None;
            }
            let lo = parse_expression(parts[0], depth + 1)?;
            let mid = parse_expression(parts[1], depth + 1)?;
            let hi = parse_expression(parts[2], depth + 1)?;
            Some(CalcNode::Clamp(Box::new(lo), Box::new(mid), Box::new(hi)))
        }
        _ => None,
    }
}

/// Splits argument values at top-level commas.
fn split_commas(args: &[ComponentValue]) -> Vec<&[ComponentValue]> {
    crate::parse::split_on(args, |v| v.is_token(&TokenKind::Comma))
}

/// Parses a whole expression (all non-whitespace items must be consumed).
fn parse_expression(values: &[ComponentValue], depth: usize) -> Option<CalcNode> {
    if depth > DEFAULT_CALC_DEPTH {
        return None;
    }
    let items: Vec<&ComponentValue> = values.iter().filter(|v| !v.is_whitespace()).collect();
    let mut parser = Parser {
        items,
        pos: 0,
        depth,
    };
    let node = parser.sum()?;
    (parser.pos == parser.items.len()).then_some(node)
}

struct Parser<'a> {
    items: Vec<&'a ComponentValue>,
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn delim_at(&self, pos: usize) -> Option<char> {
        match self.items.get(pos) {
            Some(ComponentValue::Token(t)) => match t.kind {
                TokenKind::Delim(c) => Some(c),
                _ => None,
            },
            _ => None,
        }
    }

    fn sum(&mut self) -> Option<CalcNode> {
        let mut left = self.product()?;
        loop {
            match self.delim_at(self.pos) {
                Some('+') => {
                    self.pos += 1;
                    let right = self.product()?;
                    left = checked(CalcNode::Add(Box::new(left), Box::new(right)))?;
                }
                Some('-') => {
                    self.pos += 1;
                    let right = self.product()?;
                    left = checked(CalcNode::Sub(Box::new(left), Box::new(right)))?;
                }
                _ => return Some(left),
            }
        }
    }

    fn product(&mut self) -> Option<CalcNode> {
        let mut left = self.factor()?;
        loop {
            match self.delim_at(self.pos) {
                Some('*') => {
                    self.pos += 1;
                    let right = self.factor()?;
                    left = checked(CalcNode::Mul(Box::new(left), Box::new(right)))?;
                }
                Some('/') => {
                    self.pos += 1;
                    let right = self.factor()?;
                    left = checked(CalcNode::Div(Box::new(left), Box::new(right)))?;
                }
                _ => return Some(left),
            }
        }
    }

    fn factor(&mut self) -> Option<CalcNode> {
        let item = *self.items.get(self.pos)?;
        self.pos += 1;
        match item {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::Number { value, .. } => Some(CalcNode::Number(*value)),
                TokenKind::Percentage(p) => Some(CalcNode::Percent(*p)),
                TokenKind::Dimension { value, unit, .. } => Some(CalcNode::Length {
                    value: *value,
                    unit: unit_from_name(unit)?,
                }),
                _ => None,
            },
            ComponentValue::Function(f) => parse_function(&f.name, &f.value, self.depth + 1),
            ComponentValue::Block(b) if b.kind == crate::syntax::BlockKind::Paren => {
                parse_expression(&b.value, self.depth + 1)
            }
            _ => None,
        }
    }
}

/// Keeps a combined node only if its operands type-check.
fn checked(node: CalcNode) -> Option<CalcNode> {
    node.kind().map(|_| node)
}

/// Converts an absolute or relative length to px. `to_px` handles units.
pub type ToPx<'a> = &'a dyn Fn(f64, LengthUnit) -> f64;

/// Simplifies a parsed expression for computed-value time.
///
/// `percent_base` is `Some(px)` when percentages resolve now (font-size, whose
/// percentage refers to the parent's size). Otherwise percentages stay symbolic.
pub fn simplify(node: &CalcNode, to_px: ToPx<'_>, percent_base: Option<f64>) -> CalcNode {
    match node {
        CalcNode::Number(v) => CalcNode::Number(*v),
        CalcNode::Length { value, unit } => CalcNode::Linear {
            px: to_px(*value, *unit),
            percent: 0.0,
        },
        CalcNode::Percent(p) => match percent_base {
            Some(base) => CalcNode::Linear {
                px: base * p / 100.0,
                percent: 0.0,
            },
            None => CalcNode::Linear {
                px: 0.0,
                percent: *p,
            },
        },
        CalcNode::Linear { px, percent } => CalcNode::Linear {
            px: *px,
            percent: *percent,
        },
        CalcNode::Add(a, b) => {
            let (a, b) = (
                simplify(a, to_px, percent_base),
                simplify(b, to_px, percent_base),
            );
            match (as_linear(&a), as_linear(&b), as_number(&a), as_number(&b)) {
                (Some((pa, qa)), Some((pb, qb)), _, _) => CalcNode::Linear {
                    px: pa + pb,
                    percent: qa + qb,
                },
                (_, _, Some(x), Some(y)) => CalcNode::Number(x + y),
                _ => CalcNode::Add(Box::new(a), Box::new(b)),
            }
        }
        CalcNode::Sub(a, b) => {
            let (a, b) = (
                simplify(a, to_px, percent_base),
                simplify(b, to_px, percent_base),
            );
            match (as_linear(&a), as_linear(&b), as_number(&a), as_number(&b)) {
                (Some((pa, qa)), Some((pb, qb)), _, _) => CalcNode::Linear {
                    px: pa - pb,
                    percent: qa - qb,
                },
                (_, _, Some(x), Some(y)) => CalcNode::Number(x - y),
                _ => CalcNode::Sub(Box::new(a), Box::new(b)),
            }
        }
        CalcNode::Mul(a, b) => {
            let (a, b) = (
                simplify(a, to_px, percent_base),
                simplify(b, to_px, percent_base),
            );
            match (as_linear(&a), as_linear(&b), as_number(&a), as_number(&b)) {
                (_, _, Some(x), Some(y)) => CalcNode::Number(x * y),
                (Some((p, q)), _, _, Some(k)) | (_, Some((p, q)), Some(k), _) => CalcNode::Linear {
                    px: p * k,
                    percent: q * k,
                },
                _ => CalcNode::Mul(Box::new(a), Box::new(b)),
            }
        }
        CalcNode::Div(a, b) => {
            let (a, b) = (
                simplify(a, to_px, percent_base),
                simplify(b, to_px, percent_base),
            );
            match (as_linear(&a), as_number(&a), as_number(&b)) {
                (_, Some(x), Some(y)) if y != 0.0 => CalcNode::Number(x / y),
                (Some((p, q)), _, Some(k)) if k != 0.0 => CalcNode::Linear {
                    px: p / k,
                    percent: q / k,
                },
                _ => CalcNode::Div(Box::new(a), Box::new(b)),
            }
        }
        CalcNode::Min(args) => {
            let args: Vec<CalcNode> = args
                .iter()
                .map(|a| simplify(a, to_px, percent_base))
                .collect();
            fold_extremum(args, f64::min, CalcNode::Min)
        }
        CalcNode::Max(args) => {
            let args: Vec<CalcNode> = args
                .iter()
                .map(|a| simplify(a, to_px, percent_base))
                .collect();
            fold_extremum(args, f64::max, CalcNode::Max)
        }
        CalcNode::Clamp(lo, mid, hi) => {
            let lo = simplify(lo, to_px, percent_base);
            let mid = simplify(mid, to_px, percent_base);
            let hi = simplify(hi, to_px, percent_base);
            let all: Vec<CalcNode> = vec![lo.clone(), mid.clone(), hi.clone()];
            // clamp(lo, val, hi) = max(lo, min(val, hi)); the lower bound wins.
            match all.iter().map(as_number).collect::<Option<Vec<f64>>>() {
                Some(v) => CalcNode::Number(v[0].max(v[1].min(v[2]))),
                None => match all.iter().map(as_linear).collect::<Option<Vec<_>>>() {
                    Some(v) if v.iter().all(|(_, q)| *q == 0.0) => CalcNode::Linear {
                        px: v[0].0.max(v[1].0.min(v[2].0)),
                        percent: 0.0,
                    },
                    _ => CalcNode::Clamp(Box::new(lo), Box::new(mid), Box::new(hi)),
                },
            }
        }
    }
}

/// `min()` or `max()` over simplified arguments: folds numbers, and px-only
/// lengths. Mixed or percent arguments stay as a tree.
fn fold_extremum(
    args: Vec<CalcNode>,
    pick: fn(f64, f64) -> f64,
    rebuild: impl Fn(Vec<CalcNode>) -> CalcNode,
) -> CalcNode {
    let nums: Option<Vec<f64>> = args.iter().map(as_number).collect();
    if let Some(nums) = nums {
        if let Some(first) = nums.first() {
            return CalcNode::Number(nums[1..].iter().fold(*first, |acc, x| pick(acc, *x)));
        }
    }
    let lins: Option<Vec<(f64, f64)>> = args.iter().map(as_linear).collect();
    if let Some(lins) = lins {
        if lins.iter().all(|(_, q)| *q == 0.0) {
            if let Some((first, _)) = lins.first() {
                let px = lins[1..].iter().fold(*first, |acc, (x, _)| pick(acc, *x));
                return CalcNode::Linear { px, percent: 0.0 };
            }
        }
    }
    rebuild(args)
}

fn as_number(node: &CalcNode) -> Option<f64> {
    match node {
        CalcNode::Number(v) => Some(*v),
        _ => None,
    }
}

/// `(px, percent)` for a node that is a linear length or percentage.
fn as_linear(node: &CalcNode) -> Option<(f64, f64)> {
    match node {
        CalcNode::Linear { px, percent } => Some((*px, *percent)),
        CalcNode::Length { value, .. } => Some((*value, 0.0)),
        CalcNode::Percent(p) => Some((0.0, *p)),
        _ => None,
    }
}

/// Serializes a computed or specified math node in CSS form.
pub fn serialize(node: &CalcNode) -> String {
    match node {
        CalcNode::Number(v) => format_number(*v),
        CalcNode::Length { value, unit } => {
            format!("{}{}", format_number(*value), unit_name(*unit))
        }
        CalcNode::Percent(p) => format!("{}%", format_number(*p)),
        CalcNode::Linear { px, percent } => match (*px == 0.0, *percent == 0.0) {
            (_, true) => format!("{}px", format_number(*px)),
            (true, false) => format!("{}%", format_number(*percent)),
            _ => format!(
                "calc({}px + {}%)",
                format_number(*px),
                format_number(*percent)
            ),
        },
        CalcNode::Add(a, b) => format!("({} + {})", serialize(a), serialize(b)),
        CalcNode::Sub(a, b) => format!("({} - {})", serialize(a), serialize(b)),
        CalcNode::Mul(a, b) => format!("({} * {})", serialize(a), serialize(b)),
        CalcNode::Div(a, b) => format!("({} / {})", serialize(a), serialize(b)),
        CalcNode::Min(args) => format!("min({})", join(args)),
        CalcNode::Max(args) => format!("max({})", join(args)),
        CalcNode::Clamp(a, b, c) => format!(
            "clamp({}, {}, {})",
            serialize(a),
            serialize(b),
            serialize(c)
        ),
    }
}

fn join(args: &[CalcNode]) -> String {
    args.iter().map(serialize).collect::<Vec<_>>().join(", ")
}
