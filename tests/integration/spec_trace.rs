// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Spec trace check (spec 22.5, 23.3). Every source or test file that the trace
//! names must exist, so the trace cannot silently point at files that were moved
//! or removed.

use std::fs;
use std::path::Path;

/// Backtick-quoted tokens that end in `.rs`, from the table rows of the trace.
fn rust_paths_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines().filter(|l| l.trim_start().starts_with('|')) {
        let mut rest = line;
        while let Some(start) = rest.find('`') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('`') else {
                break;
            };
            let token = &after[..end];
            if token.ends_with(".rs") && !token.contains(' ') {
                out.push(token.to_string());
            }
            rest = &after[end + 1..];
        }
    }
    out
}

#[test]
fn every_file_named_in_the_spec_trace_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let trace = fs::read_to_string(root.join("SPEC_TRACE.md")).expect("SPEC_TRACE.md exists");
    let paths = rust_paths_in(&trace);
    assert!(
        paths.len() > 40,
        "the trace should name many files, found {}",
        paths.len()
    );
    let missing: Vec<&String> = paths
        .iter()
        .filter(|p| !root.join("src").join(p).exists() && !root.join(p).exists())
        .collect();
    assert!(
        missing.is_empty(),
        "SPEC_TRACE.md names missing files: {missing:?}"
    );
}
