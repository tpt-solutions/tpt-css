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

/// Shadow-tree information (spec 9.3). Unused until M4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShadowInfo {
    /// Host element, if the node is in a shadow tree.
    pub host: Option<NodeId>,
    /// Shadow root, if the node is one.
    pub shadow_root: Option<NodeId>,
}
