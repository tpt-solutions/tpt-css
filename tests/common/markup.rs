// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! A small markup reader for tests. It builds a [`TestTree`] from a practical
//! subset of HTML: elements with attributes, void elements, text, comments, a
//! doctype, and the raw text of `<style>`. It is not an HTML parser. The engine
//! has no parser (the HTML front end is a separate crate, see DESIGN.md), so
//! this exists only to give the web profile an end-to-end test from markup to
//! computed styles.

use super::TestTree;
use tpt_css::tree::NodeKind;

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "wbr",
];

/// The result of reading a document: the tree and the text of each `<style>`
/// element, in document order.
pub struct Markup {
    pub tree: TestTree,
    pub style_sheets: Vec<String>,
}

/// Reads `source` into a document tree. Tag and attribute names are lowercased.
/// Whitespace-only text between tags is dropped. Unmatched end tags are ignored.
pub fn read(source: &str) -> Markup {
    let mut tree = TestTree::new(NodeKind::Document);
    let mut style_sheets = Vec::new();
    let mut stack = vec![tree.root()];
    let mut rest = source;

    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |i| &after[i + 3..]);
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            rest = rest.find('>').map_or("", |i| &rest[i + 1..]);
        } else if let Some(after) = rest.strip_prefix("</") {
            let end = after.find('>').unwrap_or(after.len());
            let name = after[..end].trim().to_ascii_lowercase();
            rest = after.get(end + 1..).unwrap_or("");
            // Close the nearest open element with this name, if there is one.
            if let Some(pos) = stack
                .iter()
                .rposition(|&n| n != tree.root() && tree.nodes[n.index as usize].name.local == name)
            {
                stack.truncate(pos);
            }
        } else if rest.starts_with('<') && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic())
        {
            let (name, attrs, after, self_closing) = read_start_tag(&rest[1..]);
            rest = after;
            let parent = *stack.last().expect("the root is always on the stack");
            let node = tree.element(parent, &name);
            apply_attributes(&mut tree, node, &attrs);
            if name == "style" {
                let close = rest.to_ascii_lowercase().find("</style");
                let body = close.map_or(rest, |i| &rest[..i]);
                style_sheets.push(body.to_string());
                rest = close.map_or("", |i| {
                    rest[i..].find('>').map_or("", |j| &rest[i + j + 1..])
                });
                continue;
            }
            if !self_closing && !VOID.contains(&name.as_str()) {
                stack.push(node);
            }
        } else {
            // Text runs until the next tag.
            // Skip the first character, which may be a `<` that is not a tag.
            let first = rest.chars().next().map_or(1, char::len_utf8);
            let end = rest[first..].find('<').map_or(rest.len(), |i| i + first);
            let text = &rest[..end];
            rest = &rest[end..];
            if !text.trim().is_empty() {
                let parent = *stack.last().expect("the root is always on the stack");
                tree.add_text_with(parent, text.trim());
            }
        }
    }
    Markup { tree, style_sheets }
}

/// Reads a start tag after `<`. Returns the lowercased name, the attributes, the
/// input after `>`, and whether the tag ends with `/>`.
fn read_start_tag(input: &str) -> (String, Vec<(String, String)>, &str, bool) {
    let mut name = String::new();
    for c in input.chars() {
        if c.is_ascii_alphanumeric() || c == '-' {
            name.push(c.to_ascii_lowercase());
        } else {
            break;
        }
    }
    let mut pos = name.len();
    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        let bytes = input.as_bytes();
        while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            return (name, attrs, "", self_closing);
        }
        match bytes[pos] {
            b'>' => return (name, attrs, &input[pos + 1..], self_closing),
            b'/' => {
                self_closing = true;
                pos += 1;
            }
            _ => {
                let start = pos;
                while pos < bytes.len()
                    && !(bytes[pos] as char).is_ascii_whitespace()
                    && !matches!(bytes[pos], b'=' | b'>' | b'/')
                {
                    pos += 1;
                }
                let attr_name = input[start..pos].to_ascii_lowercase();
                while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                    pos += 1;
                }
                let mut value = String::new();
                if pos < bytes.len() && bytes[pos] == b'=' {
                    pos += 1;
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                        pos += 1;
                    }
                    if pos < bytes.len() && matches!(bytes[pos], b'"' | b'\'') {
                        let quote = bytes[pos];
                        pos += 1;
                        let start = pos;
                        while pos < bytes.len() && bytes[pos] != quote {
                            pos += 1;
                        }
                        value = input[start..pos].to_string();
                        pos = (pos + 1).min(bytes.len());
                    } else {
                        let start = pos;
                        while pos < bytes.len()
                            && !(bytes[pos] as char).is_ascii_whitespace()
                            && bytes[pos] != b'>'
                        {
                            pos += 1;
                        }
                        value = input[start..pos].to_string();
                    }
                }
                if !attr_name.is_empty() {
                    attrs.push((attr_name, value));
                }
            }
        }
    }
}

fn apply_attributes(tree: &mut TestTree, node: tpt_css::tree::NodeId, attrs: &[(String, String)]) {
    for (name, value) in attrs {
        match name.as_str() {
            "id" => {
                tree.set_id(node, value);
            }
            "class" => {
                for class in value.split_ascii_whitespace() {
                    tree.add_class(node, class);
                }
            }
            "style" => {
                tree.set_inline(node, value);
            }
            _ => {
                tree.set_attr(node, name, value);
            }
        }
    }
}
