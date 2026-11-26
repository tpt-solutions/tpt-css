// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `:has()`, `:scope`, and `@scope` (Selectors 4 section 4.5; CSS Cascade 6).

use super::common::{style_of, style_with, TestTree};
use tpt_css::properties::lookup;
use tpt_css::tree::{NodeId, NodeKind};
use tpt_css::values::Value;

fn margin(css: &str, t: &TestTree, node: NodeId) -> Value {
    let map = style_with(css, t);
    style_of(&map, node)
        .raw(lookup("margin-left").unwrap())
        .clone()
}

/// root > section(card) > [img, p > span]; section2 has no img.
fn doc() -> (TestTree, NodeId, NodeId, NodeId, NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let card = t.element(root, "section");
    t.add_class(card, "card");
    let img = t.element(card, "img");
    let p = t.element(card, "p");
    let span = t.element(p, "span");
    let plain = t.element(root, "section");
    (t, card, img, span, plain)
}

#[test]
fn has_matches_a_descendant() {
    let (t, card, _, _, plain) = doc();
    let css = "section:has(span) { margin-left: 2px }";
    assert_eq!(
        margin(css, &t, card),
        Value::px(2.0),
        "span is a grandchild of card"
    );
    let css = "section:has(p span) { margin-left: 2px }";
    assert_eq!(margin(css, &t, card), Value::px(2.0));
    assert_eq!(margin(css, &t, plain), Value::px(0.0));
}

#[test]
fn has_with_a_child_combinator_requires_a_direct_child() {
    let (t, card, _, _, _) = doc();
    assert_eq!(
        margin("section:has(> img) { margin-left: 3px }", &t, card),
        Value::px(3.0)
    );
    assert_eq!(
        margin("section:has(> span) { margin-left: 3px }", &t, card),
        Value::px(0.0)
    );
}

#[test]
fn has_with_a_sibling_combinator() {
    let (t, _, img, _, _) = doc();
    let css = "img:has(+ p) { margin-left: 4px }";
    assert_eq!(
        margin(css, &t, img),
        Value::px(4.0),
        "p is the next sibling of img"
    );
    let css = "img:has(~ span) { margin-left: 4px }";
    assert_eq!(margin(css, &t, img), Value::px(0.0));
}

#[test]
fn has_lists_are_alternatives() {
    let (t, _, _, _, plain) = doc();
    let css = "section:has(img, span) { margin-left: 5px }";
    assert_eq!(margin(css, &t, plain), Value::px(0.0));
    let css = "section:has(img, p) { margin-left: 5px }";
    let (t2, card, _, _, _) = doc();
    assert_eq!(margin(css, &t2, card), Value::px(5.0));
}

#[test]
fn has_specificity_is_the_most_specific_argument() {
    let sheet = super::common::parse_sheet("div:has(#a, .b) { }").stylesheet;
    let tpt_css::ast::Rule::Style(rule) = &sheet.rules[0] else {
        panic!("expected a style rule");
    };
    let spec = rule.selectors.selectors[0].specificity;
    // :has() takes its most specific argument (#a); div adds one type point.
    assert_eq!((spec.a, spec.b, spec.c), (1, 0, 1));
}

#[test]
fn an_empty_has_is_invalid() {
    let parsed = tpt_css::parse_stylesheet("div:has() { margin-left: 1px }", Default::default());
    assert!(parsed.stylesheet.rules.is_empty());
}

#[test]
fn scope_without_a_root_is_the_root_element() {
    let (t, card, _, _, _) = doc();
    let css = "@scope { :scope { margin-left: 6px } }";
    let map = style_with(css, &t);
    assert_eq!(
        style_of(&map, t.root()).raw(lookup("margin-left").unwrap()),
        &Value::px(6.0)
    );
    assert_eq!(
        style_of(&map, card).raw(lookup("margin-left").unwrap()),
        &Value::px(0.0)
    );
}

#[test]
fn scope_root_applies_its_rules_to_the_subtree_only() {
    let (t, card, img, span, plain) = doc();
    let css = "@scope (.card) { img { margin-left: 7px } }";
    assert_eq!(margin(css, &t, img), Value::px(7.0));
    assert_eq!(margin(css, &t, plain), Value::px(0.0));
    let _ = (card, span);
}

#[test]
fn scope_colon_scope_refers_to_the_root() {
    let (t, card, _, _, _) = doc();
    let css = "@scope (.card) { :scope { margin-left: 8px } }";
    assert_eq!(margin(css, &t, card), Value::px(8.0));
}

#[test]
fn scope_limit_excludes_the_subtree_below_it() {
    let (t, _, img, span, _) = doc();
    // The p is the limit, so span (below it) is outside the scope; img is not.
    let css = "@scope (.card) to (p) { span { margin-left: 9px } img { margin-left: 9px } }";
    assert_eq!(margin(css, &t, span), Value::px(0.0));
    assert_eq!(margin(css, &t, img), Value::px(9.0));
}

#[test]
fn a_scope_root_with_no_matching_ancestor_applies_nowhere() {
    let (t, _, img, _, _) = doc();
    let css = "@scope (.missing) { img { margin-left: 1px } }";
    assert_eq!(margin(css, &t, img), Value::px(0.0));
}

#[test]
fn a_malformed_scope_prelude_drops_the_rule() {
    let parsed = tpt_css::parse_stylesheet(
        "@scope (.a) junk { img { margin-left: 1px } }",
        Default::default(),
    );
    assert!(parsed.stylesheet.rules.is_empty());
}

#[test]
fn time_state_functions_need_the_state_and_the_argument() {
    use tpt_css::tree::StateBits;
    let css =
        "li:current(.a) { color: red } li:past(.a) { color: green } li:future { color: blue }";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let list = t.element(root, "ul");
    let current_a = t.element(list, "li");
    t.add_class(current_a, "a");
    t.set_state(current_a, StateBits::CURRENT);
    let past_a = t.element(list, "li");
    t.add_class(past_a, "a");
    t.set_state(past_a, StateBits::PAST);
    let current_plain = t.element(list, "li");
    t.set_state(current_plain, StateBits::CURRENT);
    let future = t.element(list, "li");
    t.set_state(future, StateBits::FUTURE);

    let color = |node| style_of(&style_with(css, &t), node).color();
    assert_eq!(color(current_a), tpt_css::values::Color::rgb8(255, 0, 0));
    assert_eq!(color(past_a), tpt_css::values::Color::rgb8(0, 128, 0));
    // Current but not `.a`: no rule matches it.
    assert_eq!(color(current_plain), tpt_css::values::Color::BLACK);
    assert_eq!(color(future), tpt_css::values::Color::rgb8(0, 0, 255));
}
