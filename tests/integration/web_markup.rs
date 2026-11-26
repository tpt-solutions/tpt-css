// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The web profile end to end (spec 5.2): markup, its `<style>` sheets, and
//! the computed styles that come out. The markup reader is test support only
//! (see `tests/common/markup.rs`); the engine itself never parses markup.

use super::common::markup;
use tpt_css::properties::lookup;
use tpt_css::selectors::PseudoElement;
use tpt_css::values::{Color, Value};
use tpt_css::{Environment, Origin, ResolverConfig, StyleMap, StyleResolver, StyleTree};

const DOCUMENT: &str = r##"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <!-- the test document -->
  <style>
    body { color: rgb(0, 0, 255); font-size: 20px }
    .note { color: green }
    #main > p + p { margin-top: 4px }
    p::before { content: "> " }
    ol { counter-reset: item }
    li { counter-increment: item }
    li::before { content: counter(item) ". " }
    a:hover { color: red }
  </style>
</head>
<body>
  <div id="main">
    <p class="note">Hello</p>
    <p style="font-style: italic">World</p>
    <ol><li>one</li><li>two</li></ol>
    <a href="#">link</a><br>
  </div>
</body>
</html>"##;

/// Resolves the document's own `<style>` sheets with the default environment.
fn resolve_document(source: &str) -> (markup::Markup, StyleMap) {
    let doc = markup::read(source);
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    for css in &doc.style_sheets {
        let parsed = tpt_css::parse_stylesheet(css, Default::default());
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            parsed.diagnostics
        );
        resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
    }
    let map = resolver.resolve(&doc.tree, &Environment::default()).clone();
    (doc, map)
}

/// The first element named `name` in document order.
fn find(tree: &impl StyleTree, name: &str) -> tpt_css::tree::NodeId {
    let mut stack = vec![tree.root()];
    while let Some(node) = stack.pop() {
        if tree.kind(node) == tpt_css::tree::NodeKind::Element && tree.name(node).local == name {
            return node;
        }
        stack.extend(tree.children(node).iter().rev().copied());
    }
    panic!("no <{name}> in the document")
}

/// The `n`th element named `name` in document order (from 0).
fn find_nth(tree: &impl StyleTree, name: &str, n: usize) -> tpt_css::tree::NodeId {
    let mut found = Vec::new();
    let mut stack = vec![tree.root()];
    while let Some(node) = stack.pop() {
        if tree.kind(node) == tpt_css::tree::NodeKind::Element && tree.name(node).local == name {
            found.push(node);
        }
        stack.extend(tree.children(node).iter().rev().copied());
    }
    found[n]
}

#[test]
fn class_and_inheritance_from_the_document_sheet() {
    let (doc, map) = resolve_document(DOCUMENT);
    let p = find(&doc.tree, "p");
    let style = map.get(p).expect("p is styled");
    // `.note` overrides the inherited body color.
    assert_eq!(style.color(), Color::rgb8(0, 128, 0));
    // Inherited from body through div#main.
    assert_eq!(style.font_size(), 20.0);
}

#[test]
fn id_child_combinator_and_adjacent_sibling_match_markup() {
    let (doc, map) = resolve_document(DOCUMENT);
    let second = find_nth(&doc.tree, "p", 1);
    assert_eq!(
        *map.get(second).unwrap().raw(lookup("margin-top").unwrap()),
        Value::px(4.0)
    );
    // The first paragraph is not an adjacent sibling of another paragraph.
    let first = find_nth(&doc.tree, "p", 0);
    assert_eq!(
        *map.get(first).unwrap().raw(lookup("margin-top").unwrap()),
        Value::px(0.0)
    );
}

#[test]
fn inline_style_attribute_enters_the_cascade() {
    let (doc, map) = resolve_document(DOCUMENT);
    let second = find_nth(&doc.tree, "p", 1);
    assert_eq!(
        *map.get(second).unwrap().raw(lookup("font-style").unwrap()),
        Value::Keyword("italic")
    );
}

#[test]
fn generated_content_and_counters_come_from_markup() {
    let (doc, map) = resolve_document(DOCUMENT);
    let first_p = find_nth(&doc.tree, "p", 0);
    assert_eq!(
        map.get_pseudo(first_p, &PseudoElement::Before)
            .expect("p::before is generated")
            .raw(lookup("content").unwrap()),
        &Value::Strings(vec!["> ".to_string()])
    );

    let first_li = find_nth(&doc.tree, "li", 0);
    let second_li = find_nth(&doc.tree, "li", 1);
    assert_eq!(
        map.get_pseudo(first_li, &PseudoElement::Before)
            .expect("li::before is generated")
            .raw(lookup("content").unwrap()),
        &Value::Strings(vec!["1".to_string(), ". ".to_string()])
    );
    assert_eq!(
        map.get_pseudo(second_li, &PseudoElement::Before)
            .expect("li::before is generated")
            .raw(lookup("content").unwrap()),
        &Value::Strings(vec!["2".to_string(), ". ".to_string()])
    );
}

#[test]
fn the_hover_rule_does_not_apply_without_hover_state() {
    let (doc, map) = resolve_document(DOCUMENT);
    let a = find(&doc.tree, "a");
    assert_eq!(map.get(a).unwrap().color(), Color::rgb8(0, 0, 255));
}

#[test]
fn markup_structure_has_the_expected_elements_and_text() {
    let doc = markup::read(DOCUMENT);
    let main = find(&doc.tree, "div");
    assert_eq!(doc.tree.attributes(main).id.as_deref(), Some("main"));
    // Void `br` is an element with no children, and the `<a>` text is kept.
    let br = find(&doc.tree, "br");
    assert!(doc.tree.children(br).is_empty());
    let a = find(&doc.tree, "a");
    let text = doc.tree.children(a)[0];
    assert_eq!(doc.tree.text(text), "link");
    assert_eq!(doc.style_sheets.len(), 1);
}

#[test]
fn restyle_after_a_markup_change_matches_a_full_resolve() {
    let (mut doc, _) = resolve_document(DOCUMENT);
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    for css in &doc.style_sheets {
        resolver.add_stylesheet(
            tpt_css::parse_stylesheet(css, Default::default()).stylesheet,
            Origin::Author,
        );
    }
    let env = Environment::default();
    let _ = resolver.resolve(&doc.tree, &env);
    let p = find_nth(&doc.tree, "p", 1);
    doc.tree.add_class(p, "note");
    resolver.invalidate(
        &doc.tree,
        tpt_css::StyleChange::AttributeNamed(p, "class".to_string()),
    );
    let restyled = resolver.restyle(&doc.tree, &env).clone();

    let mut fresh = StyleResolver::new(ResolverConfig::default());
    for css in &doc.style_sheets {
        fresh.add_stylesheet(
            tpt_css::parse_stylesheet(css, Default::default()).stylesheet,
            Origin::Author,
        );
    }
    assert_eq!(restyled, fresh.resolve(&doc.tree, &env).clone());
    assert_eq!(restyled.get(p).unwrap().color(), Color::rgb8(0, 128, 0));
}
