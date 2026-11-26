// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Starter adapter for tpt-css.
//!
//! The engine styles any tree that implements [`StyleTree`]. This template
//! shows the smallest complete adapter: a node store, the trait, and one test.
//! To use it with your own document model, replace [`Tree`] with a view over
//! your nodes and keep the trait methods the same.
//!
//! Checklist for a real adapter:
//! - Return stable `NodeId`s. The engine keys styles by them.
//! - Make every answer deterministic: the same tree gives the same styles.
//! - Parse inline `style` attributes with `parse_declaration_list` before you
//!   return them from `inline_style`.
//! - Report element state (`:hover`, `:focus`, and so on) through `state`.
//! - Keep node kinds accurate. Only `NodeKind::Element` matches selectors.

use tpt_css::ast::Declaration;
use tpt_css::tree::{Attributes, ElementState, NamespaceId, NodeId, NodeKind, QualifiedName};
use tpt_css::{
    parse_stylesheet, ComputedStyle, Environment, Origin, ParseOptions, ResolverConfig, StyleMap,
    StyleResolver, StyleTree,
};

/// One node of the template tree.
pub struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    kind: NodeKind,
    name: QualifiedName,
    attributes: Attributes,
    state: ElementState,
}

/// A node store. `NodeId` is an index into `nodes`, with generation 0.
pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    /// A tree with only a document root.
    pub fn new() -> Tree {
        Tree {
            nodes: vec![Node {
                parent: None,
                children: Vec::new(),
                kind: NodeKind::Document,
                name: element_name("#document"),
                attributes: Attributes::default(),
                state: ElementState::default(),
            }],
        }
    }

    /// Adds an element under `parent`.
    pub fn add(&mut self, parent: NodeId, name: &str) -> NodeId {
        let id = NodeId::new(self.nodes.len() as u32, 0);
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            kind: NodeKind::Element,
            name: element_name(name),
            attributes: Attributes::default(),
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

/// Styles `tree` with `css` (author origin) and the default environment.
pub fn style(tree: &Tree, css: &str) -> StyleMap {
    let parsed = parse_stylesheet(css, ParseOptions::default());
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
    resolver.resolve(tree, &Environment::default()).clone()
}

/// The computed style of `node`, if the engine styled it.
pub fn style_of(map: &StyleMap, node: NodeId) -> Option<&ComputedStyle> {
    map.get(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paragraph_gets_the_color_from_the_stylesheet() {
        let mut tree = Tree::new();
        let body = tree.add(tree.root(), "body");
        let paragraph = tree.add(body, "p");

        let styles = style(&tree, "p { color: red }");
        let computed = style_of(&styles, paragraph).expect("paragraph is styled");

        let color = computed
            .property_values()
            .into_iter()
            .find(|(name, _)| *name == "color")
            .map(|(_, value)| value);
        assert_eq!(color.as_deref(), Some("rgb(255, 0, 0)"));
    }
}
