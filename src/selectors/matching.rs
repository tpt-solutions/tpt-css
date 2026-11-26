// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector matching (spec 12.4).
//!
//! Matching is right to left: the rightmost compound is tested against the
//! candidate element, then combinators are walked leftward. Work is bounded by
//! a step budget (spec 20.3). When the budget runs out the selector does not
//! match, so the engine fails closed rather than spending unbounded time.

use crate::ast::selector::{
    AttributeOperator, Combinator, ComplexSelector, CompoundSelector, PseudoClass, SelectorList,
    SimpleSelector,
};
use crate::selectors::specificity::Specificity;
use crate::tree::{NodeId, NodeKind, StateBits, StyleTree};

/// Returns the specificity of the most specific selector in `list` that
/// matches `node`, or `None` if none matches. Spec 12.3: a selector list
/// matches with its most specific matching member.
pub fn match_selector_list<T: StyleTree>(
    tree: &T,
    node: NodeId,
    list: &SelectorList,
    max_steps: usize,
) -> Option<Specificity> {
    let mut best: Option<Specificity> = None;
    for selector in &list.selectors {
        let mut budget = max_steps;
        if matches_complex(tree, node, selector, &mut budget) {
            best = Some(match best {
                Some(b) if b >= selector.specificity => b,
                _ => selector.specificity,
            });
        }
    }
    best
}

/// True if `node` matches the complex selector. `budget` is decremented for
/// each step and matching stops (returning false) at zero.
pub fn matches_complex<T: StyleTree>(
    tree: &T,
    node: NodeId,
    selector: &ComplexSelector,
    budget: &mut usize,
) -> bool {
    match selector.compounds.len() {
        0 => false,
        n => match_from(tree, node, selector, n - 1, budget),
    }
}

/// Precondition: `node` is the candidate for `compounds[idx]`.
fn match_from<T: StyleTree>(
    tree: &T,
    node: NodeId,
    selector: &ComplexSelector,
    idx: usize,
    budget: &mut usize,
) -> bool {
    if !charge(budget) {
        return false;
    }
    if !is_element(tree, node) || !compound_matches(tree, node, &selector.compounds[idx]) {
        return false;
    }
    if idx == 0 {
        return true;
    }
    match selector.combinators[idx - 1] {
        Combinator::Child => match tree.parent(node) {
            Some(parent) => match_from(tree, parent, selector, idx - 1, budget),
            None => false,
        },
        Combinator::Descendant => {
            let mut current = tree.parent(node);
            while let Some(ancestor) = current {
                if !charge(budget) {
                    return false;
                }
                if is_element(tree, ancestor)
                    && match_from(tree, ancestor, selector, idx - 1, budget)
                {
                    return true;
                }
                current = tree.parent(ancestor);
            }
            false
        }
    }
}

fn charge(budget: &mut usize) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    true
}

fn is_element<T: StyleTree>(tree: &T, node: NodeId) -> bool {
    tree.kind(node) == NodeKind::Element
}

fn compound_matches<T: StyleTree>(tree: &T, node: NodeId, compound: &CompoundSelector) -> bool {
    compound
        .simple
        .iter()
        .all(|s| simple_matches(tree, node, s))
}

fn simple_matches<T: StyleTree>(tree: &T, node: NodeId, simple: &SimpleSelector) -> bool {
    match simple {
        SimpleSelector::Universal => true,
        SimpleSelector::Type(local) => tree.name(node).local == *local,
        SimpleSelector::Id(id) => tree.attributes(node).id.as_deref() == Some(id.as_str()),
        SimpleSelector::Class(class) => tree.attributes(node).has_class(class),
        SimpleSelector::Attribute(attr) => match tree.attributes(node).get(&attr.name) {
            None => false,
            Some(actual) => match &attr.matcher {
                None => true,
                Some(m) => match m.operator {
                    AttributeOperator::Equals => actual == m.value,
                },
            },
        },
        SimpleSelector::PseudoClass(pseudo) => pseudo_class_matches(tree, node, *pseudo),
    }
}

fn pseudo_class_matches<T: StyleTree>(tree: &T, node: NodeId, pseudo: PseudoClass) -> bool {
    let state = tree.state(node).bits;
    match pseudo {
        PseudoClass::Root => is_root(tree, node),
        PseudoClass::Hover => state.contains(StateBits::HOVER),
        PseudoClass::Active => state.contains(StateBits::ACTIVE),
        PseudoClass::Focus => state.contains(StateBits::FOCUS),
    }
}

/// `:root` matches an element with no parent, or whose parent is not an
/// element (for example the document). Scope rules for fragments arrive in M2.
fn is_root<T: StyleTree>(tree: &T, node: NodeId) -> bool {
    match tree.parent(node) {
        None => true,
        Some(p) => tree.kind(p) != NodeKind::Element,
    }
}
