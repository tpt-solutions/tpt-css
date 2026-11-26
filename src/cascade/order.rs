// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Declaration order (spec 13.3). A total, deterministic order over every
//! declaration that can compete in the cascade. No hash-map iteration order is
//! involved (spec 21.3).

/// Position of a declaration: which sheet, which rule, which declaration.
///
/// `sheet` is the sheet's position in the resolver's sheet list, which is the
/// order in which sheets were added. Inline declarations use `u32::MAX`, so
/// they come after every sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclarationOrder {
    /// Sheet position, or `u32::MAX` for inline declarations.
    pub sheet: u32,
    /// Rule index within the sheet.
    pub rule: u32,
    /// Declaration index within the rule.
    pub declaration: u32,
}

impl DeclarationOrder {
    /// Order for an inline declaration at `index` in the `style` attribute.
    pub const fn inline(index: u32) -> DeclarationOrder {
        DeclarationOrder {
            sheet: u32::MAX,
            rule: 0,
            declaration: index,
        }
    }
}
