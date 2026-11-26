// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Media query prelude parsing (spec 16.3). Only parses into the AST; the
//! evaluation against an environment happens in `env::media`.
//!
//! A feature this engine does not know parses to `MediaFeature::Unknown`,
//! which never matches (spec 16.3: unknown features are false). A malformed
//! query is a diagnostic and the whole `@media` rule is dropped.

use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::selectors::span_of;
use super::split_on;
use crate::ast::media::{Comparison, MediaFeature, MediaQuery, MediaType, PointerKind};
use crate::syntax::{trim_whitespace, BlockKind, ComponentValue, TokenKind};
use crate::values::LengthUnit;

const MEDIA_URL: &str = "https://www.w3.org/TR/mediaqueries-5/";

/// Parses a media query list (the prelude of `@media`).
pub(crate) fn parse_media_queries(
    values: &[ComponentValue],
) -> Result<Vec<MediaQuery>, Diagnostic> {
    let values = trim_whitespace(values);
    if values.is_empty() {
        return Ok(vec![MediaQuery {
            negated: false,
            media_type: MediaType::All,
            features: Vec::new(),
        }]);
    }
    split_on(values, |v| v.is_token(&TokenKind::Comma))
        .into_iter()
        .map(|part| parse_one(trim_whitespace(part)))
        .collect()
}

fn ident_is(v: Option<&ComponentValue>, word: &str) -> bool {
    matches!(v, Some(ComponentValue::Token(t))
        if matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case(word)))
}

fn parse_one(values: &[ComponentValue]) -> Result<MediaQuery, Diagnostic> {
    let span = span_of(values);
    let mut i = 0;
    let mut negated = false;
    if ident_is(values.first(), "not") {
        negated = true;
        i += 1;
    } else if ident_is(values.first(), "only") {
        i += 1;
    }
    while i < values.len() && values[i].is_whitespace() {
        i += 1;
    }

    let mut media_type = MediaType::All;
    let mut has_type = false;
    if let Some(ComponentValue::Token(t)) = values.get(i) {
        if let TokenKind::Ident(name) = &t.kind {
            if !name.eq_ignore_ascii_case("and") {
                media_type = match name.to_ascii_lowercase().as_str() {
                    "all" => MediaType::All,
                    "screen" => MediaType::Screen,
                    "print" => MediaType::Print,
                    _ => MediaType::Unknown(name.clone()),
                };
                has_type = true;
                i += 1;
            }
        }
    }

    let mut features = Vec::new();
    let mut need_and = has_type;
    loop {
        while i < values.len() && values[i].is_whitespace() {
            i += 1;
        }
        if i >= values.len() {
            break;
        }
        if ident_is(values.get(i), "and") {
            i += 1;
            while i < values.len() && values[i].is_whitespace() {
                i += 1;
            }
        } else if need_and {
            return Err(Diagnostic::error(
                DiagnosticCode::InvalidSelector,
                span,
                "expected 'and' before a media feature",
                Some(MEDIA_URL),
            ));
        }
        match values.get(i) {
            Some(ComponentValue::Block(b)) if b.kind == BlockKind::Paren => {
                features.push(parse_feature(&b.value));
                i += 1;
                need_and = true;
            }
            _ => {
                return Err(Diagnostic::error(
                    DiagnosticCode::InvalidSelector,
                    span,
                    "malformed media query",
                    Some(MEDIA_URL),
                ))
            }
        }
    }

    Ok(MediaQuery {
        negated,
        media_type,
        features,
    })
}

/// Parses `(name)` or `(name: value)`. Anything unrecognized becomes
/// `MediaFeature::Unknown`.
pub(crate) fn parse_feature(values: &[ComponentValue]) -> MediaFeature {
    let v = trim_whitespace(values);
    let name = match v.first() {
        Some(ComponentValue::Token(t)) => match &t.kind {
            TokenKind::Ident(n) => n.to_ascii_lowercase(),
            _ => return MediaFeature::Unknown(String::new()),
        },
        _ => return MediaFeature::Unknown(String::new()),
    };
    let rest = trim_whitespace(&v[1..]);
    let value = match rest {
        [ComponentValue::Token(t), tail @ ..] if t.kind == TokenKind::Colon => {
            trim_whitespace(tail)
        }
        _ => return MediaFeature::Unknown(name),
    };

    let (base, comparison) = if let Some(b) = name.strip_prefix("min-") {
        (b, Comparison::Min)
    } else if let Some(b) = name.strip_prefix("max-") {
        (b, Comparison::Max)
    } else {
        (name.as_str(), Comparison::Equal)
    };

    match base {
        "width" => match length_value(value) {
            Some((n, unit)) => MediaFeature::Width(comparison, n, unit),
            None => MediaFeature::Unknown(name),
        },
        "height" => match length_value(value) {
            Some((n, unit)) => MediaFeature::Height(comparison, n, unit),
            None => MediaFeature::Unknown(name),
        },
        "aspect-ratio" => match ratio_value(value) {
            Some(r) => MediaFeature::AspectRatio(comparison, r),
            None => MediaFeature::Unknown(name),
        },
        "orientation" => match keyword(value).as_deref() {
            Some("landscape") => MediaFeature::Landscape(true),
            Some("portrait") => MediaFeature::Landscape(false),
            _ => MediaFeature::Unknown(name),
        },
        "prefers-color-scheme" => match keyword(value).as_deref() {
            Some("dark") => MediaFeature::PrefersDark(true),
            Some("light") => MediaFeature::PrefersDark(false),
            _ => MediaFeature::Unknown(name),
        },
        "prefers-reduced-motion" => match keyword(value).as_deref() {
            Some("reduce") => MediaFeature::PrefersReducedMotion(true),
            Some("no-preference") => MediaFeature::PrefersReducedMotion(false),
            _ => MediaFeature::Unknown(name),
        },
        "hover" => match keyword(value).as_deref() {
            Some("hover") => MediaFeature::Hover(true),
            Some("none") => MediaFeature::Hover(false),
            _ => MediaFeature::Unknown(name),
        },
        "pointer" => match keyword(value).as_deref() {
            Some("none") => MediaFeature::Pointer(PointerKind::None),
            Some("coarse") => MediaFeature::Pointer(PointerKind::Coarse),
            Some("fine") => MediaFeature::Pointer(PointerKind::Fine),
            _ => MediaFeature::Unknown(name),
        },
        _ => MediaFeature::Unknown(name),
    }
}

