// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Resource limits for hostile input (spec 19.2, 20.3).
//!
//! Every limit is configurable. Exceeding a limit produces a diagnostic and
//! the engine recovers without panicking.

/// Configurable bounds applied while parsing and matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    /// Maximum input size in bytes. Input beyond this is not tokenized.
    pub max_input_bytes: usize,
    /// Maximum number of tokens produced from one input.
    pub max_tokens: usize,
    /// Maximum nesting of blocks and functions in component values.
    pub max_nesting_depth: usize,
    /// Maximum number of compound selectors in one complex selector.
    pub max_selector_complexity: usize,
    /// Maximum declarations in one declaration block.
    pub max_declarations_per_block: usize,
    /// Maximum rules in one stylesheet.
    pub max_rules_per_sheet: usize,
    /// Maximum `var()` substitution depth (used from M1).
    pub max_var_depth: usize,
    /// Maximum calc() nesting depth (used from M1).
    pub max_calc_depth: usize,
    /// Maximum selector-matching steps per node per selector.
    pub max_match_steps: usize,
    /// Maximum stylesheets added to one resolver (spec 20.3).
    pub max_stylesheets: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        ParseLimits {
            max_input_bytes: 16 * 1024 * 1024,
            max_tokens: 4_000_000,
            max_nesting_depth: 256,
            max_selector_complexity: 64,
            max_declarations_per_block: 4096,
            max_rules_per_sheet: 1_000_000,
            max_var_depth: 32,
            max_calc_depth: 32,
            max_match_steps: 100_000,
            max_stylesheets: 4096,
        }
    }
}
