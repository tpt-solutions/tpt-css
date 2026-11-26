// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Invalidation and incremental restyle (spec 8.2, 17).
//!
//! This is the M1 foundation. It covers attribute, state, and inline-style
//! changes, which restyle the changed element and its subtree. Stylesheet
//! changes, environment changes, and tree-shape changes restyle everything.
//!
//! Sibling-sensitive rules are handled conservatively. If any rule depends on
//! sibling structure (`+`, `~`, `:nth-*`, `:empty`), a change also restyles the
//! parent's subtree, which covers every sibling. A dependency index that keeps
//! the restyle to the affected siblings only is M3 work.
//!
//! The change must refer to a node that is still in the tree. Node insertion
//! and removal are reported as [`StyleChange::TreeChanged`], which restyles
//! everything.

use crate::env::Environment;
use crate::style::computed::StyleMap;
use crate::style::resolver::{style_tree, Frame, StyleResolver};
use crate::tree::{NodeId, NodeKind, StyleTree};

/// A change that may require restyling (spec 17.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleChange {
    /// An attribute of the element changed (including `id` and `class`).
    AttributeChanged(NodeId),
    /// The element's state bits or custom states changed.
    StateChanged(NodeId),
    /// The element's inline `style` changed.
    InlineStyleChanged(NodeId),
    /// A stylesheet was added or removed. Restyles everything.
    StyleSheetChanged,
    /// Nodes were inserted, removed, or moved. Restyles everything.
    TreeChanged,
}

impl StyleResolver {
    /// Records a change, so that the next [`StyleResolver::restyle`] recomputes
    /// what it affects (spec 8.2).
    pub fn invalidate<T: StyleTree>(&mut self, tree: &T, change: StyleChange) {
        match change {
            StyleChange::AttributeChanged(node)
            | StyleChange::StateChanged(node)
            | StyleChange::InlineStyleChanged(node) => {
                self.dirty.insert(node);
                let structural = self
                    .sheets
                    .iter()
                    .any(|s| s.rules.iter().any(|r| r.structural));
                if self
                    .sheets
                    .iter()
                    .any(|s| s.rules.iter().any(|r| r.needs_ancestors))
                {
                    // :has() and :scope can depend on any ancestor, so restyle everything.
                    self.full_dirty = true;
                } else if structural {
                    if let Some(parent) = tree.parent(node) {
                        self.dirty.insert(parent);
                    }
                }
            }
            StyleChange::StyleSheetChanged | StyleChange::TreeChanged => {
                self.full_dirty = true;
            }
        }
    }

    /// Recomputes the styles affected since the last resolve or restyle (spec 8.2).
    ///
    /// Falls back to a full [`StyleResolver::resolve`] when there is no previous
    /// result, when the environment changed, or after a sheet or tree change.
    /// Otherwise only the dirty subtrees are recomputed, and everything else is
    /// reused from the previous result.
    pub fn restyle<T: StyleTree>(&mut self, tree: &T, env: &Environment) -> StyleMap {
        let reusable = match &self.cache {
            Some(cache) => cache.env == *env && !self.full_dirty && self.containers.is_none(),
            None => false,
        };
        if !reusable {
            return self.resolve(tree, env);
        }

        let mut map = self
            .cache
            .as_ref()
            .map(|c| c.map.clone())
            .unwrap_or_default();
        let active = self.active_rules(env);
        let max_var_depth = self.config.limits.max_var_depth;
        let dirty: Vec<NodeId> = std::mem::take(&mut self.dirty).into_iter().collect();
        let cutoff = self.subject_only();

        let mut styled = 0;
        for node in dirty {
            let parent_style = match tree.parent(node) {
                Some(p) if tree.kind(p) == NodeKind::Element => map.get(p).cloned(),
                _ => None,
            };
            let start = Frame {
                node,
                parent: parent_style,
                rem_base: rem_base_for(tree, node, &map, env),
            };
            styled += style_tree(
                self,
                tree,
                start,
                env,
                &active,
                max_var_depth,
                cutoff,
                &mut map,
            );
        }

        self.last_restyled = styled;
        self.cache = Some(crate::style::resolver::Cache {
            map: map.clone(),
            env: *env,
        });
        map
    }

    /// Number of elements styled by the most recent [`StyleResolver::resolve`]
    /// or [`StyleResolver::restyle`]. Used to check that a restyle stayed local.
    pub fn last_restyle_count(&self) -> usize {
        self.last_restyled
    }
}

/// The `rem` base for a subtree starting at `node`: `None` if `node` is a
/// top-level element (it is a root), otherwise the font size of its top-level
/// element ancestor.
fn rem_base_for<T: StyleTree>(
    tree: &T,
    node: NodeId,
    map: &StyleMap,
    env: &Environment,
) -> Option<f64> {
    let mut top: Option<NodeId> = None;
    let mut current = Some(node);
    while let Some(n) = current {
        if tree.kind(n) == NodeKind::Element {
            top = Some(n);
        }
        current = tree.parent(n);
    }
    let top = top?;
    if top == node {
        None
    } else {
        Some(map.get(top).map_or(env.root_font_size, |s| s.font_size()))
    }
}
