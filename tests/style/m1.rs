// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! M1 style tests: custom properties and `var()`, math functions, units,
//! colors, pseudo-elements, shorthands, and the M1 property set.

use super::common::{style_of, style_with, style_with_env, TestTree};
use tpt_css::properties::lookup;
use tpt_css::selectors::PseudoElement;
use tpt_css::tree::{NodeId, NodeKind};
use tpt_css::values::calc::CalcNode;
use tpt_css::values::{Color, Value};
use tpt_css::{Environment, ViewportInfo};

/// root > child, with `root` named by its element name.
fn chain() -> (TestTree, NodeId, NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    (t, root, child)
}

fn prop(name: &str) -> tpt_css::properties::PropertyId {
    lookup(name).unwrap_or_else(|| panic!("unknown property {name}"))
}

fn value(map: &tpt_css::StyleMap, node: NodeId, name: &str) -> Value {
    style_of(map, node).raw(prop(name)).clone()
}

fn env_with_viewport(width: f64, height: f64) -> Environment {
    Environment {
        viewport: ViewportInfo { width, height },
        ..Environment::default()
    }
}

// ---- custom properties and var() ----------------------------------------------

#[test]
fn custom_properties_inherit_and_substitute_into_properties() {
    let (t, _, child) = chain();
    let map = style_with("root { --gap: 4px } child { margin-left: var(--gap) }", &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(4.0));
    let gap = map.custom_property(child, "--gap").expect("inherited");
    assert_eq!(gap.text(), "4px");
}

#[test]
fn var_uses_the_fallback_when_the_variable_is_undefined() {
    let (t, _, child) = chain();
    let map = style_with("child { margin-left: var(--missing, 7px) }", &t);
    assert_eq!(value(&map, child, "margin-left"), Value::px(7.0));
}

#[test]
fn an_undefined_variable_without_fallback_makes_the_declaration_unset() {
    let (t, _, child) = chain();
    let map = style_with(
        "child { margin-right: 3px; margin-left: var(--missing) }",
        &t,
    );
    assert_eq!(value(&map, child, "margin-right"), Value::px(3.0));
    // margin is not inherited, so unset is the initial value.
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
}

#[test]
fn a_cycle_makes_the_custom_properties_invalid_and_falls_back() {
    let (t, _, child) = chain();
    let map = style_with(
        "child { --a: var(--b); --b: var(--a); margin-left: var(--a, 5px) }",
        &t,
    );
    assert!(map.custom_property(child, "--a").is_none());
    assert!(map.custom_property(child, "--b").is_none());
    assert_eq!(value(&map, child, "margin-left"), Value::px(5.0));
}

#[test]
fn var_in_a_shorthand_expands_after_substitution() {
    let (t, _, child) = chain();
    let map = style_with("child { --m: 1px; margin: var(--m) 2px }", &t);
    assert_eq!(value(&map, child, "margin-top"), Value::px(1.0));
    assert_eq!(value(&map, child, "margin-right"), Value::px(2.0));
    assert_eq!(value(&map, child, "margin-bottom"), Value::px(1.0));
    assert_eq!(value(&map, child, "margin-left"), Value::px(2.0));
}

#[test]
fn a_custom_property_can_be_empty() {
    let (t, _, child) = chain();
    let map = style_with("child { --empty: ; margin-left: var(--empty, 9px) }", &t);
    assert_eq!(
        map.custom_property(child, "--empty").map(|v| v.text()),
        Some(String::new())
    );
    // An empty value is still a valid value, so the fallback is not used.
    assert_eq!(value(&map, child, "margin-left"), Value::px(0.0));
}

#[test]
fn custom_property_initial_keyword_removes_the_inherited_value() {
    let (t, _, child) = chain();
    let map = style_with("root { --c: red } child { --c: initial }", &t);
    assert!(map.custom_property(child, "--c").is_none());
}

// ---- math functions -------------------------------------------------------------

