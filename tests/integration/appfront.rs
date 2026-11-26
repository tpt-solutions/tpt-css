// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! A nontrivial AppFront-style document (M1 acceptance): components with custom
//! states, a theme set by custom properties, layered base styles, a media-driven
//! density change, and incremental updates when a component's state changes.

use super::common::{parse_sheet, style_of, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::properties::lookup;
use tpt_css::tree::{NodeId, NodeKind, StateBits};
use tpt_css::values::{Color, Value};
use tpt_css::{
    Environment, MediaEnvironment, ResolverConfig, StyleChange, StyleResolver, ViewportInfo,
};

const CSS: &str = "
    @layer reset, base, components;
    @layer reset { * { margin-left: 0px } }
    @layer base {
        app { --accent: #0a84ff; --gap: 8px; font-size: 16px; color: #111 }
        .row { padding-top: var(--gap); padding-left: calc(2 * var(--gap)) }
    }
    @layer components {
        button { color: var(--accent); border: 1px solid var(--accent) }
        button:disabled { color: gray }
        button:focus-visible { margin-left: 2px }
        .row > label { font-weight: bold }
        label:has(input:checked) { color: green }
    }
    @media (max-width: 600px) {
        .row { padding-top: 4px }
    }
    @supports (display: grid) {
        .grid { display: grid }
    }
";

struct App {
    tree: TestTree,
    row: NodeId,
    label: NodeId,
    input: NodeId,
    button: NodeId,
}

fn app() -> App {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "app".into();
    let row = t.element(root, "row");
    t.add_class(row, "row");
    let label = t.element(row, "label");
    let input = t.element(label, "input");
    let button = t.element(row, "button");
    App {
        tree: t,
        row,
        label,
        input,
        button,
    }
}

fn value(map: &tpt_css::StyleMap, node: NodeId, name: &str) -> Value {
    style_of(map, node).raw(lookup(name).unwrap()).clone()
}

fn resolver() -> StyleResolver {
    let mut r = StyleResolver::new(ResolverConfig::default());
    r.add_stylesheet(parse_sheet(CSS).stylesheet, Origin::Author);
    r
}

#[test]
fn the_theme_flows_through_custom_properties() {
    let a = app();
    let map = resolver().resolve(&a.tree, &Environment::default());
    assert_eq!(
        style_of(&map, a.button).color(),
        Color::rgb8(0x0a, 0x84, 0xff)
    );
    assert_eq!(value(&map, a.row, "padding-top"), Value::px(8.0));
    assert_eq!(
        value(&map, a.row, "padding-left"),
        Value::px(16.0),
        "calc(2 * var(--gap))"
    );
}

#[test]
fn layers_order_the_base_and_component_rules() {
    let a = app();
    let map = resolver().resolve(&a.tree, &Environment::default());
    // The reset layer sets margin-left to 0 for every element; nothing later
    // overrides it, so it holds even though the component layer comes last.
    assert_eq!(value(&map, a.button, "margin-left"), Value::px(0.0));
    // The label's bold weight comes from the component layer, a descendant rule.
    assert_eq!(value(&map, a.label, "font-weight"), Value::Number(700.0));
}

#[test]
fn a_custom_state_changes_the_component_and_restyles_incrementally() {
    let mut a = app();
    let mut r = resolver();
    let env = Environment::default();
    let _ = r.resolve(&a.tree, &env);

    // The button is disabled: its text turns gray and nothing else changes.
    a.tree.set_state(a.button, StateBits::DISABLED);
    r.invalidate(&a.tree, StyleChange::StateChanged(a.button));
    let map = r.restyle(&a.tree, &env);
    assert_eq!(style_of(map, a.button).color(), Color::rgb8(128, 128, 128));
    assert_eq!(
        *map,
        resolver().resolve(&a.tree, &env),
        "restyle matches a full resolve"
    );
}

#[test]
fn checked_input_makes_its_label_green_through_has() {
    let mut a = app();
    a.tree.set_state(a.input, StateBits::CHECKED);
    let map = resolver().resolve(&a.tree, &Environment::default());
    assert_eq!(style_of(&map, a.label).color(), Color::rgb8(0, 128, 0));
}

#[test]
fn a_narrow_viewport_tightens_the_row() {
    let a = app();
    let narrow = Environment {
        viewport: ViewportInfo {
            width: 480.0,
            height: 800.0,
        },
        media: MediaEnvironment::default(),
        ..Environment::default()
    };
    let map = resolver().resolve(&a.tree, &narrow);
    assert_eq!(value(&map, a.row, "padding-top"), Value::px(4.0));
}
