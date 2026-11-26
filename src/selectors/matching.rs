// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector matching (spec 12.4).
//!
//! Matching is right to left: the rightmost compound is tested against the
//! candidate element, then combinators are walked leftward. A step budget
//! (spec 20.3) bounds the work; when it runs out the selector does not match,
//! so the engine fails closed.
//!
//! Structural pseudo-classes read element siblings through the tree contract.
//! Each such check scans the sibling list, so wide trees cost O(siblings) per
//! node. An index is future work (M3).

use crate::ast::selector::{
    AttributeOperator, Combinator, ComplexSelector, CompoundSelector, NthSelector, PseudoClass,
    SelectorList, SimpleSelector,
};
use crate::selectors::specificity::Specificity;
use crate::tree::{Direction, NodeId, NodeKind, StateBits, StyleTree};

/// Returns the highest specificity among the members of `list` that match
/// `node`, ignoring pseudo-element targets, or `None` if none matches.
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

/// True if `node` matches the element part of the complex selector.
/// `budget` is decremented per step and matching stops at zero.
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

fn list_matches<T: StyleTree>(
    tree: &T,
    node: NodeId,
    list: &SelectorList,
    budget: &mut usize,
) -> bool {
    list.selectors
        .iter()
        .any(|s| matches_complex(tree, node, s, budget))
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
    if !is_element(tree, node) || !compound_matches(tree, node, &selector.compounds[idx], budget) {
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
                if match_from(tree, ancestor, selector, idx - 1, budget) {
                    return true;
                }
                current = tree.parent(ancestor);
            }
            false
        }
        Combinator::NextSibling => match previous_element_siblings(tree, node).first() {
            Some(&prev) => match_from(tree, prev, selector, idx - 1, budget),
            None => false,
        },
        Combinator::SubsequentSibling => {
            for prev in previous_element_siblings(tree, node) {
                if !charge(budget) {
                    return false;
                }
                if match_from(tree, prev, selector, idx - 1, budget) {
                    return true;
                }
            }
            false
        }
    }
}

/// All element siblings of `node` including itself, in document order. A
/// parentless node is its own only sibling.
fn element_siblings<T: StyleTree>(tree: &T, node: NodeId) -> Vec<NodeId> {
    match tree.parent(node) {
        None => vec![node],
        Some(p) => tree
            .children(p)
            .iter()
            .copied()
            .filter(|c| is_element(tree, *c))
            .collect(),
    }
}

/// Element siblings before `node`, nearest first.
fn previous_element_siblings<T: StyleTree>(tree: &T, node: NodeId) -> Vec<NodeId> {
    let sibs = element_siblings(tree, node);
    match sibs.iter().position(|s| *s == node) {
        Some(pos) => sibs[..pos].iter().rev().copied().collect(),
        None => Vec::new(),
    }
}

fn compound_matches<T: StyleTree>(
    tree: &T,
    node: NodeId,
    compound: &CompoundSelector,
    budget: &mut usize,
) -> bool {
    compound
        .simple
        .iter()
        .all(|s| simple_matches(tree, node, s, budget))
}

fn simple_matches<T: StyleTree>(
    tree: &T,
    node: NodeId,
    simple: &SimpleSelector,
    budget: &mut usize,
) -> bool {
    match simple {
        SimpleSelector::Universal => true,
        // Replaced during nesting resolution. A bare `&` reaching the matcher is
        // a top-level rule and matches nothing.
        SimpleSelector::Nesting => false,
        SimpleSelector::Type(local) => tree.name(node).local == *local,
        SimpleSelector::Id(id) => tree.attributes(node).id.as_deref() == Some(id.as_str()),
        SimpleSelector::Class(class) => tree.attributes(node).has_class(class),
        SimpleSelector::Attribute(attr) => match tree.attributes(node).get(&attr.name) {
            None => false,
            Some(actual) => match &attr.matcher {
                None => true,
                Some(m) => attribute_value_matches(m.operator, actual, &m.value),
            },
        },
        SimpleSelector::PseudoClass(pseudo) => pseudo_class_matches(tree, node, *pseudo),
        SimpleSelector::Nth(nth) => nth_matches(tree, node, nth, budget),
        SimpleSelector::Not(list) => !list_matches(tree, node, list, budget),
        SimpleSelector::Is(list) | SimpleSelector::Where(list) => {
            list_matches(tree, node, list, budget)
        }
        SimpleSelector::Lang(tag) => language_matches(tree, node, tag),
        SimpleSelector::Dir(dir) => inherited_direction(tree, node) == Some(*dir),
    }
}

fn attribute_value_matches(op: AttributeOperator, actual: &str, expected: &str) -> bool {
    match op {
        AttributeOperator::Equals => actual == expected,
        AttributeOperator::Includes => {
            !expected.is_empty()
                && !expected.contains(char::is_whitespace)
                && actual.split_ascii_whitespace().any(|t| t == expected)
        }
        AttributeOperator::DashMatch => {
            actual == expected
                || (actual.starts_with(expected) && actual[expected.len()..].starts_with('-'))
        }
        AttributeOperator::Prefix => !expected.is_empty() && actual.starts_with(expected),
        AttributeOperator::Suffix => !expected.is_empty() && actual.ends_with(expected),
        AttributeOperator::Substring => !expected.is_empty() && actual.contains(expected),
    }
}

/// Matches `An+B` against a 1-based position (spec 12.2, `:nth-child()`).
pub fn anb_matches(position: i64, a: i64, b: i64) -> bool {
    if a == 0 {
        return position == b;
    }
    let diff = position - b;
    diff % a == 0 && diff / a >= 0
}

