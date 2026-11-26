// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Explicit property aliases (spec 15.4).
//!
//! An alias is an alternative name for a property. It is listed here by hand,
//! with the property it stands for, so that an alias is never recognized by
//! stripping a vendor prefix or by any other string rule. Only names in
//! [`ALIASES`] resolve; every other name is looked up as written.

/// `(alias, canonical name)` pairs. The canonical name must be a registered
/// property.
pub const ALIASES: &[(&str, &str)] = &[
    // CSS Text 3 keeps `word-wrap` as an alias of `overflow-wrap`.
    ("word-wrap", "overflow-wrap"),
];

/// The canonical name for `name`: the property it aliases, or `name` itself.
/// `name` must already be lowercased.
pub fn canonical_name(name: &str) -> &str {
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or(name, |(_, canonical)| canonical)
}
