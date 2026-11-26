// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property registry, grammar parsing, and shorthands (spec 15).
//!
//! Aliases (spec 15.4) are not implemented: the registry has no vendor-prefixed
//! entries. Custom properties and `var()` are in [`crate::style::custom`].

pub mod aliases;
pub mod parse;
pub mod registry;
pub mod shorthand;

pub use parse::{parse_property_value, CssWide, InvalidValue, SpecifiedValue};
pub use registry::{ids, lookup, spec, Grammar, Initial, PropertyId, PropertySpec, PROPERTIES};
pub use shorthand::{shorthand_index, ShorthandSpec, SHORTHANDS};
