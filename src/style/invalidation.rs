// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Invalidation and incremental restyle (spec 8.2, 17).
//!
//! This is the M1 foundation. It covers attribute, state, and inline-style
//! changes, which restyle the changed element and its subtree. Stylesheet
//! changes, environment changes, and tree-shape changes restyle everything.
//!
//! Sibling-sensitive rules are handled conservatively. If any rule depends on
//! sibling structure (`+`, `~`, `:nth-*`, `:empty`), a change also restyles the
//! parent's subtree, which covers every sibling. A dependency index that keeps
//! the restyle to the affected siblings only is M3 work.
//!
//! The change must refer to a node that is still in the tree. Node insertion
//! and removal are reported as [`StyleChange::TreeChanged`], which restyles
//! everything.

use std::collections::BTreeSet;

use crate::ast::selector::{ComplexSelector, SelectorList, SimpleSelector};
use crate::env::Environment;
use crate::style::computed::StyleMap;
use crate::style::resolver::{style_tree, Frame, StyleResolver};
use crate::tree::{NodeId, NodeKind, StyleTree};

/// A change that may require restyling (spec 17.1).
///
/// Not `Copy`, because [`StyleChange::AttributeNamed`] owns its attribute name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleChange {
    /// An attribute of the element changed, and its name is not known. Always
    /// treated as relevant. Prefer [`StyleChange::AttributeNamed`].
    AttributeChanged(NodeId),
    /// An attribute of the element changed. If no selector reads that attribute
    /// and style does not read it directly, nothing is restyled.
    AttributeNamed(NodeId, String),
    /// The element's state bits or custom states changed.
    StateChanged(NodeId),
    /// The element's inline `style` changed.
    InlineStyleChanged(NodeId),
    /// A stylesheet was added or removed. Restyles everything.
    StyleSheetChanged,
    /// Nodes were inserted, removed, or moved. Restyles everything.
    TreeChanged,
}

/// Attribute names that style reads without a selector: `style` is the inline
/// style, and `lang` and `dir` are read through the tree. A change to any of
/// them is always relevant.
const ALWAYS_RELEVANT: [&str; 3] = ["style", "lang", "dir"];

impl StyleResolver {
    /// Records a change, so that the next [`StyleResolver::restyle`] recomputes
    /// what it affects (spec 8.2).
    pub fn invalidate<T: StyleTree>(&mut self, tree: &T, change: StyleChange) {
        match change {
            StyleChange::AttributeChanged(node)
            | StyleChange::StateChanged(node)
            | StyleChange::InlineStyleChanged(node) => self.invalidate_element(tree, node),
            StyleChange::AttributeNamed(node, name) => {
                if self.attribute_is_read(&name) {
                    self.invalidate_element(tree, node);
                }
            }
            StyleChange::StyleSheetChanged | StyleChange::TreeChanged => {
                self.full_dirty = true;
            }
        }
    }

    /// Marks one element dirty, and whatever its selectors can reach (spec 12.4).
    fn invalidate_element<T: StyleTree>(&mut self, tree: &T, node: NodeId) {
        self.dirty.insert(node);
        let structural = self
            .sheets
            .iter()
            .any(|s| s.rules.iter().any(|r| r.structural));
        if self
            .sheets
            .iter()
            .any(|s| s.rules.iter().any(|r| r.needs_ancestors))
        {
            // :has() and :scope can depend on any ancestor, so restyle everything.
            self.full_dirty = true;
        } else if structural {
            if let Some(parent) = tree.parent(node) {
                self.dirty.insert(parent);
            }
        }
    }

    /// Whether an attribute named `name` can change a style: it is one that
    /// style reads directly, or one that some selector reads (spec 12.4).
    /// Attribute names are case-insensitive, so both sides are lowercased.
    fn attribute_is_read(&mut self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        if ALWAYS_RELEVANT.contains(&lower.as_str()) {
            return true;
        }
        if self.attr_deps.is_none() {
            self.attr_deps = Some(self.collect_attribute_names());
        }
        self.attr_deps
            .as_ref()
            .is_some_and(|deps| deps.contains(&lower))
    }

