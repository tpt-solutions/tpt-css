// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Evaluation environment (spec 16.2).
//!
//! M0 has only the viewport and the root font size. Media queries (M1),
//! container providers (M2), font metrics, URL resolution, user preferences,
//! and `env()` variables are later additions with the same shape.

/// Viewport size in CSS px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportInfo {
    /// Width in px.
    pub width: f64,
    /// Height in px.
    pub height: f64,
}

/// Environment passed to [`crate::StyleResolver::resolve`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    /// Viewport size.
    pub viewport: ViewportInfo,
    /// Font size of the root element's initial value, in px. Used for `rem`
    /// on the root element itself and as the parent size of the root.
    pub root_font_size: f64,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            viewport: ViewportInfo {
                width: 800.0,
                height: 600.0,
            },
            root_font_size: 16.0,
        }
    }
}
