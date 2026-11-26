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
    let map = style_with("child { color: color-mix(in xyz, red, blue) }", &t);
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

// ---- calc() nesting limit (spec 20.3) -------------------------------------------

fn margin_with_calc_limit(css: &str, max_calc_depth: usize) -> Value {
    use tpt_css::{ParseLimits, ParseOptions};
    let limits = ParseLimits {
        max_calc_depth,
        ..ParseLimits::default()
    };
    let parsed = tpt_css::parse_stylesheet(
        css,
        ParseOptions {
            limits,
            ..ParseOptions::default()
        },
    );
    let (t, _, child) = chain();
    let mut r = StyleResolver::new(ResolverConfig {
        limits,
        ..ResolverConfig::default()
    });
    r.add_stylesheet(parsed.stylesheet, Origin::Author);
    let map = r.resolve(&t, &Environment::default());
    style_of(&map, child)
        .raw(lookup("margin-left").unwrap())
        .clone()
}

#[test]
fn a_declaration_nested_past_max_calc_depth_is_dropped_at_parse_time() {
    let css = "child { margin-left: calc(calc(calc(1px))) }";
    // Depth 3 is within the default limit.
    assert!(px_near(&margin_with_calc_limit(css, 32), 1.0));
    // Depth 3 is past a limit of 2, so the declaration is dropped.
    assert_eq!(margin_with_calc_limit(css, 2), margin("child { }"));
}

#[test]
fn a_var_that_nests_calc_past_the_limit_is_dropped_at_computed_time() {
    let css = "
        :root { --c: calc(calc(1px)) }
        child { margin-left: var(--c) }
    ";
    assert!(px_near(&margin_with_calc_limit(css, 32), 1.0));
    assert_eq!(margin_with_calc_limit(css, 1), margin("child { }"));
}

// ---- media features added in M4 (Media Queries 5) -----------------------------

fn margin_in_env(css: &str, env: &Environment) -> Value {
    let (t, _, child) = chain();
    let map = style_with_env(css, &t, env);
    style_of(&map, child)
        .raw(lookup("margin-left").unwrap())
        .clone()
}

fn media_rule(query: &str) -> String {
    format!("@media {query} {{ child {{ margin-left: 5px }} }}")
}

#[test]
fn prefers_contrast_matches_its_own_preference_only() {
    use tpt_css::ast::media::ContrastKind;
    let css = media_rule("(prefers-contrast: more)");
    let mut env = Environment::default();
    assert!(!px_near(&margin_in_env(&css, &env), 5.0));
    env.media.contrast = ContrastKind::More;
    assert!(px_near(&margin_in_env(&css, &env), 5.0));
}

#[test]
fn dynamic_range_and_update_read_the_environment() {
    use tpt_css::ast::media::UpdateKind;
    let high = media_rule("(dynamic-range: high)");
    let mut env = Environment::default();
    assert!(!px_near(&margin_in_env(&high, &env), 5.0));
    env.media.high_dynamic_range = true;
    assert!(px_near(&margin_in_env(&high, &env), 5.0));

    let slow = media_rule("(update: slow)");
    let mut env = Environment::default();
    assert!(!px_near(&margin_in_env(&slow, &env), 5.0));
    env.media.update = UpdateKind::Slow;
    assert!(px_near(&margin_in_env(&slow, &env), 5.0));
}

#[test]
fn resolution_compares_in_device_pixels_per_px() {
    let mut env = Environment::default();
    env.media.device_pixel_ratio = 2.0;
    assert!(px_near(
        &margin_in_env(&media_rule("(min-resolution: 2dppx)"), &env),
        5.0
    ));
    assert!(!px_near(
        &margin_in_env(&media_rule("(max-resolution: 1.5x)"), &env),
        5.0
    ));

    // 96dpi is 1dppx, so a 1 dppx screen matches it exactly.
    let env = Environment::default();
    assert!(px_near(
        &margin_in_env(&media_rule("(resolution: 96dpi)"), &env),
        5.0
    ));
    // 37dpcm is about 0.98dppx and 36dpcm about 0.95dppx.
    assert!(px_near(
        &margin_in_env(&media_rule("(min-resolution: 37dpcm)"), &env),
        5.0
    ));
    assert!(!px_near(
        &margin_in_env(&media_rule("(max-resolution: 36dpcm)"), &env),
        5.0
    ));
}

