// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Rule-list parsing (spec 11.2, 11.3; CSS Syntax 3 section 5.4.1 and 5.4.3).

use super::declarations::parse_block_contents;
use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::selectors::{parse_selector_list, span_of};
use crate::ast::{DeclarationBlock, Rule, StyleRule, UnknownAtRule};
use crate::limits::ParseLimits;
use crate::syntax::{trim_whitespace, BlockKind, ComponentValue, Span, Token, TokenKind};

/// Consumes a top-level list of rules.
pub(crate) fn consume_rule_list(
    values: &[ComponentValue],
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Rule> {
    let mut rules = Vec::new();
    let mut i = 0;
    while i < values.len() {
        let v = &values[i];
        if v.is_whitespace() || v.is_token(&TokenKind::Cdo) || v.is_token(&TokenKind::Cdc) {
            i += 1;
            continue;
        }
        if rules.len() >= limits.max_rules_per_sheet {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::LimitExceeded,
                v.span(),
                "rule count limit exceeded; remaining rules dropped",
                None,
            ));
            break;
        }
        if let ComponentValue::Token(Token {
            kind: TokenKind::AtKeyword(name),
            ..
        }) = v
        {
            let (rule, next) = consume_at_rule(values, i, name);
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::UnsupportedAtRule,
                rule.span,
                format!("@{} is not supported yet; preserved but ignored", rule.name),
                None,
            ));
            rules.push(Rule::Unknown(rule));
            i = next;
            continue;
        }
        let (rule, next) = consume_qualified_rule(values, i, limits, diagnostics);
        if let Some(rule) = rule {
            rules.push(Rule::Style(rule));
        }
        i = next;
    }
    rules
}

/// Consumes an at-rule that starts at `start`. Ends at `;` or at a `{}` block.
fn consume_at_rule(values: &[ComponentValue], start: usize, name: &str) -> (UnknownAtRule, usize) {
    let mut prelude = Vec::new();
    let mut i = start + 1;
    while i < values.len() {
        match &values[i] {
            ComponentValue::Token(t) if t.kind == TokenKind::Semicolon => {
                let span = values[start].span().join(t.span);
                return (
                    UnknownAtRule {
                        name: name.to_string(),
                        prelude,
                        block: None,
                        span,
                    },
                    i + 1,
                );
            }
            ComponentValue::Block(b) if b.kind == BlockKind::Curly => {
                let span = values[start].span().join(b.span);
                return (
                    UnknownAtRule {
                        name: name.to_string(),
                        prelude,
                        block: Some(b.value.clone()),
                        span,
                    },
                    i + 1,
                );
            }
            other => prelude.push(other.clone()),
        }
        i += 1;
    }
    let span = values[start].span().join(values[values.len() - 1].span());
    (
        UnknownAtRule {
            name: name.to_string(),
            prelude,
            block: None,
            span,
        },
        values.len(),
    )
}

/// Consumes a qualified rule. A rule with an invalid selector is dropped
/// entirely (spec 12), so the function returns `None` for it.
fn consume_qualified_rule(
    values: &[ComponentValue],
    start: usize,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Option<StyleRule>, usize) {
    let block_index = values[start..]
        .iter()
        .position(|v| matches!(v, ComponentValue::Block(b) if b.kind == BlockKind::Curly))
        .map(|p| start + p);

    let block_index = match block_index {
        Some(i) => i,
        None => {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidSelector,
                span_of(&values[start..]),
                "rule has no { } block; dropped",
                None,
            ));
            return (None, values.len());
        }
    };

    let block = match &values[block_index] {
        ComponentValue::Block(b) => b,
        _ => unreachable!("position matched a curly block"),
    };
    let prelude = trim_whitespace(&values[start..block_index]);
    let span: Span = span_of(&values[start..=block_index]);

    match parse_selector_list(prelude, limits) {
        Ok(selectors) => {
            let declarations = parse_block_contents(&block.value, limits, diagnostics);
            (
                Some(StyleRule {
                    selectors,
                    declarations: DeclarationBlock {
                        declarations,
                        span: block.span,
                    },
                    span,
                }),
                block_index + 1,
            )
        }
        Err(diag) => {
            diagnostics.push(diag);
            (None, block_index + 1)
        }
    }
}
