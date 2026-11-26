// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The smallest useful program: parse a stylesheet, style a tree, print colors.
//!
//! Run with `cargo run --example minimal`.

#[cfg(not(test))]
mod common;
// Under `cargo test`, tests/examples.rs loads the shared module once at its root.
#[cfg(test)]
use crate::common;

use common::{color_of, Tree};
use tpt_css::StyleTree;
use tpt_css::{parse_stylesheet, Environment, Origin, ParseOptions, ResolverConfig, StyleResolver};

/// Styles `<body><p class="note">first</p><p>second</p></body>` and returns the
/// computed `color` of each paragraph, in document order.
pub fn styled_colors() -> Vec<(&'static str, String)> {
    let mut tree = Tree::new();
    let body = tree.add(tree.root(), "body", None);
    let note = tree.add(body, "p", Some("note"));
    let plain = tree.add(body, "p", None);

    let css = "p { color: red; font-size: 16px } .note { color: blue }";
    let parsed = parse_stylesheet(css, ParseOptions::default());

    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
    let styles = resolver.resolve(&tree, &Environment::default());

    [("p.note", note), ("p", plain)]
        .into_iter()
        .map(|(label, node)| {
            let style = styles.get(node).expect("every element is styled");
            (label, color_of(style))
        })
        .collect()
}

fn main() {
    for (label, color) in styled_colors() {
        println!("{label}: color = {color}");
    }
}
