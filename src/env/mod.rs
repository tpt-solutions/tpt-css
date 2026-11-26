// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Evaluation environment (spec 16.2).
//!
//! M1 adds the media environment (media type, color scheme, reduced motion,
//! hover, and pointer), which media queries are evaluated against. Container
//! providers (M2), font metrics, URL resolution, and `env()` variables are
//! later additions with the same shape.

pub mod media;

use crate::ast::PointerKind;

/// Viewport size in CSS px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportInfo {
    /// Width in px.
    pub width: f64,
    /// Height in px.
    pub height: f64,
}

/// The output medium (spec 16.3 `screen`, `print`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Screen output.
    Screen,
    /// Print output.
    Print,
}

/// User and device preferences that media features read (spec 16.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaEnvironment {
    /// The output medium.
    pub kind: MediaKind,
    /// `prefers-color-scheme: dark`.
    pub prefers_dark: bool,
    /// `prefers-reduced-motion: reduce`.
    pub reduced_motion: bool,
    /// `hover: hover` (the primary input can hover).
    pub hover: bool,
    /// `pointer` of the primary input.
    pub pointer: PointerKind,
}

impl Default for MediaEnvironment {
    fn default() -> Self {
        MediaEnvironment {
            kind: MediaKind::Screen,
            prefers_dark: false,
            reduced_motion: false,
            hover: true,
            pointer: PointerKind::Fine,
        }
    }
}

/// Environment passed to [`crate::StyleResolver::resolve`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    /// Viewport size.
    pub viewport: ViewportInfo,
    /// Font size of the root element's initial value, in px. Used for `rem`
    /// on the root element itself and as the parent size of the root.
    pub root_font_size: f64,
    /// Media features.
    pub media: MediaEnvironment,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            viewport: ViewportInfo {
                width: 800.0,
                height: 600.0,
            },
            root_font_size: 16.0,
            media: MediaEnvironment::default(),
        }
    }
}
