// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Syntax layer tests: tokenizer (CSS Syntax 3 section 4) and component
//! values (section 5.3).

use tpt_css::syntax::component::{build_component_values, ComponentValue};
use tpt_css::syntax::{tokenize, NumericType, SourceId, TokenKind};
use tpt_css::ParseLimits;

fn kinds(input: &str) -> Vec<TokenKind> {
    let t = tokenize(SourceId(0), input, &ParseLimits::default());
    assert!(!t.limit_exceeded);
    assert_eq!(t.tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
    t.tokens.into_iter().map(|t| t.kind).collect()
}

/// Kinds without the trailing Eof and without whitespace.
fn significant(input: &str) -> Vec<TokenKind> {
    let mut k = kinds(input);
    k.pop();
    k.into_iter().filter(|t| !t.is_whitespace()).collect()
}

#[test]
fn identifiers_functions_and_at_keywords() {
    assert_eq!(
        significant("color rgb( @media"),
        vec![
            TokenKind::Ident("color".into()),
            TokenKind::Function("rgb".into()),
            TokenKind::AtKeyword("media".into()),
        ]
    );
}

#[test]
fn hash_tokens_record_whether_they_are_identifiers() {
    assert_eq!(
        significant("#abc #1x"),
        vec![
            TokenKind::Hash {
                value: "abc".into(),
                is_id: true
            },
            TokenKind::Hash {
                value: "1x".into(),
                is_id: false
            },
        ]
    );
}

#[test]
fn numbers_percentages_and_dimensions() {
    assert_eq!(
        significant("12 1.5 1e3 -2px 50% +.5em"),
        vec![
            TokenKind::Number {
                value: 12.0,
                ty: NumericType::Integer
            },
            TokenKind::Number {
                value: 1.5,
                ty: NumericType::Number
            },
            TokenKind::Number {
                value: 1000.0,
                ty: NumericType::Number
            },
            TokenKind::Dimension {
                value: -2.0,
                ty: NumericType::Integer,
                unit: "px".into()
            },
            TokenKind::Percentage(50.0),
            TokenKind::Dimension {
                value: 0.5,
                ty: NumericType::Number,
                unit: "em".into()
            },
        ]
    );
}

#[test]
fn a_dot_without_digits_is_a_delim() {
    assert_eq!(
        significant(".a"),
        vec![TokenKind::Delim('.'), TokenKind::Ident("a".into())]
    );
}

#[test]
fn strings_with_escapes_and_line_continuations() {
    assert_eq!(
        significant(r#""a\"b" 'c\41 d'"#),
        vec![
            TokenKind::String("a\"b".into()),
            // The space after the hex escape \41 is part of the escape.
            TokenKind::String("cAd".into()),
        ]
    );
}

#[test]
fn newline_in_string_is_a_bad_string() {
    let k = significant("\"abc\ndef\"");
    assert_eq!(k[0], TokenKind::BadString);
}

#[test]
fn hex_escapes_and_replacement_characters() {
    // \41 is 'A', a zero escape becomes U+FFFD, and an escaped letter is itself.
    assert_eq!(
        significant(r"\41 \0 \z"),
        vec![TokenKind::Ident("A\u{FFFD}z".into())]
    );
}

#[test]
fn unquoted_url_and_bad_url() {
    assert_eq!(
        significant("url(a.png)"),
        vec![TokenKind::Url("a.png".into())]
    );
    assert_eq!(significant("url(a b)"), vec![TokenKind::BadUrl]);
}

#[test]
fn quoted_url_is_a_function_token() {
    assert_eq!(
        significant("url(\"a.png\")"),
        vec![
            TokenKind::Function("url".into()),
            TokenKind::String("a.png".into()),
            TokenKind::RightParen,
        ]
    );
}

#[test]
fn cdo_cdc_and_punctuation() {
    assert_eq!(
        significant("<!-- --> : ; , [ ] { } ( )"),
        vec![
            TokenKind::Cdo,
            TokenKind::Cdc,
            TokenKind::Colon,
            TokenKind::Semicolon,
            TokenKind::Comma,
            TokenKind::LeftSquare,
            TokenKind::RightSquare,
            TokenKind::LeftCurly,
            TokenKind::RightCurly,
            TokenKind::LeftParen,
            TokenKind::RightParen,
        ]
    );
}

#[test]
fn comments_produce_no_tokens() {
    assert_eq!(
        significant("a/* hidden */b"),
        vec![TokenKind::Ident("a".into()), TokenKind::Ident("b".into())]
    );
    // Unclosed comments run to end of input without panicking.
    assert_eq!(significant("a/* open"), vec![TokenKind::Ident("a".into())]);
}

#[test]
fn crlf_is_one_whitespace_token_and_spans_are_source_offsets() {
    let input = "a\r\nb";
    let t = tokenize(SourceId(7), input, &ParseLimits::default());
    let ws = &t.tokens[1];
    assert_eq!(ws.kind, TokenKind::Whitespace);
    assert_eq!((ws.span.start, ws.span.end), (1, 3));
    assert_eq!(ws.span.source, SourceId(7));
}

#[test]
fn non_ascii_identifiers_and_multibyte_spans() {
    let input = "é-x";
    let t = tokenize(SourceId(0), input, &ParseLimits::default());
    assert_eq!(t.tokens[0].kind, TokenKind::Ident("é-x".into()));
    assert_eq!(t.tokens[0].span.end as usize, input.len());
}

#[test]
fn token_limit_stops_tokenizing_and_reports_it() {
    let limits = ParseLimits {
        max_tokens: 4,
        ..ParseLimits::default()
    };
    let t = tokenize(SourceId(0), "a b c d e f g", &limits);
    assert!(t.limit_exceeded);
    assert_eq!(t.tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
    assert!(t.tokens.len() <= 4);
}

#[test]
fn input_size_limit_produces_only_eof() {
    let limits = ParseLimits {
        max_input_bytes: 3,
        ..ParseLimits::default()
    };
    let t = tokenize(SourceId(0), "aaaaaaa", &limits);
    assert!(t.limit_exceeded);
    assert_eq!(t.tokens.len(), 1);
}

#[test]
fn component_values_nest_functions_and_blocks() {
    let t = tokenize(SourceId(0), "f(a [b] {c})", &ParseLimits::default());
    let cvs = build_component_values(&t.tokens, 16).expect("within depth");
    assert_eq!(cvs.len(), 1);
    let ComponentValue::Function(f) = &cvs[0] else {
        panic!("expected a function, got {:?}", cvs[0]);
    };
    assert_eq!(f.name, "f");
    // a, whitespace, [b], whitespace, {c}
    assert_eq!(f.value.len(), 5);
    assert!(matches!(f.value[2], ComponentValue::Block(_)));
    assert!(matches!(f.value[4], ComponentValue::Block(_)));
}

#[test]
fn mismatched_closers_are_preserved_as_tokens() {
    // ']' inside '(' is not a closer for the paren block.
    let t = tokenize(SourceId(0), "(a])", &ParseLimits::default());
    let cvs = build_component_values(&t.tokens, 16).expect("within depth");
    let ComponentValue::Block(b) = &cvs[0] else {
        panic!("expected a block");
    };
    assert!(b.value.iter().any(|v| v.is_token(&TokenKind::RightSquare)));
}

#[test]
fn unclosed_blocks_are_closed_at_eof() {
    let t = tokenize(SourceId(0), "{a (b", &ParseLimits::default());
    let cvs = build_component_values(&t.tokens, 16).expect("within depth");
    assert_eq!(cvs.len(), 1);
    assert!(matches!(cvs[0], ComponentValue::Block(_)));
}

#[test]
fn nesting_beyond_the_limit_is_an_error_not_a_crash() {
    let deep = "(".repeat(5000);
    let t = tokenize(SourceId(0), &deep, &ParseLimits::default());
    assert!(build_component_values(&t.tokens, 64).is_err());
}
