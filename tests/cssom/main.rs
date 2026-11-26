// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! CSSOM mutation and serialization tests (spec 18). Built only with the `cssom`
//! feature.

#![cfg(feature = "cssom")]

use tpt_css::cssom::{CssomError, CssomSheet};
use tpt_css::StyleChange;

#[test]
fn insert_and_delete_rules_by_index() {
    let mut sheet = CssomSheet::parse("a { color: red }");
    let (idx, change) = sheet.insert_rule("b { color: blue }", 0).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(change, StyleChange::StyleSheetChanged);
    assert_eq!(sheet.len(), 2);
    assert_eq!(sheet.selector_text(0).unwrap(), "b");
    sheet.delete_rule(0).unwrap();
    assert_eq!(sheet.selector_text(0).unwrap(), "a");
}

#[test]
fn bad_indexes_and_text_are_errors_and_leave_the_sheet_unchanged() {
    let mut sheet = CssomSheet::parse("a { color: red }");
    let before = sheet.clone();
    assert_eq!(
        sheet.insert_rule("b { }", 5).unwrap_err(),
        CssomError::IndexSize
    );
    assert_eq!(sheet.delete_rule(9).unwrap_err(), CssomError::IndexSize);
    assert_eq!(
        sheet.insert_rule("not a rule", 0).unwrap_err(),
        CssomError::Syntax
    );
    assert_eq!(sheet, before);
}

#[test]
fn selector_text_round_trips_through_set_selector_text() {
    let mut sheet = CssomSheet::parse("a > b.c:not(.d)::before { color: red }");
    let text = sheet.selector_text(0).unwrap();
    assert_eq!(text, "a > b.c:not(.d)::before");
    sheet
        .set_selector_text(0, "p, span:nth-child(2n+1)")
        .unwrap();
    assert_eq!(sheet.selector_text(0).unwrap(), "p, span:nth-child(2n+1)");
}

#[test]
fn set_and_remove_declarations() {
    let mut sheet = CssomSheet::parse("a { color: red; margin: 0 }");
    sheet.set_property(0, "color", "blue").unwrap();
    assert_eq!(sheet.property(0, "color").as_deref(), Some("blue"));
    sheet.set_property(0, "padding", "1px !important").unwrap();
    let (removed, _) = sheet.remove_property(0, "margin").unwrap();
    assert!(removed);
    let (removed_again, _) = sheet.remove_property(0, "margin").unwrap();
    assert!(!removed_again);
    let text = sheet.serialize();
    assert!(text.contains("color: blue"));
    assert!(text.contains("padding: 1px !important"));
    assert!(!text.contains("margin"));
}

#[test]
fn declaration_edits_on_a_non_style_rule_are_errors() {
    let mut sheet = CssomSheet::parse("@layer base { a { color: red } }");
    assert_eq!(
        sheet.set_property(0, "color", "blue").unwrap_err(),
        CssomError::NotAStyleRule
    );
    assert_eq!(
        sheet.selector_text(0).unwrap_err(),
        CssomError::NotAStyleRule
    );
}

#[test]
fn serialization_is_canonical_and_deterministic() {
    let source = "a{color:red}  @media (min-width:100px){b{margin:0 auto}}";
    let first = CssomSheet::parse(source).serialize();
    let second = CssomSheet::parse(source).serialize();
    assert_eq!(first, second);
    let reparsed = CssomSheet::parse(&first).serialize();
    assert_eq!(
        reparsed, first,
        "serializing the canonical text gives the same text"
    );
}

#[test]
fn a_mutated_sheet_converts_to_a_stylesheet_for_the_resolver() {
    let mut sheet = CssomSheet::parse("a { color: red }");
    sheet.set_property(0, "color", "lime").unwrap();
    let stylesheet = sheet.to_stylesheet();
    assert_eq!(stylesheet.rules.len(), 1);
}

#[test]
fn an_nth_selector_serializes_with_its_formula() {
    let sheet = CssomSheet::parse("li:nth-last-child(-n+3 of .x) { color: red }");
    assert!(sheet
        .serialize()
        .starts_with("li:nth-last-child(-n+3 of .x) {"));
}

#[test]
fn shadow_selectors_serialize_in_their_own_syntax() {
    use tpt_css::cssom::CssomSheet;
    let sheet = CssomSheet::parse(
        ":host(.on) span { color: red } ::slotted(p) { margin: 0 } .a::part(label) { color: red }",
    );
    let text = sheet.serialize();
    assert!(text.contains(":host(.on) span"), "{text}");
    assert!(text.contains("::slotted(p)"), "{text}");
    assert!(text.contains(".a::part(label)"), "{text}");
}
