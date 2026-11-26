// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Container queries (spec 16.5): the host supplies container sizes through
//! `ContainerSizeProvider`, and `@container` rules apply to descendants.

use super::common::{parse_sheet, style_of, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::properties::lookup;
use tpt_css::tree::{NodeId, NodeKind};
use tpt_css::values::Value;
use tpt_css::{ContainerInfo, ContainerSizeProvider, Environment, ResolverConfig, StyleResolver};

struct Sizes(Vec<(NodeId, ContainerInfo)>);

impl ContainerSizeProvider for Sizes {
    fn container(&self, node: NodeId) -> Option<ContainerInfo> {
        self.0
            .iter()
            .find(|(n, _)| *n == node)
            .map(|(_, info)| info.clone())
    }
}

/// root (container) > child > leaf
fn tree() -> (TestTree, NodeId, NodeId, NodeId) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    let leaf = t.element(child, "leaf");
    (t, root, child, leaf)
}

fn margin(css: &str, width: f64, name: Option<&str>) -> Value {
    let (t, root, _, leaf) = tree();
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
    resolver.set_container_provider(Box::new(Sizes(vec![(
        root,
        ContainerInfo {
            width,
            height: 100.0,
            name: name.map(str::to_string),
        },
    )])));
    let map = resolver.resolve(&t, &Environment::default());
    style_of(map, leaf)
        .raw(lookup("margin-left").unwrap())
        .clone()
}

#[test]
fn a_container_query_applies_when_the_container_is_wide_enough() {
    let css = "@container (min-width: 300px) { leaf { margin-left: 1px } }";
    assert_eq!(margin(css, 400.0, None), Value::px(1.0));
    assert_eq!(margin(css, 200.0, None), Value::px(0.0));
}

#[test]
fn a_named_container_query_only_considers_containers_with_that_name() {
    let css = "@container side (max-width: 100px) { leaf { margin-left: 2px } }";
    assert_eq!(margin(css, 50.0, Some("side")), Value::px(2.0));
    assert_eq!(margin(css, 50.0, Some("main")), Value::px(0.0));
    assert_eq!(margin(css, 50.0, None), Value::px(0.0));
}

#[test]
fn a_container_query_with_a_non_size_feature_is_false() {
    let css = "@container (orientation: landscape) { leaf { margin-left: 3px } }";
    assert_eq!(margin(css, 400.0, None), Value::px(0.0));
}

#[test]
fn without_a_provider_container_queries_do_not_apply() {
    let (t, _, _, leaf) = tree();
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(
        parse_sheet("@container (min-width: 1px) { leaf { margin-left: 1px } }").stylesheet,
        Origin::Author,
    );
    let map = resolver.resolve(&t, &Environment::default());
    assert_eq!(
        style_of(map, leaf).raw(lookup("margin-left").unwrap()),
        &Value::px(0.0)
    );
}
