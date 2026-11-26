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
    style_of(map, child)
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
        style_of(map, child).raw(lookup("margin-left").unwrap()),
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

// ---- long-tail properties (spec 15.2) ----------------------------------------------

fn keyword_of(css: &str, prop: &str) -> Value {
    let (t, _, child) = chain();
    let map = style_with(css, &t);
    style_of(&map, child).raw(lookup(prop).unwrap()).clone()
}

#[test]
fn long_tail_keywords_parse_and_reject_unknown_values() {
    assert_eq!(
        keyword_of("child { cursor: pointer }", "cursor"),
        Value::Keyword("pointer")
    );
    assert_eq!(
        keyword_of("child { cursor: POINTER }", "cursor"),
        Value::Keyword("pointer")
    );
    assert_eq!(
        keyword_of("child { float: left }", "float"),
        Value::Keyword("left")
    );
    assert_eq!(
        keyword_of("child { object-fit: cover }", "object-fit"),
        Value::Keyword("cover")
    );
    // Unknown keywords are dropped, so the initial value remains.
    assert_eq!(
        keyword_of("child { float: middle }", "float"),
        Value::Keyword("none")
    );
    assert_eq!(
        keyword_of("child { cursor: sparkle }", "cursor"),
        Value::Keyword("auto")
    );
}

#[test]
fn inherited_long_tail_properties_follow_the_table() {
    let (t, root, child) = chain();
    let mut t = t;
    let grand = t.element(child, "grand");
    let _ = root;
    let map = style_with(
        "child { cursor: help; text-transform: uppercase; float: right; pointer-events: none }",
        &t,
    );
    // Inherited: cursor, text-transform, and pointer-events reach the grandchild.
    assert_eq!(
        style_of(&map, grand).raw(lookup("cursor").unwrap()),
        &Value::Keyword("help")
    );
    assert_eq!(
        style_of(&map, grand).raw(lookup("text-transform").unwrap()),
        &Value::Keyword("uppercase")
    );
    assert_eq!(
        style_of(&map, grand).raw(lookup("pointer-events").unwrap()),
        &Value::Keyword("none")
    );
    // Not inherited: float stays at its initial value on the grandchild.
    assert_eq!(
        style_of(&map, grand).raw(lookup("float").unwrap()),
        &Value::Keyword("none")
    );
}

#[test]
fn text_indent_takes_lengths_and_percentages_including_negative() {
    assert_eq!(
        keyword_of("child { text-indent: -4px }", "text-indent"),
        Value::px(-4.0)
    );
    assert!(matches!(
        keyword_of("child { text-indent: 10% }", "text-indent"),
        Value::Length { .. } | Value::Percentage(_)
    ));
}

// ---- font and list-style shorthands (CSS Fonts 4, CSS Lists 3) -------------------

fn longhands_of(css: &str, props: &[&str]) -> Vec<Value> {
    let (t, _, child) = chain();
    let map = style_with(css, &t);
    props
        .iter()
        .map(|p| style_of(&map, child).raw(lookup(p).unwrap()).clone())
        .collect()
}

const FONT_LONGHANDS: [&str; 5] = [
    "font-style",
    "font-weight",
    "font-size",
    "line-height",
    "font-family",
];

#[test]
fn font_shorthand_matches_its_longhands() {
    let shorthand = longhands_of(
        "child { font: italic bold 16px/1.5 \"Helvetica Neue\", sans-serif }",
        &FONT_LONGHANDS,
    );
    let longhand = longhands_of(
        "child { font-style: italic; font-weight: bold; font-size: 16px; line-height: 1.5; font-family: \"Helvetica Neue\", sans-serif }",
        &FONT_LONGHANDS,
    );
    assert_eq!(shorthand, longhand);
}

