// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `@supports` condition evaluation (spec 16.4).
//!
//! A condition depends only on this engine's capabilities, so it is evaluated
//! once, when the rule is parsed. Support is checked against the real property
//! registry and grammar, and against the real selector parser, rather than a
//! list of strings (spec 16.4: "do not hardcode browser-like support strings").
//!
//! Supported forms: `(property: value)`, `selector(...)`, `not`, `and`, `or`,
//! and parentheses. `font-tech()` and `font-format()` are recognized but report
//! false, because font features are the host's job (spec 3). Anything else is
//! general-enclosed, which is false.

use crate::limits::ParseLimits;
use crate::properties::{lookup, parse_property_value, spec};
use crate::syntax::component::BlockKind;
use crate::syntax::{trim_whitespace, ComponentValue, TokenKind};

/// Evaluates a `@supports` prelude. Returns `false` for an invalid condition.
pub(crate) fn supports_condition(values: &[ComponentValue], limits: &ParseLimits) -> bool {
    eval(values, limits, 0).unwrap_or(false)
}

fn eval(values: &[ComponentValue], limits: &ParseLimits, depth: usize) -> Option<bool> {
    if depth > limits.max_nesting_depth {
        return None;
    }
    let values = trim_whitespace(values);
    if values.is_empty() {
        return None;
    }
    if ident_is(values.first(), "not") {
        return eval_term(&values[1..], limits, depth).map(|v| !v);
    }

    let has_and = values.iter().any(|v| ident_is(Some(v), "and"));
    let has_or = values.iter().any(|v| ident_is(Some(v), "or"));
    if has_and && has_or {
        return None;
    }
    let joiner = if has_and {
        "and"
    } else if has_or {
        "or"
    } else {
        return eval_term(values, limits, depth);
    };

    let mut result: Option<bool> = None;
    for part in crate::parse::split_on(values, |v| ident_is(Some(v), joiner)) {
        let term = eval_term(trim_whitespace(part), limits, depth)?;
        result = Some(match (result, joiner) {
            (None, _) => term,
            (Some(acc), "and") => acc && term,
            (Some(acc), _) => acc || term,
        });
    }
    result
}

/// One term: a parenthesized condition or declaration, or a function.
fn eval_term(values: &[ComponentValue], limits: &ParseLimits, depth: usize) -> Option<bool> {
    let values = trim_whitespace(values);
    match values {
        [ComponentValue::Block(b)] if b.kind == BlockKind::Paren => {
            let inner = trim_whitespace(&b.value);
            let looks_like_condition = inner.len() > 1
                && (ident_is(inner.first(), "not")
                    || inner
                        .iter()
                        .any(|v| ident_is(Some(v), "and") || ident_is(Some(v), "or"))
                    || matches!(inner.first(), Some(ComponentValue::Block(_))));
            if looks_like_condition {
                eval(inner, limits, depth + 1)
            } else {
                Some(declaration_supported(inner))
            }
        }
        [ComponentValue::Function(f)] => match f.name.to_ascii_lowercase().as_str() {
            "selector" => Some(selector_supported(&f.value, limits)),
            "font-tech" | "font-format" => Some(false),
            _ => Some(false),
        },
        _ => Some(false),
    }
}

/// `property: value`. A value containing `var()` is treated as supported (CSS
/// Conditional 3: it cannot be checked until computed time).
fn declaration_supported(values: &[ComponentValue]) -> bool {
    let name = match values.first() {
        Some(ComponentValue::Token(t)) => match &t.kind {
            TokenKind::Ident(n) => n.to_ascii_lowercase(),
            _ => return false,
        },
        _ => return false,
    };
    let rest = trim_whitespace(&values[1..]);
    let value = match rest {
        [ComponentValue::Token(t), tail @ ..] if t.kind == TokenKind::Colon => {
            trim_whitespace(tail)
        }
        _ => return false,
    };
    if value.is_empty() {
        return false;
    }
    if crate::style::custom::contains_var(value) {
        return true;
    }
    if name.starts_with("--") {
        return true;
    }
    match lookup(&name) {
        Some(id) => parse_property_value(spec(id), value).is_ok(),
        None => {
            // Shorthands are not in the registry, so check them separately.
            crate::properties::shorthand_index(&name)
                .is_some_and(|sid| crate::properties::shorthand::expand(sid, value).is_ok())
        }
    }
}

/// `selector(...)`: the argument must parse as a selector list.
fn selector_supported(values: &[ComponentValue], limits: &ParseLimits) -> bool {
    crate::parse::selectors::parse_selector_list(trim_whitespace(values), limits).is_ok()
}

fn ident_is(v: Option<&ComponentValue>, word: &str) -> bool {
    matches!(v, Some(ComponentValue::Token(t))
        if matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case(word)))
}
