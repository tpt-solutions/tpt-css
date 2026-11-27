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
            let full = fresh.resolve(&t, &env).clone();
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

#[test]
fn padding_shorthand_expands_to_the_four_sides_for_every_arity() {
    use tpt_css::values::Value;
    // Same arity rules as margin (CSS Box 4), checked for padding.
    const SIDES: [&str; 4] = [
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
    ];
    let mut rng = Rng(0x5EED_0003);
    for _ in 0..200 {
        let n = 1 + (rng.next() % 4) as usize;
        let values: Vec<f64> = (0..n).map(|_| (rng.next() % 40) as f64).collect();
        let text = values
            .iter()
            .map(|v| format!("{v}px"))
            .collect::<Vec<_>>()
            .join(" ");
        let pick = |top: usize, right: usize, bottom: usize, left: usize| {
            [top, right, bottom, left].map(|i| values[i])
        };
        let expected = match n {
            1 => pick(0, 0, 0, 0),
            2 => pick(0, 1, 0, 1),
            3 => pick(0, 1, 2, 1),
            _ => pick(0, 1, 2, 3),
        };
        let css = format!("child {{ padding: {text} }}");
        for (side, want) in SIDES.iter().zip(expected) {
            assert_eq!(
                px_of(&css, side),
                Value::px(want),
                "padding: {text} -> {side}"
            );
        }
    }
}

/// The number of a computed length in px, if it is one.
fn px_number(value: &tpt_css::values::Value) -> Option<f64> {
    match value {
        tpt_css::values::Value::Length { value, .. } => Some(*value),
        _ => None,
    }
}

#[test]
fn rounding_and_modulo_functions_match_their_definitions() {
    use tpt_css::values::Value;
    let mut rng = Rng(0x5EED_0004);
    for _ in 0..200 {
        // Integers keep every result exact, so the checks compare with ==.
        let a = (rng.next() % 101) as f64 - 50.0;
        let mut b = (rng.next() % 24) as f64 - 12.0;
        if b == 0.0 {
            b = 7.0;
        }
        // round(nearest, a, b) rounds halfway toward +infinity (CSS Values 4).
        let nearest = (a / b + 0.5).floor() * b;
        let got = px_of(
            &format!("child {{ margin-left: round(nearest, {a}px, {b}px) }}"),
            "margin-left",
        );
        assert_eq!(got, Value::px(nearest), "round(nearest, {a}, {b})");
        // mod(a, b) has the sign of b; rem(a, b) has the sign of a.
        let modulo = a - b * (a / b).floor();
        let got = px_of(
            &format!("child {{ margin-left: mod({a}px, {b}px) }}"),
            "margin-left",
        );
        assert_eq!(got, Value::px(modulo), "mod({a}, {b})");
        let remainder = a % b;
        let got = px_of(
            &format!("child {{ margin-left: rem({a}px, {b}px) }}"),
            "margin-left",
        );
        assert_eq!(got, Value::px(remainder), "rem({a}, {b})");
        // hypot(a, b) is the Euclidean length, within float error.
        let c = (rng.next() % 61) as f64 - 30.0;
        let got = px_of(
            &format!("child {{ margin-left: hypot({a}px, {c}px) }}"),
            "margin-left",
        );
        let want = (a * a + c * c).sqrt();
        let near = px_number(&got).is_some_and(|v| (v - want).abs() < 1e-9);
        assert!(near, "hypot({a}, {c}) = {got:?}, want {want}");
    }
}

/// An environment with random viewport, root font size, color scheme, and hover.
fn random_env(rng: &mut Rng) -> Environment {
    let mut env = Environment::default();
    env.viewport.width = [300.0, 500.0, 800.0][(rng.next() as usize) % 3];
    env.root_font_size = [12.0, 16.0, 20.0][(rng.next() as usize) % 3];
    env.media.prefers_dark = rng.next() % 2 == 0;
    env.media.hover = rng.next() % 2 == 0;
    env
}

