// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selectors: specificity, pseudo-element identities, and matching (spec 12).
//!
//! The selector parser lives in [`crate::parse::selectors`].

pub mod matching;
pub mod pseudo;
pub mod specificity;

pub use matching::{match_selector_list, matches_complex};
pub use pseudo::PseudoElement;
pub use specificity::Specificity;
