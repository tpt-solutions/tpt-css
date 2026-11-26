// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Theming with custom properties and a color-scheme media query. The host
//! decides the user's preference through `Environment`, and the same stylesheet
//! gives a different text color for each.
//!
//! Run with `cargo run --example dark_mode`.

#[cfg(not(test))]
mod common;
// Under `cargo test`, tests/examples.rs loads the shared module once at its root.
#[cfg(test)]
use crate::common;

use common::{color_of, Tree};
use tpt_css::StyleTree;
use tpt_css::{parse_stylesheet, Environment, Origin, ParseOptions, ResolverConfig, StyleResolver};

const THEME: &str = "
    body { --fg: #222222; }
    @media (prefers-color-scheme: dark) {
        body { --fg: #eeeeee; }
    }
    p { color: var(--fg); }
";

/// The computed text color of a paragraph under the given color-scheme
/// preference.
pub fn paragraph_color(prefers_dark: bool) -> String {
    let mut tree = Tree::new();
    let body = tree.add(tree.root(), "body", None);
    let paragraph = tree.add(body, "p", None);

    let parsed = parse_stylesheet(THEME, ParseOptions::default());
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(parsed.stylesheet, Origin::Author);

    let mut env = Environment::default();
    env.media.prefers_dark = prefers_dark;
    let styles = resolver.resolve(&tree, &env);

    color_of(styles.get(paragraph).expect("paragraph is styled"))
}

fn main() {
    println!("light: p color = {}", paragraph_color(false));
    println!("dark:  p color = {}", paragraph_color(true));
}