#[test]
fn calc_with_absolute_lengths_is_simplified_to_px() {
    let (t, _, child) = chain();
    let map = style_with("child { width: calc(10px + 20px) }", &t);
    assert_eq!(value(&map, child, "width"), Value::px(30.0));
}

#[test]
fn calc_mixing_percent_and_px_stays_symbolic_for_layout() {
    let (t, _, child) = chain();
    let map = style_with("child { width: calc(50% - 10px) }", &t);
    assert_eq!(
        value(&map, child, "width"),
        Value::Calc(Box::new(CalcNode::Linear {
            px: -10.0,
            percent: 50.0
        }))
    );
}

#[test]
fn calc_in_font_size_resolves_percentages_against_the_parent() {
    let (mut t, root, child) = chain();
    let _ = root;
    t.nodes[0].name.local = "root".into();
    let map = style_with(
        "root { font-size: 20px } child { font-size: calc(50% + 2px) }",
        &t,
    );
    assert_eq!(style_of(&map, child).font_size(), 12.0);
}

#[test]
fn min_max_and_clamp_fold_when_the_arguments_are_absolute() {
    let (t, _, child) = chain();
    let map = style_with(
        "child {
            --a: 1;
            width: min(10px, 20px);
            height: max(10px, 20px);
            margin-left: clamp(10px, 5px, 20px);
            margin-right: clamp(0px, 30px, 20px);
        }",
        &t,
    );
    assert_eq!(value(&map, child, "width"), Value::px(10.0));
    assert_eq!(value(&map, child, "height"), Value::px(20.0));
    assert_eq!(value(&map, child, "margin-left"), Value::px(10.0));
    assert_eq!(value(&map, child, "margin-right"), Value::px(20.0));
}

#[test]
fn a_math_expression_with_a_type_error_is_dropped() {
    let (t, _, child) = chain();
    let map = style_with("child { width: calc(10px + 2) }", &t);
    assert_eq!(value(&map, child, "width"), Value::Keyword("auto"));
}

#[test]
fn negative_math_results_are_clamped_for_non_negative_properties() {
    let (t, _, child) = chain();
    let map = style_with("child { padding-left: calc(-5px) }", &t);
    assert_eq!(value(&map, child, "padding-left"), Value::px(0.0));
}

// ---- units ----------------------------------------------------------------------

#[test]
fn viewport_units_use_the_environment_viewport() {
    let (t, _, child) = chain();
    let env = env_with_viewport(800.0, 600.0);
    let map = style_with_env("child { width: 10vw; height: 10vh }", &t, &env);
    assert_eq!(value(&map, child, "width"), Value::px(80.0));
    assert_eq!(value(&map, child, "height"), Value::px(60.0));
}

#[test]
fn absolute_physical_units_convert_to_px() {
    let (t, _, child) = chain();
    let map = style_with(
        "child { width: 1in; height: 2.54cm; margin-left: 72pt }",
        &t,
    );
    assert_eq!(value(&map, child, "width"), Value::px(96.0));
    let Value::Length { value: h, .. } = value(&map, child, "height") else {
        panic!("expected a length");
    };
    assert!((h - 96.0).abs() < 1e-9);
    assert_eq!(value(&map, child, "margin-left"), Value::px(96.0));
}

// ---- colors ---------------------------------------------------------------------

fn color_of(map: &tpt_css::StyleMap, node: NodeId) -> Color {
    style_of(map, node).color()
}

#[test]
fn hsl_and_hwb_colors() {
    let (t, _, child) = chain();
    let green = style_with("child { color: hsl(120 100% 50%) }", &t);
    assert_eq!(color_of(&green, child), Color::rgb8(0, 255, 0));
    let red = style_with("child { color: hwb(0 0% 0%) }", &t);
    assert_eq!(color_of(&red, child), Color::rgb8(255, 0, 0));
    let gray = style_with("child { color: hwb(0 50% 50%) }", &t);
    let c = color_of(&gray, child);
    assert!((c.components[0] - 0.5).abs() < 1e-6);
}

