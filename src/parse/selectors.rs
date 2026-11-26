// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector parsing (spec 12). Covers Tier 0 selectors, `:root`, `:hover`,
//! `:active`, `:focus`, and the descendant and child combinators.
//!
//! Anything else valid in CSS but not implemented yet returns an
//! `UnsupportedSelector` diagnostic. The rule is dropped, as spec 12 requires
//! for invalid selectors, but the diagnostic says why.

use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::split_on;
use crate::ast::selector::{
    AttributeMatcher, AttributeOperator, AttributeSelector, Combinator, ComplexSelector,
    CompoundSelector, PseudoClass, SelectorList, SimpleSelector,
};
use crate::limits::ParseLimits;
use crate::selectors::specificity::Specificity;
use crate::syntax::{trim_whitespace, ComponentValue, Span, TokenKind};

const SELECTORS_URL: &str = "https://www.w3.org/TR/selectors-4/";

/// Parses a selector list from already-trimmed component values.
pub(crate) fn parse_selector_list(
    values: &[ComponentValue],
    limits: &ParseLimits,
) -> Result<SelectorList, Diagnostic> {
    let mut selectors = Vec::new();
    for chunk in split_on(values, |v| v.is_token(&TokenKind::Comma)) {
        selectors.push(parse_complex(trim_whitespace(chunk), limits)?);
    }
    if selectors.is_empty() {
        return Err(invalid(span_of(values), "empty selector"));
    }
    Ok(SelectorList { selectors })
}

fn parse_complex(
    values: &[ComponentValue],
    limits: &ParseLimits,
) -> Result<ComplexSelector, Diagnostic> {
    if values.is_empty() {
        return Err(invalid(Span::default(), "empty selector"));
    }
    let span = span_of(values);
    let mut compounds: Vec<CompoundSelector> = Vec::new();
    let mut combinators: Vec<Combinator> = Vec::new();
    let mut pending: Option<Combinator> = None;
    let mut i = 0;

    while i < values.len() {
        let v = &values[i];
        if v.is_whitespace() {
            if pending.is_none() {
                pending = Some(Combinator::Descendant);
            }
            i += 1;
            continue;
        }
        if v.is_token(&TokenKind::Delim('>')) {
            if compounds.is_empty() || pending == Some(Combinator::Child) {
                return Err(invalid(v.span(), "misplaced '>' combinator"));
            }
            pending = Some(Combinator::Child);
            i += 1;
            continue;
        }
        if v.is_token(&TokenKind::Delim('+')) || v.is_token(&TokenKind::Delim('~')) {
            return Err(unsupported(
                v.span(),
                "sibling combinators are not supported yet",
            ));
        }
        if !compounds.is_empty() {
            combinators.push(pending.take().unwrap_or(Combinator::Descendant));
        }
        let (compound, next) = parse_compound(values, i)?;
        compounds.push(compound);
        if compounds.len() > limits.max_selector_complexity {
            return Err(Diagnostic::error(
                DiagnosticCode::LimitExceeded,
                span,
                "selector complexity limit exceeded",
                Some(SELECTORS_URL),
            ));
        }
        i = next;
    }

    if compounds.is_empty() {
        return Err(invalid(span, "selector has no compound selector"));
    }
    if pending.is_some() {
        return Err(invalid(span, "selector ends with a combinator"));
    }

    let mut selector = ComplexSelector {
        compounds,
        combinators,
        specificity: Specificity::default(),
        span,
    };
    selector.specificity = Specificity::of_complex(&selector);
    Ok(selector)
}

