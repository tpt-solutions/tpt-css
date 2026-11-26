// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cascade origins and importance (spec 13.2; CSS Cascade 5, origins).

/// The origin of a declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// User-agent (default) styles.
    UserAgent,
    /// User styles.
    User,
    /// Author styles: the page's own stylesheets and inline styles.
    Author,
    /// CSS animation declarations.
    Animation,
    /// CSS transition declarations.
    Transition,
}

impl Origin {
    /// Precedence rank of an origin and importance pair, from lowest (0) to
    /// highest. This is the spec 13.2 order exactly:
    ///
    /// ```text
    /// normal UA < normal user < normal author < animations
    ///   < important author < important user < important UA < transitions
    /// ```
    ///
    /// Animation and transition declarations have one rank each. CSS Cascade 5
    /// does not define an important flag for them, so `important` is ignored.
    /// The origin's position in the origin list, ignoring importance. Used by
    /// `revert`, which rolls back to the origin below (spec 13.8).
    pub const fn index(self) -> u8 {
        match self {
            Origin::UserAgent => 0,
            Origin::User => 1,
            Origin::Author => 2,
            Origin::Animation => 3,
            Origin::Transition => 4,
        }
    }

    pub const fn precedence(self, important: bool) -> u8 {
        match (self, important) {
            (Origin::UserAgent, false) => 0,
            (Origin::User, false) => 1,
            (Origin::Author, false) => 2,
            (Origin::Animation, _) => 3,
            (Origin::Author, true) => 4,
            (Origin::User, true) => 5,
            (Origin::UserAgent, true) => 6,
            (Origin::Transition, _) => 7,
        }
    }
}
