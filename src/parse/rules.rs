// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Rule-list parsing (spec 11.2, 11.3; CSS Syntax 3, rule lists; CSS Nesting).
//!
//! Handles style rules (with nested style rules and nested `@media`), and the
//! at-rules `@media`, `@layer`, `@supports`, `@container`, `@property`,
//! `@keyframes`, and `@font-face`. Other at-rules are preserved as unknown.

use super::declarations::{parse_block_contents, within_math_depth};
use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::media::{parse_container_prelude, parse_media_queries};
use super::selectors::{
    contains_nesting, parse_selector_list, resolve_nesting, selector_size, span_of,
};
use super::supports::supports_condition;
use crate::ast::selector::SelectorList;
use crate::ast::{
    ContainerRule, DeclarationBlock, FontFaceRule, Keyframe, KeyframesRule, LayerRule, MediaRule,
    PropertyRule, Rule, ScopeRule, StyleRule, SupportsRule, UnknownAtRule,
};
use crate::limits::ParseLimits;
use crate::syntax::{trim_whitespace, BlockKind, ComponentValue, Span, Token, TokenKind};
use crate::values::serialize::format_number;

const MEDIA_URL: &str = "https://www.w3.org/TR/mediaqueries-5/";
const CASCADE_URL: &str = "https://www.w3.org/TR/css-cascade-5/#layering";
const CONTAINER_URL: &str = "https://www.w3.org/TR/css-contain-3/#container-queries";
const PROPERTIES_URL: &str =
    "https://www.w3.org/TR/css-properties-values-api-1/#defining-properties";
const KEYFRAMES_URL: &str = "https://www.w3.org/TR/css-animations-1/#keyframes";
const FONT_FACE_URL: &str = "https://www.w3.org/TR/css-fonts-4/#font-face-rule";
const SUPPORTS_URL: &str = "https://www.w3.org/TR/css-conditional-3/#at-supports";
const NESTING_URL: &str = "https://www.w3.org/TR/css-nesting-1/";

/// Consumes a list of rules. `depth` counts nested blocks.
pub(crate) fn consume_rule_list(
    values: &[ComponentValue],
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
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
            let (at, next) = consume_at_rule(values, i, name);
            i = next;
            rules.extend(at_rule(at, limits, diagnostics, depth));
            continue;
        }
        let (group, next) = consume_qualified_rule(values, i, limits, diagnostics, depth);
        rules.extend(group);
        i = next;
    }
    rules
}

/// Builds the rules for one at-rule. Returns nothing (with a diagnostic) if the
/// at-rule is malformed.
fn at_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) -> Vec<Rule> {
    let name = at.name.to_ascii_lowercase();
    match name.as_str() {
        "media" => media_rule(at, limits, diagnostics, depth)
            .map(Rule::Media)
            .into_iter()
            .collect(),
        "layer" => layer_rule(at, limits, diagnostics, depth)
            .map(Rule::Layer)
            .into_iter()
            .collect(),
        "supports" => supports_rule(at, limits, diagnostics, depth)
            .map(Rule::Supports)
            .into_iter()
            .collect(),
        "container" => container_rule(at, limits, diagnostics, depth)
            .map(Rule::Container)
            .into_iter()
            .collect(),
        "property" => property_rule(at, limits, diagnostics)
            .map(Rule::Property)
            .into_iter()
            .collect(),
        "keyframes" | "-webkit-keyframes" => keyframes_rule(at, limits, diagnostics)
            .map(Rule::Keyframes)
            .into_iter()
            .collect(),
        "scope" => scope_rule(at, limits, diagnostics, depth)
            .map(Rule::Scope)
            .into_iter()
            .collect(),
        "font-face" => font_face_rule(at, limits, diagnostics)
            .map(Rule::FontFace)
            .into_iter()
            .collect(),
        _ => {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::UnsupportedAtRule,
                at.span,
                format!("@{} is not supported yet; preserved but ignored", at.name),
                None,
            ));
            vec![Rule::Unknown(at)]
        }
    }
}

fn malformed(span: Span, message: &str, url: &'static str) -> Diagnostic {
    Diagnostic::error(DiagnosticCode::InvalidSelector, span, message, Some(url))
}

