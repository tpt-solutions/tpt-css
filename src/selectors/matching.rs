// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector matching (spec 12.4).
//!
//! Matching is right to left: the rightmost compound is tested against the
//! candidate element, then combinators are walked leftward. A step budget
//! (spec 20.3) bounds the work; when it runs out the selector does not match,
//! so the engine fails closed.
//!
//! [`Matcher`] carries the context for one match: the tree, the remaining step
//! budget, and the scope root for `:scope`. `:has()` matches relative to an
//! anchor element, so its leftmost compound is checked against that anchor.
//!
//! Structural pseudo-classes read element siblings through the tree contract.
//! Each such check scans the sibling list, so wide trees cost O(siblings) per
//! node. An index is M3 work (see TODO.md).

use crate::ast::selector::{
    AttributeOperator, Combinator, ComplexSelector, CompoundSelector, NthSelector, PseudoClass,
    RelativeSelector, SelectorList, SimpleSelector,
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
        let mut matcher = Matcher::new(tree, max_steps, None);
        if matcher.matches(node, selector) {
            best = Some(match best {
                Some(b) if b >= selector.specificity => b,
                _ => selector.specificity,
            });
        }
    }
    best
}

/// True if `node` matches the element part of the complex selector. `budget` is
/// decremented per step and matching stops at zero.
pub fn matches_complex<T: StyleTree>(
    tree: &T,
    node: NodeId,
    selector: &ComplexSelector,
    budget: &mut usize,
) -> bool {
    let mut matcher = Matcher {
        tree,
        budget: *budget,
        scope: None,
    };
    let result = matcher.matches(node, selector);
    *budget = matcher.budget;
    result
}

/// The context of one selector match.
pub struct Matcher<'a, T: StyleTree> {
    tree: &'a T,
    budget: usize,
    scope: Option<NodeId>,
}

/// The relation a leftmost compound must have to an anchor element.
#[derive(Clone, Copy)]
struct Anchor {
    node: NodeId,
    relation: Combinator,
}

impl<'a, T: StyleTree> Matcher<'a, T> {
    /// Creates a matcher with a step budget and an optional `:scope` root.
    pub fn new(tree: &'a T, max_steps: usize, scope: Option<NodeId>) -> Self {
        Matcher {
            tree,
            budget: max_steps,
            scope,
        }
    }

    /// True if `node` matches the complex selector's element part.
    pub fn matches(&mut self, node: NodeId, selector: &ComplexSelector) -> bool {
        match selector.compounds.len() {
            0 => false,
            n => self.match_from(node, selector, n - 1, None),
        }
    }

    /// True if `node` matches any member of `list`, sharing this matcher's budget.
    pub fn matches_any(&mut self, node: NodeId, list: &SelectorList) -> bool {
        self.list_matches(node, list)
    }

