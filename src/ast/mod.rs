// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Abstract syntax tree for stylesheets, rules, declarations, selectors, and
//! media queries.

pub mod declaration;
pub mod media;
pub mod rule;
pub mod selector;
pub mod stylesheet;

pub use declaration::{Declaration, DeclarationBlock};
pub use media::{Comparison, MediaFeature, MediaQuery, MediaType, PointerKind};
pub use rule::{
    ContainerRule, FontFaceRule, Keyframe, KeyframesRule, LayerRule, MediaRule, PropertyRule, Rule,
    ScopeRule, StyleRule, SupportsRule, UnknownAtRule,
};
pub use selector::{
    AttributeMatcher, AttributeOperator, AttributeSelector, Combinator, ComplexSelector,
    CompoundSelector, NthSelector, PseudoClass, RelativeSelector, SelectorList, SimpleSelector,
};
pub use stylesheet::Stylesheet;
