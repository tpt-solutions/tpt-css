// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector AST (spec 12.1). M1 covers Tier 0 and Tier 1 selectors, the
//! sibling combinators, and the pseudo-elements `::before`, `::after`,
//! `::marker`, and `::placeholder`.

use crate::selectors::pseudo::PseudoElement;
use crate::selectors::specificity::Specificity;
use crate::syntax::Span;
use crate::tree::Direction;

/// A selector list: `a, b c`.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorList {
    /// Complex selectors, in source order.
    pub selectors: Vec<ComplexSelector>,
}

/// A complex selector: compound selectors joined by combinators, optionally
/// ending in a pseudo-element.
///
/// Invariant: `combinators.len() + 1 == compounds.len()`.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexSelector {
    /// Compound selectors, left to right.
    pub compounds: Vec<CompoundSelector>,
    /// Combinator between `compounds[i]` and `compounds[i + 1]`.
    pub combinators: Vec<Combinator>,
    /// Pseudo-element this selector targets, if any (spec 12.5).
    pub pseudo_element: Option<PseudoElement>,
    /// Specificity computed at parse time (spec 12.3).
    pub specificity: Specificity,
    /// Source span of the whole selector.
    pub span: Span,
}

/// A compound selector: simple selectors with no combinator between them.
#[derive(Debug, Clone, PartialEq)]
pub struct CompoundSelector {
    /// Simple selectors, all of which must match the same element.
    pub simple: Vec<SimpleSelector>,
}

/// Combinators (spec 12.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Whitespace: some ancestor.
    Descendant,
    /// `>`: the parent.
    Child,
    /// `+`: the previous element sibling.
    NextSibling,
    /// `~`: any previous element sibling.
    SubsequentSibling,
}

/// A simple selector.
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleSelector {
    /// `*`
    Universal,
    /// Type selector, local name only (namespaces are not supported yet).
    Type(String),
    /// `#id`
    Id(String),
    /// `.class`
    Class(String),
    /// `[attr]`, `[attr=value]`, and the other attribute operators.
    Attribute(AttributeSelector),
    /// A state or structural pseudo-class with no arguments.
    PseudoClass(PseudoClass),
    /// `:nth-child()`, `:nth-last-child()`, `:nth-of-type()`,
    /// `:nth-last-of-type()`, and the forms that expand to them
    /// (`:first-child`, `:last-of-type`, and so on).
    Nth(NthSelector),
    /// `:not(list)`
    Not(SelectorList),
    /// `:is(list)`
    Is(SelectorList),
    /// `:where(list)`
    Where(SelectorList),
    /// `:lang(tag)`
    Lang(String),
    /// `:dir(ltr | rtl)`
    Dir(Direction),
    /// `&`, the parent selector in a nested rule. Replaced by the parent list
    /// when the nested rule is built, so it never reaches the matcher.
    Nesting,
    /// `:has(relatives)`: an element that has a related element matching one of
    /// the relative selectors (Selectors 4 section 4.5).
    Has(Vec<RelativeSelector>),
    /// `:scope`: the scope root of the enclosing `@scope`, or the root element.
    Scope,
    /// `:host` (`None`) or `:host(list)`: the shadow host of the sheet's shadow root
    /// (spec 12.2 Tier 2, Shadow DOM 1 section 3).
    Host(Option<SelectorList>),
    /// `:host-context(list)`: the host, or an ancestor of the host, matches `list`.
    HostContext(SelectorList),
}

/// The `An+B` family of structural selectors.
#[derive(Debug, Clone, PartialEq)]
pub struct NthSelector {
    /// Step `A`.
    pub a: i64,
    /// Offset `B`.
    pub b: i64,
    /// Count only siblings with the same name (`-of-type` forms).
    pub of_type: bool,
    /// Count from the end (`-last-` forms).
    pub last: bool,
    /// `of S` filter for `:nth-child`. Matches only siblings that match `S`.
    pub of: Option<SelectorList>,
}

/// An attribute selector.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeSelector {
    /// Attribute name as written.
    pub name: String,
    /// Optional operator and value.
    pub matcher: Option<AttributeMatcher>,
}

/// The matcher of an attribute selector.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeMatcher {
    /// Operator.
    pub operator: AttributeOperator,
    /// Expected value.
    pub value: String,
}

/// Attribute matching operators (spec 12.2 Tier 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeOperator {
    /// `=`
    Equals,
    /// `~=`: whitespace-separated list contains the value.
    Includes,
    /// `|=`: equal, or starts with the value followed by `-`.
    DashMatch,
    /// `^=`
    Prefix,
    /// `$=`
    Suffix,
    /// `*=`
    Substring,
}

/// Pseudo-classes without arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoClass {
    /// `:root`
    Root,
    /// `:hover` (adapter-provided state)
    Hover,
    /// `:active` (adapter-provided state)
    Active,
    /// `:focus` (adapter-provided state)
    Focus,
    /// `:empty`
    Empty,
    /// `:enabled`
    Enabled,
    /// `:disabled`
    Disabled,
    /// `:checked`
    Checked,
    /// `:required`
    Required,
    /// `:optional`
    Optional,
    /// `:read-only`
    ReadOnly,
    /// `:read-write`
    ReadWrite,
    /// `:placeholder-shown`
    PlaceholderShown,
    /// `:link`
    Link,
    /// `:any-link`
    AnyLink,
    /// `:visited`. Subject to the visited-link policy (spec 20.2).
    Visited,
    /// `:focus-visible`
    FocusVisible,
    /// `:focus-within`
    FocusWithin,
    /// `:target`
    Target,
    /// `:valid`
    Valid,
    /// `:invalid`
    Invalid,
    /// `:user-valid`
    UserValid,
    /// `:user-invalid`
    UserInvalid,
    /// `:in-range`
    InRange,
    /// `:out-of-range`
    OutOfRange,
    /// `:default`
    Default,
    /// `:indeterminate`
    Indeterminate,
    /// `:open`
    Open,
    /// `:closed`
    Closed,
    /// `:modal`
    Modal,
    /// `:popover-open`
    PopoverOpen,
    /// `:current`
    Current,
    /// `:past`
    Past,
    /// `:future`
    Future,
    /// `:defined`
    Defined,
    /// `:local-link`
    LocalLink,
    /// `:playing`
    Playing,
    /// `:paused`
    Paused,
    /// `:fullscreen`
    Fullscreen,
    /// `:autofill`
    Autofill,
}

/// A relative selector inside `:has()`: a complex selector whose subject is
/// related to the anchor element by `leading` (a descendant when no combinator is
/// written).
#[derive(Debug, Clone, PartialEq)]
pub struct RelativeSelector {
    /// The relation between the anchor and the leftmost compound.
    pub leading: Combinator,
    /// The complex selector, without its leading combinator.
    pub selector: ComplexSelector,
}