    /// Every attribute name that a rule's selectors can read: the `id` and
    /// `class` of `#id` and `.class`, the names in `[attr]`, and the same inside
    /// `:not()`, `:is()`, `:where()`, `:has()`, and `@scope` roots and limits.
    fn collect_attribute_names(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for sheet in &self.sheets {
            for rule in &sheet.rules {
                collect_list(&rule.selectors, &mut out);
                for scope in &rule.scopes {
                    if let Some(root) = &scope.root {
                        collect_list(root, &mut out);
                    }
                    if let Some(limit) = &scope.limit {
                        collect_list(limit, &mut out);
                    }
                }
            }
        }
        out
    }

    /// Recomputes the styles affected since the last resolve or restyle (spec 8.2).
    ///
    /// Falls back to a full [`StyleResolver::resolve`] when there is no previous
    /// result, when the environment changed, or after a sheet or tree change.
    /// Otherwise only the dirty subtrees are recomputed, and everything else is
    /// reused from the previous result.
    pub fn restyle<T: StyleTree>(&mut self, tree: &T, env: &Environment) -> &StyleMap {
        let reusable = match &self.cache {
            Some(cache) => cache.env == *env && !self.full_dirty && self.containers.is_none(),
            None => false,
        };
        if !reusable {
            self.resolve(tree, env);
            return self.cached_map();
        }

        // Move the cached map out and work on it in place, then put it back. The
        // caller borrows it from the cache, so nothing is copied.
        let mut map = self.cache.take().map(|c| c.map).unwrap_or_default();
        let active = self.active_rules(env);
        let limits = &self.config.limits;
        let dirty: Vec<NodeId> = std::mem::take(&mut self.dirty).into_iter().collect();
        let cutoff = self.subject_only();

        let mut styled = 0;
        for node in dirty {
            let parent_style = match tree.parent(node) {
                Some(p) if tree.kind(p) == NodeKind::Element => map.get(p).cloned(),
                _ => None,
            };
            let start = Frame {
                node,
                parent: parent_style,
                rem_base: rem_base_for(tree, node, &map, env),
            };
            styled += style_tree(self, tree, start, env, &active, limits, cutoff, &mut map);
        }

        self.last_restyled = styled;

        self.cache = Some(crate::style::resolver::Cache { map, env: *env });
        self.cached_map()
    }

    /// The map held by the cache. Every path that fills the cache leaves it set.
    pub(crate) fn cached_map(&self) -> &StyleMap {
        &self.cache.as_ref().expect("resolve fills the cache").map
    }

    /// Number of elements styled by the most recent [`StyleResolver::resolve`]
    /// or [`StyleResolver::restyle`]. Used to check that a restyle stayed local.
    pub fn last_restyle_count(&self) -> usize {
        self.last_restyled
    }
}

/// The `rem` base for a subtree starting at `node`: `None` if `node` is a
/// top-level element (it is a root), otherwise the font size of its top-level
/// element ancestor.
fn rem_base_for<T: StyleTree>(
    tree: &T,
    node: NodeId,
    map: &StyleMap,
    env: &Environment,
) -> Option<f64> {
    let mut top: Option<NodeId> = None;
    let mut current = Some(node);
    while let Some(n) = current {
        if tree.kind(n) == NodeKind::Element {
            top = Some(n);
        }
        current = tree.parent(n);
    }
    let top = top?;
    if top == node {
        None
    } else {
        Some(map.get(top).map_or(env.root_font_size, |s| s.font_size()))
    }
}

/// Adds every attribute name in a selector list (see `collect_attribute_names`).
fn collect_list(list: &SelectorList, out: &mut BTreeSet<String>) {
    for complex in &list.selectors {
        collect_complex(complex, out);
    }
}

fn collect_complex(complex: &ComplexSelector, out: &mut BTreeSet<String>) {
    for compound in &complex.compounds {
        for simple in &compound.simple {
            collect_simple(simple, out);
        }
    }
}

fn collect_simple(simple: &SimpleSelector, out: &mut BTreeSet<String>) {
    match simple {
        SimpleSelector::Id(_) => {
            out.insert("id".to_string());
        }
        SimpleSelector::Class(_) => {
            out.insert("class".to_string());
        }
        SimpleSelector::Attribute(attr) => {
            out.insert(attr.name.to_ascii_lowercase());
        }
        SimpleSelector::Not(list) | SimpleSelector::Is(list) | SimpleSelector::Where(list) => {
            collect_list(list, out);
        }
        SimpleSelector::Has(relatives) => {
            for relative in relatives {
                collect_complex(&relative.selector, out);
            }
        }
        _ => {}
    }
}
