// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Declarations and declaration blocks (spec 11.4).

use crate::syntax::{ComponentValue, Span};

/// A list of declarations, as in a style rule or an inline `style` attribute.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeclarationBlock {
    /// Valid declarations in source order. Invalid ones are dropped at parse
    /// time with a diagnostic.
    pub declarations: Vec<Declaration>,
    /// Source span of the block contents.
    pub span: Span,
}

/// One declaration: `name: value [!important]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    /// Property name. ASCII-lowercased unless it is a custom property (`--`).
    pub property: String,
    /// Component values of the value, with surrounding whitespace removed.
    pub value: Vec<ComponentValue>,
    /// Whether `!important` was present.
    pub important: bool,
    /// Source span of the declaration.
    pub span: Span,
}
