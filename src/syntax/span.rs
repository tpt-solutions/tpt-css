// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Source spans (spec 10.3).

/// Identifies the source text a span refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SourceId(pub u32);

/// A half-open byte range `[start, end)` in one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span {
    /// Source the offsets refer to.
    pub source: SourceId,
    /// Start byte offset (inclusive).
    pub start: u32,
    /// End byte offset (exclusive).
    pub end: u32,
}

impl Span {
    /// Creates a span. `end` is clamped so that `end >= start`.
    pub fn new(source: SourceId, start: u32, end: u32) -> Span {
        Span {
            source,
            start,
            end: end.max(start),
        }
    }

    /// Returns a span covering both `self` and `other` (same source assumed).
    pub fn join(self, other: Span) -> Span {
        Span {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