#[test]
fn environment_changes_match_a_full_resolve() {
    // Media queries, viewport and relative units, the root font size, and the
    // color scheme all change between restyles. Each restyle must match a full
    // resolve in the same environment.
    const DECLS: &[&str] = &[
        "margin-left: 10vw",
        "margin-left: 2em",
        "margin-left: 1rem",
        "color: light-dark(red, blue)",
        "margin-left: 3px",
        "font-size: 2em",
        "width: 50%",
        "padding-left: 1vmin",
    ];
    const WRAPS: &[(&str, &str)] = &[
        ("", ""),
        ("@media (min-width: 500px) {", "}"),
        ("@media (prefers-color-scheme: dark) {", "}"),
        ("@media (hover: hover) {", "}"),
        ("@media (max-width: 300px) {", "}"),
    ];
    let mut rng = Rng(0x5EED_0005);
    for case in 0..30 {
        let (t, _) = random_tree(&mut rng);
        let css: String = (0..6)
            .map(|_| {
                let (open, close) = WRAPS[(rng.next() as usize) % WRAPS.len()];
                let decl = DECLS[(rng.next() as usize) % DECLS.len()];
                format!("{open} {} {{ {decl} }} {close}", random_selector(&mut rng))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut resolver = StyleResolver::new(ResolverConfig::default());
        resolver.add_stylesheet(parse_sheet(&css).stylesheet, Origin::Author);
        let _ = resolver.resolve(&t, &random_env(&mut rng));
        for step in 0..8 {
            let env = random_env(&mut rng);
            let restyled = resolver.restyle(&t, &env).clone();
            let mut fresh = StyleResolver::new(ResolverConfig::default());
            fresh.add_stylesheet(parse_sheet(&css).stylesheet, Origin::Author);
            let full = fresh.resolve(&t, &env).clone();
            assert_eq!(
                restyled, full,
                "case {case} step {step}: environment restyle diverged\n{css}"
            );
        }
    }
}

#[test]
fn border_radius_expands_to_the_four_corners_for_every_arity() {
    use tpt_css::values::Value;
    // CSS Backgrounds 3: one value sets all corners; two set top-left and
    // bottom-right, then top-right and bottom-left; three set top-left, then
    // top-right and bottom-left, then bottom-right; four set each corner.
    const CORNERS: [&str; 4] = [
        "border-top-left-radius",
        "border-top-right-radius",
        "border-bottom-right-radius",
        "border-bottom-left-radius",
    ];
    let mut rng = Rng(0x5EED_0006);
    for _ in 0..200 {
        let n = 1 + (rng.next() % 4) as usize;
        let values: Vec<f64> = (0..n).map(|_| (rng.next() % 40) as f64).collect();
        let text = values
            .iter()
            .map(|v| format!("{v}px"))
            .collect::<Vec<_>>()
            .join(" ");
        let pick = |tl: usize, tr: usize, br: usize, bl: usize| [tl, tr, br, bl].map(|i| values[i]);
        let expected = match n {
            1 => pick(0, 0, 0, 0),
            2 => pick(0, 1, 0, 1),
            3 => pick(0, 1, 2, 1),
            _ => pick(0, 1, 2, 3),
        };
        let css = format!("child {{ border-radius: {text} }}");
        for (corner, want) in CORNERS.iter().zip(expected) {
            assert_eq!(
                px_of(&css, corner),
                Value::px(want),
                "border-radius: {text} -> {corner}"
            );
        }
    }
}

/// The six fuzz targets in `fuzz/fuzz_targets` (libFuzzer, Linux, nightly) run
/// here as a seeded smoke test on stable Rust, so every platform runs them. It is
/// not a coverage-guided fuzzer, but it exercises the same entry points.
#[test]
fn the_fuzz_target_bodies_do_not_panic_on_seeded_inputs() {
    use tpt_css::parse_stylesheet;
    use tpt_css::syntax::{tokenize, SourceId};
    use tpt_css::ParseLimits;
    let mut rng = Rng(0x5EED_F022);
    let tokens = [
        "calc(",
        "min(",
        "clamp(",
        "var(",
        "--a",
        ":",
        ";",
        "{",
        "}",
        "(",
        ")",
        "[",
        "]",
        ",",
        "/",
        "*",
        "1e3",
        "-",
        "+",
        "%",
        "px",
        "deg",
        "s",
        "#fff",
        "#ffff",
        "rgb(",
        "hsl(",
        "color-mix(in ",
        "oklch(",
        "light-dark(",
        "box-shadow: ",
        "transform: ",
        "font-variant: ",
        "small-caps ",
        "tabular-nums ",
        "stylistic(a) ",
        "transition: ",
        "rotate(",
        "translate(",
        "inset ",
        "red ",
        "\"",
        "\\",
        "@media ",
        "@supports ",
        "@layer ",
        ":has(",
        ":is(",
        "::before",
        "&",
        " ",
        "\n",
        "url(",
        "image-set(",
        "display-p3",
        "rec2020",
        " ",
        "~",
        ">",
        "+",
        "|",
        "@scope ",
        "to ",
    ];
    for case in 0..4000 {
        // Raw bytes, as libFuzzer would give them. Only UTF-8 reaches the parsers.
        let len = (rng.next() % 96) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next() % 256) as u8).collect();
        // Structured text, built from the tokens above.
        let parts = (rng.next() % 24) as usize;
        let mut text = String::new();
        for _ in 0..parts {
            text.push_str(rng.pick(&tokens));
        }
        for input in [String::from_utf8_lossy(&bytes).into_owned(), text.clone()] {
            let _ = parse_declaration_list(
                &format!("margin-left: calc({input})"),
                ParseOptions::default(),
            );
            let _ = parse_declaration_list(&format!("color: {input}"), ParseOptions::default());
            let _ = parse_declaration_list(&input, ParseOptions::default());
            let _ =
                parse_declaration_list(&format!("box-shadow: {input}"), ParseOptions::default());
            let _ = parse_declaration_list(&format!("transform: {input}"), ParseOptions::default());
            let _ =
                parse_declaration_list(&format!("font-variant: {input}"), ParseOptions::default());
            let _ = parse_stylesheet(&format!("{input} {{ }}"), ParseOptions::default());
            let _ = parse_stylesheet(&input, ParseOptions::default());
            let _ = tokenize(SourceId(0), &input, &ParseLimits::default());
        }
        if case % 500 == 0 {
            // A stylesheet from the same material also goes through resolution.
            let mut resolver = StyleResolver::new(ResolverConfig::default());
            resolver.add_stylesheet(
                parse_stylesheet(&text, ParseOptions::default()).stylesheet,
                Origin::Author,
            );
            let t = TestTree::new(NodeKind::Element);
            let _ = resolver.resolve(&t, &Environment::default());
        }
    }
}