#[test]
fn hsla_and_short_hex_with_alpha() {
    let (t, _, child) = chain();
    let map = style_with("child { color: hsla(0, 100%, 50%, 0.5) }", &t);
    let c = color_of(&map, child);
    assert_eq!(c.components[0], 1.0);
    assert!((c.alpha - 0.5).abs() < 1e-6);
    let map = style_with("child { color: #f008 }", &t);
    let c = color_of(&map, child);
    assert_eq!(c.components[0], 1.0);
    assert!((c.alpha - 136.0 / 255.0).abs() < 1e-6);
}

// ---- pseudo-elements and content ------------------------------------------------

#[test]
fn before_is_generated_when_content_is_set_and_inherits_from_the_element() {
    let (t, _, child) = chain();
    let map = style_with(
        "child { color: blue } child::before { content: \"x\" } child::after { content: none }",
        &t,
    );
    let before = map
        .get_pseudo(child, &PseudoElement::Before)
        .expect("::before is generated");
    assert_eq!(
        before.color(),
        Color::rgb8(0, 0, 255),
        "inherits from the element"
    );
    assert_eq!(
        *before.raw(prop("content")),
        Value::Strings(vec!["x".to_string()])
    );
    assert!(map.get_pseudo(child, &PseudoElement::After).is_none());
}

#[test]
fn a_pseudo_element_without_content_is_not_generated() {
    let (t, _, child) = chain();
    let map = style_with("child::before { color: red }", &t);
    assert!(map.get_pseudo(child, &PseudoElement::Before).is_none());
}

#[test]
fn pseudo_element_declarations_do_not_leak_into_the_element() {
    let (t, _, child) = chain();
    let map = style_with("child::before { color: red; content: \"\" }", &t);
    assert_eq!(color_of(&map, child), Color::BLACK);
}

// ---- shorthands -----------------------------------------------------------------

#[test]
fn margin_expands_one_to_four_values() {
    let (t, _, child) = chain();
    let map = style_with("child { margin: 1px 2px 3px }", &t);
    assert_eq!(value(&map, child, "margin-top"), Value::px(1.0));
    assert_eq!(value(&map, child, "margin-right"), Value::px(2.0));
    assert_eq!(value(&map, child, "margin-bottom"), Value::px(3.0));
    assert_eq!(value(&map, child, "margin-left"), Value::px(2.0));
    let map = style_with("child { padding: 5px }", &t);
    assert_eq!(value(&map, child, "padding-bottom"), Value::px(5.0));
}

#[test]
fn margin_with_too_many_values_is_dropped() {
    let (t, _, child) = chain();
    let map = style_with("child { margin: 1px 2px 3px 4px 5px }", &t);
    assert_eq!(value(&map, child, "margin-top"), Value::px(0.0));
}

#[test]
fn border_shorthand_sets_every_longhand_and_resets_missing_parts() {
    let (t, _, child) = chain();
    let map = style_with("child { border: 2px dashed red }", &t);
    assert_eq!(value(&map, child, "border-left-width"), Value::px(2.0));
    assert_eq!(
        value(&map, child, "border-left-style"),
        Value::Keyword("dashed")
    );
    assert_eq!(
        value(&map, child, "border-left-color"),
        Value::Color(Color::rgb8(255, 0, 0))
    );
    let map = style_with(
        "child { border-left-color: red; border-left: 1px solid }",
        &t,
    );
    assert_eq!(
        value(&map, child, "border-left-color"),
        Value::Color(Color::BLACK),
        "a missing color resets to currentcolor, which is the element's color"
    );
}

#[test]
fn background_shorthand_covers_the_color_only() {
    let (t, _, child) = chain();
    let map = style_with("child { background: red }", &t);
    assert_eq!(
        value(&map, child, "background-color"),
        Value::Color(Color::rgb8(255, 0, 0))
    );
    let map = style_with("child { background: url(a.png) }", &t);
    assert_eq!(
        value(&map, child, "background-color"),
        Value::Color(Color::TRANSPARENT),
        "an unsupported background value is dropped"
    );
}

