// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Media query tests (spec 16.3): parsing, evaluation against the environment,
//! nesting, and rule selection.

use super::common::{style_of, style_with_env, TestTree};
use tpt_css::properties::lookup;
use tpt_css::tree::NodeKind;
use tpt_css::values::Value;
use tpt_css::{Environment, MediaEnvironment, MediaKind, ViewportInfo};

fn chain() -> (TestTree, tpt_css::tree::NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    (t, child)
}

fn margin_left(css: &str, env: &Environment) -> Value {
    let (t, child) = chain();
    let map = style_with_env(css, &t, env);
    style_of(&map, child)
        .raw(lookup("margin-left").expect("known property"))
        .clone()
}

fn screen(width: f64, height: f64) -> Environment {
    Environment {
        viewport: ViewportInfo { width, height },
        ..Environment::default()
    }
}

fn print(width: f64, height: f64) -> Environment {
    Environment {
        viewport: ViewportInfo { width, height },
        media: MediaEnvironment {
            kind: MediaKind::Print,
            ..MediaEnvironment::default()
        },
        ..Environment::default()
    }
}

const APPLIES: &str = "child { margin-left: 3px }";

#[test]
fn min_width_applies_when_the_viewport_is_wide_enough() {
    let css = format!("@media (min-width: 500px) {{ {APPLIES} }}");
    assert_eq!(margin_left(&css, &screen(800.0, 600.0)), Value::px(3.0));
    assert_eq!(margin_left(&css, &screen(400.0, 600.0)), Value::px(0.0));
}

#[test]
fn max_width_and_exact_width() {
    let css = format!("@media (max-width: 500px) {{ {APPLIES} }}");
    assert_eq!(margin_left(&css, &screen(400.0, 600.0)), Value::px(3.0));
    assert_eq!(margin_left(&css, &screen(800.0, 600.0)), Value::px(0.0));
    let exact = format!("@media (width: 800px) {{ {APPLIES} }}");
    assert_eq!(margin_left(&exact, &screen(800.0, 600.0)), Value::px(3.0));
}

#[test]
fn media_types_screen_and_print() {
    let print_only = format!("@media print {{ {APPLIES} }}");
    assert_eq!(
        margin_left(&print_only, &screen(800.0, 600.0)),
        Value::px(0.0)
    );
    assert_eq!(
        margin_left(&print_only, &print(800.0, 600.0)),
        Value::px(3.0)
    );
    let screen_only = format!("@media screen {{ {APPLIES} }}");
    assert_eq!(
        margin_left(&screen_only, &screen(800.0, 600.0)),
        Value::px(3.0)
    );
}

#[test]
fn conditions_combine_with_and_and_lists_with_comma() {
    let both =
        format!("@media screen and (min-width: 100px) and (max-width: 900px) {{ {APPLIES} }}");
    assert_eq!(margin_left(&both, &screen(800.0, 600.0)), Value::px(3.0));
    assert_eq!(margin_left(&both, &screen(1000.0, 600.0)), Value::px(0.0));
    let list = format!("@media print, (min-width: 100px) {{ {APPLIES} }}");
    assert_eq!(margin_left(&list, &screen(800.0, 600.0)), Value::px(3.0));
}

#[test]
fn not_inverts_the_whole_query() {
    let css = format!("@media not print {{ {APPLIES} }}");
    assert_eq!(margin_left(&css, &screen(800.0, 600.0)), Value::px(3.0));
    assert_eq!(margin_left(&css, &print(800.0, 600.0)), Value::px(0.0));
}

#[test]
fn nested_media_rules_require_every_level() {
    let css = format!("@media (min-width: 100px) {{ @media (max-width: 200px) {{ {APPLIES} }} }}");
    assert_eq!(margin_left(&css, &screen(800.0, 600.0)), Value::px(0.0));
    assert_eq!(margin_left(&css, &screen(150.0, 600.0)), Value::px(3.0));
}

#[test]
fn em_lengths_in_media_queries_use_the_root_font_size() {
    // 50em with a 16px root is 800px.
    let css = format!("@media (min-width: 50em) {{ {APPLIES} }}");
    assert_eq!(margin_left(&css, &screen(800.0, 600.0)), Value::px(3.0));
    assert_eq!(margin_left(&css, &screen(799.0, 600.0)), Value::px(0.0));
}

#[test]
fn orientation_and_aspect_ratio() {
    let landscape = format!("@media (orientation: landscape) {{ {APPLIES} }}");
    assert_eq!(
        margin_left(&landscape, &screen(800.0, 600.0)),
        Value::px(3.0)
    );
    assert_eq!(
        margin_left(&landscape, &screen(600.0, 800.0)),
        Value::px(0.0)
    );
    let wide = format!("@media (min-aspect-ratio: 4/3) {{ {APPLIES} }}");
    assert_eq!(margin_left(&wide, &screen(800.0, 600.0)), Value::px(3.0));
    assert_eq!(margin_left(&wide, &screen(600.0, 600.0)), Value::px(0.0));
}

#[test]
fn user_preference_features() {
    let dark = Environment {
        media: MediaEnvironment {
            prefers_dark: true,
            ..MediaEnvironment::default()
        },
        ..Environment::default()
    };
    let css = format!("@media (prefers-color-scheme: dark) {{ {APPLIES} }}");
    assert_eq!(margin_left(&css, &dark), Value::px(3.0));
    assert_eq!(margin_left(&css, &Environment::default()), Value::px(0.0));

    let reduced = Environment {
        media: MediaEnvironment {
            reduced_motion: true,
            ..MediaEnvironment::default()
        },
        ..Environment::default()
    };
    let motion = format!("@media (prefers-reduced-motion: reduce) {{ {APPLIES} }}");
    assert_eq!(margin_left(&motion, &reduced), Value::px(3.0));
}

#[test]
fn hover_and_pointer_features() {
    let no_hover = Environment {
        media: MediaEnvironment {
            hover: false,
            pointer: tpt_css::ast::PointerKind::Coarse,
            ..MediaEnvironment::default()
        },
        ..Environment::default()
    };
    let hover = format!("@media (hover: none) {{ {APPLIES} }}");
    assert_eq!(margin_left(&hover, &no_hover), Value::px(3.0));
    let coarse = format!("@media (pointer: coarse) {{ {APPLIES} }}");
    assert_eq!(margin_left(&coarse, &no_hover), Value::px(3.0));
}

#[test]
fn unknown_features_never_match() {
    let css = format!("@media (not-a-feature: 1) {{ {APPLIES} }}");
    assert_eq!(margin_left(&css, &screen(800.0, 600.0)), Value::px(0.0));
    let negated = format!("@media not (not-a-feature: 1) {{ {APPLIES} }}");
    assert_eq!(margin_left(&negated, &screen(800.0, 600.0)), Value::px(3.0));
}

#[test]
fn a_malformed_media_prelude_drops_the_rule_with_a_diagnostic() {
    let parsed = tpt_css::parse_stylesheet(
        "@media screen (min-width: 1px) { child { margin-left: 3px } }",
        tpt_css::ParseOptions::default(),
    );
    assert!(parsed.stylesheet.rules.is_empty());
    assert!(!parsed.diagnostics.is_empty());
}

#[test]
fn a_media_rule_is_an_ordinary_rule_for_cascade_order() {
    // The media rule comes later in the sheet, so it wins over the earlier rule.
    let css = format!("child {{ margin-left: 1px }} @media screen {{ {APPLIES} }}");
    assert_eq!(margin_left(&css, &screen(800.0, 600.0)), Value::px(3.0));
}
