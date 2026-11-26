// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Stylesheet AST (spec 11.1).

use super::rule::Rule;

/// A parsed stylesheet. Diagnostics are returned alongside it in
/// [`crate::parse::ParsedStylesheet`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stylesheet {
    /// Rules in source order.
    pub rules: Vec<Rule>,
}