#[test]
fn css_wide_keyword_in_a_shorthand_applies_to_every_longhand() {
    let (t, root, child) = chain();
    let _ = root;
    let map = style_with("root { margin: 9px } child { margin: inherit }", &t);
    assert_eq!(value(&map, child, "margin-top"), Value::px(9.0));
    assert_eq!(value(&map, child, "margin-left"), Value::px(9.0));
}

// ---- the M1 property set ---------------------------------------------------------

#[test]
fn opacity_is_clamped_and_accepts_percentages() {
    let (t, _, child) = chain();
    let map = style_with("child { opacity: 2 }", &t);
    assert_eq!(value(&map, child, "opacity"), Value::Number(1.0));
    let map = style_with("child { opacity: 50% }", &t);
    assert_eq!(value(&map, child, "opacity"), Value::Number(0.5));
}

#[test]
fn z_index_takes_integers_and_rejects_fractions() {
    let (t, _, child) = chain();
    let map = style_with("child { z-index: 3 }", &t);
    assert_eq!(value(&map, child, "z-index"), Value::Number(3.0));
    let map = style_with("child { z-index: 2.5 }", &t);
    assert_eq!(value(&map, child, "z-index"), Value::Keyword("auto"));
}

#[test]
fn position_and_offsets() {
    let (t, _, child) = chain();
    let map = style_with("child { position: absolute; top: 4px; left: auto }", &t);
    assert_eq!(value(&map, child, "position"), Value::Keyword("absolute"));
    assert_eq!(value(&map, child, "top"), Value::px(4.0));
    assert_eq!(value(&map, child, "left"), Value::Keyword("auto"));
}

#[test]
fn direction_and_white_space_are_inherited() {
    let (t, _, child) = chain();
    let map = style_with("root { direction: rtl; white-space: nowrap }", &t);
    assert_eq!(value(&map, child, "direction"), Value::Keyword("rtl"));
    assert_eq!(value(&map, child, "white-space"), Value::Keyword("nowrap"));
}

#[test]
fn unicode_bidi_is_not_inherited() {
    let (t, _, child) = chain();
    let map = style_with("root { unicode-bidi: embed }", &t);
    assert_eq!(value(&map, child, "unicode-bidi"), Value::Keyword("normal"));
}

#[test]
fn content_attr_reads_the_element_attribute_at_computed_time() {
    let (mut t, _, child) = chain();
    t.set_attr(child, "title", "hi");
    let map = style_with(
        "child::before { content: \"[\" attr(TITLE) \"]\" } child::after { content: attr(missing) }",
        &t,
    );
    let before = map
        .get_pseudo(child, &PseudoElement::Before)
        .expect("::before is generated");
    assert_eq!(
        *before.raw(prop("content")),
        Value::Strings(vec!["[".to_string(), "hi".to_string(), "]".to_string()])
    );
    let after = map
        .get_pseudo(child, &PseudoElement::After)
        .expect("a missing attribute still generates the box");
    assert_eq!(
        *after.raw(prop("content")),
        Value::Strings(vec![String::new()])
    );
}

#[test]
fn content_attr_with_an_unsupported_type_is_invalid() {
    // `string` and a fallback are supported (see `attr_accepts_a_string_type_and_a_fallback`).
    let (t, _, child) = chain();
    let map = style_with("child::before { content: attr(title px) }", &t);
    assert!(map.get_pseudo(child, &PseudoElement::Before).is_none());
}

#[test]
fn a_missing_attribute_yields_its_fallback_and_a_present_one_wins() {
    let (mut t, _, child) = chain();
    let content = |t: &TestTree, css: &str| {
        let map = style_with(css, t);
        map.get_pseudo(child, &PseudoElement::Before)
            .expect("generated")
            .raw(lookup("content").unwrap())
            .clone()
    };
    let css = "child::before { content: attr(title, \"none\") }";
    assert_eq!(
        content(&t, css),
        tpt_css::values::Value::Strings(vec!["none".to_string()])
    );
    t.set_attr(child, "title", "hi");
    assert_eq!(
        content(&t, css),
        tpt_css::values::Value::Strings(vec!["hi".to_string()])
    );
}
