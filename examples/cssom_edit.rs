// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Editing a stylesheet through the CSSOM API (feature `cssom`). Each edit
//! returns the `StyleChange` a host passes to the resolver, so only the affected
//! styles are recomputed.
//!
//! Run with `cargo run --example cssom_edit --features cssom`.

use tpt_css::cssom::CssomSheet;

/// Applies a short list of edits to a sheet and returns its serialized text.
pub fn edited_sheet() -> String {
    let mut sheet = CssomSheet::parse("p { color: red }");

    // Add a second rule at the end, then change the first rule's declaration.
    sheet
        .insert_rule(".note { font-style: italic }", 1)
        .expect("valid rule at a valid index");
    sheet
        .set_property(0, "color", "blue")
        .expect("rule 0 is a style rule");

    sheet.serialize()
}

fn main() {
    println!("{}", edited_sheet());
}