// ---- scope proximity (Cascade 6) -------------------------------------------------

/// `outer > inner > p`, with classes `outer` and `inner`.
fn nested_scopes() -> (TestTree, NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "html".into();
    let outer = t.element(root, "div");
    t.add_class(outer, "outer");
    let inner = t.element(outer, "div");
    t.add_class(inner, "inner");
    let p = t.element(inner, "p");
    (t, p)
}

fn color_of(css: &str) -> Color {
    let (t, p) = nested_scopes();
    let map = style_with(css, &t);
    style_of(&map, p).color()
}

#[test]
fn the_nearer_scope_root_wins_whatever_the_source_order() {
    let red = Color::rgb8(255, 0, 0);
    let green = Color::rgb8(0, 128, 0);
    // `.inner` is one step from `p`, `.outer` is two, so `.inner` wins.
    let inner_first = "@scope (.inner) { p { color: green } } @scope (.outer) { p { color: red } }";
    let outer_first = "@scope (.outer) { p { color: red } } @scope (.inner) { p { color: green } }";
    assert_eq!(color_of(inner_first), green);
    assert_eq!(color_of(outer_first), green);
    assert_ne!(color_of(outer_first), red);
}

#[test]
fn specificity_decides_before_scope_proximity() {
    // Cascade 6 sorts specificity before proximity: the more specific unscoped
    // `div p` (0,0,2) beats the scoped `p` (0,0,1).
    let css = "@scope (.outer) { p { color: red } } div p { color: blue }";
    assert_eq!(color_of(css), Color::rgb8(0, 0, 255));
    // Without any scope, specificity decides as before.
    assert_eq!(
        color_of("p { color: red } div p { color: blue }"),
        Color::rgb8(0, 0, 255)
    );
}

#[test]
fn scope_proximity_breaks_ties_before_source_order() {
    // Equal specificity: the scoped rule wins even though the unscoped rule
    // comes later, because an unscoped rule has infinite proximity.
    let css = "@scope (.outer) { p { color: red } } p { color: blue }";
    assert_eq!(color_of(css), Color::rgb8(255, 0, 0));
}

#[test]
fn the_stylesheet_count_is_bounded_and_the_overflow_is_reported() {
    use tpt_css::{ParseLimits, StylesheetLimitExceeded};
    let config = ResolverConfig {
        limits: ParseLimits {
            max_stylesheets: 1,
            ..ParseLimits::default()
        },
        ..ResolverConfig::default()
    };
    let mut r = StyleResolver::new(config);
    let first = r.try_add_stylesheet(
        parse_sheet("child { margin-left: 1px }").stylesheet,
        Origin::Author,
    );
    assert!(first.is_ok());
    let second = r.try_add_stylesheet(
        parse_sheet("child { margin-left: 2px }").stylesheet,
        Origin::Author,
    );
    assert_eq!(second, Err(StylesheetLimitExceeded { limit: 1 }));
    // The sheet that was refused has no effect on the result.
    let (t, _, child) = chain();
    let map = r.resolve(&t, &Environment::default());
    assert!(px_near(
        style_of(&map, child).raw(lookup("margin-left").unwrap()),
        1.0
    ));
}

// ---- relative color syntax (CSS Color 5) ----------------------------------------

fn close_color(a: Color, b: Color) -> bool {
    let near = |x: f32, y: f32| (x - y).abs() < 2e-3;
    near(a.alpha, b.alpha)
        && a.components
            .iter()
            .zip(b.components.iter())
            .all(|(x, y)| near(*x, *y))
}

