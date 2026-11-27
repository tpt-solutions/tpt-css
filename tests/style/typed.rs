// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Typed `box-shadow`, `transform`, and the time values of the transition and
//! animation durations and delays (CSS Backgrounds 3, CSS Transforms 2, CSS
//! Values 4). `angle` and `time` are the new value kinds.

use super::common::{style_of, style_with};
use super::{chain, parse_value};
use tpt_css::properties::registry::{lookup, spec};
use tpt_css::values::serialize::serialize_value;
use tpt_css::values::{AngleUnit, Color, LengthUnit, Shadow, TimeUnit, TransformFunction, Value};

/// The computed value of `prop` on the child of `css`.
fn computed(css: &str, prop: &str) -> Value {
    let (t, _, _, child) = chain();
    let map = style_with(css, &t);
    style_of(&map, child).raw(lookup(prop).unwrap()).clone()
}

fn px(v: f64) -> Value {
    Value::Length {
        value: v,
        unit: LengthUnit::Px,
    }
}

fn time(v: f64, unit: TimeUnit) -> Value {
    Value::Time { value: v, unit }
}

fn shadow(x: f64, y: f64) -> Shadow {
    Shadow {
        inset: false,
        x: px(x),
        y: px(y),
        blur: None,
        spread: None,
        color: None,
    }
}

// ---- time values ----------------------------------------------------------------

#[test]
fn time_lists_keep_the_unit_they_were_written_in() {
    let v = parse_value("transition-duration", "1s, 250ms").unwrap();
    assert_eq!(
        v,
        Value::List(vec![time(1.0, TimeUnit::S), time(250.0, TimeUnit::Ms)])
    );
    assert_eq!(serialize_value(&v), "1s, 250ms");
}

#[test]
fn computed_times_are_in_seconds() {
    assert_eq!(
        computed(
            "child { transition-duration: 250ms, 2s }",
            "transition-duration"
        ),
        Value::List(vec![time(0.25, TimeUnit::S), time(2.0, TimeUnit::S)])
    );
}

#[test]
fn durations_reject_negative_times_and_delays_accept_them() {
    assert_eq!(parse_value("animation-duration", "-1s"), None);
    assert_eq!(parse_value("transition-duration", "1s, -1ms"), None);
    assert_eq!(
        parse_value("animation-delay", "-1s"),
        Some(Value::List(vec![time(-1.0, TimeUnit::S)]))
    );
    assert_eq!(
        parse_value("transition-delay", "-250ms"),
        Some(Value::List(vec![time(-250.0, TimeUnit::Ms)]))
    );
}

#[test]
fn a_time_needs_s_or_ms_and_a_list_needs_items() {
    assert_eq!(parse_value("transition-duration", "0"), None);
    assert_eq!(parse_value("transition-duration", "1"), None);
    assert_eq!(parse_value("transition-duration", "1px"), None);
    assert_eq!(parse_value("transition-duration", "1s,"), None);
    assert_eq!(parse_value("transition-duration", "1s 2s"), None);
}

#[test]
fn the_time_properties_start_at_zero_seconds() {
    assert_eq!(
        spec(lookup("transition-delay").unwrap()).initial.to_value(),
        Value::List(vec![time(0.0, TimeUnit::S)])
    );
    assert_eq!(
        spec(lookup("animation-duration").unwrap())
            .initial
            .to_value(),
        Value::List(vec![time(0.0, TimeUnit::S)])
    );
}

// ---- angles and transform -------------------------------------------------------

#[test]
fn angles_compute_to_degrees_in_every_unit() {
    let turn = computed("child { transform: rotate(0.25turn) }", "transform");
    assert_eq!(
        turn,
        Value::Transforms(vec![TransformFunction {
            name: "rotate",
            args: vec![Value::Angle {
                value: 90.0,
                unit: AngleUnit::Deg
            }],
        }])
    );
    let grad = computed("child { transform: rotate(100grad) }", "transform");
    assert_eq!(serialize_value(&grad), "rotate(90deg)");
    let rad = computed(
        "child { transform: rotate(3.141592653589793rad) }",
        "transform",
    );
    let Value::Transforms(list) = rad else {
        panic!("expected transforms")
    };
    let Value::Angle { value, .. } = list[0].args[0] else {
        panic!("expected an angle")
    };
    assert!((value - 180.0).abs() < 1e-9, "{value}");
}

#[test]
fn a_bare_zero_is_not_an_angle() {
    assert_eq!(parse_value("transform", "rotate(0)"), None);
}

#[test]
fn transform_lists_parse_in_order_and_serialize_canonically() {
    let v = parse_value("transform", "translate(10px, 20%) scale(2) rotate(45deg)").unwrap();
    assert_eq!(
        serialize_value(&v),
        "translate(10px, 20%) scale(2) rotate(45deg)"
    );
    let Value::Transforms(list) = v else {
        panic!("expected transforms")
    };
    assert_eq!(
        list.iter().map(|f| f.name).collect::<Vec<_>>(),
        vec!["translate", "scale", "rotate"]
    );
}

#[test]
fn transform_functions_check_their_argument_kinds_and_counts() {
    // Wrong kinds.
    assert_eq!(parse_value("transform", "scale(1px)"), None);
    assert_eq!(parse_value("transform", "rotate(10px)"), None);
    assert_eq!(parse_value("transform", "translateZ(10%)"), None);
    // Wrong counts.
    assert_eq!(parse_value("transform", "translate()"), None);
    assert_eq!(parse_value("transform", "matrix(1, 0, 0, 1, 0)"), None);
    assert_eq!(parse_value("transform", "rotate3d(1, 0, 0)"), None);
    // Unknown functions, bare values, and an empty list.
    assert_eq!(parse_value("transform", "wobble(1)"), None);
    assert_eq!(parse_value("transform", "translate(10px) 5"), None);
    assert_eq!(parse_value("transform", ""), None);
}

