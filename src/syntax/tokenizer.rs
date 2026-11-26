// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! CSS tokenizer (spec 10.1), following the consume-token algorithm of
//! CSS Syntax Level 3, tokenization.
//!
//! Notes on this implementation:
//! - Spans refer to the original input bytes. Newline normalization
//!   (CR, CRLF, FF) is applied logically while reading, not by copying.
//! - Comments are discarded (spec 10.1: ignored by default). Comment
//!   preservation for tooling is not implemented yet.
//! - Input larger than `max_input_bytes` or longer than `max_tokens`
//!   produces no further tokens and sets `limit_exceeded`.

use super::span::{SourceId, Span};
use super::token::{NumericType, Token, TokenKind};
use crate::limits::ParseLimits;

/// Result of tokenizing one input.
#[derive(Debug, Clone, PartialEq)]
pub struct Tokenized {
    /// Tokens, always terminated by one [`TokenKind::Eof`].
    pub tokens: Vec<Token>,
    /// True when a limit stopped tokenization early.
    pub limit_exceeded: bool,
    /// The source range of every comment, in order. Comments produce no tokens.
    pub comments: Vec<Span>,
}

/// Tokenizes `input` according to `limits`.
pub fn tokenize(source: SourceId, input: &str, limits: &ParseLimits) -> Tokenized {
    let eof_span = Span::new(source, 0, 0);
    if input.len() > limits.max_input_bytes || input.len() > u32::MAX as usize {
        return Tokenized {
            comments: Vec::new(),
            tokens: vec![Token {
                kind: TokenKind::Eof,
                span: eof_span,
            }],
            limit_exceeded: true,
        };
    }

    let mut t = Tokenizer {
        src: input,
        pos: 0,
        source,
    };
    let mut tokens = Vec::new();
    let mut limit_exceeded = false;
    let mut comments = Vec::new();

    loop {
        // Comments produce no tokens (CSS Syntax 3, comments).
        while t.rest().starts_with("/*") {
            let start = t.pos as u32;
            t.skip_comment();
            comments.push(Span::new(source, start, t.pos as u32));
        }
        let start = t.pos;
        if t.pos >= t.src.len() {
            tokens.push(Token {
                kind: TokenKind::Eof,
                span: t.span(start),
            });
            break;
        }
        if tokens.len() + 1 >= limits.max_tokens {
            limit_exceeded = true;
            tokens.push(Token {
                kind: TokenKind::Eof,
                span: t.span(t.src.len()),
            });
            break;
        }
        let kind = t.consume_token();
        tokens.push(Token {
            kind,
            span: t.span(start),
        });
    }

    Tokenized {
        tokens,
        limit_exceeded,
        comments,
    }
}

/// True for the CSS newline code points after preprocessing (LF, CR, FF).
pub(crate) fn is_newline(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{0C}')
}

/// True for CSS whitespace.
pub(crate) fn is_ws(c: char) -> bool {
    is_newline(c) || c == ' ' || c == '\t'
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || (c as u32) >= 0x80
}

fn is_name(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-'
}

fn is_non_printable(c: char) -> bool {
    matches!(c as u32, 0x00..=0x08 | 0x0B | 0x0E..=0x1F | 0x7F)
}

/// Starts a valid escape: first is `\` and second is not a newline.
/// End of input after the backslash counts as valid (it yields U+FFFD).
fn escape_ok(a: Option<char>, b: Option<char>) -> bool {
    a == Some('\\') && !b.is_some_and(is_newline)
}

/// CSS "would start an identifier" over three code points.
fn would_start_ident(la: [Option<char>; 3]) -> bool {
    match la[0] {
        Some('-') => {
            matches!(la[1], Some(c) if is_name_start(c) || c == '-') || escape_ok(la[1], la[2])
        }
        Some(c) if is_name_start(c) => true,
        Some('\\') => escape_ok(la[0], la[1]),
        _ => false,
    }
}

/// CSS "would start a number" over three code points.
fn would_start_number(la: [Option<char>; 3]) -> bool {
    match la[0] {
        Some('+') | Some('-') => match la[1] {
            Some(c) if c.is_ascii_digit() => true,
            Some('.') => matches!(la[2], Some(c) if c.is_ascii_digit()),
            _ => false,
        },
        Some('.') => matches!(la[1], Some(c) if c.is_ascii_digit()),
        Some(c) => c.is_ascii_digit(),
        None => false,
    }
}

