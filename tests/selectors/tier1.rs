// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Tier 1 selector tests (spec 12.2): attribute operators, structural and
//! functional pseudo-classes, sibling combinators, form states, `:lang()`, and
//! `:dir()`.

use super::common::{parse_sheet, TestTree};
use tpt_css::ast::Rule;
use tpt_css::selectors::{match_selector_list, Specificity};
use tpt_css::tree::{Direction, NodeId, NodeKind, StateBits, StyleTree};

fn selector_list(selector: &str) -> tpt_css::ast::SelectorList {
    let sheet = parse_sheet(&format!("{selector} {{ }}")).stylesheet;
    match &sheet.rules[0] {
        Rule::Style(rule) => rule.selectors.clone(),
        _ => panic!("expected a style rule for {selector}"),
    }
}

fn matches(tree: &TestTree, node: NodeId, selector: &str) -> bool {
    match_selector_list(tree, node, &selector_list(selector), 100_000).is_some()
}

fn specificity(selector: &str) -> Specificity {
    selector_list(selector).selectors[0].specificity
}

/// `ul` with five `li` children and one `p` at the end.
fn list() -> (TestTree, Vec<NodeId>) {
    let mut t = TestTree::new(NodeKind::Element);
    t.nodes[0].name.local = "ul".into();
    let root = t.root();
    let items: Vec<NodeId> = (0..5).map(|_| t.element(root, "li")).collect();
    t.element(root, "p");
    (t, items)
}

#[test]
fn attribute_includes_matches_a_whitespace_separated_token() {
    let (mut t, items) = list();
    t.set_attr(items[0], "data-tags", "red big");
    assert!(matches(&t, items[0], "[data-tags~=big]"));
    assert!(!matches(&t, items[0], "[data-tags~=bi]"));
    assert!(!matches(&t, items[0], "[data-tags~=\"\"]"));
}

#[test]
fn attribute_dash_match_matches_value_or_value_then_dash() {
    let (mut t, items) = list();
    t.set_attr(items[0], "lang", "en-US");
    t.set_attr(items[1], "lang", "english");
    assert!(matches(&t, items[0], "[lang|=en]"));
    assert!(matches(&t, items[0], "[lang|=en-US]"));
    assert!(!matches(&t, items[1], "[lang|=en]"));
}

#[test]
fn attribute_prefix_suffix_and_substring() {
    let (mut t, items) = list();
    t.set_attr(items[0], "href", "https://example.test/file.pdf");
    assert!(matches(&t, items[0], "[href^=https]"));
    assert!(matches(&t, items[0], "[href$=\".pdf\"]"));
    assert!(matches(&t, items[0], "[href*=example]"));
    assert!(!matches(&t, items[0], "[href^=\"http://\"]"));
    assert!(
        !matches(&t, items[0], "[href^=\"\"]"),
        "empty prefix never matches"
    );
}

#[test]
fn first_last_and_only_child() {
    let (t, items) = list();
    assert!(matches(&t, items[0], "li:first-child"));
    assert!(!matches(&t, items[1], "li:first-child"));
    // The p is the last child, so the last li is not :last-child.
    assert!(!matches(&t, items[4], "li:last-child"));
    assert!(!matches(&t, items[4], ":last-child"));
    let root_child = t.children(t.root())[5];
    assert!(matches(&t, root_child, "p:last-child"));
    assert!(!matches(&t, items[0], "li:only-child"));
}

#[test]
fn nth_child_formulas() {
    let (t, items) = list();
    // 2n+1 selects positions 1, 3, 5.
    let odd: Vec<bool> = items
        .iter()
        .map(|n| matches(&t, *n, "li:nth-child(2n+1)"))
        .collect();
    assert_eq!(odd, vec![true, false, true, false, true]);
    let even: Vec<bool> = items
        .iter()
        .map(|n| matches(&t, *n, "li:nth-child(even)"))
        .collect();
    assert_eq!(even, vec![false, true, false, true, false]);
    // -n+3 selects the first three.
    let first_three: Vec<bool> = items
        .iter()
        .map(|n| matches(&t, *n, "li:nth-child(-n+3)"))
        .collect();
    assert_eq!(first_three, vec![true, true, true, false, false]);
    // A plain integer selects exactly one position.
    assert!(matches(&t, items[3], "li:nth-child(4)"));
    assert!(!matches(&t, items[3], "li:nth-child(3)"));
}

#[test]
fn nth_last_child_counts_from_the_end() {
    let (t, items) = list();
    // The p is the last child, so the last li is second from the end.
    assert!(matches(&t, items[4], "li:nth-last-child(2)"));
    assert!(matches(&t, items[4], "li:nth-last-child(2)"));
}

#[test]
fn nth_of_type_counts_only_the_same_name() {
    let (t, _) = list();
    let p = t.children(t.root())[5];
    assert!(matches(&t, p, "p:first-of-type"));
    assert!(matches(&t, p, "p:only-of-type"));
    let first_li = t.children(t.root())[0];
    assert!(matches(&t, first_li, "li:first-of-type"));
    assert!(!matches(&t, t.children(t.root())[1], "li:first-of-type"));
}

