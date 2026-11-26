// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Custom properties and `var()` substitution (spec 13.7, 14.6), and registered
//! custom properties from `@property` (CSS Properties and Values).
//!
//! Custom properties cascade like other properties. Their values stay as tokens
//! until computed-value time, when each one is resolved with `var()`
//! substitution. A registered property also has an `inherits` flag, an
//! optional initial value, and a `syntax` that its computed value must match.
//!
//! Cycles are detected with a stack of names being resolved. A property that
//! depends on itself is invalid at computed-value time, and so is every
//! property in the cycle. Substitution depth is bounded by `max_var_depth`
//! (spec 19.2, 20.3).

use std::collections::{BTreeMap, BTreeSet};

use crate::properties::{CssWide, SpecifiedValue};
use crate::syntax::{ComponentValue, Function, SimpleBlock, TokenKind};
use crate::values::color::parse_color;
use crate::values::length::parse_length_percentage;
use crate::values::Value;

/// A resolved custom property: its tokens after `var()` substitution.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomPropertyValue {
    /// The value tokens.
    pub tokens: Vec<ComponentValue>,
}

impl CustomPropertyValue {
    /// The value as CSS text, for debugging and tests.
    pub fn text(&self) -> String {
        components_text(&self.tokens)
    }
}

/// A `@property` registration.
#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredProperty {
    /// The `syntax` descriptor, or `None` for `*` (any value).
    pub syntax: Option<String>,
    /// Whether the property inherits.
    pub inherits: bool,
    /// The initial value tokens, if declared.
    pub initial: Option<Vec<ComponentValue>>,
}

/// A `var()` substitution failed: the variable is missing or invalid and has no
/// fallback, or the depth limit was reached. The property becomes invalid at
/// computed-value time (spec 13.7 step 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarError;

/// Supplies the value of a named custom property during substitution. `None`
/// means the property is undefined or invalid, so the fallback is used.
pub trait VarResolver {
    /// Resolves `name` at substitution depth `depth`. The returned tokens are
    /// already fully substituted.
    fn resolve_var(&mut self, name: &str, depth: usize) -> Option<Vec<ComponentValue>>;
}

/// Resolves variables from a finished map of custom properties.
pub struct MapResolver<'a>(pub &'a BTreeMap<String, CustomPropertyValue>);

impl VarResolver for MapResolver<'_> {
    fn resolve_var(&mut self, name: &str, _depth: usize) -> Option<Vec<ComponentValue>> {
        self.0.get(name).map(|v| v.tokens.clone())
    }
}

/// Resolves the custom properties of one element.
struct CustomResolver<'a> {
    declared: &'a BTreeMap<String, SpecifiedValue>,
    parent: Option<&'a BTreeMap<String, CustomPropertyValue>>,
    registered: &'a BTreeMap<String, RegisteredProperty>,
    memo: BTreeMap<String, Option<Vec<ComponentValue>>>,
    stack: Vec<String>,
    max_depth: usize,
}

impl CustomResolver<'_> {
    fn inherited(&self, name: &str) -> Option<Vec<ComponentValue>> {
        self.parent
            .and_then(|p| p.get(name))
            .map(|v| v.tokens.clone())
    }

    /// The registered initial value of `name`, if it has one.
    fn initial(&self, name: &str) -> Option<Vec<ComponentValue>> {
        self.registered.get(name).and_then(|r| r.initial.clone())
    }

    /// The value `name` takes when it is not inherited: the registered initial
    /// value, or invalid for an unregistered property.
    fn not_inherited(&self, name: &str) -> Option<Vec<ComponentValue>> {
        match self.registered.get(name) {
            Some(r) if !r.inherits => self.initial(name),
            _ => self.inherited(name),
        }
    }

    /// The computed tokens of `name`, or `None` if it is invalid or undefined.
    fn value(&mut self, name: &str, depth: usize) -> Option<Vec<ComponentValue>> {
        if let Some(cached) = self.memo.get(name) {
            return cached.clone();
        }
        if depth > self.max_depth || self.stack.iter().any(|s| s == name) {
            // A cycle, or too deep: invalid at computed-value time.
            return None;
        }
        self.stack.push(name.to_string());
        let declared = self.declared;
        let raw = match declared.get(name) {
            None => self.not_inherited(name),
            Some(SpecifiedValue::CssWide(CssWide::Initial)) => self.initial(name),
            Some(SpecifiedValue::CssWide(CssWide::Inherit)) => self.inherited(name),
            Some(SpecifiedValue::CssWide(_)) => self.not_inherited(name),
            Some(SpecifiedValue::Unparsed { tokens, .. }) => {
                let max = self.max_depth;
                substitute(tokens, self, depth, max).ok()
            }
            Some(SpecifiedValue::Value(_)) => None,
        };
        let result = match (raw, self.registered.get(name)) {
            // A registered value must match its syntax, or the initial value is used.
            (Some(tokens), Some(reg)) => {
                if syntax_matches(reg.syntax.as_deref(), &tokens) {
                    Some(tokens)
                } else {
                    self.initial(name)
                }
            }
            (other, _) => other,
        };
        self.stack.pop();
        self.memo.insert(name.to_string(), result.clone());
        result
    }
}

