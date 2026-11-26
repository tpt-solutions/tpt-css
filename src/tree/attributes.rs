// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Attribute storage for tree nodes (spec 9).
//!
//! The core has no notion of which attribute is "id" or "class". The tree
//! adapter fills `id` and `classes` for `#id` and `.class` selectors. Attribute
//! selectors such as `[attr=value]` read `pairs` only, so an adapter that wants
//! `[id=...]` must also list the id in `pairs`.

/// Attributes of one element.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attributes {
    /// Value used by `#id` selectors.
    pub id: Option<String>,
    /// Tokens used by `.class` selectors.
    pub classes: Vec<String>,
    /// Name/value pairs for attribute selectors, in source order.
    pub pairs: Vec<(String, String)>,
}

impl Attributes {
    /// Returns the value of the first attribute named `name`.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Returns true if the element has the class token `class`.
    pub fn has_class(&self, class: &str) -> bool {
        self.classes.iter().any(|c| c == class)
    }
}
