// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The fixture tree and the reports for the `tpt-css` command line and the WASM
//! playground (DESIGN.md decision 40). It is a source file shared by both: the
//! binary includes it with `#[path]`, and so does `playground/`. It is not part
//! of the library's public API.

use crate::json::{self, Json};
use tpt_css::tree::{
    Attributes, ElementState, NamespaceId, NodeId, NodeKind, QualifiedName, StyleTree,
};
use tpt_css::{
    parse_declaration_list, parse_stylesheet, ComputedStyle, Environment, Origin, ParseOptions,
    ResolverConfig, Severity, StyleResolver,
};

/// One line per diagnostic, with the line and column of `offset` in `source`,
/// and whether any diagnostic is an error.
pub fn check_report(source: &str) -> (Vec<String>, bool) {
    let parsed = parse_stylesheet(source, ParseOptions::default());
    let mut errors = false;
    let mut lines = Vec::new();
    for d in &parsed.diagnostics {
        errors |= d.severity == Severity::Error;
        let offset = (d.span.start as usize).min(source.len());
        let before = &source[..offset];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, str::len) + 1;
        let label = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        lines.push(format!(
            "{label} {:?} at {line}:{column}: {}",
            d.code, d.message
        ));
    }
    if lines.is_empty() {
        lines.push("no diagnostics".to_string());
    }
    (lines, errors)
}

/// The computed styles of a JSON fixture, one line per element and property
/// (see `docs/CLI.md`). With `all`, every property is listed. `extra_sheets` are
/// stylesheets added after the fixture's own, at author origin, in order.
pub fn compute_report(
    text: &str,
    all: bool,
    extra_sheets: &[String],
) -> Result<Vec<String>, String> {
    let fixture = json::parse(text)?;

    let mut sheets = Vec::new();
    for sheet in fixture
        .get("stylesheets")
        .and_then(Json::as_array)
        .unwrap_or(&[])
    {
        sheets.push(
            sheet
                .as_str()
                .ok_or("stylesheets must be strings")?
                .to_string(),
        );
    }
    sheets.extend(extra_sheets.iter().cloned());
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    for css in &sheets {
        let parsed = parse_stylesheet(css, ParseOptions::default());
        resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
    }

    let mut env = Environment::default();
    if let Some(viewport) = fixture.get("viewport") {
        if let Some(w) = viewport.get("width").and_then(Json::as_number) {
            env.viewport.width = w;
        }
        if let Some(h) = viewport.get("height").and_then(Json::as_number) {
            env.viewport.height = h;
        }
    }

    let root = fixture
        .get("root")
        .ok_or("the fixture needs a \"root\" node")?;
    let mut tree = Fixture::new();
    tree.add(tree.root(), root)?;
    let map = resolver.resolve(&tree, &env).clone();

    // The same tree with no author styles, neither sheets nor inline. A property
    // is listed when an author style changed it, so `currentcolor` and the
    // initial values do not show up on every element.
    let mut bare = tree.clone();
    for node in &mut bare.nodes {
        node.inline = None;
    }
    let baseline = StyleResolver::new(ResolverConfig::default())
        .resolve(&bare, &env)
        .clone();

    let mut lines = Vec::new();
    // Element paths are child indexes from the document, for example `0.2.1`.
    let mut stack = vec![(tree.root(), String::new())];
    while let Some((node, path)) = stack.pop() {
        if tree.kind(node) == NodeKind::Element {
            let style = map.get(node).ok_or("an element was not styled")?;
            lines.push(format!("{path} {}", label_of(&tree, node)));
            push_changed(&mut lines, style, baseline.get(node), all);
        }
        for (i, &child) in tree.children(node).iter().enumerate().rev() {
            let child_path = if path.is_empty() {
                i.to_string()
            } else {
                format!("{path}.{i}")
            };
            stack.push((child, child_path));
        }
    }
    Ok(lines)
}