#[test]
fn transform_none_and_the_initial_value() {
    assert_eq!(
        parse_value("transform", "none"),
        Some(Value::Keyword("none"))
    );
    assert_eq!(
        spec(lookup("transform").unwrap()).initial.to_value(),
        Value::Keyword("none")
    );
}

#[test]
fn transform_lengths_compute_to_px_with_the_element_font_size() {
    let v = computed(
        "child { font-size: 10px; transform: translateX(2em) }",
        "transform",
    );
    assert_eq!(
        v,
        Value::Transforms(vec![TransformFunction {
            name: "translateX",
            args: vec![px(20.0)],
        }])
    );
}

// ---- box-shadow -----------------------------------------------------------------

#[test]
fn a_shadow_keeps_its_parts_and_defaults_the_rest() {
    assert_eq!(
        parse_value("box-shadow", "inset 0 2px 4px red"),
        Some(Value::Shadows(vec![Shadow {
            inset: true,
            x: px(0.0),
            y: px(2.0),
            blur: Some(px(4.0)),
            spread: None,
            color: Some(Value::Color(Color::rgb8(255, 0, 0))),
        }]))
    );
    assert_eq!(
        parse_value("box-shadow", "1px 2px"),
        Some(Value::Shadows(vec![shadow(1.0, 2.0)]))
    );
}

#[test]
fn shadow_parts_may_come_in_any_order_but_lengths_stay_together() {
    // The color may come before the lengths or after them.
    assert!(parse_value("box-shadow", "red 1px 2px").is_some());
    assert!(parse_value("box-shadow", "1px 2px red").is_some());
    assert!(parse_value("box-shadow", "1px 2px 3px 4px inset").is_some());
    // A color or inset between two lengths splits them, which is not allowed.
    assert_eq!(parse_value("box-shadow", "1px red 2px"), None);
    assert_eq!(parse_value("box-shadow", "1px 2px inset 3px"), None);
}

#[test]
fn shadow_lengths_are_range_checked() {
    assert_eq!(parse_value("box-shadow", "1px"), None);
    assert_eq!(parse_value("box-shadow", "1px 2px 3px 4px 5px"), None);
    // A blur cannot be negative. A spread can.
    assert_eq!(parse_value("box-shadow", "0 0 -2px"), None);
    assert!(parse_value("box-shadow", "0 0 0 -2px").is_some());
    // Percentages are not lengths here, and neither is a bare number.
    assert_eq!(parse_value("box-shadow", "50% 0"), None);
    assert_eq!(parse_value("box-shadow", "1 0"), None);
    // Two colors in one shadow.
    assert_eq!(parse_value("box-shadow", "0 0 red blue"), None);
}

#[test]
fn box_shadow_lists_and_none() {
    let v = parse_value("box-shadow", "0 0 1px red, inset 2px 2px blue").unwrap();
    let Value::Shadows(list) = &v else {
        panic!("expected shadows")
    };
    assert_eq!(list.len(), 2);
    assert_eq!(
        parse_value("box-shadow", "none"),
        Some(Value::Keyword("none"))
    );
    assert_eq!(
        spec(lookup("box-shadow").unwrap()).initial.to_value(),
        Value::Keyword("none")
    );
}

#[test]
fn box_shadow_serializes_its_parts_in_a_fixed_order() {
    let v = parse_value("box-shadow", "red inset 1px 2px 3px 4px").unwrap();
    assert_eq!(serialize_value(&v), "inset 1px 2px 3px 4px rgb(255, 0, 0)");
}

#[test]
fn box_shadow_computes_lengths_to_px_and_currentcolor_to_the_element_color() {
    let v = computed(
        "child { color: rgb(1, 2, 3); font-size: 10px; box-shadow: 1em 0 currentcolor }",
        "box-shadow",
    );
    assert_eq!(
        v,
        Value::Shadows(vec![Shadow {
            inset: false,
            x: px(10.0),
            y: px(0.0),
            blur: None,
            spread: None,
            color: Some(Value::Color(Color::rgb8(1, 2, 3))),
        }])
    );
}

// ---- font-variant sub-properties (CSS Fonts 4 section 6) ------------------------

#[test]
fn keyword_sets_come_out_in_canonical_order_and_reject_repeats() {
    // Written out of order, stored in the grammar's order.
    assert_eq!(
        parse_value("font-variant-numeric", "tabular-nums ordinal slashed-zero"),
        Some(Value::Keywords(vec![
            "ordinal",
            "slashed-zero",
            "tabular-nums"
        ]))
    );
    assert_eq!(
        parse_value("font-variant-numeric", "tabular-nums"),
        Some(Value::Keyword("tabular-nums"))
    );
    // A keyword may not repeat, and `normal` stands alone.
    assert_eq!(parse_value("font-variant-numeric", "ordinal ordinal"), None);
    assert_eq!(parse_value("font-variant-numeric", "normal ordinal"), None);
    assert_eq!(
        parse_value("font-variant-numeric", "normal"),
        Some(Value::Keyword("normal"))
    );
}

