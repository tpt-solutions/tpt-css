// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Style resolution, computed values, custom properties, and invalidation
//! (spec 8, 13.7, 16, 17).
//!
//! Inheritance and CSS-wide keyword handling live in [`resolver`], next to the
//! cascade call they depend on. Custom properties and `var()` substitution are
//! in [`custom`]. Incremental restyle is in [`invalidation`].

pub mod computed;
pub mod custom;
pub mod inspect;
pub mod invalidation;
pub mod keyframes;
pub mod resolver;

pub use computed::{ComputedStyle, StyleMap};
pub use custom::CustomPropertyValue;
pub use invalidation::StyleChange;
pub use resolver::{ResolverConfig, SheetId, StyleResolver};
