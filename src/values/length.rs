// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Length and percentage parsing (spec 14.3, Tier 0: px, em, rem, %).
//!
//! Lengths other than px, em, and rem (Tier 1 and 2 units) are not parsed yet
//! and are rejected, so the declaration is dropped.

use super::{LengthUnit, Value};
use crate::syntax::TokenKind;

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
            let unit = match unit.to_ascii_lowercase().as_str() {
                "px" => LengthUnit::Px,
                "em" => LengthUnit::Em,
                "rem" => LengthUnit::Rem,
                _ => return None,
            };
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