#[test]
fn exclusive_groups_allow_one_keyword_each() {
    // Lining and oldstyle figures exclude each other, but not the spacing group.
    assert_eq!(
        parse_value("font-variant-numeric", "lining-nums oldstyle-nums"),
        None
    );
    assert!(parse_value("font-variant-numeric", "lining-nums tabular-nums").is_some());
    assert_eq!(
        parse_value(
            "font-variant-ligatures",
            "common-ligatures no-common-ligatures"
        ),
        None
    );
    assert_eq!(
        parse_value("font-variant-ligatures", "none common-ligatures"),
        None
    );
    assert_eq!(
        parse_value("font-variant-ligatures", "none"),
        Some(Value::Keyword("none"))
    );
}

#[test]
fn position_and_emoji_are_single_keywords() {
    assert_eq!(
        parse_value("font-variant-position", "super"),
        Some(Value::Keyword("super"))
    );
    assert_eq!(parse_value("font-variant-position", "sub super"), None);
    assert_eq!(
        parse_value("font-variant-emoji", "emoji"),
        Some(Value::Keyword("emoji"))
    );
    assert_eq!(parse_value("font-variant-emoji", "color"), None);
}

#[test]
fn keyword_sets_serialize_in_canonical_order() {
    let v = parse_value("font-variant-east-asian", "ruby jis78").unwrap();
    assert_eq!(serialize_value(&v), "jis78 ruby");
}

#[test]
fn the_font_variant_shorthand_splits_keywords_over_its_longhands() {
    let longhands = |css: &str| {
        [
            "font-variant-caps",
            "font-variant-numeric",
            "font-variant-ligatures",
        ]
        .map(|p| longhands_of(css, &[p])[0].clone())
    };
    let [caps, numeric, ligatures] = longhands(
        "child { font-variant: small-caps tabular-nums slashed-zero no-discretionary-ligatures }",
    );
    assert_eq!(caps, Value::Keyword("small-caps"));
    assert_eq!(
        numeric,
        Value::Keywords(vec!["slashed-zero", "tabular-nums"])
    );
    assert_eq!(ligatures, Value::Keyword("no-discretionary-ligatures"));
}

#[test]
fn font_variant_none_sets_ligatures_none_and_the_rest_normal() {
    let [ligatures, numeric] = ["font-variant-ligatures", "font-variant-numeric"].map(|p| {
        longhands_of(
            "child { font-variant-numeric: tabular-nums; font-variant: none }",
            &[p],
        )[0]
        .clone()
    });
    assert_eq!(ligatures, Value::Keyword("none"));
    assert_eq!(numeric, Value::Keyword("normal"));
}

#[test]
fn the_font_shorthand_resets_every_font_variant_longhand() {
    let css = "child { font-variant-numeric: tabular-nums; font: 12px serif }";
    let v = longhands_of(css, &["font-variant-numeric"])[0].clone();
    assert_eq!(v, Value::Keyword("normal"));
}

fn longhands_of(css: &str, props: &[&str]) -> Vec<Value> {
    let (t, _, _, child) = chain();
    let map = style_with(css, &t);
    props
        .iter()
        .map(|p| style_of(&map, child).raw(lookup(p).unwrap()).clone())
        .collect()
}

#[test]
fn every_initial_value_serializes_to_text_that_parses_back_to_itself() {
    // A property's initial value must be expressible in its own grammar, so a
    // host that writes the initial value as text gets the same value back.
    use tpt_css::properties::registry::PROPERTIES;
    let mut failures = Vec::new();
    for spec in PROPERTIES {
        let initial = spec.initial.to_value();
        let text = serialize_value(&initial);
        if super::parse_value(spec.name, &text) != Some(initial.clone()) {
            failures.push(format!("{}: {text:?}", spec.name));
        }
    }
    assert!(
        failures.is_empty(),
        "initial values that do not round-trip:\n{}",
        failures.join("\n")
    );
}

#[test]
fn font_variant_alternates_takes_its_functions_once_each() {
    assert_eq!(
        parse_value(
            "font-variant-alternates",
            "stylistic(alt-one) historical-forms"
        ),
        Some(Value::Raw(
            "stylistic(alt-one) historical-forms".to_string()
        ))
    );
    assert_eq!(
        parse_value("font-variant-alternates", "character-variant(1 2)"),
        None,
        "character-variant takes commas"
    );
    assert_eq!(
        parse_value("font-variant-alternates", "character-variant(1, 2)"),
        Some(Value::Raw("character-variant(1, 2)".to_string()))
    );
    assert_eq!(
        parse_value("font-variant-alternates", "styleset(a, b) styleset(c)"),
        None,
        "each function at most once"
    );
    assert_eq!(parse_value("font-variant-alternates", "stylistic()"), None);
    assert_eq!(parse_value("font-variant-alternates", "wobble(a)"), None);
    assert_eq!(
        parse_value("font-variant-alternates", "normal"),
        Some(Value::Keyword("normal"))
    );
}

#[test]
fn the_font_variant_shorthand_takes_alternates_too() {
    let css = "child { font-variant: stylistic(alt-one) tabular-nums }";
    let alternates = longhands_of(css, &["font-variant-alternates"])[0].clone();
    assert_eq!(alternates, Value::Raw("stylistic(alt-one)".to_string()));
    let numeric = longhands_of(css, &["font-variant-numeric"])[0].clone();
    assert_eq!(numeric, Value::Keyword("tabular-nums"));
}

