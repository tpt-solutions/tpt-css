// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! CSS counters (CSS 2.1 section 12.4.1) through the style pass: nested and
//! sibling scopes, `::after` after the subtree, and restyle agreeing with a full
//! resolve.

use super::common::{parse_sheet, TestTree};
use tpt_css::properties::lookup;
use tpt_css::selectors::PseudoElement;
use tpt_css::tree::NodeKind;
use tpt_css::values::Value;
use tpt_css::{Environment, Origin, ResolverConfig, StyleChange, StyleMap, StyleResolver};

fn resolver_for(css: &str) -> StyleResolver {
    let mut r = StyleResolver::new(ResolverConfig::default());
    r.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
    r
}

fn full(css: &str, tree: &TestTree, env: &Environment) -> StyleMap {
    resolver_for(css).resolve(tree, env).clone()
}

/// The `content` value of the pseudo-element `pseudo` on `node`.
fn content_of(map: &StyleMap, node: tpt_css::tree::NodeId, pseudo: PseudoElement) -> Value {
    map.get_pseudo(node, &pseudo)
        .expect("pseudo-element is generated")
        .raw(lookup("content").unwrap())
        .clone()
}

fn text(s: &str) -> Value {
    Value::Strings(vec![s.to_string()])
}

#[test]
fn nested_lists_count_with_counters() {
    let css = "
        ol { counter-reset: item }
        li { counter-increment: item }
        li::before { content: counters(item, \".\") }
    ";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let outer = t.element(root, "ol");
    let first = t.element(outer, "li");
    let inner = t.element(first, "ol");
    let inner_a = t.element(inner, "li");
    let inner_b = t.element(inner, "li");
    let second = t.element(outer, "li");
    let env = Environment::default();
    let map = full(css, &t, &env);

    assert_eq!(
        content_of(&map, inner_a, PseudoElement::Before),
        text("1.1")
    );
    assert_eq!(
        content_of(&map, inner_b, PseudoElement::Before),
        text("1.2")
    );
    // The nested list ended with the first item, so the second item is 2.
    assert_eq!(content_of(&map, second, PseudoElement::Before), text("2"));
}

#[test]
fn after_sees_the_counts_of_the_subtree() {
    let css = "
        ol { counter-reset: n }
        div, span { counter-increment: n }
        div::before { content: counter(n) }
        div::after { content: counter(n) }
    ";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let list = t.element(root, "ol");
    let div = t.element(list, "div");
    t.element(div, "span");
    t.element(div, "span");
    let env = Environment::default();
    let map = full(css, &t, &env);

    assert_eq!(content_of(&map, div, PseudoElement::Before), text("1"));
    // 1 for the div, then 1 for each span.
    assert_eq!(content_of(&map, div, PseudoElement::After), text("3"));
}

#[test]
fn counter_set_and_explicit_increments() {
    let css = "
        p { counter-set: x 5 }
        p + p { counter-increment: x 2 }
        p::before { content: counter(x) }
        p + p::before { content: counter(x, upper-roman) }
    ";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let first = t.element(root, "p");
    let second = t.element(root, "p");
    let env = Environment::default();
    let map = full(css, &t, &env);

    assert_eq!(content_of(&map, first, PseudoElement::Before), text("5"));
    // Set to 5, then incremented by 2 by the second paragraph's rule: 7 = VII.
    assert_eq!(content_of(&map, second, PseudoElement::Before), text("VII"));
}

#[test]
fn counters_outside_their_scope_read_as_zero() {
    let css = "p::before { content: counter(missing) }";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let p = t.element(root, "p");
    let map = full(css, &t, &Environment::default());
    assert_eq!(content_of(&map, p, PseudoElement::Before), text("0"));
}

#[test]
fn restyle_with_counters_matches_a_full_resolve() {
    let css = "
        ol { counter-reset: item }
        li { counter-increment: item }
        li::before { content: counter(item) }
    ";
    let env = Environment::default();
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let list = t.element(root, "ol");
    let a = t.element(list, "li");
    t.element(list, "li");
    t.element(list, "li");

    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);
    t.set_attr(a, "data-x", "changed");
    r.invalidate(&t, StyleChange::AttributeNamed(a, "data-x".to_string()));
    let restyled = r.restyle(&t, &env).clone();
    assert_eq!(restyled, full(css, &t, &env));
}

#[test]
fn counter_reset_is_restricted_to_names_and_integers() {
    // Counter names cannot be `none`. Styles are checked when `content` is parsed.
    assert!(
        parse_sheet("p { counter-reset: none }")
            .stylesheet
            .rules
            .len()
            == 1
    );
    // An unknown list style makes the whole `content` declaration invalid, so no
    // `::before` is generated.
    let t = {
        let mut t = TestTree::new(NodeKind::Element);
        let root = t.root();
        t.element(root, "p");
        t
    };
    let map = full(
        "p::before { content: counter(x, circle) }",
        &t,
        &Environment::default(),
    );
    let p = tpt_css::tree::NodeId::new(1, 0);
    assert!(map.get_pseudo(p, &PseudoElement::Before).is_none());
}

#[test]
fn list_items_count_without_an_explicit_increment() {
    // `display: list-item` increments the `list-item` counter implicitly, so the
    // author only resets it on the list.
    let css = "
        ol { counter-reset: list-item }
        li { display: list-item }
        li::before { content: counter(list-item) \". \" }
    ";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let list = t.element(root, "ol");
    let first = t.element(list, "li");
    let second = t.element(list, "li");
    let map = full(css, &t, &Environment::default());
    assert_eq!(
        content_of(&map, first, PseudoElement::Before),
        Value::Strings(vec!["1".to_string(), ". ".to_string()])
    );
    assert_eq!(
        content_of(&map, second, PseudoElement::Before),
        Value::Strings(vec!["2".to_string(), ". ".to_string()])
    );
}

#[test]
fn restyle_sees_a_display_change_that_starts_counting() {
    let css = "
        ol { counter-reset: list-item }
        li { counter-increment: none }
        li.item { display: list-item }
        li::before { content: counter(list-item) }
    ";
    let env = Environment::default();
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let list = t.element(root, "ol");
    let a = t.element(list, "li");
    let b = t.element(list, "li");
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);
    t.add_class(b, "item");
    r.invalidate(&t, StyleChange::AttributeNamed(b, "class".to_string()));
    let restyled = r.restyle(&t, &env).clone();
    assert_eq!(restyled, full(css, &t, &env));
    assert_eq!(content_of(&restyled, b, PseudoElement::Before), text("1"));
    let _ = a;
}
