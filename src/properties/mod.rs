// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property registry and grammar parsing (spec 15).
//!
//! Shorthand expansion (M1), aliases (M1, spec 15.4), and custom properties
//! (M1) are not implemented yet. See TODO.md for the milestone plan.

pub mod parse;
pub mod registry;

pub use parse::{parse_property_value, CssWide, SpecifiedValue};
pub use registry::{ids, lookup, spec, Grammar, Initial, PropertyId, PropertySpec, PROPERTIES};
