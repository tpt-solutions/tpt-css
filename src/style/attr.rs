// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Typed `attr()` (CSS Values 5 section 3.6). `attr()` reads an attribute of the
//! element at computed-value time.
//!
//! A plain `attr(name)` in `content` stays a content item, read by the resolver
//! (see `values::ContentItem`). Every other form is substituted into the
//! declaration before it is parsed, as `var()` is:
//!
//! - `attr(name)` and `attr(name string)` give the attribute value as a string.
//! - `attr(name px)` (any length, angle, or time unit) gives the attribute value
//!   as a number with that unit. The value must be a plain number.
//! - `attr(name type(<syntax>))` gives the attribute value parsed as that syntax.
//!   The syntaxes are `<number>`, `<integer>`, `<length>`, `<percentage>`,
//!   `<length-percentage>`, `<color>`, and `<time>`.
//!
//! A missing attribute, or a value that does not match the type, takes the
//! fallback (the part after the first comma). Without a fallback, a string is
//! empty, and any other type makes the declaration invalid at computed-value time.

use crate::limits::ParseLimits;
use crate::style::custom::syntax_matches;
use crate::syntax::component::build_component_values;
use crate::syntax::tokenizer::tokenize;
use crate::syntax::{trim_whitespace, ComponentValue, Function, SimpleBlock, SourceId, TokenKind};

/// The units that `attr(name <unit>)` accepts.
const UNITS: &[&str] = &[
    "px", "em", "rem", "vw", "vh", "vmin", "vmax", "cm", "mm", "q", "in", "pt", "pc", "deg",
    "grad", "rad", "turn", "s", "ms",
];

/// The syntaxes that `attr(name type(<syntax>))` accepts.
const SYNTAXES: &[&str] = &[
    "<number>",
    "<integer>",
    "<length>",
    "<percentage>",
    "<length-percentage>",
    "<color>",
    "<time>",
    "<angle>",
];