#[test]
fn font_shorthand_resets_omitted_parts_and_accepts_spaced_slash() {
    // Omitted style and weight, and the line height, reset to their initial values.
    let shorthand = longhands_of("child { font: 12px serif }", &FONT_LONGHANDS);
    let longhand = longhands_of(
        "child { font-style: normal; font-weight: normal; font-size: 12px; line-height: normal; font-family: serif }",
        &FONT_LONGHANDS,
    );
    assert_eq!(shorthand, longhand);
    let spaced = longhands_of("child { font: 12px / 2 serif }", &FONT_LONGHANDS);
    assert_eq!(
        spaced[3],
        longhands_of("child { line-height: 2 }", &["line-height"])[0]
    );
}

#[test]
fn font_shorthand_without_a_family_is_dropped() {
    let dropped = longhands_of("child { font: 12px }", &["font-size"]);
    assert_eq!(dropped, longhands_of("child { }", &["font-size"]));
}

#[test]
fn list_style_shorthand_matches_its_longhands() {
    let props = ["list-style-type", "list-style-position", "list-style-image"];
    assert_eq!(
        longhands_of("child { list-style: inside square }", &props),
        longhands_of(
            "child { list-style-position: inside; list-style-type: square; list-style-image: none }",
            &props
        )
    );
    // A lone `none` sets both the type and the image.
    assert_eq!(
        longhands_of("child { list-style: none }", &props),
        longhands_of(
            "child { list-style-type: none; list-style-image: none; list-style-position: outside }",
            &props
        )
    );
}

#[test]
fn explicit_aliases_resolve_and_prefixes_are_not_stripped() {
    // `word-wrap` is an explicit alias of `overflow-wrap`.
    assert_eq!(
        longhands_of("child { word-wrap: break-word }", &["overflow-wrap"]),
        longhands_of("child { overflow-wrap: break-word }", &["overflow-wrap"])
    );
    // Vendor prefixes are not stripped: `-webkit-overflow-wrap` is not a property here.
    assert_eq!(
        longhands_of(
            "child { -webkit-overflow-wrap: break-word }",
            &["overflow-wrap"]
        ),
        longhands_of("child { }", &["overflow-wrap"])
    );
}

// ---- text-decoration and spacing (CSS Text Decoration 3, CSS Text 3) ---------------

#[test]
fn text_decoration_line_stores_the_set_in_spec_order() {
    let line = |css: &str| longhands_of(css, &["text-decoration-line"])[0].clone();
    assert_eq!(
        line("child { text-decoration-line: line-through underline }"),
        Value::Keyword("underline line-through")
    );
    assert_eq!(
        line("child { text-decoration-line: blink }"),
        Value::Keyword("blink")
    );
    // `none` cannot be combined with another line keyword, and duplicates are invalid.
    assert_eq!(
        line("child { text-decoration-line: none underline }"),
        Value::Keyword("none")
    );
    assert_eq!(
        line("child { text-decoration-line: underline underline }"),
        Value::Keyword("none")
    );
}

#[test]
fn text_decoration_shorthand_matches_its_longhands() {
    let props = [
        "text-decoration-line",
        "text-decoration-style",
        "text-decoration-color",
    ];
    assert_eq!(
        longhands_of("child { text-decoration: overline red wavy }", &props),
        longhands_of(
            "child { text-decoration-line: overline; text-decoration-style: wavy; text-decoration-color: red }",
            &props
        )
    );
    // Omitted parts reset to their initial values.
    assert_eq!(
        longhands_of("child { text-decoration: underline }", &props),
        longhands_of(
            "child { text-decoration-line: underline; text-decoration-style: solid; text-decoration-color: currentcolor }",
            &props
        )
    );
    // Two line keywords split across chunks still form one set.
    assert_eq!(
        longhands_of("child { text-decoration: underline blink }", &props)[0],
        Value::Keyword("underline blink")
    );
}

#[test]
fn letter_and_word_spacing_take_lengths_but_not_percentages() {
    assert_eq!(
        longhands_of("child { letter-spacing: 2px }", &["letter-spacing"])[0],
        Value::px(2.0)
    );
    assert_eq!(
        longhands_of("child { letter-spacing: -1px }", &["letter-spacing"])[0],
        Value::px(-1.0)
    );
    assert_eq!(
        longhands_of("child { word-spacing: 10% }", &["word-spacing"])[0],
        Value::Keyword("normal")
    );
    assert_eq!(
        longhands_of("child { word-spacing: normal }", &["word-spacing"])[0],
        Value::Keyword("normal")
    );
}

