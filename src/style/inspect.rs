// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Devtools inspection data (spec 7 `diagnostics`, spec 18).
//!
//! For an element, [`StyleResolver::inspect`] reports the declaration that wins
//! each property, with the origin, importance, layer, specificity, and source
//! position that decided it. The winner is the highest candidate, so `revert`
//! and `revert-layer` are not followed here. Use the computed value for the
//! final result.

use crate::cascade::order::DeclarationOrder;
use crate::cascade::origin::Origin;
use crate::env::Environment;
use crate::properties::registry::PROPERTIES;
use crate::properties::SpecifiedValue;
use crate::selectors::Specificity;
use crate::style::resolver::{Cascaded, StyleResolver};
use crate::tree::{NodeId, StyleTree};

/// Why one property has its value on one element.
#[derive(Debug, Clone, PartialEq)]
pub struct Inspection {
    /// The property name.
    pub property: &'static str,
    /// The winning declaration's value, as specified.
    pub specified: SpecifiedValue,
    /// The origin of the winning declaration.
    pub origin: Origin,
    /// Whether it is `!important`.
    pub important: bool,
    /// The layer rank (see `cascade::layer_rank`); `UNLAYERED_NORMAL` when
    /// unlayered and normal.
    pub layer: u32,
    /// Whether it came from a `style` attribute.
    pub inline: bool,
    /// The specificity of its selector.
    pub specificity: Specificity,
    /// Where it was declared.
    pub source: DeclarationOrder,
}

impl StyleResolver {
    /// Reports the winning declaration for each property of `node` (spec 7,
    /// devtools-facing data).
    pub fn inspect<T: StyleTree>(
        &self,
        tree: &T,
        node: NodeId,
        env: &Environment,
    ) -> Vec<Inspection> {
        self.siblings.clear();
        let active = self.active_rules(env);
        let mut targets = self.cascade(tree, node, &active);
        let cascaded = targets.remove(&None).unwrap_or_else(Cascaded::new);
        let mut out = Vec::new();
        for (idx, candidates) in cascaded.props.iter().enumerate() {
            let Some(winner) = candidates.iter().max_by(|a, b| a.key.cmp(&b.key)) else {
                continue;
            };
            out.push(Inspection {
                property: PROPERTIES[idx].name,
                specified: winner.value.clone(),
                origin: winner.key.origin,
                important: winner.key.important,
                layer: winner.key.layer,
                inline: winner.key.inline,
                specificity: winner.key.specificity,
                source: winner.key.order,
            });
        }
        out
    }
}
