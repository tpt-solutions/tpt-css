// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The abstract tree contract (spec 9). The CSS core styles anything that
//! implements [`StyleTree`]: AppFront trees, DOM adapters, and test fixtures.

use super::attributes::Attributes;
use super::node::{NodeId, NodeKind, QualifiedName};
use super::scope::{ScopeInfo, ShadowInfo};
use super::state::ElementState;
use crate::ast::Declaration;

/// Direction of text, as provided by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Left to right.
    Ltr,
    /// Right to left.
    Rtl,
}

/// A tree the style engine can read. Implementations must be deterministic:
/// the same tree must return the same answers.
///
/// Spec 9 shows `children` returning a slice, `inline_style` returning a
/// declaration block, and `attributes` returning a reference. Those forms are
/// kept here (decision recorded in DESIGN.md, spec 25.3).
pub trait StyleTree {
    /// The root node of the tree.
    fn root(&self) -> NodeId;

    /// Parent of `node`, or `None` at the top.
    fn parent(&self, node: NodeId) -> Option<NodeId>;

    /// Children of `node` in document order.
    fn children(&self, node: NodeId) -> &[NodeId];

    /// Kind of `node`.
    fn kind(&self, node: NodeId) -> NodeKind;

    /// Name of an element. Only meaningful when `kind` is `Element`.
    fn name(&self, node: NodeId) -> &QualifiedName;

    /// Attributes of an element.
    fn attributes(&self, node: NodeId) -> &Attributes;

    /// Element state bits and custom states.
    fn state(&self, node: NodeId) -> &ElementState;

    /// Inline declarations (the `style` attribute), already parsed with
    /// [`crate::parse_declaration_list`].
    fn inline_style(&self, node: NodeId) -> Option<&[Declaration]>;

    /// Scope information. Defaults to none until M2.
    fn scope(&self, _node: NodeId) -> ScopeInfo {
        ScopeInfo::default()
    }

    /// Shadow information. Defaults to none until M4.
    fn shadow(&self, _node: NodeId) -> ShadowInfo {
        ShadowInfo::default()
    }

    /// Character data of a text node. Empty for other kinds. Used by `:empty`.
    fn text(&self, _node: NodeId) -> &str {
        ""
    }

    /// Language of the node, if known. Used by `:lang()` in M1.
    fn language(&self, _node: NodeId) -> Option<&str> {
        None
    }

    /// Text direction of the node, if known. Used by `:dir()` in M1.
    fn direction(&self, _node: NodeId) -> Option<Direction> {
        None
    }
}
