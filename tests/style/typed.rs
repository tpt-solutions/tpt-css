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
