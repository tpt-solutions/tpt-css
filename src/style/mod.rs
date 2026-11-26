// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Style resolution, computed values, and the style map (spec 8, 16).
//!
//! Inheritance and CSS-wide keyword handling live in [`resolver`], next to the
//! cascade call they depend on. Invalidation and style sharing (M3) will be
//! added here as separate modules.

pub mod computed;
pub mod resolver;

pub use computed::{ComputedStyle, StyleMap};
pub use resolver::{ResolverConfig, SheetId, StyleResolver};