#[test]
fn nth_child_of_a_selector_filters_the_siblings() {
    let (mut t, items) = list();
    t.add_class(items[1], "hit");
    t.add_class(items[3], "hit");
    // Among the .hit siblings, items[1] is first and items[3] is second.
    assert!(matches(&t, items[1], "li:nth-child(1 of .hit)"));
    assert!(matches(&t, items[3], "li:nth-child(2 of .hit)"));
    assert!(!matches(&t, items[3], "li:nth-child(1 of .hit)"));
}

#[test]
fn not_is_and_where() {
    let (mut t, items) = list();
    t.add_class(items[0], "a");
    t.add_class(items[1], "b");
    assert!(!matches(&t, items[0], "li:not(.a)"));
    assert!(matches(&t, items[1], "li:not(.a)"));
    assert!(matches(&t, items[1], "li:is(.a, .b)"));
    assert!(matches(&t, items[0], ":where(.a)"));
    assert!(!matches(&t, items[2], ":is(.a, .b)"));
}

#[test]
fn is_and_not_take_the_most_specific_argument_but_where_is_zero() {
    assert_eq!(specificity(":is(#x, .y)"), Specificity { a: 1, b: 0, c: 0 });
    assert_eq!(specificity(":not(.z)"), Specificity { a: 0, b: 1, c: 0 });
    assert_eq!(specificity(":where(#x)"), Specificity { a: 0, b: 0, c: 0 });
    assert_eq!(
        specificity("li:nth-child(2n of #x)"),
        Specificity { a: 1, b: 1, c: 1 }
    );
}

#[test]
fn adjacent_and_general_sibling_combinators() {
    let (mut t, items) = list();
    t.add_class(items[1], "marker");
    // + is the immediately preceding element sibling.
    assert!(matches(&t, items[2], ".marker + li"));
    assert!(!matches(&t, items[3], ".marker + li"));
    // ~ is any preceding element sibling.
    assert!(matches(&t, items[4], ".marker ~ li"));
    assert!(!matches(&t, items[0], ".marker ~ li"));
}

#[test]
fn empty_matches_no_element_children_and_no_text_content() {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let empty = t.element(root, "div");
    let blank_text = t.element(root, "div");
    t.add_text(blank_text);
    let with_text = t.element(root, "div");
    t.add_text_with(with_text, "hello");
    assert!(matches(&t, empty, "div:empty"));
    assert!(
        matches(&t, blank_text, "div:empty"),
        "an empty text node is not content"
    );
    assert!(!matches(&t, with_text, "div:empty"));
}

#[test]
fn form_state_pseudo_classes_read_state_bits() {
    let (mut t, items) = list();
    t.set_state(items[0], StateBits::DISABLED);
    t.set_state(items[1], StateBits::CHECKED.union(StateBits::REQUIRED));
    assert!(matches(&t, items[0], "li:disabled"));
    assert!(!matches(&t, items[0], "li:enabled"));
    assert!(matches(&t, items[1], "li:checked:required"));
    assert!(!matches(&t, items[1], "li:optional"));
}

#[test]
fn lang_matches_the_language_or_a_subtag_and_is_inherited() {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.set_lang(root, "en-US");
    let child = t.element(root, "p");
    let other = t.element(root, "p");
    t.set_lang(other, "fr");
    assert!(matches(&t, child, ":lang(en)"));
    assert!(
        matches(&t, child, ":lang(en-us)"),
        "tags compare case-insensitively"
    );
    assert!(
        !matches(&t, child, ":lang(e)"),
        "a partial subtag never matches"
    );
    assert!(!matches(&t, other, ":lang(en)"));
}

#[test]
fn dir_matches_the_inherited_direction() {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.set_dir(root, Direction::Rtl);
    let child = t.element(root, "p");
    assert!(matches(&t, child, ":dir(rtl)"));
    assert!(!matches(&t, child, ":dir(ltr)"));
}

#[test]
fn pseudo_element_selectors_parse_to_their_target() {
    let sheet = parse_sheet("p::before, a:after, ::marker { }").stylesheet;
    let Rule::Style(rule) = &sheet.rules[0] else {
        panic!("expected a style rule");
    };
    let targets: Vec<_> = rule
        .selectors
        .selectors
        .iter()
        .map(|s| s.pseudo_element.clone())
        .collect();
    assert_eq!(
        targets,
        vec![
            Some(tpt_css::selectors::PseudoElement::Before),
            Some(tpt_css::selectors::PseudoElement::After),
            Some(tpt_css::selectors::PseudoElement::Marker),
        ]
    );
}

#[test]
fn pseudo_element_must_end_the_selector() {
    let parsed = parse_sheet("p::before span { color: red }");
    assert!(parsed.stylesheet.rules.is_empty());
}
