// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Token kinds (spec 10.1, CSS Syntax 3 section 4).

use super::span::Span;

/// A CSS token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is, with its data.
    pub kind: TokenKind,
    /// Source range the token was read from.
    pub span: Span,
}

/// The numeric type flag of a number token (CSS Syntax 3 section 4.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericType {
    /// Written without a fraction or exponent.
    Integer,
    /// Written with a fraction or exponent.
    Number,
}

/// Token kinds. Names follow CSS Syntax 3 and spec 10.1.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// `ident`
    Ident(String),
    /// `function`: an identifier immediately followed by `(`.
    Function(String),
    /// `at-keyword`: `@` followed by an identifier (name without `@`).
    AtKeyword(String),
    /// `hash`. `is_id` is true when the value would be a valid identifier.
    Hash {
        /// Hash name without `#`.
        value: String,
        /// Whether the name would start an identifier.
        is_id: bool,
    },
    /// `string`
    String(String),
    /// `bad-string`
    BadString,
    /// `url` token (unquoted `url(...)`).
    Url(String),
    /// `bad-url`
    BadUrl,
    /// `delim`: any other single code point.
    Delim(char),
    /// `number`
    Number {
        /// Numeric value.
        value: f64,
        /// Integer or number flag.
        ty: NumericType,
    },
    /// `percentage`
    Percentage(f64),
    /// `dimension`: a number with a unit identifier.
    Dimension {
        /// Numeric value.
        value: f64,
        /// Integer or number flag.
        ty: NumericType,
        /// Unit identifier, case preserved.
        unit: String,
    },
    /// `whitespace`
    Whitespace,
    /// `CDO` (`<!--`)
    Cdo,
    /// `CDC` (`-->`)
    Cdc,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `[`
    LeftSquare,
    /// `]`
    RightSquare,
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `{`
    LeftCurly,
    /// `}`
    RightCurly,
    /// End of input.
    Eof,
}

impl TokenKind {
    /// True for `whitespace`.
    pub fn is_whitespace(&self) -> bool {
        matches!(self, TokenKind::Whitespace)
    }
}
