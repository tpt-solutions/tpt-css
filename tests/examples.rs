// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The programs in `examples/` are compiled and checked here, so they cannot
//! drift from the API. `cargo test` builds the examples but does not run them.

// The shared example tree, loaded once. The examples use it under `cfg(test)`.
#[path = "../examples/common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../examples/minimal.rs"]
mod minimal;

#[allow(dead_code)]
#[path = "../examples/dark_mode.rs"]
mod dark_mode;

#[allow(dead_code)]
#[path = "../examples/diagnostics.rs"]
mod diagnostics;

#[allow(dead_code)]
#[path = "../examples/media_and_container.rs"]
mod media_and_container;

#[cfg(feature = "cssom")]
#[allow(dead_code)]
#[path = "../examples/cssom_edit.rs"]
mod cssom_edit;

#[test]
fn minimal_example_colors_match_the_cascade() {
    assert_eq!(
        minimal::styled_colors(),
        vec![
            ("p.note", "rgb(0, 0, 255)".to_string()),
            ("p", "rgb(255, 0, 0)".to_string()),
        ]
    );
}

#[test]
fn dark_mode_example_switches_with_the_color_scheme() {
    assert_eq!(dark_mode::paragraph_color(false), "rgb(34, 34, 34)");
    assert_eq!(dark_mode::paragraph_color(true), "rgb(238, 238, 238)");
}

#[test]
fn media_and_container_example_applies_each_rule_when_its_condition_holds() {
    use media_and_container::tile_color;
    assert_eq!(tile_color(1000.0, 400.0), "rgb(0, 0, 255)");
    assert_eq!(tile_color(500.0, 400.0), "rgb(0, 128, 0)");
    assert_eq!(tile_color(500.0, 200.0), "rgb(0, 0, 0)");
}

#[test]
fn unknown_property_is_reported_but_custom_properties_and_shorthands_are_not() {
    let lines = diagnostics::diagnostics_for("p { --x: 1; padding: 0; colr: blue; }");
    assert_eq!(lines.len(), 1, "unexpected diagnostics: {lines:?}");
    assert!(
        lines[0].starts_with("info UnknownProperty at 1:"),
        "unexpected diagnostic: {}",
        lines[0]
    );
}

#[test]
fn diagnostics_example_reports_the_invalid_declaration_with_its_position() {
    let lines = diagnostics::diagnostics_for("\np { color: red; colr: blue; }\nh1 { color: }\n");
    assert_eq!(lines.len(), 2, "unexpected diagnostics: {lines:?}");
    assert!(
        lines[0].starts_with("info UnknownProperty at 2:17:"),
        "unexpected diagnostic: {}",
        lines[0]
    );
    assert!(
        lines[1].starts_with("error InvalidDeclaration at 3:6:"),
        "unexpected diagnostic: {}",
        lines[1]
    );
}

#[cfg(feature = "cssom")]
#[test]
fn cssom_example_edits_are_serialized() {
    assert_eq!(
        cssom_edit::edited_sheet(),
        "p { color: blue }\n.note { font-style: italic }"
    );
}
