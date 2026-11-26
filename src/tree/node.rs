// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Node identity and node kinds (spec 9, 9.1).

use crate::selectors::pseudo::PseudoElement;

/// A generational node identifier (spec 9). A stale id from a removed node
/// does not compare equal to a newer node at the same index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId {
    /// Slot index in the host tree.
    pub index: u32,
    /// Generation of the slot.
    pub generation: u32,
}

impl NodeId {
    /// Creates an id from an index and generation.
    pub const fn new(index: u32, generation: u32) -> NodeId {
        NodeId { index, generation }
    }
}

/// Namespace identifier. `NamespaceId::NONE` means no namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NamespaceId(pub u32);

impl NamespaceId {
    /// No namespace.
    pub const NONE: NamespaceId = NamespaceId(0);
}

/// An element name with its namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    /// Namespace of the element.
    pub namespace: NamespaceId,
    /// Local name.
    pub local: String,
}

/// The kind of a tree node. Only element-like nodes match selectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// An element.
    Element,
    /// Text.
    Text,
    /// Comment.
    Comment,
    /// Document root.
    Document,
    /// Document fragment.
    Fragment,
    /// A generated pseudo-element node.
    Pseudo(PseudoElement),
}
