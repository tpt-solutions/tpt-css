// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Pseudo-element identities (spec 12.5). Pseudo-element selectors are parsed
//! in M1; only the identity type is defined here so node kinds can name them.

/// A pseudo-element.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PseudoElement {
    /// `::before`
    Before,
    /// `::after`
    After,
    /// `::marker`
    Marker,
    /// `::placeholder`
    Placeholder,
    /// `::selection`
    Selection,
    /// `::backdrop`
    Backdrop,
    /// `::file-selector-button`
    FileSelectorButton,
    /// `::part(name)`
    Part(String),
    /// `::slotted()`
    Slotted,
    /// `::highlight(name)`
    Highlight(String),
    /// Any other pseudo-element name, kept for diagnostics.
    Unknown(String),
}