fn nth_matches<T: StyleTree>(
    tree: &T,
    node: NodeId,
    nth: &NthSelector,
    budget: &mut usize,
) -> bool {
    if let Some(of) = &nth.of {
        if !list_matches(tree, node, of, budget) {
            return false;
        }
    }
    let mut sibs = element_siblings(tree, node);
    if nth.of_type {
        let name = &tree.name(node).local;
        sibs.retain(|s| tree.name(*s).local == *name);
    }
    if let Some(of) = &nth.of {
        let mut filtered = Vec::with_capacity(sibs.len());
        for s in sibs {
            if list_matches(tree, s, of, budget) {
                filtered.push(s);
            }
        }
        sibs = filtered;
    }
    if nth.last {
        sibs.reverse();
    }
    match sibs.iter().position(|s| *s == node) {
        Some(pos) => anb_matches((pos + 1) as i64, nth.a, nth.b),
        None => false,
    }
}

fn pseudo_class_matches<T: StyleTree>(tree: &T, node: NodeId, pseudo: PseudoClass) -> bool {
    let state = tree.state(node).bits;
    match pseudo {
        PseudoClass::Root => is_root(tree, node),
        PseudoClass::Hover => state.contains(StateBits::HOVER),
        PseudoClass::Active => state.contains(StateBits::ACTIVE),
        PseudoClass::Focus => state.contains(StateBits::FOCUS),
        PseudoClass::Empty => is_empty(tree, node),
        PseudoClass::Enabled => state.contains(StateBits::ENABLED),
        PseudoClass::Disabled => state.contains(StateBits::DISABLED),
        PseudoClass::Checked => state.contains(StateBits::CHECKED),
        PseudoClass::Required => state.contains(StateBits::REQUIRED),
        PseudoClass::Optional => state.contains(StateBits::OPTIONAL),
        PseudoClass::ReadOnly => state.contains(StateBits::READ_ONLY),
        PseudoClass::ReadWrite => state.contains(StateBits::READ_WRITE),
        PseudoClass::PlaceholderShown => state.contains(StateBits::PLACEHOLDER_SHOWN),
        PseudoClass::Link => state.contains(StateBits::LINK),
        PseudoClass::AnyLink => state.contains(StateBits::ANY_LINK),
        PseudoClass::Visited => state.contains(StateBits::VISITED),
        PseudoClass::FocusVisible => state.contains(StateBits::FOCUS_VISIBLE),
        PseudoClass::FocusWithin => state.contains(StateBits::FOCUS_WITHIN),
        PseudoClass::Target => state.contains(StateBits::TARGET),
        PseudoClass::Valid => state.contains(StateBits::VALID),
        PseudoClass::Invalid => state.contains(StateBits::INVALID),
        PseudoClass::UserValid => state.contains(StateBits::USER_VALID),
        PseudoClass::UserInvalid => state.contains(StateBits::USER_INVALID),
        PseudoClass::InRange => state.contains(StateBits::IN_RANGE),
        PseudoClass::OutOfRange => state.contains(StateBits::OUT_OF_RANGE),
        PseudoClass::Default => state.contains(StateBits::DEFAULT),
        PseudoClass::Indeterminate => state.contains(StateBits::INDETERMINATE),
        PseudoClass::Open => state.contains(StateBits::OPEN),
        PseudoClass::Closed => state.contains(StateBits::CLOSED),
        PseudoClass::Modal => state.contains(StateBits::MODAL),
        PseudoClass::PopoverOpen => state.contains(StateBits::POPOVER_OPEN),
    }
}

/// `:root` matches an element with no parent, or whose parent is not an
/// element (for example the document).
fn is_root<T: StyleTree>(tree: &T, node: NodeId) -> bool {
    match tree.parent(node) {
        None => true,
        Some(p) => tree.kind(p) != NodeKind::Element,
    }
}

/// `:empty`: no element children and no text with content. Comments are ignored.
fn is_empty<T: StyleTree>(tree: &T, node: NodeId) -> bool {
    tree.children(node).iter().all(|&c| match tree.kind(c) {
        NodeKind::Element => false,
        NodeKind::Text => tree.text(c).is_empty(),
        _ => true,
    })
}

/// The nearest language tag, from the node or an ancestor.
fn inherited_language<T: StyleTree>(tree: &T, node: NodeId) -> Option<&str> {
    let mut current = Some(node);
    while let Some(n) = current {
        if let Some(lang) = tree.language(n) {
            return Some(lang);
        }
        current = tree.parent(n);
    }
    None
}

/// The nearest text direction, from the node or an ancestor.
fn inherited_direction<T: StyleTree>(tree: &T, node: NodeId) -> Option<Direction> {
    let mut current = Some(node);
    while let Some(n) = current {
        if let Some(dir) = tree.direction(n) {
            return Some(dir);
        }
        current = tree.parent(n);
    }
    None
}

/// `:lang(tag)`: the element's language equals `tag` or is a subtag of it
/// (RFC 4647 basic filtering, ASCII case-insensitive).
fn language_matches<T: StyleTree>(tree: &T, node: NodeId, tag: &str) -> bool {
    if tag.is_empty() {
        return false;
    }
    match inherited_language(tree, node) {
        None => false,
        Some(lang) => {
            lang.eq_ignore_ascii_case(tag)
                || lang
                    .get(..tag.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(tag))
                    && lang[tag.len()..].starts_with('-')
        }
    }
}
