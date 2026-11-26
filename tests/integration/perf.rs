// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Performance measurements (spec 21.1). These are ignored by default, because
//! timings depend on the machine. Run them in release mode and record the output
//! in SPEC_TRACE.md:
//!
//! ```text
//! cargo test --release --test integration perf -- --ignored --nocapture
//! ```

use std::time::Instant;

use super::common::{parse_sheet, TestTree};
use tpt_css::cascade::origin::Origin;
use tpt_css::syntax::{tokenize, SourceId};
use tpt_css::tree::{NodeKind, StyleTree};
use tpt_css::{Environment, ParseLimits, ResolverConfig, StyleResolver};

#[test]
#[ignore]
fn tokenizer_throughput() {
    let unit = "a.b > c[d=\"e\"] { color: rgb(1 2 3); margin: 10px 2em calc(1px + 2px) } ";
    let input = unit.repeat(4000);
    let limits = ParseLimits {
        max_tokens: usize::MAX / 2,
        ..ParseLimits::default()
    };
    let start = Instant::now();
    let rounds = 5;
    let mut tokens = 0usize;
    for _ in 0..rounds {
        tokens += tokenize(SourceId(0), &input, &limits).tokens.len();
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "tokenizer: {:.0} tokens/second ({} tokens over {rounds} rounds)",
        tokens as f64 / secs,
        tokens
    );
}

#[test]
#[ignore]
fn static_tree_restyle() {
    // A tree of about 10,000 elements, styled by a small sheet.
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let mut frontier = vec![root];
    let mut count = 1;
    while count < 10_000 {
        let mut next = Vec::new();
        for parent in frontier {
            for _ in 0..10 {
                if count >= 10_000 {
                    break;
                }
                next.push(t.element(parent, "div"));
                count += 1;
            }
        }
        frontier = next;
    }
    // Subject-only rules, so the restyle cutoff applies. A descendant rule such as
    // `div div` would force the whole subtree to be restyled.
    let css = "div { margin-left: 1px } .none { color: red }";
    let mut resolver = StyleResolver::new(ResolverConfig::default());
    resolver.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
    let env = Environment::default();

    let start = Instant::now();
    let _ = resolver.resolve(&t, &env);
    let full = start.elapsed();

    // A local change: one leaf gets a class no rule uses.
    let leaf = *t.children(root).first().expect("a child");
    t.add_class(leaf, "unused");
    resolver.invalidate(&t, tpt_css::StyleChange::AttributeChanged(leaf));
    let start = Instant::now();
    let _ = resolver.restyle(&t, &env);
    let local = start.elapsed();

    println!(
        "static tree of {count} elements: full resolve {:?}, local restyle {:?} ({} elements restyled)",
        full,
        local,
        resolver.last_restyle_count()
    );
    assert_eq!(
        resolver.last_restyle_count(),
        1,
        "a local change restyles one element"
    );
}

/// Used by the timing test; keeps the generated sheet type-checked in normal runs.
#[test]
fn perf_fixtures_are_well_formed() {
    let mut t = TestTree::new(NodeKind::Element);
    let _ = t.element(t.root(), "div");
    let _ = parse_sheet("div { margin-left: 1px }");
}

/// Full resolve of a wide list with structural rules. Each sibling needs its
/// position, so a matcher that walks the siblings for every element would take
/// time that grows as the square of the list length. Time is printed for two
/// sizes; quadratic growth shows as about 16 times the time for 4 times the list.
#[test]
#[ignore]
fn wide_sibling_list_with_structural_rules() {
    let css = "li:nth-child(odd) { margin-left: 1px } li + li { color: red } li:nth-of-type(3n) { margin-top: 1px } li ~ li { padding-top: 1px }";
    let mut timings = Vec::new();
    for n in [2_000usize, 8_000] {
        let mut t = TestTree::new(NodeKind::Element);
        let root = t.root();
        t.nodes[0].name.local = "ul".into();
        for _ in 0..n {
            t.element(root, "li");
        }
        let mut resolver = StyleResolver::new(ResolverConfig::default());
        resolver.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
        let start = Instant::now();
        let map = resolver.resolve(&t, &Environment::default());
        let elapsed = start.elapsed();
        // The first li is odd and first, so it gets the margin and no red color.
        let first = *t.children(root).first().expect("a child");
        assert_eq!(
            map.get(first).map(|s| s.color()),
            Some(tpt_css::values::Color::BLACK)
        );
        timings.push((n, elapsed));
        println!("{n} siblings: full resolve {elapsed:?}");
    }
    let ratio = timings[1].1.as_secs_f64() / timings[0].1.as_secs_f64().max(1e-9);
    println!("time ratio for 4x the siblings: {ratio:.1} (quadratic would be about 16)");
}

