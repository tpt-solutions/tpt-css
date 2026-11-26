// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Golden tests (spec 22.1). Each `fixtures/golden/<name>.css` is tokenized
//! and compared with `fixtures/golden/<name>.tokens`.
//!
//! Regenerate expected files with `TPT_UPDATE_GOLDEN=1 cargo test --test golden`,
//! then review the diff before committing.

use std::fs;
use std::path::PathBuf;

use tpt_css::syntax::{tokenize, SourceId};
use tpt_css::ParseLimits;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/golden")
}

/// One line per token: byte range, then the token kind.
fn dump(input: &str) -> String {
    let t = tokenize(SourceId(0), input, &ParseLimits::default());
    let mut out = String::new();
    for tok in &t.tokens {
        out.push_str(&format!(
            "{:>4}..{:<4} {:?}\n",
            tok.span.start, tok.span.end, tok.kind
        ));
    }
    out
}

#[test]
fn tokenizer_golden_files_match() {
    let update = std::env::var("TPT_UPDATE_GOLDEN").is_ok();
    let mut checked = 0;
    let mut entries: Vec<PathBuf> = fs::read_dir(golden_dir())
        .expect("fixtures/golden exists")
        .map(|e| e.expect("readable entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "css"))
        .collect();
    entries.sort();

    for css_path in entries {
        // Normalize line endings so that Windows checkouts (CRLF) match LF goldens.
        let input = fs::read_to_string(&css_path)
            .expect("fixture is UTF-8")
            .replace("\r\n", "\n");
        let actual = dump(&input);
        let expected_path = css_path.with_extension("tokens");
        if update {
            fs::write(&expected_path, &actual).expect("write golden file");
        } else {
            let expected = fs::read_to_string(&expected_path).unwrap_or_else(|_| {
                panic!(
                    "missing {}; run with TPT_UPDATE_GOLDEN=1",
                    expected_path.display()
                )
            });
            assert_eq!(
                actual.replace("\r\n", "\n"),
                expected.replace("\r\n", "\n"),
                "golden mismatch for {}",
                css_path.display()
            );
        }
        checked += 1;
    }
    assert!(checked > 0, "no golden fixtures found");
}