/// The block of an at-rule, or a diagnostic if it has none.
fn required_block(
    at: &UnknownAtRule,
    diagnostics: &mut Vec<Diagnostic>,
    url: &'static str,
) -> Option<Vec<ComponentValue>> {
    match &at.block {
        Some(block) => Some(block.clone()),
        None => {
            diagnostics.push(malformed(
                at.span,
                &format!("@{} needs a {{ }} block; dropped", at.name),
                url,
            ));
            None
        }
    }
}

/// Checks the nesting depth before descending into a block.
fn too_deep(
    limits: &ParseLimits,
    depth: usize,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if depth + 1 > limits.max_nesting_depth {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            span,
            "nesting limit exceeded; dropped",
            None,
        ));
        true
    } else {
        false
    }
}

fn media_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) -> Option<MediaRule> {
    let block = required_block(&at, diagnostics, MEDIA_URL)?;
    if too_deep(limits, depth, at.span, diagnostics) {
        return None;
    }
    let queries = match parse_media_queries(&at.prelude) {
        Ok(q) => q,
        Err(diag) => {
            diagnostics.push(diag);
            return None;
        }
    };
    let rules = consume_rule_list(&block, limits, diagnostics, depth + 1);
    Some(MediaRule {
        queries,
        rules,
        span: at.span,
    })
}

/// `@layer a, b;` (statement) or `@layer name { ... }` (block, possibly anonymous).
fn layer_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) -> Option<LayerRule> {
    let prelude = trim_whitespace(&at.prelude);
    match &at.block {
        None => {
            let names: Option<Vec<String>> =
                super::split_on(prelude, |v| v.is_token(&TokenKind::Comma))
                    .into_iter()
                    .map(layer_name)
                    .collect();
            match names {
                Some(names) if !names.is_empty() && names.iter().all(|n| !n.is_empty()) => {
                    Some(LayerRule {
                        names,
                        rules: None,
                        span: at.span,
                    })
                }
                _ => {
                    diagnostics.push(malformed(at.span, "invalid @layer statement", CASCADE_URL));
                    None
                }
            }
        }
        Some(block) => {
            let name = if prelude.is_empty() {
                String::new()
            } else {
                match layer_name(prelude) {
                    Some(n) => n,
                    None => {
                        diagnostics.push(malformed(at.span, "invalid layer name", CASCADE_URL));
                        return None;
                    }
                }
            };
            if too_deep(limits, depth, at.span, diagnostics) {
                return None;
            }
            let rules = consume_rule_list(block, limits, diagnostics, depth + 1);
            Some(LayerRule {
                names: vec![name],
                rules: Some(rules),
                span: at.span,
            })
        }
    }
}

/// A layer name: identifiers joined by dots (`a.b`).
fn layer_name(values: &[ComponentValue]) -> Option<String> {
    let values = trim_whitespace(values);
    if values.is_empty() {
        return Some(String::new());
    }
    let mut out = String::new();
    let mut expect_ident = true;
    for v in values {
        match v {
            ComponentValue::Token(t) => match (&t.kind, expect_ident) {
                (TokenKind::Ident(s), true) => {
                    out.push_str(s);
                    expect_ident = false;
                }
                (TokenKind::Delim('.'), false) => {
                    out.push('.');
                    expect_ident = true;
                }
                _ => return None,
            },
            _ => return None,
        }
    }
    if expect_ident {
        None
    } else {
        Some(out)
    }
}

fn supports_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) -> Option<SupportsRule> {
    let block = required_block(&at, diagnostics, SUPPORTS_URL)?;
    if too_deep(limits, depth, at.span, diagnostics) {
        return None;
    }
    let supported = supports_condition(&at.prelude, limits);
    let rules = consume_rule_list(&block, limits, diagnostics, depth + 1);
    Some(SupportsRule {
        supported,
        rules,
        span: at.span,
    })
}

fn container_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) -> Option<ContainerRule> {
    let block = required_block(&at, diagnostics, CONTAINER_URL)?;
    let (name, features) = match parse_container_prelude(&at.prelude) {
        Ok(parsed) => parsed,
        Err(diag) => {
            diagnostics.push(diag);
            return None;
        }
    };
    if too_deep(limits, depth, at.span, diagnostics) {
        return None;
    }
    let rules = consume_rule_list(&block, limits, diagnostics, depth + 1);
    Some(ContainerRule {
        name,
        features,
        rules,
        span: at.span,
    })
}

