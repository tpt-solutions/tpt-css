// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Style tests: inheritance, initial values, CSS-wide keywords, relative
//! units, values and colors, and property grammars (spec 13.8, 14, 15, 16.1).

#[path = "../common/mod.rs"]
mod common;
mod m1;

use common::{style_of, style_with, TestTree};
use tpt_css::properties::ids;
use tpt_css::properties::parse_property_value;
use tpt_css::properties::registry::{lookup, spec, PROPERTIES};
use tpt_css::syntax::component::build_component_values;
use tpt_css::syntax::{tokenize, SourceId};
use tpt_css::tree::NodeKind;
use tpt_css::values::color::parse_color;
use tpt_css::values::serialize::{serialize_color, serialize_value};
use tpt_css::values::{Color, LengthUnit, Value};
use tpt_css::ParseLimits;

/// root > parent > child
fn chain() -> (
    TestTree,
    tpt_css::tree::NodeId,
    tpt_css::tree::NodeId,
    tpt_css::tree::NodeId,
) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let parent = t.element(root, "parent");
    let child = t.element(parent, "child");
    (t, root, parent, child)
}

#[test]
fn initial_values_apply_when_nothing_matches() {
    let (t, _, _, child) = chain();
    let map = style_with("", &t);
    let s = style_of(&map, child);
    assert_eq!(s.display(), "inline");
    assert_eq!(s.font_size(), 16.0);
    assert_eq!(s.font_weight(), 400.0);
    assert_eq!(
        *s.raw(ids::FONT_FAMILY),
        Value::FontFamily(vec!["serif".into()])
    );
    assert_eq!(*s.raw(lookup("width").unwrap()), Value::Keyword("auto"));
    assert_eq!(*s.raw(lookup("margin-top").unwrap()), Value::px(0.0));
}

#[test]
fn inherited_properties_flow_down_and_others_do_not() {
    let (t, _, parent, child) = chain();
    let map = style_with("parent { color: red; margin-top: 5px }", &t);
    let c = style_of(&map, child);
    assert_eq!(c.color(), Color::rgb8(255, 0, 0), "color is inherited");
    assert_eq!(
        *c.raw(lookup("margin-top").unwrap()),
        Value::px(0.0),
        "margin is not inherited"
    );
    assert_eq!(style_of(&map, parent).color(), Color::rgb8(255, 0, 0));
}

#[test]
fn explicit_inherit_takes_the_parent_value_even_for_non_inherited_properties() {
    let (t, _, _, child) = chain();
    let map = style_with(
        "parent { margin-top: 5px } child { margin-top: inherit }",
        &t,
    );
    assert_eq!(
        *style_of(&map, child).raw(lookup("margin-top").unwrap()),
        Value::px(5.0)
    );
}

#[test]
fn initial_keyword_resets_an_inherited_property() {
    let (t, _, _, child) = chain();
    let map = style_with("parent { color: red } child { color: initial }", &t);
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
}

#[test]
fn unset_inherits_only_when_the_property_inherits() {
    let (t, _, _, child) = chain();
    let map = style_with(
        "parent { color: red; margin-top: 5px } child { color: unset; margin-top: unset }",
        &t,
    );
    let c = style_of(&map, child);
    assert_eq!(c.color(), Color::rgb8(255, 0, 0));
    assert_eq!(*c.raw(lookup("margin-top").unwrap()), Value::px(0.0));
}

#[test]
fn em_in_font_size_uses_the_parent_font_size() {
    let (t, _, parent, child) = chain();
    let map = style_with("parent { font-size: 20px } child { font-size: 2em }", &t);
    assert_eq!(style_of(&map, parent).font_size(), 20.0);
    assert_eq!(style_of(&map, child).font_size(), 40.0);
}

#[test]
fn percentage_font_size_is_relative_to_the_parent() {
    let (t, _, _, child) = chain();
    let map = style_with("parent { font-size: 20px } child { font-size: 50% }", &t);
    assert_eq!(style_of(&map, child).font_size(), 10.0);
}

#[test]
fn em_in_other_properties_uses_the_element_font_size() {
    let (t, _, _, child) = chain();
    let map = style_with("child { font-size: 10px; margin-left: 2em }", &t);
    assert_eq!(
        *style_of(&map, child).raw(lookup("margin-left").unwrap()),
        Value::px(20.0)
    );
}

#[test]
fn rem_uses_the_root_font_size_not_the_parent() {
    let (t, _, _, child) = chain();
    let map = style_with(
        "root { font-size: 10px } parent { font-size: 40px } child { margin-left: 2rem }",
        &t,
    );
    assert_eq!(
        *style_of(&map, child).raw(lookup("margin-left").unwrap()),
        Value::px(20.0)
    );
}