#[test]
fn flex_shorthand_follows_the_factor_and_basis_rules_for_seeded_forms() {
    // CSS Flexbox 1 section 7.2: `[ grow shrink? || basis ]`. Each form fixes the
    // three longhands; the generator picks the form and the numbers.
    use tpt_css::values::Value;
    let mut rng = Rng(0x5EED_0007);
    for _ in 0..200 {
        let g = (rng.next() % 6) as f64;
        let s = (rng.next() % 6) as f64;
        let b = (rng.next() % 40) as f64;
        let (text, grow, shrink, basis) = match rng.next() % 5 {
            0 => (format!("{g}"), g, 1.0, Value::Percentage(0.0)),
            1 => (format!("{g} {s}"), g, s, Value::Percentage(0.0)),
            2 => (format!("{g} {s} {b}px"), g, s, Value::px(b)),
            3 => (format!("{g} {b}px"), g, 1.0, Value::px(b)),
            _ => (format!("{b}px"), 1.0, 1.0, Value::px(b)),
        };
        let css = format!("child {{ flex: {text} }}");
        assert_eq!(
            px_of(&css, "flex-grow"),
            Value::Number(grow),
            "flex: {text}"
        );
        assert_eq!(
            px_of(&css, "flex-shrink"),
            Value::Number(shrink),
            "flex: {text}"
        );
        assert_eq!(px_of(&css, "flex-basis"), basis, "flex: {text}");
    }
}

