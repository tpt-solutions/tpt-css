// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! A static, web-like document (M1 acceptance): an html tree with body, nav,
//! main, and articles, styled by `fixtures/mit/web-like.css`.

use super::common::{style_of, style_with_env, TestTree};
use tpt_css::properties::lookup;
use tpt_css::selectors::PseudoElement;
use tpt_css::tree::{NodeId, NodeKind, StyleTree};
use tpt_css::values::{Color, Value};
use tpt_css::{Environment, MediaEnvironment, MediaKind, StyleMap, ViewportInfo};

const CSS: &str = include_str!("../../fixtures/mit/web-like.css");

struct Doc {
    tree: TestTree,
    body: NodeId,
    nav_links: [NodeId; 2],
    articles: [NodeId; 3],
    /// Paragraphs of the first article: text, text, empty.
    first_article_paragraphs: [NodeId; 3],
}

fn document() -> Doc {
    let mut t = TestTree::new(NodeKind::Element);
    let html = t.root();
    t.nodes[0].name.local = "html".into();
    let body = t.element(html, "body");
    let nav = t.element(body, "nav");
    let a1 = t.element(nav, "a");
    let a2 = t.element(nav, "a");
    let main = t.element(body, "main");
    let articles = [
        t.element(main, "article"),
        t.element(main, "article"),
        t.element(main, "article"),
    ];
    let p1 = t.element(articles[0], "p");
    t.add_text_with(p1, "one");
    let p2 = t.element(articles[0], "p");
    t.add_text_with(p2, "two");
    let p3 = t.element(articles[0], "p");
    Doc {
        tree: t,
        body,
        nav_links: [a1, a2],
        articles,
        first_article_paragraphs: [p1, p2, p3],
    }
}

fn styled(doc: &Doc, env: &Environment) -> StyleMap {
    style_with_env(CSS, &doc.tree, env)
}

fn value(map: &StyleMap, node: NodeId, name: &str) -> Value {
    style_of(map, node)
        .raw(lookup(name).expect("known property"))
        .clone()
}

#[test]
fn custom_properties_and_inheritance_on_the_document() {
    let doc = document();
    let map = styled(&doc, &Environment::default());
    let [a1, _] = doc.nav_links;
    // The accent is declared on :root and inherited to the links.
    assert_eq!(
        map.custom_property(a1, "--accent").map(|v| v.text()),
        Some("#1a73e8".to_string())
    );
    assert_eq!(
        style_of(&map, a1).color(),
        Color::rgb8(0x1a, 0x73, 0xe8),
        "nav > a uses var(--accent)"
    );
    // body sets color directly, and margin 0.
    assert_eq!(
        style_of(&map, doc.body).color(),
        Color::rgb8(0x22, 0x22, 0x22)
    );
    assert_eq!(value(&map, doc.body, "margin-top"), Value::px(0.0));
}

#[test]
fn calc_and_var_in_padding_and_margin() {
    let doc = document();
    let map = styled(&doc, &Environment::default());
    let [a1, _] = doc.nav_links;
    assert_eq!(value(&map, a1, "padding-top"), Value::px(8.0));
    assert_eq!(
        value(&map, a1, "padding-left"),
        Value::px(16.0),
        "calc(2 * var(--gap))"
    );
    let [_, p2, _] = doc.first_article_paragraphs;
    assert_eq!(
        value(&map, p2, "margin-top"),
        Value::px(4.0),
        "calc(var(--gap) / 2)"
    );
}

#[test]
fn structural_and_sibling_selectors() {
    let doc = document();
    let map = styled(&doc, &Environment::default());
    let [a1, a2] = doc.nav_links;
    assert_eq!(
        value(&map, a1, "font-weight"),
        Value::Number(700.0),
        "first-child"
    );
    assert_eq!(value(&map, a2, "font-weight"), Value::Number(400.0));
    // Articles 1 and 3 are odd, so they get the striped background.
    let stripe = Value::Color(Color::rgb8(0xf5, 0xf5, 0xf5));
    assert_eq!(value(&map, doc.articles[0], "background-color"), stripe);
    assert_eq!(
        value(&map, doc.articles[1], "background-color"),
        Value::Color(Color::TRANSPARENT)
    );
    assert_eq!(value(&map, doc.articles[2], "background-color"), stripe);
    // The first paragraph has no sibling before it, so no top margin.
    let [p1, p2, p3] = doc.first_article_paragraphs;
    assert_eq!(value(&map, p1, "margin-top"), Value::px(0.0));
    assert_eq!(value(&map, p2, "margin-top"), Value::px(4.0), "p + p");
    assert_eq!(value(&map, p3, "display"), Value::Keyword("none"), ":empty");
    assert_eq!(value(&map, p1, "display"), Value::Keyword("inline"));
}

#[test]
fn pseudo_element_is_generated_from_content() {
    let doc = document();
    let map = styled(&doc, &Environment::default());
    for article in doc.articles {
        let before = map
            .get_pseudo(article, &PseudoElement::Before)
            .expect("article::before is generated");
        assert_eq!(
            *before.raw(lookup("content").unwrap()),
            Value::Strings(vec!["§".to_string()])
        );
        assert_eq!(before.color(), Color::rgb8(0x1a, 0x73, 0xe8));
    }
}

#[test]
fn a_narrow_viewport_applies_the_max_width_rule() {
    let doc = document();
    let narrow = Environment {
        viewport: ViewportInfo {
            width: 500.0,
            height: 800.0,
        },
        ..Environment::default()
    };
    let map = styled(&doc, &narrow);
    let [a1, _] = doc.nav_links;
    assert_eq!(value(&map, a1, "padding-top"), Value::px(4.0));
    // The rule uses the padding shorthand, which sets all four sides, so the
    // later rule replaces padding-left as well.
    assert_eq!(value(&map, a1, "padding-left"), Value::px(4.0));
}

#[test]
fn print_media_hides_the_nav() {
    let doc = document();
    let nav = doc.tree.parent(doc.nav_links[0]).expect("nav");
    let screen_map = styled(&doc, &Environment::default());
    assert_eq!(value(&screen_map, nav, "display"), Value::Keyword("inline"));
    let print = Environment {
        media: MediaEnvironment {
            kind: MediaKind::Print,
            ..MediaEnvironment::default()
        },
        ..Environment::default()
    };
    let print_map = styled(&doc, &print);
    assert_eq!(value(&print_map, nav, "display"), Value::Keyword("none"));
}
