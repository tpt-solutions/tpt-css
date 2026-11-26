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
