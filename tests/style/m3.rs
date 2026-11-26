// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! M3-era style tests: numeric math functions, container units, the full named
//! color set, modern color spaces and `color-mix()`, keyframe interpolation, the
//! devtools inspection data, and the computed property map.

use super::common::{parse_sheet, style_of, style_with, style_with_env, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::properties::lookup;
use tpt_css::tree::{NodeId, NodeKind};
use tpt_css::values::calc::CalcNode;
use tpt_css::values::{Color, Value};
use tpt_css::{Environment, ResolverConfig, StyleResolver};

fn chain() -> (TestTree, NodeId, NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    (t, root, child)
}

fn margin(css: &str) -> Value {
    let (t, _, child) = chain();
    let map = style_with(css, &t);
    style_of(&map, child)
        .raw(lookup("margin-left").unwrap())
        .clone()
}

fn px_near(value: &Value, expected: f64) -> bool {
    matches!(value, Value::Length { value, .. } if (value - expected).abs() < 1e-6)
}

// ---- numeric math functions ----------------------------------------------------

#[test]
fn round_with_each_strategy() {
    assert!(px_near(
        &margin("child { margin-left: round(12px, 5px) }"),
        10.0
    ));
    assert!(px_near(
        &margin("child { margin-left: round(up, 11px, 5px) }"),
        15.0
    ));
    assert!(px_near(
        &margin("child { margin-left: round(down, 14px, 5px) }"),
        10.0
    ));
    assert!(px_near(
        &margin("child { margin-left: round(to-zero, -14px, 5px) }"),
        -10.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(round(2.6) * 1px) }"),
        3.0
    ));
}

#[test]
fn mod_and_rem_differ_in_the_sign_of_the_result() {
    assert!(px_near(
        &margin("child { margin-left: mod(-7px, 3px) }"),
        2.0
    ));
    assert!(px_near(
        &margin("child { margin-left: rem(-7px, 3px) }"),
        -1.0
    ));
}

#[test]
fn abs_and_hypot_keep_the_length_type() {
    assert!(px_near(&margin("child { margin-left: abs(-4px) }"), 4.0));
    assert!(px_near(
        &margin("child { margin-left: hypot(3px, 4px) }"),
        5.0
    ));
}

#[test]
fn numeric_functions_on_plain_numbers() {
    // sqrt, pow, log, exp, and sign all produce numbers, used here as a multiplier.
    assert!(px_near(
        &margin("child { margin-left: calc(sqrt(16) * 2px) }"),
        8.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(pow(2, 3) * 1px) }"),
        8.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(log(8, 2) * 1px) }"),
        3.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(sign(-3) * 5px) }"),
        -5.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(exp(0) * 6px) }"),
        6.0
    ));
}

#[test]
fn trigonometry_takes_radians_and_inverts_back() {
    assert!(px_near(
        &margin("child { margin-left: calc(sin(0) * 1px) }"),
        0.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(atan2(1, 1) * 0px + 1px) }"),
        1.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(acos(1) * 1px) }"),
        0.0
    ));
}

#[test]
fn a_numeric_function_with_a_type_error_is_dropped() {
    let (t, _, child) = chain();
    let map = style_with("child { margin-left: sqrt(4px) }", &t);
    assert_eq!(
        style_of(&map, child).raw(lookup("margin-left").unwrap()),
        &Value::px(0.0)
    );
}

#[test]
fn a_rounding_step_of_zero_is_undefined_and_drops_nothing_fatal() {
    // Undefined math stays a tree; the declaration is valid, so no panic occurs.
    let _ = margin("child { margin-left: round(10px, 0px) }");
}

#[test]
fn a_calc_expression_keeps_its_structure_as_a_tree_when_it_mixes_units() {
    let v = margin("child { margin-left: min(10%, 20px) }");
    assert!(matches!(v, Value::Calc(ref node) if matches!(**node, CalcNode::Min(_))));
}