#[test]
fn flex_shorthand_expands_to_grow_shrink_and_basis() {
    let grow = |css: &str| longhands_of(css, &["flex-grow"])[0].clone();
    let shrink = |css: &str| longhands_of(css, &["flex-shrink"])[0].clone();
    let basis = |css: &str| longhands_of(css, &["flex-basis"])[0].clone();

    // One number: grow, shrink 1, and a `0%` basis.
    let css = "child { flex: 2 }";
    assert_eq!(grow(css), Value::Number(2.0));
    assert_eq!(shrink(css), Value::Number(1.0));
    assert_eq!(basis(css), Value::Percentage(0.0));

    // Two numbers and a length.
    let css = "child { flex: 2 3 10px }";
    assert_eq!(grow(css), Value::Number(2.0));
    assert_eq!(shrink(css), Value::Number(3.0));
    assert_eq!(basis(css), px(10.0));

    // A length alone: grow and shrink are 1.
    let css = "child { flex: 10px }";
    assert_eq!(grow(css), Value::Number(1.0));
    assert_eq!(shrink(css), Value::Number(1.0));
    assert_eq!(basis(css), px(10.0));

    // A unitless zero after two factors is a basis (CSS Flexbox 1 section 7.2).
    let css = "child { flex: 1 1 0 }";
    assert_eq!(basis(css), px(0.0));
    let css = "child { flex: 0 }";
    assert_eq!(grow(css), Value::Number(0.0));
    assert_eq!(basis(css), Value::Percentage(0.0));

    // Keywords.
    let css = "child { flex: none }";
    assert_eq!(grow(css), Value::Number(0.0));
    assert_eq!(shrink(css), Value::Number(0.0));
    assert_eq!(basis(css), Value::Keyword("auto"));
    let css = "child { flex: auto }";
    assert_eq!(grow(css), Value::Number(1.0));
    assert_eq!(shrink(css), Value::Number(1.0));
    assert_eq!(basis(css), Value::Keyword("auto"));
    let css = "child { flex: content }";
    assert_eq!(basis(css), Value::Keyword("content"));
}

#[test]
fn invalid_flex_is_dropped_and_leaves_the_initial_values() {
    // Three numbers, a negative factor, and two bases are all invalid.
    for css in [
        "child { flex: 1 2 3 }",
        "child { flex: -1 }",
        "child { flex: 1px 2px }",
        "child { flex: none 1 }",
    ] {
        assert_eq!(
            longhands_of(css, &["flex-grow"])[0],
            Value::Number(0.0),
            "{css}"
        );
        assert_eq!(
            longhands_of(css, &["flex-shrink"])[0],
            Value::Number(1.0),
            "{css}"
        );
    }
}

#[test]
fn flex_longhands_are_checked_on_their_own() {
    assert_eq!(parse_value("flex-grow", "2.5"), Some(Value::Number(2.5)));
    assert_eq!(parse_value("flex-grow", "-1"), None);
    assert_eq!(parse_value("flex-shrink", "0"), Some(Value::Number(0.0)));
    assert_eq!(
        parse_value("flex-basis", "auto"),
        Some(Value::Keyword("auto"))
    );
    assert_eq!(
        parse_value("flex-basis", "content"),
        Some(Value::Keyword("content"))
    );
    assert_eq!(parse_value("flex-basis", "-5px"), None);
}

#[test]
fn transition_shorthand_builds_one_entry_per_layer() {
    let css = "child { transition: opacity 0.2s ease-in, transform 1s linear 100ms }";
    assert_eq!(
        longhands_of(css, &["transition-property"])[0],
        Value::List(vec![
            Value::Raw("opacity".to_string()),
            Value::Raw("transform".to_string()),
        ])
    );
    assert_eq!(
        longhands_of(css, &["transition-duration"])[0],
        Value::List(vec![time(0.2, TimeUnit::S), time(1.0, TimeUnit::S)])
    );
    assert_eq!(
        longhands_of(css, &["transition-timing-function"])[0],
        Value::List(vec![Value::Keyword("ease-in"), Value::Keyword("linear")])
    );
    assert_eq!(
        longhands_of(css, &["transition-delay"])[0],
        Value::List(vec![time(0.0, TimeUnit::S), time(0.1, TimeUnit::S)])
    );
}

#[test]
fn transition_shorthand_defaults_and_the_order_of_times() {
    // Omitted parts take their initial values: `all`, `0s`, `ease`, `0s`.
    let css = "child { transition: all }";
    assert_eq!(
        longhands_of(css, &["transition-property"])[0],
        Value::List(vec![Value::Keyword("all")])
    );
    assert_eq!(
        longhands_of(css, &["transition-timing-function"])[0],
        Value::List(vec![Value::Keyword("ease")])
    );
    // The first time is the duration; the second is the delay.
    let css = "child { transition: 1s 2s }";
    assert_eq!(
        longhands_of(css, &["transition-duration"])[0],
        Value::List(vec![time(1.0, TimeUnit::S)])
    );
    assert_eq!(
        longhands_of(css, &["transition-delay"])[0],
        Value::List(vec![time(2.0, TimeUnit::S)])
    );
    // A property may follow the times.
    let css = "child { transition: 1s none }";
    assert_eq!(
        longhands_of(css, &["transition-duration"])[0],
        Value::List(vec![time(1.0, TimeUnit::S)])
    );
    // `none` is a property name here, and it is the whole layer.
    let css = "child { transition: none }";
    assert_eq!(
        longhands_of(css, &["transition-property"])[0],
        Value::List(vec![Value::Keyword("none")])
    );
}

#[test]
fn invalid_transition_is_dropped_and_leaves_the_initial_values() {
    for css in [
        "child { transition: -1s }",
        "child { transition: ease ease }",
        "child { transition: 1s 2s 3s }",
        "child { transition: opacity 1s, }",
    ] {
        assert_eq!(
            longhands_of(css, &["transition-duration"])[0],
            Value::List(vec![time(0.0, TimeUnit::S)]),
            "{css}"
        );
    }
}

