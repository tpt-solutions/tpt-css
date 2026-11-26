// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Stylesheet parsing entry point (spec 8.1).

use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::rules::consume_rule_list;
use super::ParseOptions;
use crate::ast::Stylesheet;
use crate::syntax::component::build_component_values;
use crate::syntax::{tokenize, Span, Token};

/// A parsed stylesheet with its diagnostics (spec 8.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedStylesheet {
    /// The stylesheet AST.
    pub stylesheet: Stylesheet,
    /// Diagnostics. Not required for style resolution.
    pub diagnostics: Vec<Diagnostic>,
    /// Source spans of the comments, in order. Empty unless
    /// [`ParseOptions::preserve_comments`] is set.
    pub comments: Vec<Span>,
}

/// Parses a stylesheet with spec-defined error recovery.
///
/// Invalid declarations and rules are dropped with a diagnostic. Only a
/// nesting-depth violation drops the whole sheet, because component values
/// are built for the whole input at once (finer recovery is M1 work).
pub fn parse_stylesheet(input: &str, options: ParseOptions) -> ParsedStylesheet {
    let limits = options.limits;
    let mut diagnostics = Vec::new();

    let tokenized = tokenize(options.source, input, &limits);
    if tokenized.limit_exceeded {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            first_span(&tokenized.tokens),
            "input exceeds tokenizer limits; the rest of the input was ignored",
            None,
        ));
    }

    let values = match build_component_values(&tokenized.tokens, limits.max_nesting_depth) {
        Ok(values) => values,
        Err(_) => {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::LimitExceeded,
                first_span(&tokenized.tokens),
                "nesting depth limit exceeded; stylesheet dropped",
                None,
            ));
            return ParsedStylesheet {
                stylesheet: Stylesheet::default(),
                diagnostics,
                comments: keep_comments(&options, &tokenized),
            };
        }
    };

    let rules = consume_rule_list(&values, &limits, &mut diagnostics, 0);
    ParsedStylesheet {
        stylesheet: Stylesheet { rules },
        diagnostics,
        comments: keep_comments(&options, &tokenized),
    }
}

/// The comment spans, when the caller asked for them.
fn keep_comments(options: &ParseOptions, tokenized: &crate::syntax::Tokenized) -> Vec<Span> {
    if options.preserve_comments {
        tokenized.comments.clone()
    } else {
        Vec::new()
    }
}

fn first_span(tokens: &[Token]) -> Span {
    tokens.first().map_or(Span::default(), |t| t.span)
}
