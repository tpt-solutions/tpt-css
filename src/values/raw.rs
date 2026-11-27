// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Pass-through values (spec 14.1, 15.2). Some properties are registered only
//! so that a downstream layout or paint engine can read them. Today that is
//! `grid`. This engine does not parse its grammar. It keeps their component values as canonical text, so that a
//! host can read the cascaded value and parse it itself.
//!
//! The text is canonical, not the author's spelling: whitespace is collapsed to
//! single spaces, and strings are written with double quotes.

use super::serialize::format_number;
use crate::syntax::{trim_whitespace, BlockKind, ComponentValue, TokenKind};

/// Whether a value can be passed through. The tokenizer reports bad strings and
/// bad URLs as tokens rather than failing, and CSS drops a declaration that
/// contains them, so they are rejected here.
pub(crate) fn is_passable(values: &[ComponentValue]) -> bool {
    values.iter().all(|v| match v {
        ComponentValue::Token(t) => !matches!(t.kind, TokenKind::BadString | TokenKind::BadUrl),
        ComponentValue::Function(f) => is_passable(&f.value),
        ComponentValue::Block(b) => is_passable(&b.value),
    })
}

/// The canonical text of a list of component values.
pub fn serialize_components(values: &[ComponentValue]) -> String {
    let mut out = String::new();
    write_components(values, &mut out);
    out.trim().to_string()
}

fn write_components(values: &[ComponentValue], out: &mut String) {
    for value in values {
        match value {
            ComponentValue::Token(t) => write_token(&t.kind, out),
            ComponentValue::Function(f) => {
                out.push_str(&f.name);
                out.push('(');
                write_components(trim_whitespace(&f.value), out);
                out.push(')');
            }
            ComponentValue::Block(b) => {
                let (open, close) = match b.kind {
                    BlockKind::Paren => ('(', ')'),
                    BlockKind::Square => ('[', ']'),
                    BlockKind::Curly => ('{', '}'),
                };
                out.push(open);
                write_components(trim_whitespace(&b.value), out);
                out.push(close);
            }
        }
    }
}

fn write_token(kind: &TokenKind, out: &mut String) {
    match kind {
        TokenKind::Whitespace => {
            // Collapse runs of whitespace into one space.
            if !out.ends_with(' ') {
                out.push(' ');
            }
        }
        TokenKind::Ident(s) => out.push_str(s),
        TokenKind::Function(s) => {
            out.push_str(s);
            out.push('(');
        }
        TokenKind::AtKeyword(s) => {
            out.push('@');
            out.push_str(s);
        }
        TokenKind::Hash { value, .. } => {
            out.push('#');
            out.push_str(value);
        }
        TokenKind::String(s) => write_quoted(s, out),
        TokenKind::Url(s) => {
            out.push_str("url(");
            write_quoted(s, out);
            out.push(')');
        }
        TokenKind::Delim(c) => out.push(*c),
        TokenKind::Number { value, .. } => out.push_str(&format_number(*value)),
        TokenKind::Percentage(p) => {
            out.push_str(&format_number(*p));
            out.push('%');
        }
        TokenKind::Dimension { value, unit, .. } => {
            out.push_str(&format_number(*value));
            out.push_str(unit);
        }
        TokenKind::Cdo => out.push_str("<!--"),
        TokenKind::Cdc => out.push_str("-->"),
        TokenKind::Colon => out.push(':'),
        TokenKind::Semicolon => out.push(';'),
        TokenKind::Comma => out.push(','),
        TokenKind::LeftSquare => out.push('['),
        TokenKind::RightSquare => out.push(']'),
        TokenKind::LeftParen => out.push('('),
        TokenKind::RightParen => out.push(')'),
        TokenKind::LeftCurly => out.push('{'),
        TokenKind::RightCurly => out.push('}'),
        // Bad tokens never reach here: `is_passable` rejects them. Eof is not
        // part of a component list.
        TokenKind::BadString | TokenKind::BadUrl | TokenKind::Eof => {}
    }
}

fn write_quoted(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
}
