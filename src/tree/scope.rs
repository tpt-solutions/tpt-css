// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Scope and shadow information (spec 9.3). Only the shape of the contract is
//! defined here. Scope and shadow semantics arrive in M2 and M4.

use super::node::NodeId;

/// Scope boundaries for `@scope` (spec 9.3). Unused until M2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScopeInfo {
    /// Scope root, if the node is inside a scope.
    pub scope_root: Option<NodeId>,
    /// Lower scope bound.
    pub lower_bound: Option<NodeId>,
    /// Upper scope bound.
    pub upper_bound: Option<NodeId>,
}

/// Shadow-tree information (spec 9.3). See [`StyleTree::shadow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShadowInfo {
    /// Host element, if the node is in a shadow tree.
    pub host: Option<NodeId>,
    /// Shadow root, if the node is one.
    pub shadow_root: Option<NodeId>,
}

/// The shadow root whose tree contains `node`, if any. A shadow root is the top
/// of its own tree, so this climbs to the top once and asks it (spec 9.3).
pub fn shadow_root_of<T: super::StyleTree>(tree: &T, node: NodeId) -> Option<NodeId> {
    let mut top = node;
    while let Some(parent) = tree.parent(top) {
        top = parent;
    }
    (tree.shadow(top).shadow_root == Some(top)).then_some(top)
}

/// The flat-tree parent of `node`, which the cascade inherits from (spec 9.3).
/// For a shadow root, and for the top-level children of one, this is the host.
/// Otherwise it is the tree parent. A shadow root has no tree parent, so only
/// parentless nodes need the shadow check.
pub fn flat_parent<T: super::StyleTree>(tree: &T, node: NodeId) -> Option<NodeId> {
    match tree.parent(node) {
        None => {
            if tree.shadow(node).shadow_root == Some(node) {
                tree.shadow(node).host
            } else {
                None
            }
        }
        Some(parent) => {
            if tree.parent(parent).is_none() && tree.shadow(parent).shadow_root == Some(parent) {
                tree.shadow(parent).host
            } else {
                Some(parent)
            }
        }
    }
}
