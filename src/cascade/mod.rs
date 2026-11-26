// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cascade (spec 13): origin, importance, inline style, specificity, and
//! source order.
//!
//! Layers (M2), scope proximity (M2), and animation interpolation are not
//! implemented. The key below has the same shape as spec 13.3 so those can be
//! added without changing the comparison of the existing fields.

pub mod order;
pub mod origin;

use crate::selectors::specificity::Specificity;
use order::DeclarationOrder;
use origin::Origin;
use std::cmp::Ordering;

/// Everything needed to decide which of two competing declarations wins.
///
/// Comparison (spec 13.3, Cascade 5):
/// 1. origin and importance (`Origin::precedence`), higher wins
/// 2. inline style over non-inline at the same origin and importance (spec 13.5)
/// 3. specificity, higher wins
/// 4. declaration order, later wins
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeKey {
    /// Origin of the declaration.
    pub origin: Origin,
    /// Whether the declaration is `!important`.
    pub important: bool,
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
            .then(self.inline.cmp(&other.inline))
            .then(self.specificity.cmp(&other.specificity))
            .then(self.order.cmp(&other.order))
    }
}

impl PartialOrd for CascadeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