#[test]
fn transition_layers_keep_their_order_and_defaults_for_seeded_layers() {
    // Each layer is `[property] [duration [easing] [delay]]`, where a delay needs
    // a duration. Omitted parts take their initial values. The lists keep the
    // layer order.
    use tpt_css::values::{TimeUnit, Value};
    const PROPS: &[&str] = &["opacity", "transform", "color", "all"];
    const EASINGS: &[&str] = &["ease", "linear", "ease-in", "ease-out", "ease-in-out"];
    let mut rng = Rng(0x5EED_0008);
    for _ in 0..200 {
        let layers = 1 + (rng.next() % 3) as usize;
        let (mut text, mut props, mut durations, mut easings, mut delays) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for _ in 0..layers {
            let mut parts = Vec::new();
            let prop = (rng.next() % 2 == 0).then(|| rng.pick(PROPS));
            if let Some(p) = prop {
                parts.push(p.to_string());
            }
            props.push(match prop {
                Some(p) if p != "all" => Value::Raw(p.to_string()),
                _ => Value::Keyword("all"),
            });
            let duration = (rng.next() % 2 == 0).then(|| 1 + rng.next() % 5);
            match duration {
                Some(d) => {
                    parts.push(format!("{d}s"));
                    durations.push(Value::Time {
                        value: d as f64,
                        unit: TimeUnit::S,
                    });
                }
                None => durations.push(Value::Time {
                    value: 0.0,
                    unit: TimeUnit::S,
                }),
            }
            let easing = (rng.next() % 2 == 0).then(|| rng.pick(EASINGS));
            if let Some(e) = easing {
                parts.push(e.to_string());
            }
            easings.push(Value::Keyword(easing.unwrap_or("ease")));
            // A delay in quarter seconds, so the value is exact after conversion.
            let delay = (duration.is_some() && rng.next() % 2 == 0).then(|| rng.next() % 4);
            match delay {
                Some(k) => {
                    parts.push(format!("{}ms", k * 250));
                    delays.push(Value::Time {
                        value: (k * 250) as f64 / 1000.0,
                        unit: TimeUnit::S,
                    });
                }
                None => delays.push(Value::Time {
                    value: 0.0,
                    unit: TimeUnit::S,
                }),
            }
            // A layer needs at least one part. `all` is what an omitted property means.
            if parts.is_empty() {
                parts.push("all".to_string());
            }
            text.push(parts.join(" "));
        }
        let css = format!("child {{ transition: {} }}", text.join(", "));
        assert_eq!(
            px_of(&css, "transition-property"),
            Value::List(props),
            "{css}"
        );
        assert_eq!(
            px_of(&css, "transition-duration"),
            Value::List(durations),
            "{css}"
        );
        assert_eq!(
            px_of(&css, "transition-timing-function"),
            Value::List(easings),
            "{css}"
        );
        assert_eq!(
            px_of(&css, "transition-delay"),
            Value::List(delays),
            "{css}"
        );
    }
}

#[test]
fn list_style_fills_each_longhand_from_its_own_keyword_in_any_order() {
    // `list-style: [type || position]` in either order. An omitted type is `disc`,
    // an omitted position is `outside`, and the image is `none`.
    use tpt_css::values::Value;
    const TYPES: &[&str] = &["disc", "circle", "square", "decimal"];
    const POSITIONS: &[&str] = &["inside", "outside"];
    let mut rng = Rng(0x5EED_0009);
    for _ in 0..200 {
        let ty = (rng.next() % 2 == 0).then(|| rng.pick(TYPES));
        let pos = (rng.next() % 2 == 0).then(|| rng.pick(POSITIONS));
        let mut words: Vec<&str> = ty.into_iter().chain(pos).collect();
        if words.is_empty() {
            continue;
        }
        if words.len() == 2 && rng.next() % 2 == 0 {
            words.swap(0, 1);
        }
        let css = format!("child {{ list-style: {} }}", words.join(" "));
        assert_eq!(
            px_of(&css, "list-style-type"),
            Value::Keyword(ty.unwrap_or("disc")),
            "{css}"
        );
        assert_eq!(
            px_of(&css, "list-style-position"),
            Value::Keyword(pos.unwrap_or("outside")),
            "{css}"
        );
        assert_eq!(
            px_of(&css, "list-style-image"),
            Value::Keyword("none"),
            "{css}"
        );
    }
}

