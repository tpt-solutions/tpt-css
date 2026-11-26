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
//! A [`SiblingCache`] holds each parent's element children and their positions,
//! built once per style pass, so `:nth-child()` and `+` cost O(log siblings) per
//! node. Without a cache, the matcher builds the same index for each check.
//! `~`, `:nth-of-type()`, and `:nth-child(... of S)` still walk the list.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::ast::selector::{
    AttributeOperator, Combinator, ComplexSelector, CompoundSelector, NthSelector, PseudoClass,
    RelativeSelector, SelectorList, SimpleSelector,
};
use crate::selectors::pseudo::PseudoElement;
use crate::selectors::specificity::Specificity;
use crate::tree::scope::shadow_root_of;
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
        siblings: None,
        shadow: None,
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
    siblings: Option<&'a SiblingCache>,
    /// The shadow root of the sheet being matched, if the sheet is scoped (spec 9.3).
    shadow: Option<NodeId>,
}

/// The element children of one parent, with the position of each.
struct Siblings {
    elements: Vec<NodeId>,
    positions: BTreeMap<NodeId, usize>,
}

impl Siblings {
    fn of<T: StyleTree>(tree: &T, parent: NodeId) -> Siblings {
        let elements: Vec<NodeId> = tree
            .children(parent)
            .iter()
            .copied()
            .filter(|c| tree.kind(*c) == NodeKind::Element)
            .collect();
        let positions = elements.iter().enumerate().map(|(i, n)| (*n, i)).collect();
        Siblings {
            elements,
            positions,
        }
    }
}

/// The sibling index of each parent, built on first use (spec 21.2). It is
/// valid while the tree does not change, so the resolver clears it at the start
/// of each style pass.
#[derive(Default)]
pub(crate) struct SiblingCache {
    parents: RefCell<BTreeMap<NodeId, Rc<Siblings>>>,
}

impl SiblingCache {
    /// Forgets every parent. Call this when the tree may have changed.
    pub(crate) fn clear(&self) {
        self.parents.borrow_mut().clear();
    }

