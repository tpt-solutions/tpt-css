// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Specificity (spec 12.3; Selectors 4, specificity).
//!
//! Computed at parse time. Ordering is lexicographic on `(a, b, c)`, which
//! the derived `Ord` implements because fields are declared in that order.

use crate::ast::selector::{ComplexSelector, SelectorList, SimpleSelector};

/// Explicit specificity triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity {
    /// Number of ID selectors.
    pub a: u32,
    /// Number of class, attribute, and pseudo-class selectors.
    pub b: u32,
    /// Number of type and pseudo-element selectors.
    pub c: u32,
}

impl Specificity {
    fn add(self, other: Specificity) -> Specificity {
        Specificity {
            a: self.a + other.a,
            b: self.b + other.b,
            c: self.c + other.c,
        }
    }

    /// The most specific member of a selector list (Selectors 4: `:is()`,
    /// `:not()`, and `:has()` use the most specific argument).
    pub fn of_list(list: &SelectorList) -> Specificity {
        list.selectors
            .iter()
            .map(Specificity::of_complex)
            .max()
            .unwrap_or_default()
    }

    /// Specificity of a single simple selector (spec 12.3 rules).
    pub fn of_simple(simple: &SimpleSelector) -> Specificity {
        match simple {
            SimpleSelector::Universal | SimpleSelector::Nesting => Specificity::default(),
            SimpleSelector::Type(_) => Specificity { a: 0, b: 0, c: 1 },
            SimpleSelector::Id(_) => Specificity { a: 1, b: 0, c: 0 },
            SimpleSelector::Class(_)
            | SimpleSelector::Attribute(_)
            | SimpleSelector::PseudoClass(_)
            | SimpleSelector::Lang(_)
            | SimpleSelector::Dir(_)
            | SimpleSelector::Scope => Specificity { a: 0, b: 1, c: 0 },
            SimpleSelector::Has(relatives) => relatives
                .iter()
                .map(|r| Specificity::of_complex(&r.selector))
                .max()
                .unwrap_or_default(),
            SimpleSelector::Nth(nth) => {
                let base = Specificity { a: 0, b: 1, c: 0 };
                match &nth.of {
                    Some(list) => base.add(Specificity::of_list(list)),
                    None => base,
                }
            }
            SimpleSelector::Not(list) | SimpleSelector::Is(list) => Specificity::of_list(list),
            SimpleSelector::Where(_) => Specificity::default(),
        }
    }

    /// Specificity of a complex selector: the sum over all its simple selectors,
    /// plus one type-level point for a pseudo-element.
    pub fn of_complex(selector: &ComplexSelector) -> Specificity {
        let mut total = Specificity::default();
        for compound in &selector.compounds {
            for simple in &compound.simple {
                total = total.add(Specificity::of_simple(simple));
            }
        }
        if selector.pseudo_element.is_some() {
            total = total.add(Specificity { a: 0, b: 0, c: 1 });
        }
        total
    }
}
