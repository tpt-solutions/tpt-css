// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Style sharing (DESIGN.md decision 35) must never change a result. Each case
//! resolves the same tree with sharing on and off and compares the maps.

use super::common::{parse_sheet, TestTree};
use tpt_css::tree::NodeKind;
use tpt_css::{Environment, Origin, ResolverConfig, StyleMap, StyleResolver};

fn resolve(css: &str, tree: &TestTree, share: bool) -> StyleMap {
    let mut r = StyleResolver::new(ResolverConfig {
        share_styles: share,
        ..ResolverConfig::default()
    });
    r.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
    r.resolve(tree, &Environment::default()).clone()
}

/// A list with `n` items. Every third item has a class, every fifth has an
/// attribute, and the list is nested inside a wrapper.
fn list(n: usize) -> TestTree {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let wrapper = t.element(root, "div");
    let ul = t.element(wrapper, "ul");
    for i in 0..n {
        let li = t.element(ul, "li");
        if i % 3 == 0 {
            t.add_class(li, "a");
        }
        if i % 5 == 0 {
            t.set_attr(li, "data-x", &i.to_string());
        }
        t.add_text_with(li, "x");
    }
    t
}

const SHEETS: &[&str] = &[
    "li { color: red } li.a { color: blue }",
    "li:nth-child(4n) { margin-left: 2px } li[data-x] { padding-top: 1px }",
    "li { margin-top: 1em } div { font-size: 20px } li.a + li { color: green }",
    // attr() in content makes the element's style depend on its attributes.
    "li { color: red } li[data-x]::before { content: attr(data-x) }",
];

#[test]
fn sharing_gives_the_same_styles_as_no_sharing() {
    for css in SHEETS {
        let t = list(60);
        assert_eq!(resolve(css, &t, true), resolve(css, &t, false), "{css}");
    }
}

#[test]
fn sharing_is_not_used_when_counters_or_slots_are_involved() {
    // A counter read depends on earlier siblings, so every item must still count.
    let css =
        "ol { counter-reset: n } li { counter-increment: n } li::before { content: counter(n) }";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let ol = t.element(root, "ol");
    let items: Vec<_> = (0..5).map(|_| t.element(ol, "li")).collect();
    let map = resolve(css, &t, true);
    for (i, item) in items.iter().enumerate() {
        let before = map
            .get_pseudo(*item, &tpt_css::selectors::PseudoElement::Before)
            .unwrap();
        assert_eq!(
            *before.raw(tpt_css::properties::lookup("content").unwrap()),
            tpt_css::values::Value::Strings(vec![(i + 1).to_string()])
        );
    }
    assert_eq!(map, resolve(css, &t, false));
}

/// Timing, not a correctness check. Run with
/// `cargo test --release --test integration sharing_speed -- --ignored --nocapture`.
#[test]
#[ignore]
fn sharing_speed() {
    let css = "li { color: red; margin-left: 2px } ul { font-size: 14px }";
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    let ul = t.element(root, "ul");
    for _ in 0..10_000 {
        t.element(ul, "li");
    }
    for share in [false, true] {
        let mut r = StyleResolver::new(ResolverConfig {
            share_styles: share,
            ..ResolverConfig::default()
        });
        r.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
        let start = std::time::Instant::now();
        let _ = r.resolve(&t, &Environment::default());
        println!("share={share}: {:?}", start.elapsed());
    }
}