impl VarResolver for CustomResolver<'_> {
    fn resolve_var(&mut self, name: &str, depth: usize) -> Option<Vec<ComponentValue>> {
        self.value(name, depth + 1)
    }
}

/// Resolves all custom properties for an element.
///
/// `declared` holds the cascaded custom properties, and `parent` holds the
/// parent's resolved ones. `registered` holds the `@property` registrations.
/// Properties that are invalid or undefined are absent from the result.
pub fn resolve_customs(
    declared: &BTreeMap<String, SpecifiedValue>,
    parent: Option<&BTreeMap<String, CustomPropertyValue>>,
    registered: &BTreeMap<String, RegisteredProperty>,
    max_depth: usize,
) -> BTreeMap<String, CustomPropertyValue> {
    let mut names: BTreeSet<String> = declared.keys().cloned().collect();
    if let Some(p) = parent {
        names.extend(p.keys().cloned());
    }
    names.extend(registered.keys().cloned());
    let mut resolver = CustomResolver {
        declared,
        parent,
        registered,
        memo: BTreeMap::new(),
        stack: Vec::new(),
        max_depth,
    };
    let mut out = BTreeMap::new();
    for name in names {
        if let Some(tokens) = resolver.value(&name, 0) {
            out.insert(name, CustomPropertyValue { tokens });
        }
    }
    out
}

/// Whether `tokens` match a registered `syntax`. `None` means `*`, which matches
/// anything. Unknown syntaxes are accepted rather than rejected, since the
/// registry cannot check them yet.
pub fn syntax_matches(syntax: Option<&str>, tokens: &[ComponentValue]) -> bool {
    let Some(syntax) = syntax else {
        return true;
    };
    let single = crate::syntax::trim_whitespace(tokens);
    match syntax.trim() {
        "<length>" => matches!(
            parse_length_percentage(single, false, true),
            Some(Value::Length { .. })
        ),
        "<percentage>" => matches!(
            single,
            [ComponentValue::Token(t)] if matches!(t.kind, TokenKind::Percentage(_))
        ),
        "<number>" => matches!(
            single,
            [ComponentValue::Token(t)] if matches!(t.kind, TokenKind::Number { .. })
        ),
        "<integer>" => matches!(
            single,
            [ComponentValue::Token(t)] if matches!(
                t.kind,
                TokenKind::Number { ty: crate::syntax::NumericType::Integer, .. }
            )
        ),
        "<length-percentage>" => parse_length_percentage(single, false, true).is_some(),
        "<color>" => parse_color(single).is_some(),
        _ => true,
    }
}

/// True if any `var()` appears in `values`, at any depth.
pub fn contains_var(values: &[ComponentValue]) -> bool {
    values.iter().any(|v| match v {
        ComponentValue::Function(f) => f.name.eq_ignore_ascii_case("var") || contains_var(&f.value),
        ComponentValue::Block(b) => contains_var(&b.value),
        ComponentValue::Token(_) => false,
    })
}

