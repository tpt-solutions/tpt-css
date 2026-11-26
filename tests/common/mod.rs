// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Shared test helpers: a small in-memory tree that implements `StyleTree`.
//!
//! This tree is written from the spec 9 contract only. It has no DOM types.

#![allow(dead_code)]

pub mod markup;

use tpt_css::ast::Declaration;
use tpt_css::parse::ParsedStylesheet;
use tpt_css::tree::node::NamespaceId;
use tpt_css::tree::{
    Attributes, Direction, ElementState, NodeId, NodeKind, QualifiedName, StateBits, StyleTree,
};
use tpt_css::{
    parse_declaration_list, ComputedStyle, Environment, Origin, ParseOptions, StyleMap,
    StyleResolver,
};

pub struct TestNode {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: NodeKind,
    pub name: QualifiedName,
    pub attributes: Attributes,
    pub state: ElementState,
    pub inline: Option<Vec<Declaration>>,
    pub text: String,
    pub language: Option<String>,
    pub direction: Option<Direction>,
}

pub struct TestTree {
    /// `(host, shadow root)` pairs.
    pub shadows: Vec<(NodeId, NodeId)>,
    /// `(node, slot)` pairs: the node is assigned to the slot.
    pub slots: Vec<(NodeId, NodeId)>,
    pub nodes: Vec<TestNode>,
}

impl TestTree {
    /// Creates a tree whose root is a node of `kind`.
    pub fn new(root_kind: NodeKind) -> TestTree {
        let mut tree = TestTree {
            nodes: Vec::new(),
            shadows: Vec::new(),
            slots: Vec::new(),
        };
        tree.push(None, root_kind, "root");
        tree
    }

    fn push(&mut self, parent: Option<NodeId>, kind: NodeKind, local: &str) -> NodeId {
        let id = NodeId::new(self.nodes.len() as u32, 0);
        self.nodes.push(TestNode {
            parent,
            children: Vec::new(),
            kind,
            name: QualifiedName {
                namespace: NamespaceId::NONE,
                local: local.to_string(),
            },
            attributes: Attributes::default(),
            state: ElementState::default(),
            inline: None,
            text: String::new(),
            language: None,
            direction: None,
        });
        if let Some(p) = parent {
            self.nodes[p.index as usize].children.push(id);
        }
        id
    }

    pub fn root(&self) -> NodeId {
        NodeId::new(0, 0)
    }

    /// Attaches a shadow root to `host` and returns it. The root has no tree
    /// parent, as the contract requires for a shadow root.
    pub fn attach_shadow(&mut self, host: NodeId) -> NodeId {
        let root = self.push(None, NodeKind::Fragment, "#shadow-root");
        self.shadows.push((host, root));
        root
    }

    /// Assigns `node` to `slot`, for `::slotted()`.
    pub fn assign_slot(&mut self, node: NodeId, slot: NodeId) {
        self.slots.push((node, slot));
    }

    /// Adds an element under `parent`.
    pub fn element(&mut self, parent: NodeId, name: &str) -> NodeId {
        self.push(Some(parent), NodeKind::Element, name)
    }

    /// Adds an empty text node under `parent`. Use [`TestTree::set_text`] for content.
    pub fn add_text(&mut self, parent: NodeId) -> NodeId {
        self.push(Some(parent), NodeKind::Text, "#text")
    }

    /// Adds a text node with content under `parent`.
    pub fn add_text_with(&mut self, parent: NodeId, content: &str) -> NodeId {
        let id = self.add_text(parent);
        self.set_text(id, content);
        id
    }

    pub fn set_text(&mut self, node: NodeId, content: &str) -> &mut Self {
        self.nodes[node.index as usize].text = content.to_string();
        self
    }

    pub fn set_lang(&mut self, node: NodeId, lang: &str) -> &mut Self {
        self.nodes[node.index as usize].language = Some(lang.to_string());
        self
    }

    pub fn set_dir(&mut self, node: NodeId, dir: Direction) -> &mut Self {
        self.nodes[node.index as usize].direction = Some(dir);
        self
    }

