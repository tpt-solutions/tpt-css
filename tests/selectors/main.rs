// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector matching tests (spec 12.4) against the test tree.

#[path = "../common/mod.rs"]
mod common;
mod m3;
mod tier1;

use common::{parse_sheet, TestTree};
use tpt_css::ast::Rule;
use tpt_css::selectors::match_selector_list;
use tpt_css::tree::{NodeKind, StateBits, StyleTree};

/// `html > body > div.box#main > p.lead > span` plus a sibling `p`.
fn sample() -> (
    TestTree,
    tpt_css::tree::NodeId,
    tpt_css::tree::NodeId,
    tpt_css::tree::NodeId,
) {
    let mut t = TestTree::new(NodeKind::Element);
    let html = t.root();
    t.nodes[0].name.local = "html".into();
    let body = t.element(html, "body");
    let div = t.element(body, "div");
    t.add_class(div, "box").set_id(div, "main");
    let p = t.element(div, "p");
    t.add_class(p, "lead");
    let span = t.element(p, "span");
    let sibling = t.element(div, "p");
    let _ = sibling;
    (t, div, p, span)
}

fn matches(tree: &TestTree, node: tpt_css::tree::NodeId, selector: &str) -> bool {
    let sheet = parse_sheet(&format!("{selector} {{ }}")).stylesheet;
    let Rule::Style(rule) = &sheet.rules[0] else {
        panic!("expected a style rule");
    };
    match_selector_list(tree, node, &rule.selectors, 100_000).is_some()
}

#[test]
fn type_class_id_and_universal() {
    let (t, div, p, _) = sample();
    assert!(matches(&t, div, "div"));
    assert!(matches(&t, div, ".box"));
    assert!(matches(&t, div, "#main"));
    assert!(matches(&t, div, "*"));
    assert!(!matches(&t, div, "#other"));
    assert!(!matches(&t, p, ".box"));
}

#[test]
fn descendant_and_child_combinators() {
    let (t, _, _, span) = sample();
    assert!(matches(&t, span, "div span"));
    assert!(matches(&t, span, "div > p > span"));
    assert!(!matches(&t, span, "body > span"));
    assert!(matches(&t, span, ".box .lead span"));
}

#[test]
fn child_combinator_requires_the_immediate_parent() {
    let (t, _, _, span) = sample();
    assert!(!matches(&t, span, "div > span"));
}

#[test]
fn attribute_presence_and_exact_value() {
    let (mut t, div, _, _) = sample();
    t.set_attr(div, "data-kind", "panel");
    assert!(matches(&t, div, "[data-kind]"));
    assert!(matches(&t, div, "[data-kind=panel]"));
    assert!(matches(&t, div, "[data-kind=\"panel\"]"));
    assert!(
        !matches(&t, div, "[data-kind=PANEL]"),
        "values are case-sensitive in the core"
    );
    assert!(!matches(&t, div, "[missing]"));
}

#[test]
fn root_matches_the_top_element_only() {
    let (t, div, _, _) = sample();
    assert!(matches(&t, t.root(), ":root"));
    assert!(!matches(&t, div, ":root"));
}

#[test]
fn state_pseudo_classes_use_adapter_state() {
    let (mut t, div, _, _) = sample();
    assert!(!matches(&t, div, ":hover"));
    t.set_state(div, StateBits::HOVER);
    assert!(matches(&t, div, ":hover"));
    assert!(!matches(&t, div, ":focus"));
}

#[test]
fn text_and_document_nodes_never_match() {
    let (mut t, div, _, _) = sample();
    let text = t.add_text(div);
    assert!(!matches(&t, text, "*"));
    assert_eq!(t.kind(text), NodeKind::Text);
}

#[test]
fn a_zero_step_budget_fails_closed() {
    let (t, div, _, _) = sample();
    let sheet = parse_sheet("div { }").stylesheet;
    let Rule::Style(rule) = &sheet.rules[0] else {
        panic!("expected a style rule");
    };
    assert!(match_selector_list(&t, div, &rule.selectors, 0).is_none());
}

#[test]
fn selector_list_reports_the_most_specific_matching_member() {
    let (t, div, _, _) = sample();
    let sheet = parse_sheet("div, #main { }").stylesheet;
    let Rule::Style(rule) = &sheet.rules[0] else {
        panic!("expected a style rule");
    };
    let spec = match_selector_list(&t, div, &rule.selectors, 100_000).unwrap();
    assert_eq!(spec.a, 1, "#main is the most specific match");
}
