// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Rules (spec 11.2, 11.3). M2 adds `@layer`, `@supports`, `@container`,
//! `@property`, `@keyframes`, and `@font-face`. Style rules may nest.

use super::declaration::DeclarationBlock;
use super::media::{MediaFeature, MediaQuery};
use super::selector::SelectorList;
use crate::syntax::{ComponentValue, Span};

/// A rule in a stylesheet or a conditional group rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// `selectors { declarations }`, possibly with nested rules.
    Style(StyleRule),
    /// `@media queries { rules }`
    Media(MediaRule),
    /// `@layer names;` or `@layer name { rules }` (spec 13.4).
    Layer(LayerRule),
    /// `@supports condition { rules }`. The condition is evaluated at parse
    /// time, because it depends only on the engine's capabilities.
    Supports(SupportsRule),
    /// `@container name? (features) { rules }` (spec 16.5).
    Container(ContainerRule),
    /// `@property --name { ... }` (spec 15.1, registered custom properties).
    Property(PropertyRule),
    /// `@keyframes name { ... }`
    Keyframes(KeyframesRule),
    /// `@font-face { ... }`
    FontFace(FontFaceRule),
    /// An at-rule that is not supported yet. Preserved for CSSOM and error
    /// recovery, but it does not affect style resolution.
    Unknown(UnknownAtRule),
}

/// A style rule (spec 11.3).
#[derive(Debug, Clone, PartialEq)]
pub struct StyleRule {
    /// Selector list from the prelude. For a nested rule, `&` has been resolved
    /// into the selectors, so this list matches on its own.
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

/// An `@layer` rule (spec 13.4).
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRule {
    /// Layer names. A statement may name several (`@layer reset, base;`). A
    /// block names one, or none for an anonymous layer.
    pub names: Vec<String>,
    /// The block's rules, or `None` for the statement form.
    pub rules: Option<Vec<Rule>>,
    /// Span of the whole rule.
    pub span: Span,
}

/// An `@supports` rule with its condition already evaluated.
#[derive(Debug, Clone, PartialEq)]
pub struct SupportsRule {
    /// Whether the condition holds for this engine.
    pub supported: bool,
    /// Rules inside the block.
    pub rules: Vec<Rule>,
    /// Span of the whole rule.
    pub span: Span,
}

/// An `@container` rule (spec 16.5). All features must hold.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerRule {
    /// Container name, if given.
    pub name: Option<String>,
    /// Size features (`min-width`, `max-width`, and so on).
    pub features: Vec<MediaFeature>,
    /// Rules inside the block.
    pub rules: Vec<Rule>,
    /// Span of the whole rule.
    pub span: Span,
}

/// An `@property` rule (spec 15.1, CSS Properties and Values).
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRule {
    /// The custom property name, including `--`.
    pub name: String,
    /// The `syntax` descriptor as written (for example `"<length>"`), if given.
    pub syntax: Option<String>,
    /// The `inherits` descriptor. Required for a valid rule; defaults to `false`.
    pub inherits: bool,
    /// The `initial-value` descriptor as component values, if given.
    pub initial: Option<Vec<ComponentValue>>,
    /// Span of the whole rule.
    pub span: Span,
}

/// An `@keyframes` rule. Keyframe selectors and declarations are stored as
/// written. Interpolation is not implemented yet.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframesRule {
    /// Animation name.
    pub name: String,
    /// The keyframes in source order.
    pub keyframes: Vec<Keyframe>,
    /// Span of the whole rule.
    pub span: Span,
}

/// One keyframe: `from`, `to`, or a percentage, with its declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    /// Keyframe selectors as written, for example `["from"]` or `["0%", "50%"]`.
    pub selectors: Vec<String>,
    /// The declarations.
    pub declarations: DeclarationBlock,
}

/// An `@font-face` rule. Descriptors are kept as declarations. Font loading is
/// the host's job (spec 3).
#[derive(Debug, Clone, PartialEq)]
pub struct FontFaceRule {
    /// The descriptors.
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
