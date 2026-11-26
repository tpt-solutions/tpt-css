// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Math functions (spec 14.4): `calc()`, `min()`, `max()`, `clamp()`, and the
//! numeric functions `round()`, `mod()`, `rem()`, `abs()`, `sign()`, `hypot()`,
//! `sqrt()`, `pow()`, `log()`, `exp()`, the trigonometric functions, and
//! `atan2()`.
//!
//! Parsing builds a [`CalcNode`] tree with units kept. Every node has a kind:
//! a plain `Number`, or a `Dimension` (a length or percentage). Operators and
//! function arguments are type-checked when the tree is built, so an expression
//! such as `10px + 2` is invalid and the declaration is dropped.
//!
//! Simplification runs at computed-value time (`simplify`). Lengths become px,
//! and linear forms collapse to `Linear { px, percent }`. Percentages stay
//! symbolic for layout, except in `font-size`, where the parent's size is known.
//! Expressions that mix px and percent inside `min()`, `max()`, `clamp()`, or a
//! rounding function stay as trees for layout to resolve (spec 4.4).
//!
//! Angles and times are not parsed as values, so the trigonometric functions
//! take plain numbers (radians), as CSS does for unitless arguments.

use super::serialize::format_number;
use super::LengthUnit;
use crate::syntax::{ComponentValue, TokenKind};

/// The recursion bound for math functions and parentheses inside one value. It
/// matches the default component nesting limit, which the tokenizer enforces. The
/// configured `ParseLimits::max_calc_depth` is enforced where declarations enter
/// the engine (`parse::declarations::within_math_depth` and `resolve_unparsed`).
pub const DEFAULT_CALC_DEPTH: usize = 256;

/// The rounding strategy of `round()` (spec 14.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundStrategy {
    /// `nearest`, the default. Ties round toward positive infinity.
    Nearest,
    /// `up`: toward positive infinity.
    Up,
    /// `down`: toward negative infinity.
    Down,
    /// `to-zero`: toward zero.
    ToZero,
}

/// A numeric function of `calc()` (spec 14.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathFn {
    /// `round(strategy?, value, step?)`
    Round(RoundStrategy),
    /// `mod(a, b)`: the result has the sign of `b`.
    Mod,
    /// `rem(a, b)`: the result has the sign of `a`.
    Rem,
    /// `abs(x)`
    Abs,
    /// `sign(x)`
    Sign,
    /// `hypot(...)`
    Hypot,
    /// `sqrt(x)`
    Sqrt,
    /// `pow(x, y)`
    Pow,
    /// `log(x, base?)`
    Log,
    /// `exp(x)`
    Exp,
    /// `sin(x)`, radians
    Sin,
    /// `cos(x)`, radians
    Cos,
    /// `tan(x)`, radians
    Tan,
    /// `asin(x)`
    Asin,
    /// `acos(x)`
    Acos,
    /// `atan(x)`
    Atan,
    /// `progress(v, from, to)`: where `v` lies between `from` and `to`, as a number.
    Progress,
    /// `atan2(y, x)`
    Atan2,
}

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
    /// A numeric function applied to its arguments.
    Func(MathFn, Vec<CalcNode>),
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
            CalcNode::Func(f, args) => func_kind(*f, args),
        }
    }
}

/// The kind of a numeric function applied to `args`, or `None` if the arguments
/// do not fit the function.
fn func_kind(f: MathFn, args: &[CalcNode]) -> Option<CalcKind> {
    use CalcKind::Number;
    let kinds: Vec<CalcKind> = args.iter().map(|a| a.kind()).collect::<Option<_>>()?;
    let first = *kinds.first()?;
    match f {
        MathFn::Round(_) | MathFn::Mod | MathFn::Rem | MathFn::Abs | MathFn::Hypot => {
            kinds.iter().all(|k| *k == first).then_some(first)
        }
        MathFn::Sign | MathFn::Progress => Some(Number),
        _ => kinds.iter().all(|k| *k == Number).then_some(Number),
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
        "cqw" => Some(LengthUnit::Cqw),
        "cqh" => Some(LengthUnit::Cqh),
        "cqi" => Some(LengthUnit::Cqi),
        "cqb" => Some(LengthUnit::Cqb),
        "cqmin" => Some(LengthUnit::Cqmin),
        "cqmax" => Some(LengthUnit::Cqmax),
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
        LengthUnit::Cqw => "cqw",
        LengthUnit::Cqh => "cqh",
        LengthUnit::Cqi => "cqi",
        LengthUnit::Cqb => "cqb",
        LengthUnit::Cqmin => "cqmin",
        LengthUnit::Cqmax => "cqmax",
    }
}