fn label_of(tree: &Fixture, node: NodeId) -> String {
    let attrs = tree.attributes(node);
    let mut label = tree.name(node).local.clone();
    if let Some(id) = &attrs.id {
        label.push('#');
        label.push_str(id);
    }
    for class in &attrs.classes {
        label.push('.');
        label.push_str(class);
    }
    label
}

fn push_changed(
    lines: &mut Vec<String>,
    style: &ComputedStyle,
    base: Option<&ComputedStyle>,
    all: bool,
) {
    let base = base.map(ComputedStyle::property_values);
    for (i, (name, value)) in style.property_values().into_iter().enumerate() {
        let unchanged = base.as_ref().is_some_and(|b| b[i].1 == value);
        if all || !unchanged {
            lines.push(format!("  {name}: {value}"));
        }
    }
}

/// An arena tree built from a fixture. It implements the tree contract.
#[derive(Clone)]
pub struct Fixture {
    pub nodes: Vec<Node>,
}

#[derive(Clone)]
pub struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    kind: NodeKind,
    name: QualifiedName,
    attributes: Attributes,
    state: ElementState,
    inline: Option<Vec<tpt_css::ast::Declaration>>,
    text: String,
}

impl Fixture {
    fn new() -> Fixture {
        let mut f = Fixture { nodes: Vec::new() };
        f.push(None, NodeKind::Document, "#document", String::new());
        f
    }

    fn push(&mut self, parent: Option<NodeId>, kind: NodeKind, name: &str, text: String) -> NodeId {
        let id = NodeId::new(self.nodes.len() as u32, 0);
        self.nodes.push(Node {
            parent,
            children: Vec::new(),
            kind,
            name: QualifiedName {
                namespace: NamespaceId::NONE,
                local: name.to_string(),
            },
            attributes: Attributes::default(),
            state: ElementState::default(),
            inline: None,
            text,
        });
        if let Some(p) = parent {
            self.nodes[p.index as usize].children.push(id);
        }
        id
    }

    /// Adds the JSON node `value` under `parent`. Elements need `element`, text
    /// nodes need `text`. Attributes: `id`, `class` (space separated), `style`
    /// (inline declarations), and any other string members of `attributes`.
    fn add(&mut self, parent: NodeId, value: &Json) -> Result<(), String> {
        if let Some(text) = value.get("text").and_then(Json::as_str) {
            self.push(Some(parent), NodeKind::Text, "#text", text.to_string());
            return Ok(());
        }
        let name = value
            .get("element")
            .and_then(Json::as_str)
            .ok_or("a node needs \"element\" or \"text\"")?;
        let id = self.push(
            Some(parent),
            NodeKind::Element,
            &name.to_ascii_lowercase(),
            String::new(),
        );
        let node = &mut self.nodes[id.index as usize];
        if let Some(v) = value.get("id").and_then(Json::as_str) {
            node.attributes.id = Some(v.to_string());
        }
        if let Some(v) = value.get("class").and_then(Json::as_str) {
            node.attributes.classes = v.split_ascii_whitespace().map(str::to_string).collect();
        }
        if let Some(v) = value.get("style").and_then(Json::as_str) {
            node.inline = Some(parse_declaration_list(v, ParseOptions::default()).declarations);
        }
        if let Some(Json::Object(members)) = value.get("attributes") {
            for (k, v) in members {
                if let Some(v) = v.as_str() {
                    node.attributes.pairs.push((k.clone(), v.to_string()));
                }
            }
        }
        for child in value
            .get("children")
            .and_then(Json::as_array)
            .unwrap_or(&[])
        {
            self.add(id, child)?;
        }
        Ok(())
    }
}

impl StyleTree for Fixture {
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
    fn inline_style(&self, node: NodeId) -> Option<&[tpt_css::ast::Declaration]> {
        self.nodes[node.index as usize].inline.as_deref()
    }
    fn text(&self, node: NodeId) -> &str {
        &self.nodes[node.index as usize].text
    }
}
