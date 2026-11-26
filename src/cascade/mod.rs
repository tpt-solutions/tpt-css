// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cascade (spec 13): origin, importance, layers, inline style, specificity,
//! and source order.
//!
//! Scope proximity (Cascade 6, `@scope`) is part of the key, after specificity
//! (DESIGN.md decision 12).

pub mod order;
pub mod origin;

use crate::selectors::specificity::Specificity;
use order::DeclarationOrder;
use origin::Origin;
use std::cmp::Ordering;

/// Layer rank for a declaration that is not in any layer (spec 13.4).
///
/// Unlayered normal declarations beat every layered normal declaration. For
/// important declarations the order reverses, and unlayered important
/// declarations lose to every layered important declaration.
pub const UNLAYERED_NORMAL: u32 = u32::MAX;
/// Layer rank for an unlayered important declaration.
pub const UNLAYERED_IMPORTANT: u32 = 0;

/// Layer rank for a declaration in layer `index` (0 is the first layer
/// declared), given its importance.
///
/// Normal: later layers rank higher, so they win (spec 13.4). Important:
/// earlier layers rank higher, so they win.
pub fn layer_rank(index: usize, important: bool) -> u32 {
    let index = index as u32;
    if important {
        // Leave room below for UNLAYERED_IMPORTANT and above the earliest layer.
        u32::MAX - 1 - index
    } else {
        index + 1
    }
}

/// Everything needed to decide which of two competing declarations wins.
///
/// Comparison (spec 13.3, Cascade 5 with layers from 13.4):
/// 1. origin and importance (`Origin::precedence`), higher wins
/// 2. layer rank (`layer_rank`), higher wins
/// 3. inline style over non-inline at the same origin and importance (spec 13.5)
/// 4. specificity, higher wins
/// 5. scope proximity, lower wins (Cascade 6, after specificity)
/// 6. declaration order, later wins
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeKey {
    /// Origin of the declaration.
    pub origin: Origin,
    /// Whether the declaration is `!important`.
    pub important: bool,
    /// Layer rank. See [`layer_rank`] and [`UNLAYERED_NORMAL`].
    pub layer: u32,
    /// Scope proximity: the number of ancestor steps from the element up to the
    /// root of the `@scope` the rule is scoped to. Lower wins. Unscoped rules
    /// use `u32::MAX` (Cascade 6).
    pub scope_proximity: u32,
    /// Whether it came from a `style` attribute.
    pub inline: bool,
    /// Specificity of the matching selector. Inline styles use zero.
    pub specificity: Specificity,
    /// Source position.
    pub order: DeclarationOrder,
}

impl CascadeKey {
    /// Origin and importance rank.
    pub fn rank(&self) -> u8 {
        self.origin.precedence(self.important)
    }
}

impl Ord for CascadeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank()
            .cmp(&other.rank())
            .then(self.layer.cmp(&other.layer))
            .then(self.inline.cmp(&other.inline))
            .then(self.specificity.cmp(&other.specificity))
            .then(other.scope_proximity.cmp(&self.scope_proximity))
            .then(self.order.cmp(&other.order))
    }
}

impl PartialOrd for CascadeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
