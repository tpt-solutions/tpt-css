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
    resolver_for(css).resolve(tree, env).clone()
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
    let map = r.restyle(&t, &env).clone();

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
    assert_eq!(*map, full(css, &t, &env));
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
    assert_eq!(*map, full(css, &t, &env));
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
    let map = r.restyle(&t, &env).clone();

    assert!(r.last_restyle_count() > 3, "siblings must be restyled");
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn an_environment_change_restyles_what_its_media_queries_reach() {
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
    assert_eq!(*second, full(css, &t, &narrow));
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
    assert_eq!(*map, full(css, &t, &env));
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
    let first = r.resolve(&t, &env).clone();
    let again = r.restyle(&t, &env);
    assert_eq!(first, *again);
    assert_eq!(r.last_restyle_count(), 0);
}

#[test]
fn an_unchanged_style_stops_the_restyle_when_rules_are_subject_only() {
    let css = "div { margin-left: 1px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    // The class is not used by any rule, so the div's style is unchanged and its
    // descendants need no restyle.
    t.add_class(divs[4], "unrelated");
    r.invalidate(&t, StyleChange::AttributeChanged(divs[4]));
    let map = r.restyle(&t, &env).clone();
    assert_eq!(
        r.last_restyle_count(),
        1,
        "only the changed element is restyled"
    );
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn a_changed_style_propagates_to_the_subtree() {
    let css = "div { margin-left: 1px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    t.set_inline(divs[4], "margin-left: 5px");
    r.invalidate(&t, StyleChange::InlineStyleChanged(divs[4]));
    let map = r.restyle(&t, &env).clone();
    // The div changed, so its span is restyled. The span is unchanged (margins are
    // not inherited), so the em below it is skipped by the equality cutoff.
    assert_eq!(r.last_restyle_count(), 2, "the div and its span");
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn a_has_rule_forces_a_full_restyle_on_attribute_changes() {
    let css = "div:has(em) { margin-left: 1px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);
    t.add_class(divs[0], "unrelated");
    r.invalidate(&t, StyleChange::AttributeChanged(divs[0]));
    let map = r.restyle(&t, &env);
    assert_eq!(*map, full(css, &t, &env));
}

#[test]
fn an_attribute_that_no_selector_reads_restyles_nothing() {
    let css = ".hot { color: red } [data-live] { margin-left: 2px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    t.set_attr(divs[2], "data-other", "1");
    r.invalidate(
        &t,
        StyleChange::AttributeNamed(divs[2], "data-other".to_string()),
    );
    let map = r.restyle(&t, &env).clone();
    assert_eq!(r.last_restyle_count(), 0, "no selector reads data-other");
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn an_attribute_that_a_selector_reads_restyles_the_element() {
    let css = "[data-live] { margin-left: 2px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);

    // Attribute names are case-insensitive, so the change may use another case.
    t.set_attr(divs[4], "DATA-LIVE", "yes");
    r.invalidate(
        &t,
        StyleChange::AttributeNamed(divs[4], "DATA-LIVE".to_string()),
    );
    let map = r.restyle(&t, &env).clone();
    assert_eq!(r.last_restyle_count(), 1);
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn attributes_read_inside_negations_and_scope_roots_count() {
    let env = Environment::default();
    for (css, attr) in [
        ("div:not([hidden]) { margin-left: 2px }", "hidden"),
        (
            "@scope ([data-root]) { span { margin-left: 2px } }",
            "data-root",
        ),
        ("div:has([data-x]) { margin-left: 2px }", "data-x"),
    ] {
        let (mut t, divs) = big_tree();
        let mut r = resolver_for(css);
        let _ = r.resolve(&t, &env);
        t.set_attr(divs[1], attr, "1");
        r.invalidate(&t, StyleChange::AttributeNamed(divs[1], attr.to_string()));
        let map = r.restyle(&t, &env).clone();
        assert!(r.last_restyle_count() > 0, "{attr} is read by {css}");
        assert_eq!(map, full(css, &t, &env), "{css}");
    }
}

#[test]
fn style_and_lang_changes_are_always_relevant() {
    let css = "div { margin-left: 1px }";
    let env = Environment::default();
    let (mut t, divs) = big_tree();
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);
    t.set_attr(divs[0], "style", "margin-left: 9px");
    r.invalidate(
        &t,
        StyleChange::AttributeNamed(divs[0], "style".to_string()),
    );
    let _ = r.restyle(&t, &env);
    assert!(r.last_restyle_count() > 0);
}

#[test]
fn a_custom_property_change_reaches_descendants_that_read_it() {
    let css = ".theme { --accent: #00ff00 } p { color: var(--accent) }";
    let env = Environment::default();
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "html".into();
    let div = t.element(root, "div");
    let p = t.element(div, "p");
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);
    t.add_class(div, "theme");
    r.invalidate(&t, StyleChange::AttributeChanged(div));
    let map = r.restyle(&t, &env).clone();
    assert_eq!(map.get(p).unwrap().color(), Color::rgb8(0, 255, 0));
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn an_attribute_read_by_content_restyles_the_pseudo_element() {
    let css = "p::before { content: attr(data-label) }";
    let env = Environment::default();
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "html".into();
    let p = t.element(root, "p");
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);
    t.set_attr(p, "data-label", "new");
    r.invalidate(&t, StyleChange::AttributeNamed(p, "data-label".to_string()));
    let map = r.restyle(&t, &env).clone();
    let before = map
        .get_pseudo(p, &tpt_css::selectors::PseudoElement::Before)
        .expect("::before is generated");
    assert_eq!(
        *before.raw(tpt_css::properties::lookup("content").unwrap()),
        tpt_css::values::Value::Strings(vec!["new".to_string()])
    );
    assert_eq!(map, full(css, &t, &env));
}

/// `:nth-child(of S)` results come from a per-parent index that is cached for a
/// pass. Positions must be right, and a class change must restyle the same as a
/// full resolve, which checks that the cache is rebuilt when the tree changes.
#[test]
fn nth_child_of_s_uses_positions_among_matching_siblings() {
    let css = "li:nth-child(1 of .x) { margin-left: 1px } li:nth-last-child(1 of .x) { margin-right: 2px }";
    let env = Environment::default();
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "ul".into();
    let items: Vec<NodeId> = (0..5).map(|_| t.element(root, "li")).collect();
    for &i in &[1usize, 3, 4] {
        t.add_class(items[i], "x");
    }
    let mut r = resolver_for(css);
    let first_margin = |m: &tpt_css::StyleMap, n: NodeId| {
        m.get(n)
            .unwrap()
            .raw(tpt_css::properties::lookup("margin-left").unwrap())
            .clone()
    };
    let last_margin = |m: &tpt_css::StyleMap, n: NodeId| {
        m.get(n)
            .unwrap()
            .raw(tpt_css::properties::lookup("margin-right").unwrap())
            .clone()
    };
    let map = r.resolve(&t, &env).clone();
    // Matching siblings are items 1, 3, and 4. The first of them is item 1, and
    // the last is item 4.
    assert_eq!(
        first_margin(&map, items[1]),
        tpt_css::values::Value::px(1.0)
    );
    assert_eq!(
        first_margin(&map, items[3]),
        tpt_css::values::Value::px(0.0)
    );
    assert_eq!(last_margin(&map, items[4]), tpt_css::values::Value::px(2.0));
    assert_eq!(last_margin(&map, items[1]), tpt_css::values::Value::px(0.0));

    // Adding the class to item 0 moves the first match to item 0.
    t.add_class(items[0], "x");
    r.invalidate(&t, StyleChange::AttributeChanged(items[0]));
    let map = r.restyle(&t, &env).clone();
    assert_eq!(
        first_margin(&map, items[0]),
        tpt_css::values::Value::px(1.0)
    );
    assert_eq!(
        first_margin(&map, items[1]),
        tpt_css::values::Value::px(0.0)
    );
    assert_eq!(map, full(css, &t, &env));
}

#[test]
fn a_media_flip_restyles_only_the_elements_its_rule_matches() {
    // Only `.hot` and its three-element subtree (div, span, em) are reached.
    let css = "@media (min-width: 500px) { .hot { margin-left: 3px } }";
    let (mut t, divs) = big_tree();
    t.add_class(divs[4], "hot");
    let wide = Environment::default();
    let narrow = Environment {
        viewport: tpt_css::ViewportInfo {
            width: 300.0,
            height: 600.0,
        },
        ..Environment::default()
    };
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &wide);
    let map = r.restyle(&t, &narrow).clone();
    // The subtree is recomputed, but the equality cutoff stops below the first
    // element whose style is unchanged, so far fewer than the 151 elements.
    assert!(r.last_restyle_count() <= 3, "{}", r.last_restyle_count());
    assert_eq!(map, full(css, &t, &narrow));
}

#[test]
fn a_viewport_change_restyles_nothing_when_no_rule_reads_the_viewport() {
    let css = ".hot { color: red } div { margin-left: 1px }";
    let (t, _) = big_tree();
    let wide = Environment::default();
    let narrow = Environment {
        viewport: tpt_css::ViewportInfo {
            width: 300.0,
            height: 600.0,
        },
        ..Environment::default()
    };
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &wide);
    let map = r.restyle(&t, &narrow).clone();
    assert_eq!(r.last_restyle_count(), 0);
    assert_eq!(map, full(css, &t, &narrow));
}