// ---- rule index (spec 21.2): every subject shape is still matched -----------------

#[test]
fn every_subject_shape_is_found_by_the_index() {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    t.set_id(child, "x");
    t.add_class(child, "a");
    t.add_class(child, "a");
    t.add_class(child, "b");
    let css = "
        child#x.a { margin-left: 1px }
        .b { margin-top: 2px }
        :is(.a) { margin-right: 3px }
        * { margin-bottom: 4px }
        #x { padding-left: 5px }
        child.b { padding-top: 6px }
        .zz { padding-right: 7px }
    ";
    let map = style_with(css, &t);
    let s = style_of(&map, child);
    assert!(
        px_near(s.raw(lookup("margin-left").unwrap()), 1.0),
        "id, class, and type"
    );
    assert!(
        px_near(s.raw(lookup("margin-top").unwrap()), 2.0),
        "class subject"
    );
    assert!(
        px_near(s.raw(lookup("margin-right").unwrap()), 3.0),
        "unkeyed :is() subject"
    );
    assert!(
        px_near(s.raw(lookup("margin-bottom").unwrap()), 4.0),
        "universal subject"
    );
    assert!(
        px_near(s.raw(lookup("padding-left").unwrap()), 5.0),
        "id subject"
    );
    assert!(
        px_near(s.raw(lookup("padding-top").unwrap()), 6.0),
        "type with class"
    );
    // A class the node does not have must not match.
    assert!(!px_near(s.raw(lookup("padding-right").unwrap()), 7.0));
}

#[test]
fn the_configured_calc_limit_is_the_effective_limit() {
    // 33 nested calc() calls: past the old internal bound of 32, within a configured limit of 40.
    let mut css = String::from("child { margin-left: ");
    for _ in 0..33 {
        css.push_str("calc(");
    }
    css.push_str("1px");
    for _ in 0..33 {
        css.push(')');
    }
    css.push_str(" }");
    assert!(px_near(&margin_with_calc_limit(&css, 40), 1.0));
    assert_eq!(margin_with_calc_limit(&css, 32), margin("child { }"));
}

// ---- shadow DOM (spec 9.3, Shadow DOM 1) ----------------------------------------

struct Shadowed {
    t: TestTree,
    host: NodeId,
    light: NodeId,
    unslotted: NodeId,
    inner: NodeId,
}

/// `div.card` (the host) with two light `p` children, one assigned to a slot in
/// its shadow tree and one not. The shadow tree holds a `slot` and a `span`.
fn shadowed() -> Shadowed {
    let mut t = TestTree::new(NodeKind::Element);
    let doc = t.root();
    t.nodes[0].name.local = "html".into();
    let host = t.element(doc, "div");
    t.add_class(host, "card");
    let light = t.element(host, "p");
    let unslotted = t.element(host, "p");
    let root = t.attach_shadow(host);
    let slot = t.element(root, "slot");
    let inner = t.element(root, "span");
    t.assign_slot(light, slot);
    Shadowed {
        t,
        host,
        light,
        unslotted,
        inner,
    }
}

fn styled(doc_css: &str, shadow_css: &str, s: &Shadowed) -> tpt_css::StyleMap {
    let mut r = StyleResolver::new(ResolverConfig::default());
    r.add_stylesheet(parse_sheet(doc_css).stylesheet, Origin::Author);
    let root = s.t.shadow_root_for(s.host);
    r.add_shadow_stylesheet(root, parse_sheet(shadow_css).stylesheet, Origin::Author);
    r.resolve(&s.t, &Environment::default()).clone()
}