/// `@property --name { syntax; inherits; initial-value }` (spec 15.1).
fn property_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PropertyRule> {
    let name = match trim_whitespace(&at.prelude) {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(n) if n.starts_with("--") && n.len() > 2 => n.clone(),
            _ => {
                diagnostics.push(malformed(
                    at.span,
                    "@property needs a custom property name",
                    PROPERTIES_URL,
                ));
                return None;
            }
        },
        _ => {
            diagnostics.push(malformed(
                at.span,
                "@property needs a custom property name",
                PROPERTIES_URL,
            ));
            return None;
        }
    };
    let block = required_block(&at, diagnostics, PROPERTIES_URL)?;
    let decls = parse_block_contents(&block, limits, diagnostics);

    let mut syntax = None;
    let mut inherits = None;
    let mut initial = None;
    for decl in &decls {
        match decl.property.as_str() {
            "syntax" => match trim_whitespace(&decl.value) {
                [ComponentValue::Token(t)] => match &t.kind {
                    TokenKind::String(s) => syntax = Some(s.clone()),
                    _ => {
                        diagnostics.push(malformed(
                            decl.span,
                            "syntax must be a string",
                            PROPERTIES_URL,
                        ));
                        return None;
                    }
                },
                _ => {
                    diagnostics.push(malformed(
                        decl.span,
                        "syntax must be a string",
                        PROPERTIES_URL,
                    ));
                    return None;
                }
            },
            "inherits" => match trim_whitespace(&decl.value) {
                [ComponentValue::Token(t)] => match &t.kind {
                    TokenKind::Ident(s) if s.eq_ignore_ascii_case("true") => inherits = Some(true),
                    TokenKind::Ident(s) if s.eq_ignore_ascii_case("false") => {
                        inherits = Some(false)
                    }
                    _ => {
                        diagnostics.push(malformed(
                            decl.span,
                            "inherits must be true or false",
                            PROPERTIES_URL,
                        ));
                        return None;
                    }
                },
                _ => {
                    diagnostics.push(malformed(
                        decl.span,
                        "inherits must be true or false",
                        PROPERTIES_URL,
                    ));
                    return None;
                }
            },
            "initial-value" => initial = Some(decl.value.clone()),
            _ => {}
        }
    }
    let Some(inherits) = inherits else {
        diagnostics.push(malformed(
            at.span,
            "@property requires an inherits descriptor",
            PROPERTIES_URL,
        ));
        return None;
    };
    Some(PropertyRule {
        name,
        syntax,
        inherits,
        initial,
        span: at.span,
    })
}

/// `@keyframes name { from { } 50% { } to { } }`.
fn keyframes_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<KeyframesRule> {
    let name = match trim_whitespace(&at.prelude) {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(n) | TokenKind::String(n) => n.clone(),
            _ => {
                diagnostics.push(malformed(at.span, "@keyframes needs a name", KEYFRAMES_URL));
                return None;
            }
        },
        _ => {
            diagnostics.push(malformed(at.span, "@keyframes needs a name", KEYFRAMES_URL));
            return None;
        }
    };
    let block = required_block(&at, diagnostics, KEYFRAMES_URL)?;
    let mut keyframes = Vec::new();
    let mut start = 0;
    for (i, v) in block.iter().enumerate() {
        let ComponentValue::Block(b) = v else {
            continue;
        };
        if b.kind != BlockKind::Curly {
            continue;
        }
        let prelude = trim_whitespace(&block[start..i]);
        start = i + 1;
        let selectors: Option<Vec<String>> =
            super::split_on(prelude, |v| v.is_token(&TokenKind::Comma))
                .into_iter()
                .map(|part| keyframe_selector(trim_whitespace(part)))
                .collect();
        match selectors {
            Some(selectors) if !selectors.is_empty() => {
                let declarations = DeclarationBlock {
                    declarations: parse_block_contents(&b.value, limits, diagnostics),
                    span: b.span,
                };
                keyframes.push(Keyframe {
                    selectors,
                    declarations,
                });
            }
            _ => diagnostics.push(malformed(
                b.span,
                "invalid keyframe selector",
                KEYFRAMES_URL,
            )),
        }
    }
    Some(KeyframesRule {
        name,
        keyframes,
        span: at.span,
    })
}

