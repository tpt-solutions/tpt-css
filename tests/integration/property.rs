// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property-style tests (spec 22.4). Inputs come from a seeded generator, so the
//! runs are reproducible. Each property is checked over many random cases.

use super::common::{parse_sheet, style_with, TestTree};
use tpt_css::ast::{ComplexSelector, Rule};
use tpt_css::cascade::origin::Origin;
use tpt_css::parse::parse_declaration_list;
use tpt_css::selectors::matching::matches_complex;
use tpt_css::tree::{NodeId, NodeKind};
use tpt_css::{Environment, ParseOptions, ResolverConfig, StyleResolver};

/// A seeded LCG, so every run generates the same cases.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() as usize) % items.len()]
    }
}

const TAGS: &[&str] = &["a", "b", "p", "div", "span"];
const CLASSES: &[&str] = &["x", "y", "z"];

/// A random compound selector: a tag, optionally a class and an id.
fn random_compound(rng: &mut Rng) -> String {
    let mut s = rng.pick(TAGS).to_string();
    if rng.next() % 2 == 0 {
        s.push('.');
        s.push_str(rng.pick(CLASSES));
    }
    if rng.next() % 4 == 0 {
        s.push_str("#i");
    }
    s
}

/// A random complex selector of 1 to 3 compounds joined by descendant spaces.
fn random_selector(rng: &mut Rng) -> String {
    let n = 1 + (rng.next() % 3) as usize;
    (0..n)
        .map(|_| random_compound(rng))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first complex selector of `text`, parsed as a style rule.
fn first_selector(text: &str) -> ComplexSelector {
    let sheet = parse_sheet(&format!("{text} {{ }}")).stylesheet;
    match &sheet.rules[0] {
        Rule::Style(s) => s.selectors.selectors[0].clone(),
        _ => panic!("style rule expected"),
    }
}

/// A tree with a few tags, classes, and an id, to match random selectors against.
fn random_tree(rng: &mut Rng) -> (TestTree, Vec<NodeId>) {
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "div".into();
    let mut nodes = vec![root];
    for _ in 0..30 {
        let parent = nodes[(rng.next() as usize) % nodes.len()];
        let node = t.element(parent, rng.pick(TAGS));
        if rng.next() % 2 == 0 {
            t.add_class(node, rng.pick(CLASSES));
        }
        if rng.next() % 4 == 0 {
            t.set_id(node, "i");
        }
        nodes.push(node);
    }
    (t, nodes)
}

#[test]
fn specificity_is_additive_over_compounds() {
    let mut rng = Rng(1);
    for _ in 0..300 {
        let sel = random_selector(&mut rng);
        let one = first_selector(&sel).specificity;
        // Adding a class to the last compound raises b by exactly one.
        let more = first_selector(&format!("{sel}.x")).specificity;
        assert_eq!(more.b, one.b + 1, "{sel}");
        assert_eq!((more.a, more.c), (one.a, one.c), "{sel}");
    }
}

#[test]
fn is_and_where_match_like_their_argument_list() {
    let mut rng = Rng(2);
    let (t, nodes) = random_tree(&mut rng);
    for _ in 0..200 {
        let arg = random_selector(&mut rng);
        let direct = first_selector(&arg);
        let via_is = first_selector(&format!(":is({arg})"));
        let via_where = first_selector(&format!(":where({arg})"));
        for node in &nodes {
            let mut budget = 10_000;
            let plain = matches_complex(&t, *node, &direct, &mut budget);
            let mut budget = 10_000;
            let is_match = matches_complex(&t, *node, &via_is, &mut budget);
            let mut budget = 10_000;
            let where_match = matches_complex(&t, *node, &via_where, &mut budget);
            assert_eq!(is_match, plain, "{arg} on {node:?}");
            assert_eq!(where_match, plain, "{arg} on {node:?}");
        }
    }
}

#[test]
fn restyling_after_random_changes_matches_a_full_resolve() {
    let mut rng = Rng(3);
    for case in 0..20 {
        let (mut t, nodes) = random_tree(&mut rng);
        let css: String = (0..4)
            .map(|_| {
                format!(
                    "{} {{ margin-left: {}px }}",
                    random_selector(&mut rng),
                    (rng.next() % 9) + 1
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let env = Environment::default();
        let mut resolver = StyleResolver::new(ResolverConfig::default());
        resolver.add_stylesheet(parse_sheet(&css).stylesheet, Origin::Author);
        let _ = resolver.resolve(&t, &env);
        for _ in 0..5 {
            let node = nodes[(rng.next() as usize) % nodes.len()];
            if node == t.root() {
                continue;
            }
            t.add_class(node, rng.pick(CLASSES));
            resolver.invalidate(&t, tpt_css::StyleChange::AttributeChanged(node));
            let restyled = resolver.restyle(&t, &env);
            let mut fresh = StyleResolver::new(ResolverConfig::default());
            fresh.add_stylesheet(parse_sheet(&css).stylesheet, Origin::Author);
            let full = fresh.resolve(&t, &env);
            assert_eq!(*restyled, full, "case {case}: incremental restyle diverged");
        }
    }
}

#[test]
fn custom_property_cycles_terminate_and_resolve_through_the_fallback() {
    let mut rng = Rng(4);
    for _ in 0..50 {
        // A ring of n custom properties, each defined from the next. The ring is
        // invalid, so margin uses its fallback.
        let n = 2 + (rng.next() % 6) as usize;
        let mut css = String::from("child {");
        for i in 0..n {
            css.push_str(&format!(" --p{i}: var(--p{}, 1px);", (i + 1) % n));
        }
        css.push_str(" margin-left: var(--p0, 4px) }");
        let mut t = TestTree::new(NodeKind::Element);
        let root = t.root();
        t.nodes[0].name.local = "root".into();
        let child = t.element(root, "child");
        let map = style_with(&css, &t);
        let margin = map
            .get(child)
            .unwrap()
            .raw(tpt_css::properties::lookup("margin-left").unwrap());
        // The ring resolves to its own fallback (1px) inside each member, so the
        // margin resolves to a length rather than being left invalid.
        assert!(
            matches!(margin, tpt_css::values::Value::Length { .. }),
            "margin resolves to a length, got {margin:?}"
        );
    }
}

#[test]
fn declaration_parsing_is_deterministic_and_serialization_round_trips() {
    let mut rng = Rng(5);
    for _ in 0..200 {
        let props = ["margin-left", "padding-top", "width", "color", "opacity"];
        let p = rng.pick(&props);
        let v = rng.next() % 50;
        let text = format!("{p}: {v}px");
        let a = parse_declaration_list(&text, ParseOptions::default());
        let b = parse_declaration_list(&text, ParseOptions::default());
        assert_eq!(a, b, "parsing is deterministic");
        let serialized = a
            .declarations
            .iter()
            .map(|d| {
                format!(
                    "{}: {}",
                    d.property,
                    tpt_css::style::custom::components_text(&d.value)
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let again = parse_declaration_list(&serialized, ParseOptions::default());
        assert_eq!(
            again.declarations, a.declarations,
            "serialize then parse is stable"
        );
    }
}

#[test]
fn seeded_hostile_text_does_not_panic_the_css_modules() {
    let mut rng = Rng(6);
    let alphabet = [
        ":",
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        "&",
        ".",
        "#",
        "@",
        "calc(",
        "var(",
        "--",
        "1",
        "px",
        " ",
        "has(",
        "scope",
        "to",
        "\"",
        "\\",
        "@layer ",
        "@supports ",
        "@property ",
    ];
    for _ in 0..800 {
        let len = (rng.next() % 40) as usize;
        let text: String = (0..len).map(|_| rng.pick(&alphabet)).collect();
        let parsed = tpt_css::parse_stylesheet(&text, ParseOptions::default());
        let mut resolver = StyleResolver::new(ResolverConfig::default());
        resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
        let t = TestTree::new(NodeKind::Element);
        let _ = resolver.resolve(&t, &Environment::default());
    }
}

/// A one-child tree and the child's computed value of `prop` under `css`.
fn px_of(css: &str, prop: &str) -> tpt_css::values::Value {
    use tpt_css::properties::lookup;
    let mut t = TestTree::new(NodeKind::Element);
    let root = t.root();
    t.nodes[0].name.local = "root".into();
    let child = t.element(root, "child");
    let map = style_with(css, &t);
    super::common::style_of(&map, child)
        .raw(lookup(prop).unwrap())
        .clone()
}

#[test]
fn box_shorthands_expand_to_the_four_sides_for_every_arity() {
    use tpt_css::values::Value;
    // CSS Box 4: one value sets all four sides; two set vertical and
    // horizontal; three set top, horizontal, and bottom; four set each side.
    const SIDES: [&str; 4] = ["margin-top", "margin-right", "margin-bottom", "margin-left"];
    let mut rng = Rng(0x5EED_0001);
    for _ in 0..200 {
        let n = 1 + (rng.next() % 4) as usize;
        let values: Vec<f64> = (0..n).map(|_| (rng.next() % 40) as f64).collect();
        let text = values
            .iter()
            .map(|v| format!("{v}px"))
            .collect::<Vec<_>>()
            .join(" ");
        // Map each side to the index of the value that supplies it.
        let pick = |top: usize, right: usize, bottom: usize, left: usize| {
            [top, right, bottom, left].map(|i| values[i])
        };
        let expected = match n {
            1 => pick(0, 0, 0, 0),
            2 => pick(0, 1, 0, 1),
            3 => pick(0, 1, 2, 1),
            _ => pick(0, 1, 2, 3),
        };
        let css = format!("child {{ margin: {text} }}");
        for (side, want) in SIDES.iter().zip(expected) {
            assert_eq!(
                px_of(&css, side),
                Value::px(want),
                "margin: {text} -> {side}"
            );
        }
    }
}

#[test]
fn calc_and_min_max_clamp_agree_with_their_definitions() {
    use tpt_css::values::Value;
    let mut rng = Rng(0x5EED_0002);
    let px = |v: f64| format!("{v}px");
    for _ in 0..200 {
        let a = (rng.next() % 41) as f64 - 20.0;
        let b = (rng.next() % 41) as f64 - 20.0;
        let c = (rng.next() % 41) as f64 - 20.0;
        let sum = px_of(
            &format!("child {{ margin-left: calc({} + {}) }}", px(a), px(b)),
            "margin-left",
        );
        assert_eq!(sum, Value::px(a + b), "calc sum of {a} and {b}");
        let lo = a.min(b);
        let hi = a.max(b);
        let min = px_of(
            &format!("child {{ margin-left: min({}, {}) }}", px(a), px(b)),
            "margin-left",
        );
        assert_eq!(min, Value::px(lo), "min({a}, {b})");
        let max = px_of(
            &format!("child {{ margin-left: max({}, {}) }}", px(a), px(b)),
            "margin-left",
        );
        assert_eq!(max, Value::px(hi), "max({a}, {b})");
        // CSS Values 4: clamp(lo, v, hi) is max(lo, min(v, hi)), so a lower
        // bound above the upper bound wins.
        let clamp = px_of(
            &format!(
                "child {{ margin-left: clamp({}, {}, {}) }}",
                px(a),
                px(b),
                px(c)
            ),
            "margin-left",
        );
        assert_eq!(clamp, Value::px(a.max(b.min(c))), "clamp({a}, {b}, {c})");
    }
}