struct Tokenizer<'a> {
    src: &'a str,
    pos: usize,
    source: SourceId,
}

impl<'a> Tokenizer<'a> {
    fn rest(&self) -> &'a str {
        let src: &'a str = self.src;
        &src[self.pos..]
    }

    fn span(&self, start: usize) -> Span {
        Span::new(self.source, start as u32, self.pos as u32)
    }

    fn peek(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn lookahead_at(&self, n: usize) -> [Option<char>; 3] {
        let mut it = self.rest().chars().skip(n);
        [it.next(), it.next(), it.next()]
    }

    fn lookahead(&self) -> [Option<char>; 3] {
        self.lookahead_at(0)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.rest().chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_comment(&mut self) {
        // Caller checked the input starts with "/*".
        self.pos += 2;
        match self.rest().find("*/") {
            Some(i) => self.pos += i + 2,
            None => self.pos = self.src.len(),
        }
    }

    /// Consumes a newline sequence (CRLF counts as one).
    fn consume_newline(&mut self) {
        if self.peek(0) == Some('\r') && self.peek(1) == Some('\n') {
            self.bump();
            self.bump();
        } else if matches!(self.peek(0), Some(c) if is_newline(c)) {
            self.bump();
        }
    }

    /// Consume-a-token (CSS Syntax 3, consume a token). Precondition: not at EOF.
    fn consume_token(&mut self) -> TokenKind {
        let la = self.lookahead();
        let c = match la[0] {
            Some(c) => c,
            None => return TokenKind::Eof,
        };

        if is_ws(c) {
            while matches!(self.peek(0), Some(n) if is_ws(n)) {
                self.bump();
            }
            return TokenKind::Whitespace;
        }
        if c == '"' || c == '\'' {
            self.bump();
            return self.consume_string(c);
        }
        if c == '#' {
            let la1 = self.lookahead_at(1);
            self.bump();
            if matches!(la1[0], Some(n) if is_name(n)) || escape_ok(la1[0], la1[1]) {
                let is_id = would_start_ident(la1);
                let value = self.consume_name();
                return TokenKind::Hash { value, is_id };
            }
            return TokenKind::Delim('#');
        }
        if c == '(' {
            self.bump();
            return TokenKind::LeftParen;
        }
        if c == ')' {
            self.bump();
            return TokenKind::RightParen;
        }
        if c == '[' {
            self.bump();
            return TokenKind::LeftSquare;
        }
        if c == ']' {
            self.bump();
            return TokenKind::RightSquare;
        }
        if c == '{' {
            self.bump();
            return TokenKind::LeftCurly;
        }
        if c == '}' {
            self.bump();
            return TokenKind::RightCurly;
        }
        if c == ',' {
            self.bump();
            return TokenKind::Comma;
        }
        if c == ':' {
            self.bump();
            return TokenKind::Colon;
        }
        if c == ';' {
            self.bump();
            return TokenKind::Semicolon;
        }
        if c == '+' || c == '.' {
            if would_start_number(la) {
                return self.consume_numeric();
            }
            self.bump();
            return TokenKind::Delim(c);
        }
        if c == '-' {
            if would_start_number(la) {
                return self.consume_numeric();
            }
            if self.rest().starts_with("-->") {
                self.pos += 3;
                return TokenKind::Cdc;
            }
            if would_start_ident(la) {
                return self.consume_ident_like();
            }
            self.bump();
            return TokenKind::Delim('-');
        }
        if c == '<' {
            if self.rest().starts_with("<!--") {
                self.pos += 4;
                return TokenKind::Cdo;
            }
            self.bump();
            return TokenKind::Delim('<');
        }
        if c == '@' {
            if would_start_ident(self.lookahead_at(1)) {
                self.bump();
                let name = self.consume_name();
                return TokenKind::AtKeyword(name);
            }
            self.bump();
            return TokenKind::Delim('@');
        }
        if c == '\\' {
            if escape_ok(la[0], la[1]) {
                return self.consume_ident_like();
            }
            self.bump();
            return TokenKind::Delim('\\');
        }
        if c.is_ascii_digit() {
            return self.consume_numeric();
        }
        if is_name_start(c) {
            return self.consume_ident_like();
        }
        self.bump();
        TokenKind::Delim(c)
    }

    /// Consume a string token. The opening quote has been consumed.
    fn consume_string(&mut self, quote: char) -> TokenKind {
        let mut out = String::new();
        loop {
            match self.peek(0) {
                None => return TokenKind::String(out),
                Some(c) if c == quote => {
                    self.bump();
                    return TokenKind::String(out);
                }
                Some(c) if is_newline(c) => return TokenKind::BadString,
                Some('\\') => match self.peek(1) {
                    // Backslash at EOF: nothing is added.
                    None => {
                        self.bump();
                    }
                    // Escaped newline is a line continuation.
                    Some(n) if is_newline(n) => {
                        self.bump();
                        self.consume_newline();
                    }
                    Some(_) => {
                        self.bump();
                        out.push(self.consume_escape());
                    }
                },
                Some('\0') => {
                    self.bump();
                    out.push('\u{FFFD}');
                }
                Some(c) => {
                    self.bump();
                    out.push(c);
                }
            }
        }
    }

    /// Consume an escaped code point. The backslash has been consumed.
    fn consume_escape(&mut self) -> char {
        let c = match self.peek(0) {
            None => return '\u{FFFD}',
            Some(c) => c,
        };
        if c.is_ascii_hexdigit() {
            let mut value: u32 = 0;
            let mut count = 0;
            while count < 6 {
                match self.peek(0) {
                    Some(h) if h.is_ascii_hexdigit() => {
                        value = value * 16 + h.to_digit(16).unwrap_or(0);
                        self.bump();
                        count += 1;
                    }
                    _ => break,
                }
            }
            // One whitespace after a hex escape is part of the escape.
            if matches!(self.peek(0), Some(w) if is_ws(w)) {
                self.consume_newline_or_space();
            }
            if value == 0 || (0xD800..=0xDFFF).contains(&value) || value > 0x10FFFF {
                return '\u{FFFD}';
            }
            return char::from_u32(value).unwrap_or('\u{FFFD}');
        }
        self.bump();
        c
    }

    fn consume_newline_or_space(&mut self) {
        if self.peek(0) == Some('\r') && self.peek(1) == Some('\n') {
            self.bump();
            self.bump();
        } else {
            self.bump();
        }
    }

    /// Consume a name (identifier body).
    fn consume_name(&mut self) -> String {
        let mut out = String::new();
        loop {
            match self.peek(0) {
                Some(c) if is_name(c) => {
                    self.bump();
                    out.push(c);
                }
                Some(c) if escape_ok(Some(c), self.peek(1)) => {
                    self.bump();
                    out.push(self.consume_escape());
                }
                Some('\0') => {
                    self.bump();
                    out.push('\u{FFFD}');
                }
                _ => break,
            }
        }
        out
    }

    /// Consume an ident-like token: ident, function, or url.
    fn consume_ident_like(&mut self) -> TokenKind {
        let name = self.consume_name();
        if name.eq_ignore_ascii_case("url") && self.peek(0) == Some('(') {
            self.bump();
            // Skip whitespace; if a quote follows, this is a function token.
            let mut k = 0;
            while matches!(self.peek(k), Some(w) if is_ws(w)) {
                k += 1;
            }
            if matches!(self.peek(k), Some('"') | Some('\'')) {
                return TokenKind::Function(name);
            }
            for _ in 0..k {
                self.bump();
            }
            return self.consume_url();
        }
        if self.peek(0) == Some('(') {
            self.bump();
            return TokenKind::Function(name);
        }
        TokenKind::Ident(name)
    }

    /// Consume a url token body. The `url(` prefix and leading whitespace
    /// have been consumed.
    fn consume_url(&mut self) -> TokenKind {
        let mut out = String::new();
        loop {
            match self.peek(0) {
                None => return TokenKind::Url(out),
                Some(')') => {
                    self.bump();
                    return TokenKind::Url(out);
                }
                Some(c) if is_ws(c) => {
                    while matches!(self.peek(0), Some(w) if is_ws(w)) {
                        self.bump();
                    }
                    match self.peek(0) {
                        None => return TokenKind::Url(out),
                        Some(')') => {
                            self.bump();
                            return TokenKind::Url(out);
                        }
                        Some(_) => {
                            self.consume_url_remnants();
                            return TokenKind::BadUrl;
                        }
                    }
                }
                Some('"') | Some('\'') | Some('(') => {
                    self.consume_url_remnants();
                    return TokenKind::BadUrl;
                }
                Some(c) if is_non_printable(c) => {
                    self.consume_url_remnants();
                    return TokenKind::BadUrl;
                }
                Some('\\') => {
                    if escape_ok(Some('\\'), self.peek(1)) {
                        self.bump();
                        out.push(self.consume_escape());
                    } else {
                        self.consume_url_remnants();
                        return TokenKind::BadUrl;
                    }
                }
                Some('\0') => {
                    self.bump();
                    out.push('\u{FFFD}');
                }
                Some(c) => {
                    self.bump();
                    out.push(c);
                }
            }
        }
    }

    /// Consume the remnants of a bad url up to `)` or EOF.
    fn consume_url_remnants(&mut self) {
        loop {
            match self.peek(0) {
                None => return,
                Some(')') => {
                    self.bump();
                    return;
                }
                Some('\\') if escape_ok(Some('\\'), self.peek(1)) => {
                    self.bump();
                    let _ = self.consume_escape();
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    /// Consume a number and return its value and type flag.
    fn consume_number(&mut self) -> (f64, NumericType) {
        let mut repr = String::new();
        let mut ty = NumericType::Integer;

        if let Some(s @ ('+' | '-')) = self.peek(0) {
            self.bump();
            if s == '-' {
                repr.push('-');
            }
        }
        let mut int_part = String::new();
        while let Some(d) = self.peek(0).filter(|c| c.is_ascii_digit()) {
            self.bump();
            int_part.push(d);
        }
        let mut frac_part = String::new();
        if self.peek(0) == Some('.') && matches!(self.peek(1), Some(d) if d.is_ascii_digit()) {
            self.bump();
            ty = NumericType::Number;
            while let Some(d) = self.peek(0).filter(|c| c.is_ascii_digit()) {
                self.bump();
                frac_part.push(d);
            }
        }
        let mut exp_sign = '+';
        let mut exp_part = String::new();
        let exp_follows = match self.peek(0) {
            Some('e') | Some('E') => match self.peek(1) {
                Some(d) if d.is_ascii_digit() => true,
                Some('+') | Some('-') => matches!(self.peek(2), Some(d) if d.is_ascii_digit()),
                _ => false,
            },
            _ => false,
        };
        if exp_follows {
            self.bump();
            ty = NumericType::Number;
            if let Some(s @ ('+' | '-')) = self.peek(0) {
                self.bump();
                exp_sign = s;
            }
            while let Some(d) = self.peek(0).filter(|c| c.is_ascii_digit()) {
                self.bump();
                exp_part.push(d);
            }
        }

        if int_part.is_empty() {
            int_part.push('0');
        }
        if frac_part.is_empty() {
            frac_part.push('0');
        }
        if exp_part.is_empty() {
            exp_part.push('0');
        }
        repr.push_str(&int_part);
        repr.push('.');
        repr.push_str(&frac_part);
        repr.push('e');
        repr.push(exp_sign);
        repr.push_str(&exp_part);

        let value = repr.parse::<f64>().unwrap_or(0.0);
        let value = if value.is_finite() {
            value
        } else if value.is_sign_negative() {
            f64::MIN
        } else {
            f64::MAX
        };
        (value, ty)
    }

    /// Consume a numeric, percentage, or dimension token.
    fn consume_numeric(&mut self) -> TokenKind {
        let (value, ty) = self.consume_number();
        if would_start_ident(self.lookahead()) {
            let unit = self.consume_name();
            return TokenKind::Dimension { value, ty, unit };
        }
        if self.peek(0) == Some('%') {
            self.bump();
            return TokenKind::Percentage(value);
        }
        TokenKind::Number { value, ty }
    }
}