/// The unitless value of a math expression, checked as `calc(<expr> * 1px)`.
fn unitless(expr: &str) -> Option<f64> {
    px_number(&px_of(
        &format!("child {{ margin-left: calc({expr} * 1px) }}"),
        "margin-left",
    ))
}

#[test]
fn font_shorthand_fills_each_longhand_for_seeded_forms() {
    // CSS Fonts 4 section 4.3: `[style] [weight] size[/line-height] family`.
    // An omitted style is `normal`, weight is 400, and line height is `normal`.
    use tpt_css::values::Value;
    const FAMILIES: &[&str] = &["serif", "sans-serif", "monospace"];
    let mut rng = Rng(0x5EED_000A);
    for _ in 0..200 {
        let italic = rng.next() % 2 == 0;
        let (weight_text, weight_want) = match rng.next() % 3 {
            0 => (None, Value::Number(400.0)),
            // `bold` computes to 700 (CSS Fonts 4 section 3.2).
            1 => (Some("bold".to_string()), Value::Number(700.0)),
            _ => {
                let n = 100 * (1 + rng.next() % 9);
                (Some(n.to_string()), Value::Number(n as f64))
            }
        };
        let size = 10 + rng.next() % 31;
        let line = (rng.next() % 2 == 0).then(|| 12 + rng.next() % 20);
        let family = rng.pick(FAMILIES);

        let mut parts = Vec::new();
        if italic {
            parts.push("italic".to_string());
        }
        if let Some(w) = &weight_text {
            parts.push(w.clone());
        }
        let mut size_text = format!("{size}px");
        if let Some(l) = line {
            size_text.push_str(&format!("/{l}px"));
        }
        parts.push(size_text);
        parts.push(family.to_string());
        let css = format!("child {{ font: {} }}", parts.join(" "));

        let style = if italic { "italic" } else { "normal" };
        assert_eq!(px_of(&css, "font-style"), Value::Keyword(style), "{css}");
        assert_eq!(px_of(&css, "font-weight"), weight_want, "{css}");
        assert_eq!(px_of(&css, "font-size"), Value::px(size as f64), "{css}");
        let line_want = match line {
            Some(l) => Value::px(l as f64),
            None => Value::Keyword("normal"),
        };
        assert_eq!(px_of(&css, "line-height"), line_want, "{css}");
        assert_eq!(
            px_of(&css, "font-family"),
            Value::FontFamily(vec![family.to_string()]),
            "{css}"
        );
    }
}

#[test]
fn background_shorthand_sets_color_and_image_in_either_order() {
    // `background` takes one color and one image (`none` here), in any order.
    // An omitted color is `transparent`, and an omitted image is `none`.
    use tpt_css::values::Value;
    const COLORS: &[&str] = &["red", "blue", "transparent"];
    let mut rng = Rng(0x5EED_000B);
    for _ in 0..200 {
        let color = (rng.next() % 2 == 0).then(|| rng.pick(COLORS));
        let image = rng.next() % 2 == 0;
        let mut words: Vec<&str> = color.into_iter().collect();
        if image {
            words.push("none");
        }
        if words.is_empty() {
            continue;
        }
        if words.len() == 2 && rng.next() % 2 == 0 {
            words.swap(0, 1);
        }
        let css = format!("child {{ background: {} }}", words.join(" "));
        let want = px_of(
            &format!(
                "child {{ background-color: {} }}",
                color.unwrap_or("transparent")
            ),
            "background-color",
        );
        assert_eq!(px_of(&css, "background-color"), want, "{css}");
        assert_eq!(
            px_of(&css, "background-image"),
            Value::Keyword("none"),
            "{css}"
        );
    }
}

