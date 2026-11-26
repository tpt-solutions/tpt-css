// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Abstract syntax tree for stylesheets, rules, declarations, and selectors.

pub mod declaration;
pub mod rule;
pub mod selector;
pub mod stylesheet;

pub use declaration::{Declaration, DeclarationBlock};
pub use rule::{Rule, StyleRule, UnknownAtRule};
pub use selector::{
    AttributeMatcher, AttributeOperator, AttributeSelector, Combinator, ComplexSelector,
    CompoundSelector, PseudoClass, SelectorList, SimpleSelector,
};
pub use stylesheet::Stylesheet;