/// Parses one compound selector starting at `start`. Returns the compound and
/// the index of the first value after it.
fn parse_compound(
    values: &[ComponentValue],
    start: usize,
) -> Result<(CompoundSelector, usize), Diagnostic> {
    let mut simple = Vec::new();
    let mut i = start;

    while i < values.len() {
        let v = &values[i];
        if v.is_whitespace() || v.is_token(&TokenKind::Delim('>')) {
            break;
        }
        match v {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::Delim('*') => {
                    simple.push(SimpleSelector::Universal);
                    i += 1;
                }
                TokenKind::Ident(name) => {
                    if i != start {
                        return Err(invalid(t.span, "type selector must come first"));
                    }
                    if values
                        .get(i + 1)
                        .is_some_and(|n| n.is_token(&TokenKind::Delim('|')))
                    {
                        return Err(unsupported(
                            t.span,
                            "namespace prefixes are not supported yet",
                        ));
                    }
                    simple.push(SimpleSelector::Type(name.clone()));
                    i += 1;
                }
                TokenKind::Hash { value, is_id } => {
                    if !is_id {
                        return Err(invalid(t.span, "hash selector is not a valid identifier"));
                    }
                    simple.push(SimpleSelector::Id(value.clone()));
                    i += 1;
                }
                TokenKind::Delim('.') => match values.get(i + 1) {
                    Some(ComponentValue::Token(next)) => match &next.kind {
                        TokenKind::Ident(name) => {
                            simple.push(SimpleSelector::Class(name.clone()));
                            i += 2;
                        }
                        _ => return Err(invalid(t.span, "expected class name after '.'")),
                    },
                    _ => return Err(invalid(t.span, "expected class name after '.'")),
                },
                TokenKind::Colon => match values.get(i + 1) {
                    Some(ComponentValue::Token(next)) => match &next.kind {
                        TokenKind::Colon => {
                            return Err(unsupported(
                                t.span,
                                "pseudo-element selectors are not supported yet",
                            ))
                        }
                        TokenKind::Ident(name) => {
                            simple
                                .push(SimpleSelector::PseudoClass(pseudo_class(name, next.span)?));
                            i += 2;
                        }
                        _ => return Err(invalid(t.span, "expected pseudo-class name")),
                    },
                    Some(ComponentValue::Function(f)) => {
                        return Err(unsupported(
                            f.span,
                            format!("functional pseudo-class :{}() is not supported yet", f.name),
                        ))
                    }
                    _ => return Err(invalid(t.span, "expected pseudo-class name")),
                },
                _ => return Err(invalid(t.span, "unexpected token in selector")),
            },
            ComponentValue::Block(b) if b.kind == crate::syntax::BlockKind::Square => {
                simple.push(SimpleSelector::Attribute(parse_attribute(
                    &b.value, b.span,
                )?));
                i += 1;
            }
            ComponentValue::Function(f) => {
                return Err(unsupported(
                    f.span,
                    format!("functional selector {}() is not supported yet", f.name),
                ))
            }
            other => return Err(invalid(other.span(), "unexpected block in selector")),
        }
    }

    if simple.is_empty() {
        return Err(invalid(
            span_of(&values[start..i.max(start)]),
            "empty compound selector",
        ));
    }
    Ok((CompoundSelector { simple }, i))
}

fn pseudo_class(name: &str, span: Span) -> Result<PseudoClass, Diagnostic> {
    match name.to_ascii_lowercase().as_str() {
        "root" => Ok(PseudoClass::Root),
        "hover" => Ok(PseudoClass::Hover),
        "active" => Ok(PseudoClass::Active),
        "focus" => Ok(PseudoClass::Focus),
        _ => Err(unsupported(
            span,
            format!("pseudo-class :{name} is not supported yet"),
        )),
    }
}

/// Parses the inside of `[ ... ]`: `name`, or `name = value`.
fn parse_attribute(values: &[ComponentValue], span: Span) -> Result<AttributeSelector, Diagnostic> {
    let v = trim_whitespace(values);
    let name = match v.first() {
        Some(ComponentValue::Token(t)) => match &t.kind {
            TokenKind::Ident(n) => n.clone(),
            _ => return Err(invalid(span, "expected attribute name")),
        },
        _ => return Err(invalid(span, "expected attribute name")),
    };

    let rest = trim_whitespace(&v[1..]);
    if rest.is_empty() {
        return Ok(AttributeSelector {
            name,
            matcher: None,
        });
    }
    match &rest[0] {
        ComponentValue::Token(t) if t.kind == TokenKind::Delim('=') => {
            let value_part = trim_whitespace(&rest[1..]);
            let value = match value_part {
                [ComponentValue::Token(t)] => match &t.kind {
                    TokenKind::Ident(s) | TokenKind::String(s) => s.clone(),
                    _ => {
                        return Err(invalid(
                            t.span,
                            "attribute value must be an identifier or string",
                        ))
                    }
                },
                _ => return Err(invalid(span, "expected a single attribute value")),
            };
            Ok(AttributeSelector {
                name,
                matcher: Some(AttributeMatcher {
                    operator: AttributeOperator::Equals,
                    value,
                }),
            })
        }
        ComponentValue::Token(t)
            if matches!(t.kind, TokenKind::Delim('~' | '|' | '^' | '$' | '*')) =>
        {
            Err(unsupported(
                t.span,
                "attribute operators other than '=' are not supported yet",
            ))
        }
        other => Err(invalid(other.span(), "malformed attribute selector")),
    }
}

fn invalid(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidSelector,
        span,
        message,
        Some(SELECTORS_URL),
    )
}

fn unsupported(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::UnsupportedSelector,
        span,
        message,
        Some(SELECTORS_URL),
    )
}

/// Span covering all values, or an empty span for an empty slice.
pub(crate) fn span_of(values: &[ComponentValue]) -> Span {
    match (values.first(), values.last()) {
        (Some(a), Some(b)) => a.span().join(b.span()),
        _ => Span::default(),
    }
}