#[test]
fn a_shadow_sheet_styles_its_own_tree_and_document_rules_do_not_reach_it() {
    let s = shadowed();
    let red = Color::rgb8(255, 0, 0);
    let map = styled("span { color: blue }", "span { color: red }", &s);
    assert_eq!(style_of(&map, s.inner).color(), red);
    // The document rule does not reach the span in the shadow tree.
    let map = styled("span { color: blue }", "", &s);
    assert_ne!(style_of(&map, s.inner).color(), Color::rgb8(0, 0, 255));
}

#[test]
fn host_matches_the_host_and_its_functional_forms() {
    let s = shadowed();
    let css = ":host { margin-left: 3px } :host(.card) { margin-top: 4px } :host(.nope) { margin-right: 9px } :host-context(body) { margin-bottom: 5px }";
    let map = styled("", css, &s);
    let host = style_of(&map, s.host);
    assert!(px_near(host.raw(lookup("margin-left").unwrap()), 3.0));
    assert!(px_near(host.raw(lookup("margin-top").unwrap()), 4.0));
    assert!(!px_near(host.raw(lookup("margin-right").unwrap()), 9.0));
    // `:host-context(body)` needs an ancestor body; there is none here.
    assert!(!px_near(host.raw(lookup("margin-bottom").unwrap()), 5.0));
    // Outside the host, `:host` matches nothing.
    assert!(!px_near(
        style_of(&map, s.light).raw(lookup("margin-left").unwrap()),
        3.0
    ));
}

#[test]
fn host_descendant_selectors_reach_into_the_shadow_tree() {
    let s = shadowed();
    let map = styled("", ":host span { color: green }", &s);
    assert_eq!(style_of(&map, s.inner).color(), Color::rgb8(0, 128, 0));
    // A document selector cannot cross into the shadow tree.
    let map = styled("div span { color: blue }", "", &s);
    assert_ne!(style_of(&map, s.inner).color(), Color::rgb8(0, 0, 255));
}

#[test]
fn shadow_children_inherit_from_the_host() {
    let s = shadowed();
    let map = styled(".card { color: #010203 }", "", &s);
    assert_eq!(style_of(&map, s.inner).color(), Color::rgb8(1, 2, 3));
}

#[test]
fn slotted_applies_to_the_assigned_element_only() {
    let s = shadowed();
    let map = styled("", "::slotted(p) { margin-left: 4px }", &s);
    assert!(px_near(
        style_of(&map, s.light).raw(lookup("margin-left").unwrap()),
        4.0
    ));
    assert!(!px_near(
        style_of(&map, s.unslotted).raw(lookup("margin-left").unwrap()),
        4.0
    ));
    // A document sheet's `::slotted()` has no shadow tree to refer to, so it does nothing.
    let map = styled("::slotted(p) { margin-left: 4px }", "", &s);
    assert!(!px_near(
        style_of(&map, s.light).raw(lookup("margin-left").unwrap()),
        4.0
    ));
}

#[test]
fn part_matches_an_element_with_the_part_name_under_a_matching_host() {
    let mut s = shadowed();
    s.t.set_attr(s.inner, "part", "label other");
    let map = styled(".card::part(label) { color: purple }", "", &s);
    assert_eq!(style_of(&map, s.inner).color(), Color::rgb8(128, 0, 128));
    // A part name the element does not have does not match.
    let map = styled(".card::part(missing) { color: purple }", "", &s);
    assert_ne!(style_of(&map, s.inner).color(), Color::rgb8(128, 0, 128));
    // A host selector that does not match the host does not match either.
    let map = styled(".other::part(label) { color: purple }", "", &s);
    assert_ne!(style_of(&map, s.inner).color(), Color::rgb8(128, 0, 128));
}

#[test]
fn a_host_attribute_change_restyles_host_dependent_rules() {
    let mut s = shadowed();
    let mut r = StyleResolver::new(ResolverConfig::default());
    let root = s.t.shadow_root_for(s.host);
    r.add_shadow_stylesheet(
        root,
        parse_sheet(":host(.on) span { color: green }").stylesheet,
        Origin::Author,
    );
    let _ = r.resolve(&s.t, &Environment::default());
    s.t.add_class(s.host, "on");
    r.invalidate(&s.t, tpt_css::StyleChange::AttributeChanged(s.host));
    let restyled = r.restyle(&s.t, &Environment::default()).clone();
    assert_eq!(style_of(&restyled, s.inner).color(), Color::rgb8(0, 128, 0));
    let full = styled("", ":host(.on) span { color: green }", &s);
    assert_eq!(restyled, full);
}