/// Replaces every `var()` in `values` with its value (spec 14.6).
pub fn substitute(
    values: &[ComponentValue],
    resolver: &mut dyn VarResolver,
    depth: usize,
    max_depth: usize,
) -> Result<Vec<ComponentValue>, VarError> {
    let mut out = Vec::with_capacity(values.len());
    for v in values {
        match v {
            ComponentValue::Function(f) if f.name.eq_ignore_ascii_case("var") => {
                if depth >= max_depth {
                    return Err(VarError);
                }
                let (name, fallback) = parse_var_args(&f.value).ok_or(VarError)?;
                if let Some(tokens) = resolver.resolve_var(&name, depth) {
                    out.extend(tokens);
                } else if let Some(fb) = fallback {
                    out.extend(substitute(fb, resolver, depth + 1, max_depth)?);
                } else {
                    return Err(VarError);
                }
            }
            ComponentValue::Function(f) => {
                let value = substitute(&f.value, resolver, depth, max_depth)?;
                out.push(ComponentValue::Function(Function {
                    name: f.name.clone(),
                    value,
                    span: f.span,
                }));
            }
            ComponentValue::Block(b) => {
                let value = substitute(&b.value, resolver, depth, max_depth)?;
                out.push(ComponentValue::Block(SimpleBlock {
                    kind: b.kind,
                    value,
                    span: b.span,
                }));
            }
            ComponentValue::Token(_) => out.push(v.clone()),
        }
    }
    Ok(out)
}

/// Parses the arguments of `var(--name [, fallback])`.
fn parse_var_args(args: &[ComponentValue]) -> Option<(String, Option<&[ComponentValue]>)> {
    let args = crate::syntax::trim_whitespace(args);
    let name = match args.first() {
        Some(ComponentValue::Token(t)) => match &t.kind {
            TokenKind::Ident(n) if n.starts_with("--") => n.clone(),
            _ => return None,
        },
        _ => return None,
    };
    let rest = crate::syntax::trim_whitespace(&args[1..]);
    match rest.first() {
        None => Some((name, None)),
        Some(ComponentValue::Token(t)) if t.kind == TokenKind::Comma => {
            Some((name, Some(crate::syntax::trim_whitespace(&rest[1..]))))
        }
        _ => None,
    }
}

/// Serializes component values back to CSS text (used for custom property text
/// and for tests). Whitespace collapses to single spaces.
pub fn components_text(values: &[ComponentValue]) -> String {
    let mut out = String::new();
    for v in values {
        match v {
            ComponentValue::Token(t) => out.push_str(&token_text(&t.kind)),
            ComponentValue::Function(f) => {
                out.push_str(&f.name);
                out.push('(');
                out.push_str(&components_text(&f.value));
                out.push(')');
            }
            ComponentValue::Block(b) => {
                let (open, close) = match b.kind {
                    crate::syntax::BlockKind::Paren => ('(', ')'),
                    crate::syntax::BlockKind::Square => ('[', ']'),
                    crate::syntax::BlockKind::Curly => ('{', '}'),
                };
                out.push(open);
                out.push_str(&components_text(&b.value));
                out.push(close);
            }
        }
    }
    out
}

fn token_text(kind: &TokenKind) -> String {
    use crate::values::serialize::format_number;
    match kind {
        TokenKind::Ident(s) => s.clone(),
        TokenKind::Function(s) => format!("{s}("),
        TokenKind::AtKeyword(s) => format!("@{s}"),
        TokenKind::Hash { value, .. } => format!("#{value}"),
        TokenKind::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        TokenKind::Url(s) => format!("url({s})"),
        TokenKind::Delim(c) => c.to_string(),
        TokenKind::Number { value, .. } => format_number(*value),
        TokenKind::Percentage(p) => format!("{}%", format_number(*p)),
        TokenKind::Dimension { value, unit, .. } => format!("{}{unit}", format_number(*value)),
        TokenKind::Whitespace => " ".to_string(),
        TokenKind::Colon => ":".to_string(),
        TokenKind::Semicolon => ";".to_string(),
        TokenKind::Comma => ",".to_string(),
        TokenKind::Cdo => "<!--".to_string(),
        TokenKind::Cdc => "-->".to_string(),
        TokenKind::BadString | TokenKind::BadUrl | TokenKind::Eof => String::new(),
        TokenKind::LeftSquare => "[".to_string(),
        TokenKind::RightSquare => "]".to_string(),
        TokenKind::LeftParen => "(".to_string(),
        TokenKind::RightParen => ")".to_string(),
        TokenKind::LeftCurly => "{".to_string(),
        TokenKind::RightCurly => "}".to_string(),
    }
}