/// `from`, `to`, or a percentage such as `50%`.
fn keyframe_selector(values: &[ComponentValue]) -> Option<String> {
    match values {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(s) if s.eq_ignore_ascii_case("from") => Some("from".to_string()),
            TokenKind::Ident(s) if s.eq_ignore_ascii_case("to") => Some("to".to_string()),
            TokenKind::Percentage(p) if (0.0..=100.0).contains(p) => {
                Some(format!("{}%", format_number(*p)))
            }
            _ => None,
        },
        _ => None,
    }
}

fn font_face_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<FontFaceRule> {
    if !at.prelude.iter().all(|v| v.is_whitespace()) {
        diagnostics.push(malformed(
            at.span,
            "@font-face takes no prelude",
            FONT_FACE_URL,
        ));
        return None;
    }
    let block = required_block(&at, diagnostics, FONT_FACE_URL)?;
    let declarations = parse_block_contents(&block, limits, diagnostics);
    Some(FontFaceRule {
        declarations: DeclarationBlock {
            declarations,
            span: at.span,
        },
        span: at.span,
    })
}

/// Consumes a top-level qualified rule. Returns the style rule followed by its
/// nested rules, or nothing if the rule is invalid (spec 12: an invalid selector
/// drops the whole rule).
fn consume_qualified_rule(
    values: &[ComponentValue],
    start: usize,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) -> (Vec<Rule>, usize) {
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
            return (Vec::new(), values.len());
        }
    };

    let block = match &values[block_index] {
        ComponentValue::Block(b) => b,
        _ => unreachable!("position matched a curly block"),
    };
    let prelude = trim_whitespace(&values[start..block_index]);
    let span: Span = span_of(&values[start..=block_index]);

    let selectors = match parse_selector_list(prelude, limits) {
        Ok(selectors) => selectors,
        Err(diag) => {
            diagnostics.push(diag);
            return (Vec::new(), block_index + 1);
        }
    };
    if contains_nesting(&selectors) {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidSelector,
            span,
            "'&' is only valid inside a nested rule",
            Some(NESTING_URL),
        ));
        return (Vec::new(), block_index + 1);
    }

    let (declarations, nested) =
        parse_style_block(&block.value, limits, diagnostics, depth, &selectors);
    let mut out = vec![Rule::Style(StyleRule {
        selectors,
        declarations: DeclarationBlock {
            declarations,
            span: block.span,
        },
        span,
    })];
    out.extend(nested);
    (out, block_index + 1)
}

/// Parses the contents of a style rule's block: declarations, nested style
/// rules, and nested `@media`. `parent` is the selector list of the enclosing
/// rule, used to resolve `&`.
fn parse_style_block(
    values: &[ComponentValue],
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
    parent: &SelectorList,
) -> (Vec<crate::ast::Declaration>, Vec<Rule>) {
    let mut declarations = Vec::new();
    let mut nested = Vec::new();
    if depth + 1 > limits.max_nesting_depth {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            span_of(values),
            "nesting limit exceeded; nested rules dropped",
            Some(NESTING_URL),
        ));
        return (declarations, nested);
    }

    let mut start = 0;
    for (i, v) in values.iter().enumerate() {
        if v.is_token(&TokenKind::Semicolon) {
            push_declaration(&values[start..i], limits, diagnostics, &mut declarations);
            start = i + 1;
        } else if let ComponentValue::Block(b) = v {
            if b.kind == BlockKind::Curly {
                let prelude = trim_whitespace(&values[start..i]);
                nested_rule(prelude, b, limits, diagnostics, depth, parent, &mut nested);
                start = i + 1;
            }
        }
    }
    push_declaration(&values[start..], limits, diagnostics, &mut declarations);
    (declarations, nested)
}

fn push_declaration(
    chunk: &[ComponentValue],
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    out: &mut Vec<crate::ast::Declaration>,
) {
    let chunk = trim_whitespace(chunk);
    if chunk.is_empty() {
        return;
    }
    if out.len() >= limits.max_declarations_per_block {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            span_of(chunk),
            "declaration count limit exceeded",
            None,
        ));
        return;
    }
    match super::declarations::parse_declaration(chunk) {
        Ok(decl) => {
            super::declarations::check_property_name(&decl, diagnostics);
            if within_math_depth(&decl, limits, diagnostics) {
                out.push(decl);
            }
        }
        Err(diag) => diagnostics.push(diag),
    }
}