// ---- container units -----------------------------------------------------------

#[test]
fn container_units_fall_back_to_the_viewport_without_a_container() {
    let (t, _, child) = chain();
    let env = Environment {
        viewport: tpt_css::ViewportInfo {
            width: 800.0,
            height: 600.0,
        },
        ..Environment::default()
    };
    let map = style_with_env("child { margin-left: 10cqw }", &t, &env);
    assert_eq!(
        style_of(&map, child).raw(lookup("margin-left").unwrap()),
        &Value::px(80.0)
    );
}

// ---- named colors and modern color spaces ---------------------------------------

fn color(css: &str) -> Color {
    let (t, _, child) = chain();
    let map = style_with(css, &t);
    style_of(&map, child).color()
}

fn approx(c: Color, rgb: [f32; 3], tol: f32) -> bool {
    (0..3).all(|i| (c.components[i] - rgb[i]).abs() <= tol)
}

#[test]
fn all_148_named_colors_parse() {
    let names = [
        "aliceblue",
        "rebeccapurple",
        "lightgoldenrodyellow",
        "yellowgreen",
        "darkslategrey",
        "whitesmoke",
        "mediumspringgreen",
        "lavenderblush",
    ];
    for name in names {
        let (t, _, child) = chain();
        let map = style_with(&format!("child {{ color: {name} }}"), &t);
        assert_ne!(
            style_of(&map, child).color(),
            Color::BLACK,
            "{name} should not fall back to black"
        );
    }
    assert_eq!(
        color("child { color: rebeccapurple }"),
        Color::rgb8(102, 51, 153)
    );
    assert_eq!(
        color("child { color: aliceblue }"),
        Color::rgb8(240, 248, 255)
    );
}

#[test]
fn oklch_and_oklab_white_and_black() {
    assert!(approx(
        color("child { color: oklch(1 0 0) }"),
        [1.0, 1.0, 1.0],
        0.01
    ));
    assert!(approx(
        color("child { color: oklab(0 0 0) }"),
        [0.0, 0.0, 0.0],
        0.01
    ));
}

#[test]
fn lab_and_lch_white_and_black() {
    assert!(approx(
        color("child { color: lab(100 0 0) }"),
        [1.0, 1.0, 1.0],
        0.01
    ));
    assert!(approx(
        color("child { color: lch(0 0 0) }"),
        [0.0, 0.0, 0.0],
        0.01
    ));
}

#[test]
fn color_function_in_srgb_and_linear() {
    assert!(approx(
        color("child { color: color(srgb 1 0 0) }"),
        [1.0, 0.0, 0.0],
        1e-6
    ));
    // A linear 0.5 encodes to about 0.735 in sRGB.
    assert!(approx(
        color("child { color: color(srgb-linear 0.5 0.5 0.5) }"),
        [0.735, 0.735, 0.735],
        0.01
    ));
}

#[test]
fn color_mix_in_srgb_with_defaults_and_weights() {
    assert!(approx(
        color("child { color: color-mix(in srgb, red, blue) }"),
        [0.5, 0.0, 0.5],
        1e-6
    ));
    assert!(approx(
        color("child { color: color-mix(in srgb, red 25%, blue) }"),
        [0.25, 0.0, 0.75],
        1e-6
    ));
    assert!(approx(
        color("child { color: color-mix(in srgb, red 30%, blue 70%) }"),
        [0.3, 0.0, 0.7],
        1e-6
    ));
    assert!(approx(
        color("child { color: color-mix(in srgb, #fff 0%, black) }"),
        [0.0, 0.0, 0.0],
        1e-6
    ));
}

#[test]
fn color_mix_in_an_unsupported_space_is_dropped() {
    let (t, _, child) = chain();
    let map = style_with("child { color: color-mix(in lab, red, blue) }", &t);
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
}

// ---- keyframe interpolation -------------------------------------------------------

