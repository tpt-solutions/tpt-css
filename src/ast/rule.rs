// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Rules (spec 11.2). M0 implements style rules and preserves unknown
//! at-rules. Other at-rules are added in later milestones.

use super::declaration::DeclarationBlock;
use super::selector::SelectorList;
use crate::syntax::{ComponentValue, Span};

/// A rule in a stylesheet.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// `selectors { declarations }`
    Style(StyleRule),
    /// An at-rule that is not supported yet. Preserved for CSSOM and error
    /// recovery, but it does not affect style resolution.
    Unknown(UnknownAtRule),
}

/// A style rule (spec 11.3).
#[derive(Debug, Clone, PartialEq)]
pub struct StyleRule {
    /// Selector list from the prelude.
    pub selectors: SelectorList,
    /// Declarations from the block.
    pub declarations: DeclarationBlock,
    /// Span of the whole rule.
    pub span: Span,
}

/// An unsupported at-rule, preserved verbatim as component values.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownAtRule {
    /// At-keyword name without `@`.
    pub name: String,
    /// Prelude component values.
    pub prelude: Vec<ComponentValue>,
    /// Block component values, if the rule had a `{}` block.
    pub block: Option<Vec<ComponentValue>>,
    /// Span of the whole rule.
    pub span: Span,
}