/// Handles one nested rule: a style rule, or an `@media` that wraps declarations
/// and rules for the parent selectors.
fn nested_rule(
    prelude: &[ComponentValue],
    block: &crate::syntax::SimpleBlock,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
    parent: &SelectorList,
    out: &mut Vec<Rule>,
) {
    let span = span_of(prelude).join(block.span);
    if let Some(ComponentValue::Token(Token {
        kind: TokenKind::AtKeyword(name),
        ..
    })) = prelude.first()
    {
        if name.eq_ignore_ascii_case("media") {
            let queries = match parse_media_queries(&prelude[1..]) {
                Ok(q) => q,
                Err(diag) => {
                    diagnostics.push(diag);
                    return;
                }
            };
            let (declarations, mut rules) =
                parse_style_block(&block.value, limits, diagnostics, depth + 1, parent);
            let mut inner = vec![Rule::Style(StyleRule {
                selectors: parent.clone(),
                declarations: DeclarationBlock {
                    declarations,
                    span: block.span,
                },
                span,
            })];
            inner.append(&mut rules);
            out.push(Rule::Media(MediaRule {
                queries,
                rules: inner,
                span,
            }));
            return;
        }
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::UnsupportedAtRule,
            span,
            format!("@{name} is not supported inside a style rule"),
            Some(NESTING_URL),
        ));
        return;
    }

    let child = match parse_selector_list(prelude, limits) {
        Ok(list) => list,
        Err(diag) => {
            diagnostics.push(diag);
            return;
        }
    };
    let resolved = resolve_nesting(&child, parent);
    if selector_size(&resolved) > limits.max_resolved_selector_size {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            span,
            "nested selector list too large; nested rule dropped",
            Some(NESTING_URL),
        ));
        return;
    }
    let (declarations, rules) =
        parse_style_block(&block.value, limits, diagnostics, depth + 1, &resolved);
    out.push(Rule::Style(StyleRule {
        selectors: resolved,
        declarations: DeclarationBlock {
            declarations,
            span: block.span,
        },
        span,
    }));
    out.extend(rules);
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

/// `@scope (root)? (to (limit))? { rules }` (CSS Cascade 6).
fn scope_rule(
    at: UnknownAtRule,
    limits: &ParseLimits,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) -> Option<ScopeRule> {
    const SCOPE_URL: &str = "https://www.w3.org/TR/css-cascade-6/#scope-atrule";
    let block = required_block(&at, diagnostics, SCOPE_URL)?;
    let prelude = trim_whitespace(&at.prelude);
    let paren_selector = |v: Option<&ComponentValue>| -> Result<Option<SelectorList>, Diagnostic> {
        match v {
            Some(ComponentValue::Block(b)) if b.kind == BlockKind::Paren => {
                parse_selector_list(trim_whitespace(&b.value), limits).map(Some)
            }
            _ => Ok(None),
        }
    };
    let mut i = 0;
    let root = match paren_selector(prelude.first()) {
        Ok(root) => root,
        Err(diag) => {
            diagnostics.push(diag);
            return None;
        }
    };
    if root.is_some() {
        i = 1;
    }
    while i < prelude.len() && prelude[i].is_whitespace() {
        i += 1;
    }
    let mut limit = None;
    if i < prelude.len() {
        if !ident_word(prelude.get(i), "to") {
            diagnostics.push(malformed(at.span, "malformed @scope prelude", SCOPE_URL));
            return None;
        }
        i += 1;
        while i < prelude.len() && prelude[i].is_whitespace() {
            i += 1;
        }
        match paren_selector(prelude.get(i)) {
            Ok(Some(list)) => limit = Some(list),
            Ok(None) => {
                diagnostics.push(malformed(
                    at.span,
                    "@scope limit needs a selector",
                    SCOPE_URL,
                ));
                return None;
            }
            Err(diag) => {
                diagnostics.push(diag);
                return None;
            }
        }
        i += 1;
        if prelude[i..].iter().any(|v| !v.is_whitespace()) {
            diagnostics.push(malformed(at.span, "malformed @scope prelude", SCOPE_URL));
            return None;
        }
    }
    if too_deep(limits, depth, at.span, diagnostics) {
        return None;
    }
    let rules = consume_rule_list(&block, limits, diagnostics, depth + 1);
    Some(ScopeRule {
        root,
        limit,
        rules,
        span: at.span,
    })
}

/// True if `v` is the identifier `word` (ASCII case-insensitive).
fn ident_word(v: Option<&ComponentValue>, word: &str) -> bool {
    matches!(v, Some(ComponentValue::Token(t))
        if matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case(word)))
}
