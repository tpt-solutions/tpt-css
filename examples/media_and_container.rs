// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Media queries and container queries on one element. The viewport width
//! comes from `Environment`, and the width of the enclosing container comes
//! from the host through `ContainerSizeProvider`. The same stylesheet gives a
//! different color for each combination.
//!
//! Run with `cargo run --example media_and_container`.

#[cfg(not(test))]
mod common;
// Under `cargo test`, tests/examples.rs loads the shared module once at its root.
#[cfg(test)]
use crate::common;

use common::{color_of, Tree};
use tpt_css::tree::NodeId;
use tpt_css::{
    parse_stylesheet, ContainerInfo, ContainerSizeProvider, Environment, Origin, ParseOptions,
    ResolverConfig, StyleResolver, StyleTree,
};

const SHEET: &str = "
    .tile { color: rgb(0, 0, 0); }
    @container (min-width: 300px) { .tile { color: rgb(0, 128, 0); } }
    @media (min-width: 800px) { .tile { color: rgb(0, 0, 255); } }
";

/// The host's container sizes: one node and its width in px.
struct Containers(Vec<(NodeId, f64)>);

impl ContainerSizeProvider for Containers {
    fn container(&self, node: NodeId) -> Option<ContainerInfo> {
        self.0
            .iter()
            .find(|(id, _)| *id == node)
            .map(|(_, width)| ContainerInfo {
                width: *width,
                height: 100.0,
                name: None,
            })
    }
}

/// The computed text color of a `.tile` inside a container of
/// `container_width` px, in a viewport of `viewport_width` px.
pub fn tile_color(viewport_width: f64, container_width: f64) -> String {
    let mut tree = Tree::new();
    let layout = tree.add(tree.root(), "main", Some("layout"));
    let card = tree.add(layout, "div", Some("card"));
    let tile = tree.add(card, "p", Some("tile"));

    let parsed = parse_stylesheet(SHEET, ParseOptions::default());
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
    resolver.set_container_provider(Box::new(Containers(vec![(layout, container_width)])));

    let mut env = Environment::default();
    env.viewport.width = viewport_width;
    let styles = resolver.resolve(&tree, &env);

    color_of(styles.get(tile).expect("tile is styled"))
}

fn main() {
    println!(
        "wide viewport, wide container:   {}",
        tile_color(1000.0, 400.0)
    );
    println!(
        "narrow viewport, wide container: {}",
        tile_color(500.0, 400.0)
    );
    println!(
        "narrow viewport, narrow box:     {}",
        tile_color(500.0, 200.0)
    );
}