#[test]
fn state_selector_matches_custom_states_case_sensitively() {
    let (mut t, _, child) = chain();
    let map = style_with("child:state(loading) { color: red }", &t);
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
    t.nodes[child.index as usize].state.custom.insert("loading");
    let map = style_with("child:state(loading) { color: red }", &t);
    assert_eq!(style_of(&map, child).color(), Color::rgb8(255, 0, 0));
    let map = style_with("child:state(Loading) { color: red }", &t);
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
}

#[test]
fn slotted_elements_inherit_from_the_slot_and_others_from_the_host() {
    let s = shadowed();
    let map = styled(".card { color: #ff0000 }", "slot { color: #0000ff }", &s);
    assert_eq!(style_of(&map, s.light).color(), Color::rgb8(0, 0, 255));
    assert_eq!(style_of(&map, s.unslotted).color(), Color::rgb8(255, 0, 0));
}

#[test]
fn highlight_pseudo_element_is_stored_under_its_name() {
    let (t, _, child) = chain();
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(
        parse_sheet("child::highlight(spell) { color: red }").stylesheet,
        Origin::Author,
    );
    let map = resolver.resolve(&t, &Environment::default()).clone();
    let pseudo = map
        .get_pseudo(
            child,
            &tpt_css::selectors::PseudoElement::Highlight("spell".into()),
        )
        .expect("the highlight style is stored");
    assert_eq!(pseudo.color(), Color::rgb8(255, 0, 0));
    assert_eq!(style_of(&map, child).color(), Color::BLACK);
}

#[test]
fn border_radius_expands_to_the_four_corners_in_clockwise_order() {
    let corners = [
        "border-top-left-radius",
        "border-top-right-radius",
        "border-bottom-right-radius",
        "border-bottom-left-radius",
    ];
    assert_eq!(
        longhands_of("child { border-radius: 1px 2px 3px }", &corners),
        longhands_of(
            "child { border-top-left-radius: 1px; border-top-right-radius: 2px; border-bottom-right-radius: 3px; border-bottom-left-radius: 2px }",
            &corners
        )
    );
    // The elliptical `/` form is not supported, so the declaration is dropped.
    assert_eq!(
        longhands_of("child { border-radius: 1px / 2px }", &corners),
        longhands_of("child { }", &corners)
    );
}

#[test]
fn nth_of_type_counts_only_siblings_of_the_same_name() {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "html".into();
    let a = t.element(root, "div");
    let b = t.element(root, "span");
    let c = t.element(root, "div");
    let d = t.element(root, "span");
    let e = t.element(root, "div");
    let css = "div:nth-of-type(2) { margin-left: 1px } \
               span:nth-last-of-type(1) { margin-top: 2px } \
               div:nth-last-of-type(1) { margin-right: 3px } \
               b ~ span { color: red }";
    let map = style_with(css, &t);
    assert!(!px_near(
        style_of(&map, a).raw(lookup("margin-left").unwrap()),
        1.0
    ));
    assert!(px_near(
        style_of(&map, c).raw(lookup("margin-left").unwrap()),
        1.0
    ));
    assert!(!px_near(
        style_of(&map, e).raw(lookup("margin-left").unwrap()),
        1.0
    ));
    assert!(px_near(
        style_of(&map, d).raw(lookup("margin-top").unwrap()),
        2.0
    ));
    assert!(px_near(
        style_of(&map, e).raw(lookup("margin-right").unwrap()),
        3.0
    ));
    // `b ~ span` needs an earlier `b` sibling, and there is none.
    assert_eq!(style_of(&map, b).color(), Color::BLACK);
}