#[test]
fn relative_syntax_round_trips_every_space() {
    // Each function, with the keywords for its own channels, must give back
    // the origin color. This checks the forward conversions against the
    // existing inverses.
    let origins = [
        ("#3a7fc2", Color::rgb8(0x3a, 0x7f, 0xc2)),
        ("#ff8000", Color::rgb8(0xff, 0x80, 0x00)),
        ("#808080", Color::rgb8(0x80, 0x80, 0x80)),
        ("#102030", Color::rgb8(0x10, 0x20, 0x30)),
        ("white", Color::rgb8(255, 255, 255)),
        ("black", Color::rgb8(0, 0, 0)),
    ];
    let functions = [
        ("rgb", "r g b"),
        ("hsl", "h s l"),
        ("hwb", "h w b"),
        ("lab", "l a b"),
        ("lch", "l c h"),
        ("oklab", "l a b"),
        ("oklch", "l c h"),
    ];
    for (origin, want) in origins {
        for (function, channels) in functions {
            let css = format!("p {{ color: {function}(from {origin} {channels}) }}");
            let got = color_of(&css);
            assert!(
                close_color(got, want),
                "{function}(from {origin} {channels}) gave {got:?}, want {want:?}"
            );
        }
    }
}

#[test]
fn the_alpha_defaults_to_the_origin_and_can_be_set() {
    assert!(
        (color_of("p { color: rgb(from rgb(0 0 255 / 0.25) r g b) }").alpha - 0.25).abs() < 1e-6
    );
    assert!((color_of("p { color: rgb(from red r g b / 0.5) }").alpha - 0.5).abs() < 1e-6);
}

#[test]
fn keywords_take_math_and_percentages() {
    // r = 255 halved is 127.5, which is 0.5 of full scale.
    let half = color_of("p { color: rgb(from red calc(r * 0.5) g b) }");
    assert!((half.components[0] - 0.5).abs() < 1e-3);
    assert!(half.components[1].abs() < 1e-6);
    // 50% of 255 for red.
    let pct = color_of("p { color: rgb(from black 50% 0 0) }");
    assert!((pct.components[0] - 0.5).abs() < 1e-3);
    // Lighter red: hsl(0 100% 70%), which is (1, 0.4, 0.4).
    let light = color_of("p { color: hsl(from #ff0000 h s calc(l + 20)) }");
    assert!((light.components[0] - 1.0).abs() < 1e-3);
    assert!((light.components[1] - 0.4).abs() < 1e-3);
    assert!((light.components[2] - 0.4).abs() < 1e-3);
}

#[test]
fn malformed_relative_colors_are_dropped() {
    let initial = color_of("p { }");
    // Too few channels, and a keyword that the space does not have.
    assert_eq!(color_of("p { color: rgb(from red r g) }"), initial);
    assert_eq!(color_of("p { color: rgb(from red x g b) }"), initial);
    // A missing origin.
    assert_eq!(color_of("p { color: rgb(from r g b) }"), initial);
}

// ---- color-mix() in more spaces and relative color() -------------------------------

#[test]
fn mixing_a_color_with_itself_gives_it_back_in_every_space() {
    for space in [
        "srgb",
        "srgb-linear",
        "hsl",
        "hwb",
        "lab",
        "lch",
        "oklab",
        "oklch",
    ] {
        for origin in ["#3a7fc2", "#ff8000", "#808080", "#102030"] {
            let css = format!("p {{ color: color-mix(in {space}, {origin} 50%, {origin}) }}");
            let want = color_of(&format!("p {{ color: {origin} }}"));
            assert!(
                close_color(color_of(&css), want),
                "color-mix in {space} of {origin} with itself"
            );
        }
    }
}

#[test]
fn a_hundred_percent_weight_returns_that_color() {
    for space in ["srgb-linear", "lab", "lch", "oklab", "oklch", "hsl", "hwb"] {
        let css = format!("p {{ color: color-mix(in {space}, #3a7fc2 100%, #ff8000) }}");
        assert!(
            close_color(color_of(&css), Color::rgb8(0x3a, 0x7f, 0xc2)),
            "{space}"
        );
    }
}