/// Parses a math function (`calc`, `min`, `max`, `clamp`, or a numeric function)
/// whose result must be a length or percentage, with the default depth limit.
pub fn parse_math_function(name: &str, args: &[ComponentValue]) -> Option<CalcNode> {
    parse_math_function_limited(name, args, DEFAULT_CALC_DEPTH)
}

/// Like [`parse_math_function`], with an explicit nesting limit (`max_calc_depth`).
pub fn parse_math_function_limited(
    name: &str,
    args: &[ComponentValue],
    max_depth: usize,
) -> Option<CalcNode> {
    let node = parse_function(name, args, 0, max_depth)?;
    (node.kind()? == CalcKind::Dimension).then_some(node)
}

fn parse_function(
    name: &str,
    args: &[ComponentValue],
    depth: usize,
    max: usize,
) -> Option<CalcNode> {
    if depth > max {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "calc" => parse_expression(args, depth + 1, max),
        "min" | "max" => {
            let mut parts = Vec::new();
            for part in split_commas(args) {
                parts.push(parse_expression(part, depth + 1, max)?);
            }
            if parts.is_empty() {
                return None;
            }
            Some(if lower == "min" {
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
            let lo = parse_expression(parts[0], depth + 1, max)?;
            let mid = parse_expression(parts[1], depth + 1, max)?;
            let hi = parse_expression(parts[2], depth + 1, max)?;
            Some(CalcNode::Clamp(Box::new(lo), Box::new(mid), Box::new(hi)))
        }
        "round" => {
            let mut parts = split_commas(args);
            let strategy = match parts.first().map(|p| trim(p)) {
                Some([ComponentValue::Token(t)]) if parts.len() == 3 => match &t.kind {
                    TokenKind::Ident(s) => match s.to_ascii_lowercase().as_str() {
                        "nearest" => Some(RoundStrategy::Nearest),
                        "up" => Some(RoundStrategy::Up),
                        "down" => Some(RoundStrategy::Down),
                        "to-zero" => Some(RoundStrategy::ToZero),
                        _ => return None,
                    },
                    _ => None,
                },
                _ => None,
            };
            if strategy.is_some() {
                parts.remove(0);
            }
            if parts.is_empty() || parts.len() > 2 {
                return None;
            }
            let nodes = parse_parts(&parts, depth, max)?;
            func(
                MathFn::Round(strategy.unwrap_or(RoundStrategy::Nearest)),
                nodes,
            )
        }
        "mod" | "rem" | "pow" | "atan2" => {
            let parts = split_commas(args);
            if parts.len() != 2 {
                return None;
            }
            let f = match lower.as_str() {
                "mod" => MathFn::Mod,
                "rem" => MathFn::Rem,
                "pow" => MathFn::Pow,
                _ => MathFn::Atan2,
            };
            func(f, parse_parts(&parts, depth, max)?)
        }
        "abs" | "sign" | "sqrt" | "exp" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" => {
            let parts = split_commas(args);
            if parts.len() != 1 {
                return None;
            }
            let f = match lower.as_str() {
                "abs" => MathFn::Abs,
                "sign" => MathFn::Sign,
                "sqrt" => MathFn::Sqrt,
                "exp" => MathFn::Exp,
                "sin" => MathFn::Sin,
                "cos" => MathFn::Cos,
                "tan" => MathFn::Tan,
                "asin" => MathFn::Asin,
                "acos" => MathFn::Acos,
                _ => MathFn::Atan,
            };
            func(f, parse_parts(&parts, depth, max)?)
        }
        "progress" => {
            let parts = split_commas(args);
            if parts.len() != 3 {
                return None;
            }
            func(MathFn::Progress, parse_parts(&parts, depth, max)?)
        }
        "log" => {
            let parts = split_commas(args);
            if parts.is_empty() || parts.len() > 2 {
                return None;
            }
            func(MathFn::Log, parse_parts(&parts, depth, max)?)
        }
        "hypot" => {
            let parts = split_commas(args);
            if parts.is_empty() {
                return None;
            }
            func(MathFn::Hypot, parse_parts(&parts, depth, max)?)
        }
        _ => None,
    }
}

/// Builds a numeric function node if its arguments type-check.
fn func(f: MathFn, args: Vec<CalcNode>) -> Option<CalcNode> {
    let node = CalcNode::Func(f, args);
    node.kind().map(|_| node)
}

fn trim(values: &[ComponentValue]) -> &[ComponentValue] {
    crate::syntax::trim_whitespace(values)
}

fn parse_parts(parts: &[&[ComponentValue]], depth: usize, max: usize) -> Option<Vec<CalcNode>> {
    parts
        .iter()
        .map(|p| parse_expression(p, depth + 1, max))
        .collect()
}

/// Splits argument values at top-level commas.
fn split_commas(args: &[ComponentValue]) -> Vec<&[ComponentValue]> {
    crate::parse::split_on(args, |v| v.is_token(&TokenKind::Comma))
}

/// Parses a whole expression (all non-whitespace items must be consumed).
fn parse_expression(values: &[ComponentValue], depth: usize, max: usize) -> Option<CalcNode> {
    if depth > max {
        return None;
    }
    let items: Vec<&ComponentValue> = values.iter().filter(|v| !v.is_whitespace()).collect();
    let mut parser = Parser {
        items,
        pos: 0,
        depth,
        max,
    };
    let node = parser.sum()?;
    (parser.pos == parser.items.len()).then_some(node)
}

struct Parser<'a> {
    items: Vec<&'a ComponentValue>,
    pos: usize,
    depth: usize,
    max: usize,
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
            ComponentValue::Function(f) => {
                parse_function(&f.name, &f.value, self.depth + 1, self.max)
            }
            ComponentValue::Block(b) if b.kind == crate::syntax::BlockKind::Paren => {
                parse_expression(&b.value, self.depth + 1, self.max)
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
    let s = |n: &CalcNode| simplify(n, to_px, percent_base);
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
            let (a, b) = (s(a), s(b));
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
            let (a, b) = (s(a), s(b));
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
            let (a, b) = (s(a), s(b));
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
            let (a, b) = (s(a), s(b));
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
            let args: Vec<CalcNode> = args.iter().map(s).collect();
            fold_extremum(args, f64::min, CalcNode::Min)
        }
        CalcNode::Max(args) => {
            let args: Vec<CalcNode> = args.iter().map(s).collect();
            fold_extremum(args, f64::max, CalcNode::Max)
        }
        CalcNode::Clamp(lo, mid, hi) => {
            let lo = s(lo);
            let mid = s(mid);
            let hi = s(hi);
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
        CalcNode::Func(f, args) => {
            let args: Vec<CalcNode> = args.iter().map(s).collect();
            fold_func(*f, args)
        }
    }
}