#[test]
fn transition_property_lists_name_properties_or_all() {
    assert_eq!(
        parse_value("transition-property", "none"),
        Some(Value::List(vec![Value::Keyword("none")]))
    );
    assert_eq!(
        parse_value("transition-property", "all, Opacity"),
        Some(Value::List(vec![
            Value::Keyword("all"),
            Value::Raw("opacity".to_string()),
        ]))
    );
    // `none` and CSS-wide keywords cannot be names inside a list.
    assert_eq!(parse_value("transition-property", "none, opacity"), None);
    assert_eq!(parse_value("transition-property", "inherit"), None);
    assert_eq!(parse_value("transition-property", "1px"), None);
    assert_eq!(parse_value("transition-property", ""), None);
}

#[test]
fn transition_timing_functions_accept_keywords_bezier_and_steps() {
    assert_eq!(
        parse_value("transition-timing-function", "ease, linear"),
        Some(Value::List(vec![
            Value::Keyword("ease"),
            Value::Keyword("linear"),
        ]))
    );
    for good in [
        "cubic-bezier(0.1, 0.7, 1, 0.1)",
        "steps(4, jump-end)",
        "steps(1)",
        "steps(2, jump-none)",
    ] {
        assert!(
            matches!(
                parse_value("transition-timing-function", good),
                Some(Value::List(ref items)) if items.len() == 1 && matches!(items[0], Value::Raw(_))
            ),
            "{good} should parse"
        );
    }
    for bad in [
        "cubic-bezier(1.5, 0, 1, 1)",
        "cubic-bezier(0, 0, 1)",
        "steps(0)",
        "steps(1, jump-none)",
        "steps(2, sideways)",
        "bounce",
        "ease,",
    ] {
        assert_eq!(
            parse_value("transition-timing-function", bad),
            None,
            "{bad}"
        );
    }
}

#[test]
fn animation_shorthand_fills_all_eight_longhands() {
    let css = "child { animation: spin 2s linear 1s infinite alternate both paused }";
    let value = |prop: &str| longhands_of(css, &[prop])[0].clone();
    assert_eq!(
        value("animation-name"),
        Value::List(vec![Value::Raw("spin".to_string())])
    );
    assert_eq!(
        value("animation-duration"),
        Value::List(vec![time(2.0, TimeUnit::S)])
    );
    assert_eq!(
        value("animation-timing-function"),
        Value::List(vec![Value::Keyword("linear")])
    );
    assert_eq!(
        value("animation-delay"),
        Value::List(vec![time(1.0, TimeUnit::S)])
    );
    assert_eq!(
        value("animation-iteration-count"),
        Value::List(vec![Value::Keyword("infinite")])
    );
    assert_eq!(
        value("animation-direction"),
        Value::List(vec![Value::Keyword("alternate")])
    );
    assert_eq!(
        value("animation-fill-mode"),
        Value::List(vec![Value::Keyword("both")])
    );
    assert_eq!(
        value("animation-play-state"),
        Value::List(vec![Value::Keyword("paused")])
    );
}

#[test]
fn animation_shorthand_defaults_and_the_two_none_keywords() {
    // Omitted parts take their initial values.
    let css = "child { animation: fade }";
    assert_eq!(
        longhands_of(css, &["animation-iteration-count"])[0],
        Value::List(vec![Value::Number(1.0)])
    );
    assert_eq!(
        longhands_of(css, &["animation-fill-mode"])[0],
        Value::List(vec![Value::Keyword("none")])
    );
    // `none` is the name when no name is given, and the fill mode otherwise.
    let css = "child { animation: none }";
    assert_eq!(
        longhands_of(css, &["animation-name"])[0],
        Value::List(vec![Value::Keyword("none")])
    );
    let css = "child { animation: none forwards }";
    assert_eq!(
        longhands_of(css, &["animation-name"])[0],
        Value::List(vec![Value::Keyword("none")])
    );
    assert_eq!(
        longhands_of(css, &["animation-fill-mode"])[0],
        Value::List(vec![Value::Keyword("forwards")])
    );
    let css = "child { animation: forwards none }";
    assert_eq!(
        longhands_of(css, &["animation-name"])[0],
        Value::List(vec![Value::Keyword("none")])
    );
}

#[test]
fn animation_shorthand_layers_and_the_time_order() {
    let css = "child { animation: Spin 1s, fade 2s 3 }";
    assert_eq!(
        longhands_of(css, &["animation-name"])[0],
        Value::List(vec![
            Value::Raw("Spin".to_string()),
            Value::Raw("fade".to_string()),
        ]),
        "animation names keep their case"
    );
    assert_eq!(
        longhands_of(css, &["animation-duration"])[0],
        Value::List(vec![time(1.0, TimeUnit::S), time(2.0, TimeUnit::S)])
    );
    assert_eq!(
        longhands_of(css, &["animation-iteration-count"])[0],
        Value::List(vec![Value::Number(1.0), Value::Number(3.0)])
    );
    let css = "child { animation: fade 1s 2s }";
    assert_eq!(
        longhands_of(css, &["animation-delay"])[0],
        Value::List(vec![time(2.0, TimeUnit::S)])
    );
}

#[test]
fn invalid_animation_is_dropped_and_leaves_the_initial_values() {
    for css in [
        "child { animation: ease ease }",
        "child { animation: a b }",
        "child { animation: 1s 2s 3s }",
        "child { animation: a, }",
        "child { animation: -1s }",
        "child { animation: 1s, forwards both }",
    ] {
        assert_eq!(
            longhands_of(css, &["animation-name"])[0],
            Value::List(vec![Value::Keyword("none")]),
            "{css}"
        );
        assert_eq!(
            longhands_of(css, &["animation-duration"])[0],
            Value::List(vec![time(0.0, TimeUnit::S)]),
            "{css}"
        );
    }
}