/// Growth of each sibling-walking construct on its own, so a slow one is not
/// hidden by the fast ones in the combined test above. For each rule, the
/// time is printed for 2,000 and 8,000 siblings; quadratic growth shows as
/// about 16 times the time for 4 times the list.
#[test]
#[ignore]
fn sibling_constructs_scale_separately() {
    let rules = [
        ("li ~ li", "li ~ li { padding-top: 1px }"),
        (
            "li:nth-of-type(3n)",
            "li:nth-of-type(3n) { margin-top: 1px }",
        ),
        (
            "li:nth-child(2n of .x)",
            "li:nth-child(2n of .x) { margin-top: 1px }",
        ),
    ];
    for (label, css) in rules {
        let mut timings = Vec::new();
        for n in [2_000usize, 8_000] {
            let mut t = TestTree::new(NodeKind::Element);
            let root = t.root();
            t.nodes[0].name.local = "ul".into();
            for i in 0..n {
                let li = t.element(root, "li");
                if i % 2 == 0 {
                    t.add_class(li, "x");
                }
            }
            let mut resolver = StyleResolver::new(ResolverConfig::default());
            resolver.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
            let start = Instant::now();
            let _ = resolver.resolve(&t, &Environment::default());
            timings.push((n, start.elapsed()));
        }
        let ratio = timings[1].1.as_secs_f64() / timings[0].1.as_secs_f64().max(1e-9);
        println!(
            "{label}: {} siblings {:?}, {} siblings {:?}, ratio for 4x: {ratio:.1} (quadratic about 16)",
            timings[0].0, timings[0].1, timings[1].0, timings[1].1
        );
    }
}

/// Splits a full resolve into the part that does not depend on the sheet (tree
/// walk and computing every property) and the part that does (selector
/// matching and the cascade). An empty sheet computes initial values for every
/// element, so its time is the first part; the difference from a real sheet is
/// the second. This decides whether style sharing (DESIGN.md decision 35) can
/// save much: sharing skips computation, not matching.
#[test]
#[ignore]
fn resolve_split_between_computation_and_matching() {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let mut frontier = vec![root];
    let mut count = 1;
    while count < 10_000 {
        let mut next = Vec::new();
        for parent in frontier {
            for _ in 0..10 {
                if count >= 10_000 {
                    break;
                }
                next.push(t.element(parent, "div"));
                count += 1;
            }
        }
        frontier = next;
    }
    let sheet = "div { margin-left: 1px; color: red } .a div { padding: 2px } div:nth-child(2n) { margin-top: 3px }";
    let time = |css: &str| {
        let mut resolver = StyleResolver::new(ResolverConfig::default());
        resolver.add_stylesheet(parse_sheet(css).stylesheet, Origin::Author);
        let env = Environment::default();
        let _ = resolver.resolve(&t, &env);
        let start = Instant::now();
        let rounds = 5;
        for _ in 0..rounds {
            let _ = resolver.resolve(&t, &env);
        }
        start.elapsed() / rounds
    };
    let empty = time("");
    let full = time(sheet);
    println!(
        "10,000 elements: empty sheet {:?} (computation and traversal), real sheet {:?}, matching and cascade about {:?}",
        empty,
        full,
        full.saturating_sub(empty)
    );
}