/// Evaluates a numeric function on plain scalars. `None` means the result is
/// undefined (for example a division by zero or a square root of a negative).
fn eval_scalar(f: MathFn, v: &[f64]) -> Option<f64> {
    let x = *v.first()?;
    match f {
        MathFn::Round(strategy) => {
            let step = v.get(1).copied().unwrap_or(1.0);
            if step == 0.0 {
                return None;
            }
            let q = x / step;
            let r = match strategy {
                RoundStrategy::Nearest => (q + 0.5).floor(),
                RoundStrategy::Up => q.ceil(),
                RoundStrategy::Down => q.floor(),
                RoundStrategy::ToZero => q.trunc(),
            };
            Some(r * step)
        }
        MathFn::Mod => {
            let b = *v.get(1)?;
            (b != 0.0).then(|| x - b * (x / b).floor())
        }
        MathFn::Rem => {
            let b = *v.get(1)?;
            (b != 0.0).then(|| x - b * (x / b).trunc())
        }
        MathFn::Abs => Some(x.abs()),
        MathFn::Sign => Some(if x == 0.0 { 0.0 } else { x.signum() }),
        MathFn::Hypot => Some(v.iter().map(|a| a * a).sum::<f64>().sqrt()),
        MathFn::Sqrt => (x >= 0.0).then(|| x.sqrt()),
        MathFn::Pow => Some(x.powf(*v.get(1)?)),
        MathFn::Log => match v.get(1) {
            Some(base) => Some(x.ln() / base.ln()),
            None => Some(x.ln()),
        },
        MathFn::Exp => Some(x.exp()),
        MathFn::Sin => Some(x.sin()),
        MathFn::Cos => Some(x.cos()),
        MathFn::Tan => Some(x.tan()),
        MathFn::Asin => (-1.0..=1.0).contains(&x).then(|| x.asin()),
        MathFn::Acos => (-1.0..=1.0).contains(&x).then(|| x.acos()),
        MathFn::Atan => Some(x.atan()),
        MathFn::Atan2 => Some(x.atan2(*v.get(1)?)),
        MathFn::Progress => {
            let (from, to) = (*v.get(1)?, *v.get(2)?);
            (from != to).then(|| (x - from) / (to - from))
        }
    }
}

