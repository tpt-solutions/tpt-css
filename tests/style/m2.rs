// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! M2 style tests: CSS nesting, `@supports`, `@property`, `all`, `light-dark()`,
//! `:visited`, the new state pseudo-classes, and stored at-rules.

use super::common::{parse_sheet, style_of, style_with, style_with_env, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::properties::lookup;
use tpt_css::selectors::PseudoElement;
use tpt_css::tree::{NodeId, NodeKind, StateBits};
use tpt_css::values::{Color, Value};
use tpt_css::{Environment, MediaEnvironment, ResolverConfig, StyleResolver, VisitedPolicy};

/// root > child, child has class `x`.
fn chain() -> (TestTree, NodeId, NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    t.add_class(child, "x");
    (t, root, child)
}

fn value(map: &tpt_css::StyleMap, node: NodeId, name: &str) -> Value {
    style_of(map, node).raw(lookup(name).unwrap()).clone()
}

// ---- nesting -------------------------------------------------------------------

#[test]
fn a_nested_rule_matches_as_a_descendant_of_its_parent() {
    let (t, _, child) = chain();
    let map = style_with("root { child { margin-left: 4px } }", &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(4.0));
}

#[test]
fn an_ampersand_refers_to_the_parent() {
    let (t, root, child) = chain();
    // `&.x` on the root would need the root to be .x; here `& child` matches child.
    let map = style_with("root { & child.x { margin-left: 5px } }", &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(5.0));
    let map = style_with("root { &.y { margin-left: 5px } }", &t);
    assert_eq!(value(&map, root, "margin-left"), Value::px(0.0));
}

#[test]
fn a_nested_rule_does_not_match_when_the_parent_does_not() {
    let (t, _, child) = chain();
    let map = style_with("other { child { margin-left: 4px } }", &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
}

#[test]
fn nested_media_applies_for_the_parent_selectors() {
    let (t, _, child) = chain();
    let map = style_with(
        "child { @media (min-width: 100px) { margin-left: 3px } }",
        &t,
    );
    assert_eq!(value(&map, child, "margin-left"), Value::px(3.0));
    let map = style_with("child { @media print { margin-left: 3px } }", &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
}

#[test]
fn ampersand_in_a_top_level_rule_is_rejected() {
    let parsed = tpt_css::parse_stylesheet("& child { margin-left: 1px }", Default::default());
    assert!(parsed.stylesheet.rules.is_empty());
}

// ---- @supports -----------------------------------------------------------------

#[test]
fn supports_uses_the_property_registry() {
    let (t, _, child) = chain();
    let map = style_with(
        "@supports (display: grid) { child { margin-left: 1px } }",
        &t,
    );
    assert_eq!(value(&map, child, "margin-left"), Value::px(1.0));
    let map = style_with(
        "@supports (not-a-property: 1) { child { margin-left: 1px } }",
        &t,
    );
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
    let map = style_with(
        "@supports (display: no-such-keyword) { child { margin-left: 1px } }",
        &t,
    );
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
}

#[test]
fn supports_combines_with_not_and_or() {
    let (t, _, child) = chain();
    let check = |cond: &str| {
        let css = format!("@supports {cond} {{ child {{ margin-left: 1px }} }}");
        value(&style_with(&css, &t), child, "margin-left") == Value::px(1.0)
    };
    assert!(check("(display: grid) and (width: 1px)"));
    assert!(!check("not (display: grid)"));
    assert!(check("(display: grid) or (foo: 1)"));
    assert!(!check("(foo: 1) and (display: grid)"));
    assert!(check("not (foo: 1)"));
}

#[test]
fn supports_selector_checks_the_selector_parser() {
    let (t, _, child) = chain();
    let map = style_with(
        "@supports selector(child > p) { child { margin-left: 1px } }",
        &t,
    );
    assert_eq!(value(&map, child, "margin-left"), Value::px(1.0));
    let map = style_with(
        "@supports selector(p::first-line) { child { margin-left: 1px } }",
        &t,
    );
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
}

// ---- @property -----------------------------------------------------------------

#[test]
fn an_unset_registered_property_takes_its_initial_value() {
    let (t, _, child) = chain();
    let map = style_with(
        "@property --gap { syntax: '<length>'; inherits: false; initial-value: 5px } ",
        &t,
    );
    let gap = map
        .custom_property(child, "--gap")
        .expect("registered initial value");
    assert_eq!(gap.text(), "5px");
}

#[test]
fn a_registered_property_that_does_not_inherit_uses_its_initial_value() {
    let (t, root, child) = chain();
    let css = "@property --gap { syntax: '<length>'; inherits: false; initial-value: 5px } root { --gap: 9px }";
    let map = style_with(css, &t);
    assert_eq!(map.custom_property(root, "--gap").unwrap().text(), "9px");
    assert_eq!(map.custom_property(child, "--gap").unwrap().text(), "5px");
}

#[test]
fn a_registered_property_that_inherits_passes_its_value_down() {
    let (t, root, child) = chain();
    let css = "@property --gap { syntax: '<length>'; inherits: true; initial-value: 5px } root { --gap: 9px }";
    let map = style_with(css, &t);
    assert_eq!(map.custom_property(root, "--gap").unwrap().text(), "9px");
    assert_eq!(map.custom_property(child, "--gap").unwrap().text(), "9px");
}

#[test]
fn a_value_that_does_not_match_the_syntax_falls_back_to_the_initial_value() {
    let (t, _, child) = chain();
    let css = "@property --gap { syntax: '<length>'; inherits: false; initial-value: 5px } child { --gap: red }";
    let map = style_with(css, &t);
    assert_eq!(map.custom_property(child, "--gap").unwrap().text(), "5px");
}

#[test]
fn a_registered_length_is_usable_with_var() {
    let (t, _, child) = chain();
    let css = "@property --gap { syntax: '<length>'; inherits: false; initial-value: 5px } child { margin-left: var(--gap) }";
    let map = style_with(css, &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(5.0));
}

#[test]
fn a_property_rule_without_inherits_is_dropped() {
    let parsed = tpt_css::parse_stylesheet("@property --x { syntax: '*'; }", Default::default());
    assert!(parsed.stylesheet.rules.is_empty());
    assert!(!parsed.diagnostics.is_empty());
}

// ---- all -----------------------------------------------------------------------

#[test]
fn all_resets_every_property_except_direction_and_unicode_bidi() {
    let (t, root, child) = chain();
    let css =
        "root { color: red; direction: rtl } child { margin-left: 5px; color: blue; all: initial }";
    let map = style_with(css, &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
    assert_eq!(value(&map, child, "direction"), Value::Keyword("rtl"));
    let _ = root;
}

#[test]
fn all_accepts_only_css_wide_keywords() {
    let (t, _, child) = chain();
    let map = style_with("child { margin-left: 5px; all: block }", &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(5.0));
}

// ---- light-dark() -------------------------------------------------------------

#[test]
fn light_dark_follows_the_color_scheme() {
    let (t, _, child) = chain();
    let css = "child { color: light-dark(red, blue) }";
    let light = style_with(css, &t);
    assert_eq!(style_of(&light, child).color(), Color::rgb8(255, 0, 0));
    let dark_env = Environment {
        media: MediaEnvironment {
            prefers_dark: true,
            ..MediaEnvironment::default()
        },
        ..Environment::default()
    };
    let dark = style_with_env(css, &t, &dark_env);
    assert_eq!(style_of(&dark, child).color(), Color::rgb8(0, 0, 255));
}

// ---- :visited and the state pseudo-classes -------------------------------------

#[test]
fn visited_applies_only_color_properties() {
    let (mut t, _, child) = chain();
    t.set_state(child, StateBits::VISITED);
    let map = style_with("child:visited { color: red; margin-left: 3px }", &t);
    assert_eq!(style_of(&map, child).color(), Color::rgb8(255, 0, 0));
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
}

#[test]
fn visited_does_not_match_without_the_state() {
    let (t, _, child) = chain();
    let map = style_with("child:visited { color: red }", &t);
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
}

#[test]
fn visited_disabled_never_matches() {
    let (mut t, _, child) = chain();
    t.set_state(child, StateBits::VISITED);
    let mut resolver = StyleResolver::new(ResolverConfig {
        visited: VisitedPolicy::Disabled,
        ..ResolverConfig::default()
    });
    resolver.add_stylesheet(
        parse_sheet("child:visited { color: red }").stylesheet,
        Origin::Author,
    );
    let map = resolver.resolve(&t, &Environment::default());
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
}

#[test]
fn modern_state_pseudo_classes_read_their_state_bits() {
    let cases = [
        (StateBits::LINK, ":link"),
        (StateBits::ANY_LINK, ":any-link"),
        (StateBits::FOCUS_VISIBLE, ":focus-visible"),
        (StateBits::FOCUS_WITHIN, ":focus-within"),
        (StateBits::TARGET, ":target"),
        (StateBits::OPEN, ":open"),
        (StateBits::MODAL, ":modal"),
        (StateBits::VALID, ":valid"),
        (StateBits::INDETERMINATE, ":indeterminate"),
    ];
    for (bits, pseudo) in cases {
        let (mut t, _, child) = chain();
        t.set_state(child, bits);
        let css = format!("child{pseudo} {{ margin-left: 1px }}");
        let map = style_with(&css, &t);
        assert_eq!(
            value(&map, child, "margin-left"),
            Value::px(1.0),
            "{pseudo} should match its state bit"
        );
    }
}

// ---- stored at-rules -----------------------------------------------------------

#[test]
fn keyframes_and_font_face_are_stored_on_the_resolver() {
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    let sheet = "@keyframes spin { from { margin-left: 0px } 50%, to { margin-left: 10px } } \
                 @font-face { font-family: X; src: url(x.woff) }";
    resolver.add_stylesheet(parse_sheet(sheet).stylesheet, Origin::Author);
    let spin = resolver.keyframes("spin").expect("stored");
    assert_eq!(spin.keyframes.len(), 2);
    assert_eq!(spin.keyframes[0].selectors, vec!["from".to_string()]);
    assert_eq!(
        spin.keyframes[1].selectors,
        vec!["50%".to_string(), "to".to_string()]
    );
    assert_eq!(resolver.font_faces().len(), 1);
}

#[test]
fn an_animation_origin_sheet_participates_in_the_cascade() {
    let (t, _, child) = chain();
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(
        parse_sheet("child { margin-left: 1px }").stylesheet,
        Origin::Author,
    );
    resolver.add_stylesheet(
        parse_sheet("child { margin-left: 8px }").stylesheet,
        Origin::Animation,
    );
    let map = resolver.resolve(&t, &Environment::default());
    assert_eq!(value(&map, child, "margin-left"), Value::px(8.0));
}

#[test]
fn pseudo_element_states_generated_without_content() {
    let (t, _, child) = chain();
    let map = style_with("child::selection { color: red }", &t);
    assert!(map.get_pseudo(child, &PseudoElement::Selection).is_some());
}
