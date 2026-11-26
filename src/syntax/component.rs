// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Component values (spec 10.2, CSS Syntax 3, component values).
//!
//! A component value is a preserved token, a function, or a simple block
//! (`()`, `[]`, `{}`). Values are not interpreted as properties here; that
//! happens in the property layer.
//!
//! Nesting is built iteratively with an explicit stack, so hostile input
//! cannot overflow the call stack. Exceeding `max_nesting_depth` makes the
//! whole build fail; the caller reports that as a diagnostic.

use super::span::Span;
use super::token::{Token, TokenKind};

/// A component value.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentValue {
    /// A preserved token (anything that is not a block opener or function).
    Token(Token),
    /// A function such as `rgb(...)`.
    Function(Function),
    /// A simple block such as `(...)`, `[...]`, or `{...}`.
    Block(SimpleBlock),
}

/// A function component value.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Function name, case preserved.
    pub name: String,
    /// Contained component values.
    pub value: Vec<ComponentValue>,
    /// Span from the function name through its closing `)`, or to EOF.
    pub span: Span,
}

/// The kind of a simple block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// `( ... )`
    Paren,
    /// `[ ... ]`
    Square,
    /// `{ ... }`
    Curly,
}

/// A simple block component value.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleBlock {
    /// Which delimiters enclose the block.
    pub kind: BlockKind,
    /// Contained component values.
    pub value: Vec<ComponentValue>,
    /// Span from the opening delimiter through the closing one, or to EOF.
    pub span: Span,
}

impl ComponentValue {
    /// Returns the span of this component value.
    pub fn span(&self) -> Span {
        match self {
            ComponentValue::Token(t) => t.span,
            ComponentValue::Function(f) => f.span,
            ComponentValue::Block(b) => b.span,
        }
    }

    /// True if this is a whitespace token.
    pub fn is_whitespace(&self) -> bool {
        matches!(self, ComponentValue::Token(t) if t.kind.is_whitespace())
    }

    /// True if this is a token of the given kind (compared with `==`).
    pub fn is_token(&self, kind: &TokenKind) -> bool {
        matches!(self, ComponentValue::Token(t) if &t.kind == kind)
    }
}

/// Removes leading and trailing whitespace tokens.
pub fn trim_whitespace(values: &[ComponentValue]) -> &[ComponentValue] {
    let start = values
        .iter()
        .position(|v| !v.is_whitespace())
        .unwrap_or(values.len());
    let end = values
        .iter()
        .rposition(|v| !v.is_whitespace())
        .map_or(start, |i| i + 1);
    &values[start..end.max(start)]
}

/// Error returned when nesting exceeds the configured limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestingExceeded;

enum Opener {
    Root,
    Function(String),
    Block(BlockKind),
}

struct Frame {
    opener: Opener,
    open_span: Span,
    children: Vec<ComponentValue>,
}

/// Builds component values from a token stream (the final `Eof` is ignored).
///
/// `max_depth` bounds how many blocks and functions may be open at once.
pub fn build_component_values(
    tokens: &[Token],
    max_depth: usize,
) -> Result<Vec<ComponentValue>, NestingExceeded> {
    let root_span = tokens.first().map_or(Span::default(), |t| t.span);
    let mut stack = vec![Frame {
        opener: Opener::Root,
        open_span: root_span,
        children: Vec::new(),
    }];

    for tok in tokens {
        match &tok.kind {
            TokenKind::Eof => break,
            TokenKind::Function(name) => {
                if stack.len() > max_depth {
                    return Err(NestingExceeded);
                }
                stack.push(Frame {
                    opener: Opener::Function(name.clone()),
                    open_span: tok.span,
                    children: Vec::new(),
                });
            }
            TokenKind::LeftParen => push_block(&mut stack, BlockKind::Paren, tok.span, max_depth)?,
            TokenKind::LeftSquare => {
                push_block(&mut stack, BlockKind::Square, tok.span, max_depth)?
            }
            TokenKind::LeftCurly => push_block(&mut stack, BlockKind::Curly, tok.span, max_depth)?,
            TokenKind::RightParen => close_if(&mut stack, BlockKind::Paren, tok.span, true),
            TokenKind::RightSquare => close_if(&mut stack, BlockKind::Square, tok.span, false),
            TokenKind::RightCurly => close_if(&mut stack, BlockKind::Curly, tok.span, false),
            _ => {
                let top = stack.last_mut().expect("root frame is never popped");
                top.children.push(ComponentValue::Token(tok.clone()));
            }
        }
    }

    // Close anything still open at EOF.
    while stack.len() > 1 {
        let frame = stack.pop().expect("length checked");
        let value = finish(frame, None);
        stack
            .last_mut()
            .expect("root frame is never popped")
            .children
            .push(value);
    }

    let root = stack.pop().expect("root frame is never popped");
    Ok(root.children)
}

fn push_block(
    stack: &mut Vec<Frame>,
    kind: BlockKind,
    span: Span,
    max_depth: usize,
) -> Result<(), NestingExceeded> {
    if stack.len() > max_depth {
        return Err(NestingExceeded);
    }
    stack.push(Frame {
        opener: Opener::Block(kind),
        open_span: span,
        children: Vec::new(),
    });
    Ok(())
}

/// Closes the top frame if it matches `kind`. A `)` closes a function too.
/// Otherwise the closer is kept as a preserved token.
fn close_if(stack: &mut Vec<Frame>, kind: BlockKind, span: Span, closes_functions: bool) {
    let matches = match stack.last().map(|f| &f.opener) {
        Some(Opener::Block(k)) => *k == kind,
        Some(Opener::Function(_)) => closes_functions,
        _ => false,
    };
    if matches && stack.len() > 1 {
        let frame = stack.pop().expect("length checked");
        let value = finish(frame, Some(span));
        stack
            .last_mut()
            .expect("root frame is never popped")
            .children
            .push(value);
    } else {
        let kind_tok = match kind {
            BlockKind::Paren => TokenKind::RightParen,
            BlockKind::Square => TokenKind::RightSquare,
            BlockKind::Curly => TokenKind::RightCurly,
        };
        let top = stack.last_mut().expect("root frame is never popped");
        top.children.push(ComponentValue::Token(Token {
            kind: kind_tok,
            span,
        }));
    }
}

fn finish(frame: Frame, close_span: Option<Span>) -> ComponentValue {
    let span = match close_span {
        Some(c) => frame.open_span.join(c),
        None => frame.open_span,
    };
    match frame.opener {
        Opener::Function(name) => ComponentValue::Function(Function {
            name,
            value: frame.children,
            span,
        }),
        Opener::Block(kind) => ComponentValue::Block(SimpleBlock {
            kind,
            value: frame.children,
            span,
        }),
        Opener::Root => unreachable!("root frame is never finished as a value"),
    }
}