fn keyword(values: &[ComponentValue]) -> Option<String> {
    match values {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(s) => Some(s.to_ascii_lowercase()),
            _ => None,
        },
        _ => None,
    }
}

/// A single absolute or font-relative length: `px`, `em`, or `rem`.
fn length_value(values: &[ComponentValue]) -> Option<(f64, LengthUnit)> {
    match values {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Dimension { value, unit, .. } => {
                let unit = match unit.to_ascii_lowercase().as_str() {
                    "px" => LengthUnit::Px,
                    "em" => LengthUnit::Em,
                    "rem" => LengthUnit::Rem,
                    _ => return None,
                };
                Some((*value, unit))
            }
            TokenKind::Number { value, .. } if *value == 0.0 => Some((0.0, LengthUnit::Px)),
            _ => None,
        },
        _ => None,
    }
}

/// `w / h` or a single number, as a ratio.
fn ratio_value(values: &[ComponentValue]) -> Option<f64> {
    let parts: Vec<&ComponentValue> = values.iter().filter(|v| !v.is_whitespace()).collect();
    let number = |v: &ComponentValue| match v {
        ComponentValue::Token(t) => match t.kind {
            TokenKind::Number { value, .. } => Some(value),
            _ => None,
        },
        _ => None,
    };
    match parts.as_slice() {
        [n] => number(n),
        [w, slash, h] if slash.is_token(&TokenKind::Delim('/')) => {
            let (w, h) = (number(w)?, number(h)?);
            if h == 0.0 {
                None
            } else {
                Some(w / h)
            }
        }
        _ => None,
    }
}

/// Parses the prelude of `@container`: an optional container name, then one or
/// more size features joined with `and` (spec 16.5). The name is any identifier
/// that is followed by a feature block.
pub(crate) fn parse_container_prelude(
    values: &[ComponentValue],
) -> Result<(Option<String>, Vec<MediaFeature>), Diagnostic> {
    let values = trim_whitespace(values);
    let mut i = 0;
    let mut name = None;
    if let Some(ComponentValue::Token(t)) = values.first() {
        if let TokenKind::Ident(s) = &t.kind {
            let next = values[1..].iter().find(|v| !v.is_whitespace());
            if !s.eq_ignore_ascii_case("not") && matches!(next, Some(ComponentValue::Block(_))) {
                name = Some(s.clone());
                i = 1;
            }
        }
    }
    let mut features = Vec::new();
    loop {
        while i < values.len() && values[i].is_whitespace() {
            i += 1;
        }
        if i >= values.len() {
            break;
        }
        if !features.is_empty() {
            if !ident_is(values.get(i), "and") {
                return Err(Diagnostic::error(
                    DiagnosticCode::InvalidSelector,
                    span_of(&values[i..]),
                    "expected 'and' between container features",
                    Some(MEDIA_URL),
                ));
            }
            i += 1;
            while i < values.len() && values[i].is_whitespace() {
                i += 1;
            }
        }
        match values.get(i) {
            Some(ComponentValue::Block(b)) if b.kind == BlockKind::Paren => {
                features.push(parse_feature(&b.value));
                i += 1;
            }
            _ => {
                return Err(Diagnostic::error(
                    DiagnosticCode::InvalidSelector,
                    span_of(&values[i..]),
                    "malformed container query",
                    Some(MEDIA_URL),
                ))
            }
        }
    }
    if features.is_empty() {
        return Err(Diagnostic::error(
            DiagnosticCode::InvalidSelector,
            span_of(values),
            "a container query needs at least one feature",
            Some(MEDIA_URL),
        ));
    }
    Ok((name, features))
}
