// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Parser tests: stylesheets, rules, declarations, selectors (spec 8.1, 11, 12.3).

use tpt_css::ast::{Combinator, PseudoClass, Rule, SimpleSelector};
use tpt_css::selectors::Specificity;
use tpt_css::{parse_declaration_list, parse_stylesheet, DiagnosticCode, ParseOptions};

fn sheet(css: &str) -> tpt_css::ParsedStylesheet {
    parse_stylesheet(css, ParseOptions::default())
}

fn style_rules(css: &str) -> Vec<tpt_css::ast::StyleRule> {
    sheet(css)
        .stylesheet
        .rules
        .into_iter()
        .filter_map(|r| match r {
            Rule::Style(s) => Some(s),
            _ => None,
        })
        .collect()
}

fn specificity_of(selector: &str) -> Specificity {
    let rules = style_rules(&format!("{selector} {{ }}"));
    assert_eq!(rules.len(), 1, "selector should parse: {selector}");
    rules[0].selectors.selectors[0].specificity
}

#[test]
fn a_simple_rule_parses_selectors_and_declarations() {
    let rules = style_rules("p.note, #main > a { color: red; margin: 0 }");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].selectors.selectors.len(), 2);
    let decls = &rules[0].declarations.declarations;
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "color");
    assert!(!decls[0].important);
}

#[test]
fn property_names_are_lowercased_but_custom_properties_keep_case() {
    let parsed = parse_declaration_list("COLOR: red; --Mixed-Case: 1", ParseOptions::default());
    assert_eq!(parsed.declarations[0].property, "color");
    assert_eq!(parsed.declarations[1].property, "--Mixed-Case");
}

#[test]
fn important_is_detected_case_insensitively_with_space() {
    let parsed = parse_declaration_list("color: red ! IMPORTANT", ParseOptions::default());
    assert!(parsed.declarations[0].important);
    assert_eq!(parsed.declarations[0].value.len(), 1);
}

#[test]
fn an_invalid_declaration_does_not_abort_its_neighbours() {
    let rules = style_rules("p { color red; margin: 0; : bad; width: 1px }");
    let names: Vec<&str> = rules[0]
        .declarations
        .declarations
        .iter()
        .map(|d| d.property.as_str())
        .collect();
    assert_eq!(names, vec!["margin", "width"]);
}

#[test]
fn an_invalid_declaration_reports_a_diagnostic() {
    let parsed = sheet("p { color red }");
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::InvalidDeclaration));
}

#[test]
fn a_rule_with_an_invalid_selector_is_dropped_and_others_survive() {
    let parsed = sheet("p > > em { color: red } em { color: blue }");
    let rules = parsed
        .stylesheet
        .rules
        .iter()
        .filter(|r| matches!(r, Rule::Style(_)))
        .count();
    assert_eq!(rules, 1);
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::InvalidSelector));
}

#[test]
fn unsupported_selectors_report_unsupported_not_invalid() {
    let parsed = sheet("a::first-line { color: red }");
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::UnsupportedSelector));
}

#[test]
fn unknown_at_rules_are_preserved_and_do_not_break_the_sheet() {
    let parsed = sheet("@charset \"utf-8\"; @foo bar { x: y } p { color: red }");
    assert_eq!(parsed.stylesheet.rules.len(), 3);
    assert!(matches!(parsed.stylesheet.rules[0], Rule::Unknown(_)));
    assert!(matches!(parsed.stylesheet.rules[1], Rule::Unknown(_)));
    assert!(matches!(parsed.stylesheet.rules[2], Rule::Style(_)));
}

#[test]
fn cdo_and_cdc_at_top_level_are_ignored() {
    let rules = style_rules("<!-- p { color: red } -->");
    assert_eq!(rules.len(), 1);
}

#[test]
fn descendant_and_child_combinators_are_recorded() {
    let rules = style_rules("a b > c { }");
    let sel = &rules[0].selectors.selectors[0];
    assert_eq!(sel.compounds.len(), 3);
    assert_eq!(
        sel.combinators,
        vec![Combinator::Descendant, Combinator::Child]
    );
}

