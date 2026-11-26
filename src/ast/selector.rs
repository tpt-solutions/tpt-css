// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector AST (spec 12.1). M0 covers Tier 0 selectors plus `:root`,
//! `:hover`, `:active`, `:focus`, and the descendant and child combinators.

use crate::selectors::specificity::Specificity;
use crate::syntax::Span;

/// A selector list: `a, b c`.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorList {
    /// Complex selectors, in source order.
    pub selectors: Vec<ComplexSelector>,
}

/// A complex selector: compound selectors joined by combinators.
///
/// Invariant: `combinators.len() + 1 == compounds.len()`.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexSelector {
    /// Compound selectors, left to right.
    pub compounds: Vec<CompoundSelector>,
    /// Combinator between `compounds[i]` and `compounds[i + 1]`.
    pub combinators: Vec<Combinator>,
    /// Specificity computed at parse time (spec 12.3).
    pub specificity: Specificity,
    /// Source span of the whole selector.
    pub span: Span,
}

/// A compound selector: simple selectors with no combinator between them.
#[derive(Debug, Clone, PartialEq)]
pub struct CompoundSelector {
    /// Simple selectors, all of which must match the same element.
    pub simple: Vec<SimpleSelector>,
}

/// Combinators supported in M0 (spec 12.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Whitespace: some ancestor.
    Descendant,
    /// `>`: the parent.
    Child,
}

/// A simple selector.
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleSelector {
    /// `*`
    Universal,
    /// Type selector, local name only (namespaces are not supported yet).
    Type(String),
    /// `#id`
    Id(String),
    /// `.class`
    Class(String),
    /// `[attr]` or `[attr=value]`
    Attribute(AttributeSelector),
    /// `:name`
    PseudoClass(PseudoClass),
}

/// An attribute selector.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeSelector {
    /// Attribute name as written.
    pub name: String,
    /// Optional operator and value. M0 supports only `=`.
    pub matcher: Option<AttributeMatcher>,
}

/// The matcher of an attribute selector.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeMatcher {
    /// Operator. M0 has only exact match.
    pub operator: AttributeOperator,
    /// Expected value.
    pub value: String,
}

/// Attribute matching operators (only `=` in M0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeOperator {
    /// `=`
    Equals,
}

/// Pseudo-classes supported in M0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoClass {
    /// `:root`
    Root,
    /// `:hover` (adapter-provided state)
    Hover,
    /// `:active` (adapter-provided state)
    Active,
    /// `:focus` (adapter-provided state)
    Focus,
}
