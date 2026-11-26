// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Specificity (spec 12.3; Selectors 4, specificity).
//!
//! Computed at parse time. Ordering is lexicographic on `(a, b, c)`, which
//! the derived `Ord` implements because fields are declared in that order.

use crate::ast::selector::{ComplexSelector, SimpleSelector};

/// Explicit specificity triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity {
    /// Number of ID selectors.
    pub a: u32,
    /// Number of class, attribute, and pseudo-class selectors.
    pub b: u32,
    /// Number of type selectors.
    pub c: u32,
}

impl Specificity {
    /// Specificity of a single simple selector (spec 12.3 rules).
    pub fn of_simple(simple: &SimpleSelector) -> Specificity {
        match simple {
            SimpleSelector::Universal => Specificity::default(),
            SimpleSelector::Type(_) => Specificity { a: 0, b: 0, c: 1 },
            SimpleSelector::Id(_) => Specificity { a: 1, b: 0, c: 0 },
            SimpleSelector::Class(_)
            | SimpleSelector::Attribute(_)
            | SimpleSelector::PseudoClass(_) => Specificity { a: 0, b: 1, c: 0 },
        }
    }

    /// Specificity of a complex selector: the sum over all its simple selectors.
    pub fn of_complex(selector: &ComplexSelector) -> Specificity {
        let mut total = Specificity::default();
        for compound in &selector.compounds {
            for simple in &compound.simple {
                let s = Specificity::of_simple(simple);
                total.a += s.a;
                total.b += s.b;
                total.c += s.c;
            }
        }
        total
    }
}