    pub fn set_id(&mut self, node: NodeId, id: &str) -> &mut Self {
        self.nodes[node.index as usize].attributes.id = Some(id.to_string());
        self
    }

    pub fn add_class(&mut self, node: NodeId, class: &str) -> &mut Self {
        self.nodes[node.index as usize]
            .attributes
            .classes
            .push(class.to_string());
        self
    }

    pub fn set_attr(&mut self, node: NodeId, name: &str, value: &str) -> &mut Self {
        self.nodes[node.index as usize]
            .attributes
            .pairs
            .push((name.to_string(), value.to_string()));
        self
    }

    pub fn set_state(&mut self, node: NodeId, bits: StateBits) -> &mut Self {
        let s = &mut self.nodes[node.index as usize].state;
        s.bits = s.bits.union(bits);
        self
    }

    /// Sets the inline `style` attribute by parsing `css`.
    pub fn set_inline(&mut self, node: NodeId, css: &str) -> &mut Self {
        let parsed = parse_declaration_list(css, ParseOptions::default());
        self.nodes[node.index as usize].inline = Some(parsed.declarations);
        self
    }
}

impl StyleTree for TestTree {
    fn root(&self) -> NodeId {
        NodeId::new(0, 0)
    }
    fn shadow(&self, node: NodeId) -> tpt_css::tree::ShadowInfo {
        // The top of the node's tree decides whether it is in a shadow tree.
        let mut top = node;
        while let Some(parent) = self.parent(top) {
            top = parent;
        }
        match self.shadows.iter().find(|(_, root)| *root == top) {
            Some((host, root)) => tpt_css::tree::ShadowInfo {
                host: Some(*host),
                shadow_root: (node == *root).then_some(*root),
            },
            None => tpt_css::tree::ShadowInfo::default(),
        }
    }
    fn shadow_root(&self, host: NodeId) -> Option<NodeId> {
        self.shadows
            .iter()
            .find(|(h, _)| *h == host)
            .map(|(_, root)| *root)
    }
    fn assigned_slot(&self, node: NodeId) -> Option<NodeId> {
        self.slots
            .iter()
            .find(|(n, _)| *n == node)
            .map(|(_, slot)| *slot)
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
    fn inline_style(&self, node: NodeId) -> Option<&[Declaration]> {
        self.nodes[node.index as usize].inline.as_deref()
    }
    fn text(&self, node: NodeId) -> &str {
        &self.nodes[node.index as usize].text
    }
    fn language(&self, node: NodeId) -> Option<&str> {
        self.nodes[node.index as usize].language.as_deref()
    }
    fn direction(&self, node: NodeId) -> Option<Direction> {
        self.nodes[node.index as usize].direction
    }
}

/// Parses `css` as an author stylesheet.
pub fn parse_sheet(css: &str) -> ParsedStylesheet {
    tpt_css::parse_stylesheet(css, ParseOptions::default())
}

/// Resolves `css` (author origin) against `tree` with the default environment.
pub fn style_with(css: &str, tree: &TestTree) -> StyleMap {
    style_with_env(css, tree, &Environment::default())
}

/// Resolves `css` (author origin) against `tree` with the given environment.
pub fn style_with_env(css: &str, tree: &TestTree, env: &Environment) -> StyleMap {
    let mut resolver = StyleResolver::new(Default::default());
    let parsed = parse_sheet(css);
    resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
    resolver.resolve(tree, env).clone()
}

/// The computed style of `node`, panicking if the node was not styled.
pub fn style_of(map: &StyleMap, node: NodeId) -> &ComputedStyle {
    map.get(node).expect("node should have a computed style")
}

impl TestTree {
    /// The shadow root attached to `host`. Panics if there is none, since a test
    /// that asks for one has built it.
    pub fn shadow_root_for(&self, host: NodeId) -> NodeId {
        self.shadows
            .iter()
            .find(|(h, _)| *h == host)
            .map(|(_, root)| *root)
            .expect("the host has a shadow root")
    }
}
