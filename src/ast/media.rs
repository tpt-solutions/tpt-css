// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Media query AST (spec 16.3). Parsing is here; evaluation against an
//! environment is in [`crate::env::media`], so matching and evaluation stay
//! separate.

use crate::values::LengthUnit;

/// One media query: `[not|only] type [and (feature)]*`.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaQuery {
    /// `not` prefix.
    pub negated: bool,
    /// Media type. `All` when the query has no type.
    pub media_type: MediaType,
    /// Feature conditions, all of which must hold.
    pub features: Vec<MediaFeature>,
}

/// Media types (spec 16.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    /// `all`, or no type given.
    All,
    /// `screen`
    Screen,
    /// `print`
    Print,
    /// Any other type. Never matches.
    Unknown(String),
}

/// How a numeric feature is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// `width: x`
    Equal,
    /// `min-width: x`
    Min,
    /// `max-width: x`
    Max,
}

/// Pointer capability (spec 16.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerKind {
    /// No pointing device.
    None,
    /// Limited accuracy (touch).
    Coarse,
    /// Precise (mouse).
    Fine,
}

/// A feature condition inside parentheses.
#[derive(Debug, Clone, PartialEq)]
pub enum MediaFeature {
    /// `width`, `min-width`, `max-width` against a length.
    Width(Comparison, f64, LengthUnit),
    /// `height`, `min-height`, `max-height` against a length.
    Height(Comparison, f64, LengthUnit),
    /// `aspect-ratio`, `min-aspect-ratio`, `max-aspect-ratio` as width/height.
    AspectRatio(Comparison, f64),
    /// `orientation: portrait | landscape`. `true` means landscape.
    Landscape(bool),
    /// `prefers-color-scheme: dark`. `true` means dark.
    PrefersDark(bool),
    /// `prefers-reduced-motion: reduce`. `true` means reduce.
    PrefersReducedMotion(bool),
    /// `hover: hover`. `true` means the primary input can hover.
    Hover(bool),
    /// `pointer: none | coarse | fine`.
    Pointer(PointerKind),
    /// `prefers-contrast: no-preference | more | less | custom`.
    PrefersContrast(ContrastKind),
    /// `dynamic-range: standard | high`. `true` means high.
    DynamicRange(bool),
    /// `update: none | slow | fast`.
    Update(UpdateKind),
    /// `resolution`, `min-resolution`, `max-resolution` in device pixels per
    /// CSS pixel (`dppx`). `dpi` and `dpcm` are converted at parse time.
    Resolution(Comparison, f64),
    /// A feature this engine does not know. The query never matches.
    Unknown(String),
}

/// Contrast preference (Media Queries 5, `prefers-contrast`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContrastKind {
    /// `no-preference`
    NoPreference,
    /// `more`
    More,
    /// `less`
    Less,
    /// `custom`
    Custom,
}

/// How often the output can change (Media Queries 5, `update`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateKind {
    /// `none`, such as print.
    None,
    /// `slow`, such as e-ink.
    Slow,
    /// `fast`, such as a screen.
    Fast,
}
