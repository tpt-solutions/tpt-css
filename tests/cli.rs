// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The `tpt-css` command line: exit codes, diagnostics, and the `compute`
//! fixture format. The binary is built by cargo for integration tests.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_tpt-css");

fn check_stdin(input: &str) -> (i32, String) {
    let mut child = Command::new(BIN)
        .args(["check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("binary runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
    )
}

#[test]
fn check_reports_an_unknown_property_as_info_and_exits_zero() {
    let (code, out) = check_stdin("p { colr: blue }\n");
    assert_eq!(code, 0);
    assert!(
        out.starts_with("info UnknownProperty at 1:5:"),
        "unexpected output: {out}"
    );
}

#[test]
fn check_exits_one_on_an_error_and_names_its_position() {
    let (code, out) = check_stdin("p { color: red }\nh1 { color: }\n");
    assert_eq!(code, 1);
    assert!(
        out.contains("error InvalidDeclaration at 2:"),
        "unexpected output: {out}"
    );
}

#[test]
fn check_of_a_clean_sheet_says_so() {
    let (code, out) = check_stdin("p { color: red }\n");
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "no diagnostics");
}

#[test]
fn usage_errors_exit_two() {
    let out = Command::new(BIN).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let out = Command::new(BIN)
        .args(["check", "does-not-exist.css"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn compute_prints_only_what_the_stylesheet_changed() {
    let fixture = r#"{
        "stylesheets": ["p { color: red } .note { color: blue }"],
        "root": {"element": "div", "id": "main", "children": [
            {"element": "p", "class": "note", "style": "margin-top: 3px",
             "children": [{"text": "hi"}]},
            {"element": "p"}
        ]}
    }"#;
    let path = std::env::temp_dir().join(format!("tpt-css-cli-{}.json", std::process::id()));
    std::fs::write(&path, fixture).unwrap();
    let out = Command::new(BIN)
        .args(["compute", path.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_file(&path).ok();

    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    // The div is unchanged, so it has no property lines.
    assert!(text.starts_with("0 div#main\n0.0 p.note\n"), "{text}");
    assert!(text.contains("  color: rgb(0, 0, 255)\n"), "{text}");
    assert!(text.contains("  margin-top: 3px\n"), "{text}");
    assert!(text.contains("0.1 p\n  color: rgb(255, 0, 0)\n"), "{text}");
    // Nothing the author did not touch is printed.
    assert!(!text.contains("font-size"), "{text}");
}

#[test]
fn compute_needs_a_root() {
    let path = std::env::temp_dir().join(format!("tpt-css-cli-noroot-{}.json", std::process::id()));
    std::fs::write(&path, r#"{"stylesheets": []}"#).unwrap();
    let out = Command::new(BIN)
        .args(["compute", path.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("root"));
}

#[test]
fn inspect_lists_the_winning_declaration_with_its_origin_and_specificity() {
    let fixture = r#"{
        "stylesheets": ["p { color: red } #main p.note { color: blue !important }"],
        "root": {"element": "div", "id": "main", "children": [
            {"element": "p", "class": "note", "style": "margin-top: 3px",
             "children": [{"text": "hi"}]}
        ]}
    }"#;
    let path =
        std::env::temp_dir().join(format!("tpt-css-cli-inspect-{}.json", std::process::id()));
    std::fs::write(&path, fixture).unwrap();
    let out = Command::new(BIN)
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_file(&path).ok();

    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("0 div#main\n0.0 p.note\n"), "{text}");
    // The important author rule beats the plain one, and the note says why.
    assert!(
        text.contains("  color: rgb(0, 0, 255) [author; important; layer 0; specificity 1,1,1; sheet 0 rule 1 declaration 0]\n"),
        "{text}"
    );
    // The inline declaration is marked as coming from the style attribute.
    assert!(
        text.contains(
            "  margin-top: 3px [author; unlayered; inline; specificity 0,0,0; style attribute]\n"
        ),
        "{text}"
    );
}

#[test]
fn inspect_needs_exactly_one_tree() {
    let out = Command::new(BIN).args(["inspect"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}
