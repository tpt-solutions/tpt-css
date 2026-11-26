// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Computed-style snapshots for the MIT fixtures (DESIGN.md decision 41). Each
//! case resolves a fixture stylesheet against a tree in `fixtures/snapshots/`,
//! and the output is compared with the checked-in `.snap` file. A change fails
//! the test and prints the lines that differ. `TPT_UPDATE_SNAPSHOTS=1 cargo test`
//! writes new snapshots, which then need review like any other change.

use std::path::Path;

use super::fixture;

/// `(name, stylesheet, tree)`, paths relative to the repository root.
const CASES: &[(&str, &str, &str)] = &[
    (
        "basic",
        "fixtures/mit/basic.css",
        "fixtures/snapshots/basic.json",
    ),
    (
        "web-like",
        "fixtures/mit/web-like.css",
        "fixtures/snapshots/web-like.json",
    ),
];

fn repo_file(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

/// Lines that differ between `expected` and `actual`, as a unified-style list:
/// `-` for lines only in the expected output, `+` for lines only in the actual.
fn line_diff(expected: &str, actual: &str) -> String {
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    // Longest common subsequence table, then walk it back.
    let mut lcs = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = String::new();
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push_str(&format!("- {}\n", a[i]));
            i += 1;
        } else {
            out.push_str(&format!("+ {}\n", b[j]));
            j += 1;
        }
    }
    out
}

#[test]
fn computed_style_snapshots_match() {
    let update = std::env::var_os("TPT_UPDATE_SNAPSHOTS").is_some();
    let mut failures = Vec::new();
    for (name, css, tree) in CASES {
        let css_text = std::fs::read_to_string(repo_file(css)).expect("stylesheet exists");
        let tree_text = std::fs::read_to_string(repo_file(tree)).expect("tree exists");
        let lines = fixture::compute_report(&tree_text, false, &[css_text])
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let actual = format!("{}\n", lines.join("\n"));
        let snap = repo_file(&format!("fixtures/snapshots/{name}.snap"));
        if update {
            std::fs::write(&snap, &actual).expect("snapshot is writable");
            continue;
        }
        let expected = std::fs::read_to_string(&snap).unwrap_or_default();
        if expected != actual {
            failures.push(format!(
                "snapshot `{name}` changed:\n{}",
                line_diff(&expected, &actual)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{}\nRun `TPT_UPDATE_SNAPSHOTS=1 cargo test --test integration snapshots` to accept.",
        failures.join("\n")
    );
}

#[test]
fn line_diff_marks_only_the_changed_lines() {
    assert_eq!(line_diff("a\nb\nc\n", "a\nx\nc\n"), "- b\n+ x\n");
    assert_eq!(line_diff("a\n", "a\n"), "");
}
