// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cascade layers (spec 13.4) and the `revert` keywords (spec 13.8).

use super::common::{parse_sheet, style_of, style_with, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::properties::lookup;
use tpt_css::tree::{NodeId, NodeKind, StateBits};
use tpt_css::values::Value;
use tpt_css::{Environment, ResolverConfig, StyleResolver};

/// root > child, the child with class `x`.
fn chain() -> (TestTree, NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    t.add_class(child, "x");
    (t, child)
}

fn margin_left(css: &str) -> Value {
    let (t, child) = chain();
    let map = style_with(css, &t);
    style_of(&map, child)
        .raw(lookup("margin-left").unwrap())
        .clone()
}

fn px(v: f64) -> Value {
    Value::px(v)
}

#[test]
fn unlayered_normal_beats_layered_normal_whatever_the_specificity() {
    assert_eq!(
        margin_left("@layer base { child.x { margin-left: 1px } } child { margin-left: 2px }"),
        px(2.0)
    );
}

#[test]
fn a_later_layer_beats_an_earlier_layer_for_normal_declarations() {
    // The statement fixes the order: reset, then base.
    assert_eq!(
        margin_left(
            "@layer reset, base; @layer base { child { margin-left: 1px } } @layer reset { child { margin-left: 2px } }"
        ),
        px(1.0)
    );
}

#[test]
fn an_earlier_layer_beats_a_later_layer_for_important_declarations() {
    assert_eq!(
        margin_left(
            "@layer a { child { margin-left: 1px !important } } @layer b { child { margin-left: 2px !important } }"
        ),
        px(1.0)
    );
}

#[test]
fn layered_important_beats_unlayered_important() {
    assert_eq!(
        margin_left("child { margin-left: 5px !important } @layer a { child { margin-left: 1px !important } }"),
        px(1.0)
    );
}

#[test]
fn a_later_anonymous_layer_wins_over_an_earlier_one() {
    assert_eq!(
        margin_left("@layer { child { margin-left: 1px } } @layer { child { margin-left: 2px } }"),
        px(2.0)
    );
}

#[test]
fn nested_layer_names_are_dotted_and_shared_with_top_level_names() {
    // `a.b` declared inside `a` is the same layer as a top-level `a.b`.
    assert_eq!(
        margin_left("@layer a { @layer b { child { margin-left: 1px } } } @layer a.b { child { margin-left: 3px } }"),
        px(3.0)
    );
}

#[test]
fn style_rules_in_a_layer_keep_their_declared_order_inside_the_layer() {
    assert_eq!(
        margin_left("@layer a { child { margin-left: 1px; margin-left: 4px } }"),
        px(4.0)
    );
}

#[test]
fn revert_rolls_back_to_the_user_agent_origin() {
    let (t, child) = chain();
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(
        parse_sheet("child { margin-left: 7px }").stylesheet,
        Origin::UserAgent,
    );
    resolver.add_stylesheet(
        parse_sheet("child { margin-left: 2px } child.x { margin-left: revert }").stylesheet,
        Origin::Author,
    );
    let map = resolver.resolve(&t, &Environment::default());
    assert_eq!(
        style_of(&map, child).raw(lookup("margin-left").unwrap()),
        &px(7.0)
    );
}

#[test]
fn revert_with_nothing_below_is_unset() {
    assert_eq!(
        margin_left("child { margin-left: 3px } child.x { margin-left: revert }"),
        px(0.0)
    );
}

#[test]
fn revert_layer_rolls_back_to_the_previous_layer() {
    assert_eq!(
        margin_left("@layer a { child { margin-left: 1px } } @layer b { child { margin-left: 3px } child { margin-left: revert-layer } }"),
        px(1.0)
    );
}

#[test]
fn revert_layer_in_the_first_layer_is_unset() {
    assert_eq!(
        margin_left("@layer a { child { margin-left: 3px; margin-left: revert-layer } }"),
        px(0.0)
    );
}

#[test]
fn revert_layer_in_unlayered_rolls_back_to_the_last_layer() {
    assert_eq!(
        margin_left("@layer a { child { margin-left: 1px } } child { margin-left: revert-layer }"),
        px(1.0)
    );
}

#[test]
fn an_element_with_no_matching_layered_rule_is_unaffected_by_layers() {
    let (mut t, child) = chain();
    t.set_state(child, StateBits::empty());
    let map = style_with(
        "@layer a { other { margin-left: 1px } } child { margin-left: 4px }",
        &t,
    );
    assert_eq!(
        style_of(&map, child).raw(lookup("margin-left").unwrap()),
        &px(4.0)
    );
}
