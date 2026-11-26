// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Integration tests (spec 24 M0 acceptance, spec 26): the same resolver over
//! an AppFront-style tree and a DOM-adapter-style tree, deep trees, and
//! hostile input.

mod appfront;
#[path = "../common/mod.rs"]
mod common;
mod container;
mod invalidation;
mod perf;
mod property;
mod spec_trace;
mod web_like;

use common::{style_of, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::properties::registry::lookup;
use tpt_css::tree::{NodeId, NodeKind, StateBits, StyleTree};
use tpt_css::{ComputedStyle, Environment, ParseOptions, ResolverConfig, StyleResolver};

const BASIC_CSS: &str = include_str!("../../fixtures/mit/basic.css");

/// AppFront-style: no document wrapper, no text nodes, state set directly.
fn appfront_tree() -> (TestTree, Vec<NodeId>) {
    let mut t = TestTree::new(NodeKind::Element);
    t.nodes[0].name.local = "app".into();
    let root = t.root();
    let panel = t.element(root, "panel");
    t.add_class(panel, "card");
    let label = t.element(panel, "label");
    t.set_state(panel, StateBits::HOVER);
    let button = t.element(panel, "button");
    t.add_class(button, "primary");
    t.set_state(button, StateBits::DISABLED);
    (t, vec![root, panel, label, button])
}

/// DOM-adapter-style: a document node, an html element, and text nodes
/// between every element. Same logical elements as `appfront_tree`.
fn dom_tree() -> (TestTree, Vec<NodeId>) {
    let mut t = TestTree::new(NodeKind::Document);
    let doc = t.root();
    let html = t.element(doc, "app");
    t.add_text(doc);
    let _ws = t.add_text(html);
    let panel = t.element(html, "panel");
    t.add_class(panel, "card");
    t.set_state(panel, StateBits::HOVER);
    let _ws2 = t.add_text(panel);
    let label = t.element(panel, "label");
    let _ws3 = t.add_text(panel);
    let button = t.element(panel, "button");
    t.add_class(button, "primary");
    t.set_state(button, StateBits::DISABLED);
    (t, vec![html, panel, label, button])
}

const CSS: &str = BASIC_CSS;

fn summarize(s: &ComputedStyle) -> Vec<String> {
    [
        "color",
        "font-size",
        "font-weight",
        "margin-left",
        "border-top-width",
        "line-height",
    ]
    .iter()
    .map(|name| {
        let id = lookup(name).expect("known property");
        format!(
            "{name}={}",
            tpt_css::values::serialize::serialize_value(s.raw(id))
        )
    })
    .collect()
}

#[test]
fn the_same_resolver_gives_the_same_styles_on_both_tree_shapes() {
    let (app, app_nodes) = appfront_tree();
    let (dom, dom_nodes) = dom_tree();

    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(common::parse_sheet(CSS).stylesheet, Origin::Author);
    let app_map = resolver.resolve(&app, &Environment::default());
    let dom_map = resolver.resolve(&dom, &Environment::default());

    for (a, d) in app_nodes.iter().zip(dom_nodes.iter()) {
        assert_eq!(
            summarize(style_of(&app_map, *a)),
            summarize(style_of(&dom_map, *d)),
            "styles differ for element {}",
            app.name(*a).local
        );
    }
}

#[test]
fn expected_values_on_the_appfront_tree() {
    let (app, nodes) = appfront_tree();
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(common::parse_sheet(CSS).stylesheet, Origin::Author);
    let map = resolver.resolve(&app, &Environment::default());
    let [_root, panel, label, button] = [nodes[0], nodes[1], nodes[2], nodes[3]];

    // app: 18px, color #222 inherited by everything below it.
    assert_eq!(style_of(&map, panel).font_size(), 18.0);
    assert_eq!(style_of(&map, label).font_weight(), 700.0);
    // panel:hover turns the border on, and the 2px width applies.
    assert_eq!(
        style_of(&map, panel).raw(lookup("border-top-width").unwrap()),
        &tpt_css::values::Value::px(2.0)
    );
    // The label is not hovered, so no border is set on it.
    assert_eq!(
        style_of(&map, label).raw(lookup("border-top-width").unwrap()),
        &tpt_css::values::Value::px(0.0)
    );
    // button.primary sets 1rem, which is the root's 18px.
    assert_eq!(style_of(&map, button).font_size(), 18.0);
    // button.primary sets blue; it is the only rule for the button's color.
    assert_eq!(
        style_of(&map, button).color(),
        tpt_css::values::Color::rgb8(0, 0, 255)
    );
    // panel > label sets line-height on the label.
    assert_eq!(
        style_of(&map, label).raw(lookup("line-height").unwrap()),
        &tpt_css::values::Value::Number(1.5)
    );
}

#[test]
fn the_committed_basic_fixture_parses_without_errors() {
    let parsed = common::parse_sheet(BASIC_CSS);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    assert!(!parsed.stylesheet.rules.is_empty());
}

#[test]
fn a_deep_tree_resolves_without_overflowing_the_stack() {
    const DEPTH: usize = 20_000;
    let mut t = TestTree::new(NodeKind::Element);
    let mut current = t.root();
    t.nodes[0].name.local = "n".into();
    for _ in 0..DEPTH {
        current = t.element(current, "n");
    }
    let map = common::style_with("n { font-size: 2em }", &t);
    // Each level doubles the font size inherited from its parent. After about
    // 1024 levels the value overflows, and it clamps to the largest finite f64.
    let leaf = style_of(&map, current);
    assert_eq!(leaf.font_size(), f64::MAX);
    assert_eq!(map.len(), DEPTH + 1);
}

/// Deterministic pseudo-random generator (LCG). Keeps the test reproducible.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn hostile_input_never_panics() {
    const ALPHABET: &[char] = &[
        '{', '}', '(', ')', '[', ']', ':', ';', ',', '.', '#', '>', '+', '~', '*', '"', '\'', '\\',
        '/', '!', '-', '@', ' ', 'a', 'b', 'x', '1', '0', '9', '%', '\n', '\r', '\t', '\u{0C}',
        '\u{0}', '\u{FFFD}', 'é', '<', '=', '|', '$', '^', 'u', 'r', 'l', 'i', 'm', 'p', 'o', 't',
        'n', 'a', 'n', 't', 'e', 'r',
    ];
    let mut rng = Lcg(0x5EED);
    let mut t = TestTree::new(NodeKind::Element);
    let child = t.element(t.root(), "a");
    t.add_class(child, "b");

    for _ in 0..3000 {
        let len = (rng.next() % 200) as usize;
        let input: String = (0..len)
            .map(|_| ALPHABET[(rng.next() as usize) % ALPHABET.len()])
            .collect();

        let parsed = tpt_css::parse_stylesheet(&input, ParseOptions::default());
        let _ = tpt_css::parse_declaration_list(&input, ParseOptions::default());

        let mut resolver = StyleResolver::new(ResolverConfig::default());
        resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
        let _ = resolver.resolve(&t, &Environment::default());

        // Also drive a stylesheet made of the tree's own names through the resolver.
        let _ = common::style_with(&input, &t);
    }
}

#[test]
fn the_tree_contract_is_usable_through_a_trait_object_free_generic() {
    fn count_elements<T: StyleTree>(tree: &T) -> usize {
        let mut stack = vec![tree.root()];
        let mut n = 0;
        while let Some(node) = stack.pop() {
            if tree.kind(node) == NodeKind::Element {
                n += 1;
            }
            stack.extend_from_slice(tree.children(node));
        }
        n
    }
    let (app, _) = appfront_tree();
    let (dom, _) = dom_tree();
    assert_eq!(count_elements(&app), 4);
    assert_eq!(count_elements(&dom), 4);
}

#[test]
fn computed_style_debug_output_is_stable_across_runs() {
    let (app, nodes) = appfront_tree();
    let a = common::style_with(CSS, &app);
    let b = common::style_with(CSS, &app);
    assert_eq!(
        format!("{:?}", style_of(&a, nodes[1])),
        format!("{:?}", style_of(&b, nodes[1]))
    );
}

#[test]
fn environment_default_values() {
    let env = Environment::default();
    assert_eq!(env.root_font_size, 16.0);
    assert_eq!(env.viewport.width, 800.0);
}
