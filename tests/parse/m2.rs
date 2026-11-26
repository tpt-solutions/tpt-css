// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Parse-level tests for the M2 at-rules and nesting shapes.

use tpt_css::ast::Rule;
use tpt_css::{parse_stylesheet, ParseOptions};

fn rules(css: &str) -> Vec<Rule> {
    parse_stylesheet(css, ParseOptions::default())
        .stylesheet
        .rules
}

#[test]
fn a_layer_statement_names_its_layers_in_order() {
    match &rules("@layer reset, base.theme;")[0] {
        Rule::Layer(l) => {
            assert_eq!(l.names, vec!["reset".to_string(), "base.theme".to_string()]);
            assert!(l.rules.is_none());
        }
        other => panic!("expected a layer, got {other:?}"),
    }
}

#[test]
fn an_anonymous_layer_block_has_an_empty_name() {
    match &rules("@layer { p { color: red } }")[0] {
        Rule::Layer(l) => assert_eq!(l.names, vec![String::new()]),
        other => panic!("expected a layer, got {other:?}"),
    }
}

#[test]
fn a_nested_style_rule_becomes_a_sibling_after_its_parent() {
    let rs = rules("a { color: red; b { margin: 0 } }");
    assert_eq!(rs.len(), 2);
    assert!(matches!(rs[0], Rule::Style(_)));
    assert!(matches!(rs[1], Rule::Style(_)));
}

#[test]
fn a_container_rule_keeps_its_name_and_features() {
    match &rules("@container card (min-width: 20em) { p { margin: 0 } }")[0] {
        Rule::Container(c) => {
            assert_eq!(c.name.as_deref(), Some("card"));
            assert_eq!(c.features.len(), 1);
        }
        other => panic!("expected a container rule, got {other:?}"),
    }
}

#[test]
fn a_container_rule_needs_a_feature() {
    assert!(rules("@container card { p { margin: 0 } }").is_empty());
}

#[test]
fn supports_records_its_evaluated_condition() {
    match &rules("@supports (display: grid) { p { margin: 0 } }")[0] {
        Rule::Supports(s) => assert!(s.supported),
        other => panic!("expected supports, got {other:?}"),
    }
    match &rules("@supports (display: nope) { p { margin: 0 } }")[0] {
        Rule::Supports(s) => assert!(!s.supported),
        other => panic!("expected supports, got {other:?}"),
    }
}

#[test]
fn a_property_rule_records_syntax_inherits_and_initial_value() {
    match &rules("@property --size { syntax: '<length>'; inherits: false; initial-value: 4px }")[0]
    {
        Rule::Property(p) => {
            assert_eq!(p.name, "--size");
            assert_eq!(p.syntax.as_deref(), Some("<length>"));
            assert!(!p.inherits);
            assert!(p.initial.is_some());
        }
        other => panic!("expected a property rule, got {other:?}"),
    }
}

#[test]
fn keyframes_and_font_face_are_parsed_not_dropped() {
    assert!(matches!(
        rules("@keyframes k { to { margin: 0 } }")[0],
        Rule::Keyframes(_)
    ));
    assert!(matches!(
        rules("@font-face { font-family: F }")[0],
        Rule::FontFace(_)
    ));
}

#[test]
fn ampersand_is_a_nesting_selector_only_inside_a_style_rule() {
    let rs = rules("a { & b { margin: 0 } }");
    assert_eq!(rs.len(), 2);
}