#[test]
fn srgb_linear_mixes_in_linear_light() {
    // Half of black and white in linear light is 0.5 linear, which encodes to
    // about 0.735 in sRGB. Mixing in gamma-encoded sRGB would give 0.5.
    let c = color_of("p { color: color-mix(in srgb-linear, black, white) }");
    assert!((c.components[0] - 0.7354).abs() < 2e-3, "{c:?}");
}

#[test]
fn hue_interpolation_follows_the_chosen_arc() {
    // Hue 350 to hue 10. The shorter arc passes through 0 (red), and the
    // longer arc passes through 180 (cyan).
    let shorter =
        color_of("p { color: color-mix(in hsl shorter hue, hsl(350 100% 50%), hsl(10 100% 50%)) }");
    assert!(
        shorter.components[0] > 0.99 && shorter.components[1] < 0.01,
        "{shorter:?}"
    );
    let longer =
        color_of("p { color: color-mix(in hsl longer hue, hsl(350 100% 50%), hsl(10 100% 50%)) }");
    assert!(
        longer.components[0] < 0.01 && longer.components[1] > 0.99,
        "{longer:?}"
    );
}

#[test]
fn alpha_is_premultiplied_so_transparent_colors_do_not_tint() {
    // `transparent` is black at alpha 0. Premultiplying keeps the red unchanged.
    let c = color_of("p { color: color-mix(in srgb, transparent 50%, red) }");
    assert!((c.alpha - 0.5).abs() < 1e-6);
    assert!((c.components[0] - 1.0).abs() < 1e-3);
    assert!(c.components[1].abs() < 1e-3);
}

#[test]
fn relative_color_function_binds_the_origin_channels() {
    let origin = Color::rgb8(0x3a, 0x7f, 0xc2);
    assert!(close_color(
        color_of("p { color: color(from #3a7fc2 srgb r g b) }"),
        origin
    ));
    assert!(close_color(
        color_of("p { color: color(from #3a7fc2 srgb-linear r g b) }"),
        origin
    ));
    let half = color_of("p { color: color(from red srgb calc(r * 0.5) g b) }");
    assert!((half.components[0] - 0.5).abs() < 1e-3);
}

#[test]
fn malformed_mixes_are_dropped() {
    let initial = color_of("p { }");
    // A longer-hue method needs a polar space.
    assert_eq!(
        color_of("p { color: color-mix(in srgb longer hue, red, blue) }"),
        initial
    );
    assert_eq!(
        color_of("p { color: color-mix(in xyz, red, blue) }"),
        initial
    );
}

// ---- long-tail state pseudo-classes (spec 9.2, 12.2) -----------------------------

#[test]
fn host_reported_state_pseudo_classes_match_only_when_set() {
    use tpt_css::tree::StateBits;
    let cases = [
        ("current", StateBits::CURRENT),
        ("past", StateBits::PAST),
        ("future", StateBits::FUTURE),
        ("defined", StateBits::DEFINED),
        ("local-link", StateBits::LOCAL_LINK),
        ("playing", StateBits::PLAYING),
        ("paused", StateBits::PAUSED),
        ("fullscreen", StateBits::FULLSCREEN),
        ("autofill", StateBits::AUTOFILL),
    ];
    let css = |name: &str| format!("child:{name} {{ color: red }}");
    for (name, bit) in cases {
        let (mut t, _, child) = chain();
        assert_eq!(
            style_of(&style_with(&css(name), &t), child).color(),
            Color::BLACK,
            ":{name} without its state"
        );
        t.set_state(child, bit);
        assert_eq!(
            style_of(&style_with(&css(name), &t), child).color(),
            Color::rgb8(255, 0, 0),
            ":{name} with its state"
        );
    }
}