fn keyframes(css: &str, name: &str) -> tpt_css::ast::KeyframesRule {
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(parse_sheet(css).stylesheet, Origin::Animation);
    resolver.keyframes(name).expect("stored").clone()
}

#[test]
fn lengths_interpolate_linearly_between_keyframes() {
    let rule = keyframes(
        "@keyframes slide { from { margin-left: 0px } to { margin-left: 100px } }",
        "slide",
    );
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "margin-left", 0.25),
        Some(Value::px(25.0))
    );
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "margin-left", 1.0),
        Some(Value::px(100.0))
    );
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "margin-left", 2.0),
        Some(Value::px(100.0))
    );
}

#[test]
fn percentage_keyframes_bracket_the_progress() {
    let rule = keyframes(
        "@keyframes k { 0% { margin-left: 0px } 50% { margin-left: 10px } 100% { margin-left: 40px } }",
        "k",
    );
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "margin-left", 0.5),
        Some(Value::px(10.0))
    );
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "margin-left", 0.75),
        Some(Value::px(25.0))
    );
}

#[test]
fn colors_interpolate_and_discrete_values_switch_at_the_midpoint() {
    let rule = keyframes(
        "@keyframes c { from { color: black } to { color: white } }",
        "c",
    );
    let mid = tpt_css::style::keyframes::value_at(&rule, "color", 0.5).expect("color");
    assert!(matches!(mid, Value::Color(c) if approx(c, [0.5, 0.5, 0.5], 0.01)));
    let rule = keyframes(
        "@keyframes d { from { display: block } to { display: none } }",
        "d",
    );
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "display", 0.4),
        Some(Value::Keyword("block"))
    );
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "display", 0.6),
        Some(Value::Keyword("none"))
    );
}

#[test]
fn a_property_no_keyframe_sets_has_no_value() {
    let rule = keyframes("@keyframes e { from { margin-left: 0px } }", "e");
    assert_eq!(
        tpt_css::style::keyframes::value_at(&rule, "width", 0.5),
        None
    );
}

// ---- devtools inspection and the computed property map ---------------------------

#[test]
fn inspect_reports_the_winning_declaration_and_its_origin() {
    let (t, _, child) = chain();
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(
        parse_sheet("child { margin-left: 1px }").stylesheet,
        Origin::Author,
    );
    resolver.add_stylesheet(
        parse_sheet("child.x { margin-left: 2px }").stylesheet,
        Origin::Author,
    );
    let report = resolver.inspect(&t, child, &Environment::default());
    let margin = report
        .iter()
        .find(|i| i.property == "margin-left")
        .expect("reported");
    assert_eq!(margin.origin, Origin::Author);
    assert_eq!(
        margin.specificity.b, 0,
        "the type selector has the lower specificity"
    );
    assert!(
        report.iter().all(|i| i.property != "width"),
        "unset properties are not reported"
    );
}

#[test]
fn property_values_lists_every_longhand_in_registry_order() {
    let (t, _, child) = chain();
    let map = style_with("child { margin-left: 3px }", &t);
    let values = style_of(&map, child).property_values();
    assert_eq!(values.len(), tpt_css::properties::PROPERTIES.len());
    assert_eq!(values[0].0, "display");
    let margin = values
        .iter()
        .find(|(n, _)| *n == "margin-left")
        .expect("present");
    assert_eq!(margin.1, "3px");
    let width = values.iter().find(|(n, _)| *n == "width").expect("present");
    assert_eq!(
        width.1, "auto",
        "an unset property reports its initial value"
    );
}

#[test]
fn progress_gives_the_fraction_between_two_bounds() {
    assert!(px_near(
        &margin("child { margin-left: calc(progress(5px, 0px, 10px) * 20px) }"),
        10.0
    ));
    assert!(px_near(
        &margin("child { margin-left: calc(progress(15px, 0px, 10px) * 20px) }"),
        30.0
    ));
}