/// Folds a numeric function: constant arguments give a number, and px-only
/// length arguments give px (for the functions that return a length). Other
/// cases stay as a tree.
fn fold_func(f: MathFn, args: Vec<CalcNode>) -> CalcNode {
    if let Some(nums) = args.iter().map(as_number).collect::<Option<Vec<f64>>>() {
        return match eval_scalar(f, &nums) {
            Some(v) => CalcNode::Number(v),
            None => CalcNode::Func(f, args),
        };
    }
    let lins: Option<Vec<(f64, f64)>> = args.iter().map(as_linear).collect();
    if let Some(lins) = lins {
        if lins.iter().all(|(_, q)| *q == 0.0) {
            let px: Vec<f64> = lins.iter().map(|(p, _)| *p).collect();
            if let Some(v) = eval_scalar(f, &px) {
                return match f {
                    MathFn::Sign | MathFn::Progress => CalcNode::Number(v),
                    MathFn::Round(_) | MathFn::Mod | MathFn::Rem | MathFn::Abs | MathFn::Hypot => {
                        CalcNode::Linear {
                            px: v,
                            percent: 0.0,
                        }
                    }
                    _ => CalcNode::Func(f, args),
                };
            }
        }
    }
    CalcNode::Func(f, args)
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

/// The CSS name of a numeric function.
fn func_name(f: MathFn) -> &'static str {
    match f {
        MathFn::Round(_) => "round",
        MathFn::Mod => "mod",
        MathFn::Rem => "rem",
        MathFn::Abs => "abs",
        MathFn::Sign => "sign",
        MathFn::Hypot => "hypot",
        MathFn::Sqrt => "sqrt",
        MathFn::Pow => "pow",
        MathFn::Log => "log",
        MathFn::Exp => "exp",
        MathFn::Sin => "sin",
        MathFn::Cos => "cos",
        MathFn::Tan => "tan",
        MathFn::Asin => "asin",
        MathFn::Acos => "acos",
        MathFn::Atan => "atan",
        MathFn::Atan2 => "atan2",
        MathFn::Progress => "progress",
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
        CalcNode::Func(f, args) => match f {
            MathFn::Round(RoundStrategy::Nearest) => format!("round({})", join(args)),
            MathFn::Round(RoundStrategy::Up) => format!("round(up, {})", join(args)),
            MathFn::Round(RoundStrategy::Down) => format!("round(down, {})", join(args)),
            MathFn::Round(RoundStrategy::ToZero) => format!("round(to-zero, {})", join(args)),
            other => format!("{}({})", func_name(*other), join(args)),
        },
    }
}

fn join(args: &[CalcNode]) -> String {
    args.iter().map(serialize).collect::<Vec<_>>().join(", ")
}

/// The names of the math functions that count toward `max_calc_depth`.
fn is_math_function_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "calc"
            | "min"
            | "max"
            | "clamp"
            | "round"
            | "mod"
            | "rem"
            | "abs"
            | "sign"
            | "hypot"
            | "sqrt"
            | "pow"
            | "log"
            | "exp"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "progress"
    )
}

/// The deepest nesting of math functions in `values`, the quantity that
/// `ParseLimits::max_calc_depth` bounds (spec 20.3). Other functions and blocks
/// add no depth of their own, but they can contain math functions.
pub fn math_nesting_depth(values: &[ComponentValue]) -> usize {
    values
        .iter()
        .map(|v| match v {
            ComponentValue::Function(f) => {
                let inner = math_nesting_depth(&f.value);
                if is_math_function_name(&f.name) {
                    inner + 1
                } else {
                    inner
                }
            }
            ComponentValue::Block(b) => math_nesting_depth(&b.value),
            ComponentValue::Token(_) => 0,
        })
        .max()
        .unwrap_or(0)
}