#[test]
fn animation_longhand_lists_are_checked() {
    assert_eq!(
        parse_value("animation-name", "Spin, fade"),
        Some(Value::List(vec![
            Value::Raw("Spin".to_string()),
            Value::Raw("fade".to_string()),
        ]))
    );
    assert_eq!(parse_value("animation-name", "none, fade"), None);
    assert_eq!(parse_value("animation-name", "inherit"), None);
    assert_eq!(
        parse_value("animation-iteration-count", "infinite, 2.5, 0"),
        Some(Value::List(vec![
            Value::Keyword("infinite"),
            Value::Number(2.5),
            Value::Number(0.0),
        ]))
    );
    assert_eq!(parse_value("animation-iteration-count", "-1"), None);
    assert_eq!(parse_value("animation-iteration-count", "many"), None);
    assert_eq!(
        parse_value("animation-direction", "alternate-reverse"),
        Some(Value::List(vec![Value::Keyword("alternate-reverse")]))
    );
    assert_eq!(parse_value("animation-direction", "sideways"), None);
    assert_eq!(
        parse_value("animation-fill-mode", "none, both"),
        Some(Value::List(vec![
            Value::Keyword("none"),
            Value::Keyword("both"),
        ]))
    );
    assert_eq!(parse_value("animation-fill-mode", "forward"), None);
    assert_eq!(
        parse_value("animation-play-state", "paused"),
        Some(Value::List(vec![Value::Keyword("paused")]))
    );
    assert_eq!(parse_value("animation-play-state", "stopped"), None);
    assert!(matches!(
        parse_value("animation-timing-function", "steps(2, end)"),
        Some(Value::List(ref items)) if items.len() == 1
    ));
}

#[test]
fn filter_list_takes_filter_functions_with_their_defaults() {
    let value = |css: &str| longhands_of(css, &["filter"])[0].clone();
    let func = |name: &'static str, args: Vec<Value>| TransformFunction { name, args };
    assert_eq!(value("child { filter: none }"), Value::Keyword("none"));
    assert_eq!(
        value("child { filter: blur(2px) brightness(50%) }"),
        Value::Transforms(vec![
            func("blur", vec![px(2.0)]),
            func("brightness", vec![Value::Number(0.5)]),
        ])
    );
    // Omitted amounts take their defaults: 0 for blur, 1 for the amounts, 0deg.
    assert_eq!(
        value("child { filter: blur() grayscale() hue-rotate() }"),
        Value::Transforms(vec![
            func("blur", vec![px(0.0)]),
            func("grayscale", vec![Value::Number(1.0)]),
            func(
                "hue-rotate",
                vec![Value::Angle {
                    value: 0.0,
                    unit: AngleUnit::Deg,
                }]
            ),
        ])
    );
    // The color may come before or after the lengths.
    let red = parse_value("color", "red").unwrap();
    assert_eq!(
        value("child { filter: drop-shadow(red 2px 4px) }"),
        Value::Transforms(vec![func(
            "drop-shadow",
            vec![px(2.0), px(4.0), red.clone()]
        )])
    );
    assert_eq!(
        value("child { filter: drop-shadow(2px 4px 1px red) }"),
        Value::Transforms(vec![func(
            "drop-shadow",
            vec![px(2.0), px(4.0), px(1.0), red]
        )])
    );
}

#[test]
fn filter_list_rejects_bad_arguments_and_unknown_functions() {
    let list = |v: &str| parse_value("filter", v);
    assert_eq!(list("blur(-1px)"), None, "a blur is never negative");
    assert_eq!(list("brightness(-1)"), None);
    assert_eq!(list("hue-rotate(1px)"), None, "an angle, not a length");
    assert_eq!(list("drop-shadow(1px)"), None, "two offsets at least");
    assert_eq!(list("drop-shadow(1px 2px -3px)"), None, "a negative blur");
    assert_eq!(list("drop-shadow(red blue 1px 2px)"), None, "one color");
    assert_eq!(list("wobble(1)"), None);
    assert_eq!(
        list("blur(1px), grayscale()"),
        None,
        "filters are space-separated"
    );
    assert_eq!(list(""), None);
}

#[test]
fn filter_list_with_a_url_is_kept_as_text() {
    let value = longhands_of("child { filter: url(a.svg) blur(2px) }", &["filter"])[0].clone();
    assert!(
        matches!(&value, Value::Raw(text) if text.contains("blur(2px)")),
        "{value:?}"
    );
}

/// The computed value of `prop` on a child that has `attrs`, under `css`.
fn with_attrs(css: &str, attrs: &[(&str, &str)], prop: &str) -> Value {
    let (mut t, _, _, child) = chain();
    for (name, value) in attrs {
        t.set_attr(child, name, value);
    }
    let map = style_with(css, &t);
    style_of(&map, child).raw(lookup(prop).unwrap()).clone()
}

#[test]
fn typed_attr_reads_a_number_with_a_unit_in_any_property() {
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w px) }",
            &[("data-w", "10")],
            "width"
        ),
        px(10.0)
    );
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w px) }",
            &[("data-w", "1.5")],
            "width"
        ),
        px(1.5)
    );
    // A unit that the value already has is not a number, so it is invalid.
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w px) }",
            &[("data-w", "10px")],
            "width"
        ),
        Value::Keyword("auto")
    );
}

