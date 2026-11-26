// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cascade tests: origin and importance (spec 13.2), inline style (13.5),
//! specificity and source order (13.3).

#[path = "../common/mod.rs"]
mod common;
mod layers;
mod media;

use common::{parse_sheet, TestTree};
use tpt_css::cascade::order::DeclarationOrder;
use tpt_css::cascade::origin::Origin;
use tpt_css::cascade::CascadeKey;
use tpt_css::properties::ids;
use tpt_css::selectors::Specificity;
use tpt_css::tree::NodeKind;
use tpt_css::values::{Color, Value};
use tpt_css::{ComputedStyle, Environment, ResolverConfig, StyleMap, StyleResolver};

fn key(
    origin: Origin,
    important: bool,
    inline: bool,
    spec: Specificity,
    order: DeclarationOrder,
) -> CascadeKey {
    CascadeKey {
        origin,
        important,
        layer: tpt_css::cascade::UNLAYERED_NORMAL,
        inline,
        specificity: spec,
        order,
    }
}

fn order(sheet: u32, rule: u32, declaration: u32) -> DeclarationOrder {
    DeclarationOrder {
        sheet,
        rule,
        declaration,
    }
}

const ZERO: Specificity = Specificity { a: 0, b: 0, c: 0 };

#[test]
fn origin_and_importance_follow_spec_13_2() {
    // normal UA < normal user < normal author < animations
    //   < important author < important user < important UA < transitions
    let chain = [
        key(Origin::UserAgent, false, false, ZERO, order(0, 0, 0)),
        key(Origin::User, false, false, ZERO, order(0, 0, 0)),
        key(Origin::Author, false, false, ZERO, order(0, 0, 0)),
        key(Origin::Animation, false, false, ZERO, order(0, 0, 0)),
        key(Origin::Author, true, false, ZERO, order(0, 0, 0)),
        key(Origin::User, true, false, ZERO, order(0, 0, 0)),
        key(Origin::UserAgent, true, false, ZERO, order(0, 0, 0)),
        key(Origin::Transition, false, false, ZERO, order(0, 0, 0)),
    ];
    for pair in chain.windows(2) {
        assert!(
            pair[0] < pair[1],
            "{:?} should be below {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn specificity_beats_source_order_and_order_breaks_ties() {
    let high = key(
        Origin::Author,
        false,
        false,
        Specificity { a: 1, b: 0, c: 0 },
        order(0, 0, 0),
    );
    let low_later = key(
        Origin::Author,
        false,
        false,
        Specificity { a: 0, b: 1, c: 0 },
        order(0, 9, 0),
    );
    assert!(high > low_later);
    let early = key(Origin::Author, false, false, ZERO, order(0, 1, 0));
    let late = key(Origin::Author, false, false, ZERO, order(0, 2, 0));
    assert!(late > early);
}

#[test]
fn inline_beats_any_selector_at_the_same_importance() {
    let selector = key(
        Origin::Author,
        false,
        false,
        Specificity { a: 9, b: 9, c: 9 },
        order(0, 0, 0),
    );
    let inline = key(
        Origin::Author,
        false,
        true,
        ZERO,
        DeclarationOrder::inline(0),
    );
    assert!(inline > selector);
}

#[test]
fn important_author_selector_beats_normal_inline() {
    let important = key(Origin::Author, true, false, ZERO, order(0, 0, 0));
    let inline = key(
        Origin::Author,
        false,
        true,
        ZERO,
        DeclarationOrder::inline(0),
    );
    assert!(important > inline);
}

#[test]
fn ua_important_beats_author_important() {
    let ua = key(Origin::UserAgent, true, false, ZERO, order(0, 0, 0));
    let author = key(
        Origin::Author,
        true,
        false,
        Specificity { a: 9, b: 0, c: 0 },
        order(9, 9, 9),
    );
    assert!(ua > author);
}

fn resolve_two_sheets(ua_css: &str, author_css: &str, tree: &TestTree) -> StyleMap {
    let mut r = StyleResolver::new(ResolverConfig::default());
    r.add_stylesheet(parse_sheet(ua_css).stylesheet, Origin::UserAgent);
    r.add_stylesheet(parse_sheet(author_css).stylesheet, Origin::Author);
    r.resolve(tree, &Environment::default())
}

fn color_of(style: &ComputedStyle) -> Color {
    style.color()
}

#[test]
fn author_normal_overrides_user_agent_normal() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    let map = resolve_two_sheets("p { color: red }", "p { color: blue }", &t);
    assert_eq!(color_of(map.get(p).unwrap()), Color::rgb8(0, 0, 255));
}

#[test]
fn user_agent_important_beats_author_normal() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    let map = resolve_two_sheets("p { color: red !important }", "p { color: blue }", &t);
    assert_eq!(color_of(map.get(p).unwrap()), Color::rgb8(255, 0, 0));
}

#[test]
fn higher_specificity_wins_regardless_of_order() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    t.add_class(p, "x");
    let map = common::style_with("p.x { color: red } p { color: blue }", &t);
    assert_eq!(color_of(map.get(p).unwrap()), Color::rgb8(255, 0, 0));
}

#[test]
fn later_declaration_wins_within_one_block() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    let map = common::style_with("p { color: red; color: green }", &t);
    assert_eq!(color_of(map.get(p).unwrap()), Color::rgb8(0, 128, 0));
}

#[test]
fn later_important_declaration_still_wins_over_normal_in_same_block() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    let map = common::style_with("p { color: red !important; color: green }", &t);
    assert_eq!(color_of(map.get(p).unwrap()), Color::rgb8(255, 0, 0));
}

#[test]
fn inline_style_beats_a_more_specific_selector() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    t.add_class(p, "x").add_class(p, "y");
    t.set_inline(p, "color: lime");
    let map = common::style_with("p.x.y { color: red }", &t);
    assert_eq!(color_of(map.get(p).unwrap()), Color::rgb8(0, 255, 0));
}

#[test]
fn inline_important_is_not_beaten_by_author_important_selector() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    t.set_inline(p, "color: lime !important");
    let map = common::style_with("p { color: red !important }", &t);
    assert_eq!(color_of(map.get(p).unwrap()), Color::rgb8(0, 255, 0));
}

#[test]
fn removing_a_sheet_removes_its_declarations() {
    let mut t = TestTree::new(NodeKind::Element);
    let p = t.element(t.root(), "p");
    let mut r = StyleResolver::new(ResolverConfig::default());
    let id = r.add_stylesheet(parse_sheet("p { color: red }").stylesheet, Origin::Author);
    let with = r.resolve(&t, &Environment::default());
    r.remove_stylesheet(id);
    let without = r.resolve(&t, &Environment::default());
    assert_eq!(color_of(with.get(p).unwrap()), Color::rgb8(255, 0, 0));
    assert_eq!(color_of(without.get(p).unwrap()), Color::BLACK);
    assert_eq!(
        *without.get(p).unwrap().raw(ids::COLOR),
        Value::Color(Color::BLACK)
    );
}