#[test]
fn percentages_are_kept_for_layout_except_font_size() {
    let (t, _, _, child) = chain();
    let map = style_with("child { width: 50% }", &t);
    assert_eq!(
        *style_of(&map, child).raw(lookup("width").unwrap()),
        Value::Percentage(50.0)
    );
}

#[test]
fn currentcolor_in_border_color_uses_the_element_color() {
    let (t, _, _, child) = chain();
    let map = style_with(
        "child { color: green; border-top-style: solid; border-top-width: 2px }",
        &t,
    );
    let c = style_of(&map, child);
    assert_eq!(
        *c.raw(lookup("border-top-color").unwrap()),
        Value::Color(Color::rgb8(0, 128, 0))
    );
}

#[test]
fn border_width_computes_to_zero_when_style_is_none() {
    let (t, _, _, child) = chain();
    let map = style_with("child { border-top-width: 4px }", &t);
    assert_eq!(
        *style_of(&map, child).raw(lookup("border-top-width").unwrap()),
        Value::px(0.0)
    );
    let map = style_with(
        "child { border-top-width: 4px; border-top-style: dashed }",
        &t,
    );
    assert_eq!(
        *style_of(&map, child).raw(lookup("border-top-width").unwrap()),
        Value::px(4.0)
    );
}

#[test]
fn border_width_keywords_map_to_px() {
    let (t, _, _, child) = chain();
    let map = style_with(
        "child { border-top-width: thick; border-top-style: solid }",
        &t,
    );
    assert_eq!(
        *style_of(&map, child).raw(lookup("border-top-width").unwrap()),
        Value::px(5.0)
    );
}

#[test]
fn font_weight_keywords_compute_to_numbers_and_relative_keywords_use_the_parent() {
    let (t, _, _, child) = chain();
    let map = style_with(
        "parent { font-weight: bold } child { font-weight: bolder }",
        &t,
    );
    assert_eq!(style_of(&map, child).font_weight(), 900.0);
    let map = style_with(
        "parent { font-weight: 300 } child { font-weight: lighter }",
        &t,
    );
    assert_eq!(style_of(&map, child).font_weight(), 100.0);
}

#[test]
fn invalid_values_are_dropped_and_the_cascade_falls_back() {
    let (t, _, _, child) = chain();
    let map = style_with("child { display: sideways; width: -5px }", &t);
    let c = style_of(&map, child);
    assert_eq!(c.display(), "inline");
    assert_eq!(*c.raw(lookup("width").unwrap()), Value::Keyword("auto"));
}

#[test]
fn text_nodes_and_document_roots_do_not_get_styles() {
    let mut t = TestTree::new(NodeKind::Document);
    let html = t.element(t.root(), "html");
    let text = t.add_text(html);
    let map = style_with("", &t);
    assert!(map.get(text).is_none());
    assert!(map.get(html).is_some());
}

#[test]
fn styling_the_same_tree_twice_gives_identical_results() {
    let (t, _, _, _) = chain();
    let css = "parent { color: red; font-size: 1.5em } child { margin: 0 }";
    let a = style_with(css, &t);
    let b = style_with(css, &t);
    assert_eq!(a, b);
}

// ---- property grammars --------------------------------------------------

fn parse_value(prop: &str, css_value: &str) -> Option<Value> {
    let t = tokenize(SourceId(0), css_value, &ParseLimits::default());
    let cvs = build_component_values(&t.tokens, 16).ok()?;
    let spec = spec(lookup(prop)?);
    parse_property_value(spec, &cvs).ok().and_then(|v| match v {
        tpt_css::properties::SpecifiedValue::Value(v) => Some(v),
        _ => None,
    })
}

#[test]
fn registry_has_37_properties_and_ids_match_their_rows() {
    assert_eq!(PROPERTIES.len(), 50);
    assert_eq!(PROPERTIES[ids::DISPLAY.index()].name, "display");
    assert_eq!(PROPERTIES[ids::COLOR.index()].name, "color");
    assert_eq!(PROPERTIES[ids::FONT_FAMILY.index()].name, "font-family");
    assert_eq!(PROPERTIES[ids::FONT_SIZE.index()].name, "font-size");
    assert_eq!(PROPERTIES[ids::FONT_WEIGHT.index()].name, "font-weight");
    assert_eq!(
        PROPERTIES[ids::BORDER_TOP_WIDTH.index()].name,
        "border-top-width"
    );
    assert_eq!(
        PROPERTIES[ids::BORDER_TOP_STYLE.index()].name,
        "border-top-style"
    );
}

#[test]
fn length_grammar_accepts_px_em_rem_percent_and_unitless_zero() {
    assert_eq!(parse_value("width", "10px"), Some(Value::px(10.0)));
    assert_eq!(
        parse_value("width", "2EM"),
        Some(Value::Length {
            value: 2.0,
            unit: LengthUnit::Em
        })
    );
    assert_eq!(parse_value("width", "0"), Some(Value::px(0.0)));
    assert_eq!(parse_value("width", "10%"), Some(Value::Percentage(10.0)));
}

