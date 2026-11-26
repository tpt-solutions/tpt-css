// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Incremental restyle tests (spec 8.2, 17): a restyle must match a full
//! resolve, and a local change must restyle only the local part of the tree.

use super::common::{parse_sheet, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::properties::lookup;
use tpt_css::tree::{NodeId, NodeKind, StateBits};
use tpt_css::values::{Color, Value};
use tpt_css::{Environment, ResolverConfig, StyleChange, StyleResolver};

/// `root` with 50 `div` children. Each child has one `span` descendant with
/// one `em` below it.
fn big_tree() -> (TestTree, Vec<NodeId>) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let mut divs = Vec::new();
    for _ in 0..50 {
        let div = t.element(root, "div");
        let span = t.element(div, "span");
        t.element(span, "em");
        divs.push(div);
    }
    (t, divs)
}

fn resolver_for(css: &str) -> StyleResolver {
    let mut r = StyleResolver::new(ResolverConfig::default());
    r.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
    r
}

fn full(css: &str, tree: &TestTree, env: &Environment) -> tpt_css::StyleMap {
    resolver_for(css).resolve(tree, env)
}

#[test]
fn an_attribute_change_restyles_only_the_changed_element_when_no_rule_is_structural() {
    let css = ".hot { color: red } div { margin-left: 1px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    t.add_class(divs[7], "hot");
    r.invalidate(&t, StyleChange::AttributeChanged(divs[7]));
    let map = r.restyle(&t, &env);

    // div, span, and em under it: three elements, not the whole 151-element tree.
    assert_eq!(r.last_restyle_count(), 3);
    assert_eq!(map.get(divs[7]).unwrap().color(), Color::rgb8(255, 0, 0));
    assert_eq!(
        map,
        full(css, &t, &env),
        "restyle must match a full resolve"
    );
}

#[test]
fn a_state_change_restyles_the_element_and_its_subtree() {
    let css = "div:hover { color: blue } span { margin-left: 2px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    t.set_state(divs[3], StateBits::HOVER);
    r.invalidate(&t, StyleChange::StateChanged(divs[3]));
    let map = r.restyle(&t, &env);

    assert_eq!(map.get(divs[3]).unwrap().color(), Color::rgb8(0, 0, 255));
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn inline_style_changes_are_restyled() {
    let css = "div { color: red }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    t.set_inline(divs[0], "color: lime");
    r.invalidate(&t, StyleChange::InlineStyleChanged(divs[0]));
    let map = r.restyle(&t, &env);
    assert_eq!(map.get(divs[0]).unwrap().color(), Color::rgb8(0, 255, 0));
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn sibling_sensitive_rules_restyle_the_parent_subtree() {
    // Changing one child can change its siblings through `+`, so the whole
    // parent subtree is restyled.
    let css = "div + div { margin-top: 4px } .mark { color: red }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    t.add_class(divs[10], "mark");
    r.invalidate(&t, StyleChange::AttributeChanged(divs[10]));
    let map = r.restyle(&t, &env);

    assert!(r.last_restyle_count() > 3, "siblings must be restyled");
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn an_environment_change_forces_a_full_resolve() {
    let css = "@media (min-width: 500px) { div { margin-left: 3px } }";
    let (t, divs) = big_tree();
    let wide = Environment::default();
    let narrow = Environment {
        viewport: tpt_css::ViewportInfo {
            width: 300.0,
            height: 600.0,
        },
        ..Environment::default()
    };
    let mut r = resolver_for(css);
    let first = r.resolve(&t, &wide);
    let margin = lookup("margin-left").unwrap();
    assert_eq!(first.get(divs[0]).unwrap().raw(margin), &Value::px(3.0));

    let second = r.restyle(&t, &narrow);
    assert_eq!(second.get(divs[0]).unwrap().raw(margin), &Value::px(0.0));
    assert_eq!(second, full(css, &t, &narrow));
}

#[test]
fn a_stylesheet_change_restyles_everything() {
    let (t, divs) = big_tree();
    let env = Environment::default();
    let mut r = resolver_for("div { color: red }");
    let _ = r.resolve(&t, &env);
    let id = r.add_stylesheet(
        parse_sheet("div { color: blue }").stylesheet,
        Origin::Author,
    );
    r.invalidate(&t, StyleChange::StyleSheetChanged);
    let map = r.restyle(&t, &env);
    // Later sheets win at equal specificity, so blue wins everywhere.
    assert_eq!(map.get(divs[0]).unwrap().color(), Color::rgb8(0, 0, 255));
    r.remove_stylesheet(id);
    r.invalidate(&t, StyleChange::StyleSheetChanged);
    let map = r.restyle(&t, &env);
    assert_eq!(map.get(divs[0]).unwrap().color(), Color::rgb8(255, 0, 0));
}

#[test]
fn restyle_without_a_previous_result_is_a_full_resolve() {
    let css = "div { color: red }";
    let env = Environment::default();
    let (t, _) = big_tree();
    let mut r = resolver_for(css);
    let map = r.restyle(&t, &env);
    assert_eq!(map, full(css, &t, &env));
    assert_eq!(
        r.last_restyle_count(),
        151,
        "the root, then 50 divs, spans, and ems"
    );
}

#[test]
fn a_restyle_with_nothing_dirty_reuses_the_previous_result() {
    let css = "div { color: red }";
    let env = Environment::default();
    let (t, _) = big_tree();
    let mut r = resolver_for(css);
    let first = r.resolve(&t, &env);
    let again = r.restyle(&t, &env);
    assert_eq!(first, again);
    assert_eq!(r.last_restyle_count(), 0);
}
