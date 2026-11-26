// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Parsing (spec 8.1, 11): stylesheets, rules, declarations, and selectors.
//!
//! Parsing depends on the syntax layer and the AST. It does not depend on
//! properties or the cascade.

pub mod declarations;
pub mod diagnostics;
pub(crate) mod media;
pub mod rules;
pub mod selectors;
pub mod stylesheet;
pub(crate) mod supports;

use crate::limits::ParseLimits;
use crate::syntax::{ComponentValue, SourceId};

pub use declarations::{parse_declaration_list, ParsedDeclarationList};
pub use diagnostics::{Diagnostic, DiagnosticCode, Severity};
pub use stylesheet::{parse_stylesheet, ParsedStylesheet};

/// Options for parsing (spec 8.1).
///
/// Deviations from the spec 8.1 sketch, recorded in DESIGN.md: the origin is
/// given to [`crate::StyleResolver::add_stylesheet`] instead, spans are always
/// preserved, and quirks mode is not modeled in the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    /// Source identifier recorded in spans.
    pub source: SourceId,
    /// Limits applied while parsing.
    pub limits: ParseLimits,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            source: SourceId(0),
            limits: ParseLimits::default(),
        }
    }
}

/// Splits `values` at each top-level value matching `is_sep`. Separators are
/// dropped. Nested blocks and functions are single values, so only top-level
/// separators match.
pub(crate) fn split_on(
    values: &[ComponentValue],
    is_sep: impl Fn(&ComponentValue) -> bool,
) -> Vec<&[ComponentValue]> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, v) in values.iter().enumerate() {
        if is_sep(v) {
            out.push(&values[start..i]);
            start = i + 1;
        }
    }
    out.push(&values[start..]);
    out
}