#[test]
fn typed_attr_takes_its_fallback_when_the_attribute_is_missing_or_wrong() {
    assert_eq!(
        with_attrs("child { width: attr(data-w px, 5px) }", &[], "width"),
        px(5.0)
    );
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w px, 5px) }",
            &[("data-w", "wide")],
            "width"
        ),
        px(5.0)
    );
    // Without a fallback, a value of the wrong type makes the declaration invalid.
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w px) }",
            &[("data-w", "wide")],
            "width"
        ),
        Value::Keyword("auto")
    );
}

#[test]
fn typed_attr_parses_the_syntax_it_names() {
    let red = parse_value("color", "red").unwrap();
    assert_eq!(
        with_attrs(
            "child { color: attr(data-c type(<color>)) }",
            &[("data-c", "red")],
            "color"
        ),
        red
    );
    assert_eq!(
        with_attrs(
            "child { opacity: attr(data-o type(<number>)) }",
            &[("data-o", "0.5")],
            "opacity"
        ),
        Value::Number(0.5)
    );
    // `0.5` is not an integer, so the fallback applies.
    assert_eq!(
        with_attrs(
            "child { opacity: attr(data-o type(<integer>), 1) }",
            &[("data-o", "0.5")],
            "opacity"
        ),
        Value::Number(1.0)
    );
    // A unitless zero is a valid <length>.
    assert_eq!(
        with_attrs(
            "child { margin-left: attr(data-m type(<length>)) }",
            &[("data-m", "0")],
            "margin-left"
        ),
        px(0.0)
    );
    // `3s` is a valid <time>, so the fallback does not apply. It is not a length,
    // so the declaration is invalid at computed-value time and margin keeps 0px.
    assert_eq!(
        with_attrs(
            "child { margin-left: attr(data-m type(<time>), 2px) }",
            &[("data-m", "3s")],
            "margin-left"
        ),
        px(0.0)
    );
}

#[test]
fn typed_attr_accepts_an_angle_only_when_it_has_an_angle_unit() {
    assert_eq!(
        with_attrs(
            "child { margin-left: attr(data-a type(<angle>), 2px) }",
            &[("data-a", "1turn")],
            "margin-left"
        ),
        Value::Length {
            value: 0.0,
            unit: LengthUnit::Px,
        },
        "a turn is an angle, but not a length, so margin stays at its initial value"
    );
    // A bare number is not an angle, so the fallback applies.
    assert_eq!(
        with_attrs(
            "child { margin-left: attr(data-a type(<angle>), 2px) }",
            &[("data-a", "45")],
            "margin-left"
        ),
        px(2.0)
    );
}

#[test]
fn typed_attr_in_content_gives_the_attribute_text_as_a_string() {
    assert_eq!(
        with_attrs(
            "child { content: \"#\" attr(data-n type(<integer>)) }",
            &[("data-n", "3")],
            "content"
        ),
        Value::Strings(vec!["#".to_string(), "3".to_string()])
    );
}

#[test]
fn unknown_attr_types_and_plain_attr_in_other_properties_are_invalid() {
    // A plain `attr()` is a string, which is not a width.
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w) }",
            &[("data-w", "10")],
            "width"
        ),
        Value::Keyword("auto")
    );
    // Unknown syntaxes and units are not accepted.
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w type(<wobble>)) }",
            &[("data-w", "10")],
            "width"
        ),
        Value::Keyword("auto")
    );
    assert_eq!(
        with_attrs(
            "child { width: attr(data-w furlong) }",
            &[("data-w", "10")],
            "width"
        ),
        Value::Keyword("auto")
    );
}

#[test]
fn background_layers_fill_each_longhand_per_layer() {
    let css = "child { background: url(a.png) left 10px top / cover no-repeat fixed \
               content-box border-box, url(b.png) center repeat-x padding-box, red }";
    let one = |prop: &str| longhands_of(css, &[prop])[0].clone();
    assert_eq!(
        one("background-color"),
        parse_value("color", "red").unwrap()
    );
    assert_eq!(
        one("background-image"),
        Value::Raw("url(\"a.png\"), url(\"b.png\"), none".to_string())
    );
    assert_eq!(
        one("background-position"),
        Value::List(vec![
            Value::Raw("left 10px top".to_string()),
            Value::Raw("center".to_string()),
            Value::Raw("0% 0%".to_string()),
        ])
    );
    assert_eq!(
        one("background-size"),
        Value::List(vec![
            Value::Keyword("cover"),
            Value::Keyword("auto"),
            Value::Keyword("auto"),
        ])
    );
    assert_eq!(
        one("background-repeat"),
        Value::List(vec![
            Value::Keyword("no-repeat"),
            Value::Keyword("repeat-x"),
            Value::Keyword("repeat"),
        ])
    );
    assert_eq!(
        one("background-attachment"),
        Value::List(vec![
            Value::Keyword("fixed"),
            Value::Keyword("scroll"),
            Value::Keyword("scroll"),
        ])
    );
    // A pair of boxes sets origin then clip. One box sets both.
    assert_eq!(
        one("background-origin"),
        Value::List(vec![
            Value::Keyword("content-box"),
            Value::Keyword("padding-box"),
            Value::Keyword("padding-box"),
        ])
    );
    assert_eq!(
        one("background-clip"),
        Value::List(vec![
            Value::Keyword("border-box"),
            Value::Keyword("padding-box"),
            Value::Keyword("border-box"),
        ])
    );
}

