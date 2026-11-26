// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Reporting parse diagnostics. Invalid declarations are dropped without
//! stopping the rest of the sheet, and each drop is reported here with its
//! code, location, and specification link.
//!
//! Run with `cargo run --example diagnostics`.

use tpt_css::{parse_stylesheet, Diagnostic, ParseOptions, Severity};

const SHEET: &str = "
    p { color: red; colr: blue; }
    h1 { color: }
    .ok { margin: 4px }
";

/// One line per diagnostic: severity, code, the line and column of the
/// problem, and the message.
pub fn describe(diagnostic: &Diagnostic, source: &str) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    let (line, column) = line_and_column(source, diagnostic.span.start as usize);
    format!(
        "{severity} {:?} at {line}:{column}: {}",
        diagnostic.code, diagnostic.message
    )
}

/// The 1-based line and column of byte offset `offset` in `source`.
fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, str::len) + 1;
    (line, column)
}

/// Every diagnostic for `sheet`, in source order.
pub fn diagnostics_for(sheet: &str) -> Vec<String> {
    parse_stylesheet(sheet, ParseOptions::default())
        .diagnostics
        .iter()
        .map(|d| describe(d, sheet))
        .collect()
}

fn main() {
    let lines = diagnostics_for(SHEET);
    if lines.is_empty() {
        println!("no diagnostics");
    }
    for line in lines {
        println!("{line}");
    }
}