/// The type part of an `attr()` head.
#[derive(Debug, Clone, PartialEq)]
enum AttrType {
    /// `string`, the default.
    String,
    /// A unit, lowercase, from [`UNITS`].
    Unit(&'static str),
    /// A syntax, from [`SYNTAXES`].
    Syntax(&'static str),
    /// `url`: the attribute value as a URL, quoted so that any text is allowed.
    Url,
}

/// Whether any `attr()` appears in `values`, at any depth.
pub(crate) fn contains_attr(values: &[ComponentValue]) -> bool {
    values.iter().any(|v| match v {
        ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("attr") => true,
        ComponentValue::Function(f) => contains_attr(&f.value),
        ComponentValue::Block(b) => contains_attr(&b.value),
        ComponentValue::Token(_) => false,
    })
}

/// Whether a declaration needs `attr()` substitution at computed-value time: any
/// `attr()` outside `content`, and any typed `attr()` inside it.
pub(crate) fn needs_substitution(values: &[ComponentValue], is_content: bool) -> bool {
    values.iter().any(|v| match v {
        ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("attr") => {
            !is_content || is_typed(&f.value)
        }
        ComponentValue::Function(f) => needs_substitution(&f.value, is_content),
        ComponentValue::Block(b) => needs_substitution(&b.value, is_content),
        ComponentValue::Token(_) => false,
    })
}

/// Replaces every `attr()` that needs substitution with the attribute's value.
/// Returns `None` when the declaration is invalid at computed-value time.
pub(crate) fn substitute(
    values: &[ComponentValue],
    attrs: &dyn Fn(&str) -> Option<String>,
    is_content: bool,
    limits: &ParseLimits,
) -> Option<Vec<ComponentValue>> {
    let mut out = Vec::with_capacity(values.len());
    for v in values {
        match v {
            ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("attr") => {
                if is_content && !is_typed(&f.value) {
                    out.push(v.clone());
                } else {
                    out.extend(resolve(&f.value, attrs, is_content, limits)?);
                }
            }
            ComponentValue::Function(f) => {
                let value = substitute(&f.value, attrs, is_content, limits)?;
                out.push(ComponentValue::Function(Function {
                    name: f.name.clone(),
                    value,
                    span: f.span,
                }));
            }
            ComponentValue::Block(b) => {
                let value = substitute(&b.value, attrs, is_content, limits)?;
                out.push(ComponentValue::Block(SimpleBlock {
                    kind: b.kind,
                    value,
                    span: b.span,
                }));
            }
            ComponentValue::Token(_) => out.push(v.clone()),
        }
    }
    Some(out)
}

/// Whether the head of an `attr()` (before its first top-level comma) has a type
/// after the name.
fn is_typed(args: &[ComponentValue]) -> bool {
    args.iter()
        .take_while(|v| !v.is_token(&TokenKind::Comma))
        .filter(|v| !v.is_whitespace())
        .count()
        > 1
}

/// The tokens of one `attr()` call, with its fallback used when needed.
fn resolve(
    args: &[ComponentValue],
    attrs: &dyn Fn(&str) -> Option<String>,
    is_content: bool,
    limits: &ParseLimits,
) -> Option<Vec<ComponentValue>> {
    let comma = args.iter().position(|v| v.is_token(&TokenKind::Comma));
    let (head, fallback) = match comma {
        Some(i) => (&args[..i], Some(&args[i + 1..])),
        None => (args, None),
    };
    let (name, ty) = parse_head(head)?;
    if let Some(text) = attrs(&name) {
        if let Some(tokens) = attr_tokens(&text, &ty, limits) {
            // `content` takes strings, so a typed value is given as its text.
            if is_content {
                return tokenize_text(&quote(text.trim()), limits);
            }
            return Some(tokens);
        }
    }
    match fallback {
        Some(fallback) => substitute(trim_whitespace(fallback), attrs, is_content, limits),
        None if ty == AttrType::String || is_content => tokenize_text(&quote(""), limits),
        None => None,
    }
}

/// The attribute name (lowercase) and its type from the head of an `attr()`.
fn parse_head(head: &[ComponentValue]) -> Option<(String, AttrType)> {
    let parts: Vec<&ComponentValue> = head.iter().filter(|v| !v.is_whitespace()).collect();
    let ComponentValue::Token(name) = parts.first()? else {
        return None;
    };
    let TokenKind::Ident(name) = &name.kind else {
        return None;
    };
    let name = name.to_ascii_lowercase();
    let ty = match &parts[1..] {
        [] => AttrType::String,
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(word) => {
                let word = word.to_ascii_lowercase();
                if word == "string" {
                    AttrType::String
                } else if word == "url" {
                    AttrType::Url
                } else {
                    AttrType::Unit(UNITS.iter().copied().find(|u| *u == word)?)
                }
            }
            _ => return None,
        },
        [ComponentValue::Function(f)] if f.name.eq_ignore_ascii_case("type") => {
            let text = crate::values::raw::serialize_components(&f.value);
            AttrType::Syntax(SYNTAXES.iter().copied().find(|s| *s == text.trim())?)
        }
        _ => return None,
    };
    Some((name, ty))
}

/// The tokens for an attribute value of type `ty`, or `None` if it does not fit.
fn attr_tokens(text: &str, ty: &AttrType, limits: &ParseLimits) -> Option<Vec<ComponentValue>> {
    let text = text.trim();
    match ty {
        AttrType::String => tokenize_text(&quote(text), limits),
        AttrType::Url => tokenize_text(&format!("url({})", quote(text)), limits),
        AttrType::Unit(unit) => {
            // The value must be a plain number before the unit is added.
            let plain = tokenize_text(text, limits)?;
            match trim_whitespace(&plain) {
                [ComponentValue::Token(t)] if matches!(t.kind, TokenKind::Number { .. }) => {}
                _ => return None,
            }
            let tokens = tokenize_text(&format!("{text}{unit}"), limits)?;
            match trim_whitespace(&tokens) {
                [ComponentValue::Token(t)] if matches!(t.kind, TokenKind::Dimension { .. }) => {
                    Some(tokens)
                }
                _ => None,
            }
        }
        AttrType::Syntax(syntax) => {
            let tokens = tokenize_text(text, limits)?;
            if syntax_ok(syntax, &tokens) {
                Some(trim_whitespace(&tokens).to_vec())
            } else {
                None
            }
        }
    }
}

/// Whether `tokens` is one value of `syntax`. `<time>` and `<angle>` are single
/// dimension tokens with a unit of their own. The other syntaxes use the checks
/// that custom properties use.
fn syntax_ok(syntax: &str, tokens: &[ComponentValue]) -> bool {
    let units: &[&str] = match syntax {
        "<time>" => &["s", "ms"],
        "<angle>" => &["deg", "grad", "rad", "turn"],
        _ => return syntax_matches(Some(syntax), tokens),
    };
    matches!(
        trim_whitespace(tokens),
        [ComponentValue::Token(t)] if matches!(
            &t.kind,
            TokenKind::Dimension { unit, .. }
                if units.iter().any(|u| unit.eq_ignore_ascii_case(u))
        )
    )
}

/// Tokenizes `text` as component values. The caller checks the shape it needs.
fn tokenize_text(text: &str, limits: &ParseLimits) -> Option<Vec<ComponentValue>> {
    let tokenized = tokenize(SourceId(0), text, limits);
    if tokenized.limit_exceeded {
        return None;
    }
    build_component_values(&tokenized.tokens, limits.max_nesting_depth).ok()
}

/// `text` as a quoted CSS string, with the characters that need escaping.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\a "),
            '\r' => out.push_str("\\d "),
            '\u{c}' => out.push_str("\\c "),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