    fn charge(&mut self) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        true
    }

    fn is_element(&self, node: NodeId) -> bool {
        self.tree.kind(node) == NodeKind::Element
    }

    /// Precondition: `node` is the candidate for `compounds[idx]`.
    fn match_from(
        &mut self,
        node: NodeId,
        selector: &ComplexSelector,
        idx: usize,
        anchor: Option<Anchor>,
    ) -> bool {
        if !self.charge() {
            return false;
        }
        if !self.is_element(node) || !self.compound_matches(node, &selector.compounds[idx]) {
            return false;
        }
        if idx == 0 {
            return match anchor {
                None => true,
                Some(a) => self.related(node, a),
            };
        }
        match selector.combinators[idx - 1] {
            Combinator::Child => match self.tree.parent(node) {
                Some(parent) => self.match_from(parent, selector, idx - 1, anchor),
                None => false,
            },
            Combinator::Descendant => {
                let mut current = self.tree.parent(node);
                while let Some(ancestor) = current {
                    if !self.charge() {
                        return false;
                    }
                    if self.match_from(ancestor, selector, idx - 1, anchor) {
                        return true;
                    }
                    current = self.tree.parent(ancestor);
                }
                false
            }
            Combinator::NextSibling => match self.previous_element_siblings(node).first() {
                Some(&prev) => self.match_from(prev, selector, idx - 1, anchor),
                None => false,
            },
            Combinator::SubsequentSibling => {
                for prev in self.previous_element_siblings(node) {
                    if !self.charge() {
                        return false;
                    }
                    if self.match_from(prev, selector, idx - 1, anchor) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// True if `node` has the relation to the anchor (for `:has()`).
    fn related(&self, node: NodeId, anchor: Anchor) -> bool {
        match anchor.relation {
            Combinator::Child => self.tree.parent(node) == Some(anchor.node),
            Combinator::Descendant => {
                let mut current = self.tree.parent(node);
                while let Some(p) = current {
                    if p == anchor.node {
                        return true;
                    }
                    current = self.tree.parent(p);
                }
                false
            }
            Combinator::NextSibling => {
                self.previous_element_siblings(node).first() == Some(&anchor.node)
            }
            Combinator::SubsequentSibling => {
                self.previous_element_siblings(node).contains(&anchor.node)
            }
        }
    }

    /// All element siblings of `node` including itself, in document order. A
    /// parentless node is its own only sibling.
    fn element_siblings(&self, node: NodeId) -> Vec<NodeId> {
        match self.tree.parent(node) {
            None => vec![node],
            Some(p) => self
                .tree
                .children(p)
                .iter()
                .copied()
                .filter(|c| self.tree.kind(*c) == NodeKind::Element)
                .collect(),
        }
    }

    /// Element siblings before `node`, nearest first.
    fn previous_element_siblings(&self, node: NodeId) -> Vec<NodeId> {
        let sibs = self.element_siblings(node);
        match sibs.iter().position(|s| *s == node) {
            Some(pos) => sibs[..pos].iter().rev().copied().collect(),
            None => Vec::new(),
        }
    }

    /// Element siblings after `node`, nearest first.
    fn following_element_siblings(&self, node: NodeId) -> Vec<NodeId> {
        let sibs = self.element_siblings(node);
        match sibs.iter().position(|s| *s == node) {
            Some(pos) => sibs[pos + 1..].to_vec(),
            None => Vec::new(),
        }
    }

    fn compound_matches(&mut self, node: NodeId, compound: &CompoundSelector) -> bool {
        compound.simple.iter().all(|s| self.simple_matches(node, s))
    }

    fn list_matches(&mut self, node: NodeId, list: &SelectorList) -> bool {
        list.selectors.iter().any(|s| self.matches_in_list(node, s))
    }

    /// Matches a complex selector nested in a functional pseudo-class. The step
    /// budget is shared with the enclosing match.
    fn matches_in_list(&mut self, node: NodeId, selector: &ComplexSelector) -> bool {
        self.matches(node, selector)
    }

    fn simple_matches(&mut self, node: NodeId, simple: &SimpleSelector) -> bool {
        match simple {
            SimpleSelector::Universal => true,
            // Replaced during nesting resolution. A bare `&` reaching the matcher is
            // a top-level rule and matches nothing.
            SimpleSelector::Nesting => false,
            SimpleSelector::Type(local) => self.tree.name(node).local == *local,
            SimpleSelector::Id(id) => self.tree.attributes(node).id.as_deref() == Some(id.as_str()),
            SimpleSelector::Class(class) => self.tree.attributes(node).has_class(class),
            SimpleSelector::Attribute(attr) => match self.tree.attributes(node).get(&attr.name) {
                None => false,
                Some(actual) => match &attr.matcher {
                    None => true,
                    Some(m) => attribute_value_matches(m.operator, actual, &m.value),
                },
            },
            SimpleSelector::PseudoClass(pseudo) => self.pseudo_class_matches(node, *pseudo),
            SimpleSelector::Nth(nth) => self.nth_matches(node, nth),
            SimpleSelector::Not(list) => !self.list_matches(node, list),
            SimpleSelector::Is(list) | SimpleSelector::Where(list) => self.list_matches(node, list),
            SimpleSelector::Lang(tag) => language_matches(self.tree, node, tag),
            SimpleSelector::Dir(dir) => inherited_direction(self.tree, node) == Some(*dir),
            SimpleSelector::Has(relatives) => self.has_matches(node, relatives),
            SimpleSelector::Scope => match self.scope {
                Some(root) => node == root,
                None => is_root(self.tree, node),
            },
        }
    }

    /// `:has(relatives)`: some element related to `node` matches a relative
    /// selector, with the relation given by its leading combinator.
    fn has_matches(&mut self, node: NodeId, relatives: &[RelativeSelector]) -> bool {
        for relative in relatives {
            let candidates = self.relative_candidates(node, relative.leading);
            for candidate in candidates {
                let selector = &relative.selector;
                if selector.compounds.is_empty() {
                    continue;
                }
                let anchor = Anchor {
                    node,
                    relation: relative.leading,
                };
                if self.match_from(
                    candidate,
                    selector,
                    selector.compounds.len() - 1,
                    Some(anchor),
                ) {
                    return true;
                }
            }
        }
        false
    }

    /// The elements a relative selector can reach from `node`.
    fn relative_candidates(&self, node: NodeId, leading: Combinator) -> Vec<NodeId> {
        match leading {
            Combinator::Child => self
                .tree
                .children(node)
                .iter()
                .copied()
                .filter(|c| self.is_element(*c))
                .collect(),
            Combinator::Descendant => {
                let mut out = Vec::new();
                let mut stack: Vec<NodeId> =
                    self.tree.children(node).iter().rev().copied().collect();
                while let Some(n) = stack.pop() {
                    if self.is_element(n) {
                        out.push(n);
                    }
                    stack.extend(self.tree.children(n).iter().rev().copied());
                }
                out
            }
            Combinator::NextSibling => self
                .following_element_siblings(node)
                .into_iter()
                .take(1)
                .collect(),
            Combinator::SubsequentSibling => self.following_element_siblings(node),
        }
    }

    fn nth_matches(&mut self, node: NodeId, nth: &NthSelector) -> bool {
        if let Some(of) = &nth.of {
            if !self.list_matches(node, of) {
                return false;
            }
        }
        let mut sibs = self.element_siblings(node);
        if nth.of_type {
            let name = self.tree.name(node).local.clone();
            sibs.retain(|s| self.tree.name(*s).local == name);
        }
        if let Some(of) = &nth.of {
            let mut filtered = Vec::with_capacity(sibs.len());
            for s in sibs {
                if self.list_matches(s, of) {
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

    fn pseudo_class_matches(&self, node: NodeId, pseudo: PseudoClass) -> bool {
        let state = self.tree.state(node).bits;
        match pseudo {
            PseudoClass::Root => is_root(self.tree, node),
            PseudoClass::Empty => is_empty(self.tree, node),
            other => state_bit_matches(state, other),
        }
    }
}

/// The state bit a state pseudo-class reads (spec 9.2).
fn state_bit_matches(state: StateBits, pseudo: PseudoClass) -> bool {
    let bit = match pseudo {
        PseudoClass::Hover => StateBits::HOVER,
        PseudoClass::Active => StateBits::ACTIVE,
        PseudoClass::Focus => StateBits::FOCUS,
        PseudoClass::Enabled => StateBits::ENABLED,
        PseudoClass::Disabled => StateBits::DISABLED,
        PseudoClass::Checked => StateBits::CHECKED,
        PseudoClass::Required => StateBits::REQUIRED,
        PseudoClass::Optional => StateBits::OPTIONAL,
        PseudoClass::ReadOnly => StateBits::READ_ONLY,
        PseudoClass::ReadWrite => StateBits::READ_WRITE,
        PseudoClass::PlaceholderShown => StateBits::PLACEHOLDER_SHOWN,
        PseudoClass::Link => StateBits::LINK,
        PseudoClass::AnyLink => StateBits::ANY_LINK,
        PseudoClass::Visited => StateBits::VISITED,
        PseudoClass::FocusVisible => StateBits::FOCUS_VISIBLE,
        PseudoClass::FocusWithin => StateBits::FOCUS_WITHIN,
        PseudoClass::Target => StateBits::TARGET,
        PseudoClass::Valid => StateBits::VALID,
        PseudoClass::Invalid => StateBits::INVALID,
        PseudoClass::UserValid => StateBits::USER_VALID,
        PseudoClass::UserInvalid => StateBits::USER_INVALID,
        PseudoClass::InRange => StateBits::IN_RANGE,
        PseudoClass::OutOfRange => StateBits::OUT_OF_RANGE,
        PseudoClass::Default => StateBits::DEFAULT,
        PseudoClass::Indeterminate => StateBits::INDETERMINATE,
        PseudoClass::Open => StateBits::OPEN,
        PseudoClass::Closed => StateBits::CLOSED,
        PseudoClass::Modal => StateBits::MODAL,
        PseudoClass::PopoverOpen => StateBits::POPOVER_OPEN,
        // Structural pseudo-classes are handled by the caller.
        PseudoClass::Root | PseudoClass::Empty => return false,
    };
    state.contains(bit)
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