#[test]
fn length_grammar_rejects_unknown_units_and_bad_numbers() {
    assert_eq!(
        parse_value("width", "10ch"),
        None,
        "units outside the supported list are rejected"
    );
    assert_eq!(
        parse_value("width", "5"),
        None,
        "non-zero unitless numbers are invalid"
    );
    assert_eq!(parse_value("width", "1px 2px"), None);
}

#[test]
fn non_negative_properties_reject_negative_lengths_but_margins_allow_them() {
    assert_eq!(parse_value("padding-left", "-1px"), None);
    assert_eq!(parse_value("margin-left", "-1px"), Some(Value::px(-1.0)));
}

#[test]
fn keyword_grammars_are_case_insensitive_and_return_canonical_keywords() {
    assert_eq!(
        parse_value("display", "BLOCK"),
        Some(Value::Keyword("block"))
    );
    assert_eq!(parse_value("display", "block block"), None);
}

#[test]
fn line_height_accepts_normal_numbers_and_lengths() {
    assert_eq!(
        parse_value("line-height", "normal"),
        Some(Value::Keyword("normal"))
    );
    assert_eq!(parse_value("line-height", "1.5"), Some(Value::Number(1.5)));
    assert_eq!(parse_value("line-height", "-1"), None);
    assert_eq!(parse_value("line-height", "20px"), Some(Value::px(20.0)));
}

#[test]
fn font_family_lists_join_identifier_sequences_and_accept_strings() {
    assert_eq!(
        parse_value("font-family", "\"Fira Code\", Times New Roman, serif"),
        Some(Value::FontFamily(vec![
            "Fira Code".into(),
            "Times New Roman".into(),
            "serif".into()
        ]))
    );
    assert_eq!(parse_value("font-family", "a,,b"), None);
}

#[test]
fn css_wide_keywords_are_recognised_for_any_property() {
    for prop in ["width", "color", "display"] {
        assert!(matches!(
            parse_property_value(spec(lookup(prop).unwrap()), &build("inherit")),
            Ok(tpt_css::properties::SpecifiedValue::CssWide(_))
        ));
    }
    // revert is valid CSS but needs M2 origin tracking, so it is rejected for now.
    assert!(parse_property_value(spec(lookup("width").unwrap()), &build("revert")).is_err());
}

fn build(css: &str) -> Vec<tpt_css::syntax::ComponentValue> {
    let t = tokenize(SourceId(0), css, &ParseLimits::default());
    build_component_values(&t.tokens, 16).unwrap()
}

// ---- colors ----------------------------------------------------------------

fn color(css: &str) -> Option<Color> {
    parse_color(&build(css))
}

#[test]
fn hex_colors_in_both_short_and_long_form() {
    assert_eq!(color("#f00"), Some(Color::rgb8(255, 0, 0)));
    assert_eq!(color("#00FF00"), Some(Color::rgb8(0, 255, 0)));
    assert_eq!(color("#12"), None);
    assert_eq!(color("#ggg"), None);
}

#[test]
fn named_colors_are_case_insensitive_and_transparent_is_special() {
    assert_eq!(color("Navy"), Some(Color::rgb8(0, 0, 128)));
    assert_eq!(color("transparent"), Some(Color::TRANSPARENT));
    assert_eq!(color("notacolor"), None);
}

#[test]
fn rgb_and_rgba_legacy_and_space_forms() {
    assert_eq!(color("rgb(255, 0, 0)"), Some(Color::rgb8(255, 0, 0)));
    assert_eq!(color("rgb(0 0 255)"), Some(Color::rgb8(0, 0, 255)));
    assert_eq!(color("rgb(100% 0% 0%)"), Some(Color::rgb8(255, 0, 0)));
    let half = color("rgba(0, 0, 0, 0.5)").unwrap();
    assert!((half.alpha - 0.5).abs() < 1e-6);
    let slash = color("rgb(0 0 0 / 25%)").unwrap();
    assert!((slash.alpha - 0.25).abs() < 1e-6);
    assert_eq!(color("rgb(1, 2)"), None);
}

#[test]
fn color_serialization_is_canonical() {
    assert_eq!(serialize_color(&Color::rgb8(255, 0, 0)), "rgb(255, 0, 0)");
    let translucent = Color {
        alpha: 0.5,
        ..Color::rgb8(0, 0, 0)
    };
    assert_eq!(serialize_color(&translucent), "rgba(0, 0, 0, 0.5)");
    assert_eq!(serialize_value(&Value::px(16.0)), "16px");
    assert_eq!(serialize_value(&Value::Percentage(0.0)), "0%");
}

#[test]
fn every_property_has_a_defining_spec_url() {
    for p in PROPERTIES {
        assert!(p.spec_url.starts_with("https://www.w3.org/"), "{}", p.name);
    }
}
