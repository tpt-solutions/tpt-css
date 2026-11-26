// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Element state (spec 9.2). The core does not know whether a bit came from
//! HTML, AppFront, or another host.

/// A set of state bits. Bit names follow spec 9.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StateBits(u64);

impl StateBits {
    /// `:hover`
    pub const HOVER: StateBits = StateBits(1 << 0);
    /// `:active`
    pub const ACTIVE: StateBits = StateBits(1 << 1);
    /// `:focus`
    pub const FOCUS: StateBits = StateBits(1 << 2);
    /// `:focus-visible`
    pub const FOCUS_VISIBLE: StateBits = StateBits(1 << 3);
    /// `:focus-within`
    pub const FOCUS_WITHIN: StateBits = StateBits(1 << 4);
    /// `:visited`
    pub const VISITED: StateBits = StateBits(1 << 5);
    /// `:link`
    pub const LINK: StateBits = StateBits(1 << 6);
    /// `:any-link`
    pub const ANY_LINK: StateBits = StateBits(1 << 7);
    /// `:checked`
    pub const CHECKED: StateBits = StateBits(1 << 8);
    /// `:disabled`
    pub const DISABLED: StateBits = StateBits(1 << 9);
    /// `:enabled`
    pub const ENABLED: StateBits = StateBits(1 << 10);
    /// `:required`
    pub const REQUIRED: StateBits = StateBits(1 << 11);
    /// `:optional`
    pub const OPTIONAL: StateBits = StateBits(1 << 12);
    /// `:valid`
    pub const VALID: StateBits = StateBits(1 << 13);
    /// `:invalid`
    pub const INVALID: StateBits = StateBits(1 << 14);
    /// `:user-valid`
    pub const USER_VALID: StateBits = StateBits(1 << 15);
    /// `:user-invalid`
    pub const USER_INVALID: StateBits = StateBits(1 << 16);
    /// `:in-range`
    pub const IN_RANGE: StateBits = StateBits(1 << 17);
    /// `:out-of-range`
    pub const OUT_OF_RANGE: StateBits = StateBits(1 << 18);
    /// `:read-only`
    pub const READ_ONLY: StateBits = StateBits(1 << 19);
    /// `:read-write`
    pub const READ_WRITE: StateBits = StateBits(1 << 20);
    /// `:placeholder-shown`
    pub const PLACEHOLDER_SHOWN: StateBits = StateBits(1 << 21);
    /// `:default`
    pub const DEFAULT: StateBits = StateBits(1 << 22);
    /// `:indeterminate`
    pub const INDETERMINATE: StateBits = StateBits(1 << 23);
    /// `:target`
    pub const TARGET: StateBits = StateBits(1 << 24);
    /// `:open`
    pub const OPEN: StateBits = StateBits(1 << 25);
    /// `:closed`
    pub const CLOSED: StateBits = StateBits(1 << 26);
    /// `:modal`
    pub const MODAL: StateBits = StateBits(1 << 27);
    /// `:popover-open`
    pub const POPOVER_OPEN: StateBits = StateBits(1 << 28);

    /// No bits set.
    pub const fn empty() -> StateBits {
        StateBits(0)
    }

    /// Returns the union of two sets.
    pub const fn union(self, other: StateBits) -> StateBits {
        StateBits(self.0 | other.0)
    }

    /// True if every bit in `other` is set in `self`.
    pub const fn contains(self, other: StateBits) -> bool {
        self.0 & other.0 == other.0
    }
}

/// Author-defined custom states (`:state(name)`), kept sorted and unique so
/// that comparisons are deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomStateSet {
    names: Vec<String>,
}

impl CustomStateSet {
    /// Adds a custom state name.
    pub fn insert(&mut self, name: &str) {
        if let Err(i) = self.names.binary_search_by(|n| n.as_str().cmp(name)) {
            self.names.insert(i, name.to_string());
        }
    }

    /// True if the custom state is present.
    pub fn contains(&self, name: &str) -> bool {
        self.names
            .binary_search_by(|n| n.as_str().cmp(name))
            .is_ok()
    }
}

/// Full element state: built-in bits and custom states.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElementState {
    /// Built-in state bits.
    pub bits: StateBits,
    /// Custom states.
    pub custom: CustomStateSet,
}
