// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Declaration parsing (spec 11.4; CSS Syntax 3, declarations).
//!
//! Invalid declarations are dropped with a diagnostic and never abort the
//! surrounding block or stylesheet.
//!
//! Custom properties (`--name`) are parsed and kept with their raw value.
//! Resolving them is M1 work.

use super::diagnostics::{Diagnostic, DiagnosticCode, Severity};
use super::selectors::span_of;
use super::split_on;
use super::ParseOptions;
use crate::ast::Declaration;
use crate::limits::ParseLimits;
use crate::syntax::component::build_component_values;
use crate::syntax::{tokenize, trim_whitespace, ComponentValue, TokenKind};

const DECLARATIONS_URL: &str = "https://www.w3.org/TR/css-syntax-3/#consume-declaration";

/// Result of parsing an inline declaration list (`style` attribute).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDeclarationList {
    /// Valid declarations in source order.
    pub declarations: Vec<Declaration>,
    /// Diagnostics for dropped declarations and limits.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses a declaration list such as a `style` attribute value.
pub fn parse_declaration_list(input: &str, options: ParseOptions) -> ParsedDeclarationList {
    let mut diagnostics = Vec::new();
    let limits = options.limits;
    let tokenized = tokenize(options.source, input, &limits);
    if tokenized.limit_exceeded {
        diagnostics.push(limit_diag(
            &tokenized.tokens,
            "input exceeds tokenizer limits",
        ));
    }
    let values = match build_component_values(&tokenized.tokens, limits.max_nesting_depth) {
        Ok(v) => v,
        Err(_) => {
            diagnostics.push(limit_diag(
                &tokenized.tokens,
                "nesting depth limit exceeded",
            ));
            return ParsedDeclarationList {
                declarations: Vec::new(),
                diagnostics,
            };
        }
    };
    let declarations = parse_block_contents(&values, &limits, &mut diagnostics);
    ParsedDeclarationList {
        declarations,
        diagnostics,
    }
}

/// Parses the contents of a `{ }` block, or a whole declaration list.
pub(crate) fn parse_block_contents(
    values: &[ComponentValue],
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Declaration> {
    let mut out = Vec::new();
    for chunk in split_on(values, |v| v.is_token(&TokenKind::Semicolon)) {
        let chunk = trim_whitespace(chunk);
        if chunk.is_empty() {
            continue;
        }
        if out.len() >= limits.max_declarations_per_block {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::LimitExceeded,
                span_of(chunk),
                "declaration count limit exceeded",
                Some(DECLARATIONS_URL),
            ));
            break;
        }
        match parse_declaration(chunk) {
            Ok(decl) => out.push(decl),
            Err(diag) => diagnostics.push(diag),
        }
    }
    out
}

fn parse_declaration(values: &[ComponentValue]) -> Result<Declaration, Diagnostic> {
    let span = span_of(values);
    let name = match values.first() {
        Some(ComponentValue::Token(t)) => match &t.kind {
            TokenKind::Ident(n) => n.clone(),
            _ => return Err(invalid(span, "expected a property name")),
        },
        _ => return Err(invalid(span, "expected a property name")),
    };

    let mut i = 1;
    while i < values.len() && values[i].is_whitespace() {
        i += 1;
    }
    if !values.get(i).is_some_and(|v| v.is_token(&TokenKind::Colon)) {
        return Err(invalid(span, "expected ':' after property name"));
    }

    let (important, value_part) = strip_important(&values[i + 1..]);
    let value = trim_whitespace(value_part);
    if value.is_empty() {
        return Err(invalid(span, "declaration has no value"));
    }

    let property = if name.starts_with("--") {
        name
    } else {
        name.to_ascii_lowercase()
    };
    Ok(Declaration {
        property,
        value: value.to_vec(),
        important,
        span,
    })
}

/// Splits a trailing `! important` off a value. Whitespace is allowed between
/// `!` and `important`, and the keyword is ASCII case-insensitive.
fn strip_important(values: &[ComponentValue]) -> (bool, &[ComponentValue]) {
    let v = trim_whitespace(values);
    let last = match v.len().checked_sub(1) {
        Some(i) => i,
        None => return (false, v),
    };
    let is_important = match &v[last] {
        ComponentValue::Token(t) => {
            matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case("important"))
        }
        _ => false,
    };
    if !is_important {
        return (false, v);
    }
    let before = trim_whitespace(&v[..last]);
    match before.last() {
        Some(ComponentValue::Token(t)) if t.kind == TokenKind::Delim('!') => {
            let cut = before.len() - 1;
            (true, &before[..cut])
        }
        _ => (false, v),
    }
}

fn invalid(span: crate::syntax::Span, message: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::InvalidDeclaration,
        span,
        message: message.to_string(),
        spec_url: Some(DECLARATIONS_URL),
    }
}

fn limit_diag(tokens: &[crate::syntax::Token], message: &str) -> Diagnostic {
    let span = tokens.first().map_or(Default::default(), |t| t.span);
    Diagnostic::error(
        DiagnosticCode::LimitExceeded,
        span,
        message,
        Some("https://www.w3.org/TR/css-syntax-3/"),
    )
}
