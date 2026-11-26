// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Rule index for selector matching (spec 21.2).
//!
//! Each selector is filed under the key of its subject compound: its id if it
//! has one, else a class, else its type, else nothing. A node can only match a
//! selector whose key is its own id, one of its classes, or its type, so the
//! matcher checks only those selectors, plus the unkeyed ones. The key is a
//! necessary condition, so no selector that could match is skipped.

use std::collections::BTreeMap;

use super::resolver::LoadedSheet;
use crate::ast::selector::{ComplexSelector, SimpleSelector};
use crate::tree::{NodeId, StyleTree};

/// One selector of one rule: the position in the resolver's sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub sheet: usize,
    pub rule: usize,
    pub selector: usize,
}

/// The key that a selector is filed under.
enum Key {
    Id(String),
    Class(String),
    Type(String),
    Unkeyed,
}

/// The most selective simple selector of the subject compound: an id, then a
/// class, then a type. Anything else (`*`, pseudo-classes alone) is unkeyed.
fn subject_key(selector: &ComplexSelector) -> Key {
    // A `::part()` selector matches the part element, which the subject compound
    // does not describe, so it cannot be filed by that compound.
    if matches!(
        selector.pseudo_element,
        Some(crate::selectors::PseudoElement::Part(_))
    ) {
        return Key::Unkeyed;
    }
    let Some(subject) = selector.compounds.last() else {
        return Key::Unkeyed;
    };
    let mut class = None;
    let mut ty = None;
    for simple in &subject.simple {
        match simple {
            SimpleSelector::Id(id) => return Key::Id(id.clone()),
            SimpleSelector::Class(c) if class.is_none() => class = Some(c.clone()),
            SimpleSelector::Type(t) if ty.is_none() => ty = Some(t.clone()),
            _ => {}
        }
    }
    match (class, ty) {
        (Some(c), _) => Key::Class(c),
        (None, Some(t)) => Key::Type(t),
        (None, None) => Key::Unkeyed,
    }
}

/// Candidate selectors grouped by the key of their subject.
#[derive(Debug, Default)]
pub(crate) struct RuleIndex {
    by_id: BTreeMap<String, Vec<Candidate>>,
    by_class: BTreeMap<String, Vec<Candidate>>,
    by_type: BTreeMap<String, Vec<Candidate>>,
    unkeyed: Vec<Candidate>,
}

impl RuleIndex {
    /// Builds the index for `sheets`. Rebuild it whenever the sheets change.
    pub(crate) fn build(sheets: &[LoadedSheet]) -> RuleIndex {
        let mut index = RuleIndex::default();
        for (sheet, loaded) in sheets.iter().enumerate() {
            for (rule, compiled) in loaded.rules.iter().enumerate() {
                for (selector, complex) in compiled.selectors.selectors.iter().enumerate() {
                    let candidate = Candidate {
                        sheet,
                        rule,
                        selector,
                    };
                    match subject_key(complex) {
                        Key::Id(k) => index.by_id.entry(k).or_default().push(candidate),
                        Key::Class(k) => index.by_class.entry(k).or_default().push(candidate),
                        Key::Type(k) => index.by_type.entry(k).or_default().push(candidate),
                        Key::Unkeyed => index.unkeyed.push(candidate),
                    }
                }
            }
        }
        index
    }

    /// Calls `f` once for each selector that could match `node`. Each selector
    /// is filed under one key, so it is reported at most once.
    pub(crate) fn for_each_candidate<T: StyleTree>(
        &self,
        tree: &T,
        node: NodeId,
        mut f: impl FnMut(Candidate),
    ) {
        for candidate in &self.unkeyed {
            f(*candidate);
        }
        let attributes = tree.attributes(node);
        if let Some(list) = attributes.id.as_ref().and_then(|id| self.by_id.get(id)) {
            for candidate in list {
                f(*candidate);
            }
        }
        for (i, class) in attributes.classes.iter().enumerate() {
            // A repeated class name would report the same selectors twice.
            if attributes.classes[..i].contains(class) {
                continue;
            }
            if let Some(list) = self.by_class.get(class) {
                for candidate in list {
                    f(*candidate);
                }
            }
        }
        if let Some(list) = self.by_type.get(&tree.name(node).local) {
            for candidate in list {
                f(*candidate);
            }
        }
    }
}
