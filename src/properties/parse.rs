// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property grammar parsing (spec 14.1, 15.1 `parse`).
//!
//! Turns component values into a typed [`SpecifiedValue`], or rejects them.
//! Rejected values are dropped at parse time, as CSS requires for values that
//! do not match the grammar. Values containing `var()` are not handled yet
//! (M1), so they are rejected here.

use super::registry::{Grammar, PropertySpec};
use crate::syntax::{trim_whitespace, ComponentValue, TokenKind};
use crate::values::color::parse_color;
use crate::values::length::numeric_to_length;
use crate::values::Value;

/// CSS-wide keywords supported in M0 (spec 13.8). `revert` and `revert-layer`
/// need origin and layer tracking (M2) and are rejected for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssWide {
    /// `initial`
    Initial,
    /// `inherit`
    Inherit,
    /// `unset`
    Unset,
}

/// The value does not match the property grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidValue;

/// A parsed declaration value before computation: either a CSS-wide keyword
/// or a typed value.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecifiedValue {
    /// A CSS-wide keyword, resolved during computation.
    CssWide(CssWide),
    /// A typed value.
    Value(Value),
}

/// Parses a declaration value against a property's grammar.
///
/// Returns `Err(InvalidValue)` when the value is invalid. The caller reports the
/// diagnostic and drops the declaration.
pub fn parse_property_value(
    spec: &PropertySpec,
    values: &[ComponentValue],
) -> Result<SpecifiedValue, InvalidValue> {
    let values = trim_whitespace(values);
    if values.is_empty() {
        return Err(InvalidValue);
    }

    if let [ComponentValue::Token(t)] = values {
        if let TokenKind::Ident(name) = &t.kind {
            if let Some(wide) = css_wide(name) {
                return Ok(SpecifiedValue::CssWide(wide));
            }
        }
    }

    match spec.grammar {
        Grammar::Keywords(list) => keyword(values, list)
            .map(SpecifiedValue::Value)
            .ok_or(InvalidValue),
        Grammar::Color => {
            if let Some(k) = keyword(values, &["currentcolor"]) {
                return Ok(SpecifiedValue::Value(k));
            }
            parse_color(values)
                .map(|c| SpecifiedValue::Value(Value::Color(c)))
                .ok_or(InvalidValue)
        }
        Grammar::LengthPercentage {
            non_negative,
            keywords,
        } => {
            if let Some(k) = keyword(values, keywords) {
                return Ok(SpecifiedValue::Value(k));
            }
            single_token_length(values, non_negative)
                .map(SpecifiedValue::Value)
                .ok_or(InvalidValue)
        }
        Grammar::BorderWidth => {
            if let Some(k) = keyword(values, &["thin", "medium", "thick"]) {
                let px = match k {
                    Value::Keyword("thin") => 1.0,
                    Value::Keyword("thick") => 5.0,
                    _ => 3.0,
                };
                return Ok(SpecifiedValue::Value(Value::px(px)));
            }
            single_token_length(values, true)
                .map(SpecifiedValue::Value)
                .ok_or(InvalidValue)
        }
        Grammar::LineHeight => {
            if let Some(k) = keyword(values, &["normal"]) {
                return Ok(SpecifiedValue::Value(k));
            }
            if let [ComponentValue::Token(t)] = values {
                if let TokenKind::Number { value, .. } = t.kind {
                    return if value >= 0.0 {
                        Ok(SpecifiedValue::Value(Value::Number(value)))
                    } else {
                        Err(InvalidValue)
                    };
                }
            }
            single_token_length(values, true)
                .map(SpecifiedValue::Value)
                .ok_or(InvalidValue)
        }
        Grammar::FontWeight => {
            if let Some(k) = keyword(values, &["normal", "bold", "bolder", "lighter"]) {
                return Ok(SpecifiedValue::Value(k));
            }
            if let [ComponentValue::Token(t)] = values {
                if let TokenKind::Number { value, .. } = t.kind {
                    return if (1.0..=1000.0).contains(&value) {
                        Ok(SpecifiedValue::Value(Value::Number(value)))
                    } else {
                        Err(InvalidValue)
                    };
                }
            }
            Err(InvalidValue)
        }
        Grammar::FontFamily => parse_font_family(values)
            .map(SpecifiedValue::Value)
            .ok_or(InvalidValue),
    }
}

fn css_wide(name: &str) -> Option<CssWide> {
    match name.to_ascii_lowercase().as_str() {
        "initial" => Some(CssWide::Initial),
        "inherit" => Some(CssWide::Inherit),
        "unset" => Some(CssWide::Unset),
        _ => None,
    }
}

/// Matches a single identifier against `list`, ASCII case-insensitively.
/// Returns the matching entry as a `'static` keyword value.
fn keyword(values: &[ComponentValue], list: &'static [&'static str]) -> Option<Value> {
    let [ComponentValue::Token(t)] = values else {
        return None;
    };
    let TokenKind::Ident(name) = &t.kind else {
        return None;
    };
    list.iter()
        .copied()
        .find(|k| k.eq_ignore_ascii_case(name))
        .map(Value::Keyword)
}

fn single_token_length(values: &[ComponentValue], non_negative: bool) -> Option<Value> {
    let [ComponentValue::Token(t)] = values else {
        return None;
    };
    numeric_to_length(&t.kind, non_negative, true)
}

/// `font-family`: a comma-separated list. Each entry is either one string or
/// a sequence of identifiers joined with single spaces.
fn parse_font_family(values: &[ComponentValue]) -> Option<Value> {
    let mut names = Vec::new();
    for part in crate::parse::split_on(values, |v| v.is_token(&TokenKind::Comma)) {
        let part = trim_whitespace(part);
        if part.is_empty() {
            return None;
        }
        if let [ComponentValue::Token(t)] = part {
            if let TokenKind::String(s) = &t.kind {
                names.push(s.clone());
                continue;
            }
        }
        let mut words = Vec::new();
        for v in part {
            match v {
                ComponentValue::Token(t) => match &t.kind {
                    TokenKind::Ident(w) => words.push(w.as_str()),
                    TokenKind::Whitespace => {}
                    _ => return None,
                },
                _ => return None,
            }
        }
        names.push(words.join(" "));
    }
    Some(Value::FontFamily(names))
}
