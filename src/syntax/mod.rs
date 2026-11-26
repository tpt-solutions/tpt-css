// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Syntax layer: tokens, spans, and component values (spec 10).
//!
//! Implements CSS Syntax Level 3 sections 4 (tokenizing) and 5.3 (component
//! values). Nothing here knows about properties or selectors.

pub mod component;
pub mod span;
pub mod token;
pub mod tokenizer;

pub use component::{trim_whitespace, BlockKind, ComponentValue, Function, SimpleBlock};
pub use span::{SourceId, Span};
pub use token::{NumericType, Token, TokenKind};
pub use tokenizer::{tokenize, Tokenized};