#[test]
fn a_viewport_change_restyles_the_rules_that_use_viewport_units() {
    let css = "div { margin-left: 10vw } span { color: red }";
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
    let _ = r.resolve(&t, &wide);
    let margin = lookup("margin-left").unwrap();
    let map = r.restyle(&t, &narrow).clone();
    assert_eq!(map.get(divs[0]).unwrap().raw(margin), &Value::px(30.0));
    assert_eq!(map, full(css, &t, &narrow));
}

#[test]
fn a_color_scheme_change_restyles_the_rules_that_use_light_dark() {
    let css = "span { color: light-dark(red, blue) } div { margin-left: 1px }";
    let (t, _) = big_tree();
    let light = Environment::default();
    let mut dark = Environment::default();
    dark.media.prefers_dark = true;
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &light);
    let map = r.restyle(&t, &dark).clone();
    assert_eq!(map, full(css, &t, &dark));
    // The spans and the ems below them change; the divs do not.
    assert_eq!(r.last_restyle_count(), 100);
}

#[test]
fn a_root_font_size_change_restyles_the_rules_that_use_relative_lengths() {
    // The root element's font size is `2em` of the environment's root font size,
    // which is 40px. `rem` is that computed size of the root element, so `1rem`
    // is 40px too.
    let css = "root { font-size: 2em } div { margin-left: 1rem }";
    let (t, divs) = big_tree();
    let base = Environment::default();
    let big = Environment {
        root_font_size: 20.0,
        ..Environment::default()
    };
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &base);
    let map = r.restyle(&t, &big).clone();
    assert_eq!(map, full(css, &t, &big));
    let margin = lookup("margin-left").unwrap();
    assert_eq!(map.get(divs[0]).unwrap().raw(margin), &Value::px(40.0));
    assert_eq!(map.get(t.root()).unwrap().font_size(), 40.0);
}

#[test]
fn a_typed_attr_in_another_property_restyles_when_the_attribute_changes() {
    let css = "p { width: attr(data-w px, 5px) }";
    let env = Environment::default();
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "html".into();
    let p = t.element(root, "p");
    let mut r = resolver_for(css);
    let _ = r.resolve(&t, &env);
    t.set_attr(p, "data-w", "40");
    r.invalidate(&t, StyleChange::AttributeNamed(p, "data-w".to_string()));
    let map = r.restyle(&t, &env).clone();
    assert_eq!(
        *map.get(p)
            .unwrap()
            .raw(tpt_css::properties::lookup("width").unwrap()),
        tpt_css::values::Value::px(40.0)
    );
    assert_eq!(map, full(css, &t, &env));
}