#[test]
fn text_decoration_line_stores_every_subset_in_spec_order() {
    // Any non-empty set of line keywords, in any order, is stored once each in the
    // fixed order `underline overline line-through blink`.
    use tpt_css::values::Value;
    const LINES: &[&str] = &["underline", "overline", "line-through", "blink"];
    let mut rng = Rng(0x5EED_000C);
    for _ in 0..200 {
        let mut chosen: Vec<&str> = LINES
            .iter()
            .copied()
            .filter(|_| rng.next() % 2 == 0)
            .collect();
        if chosen.is_empty() {
            continue;
        }
        for i in (1..chosen.len()).rev() {
            let j = (rng.next() % (i as u64 + 1)) as usize;
            chosen.swap(i, j);
        }
        let css = format!("child {{ text-decoration-line: {} }}", chosen.join(" "));
        let want: Vec<&str> = LINES
            .iter()
            .copied()
            .filter(|l| chosen.contains(l))
            .collect();
        match px_of(&css, "text-decoration-line") {
            Value::Keyword(stored) => {
                assert_eq!(stored.split(' ').collect::<Vec<_>>(), want, "{css}")
            }
            other => panic!("{css}: unexpected {other:?}"),
        }
    }
}

#[test]
fn math_functions_match_their_definitions_on_seeded_inputs() {
    // The unitless functions, checked inside `calc(... * 1px)`. Tolerances cover
    // the float error of the transcendental functions only.
    use tpt_css::values::Value;
    let near = |got: Option<f64>, want: f64, what: &str| {
        let ok = got.is_some_and(|v| (v - want).abs() < 1e-9);
        assert!(ok, "{what} = {got:?}, want {want}");
    };
    let mut rng = Rng(0x5EED_000D);
    for _ in 0..200 {
        let a = (rng.next() % 41) as f64 - 20.0;
        let x = (rng.next() % 5) as f64 + 1.0;
        let y = (rng.next() % 5) as f64;
        let r = (rng.next() % 300) as f64 / 100.0 - 1.5;
        let s = (rng.next() % 201) as f64 / 100.0 - 1.0;

        // abs and sign of a length.
        assert_eq!(
            px_of(
                &format!("child {{ margin-left: abs({a}px) }}"),
                "margin-left"
            ),
            Value::px(a.abs()),
            "abs({a}px)"
        );
        let sign = if a > 0.0 {
            1.0
        } else if a < 0.0 {
            -1.0
        } else {
            0.0
        };
        near(
            unitless(&format!("sign({a}px) * 10")),
            sign * 10.0,
            &format!("sign({a}px)"),
        );

        near(
            unitless(&format!("sqrt({x})")),
            x.sqrt(),
            &format!("sqrt({x})"),
        );
        near(
            unitless(&format!("pow({x}, {y})")),
            x.powf(y),
            &format!("pow({x}, {y})"),
        );
        near(
            unitless(&format!("exp({r})")),
            r.exp(),
            &format!("exp({r})"),
        );
        near(unitless(&format!("log({x})")), x.ln(), &format!("log({x})"));
        near(
            unitless(&format!("sin({r})")),
            r.sin(),
            &format!("sin({r})"),
        );
        near(
            unitless(&format!("cos({r})")),
            r.cos(),
            &format!("cos({r})"),
        );
        near(
            unitless(&format!("tan({r})")),
            r.tan(),
            &format!("tan({r})"),
        );
        near(
            unitless(&format!("asin({s})")),
            s.asin(),
            &format!("asin({s})"),
        );
        near(
            unitless(&format!("acos({s})")),
            s.acos(),
            &format!("acos({s})"),
        );
        near(
            unitless(&format!("atan({r})")),
            r.atan(),
            &format!("atan({r})"),
        );
        near(
            unitless(&format!("atan2({a}, {x})")),
            (a).atan2(x),
            &format!("atan2({a}, {x})"),
        );
    }
}
