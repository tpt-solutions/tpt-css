// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! A tiny arena tree shared by the examples. A real host would store its own
//! DOM and implement `StyleTree` over it; this is the smallest such host.

#![allow(dead_code)]

use tpt_css::ast::Declaration;
use tpt_css::tree::{Attributes, ElementState, NamespaceId, NodeId, NodeKind, QualifiedName};
use tpt_css::StyleTree;

/// One node of the arena.
pub struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    kind: NodeKind,
    name: QualifiedName,
    attributes: Attributes,
    state: ElementState,
}

/// An arena tree. `NodeId` is an index into `nodes`, with generation 0.
pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    /// A tree with a document root and no elements.
    pub fn new() -> Tree {
        let root = Node {
            parent: None,
            children: Vec::new(),
            kind: NodeKind::Document,
            name: element_name("#document"),
            attributes: Attributes::default(),
            state: ElementState::default(),
        };
        Tree { nodes: vec![root] }
    }

    /// Adds an element under `parent`. `class` is optional.
    pub fn add(&mut self, parent: NodeId, name: &str, class: Option<&str>) -> NodeId {
        let id = NodeId::new(self.nodes.len() as u32, 0);
        let mut attributes = Attributes::default();
        if let Some(class) = class {
            attributes.classes.push(class.to_string());
        }
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            kind: NodeKind::Element,
            name: element_name(name),
            attributes,
            state: ElementState::default(),
        });
        self.nodes[parent.index as usize].children.push(id);
        id
    }
}

fn element_name(local: &str) -> QualifiedName {
    QualifiedName {
        namespace: NamespaceId::NONE,
        local: local.to_string(),
    }
}

impl StyleTree for Tree {
    fn root(&self) -> NodeId {
        NodeId::new(0, 0)
    }
    fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node.index as usize].parent
    }
    fn children(&self, node: NodeId) -> &[NodeId] {
        &self.nodes[node.index as usize].children
    }
    fn kind(&self, node: NodeId) -> NodeKind {
        self.nodes[node.index as usize].kind.clone()
    }
    fn name(&self, node: NodeId) -> &QualifiedName {
        &self.nodes[node.index as usize].name
    }
    fn attributes(&self, node: NodeId) -> &Attributes {
        &self.nodes[node.index as usize].attributes
    }
    fn state(&self, node: NodeId) -> &ElementState {
        &self.nodes[node.index as usize].state
    }
    fn inline_style(&self, _node: NodeId) -> Option<&[Declaration]> {
        None
    }
}

/// The `color` of a computed style, as the engine serializes it.
pub fn color_of(style: &tpt_css::ComputedStyle) -> String {
    property_of(style, "color")
}

/// The serialized value of one property, or an empty string if it is not set.
pub fn property_of(style: &tpt_css::ComputedStyle, name: &str) -> String {
    style
        .property_values()
        .into_iter()
        .find(|(prop, _)| *prop == name)
        .map(|(_, value)| value)
        .unwrap_or_default()
}