#[test]
fn pseudo_classes_and_attributes_parse() {
    let rules = style_rules("[data-x=\"1\"]:hover:root { }");
    let compound = &rules[0].selectors.selectors[0].compounds[0];
    assert!(matches!(compound.simple[0], SimpleSelector::Attribute(_)));
    assert_eq!(
        compound.simple[2],
        SimpleSelector::PseudoClass(PseudoClass::Root)
    );
}

#[test]
fn specificity_follows_spec_12_3() {
    assert_eq!(specificity_of("*"), Specificity { a: 0, b: 0, c: 0 });
    assert_eq!(specificity_of("div"), Specificity { a: 0, b: 0, c: 1 });
    assert_eq!(specificity_of(".a"), Specificity { a: 0, b: 1, c: 0 });
    assert_eq!(specificity_of("#a"), Specificity { a: 1, b: 0, c: 0 });
    assert_eq!(specificity_of("[x]"), Specificity { a: 0, b: 1, c: 0 });
    assert_eq!(specificity_of(":root"), Specificity { a: 0, b: 1, c: 0 });
    assert_eq!(specificity_of("#a .b c"), Specificity { a: 1, b: 1, c: 1 });
}

#[test]
fn specificity_orders_lexicographically() {
    assert!(specificity_of("#a") > specificity_of(".a .b .c .d"));
    assert!(specificity_of(".a") > specificity_of("div div div"));
}

#[test]
fn a_selector_list_entry_carries_its_own_specificity() {
    let rules = style_rules("#x, p { }");
    let list = &rules[0].selectors.selectors;
    assert_eq!(list[0].specificity, Specificity { a: 1, b: 0, c: 0 });
    assert_eq!(list[1].specificity, Specificity { a: 0, b: 0, c: 1 });
}

#[test]
fn selector_complexity_limit_rejects_long_selectors() {
    let limits = tpt_css::ParseLimits {
        max_selector_complexity: 2,
        ..Default::default()
    };
    let opts = ParseOptions {
        limits,
        ..Default::default()
    };
    let parsed = parse_stylesheet("a b c d { color: red }", opts);
    assert!(parsed.stylesheet.rules.is_empty());
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::LimitExceeded));
}

#[test]
fn declaration_limit_stops_the_block() {
    let limits = tpt_css::ParseLimits {
        max_declarations_per_block: 2,
        ..Default::default()
    };
    let opts = ParseOptions {
        limits,
        ..Default::default()
    };
    let parsed = parse_declaration_list("a: 1; b: 2; c: 3", opts);
    assert_eq!(parsed.declarations.len(), 2);
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::LimitExceeded));
}

#[test]
fn deep_nesting_drops_the_sheet_with_a_diagnostic() {
    let limits = tpt_css::ParseLimits {
        max_nesting_depth: 8,
        ..Default::default()
    };
    let opts = ParseOptions {
        limits,
        ..Default::default()
    };
    let css = format!("p {{ x: {} }}", "f(".repeat(100));
    let parsed = parse_stylesheet(&css, opts);
    assert!(parsed.stylesheet.rules.is_empty());
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::LimitExceeded));
}

mod m2;

#[test]
fn comment_spans_are_kept_only_when_asked_for() {
    let css = "/* a */ p { color: red } /* b */";
    let off = tpt_css::parse_stylesheet(css, ParseOptions::default());
    assert!(off.comments.is_empty());

    let on = tpt_css::parse_stylesheet(
        css,
        ParseOptions {
            preserve_comments: true,
            ..Default::default()
        },
    );
    let texts: Vec<&str> = on
        .comments
        .iter()
        .map(|s| &css[s.start as usize..s.end as usize])
        .collect();
    assert_eq!(texts, vec!["/* a */", "/* b */"]);
}

/// Regression test for a fuzzer out-of-memory. Each nested level clones the
/// parent selector list into `:is()` once per child selector, so a comma list
/// nested inside itself doubles the list at every level. The size limit must
/// stop this well before it allocates the full tree.
#[test]
fn nested_comma_lists_are_bounded_by_resolved_selector_size() {
    let css = format!("{}{}", "a,b{".repeat(40), "}".repeat(40));
    let parsed = parse_stylesheet(&css, ParseOptions::default());
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::LimitExceeded));
    let total: usize = style_rules(&css)
        .iter()
        .map(|r| r.selectors.selectors.len())
        .sum();
    assert!(total < 10_000, "resolved selectors grew to {total}");
}
