// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Length and percentage parsing (spec 14.3, Tier 0 and Tier 1 units, and
//! math functions from spec 14.4).

use super::calc::{parse_math_function, unit_from_name};
use super::Value;
use crate::syntax::{trim_whitespace, ComponentValue, TokenKind};

/// Converts a numeric token into a length or percentage value.
///
/// `non_negative` rejects negative values. `unitless_zero` allows a bare `0`
/// as a length, which CSS permits for every length type.
pub fn numeric_to_length(
    kind: &TokenKind,
    non_negative: bool,
    unitless_zero: bool,
) -> Option<Value> {
    match kind {
        TokenKind::Dimension { value, unit, .. } => {
            if non_negative && *value < 0.0 {
                return None;
            }
            let unit = unit_from_name(unit)?;
            Some(Value::Length {
                value: *value,
                unit,
            })
        }
        TokenKind::Percentage(value) => {
            if non_negative && *value < 0.0 {
                return None;
            }
            Some(Value::Percentage(*value))
        }
        TokenKind::Number { value, .. } if unitless_zero && *value == 0.0 => Some(Value::px(0.0)),
        _ => None,
    }
}

/// Parses a single `<length-percentage>`: a numeric token, or a math function.
/// Keywords are handled by the caller.
pub fn parse_length_percentage(
    values: &[ComponentValue],
    non_negative: bool,
    unitless_zero: bool,
) -> Option<Value> {
    match trim_whitespace(values) {
        [ComponentValue::Token(t)] => numeric_to_length(&t.kind, non_negative, unitless_zero),
        [ComponentValue::Function(f)] => {
            parse_math_function(&f.name, &f.value).map(|node| Value::Calc(Box::new(node)))
        }
        _ => None,
    }
}
