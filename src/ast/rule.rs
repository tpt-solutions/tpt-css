// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Rules (spec 11.2, 11.3). M1 implements style rules, `@media`, and
//! preserves other at-rules. The remaining at-rules are later milestones.

use super::declaration::DeclarationBlock;
use super::media::MediaQuery;
use super::selector::SelectorList;
use crate::syntax::{ComponentValue, Span};

/// A rule in a stylesheet or a conditional group rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// `selectors { declarations }`
    Style(StyleRule),
    /// `@media queries { rules }`
    Media(MediaRule),
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

/// A conditional `@media` rule (spec 16.3).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaRule {
    /// Queries from the prelude. The rule applies if any query matches.
    pub queries: Vec<MediaQuery>,
    /// Rules inside the block.
    pub rules: Vec<Rule>,
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