#[test]
fn background_repeat_pairs_and_sizes_are_kept_as_text() {
    assert_eq!(
        longhands_of(
            "child { background: url(a.png) repeat space }",
            &["background-repeat"]
        )[0],
        Value::List(vec![Value::Raw("repeat space".to_string())])
    );
    assert_eq!(
        longhands_of(
            "child { background: left / 10px auto }",
            &["background-size"]
        )[0],
        Value::List(vec![Value::Raw("10px auto".to_string())])
    );
    assert_eq!(
        longhands_of("child { background: none }", &["background-position"])[0],
        Value::List(vec![Value::Raw("0% 0%".to_string())]),
        "an omitted position is 0% 0%"
    );
}

#[test]
fn invalid_background_layers_are_dropped() {
    for css in [
        "child { background: left right }",
        "child { background: / cover }",
        "child { background: left / cover / auto }",
        "child { background: left / 10px 20px 30px }",
        "child { background: red, blue }",
        "child { background: url(a.png) red url(b.png) }",
        "child { background: url(a.png) repeat repeat repeat }",
        "child { background: left / cover left }",
    ] {
        assert_eq!(
            longhands_of(css, &["background-color"])[0],
            Value::Color(tpt_css::values::Color::TRANSPARENT),
            "{css}"
        );
        assert_eq!(
            longhands_of(css, &["background-position"])[0],
            Value::List(vec![Value::Raw("0% 0%".to_string())]),
            "{css}"
        );
    }
}

#[test]
fn background_position_takes_keywords_offsets_and_edges() {
    let pos = |v: &str| parse_value("background-position", v);
    assert_eq!(
        pos("top left"),
        Some(Value::List(vec![Value::Raw("top left".to_string())]))
    );
    assert_eq!(
        pos("right 10px bottom 20px"),
        Some(Value::List(vec![Value::Raw(
            "right 10px bottom 20px".to_string()
        )]))
    );
    assert_eq!(
        pos("left top 10px"),
        Some(Value::List(vec![Value::Raw("left top 10px".to_string())]))
    );
    assert_eq!(pos("top bottom"), None, "two vertical keywords");
    assert_eq!(pos("left right"), None, "two horizontal keywords");
    assert_eq!(
        pos("top 10px"),
        None,
        "a vertical keyword cannot take an offset first"
    );
    assert_eq!(pos("10px 20px 30px"), None, "three offsets");
}

#[test]
fn background_size_is_one_keyword_or_one_or_two_sizes() {
    let size = |v: &str| parse_value("background-size", v);
    assert_eq!(
        size("cover"),
        Some(Value::List(vec![Value::Keyword("cover")]))
    );
    assert_eq!(size("contain auto"), None, "a keyword stands alone");
    assert_eq!(
        size("10px auto"),
        Some(Value::List(vec![Value::Raw("10px auto".to_string())]))
    );
    assert_eq!(size("-1px"), None, "a size is not negative");
}

#[test]
fn background_repeat_and_attachment_words() {
    let repeat = |v: &str| parse_value("background-repeat", v);
    assert_eq!(
        repeat("no-repeat"),
        Some(Value::List(vec![Value::Keyword("no-repeat")]))
    );
    assert_eq!(
        repeat("round space"),
        Some(Value::List(vec![Value::Raw("round space".to_string())]))
    );
    assert_eq!(
        repeat("repeat-x repeat"),
        None,
        "a pair cannot use repeat-x"
    );
    assert_eq!(
        parse_value("background-attachment", "local"),
        Some(Value::List(vec![Value::Keyword("local")]))
    );
    assert_eq!(parse_value("background-attachment", "sticky"), None);
}

#[test]
fn typed_attr_reads_a_url_as_a_quoted_image() {
    assert_eq!(
        with_attrs(
            "child { background-image: attr(data-src url) }",
            &[("data-src", "a b.png")],
            "background-image"
        ),
        Value::Raw("url(\"a b.png\")".to_string())
    );
}

#[test]
fn every_keyword_property_accepts_its_keywords_in_any_case_and_no_others() {
    // Generic coverage for every property with a keyword grammar, including those
    // that no other test names. Each keyword is its own value, in any letter case,
    // and a word outside the list is invalid.
    use tpt_css::properties::registry::PROPERTIES;
    use tpt_css::properties::Grammar;
    let mut checked = 0;
    for prop in PROPERTIES {
        let Grammar::Keywords(list) = prop.grammar else {
            continue;
        };
        for &keyword in list {
            checked += 1;
            assert_eq!(
                parse_value(prop.name, keyword),
                Some(Value::Keyword(keyword)),
                "{}: {keyword}",
                prop.name
            );
            assert_eq!(
                parse_value(prop.name, &keyword.to_ascii_uppercase()),
                Some(Value::Keyword(keyword)),
                "{}: {keyword} in capitals",
                prop.name
            );
        }
        assert_eq!(
            parse_value(prop.name, "not-a-keyword-anywhere"),
            None,
            "{}",
            prop.name
        );
    }
    assert!(checked > 100, "only {checked} keywords were checked");
}

#[test]
fn min_and_max_height_take_non_negative_lengths_and_their_keyword() {
    assert_eq!(parse_value("min-height", "12px"), Some(Value::px(12.0)));
    assert_eq!(
        parse_value("min-height", "50%"),
        Some(Value::Percentage(50.0))
    );
    assert_eq!(parse_value("min-height", "-1px"), None);
    assert_eq!(
        parse_value("min-height", "none"),
        None,
        "min-height has no none"
    );
    assert_eq!(
        parse_value("max-height", "none"),
        Some(Value::Keyword("none"))
    );
    assert_eq!(parse_value("max-height", "-4px"), None);
    assert_eq!(
        parse_value("max-height", "auto"),
        None,
        "max-height has no auto"
    );
}