    fn get<T: StyleTree>(&self, tree: &T, parent: NodeId) -> Rc<Siblings> {
        if let Some(found) = self.parents.borrow().get(&parent) {
            return Rc::clone(found);
        }
        let built = Rc::new(Siblings::of(tree, parent));
        self.parents.borrow_mut().insert(parent, Rc::clone(&built));
        built
    }
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
            siblings: None,
            shadow: None,
        }
    }

    /// Like [`Matcher::new`], but sibling positions come from `siblings`.
    pub(crate) fn cached(
        tree: &'a T,
        max_steps: usize,
        scope: Option<NodeId>,
        siblings: &'a SiblingCache,
    ) -> Self {
        Matcher {
            tree,
            budget: max_steps,
            scope,
            siblings: Some(siblings),
            shadow: None,
        }
    }

    /// True if `node` matches the complex selector's element part.
    pub fn matches(&mut self, node: NodeId, selector: &ComplexSelector) -> bool {
        match &selector.pseudo_element {
            Some(PseudoElement::Part(name)) => return self.matches_part(node, selector, name),
            Some(PseudoElement::Slotted) if !self.is_slotted(node) => return false,
            _ => {}
        }
        match selector.compounds.len() {
            0 => false,
            n => self.match_from(node, selector, n - 1, None),
        }
    }

    /// `::part(name)` (Shadow DOM 1 section 4): `node` has the part, and the rest
    /// of the selector matches its shadow host.
    fn matches_part(&mut self, node: NodeId, selector: &ComplexSelector, name: &str) -> bool {
        let has_part = self
            .tree
            .attributes(node)
            .get("part")
            .is_some_and(|parts| parts.split_ascii_whitespace().any(|p| p == name));
        if !has_part {
            return false;
        }
        let Some(root) = shadow_root_of(self.tree, node) else {
            return false;
        };
        let Some(host) = self.tree.shadow(root).host else {
            return false;
        };
        let Some(count) = selector.compounds.len().checked_sub(1) else {
            return false;
        };
        // The host side is matched in the document, so the sheet's shadow root
        // does not apply to it.
        let saved = self.shadow.take();
        let matched = self.match_from(host, selector, count, None);
        self.shadow = saved;
        matched
    }

    /// `::slotted()` (Shadow DOM 1 section 5): `node` is assigned to a slot in the
    /// shadow tree that this sheet is scoped to.
    fn is_slotted(&self, node: NodeId) -> bool {
        match (self.shadow, self.tree.assigned_slot(node)) {
            (Some(root), Some(slot)) => shadow_root_of(self.tree, slot) == Some(root),
            _ => false,
        }
    }

    /// Sets the shadow root of the sheet being matched. Document sheets pass `None`.
    pub(crate) fn in_shadow(mut self, root: Option<NodeId>) -> Self {
        self.shadow = root;
        self
    }

    /// The node that a compound at a combinator may be matched against, when
    /// stepping up from `from`. Stepping from a shadow root goes to its host, and
    /// only for a host compound (`:host`, `:host()`, `:host-context()`), so a
    /// document selector cannot match across a shadow boundary (spec 9.3).
    fn upward(&self, from: NodeId, compound: &CompoundSelector) -> Option<NodeId> {
        let parent = self.tree.parent(from)?;
        // A shadow root is parentless, so only a parentless node can be one.
        if self.tree.parent(parent).is_some()
            || self.tree.shadow(parent).shadow_root != Some(parent)
        {
            return Some(parent);
        }
        let host_compound = compound
            .simple
            .iter()
            .any(|s| matches!(s, SimpleSelector::Host(_) | SimpleSelector::HostContext(_)));
        if self.shadow == Some(parent) && host_compound {
            self.tree.shadow(parent).host
        } else {
            None
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
            Combinator::Child => match self.upward(node, &selector.compounds[idx - 1]) {
                Some(parent) => self.match_from(parent, selector, idx - 1, anchor),
                None => false,
            },
            Combinator::Descendant => {
                let compound = &selector.compounds[idx - 1];
                let mut current = self.upward(node, compound);
                while let Some(ancestor) = current {
                    if !self.charge() {
                        return false;
                    }
                    if self.match_from(ancestor, selector, idx - 1, anchor) {
                        return true;
                    }
                    current = self.upward(ancestor, compound);
                }
                false
            }
            Combinator::NextSibling => match self.previous_element(node) {
                Some(prev) => self.match_from(prev, selector, idx - 1, anchor),
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
            Combinator::NextSibling => self.previous_element(node) == Some(anchor.node),
            Combinator::SubsequentSibling => {
                self.previous_element_siblings(node).contains(&anchor.node)
            }
        }
    }

    /// Runs `f` on the element siblings of `node` (itself included) and on the
    /// index of `node` among them. A parentless node is its own only sibling.
    /// Returns `None` if `node` is not an element child of its parent.
    fn with_siblings<R>(&self, node: NodeId, f: impl FnOnce(&[NodeId], usize) -> R) -> Option<R> {
        let Some(parent) = self.tree.parent(node) else {
            return Some(f(&[node], 0));
        };
        let siblings = match self.siblings {
            Some(cache) => cache.get(self.tree, parent),
            None => Rc::new(Siblings::of(self.tree, parent)),
        };
        let pos = *siblings.positions.get(&node)?;
        Some(f(&siblings.elements, pos))
    }

    /// The element sibling just before `node`, if any.
    fn previous_element(&self, node: NodeId) -> Option<NodeId> {
        self.with_siblings(node, |list, pos| pos.checked_sub(1).map(|p| list[p]))
            .flatten()
    }

    /// Element siblings before `node`, nearest first.
    fn previous_element_siblings(&self, node: NodeId) -> Vec<NodeId> {
        self.with_siblings(node, |list, pos| {
            list[..pos].iter().rev().copied().collect()
        })
        .unwrap_or_default()
    }

    /// Element siblings after `node`, nearest first.
    fn following_element_siblings(&self, node: NodeId) -> Vec<NodeId> {
        self.with_siblings(node, |list, pos| list[pos + 1..].to_vec())
            .unwrap_or_default()
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

    /// The host of the sheet's shadow root, if the sheet is scoped.
    fn sheet_host(&self) -> Option<NodeId> {
        self.shadow.and_then(|root| self.tree.shadow(root).host)
    }

    /// True if `node` is the host of the shadow root this sheet is scoped to.
    /// Document sheets have no host, so `:host` never matches for them.
    fn is_host(&self, node: NodeId) -> bool {
        self.sheet_host() == Some(node)
    }

    /// True if `node` or one of its ancestors in the document matches `list`.
    fn host_or_ancestor_matches(&mut self, node: NodeId, list: &SelectorList) -> bool {
        let mut current = Some(node);
        while let Some(candidate) = current {
            if self.is_element(candidate) && self.list_matches(candidate, list) {
                return true;
            }
            current = self.tree.parent(candidate);
        }
        false
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
            SimpleSelector::Host(argument) => {
                self.is_host(node)
                    && argument
                        .as_ref()
                        .map_or(true, |l| self.list_matches(node, l))
            }
            SimpleSelector::HostContext(list) => {
                self.is_host(node) && self.host_or_ancestor_matches(node, list)
            }
            SimpleSelector::CustomState(name) => self.tree.state(node).custom.contains(name),
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
        // The plain forms need only the position, so they use the index directly.
        if nth.of.is_none() && !nth.of_type {
            let position =
                self.with_siblings(
                    node,
                    |list, pos| {
                        if nth.last {
                            list.len() - pos
                        } else {
                            pos + 1
                        }
                    },
                );
            return match position {
                Some(p) => anb_matches(p as i64, nth.a, nth.b),
                None => false,
            };
        }
        let mut sibs = match self.with_siblings(node, |list, _| list.to_vec()) {
            Some(list) => list,
            None => return false,
        };
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
        PseudoClass::Current => StateBits::CURRENT,
        PseudoClass::Past => StateBits::PAST,
        PseudoClass::Future => StateBits::FUTURE,
        PseudoClass::Defined => StateBits::DEFINED,
        PseudoClass::LocalLink => StateBits::LOCAL_LINK,
        PseudoClass::Playing => StateBits::PLAYING,
        PseudoClass::Paused => StateBits::PAUSED,
        PseudoClass::Fullscreen => StateBits::FULLSCREEN,
        PseudoClass::Autofill => StateBits::AUTOFILL,
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
