// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Selector parsing (spec 12). Covers Tier 0 and Tier 1 selectors, the
//! sibling combinators, and the pseudo-elements `::before`, `::after`,
//! `::marker`, and `::placeholder`.
//!
//! Anything valid CSS that is not implemented yet returns an
//! `UnsupportedSelector` diagnostic. The rule is dropped, as spec 12 requires
//! for invalid selectors, and the diagnostic says why.

use super::diagnostics::{Diagnostic, DiagnosticCode};
use super::split_on;
use crate::ast::selector::{
    AttributeMatcher, AttributeOperator, AttributeSelector, Combinator, ComplexSelector,
    CompoundSelector, NthSelector, PseudoClass, RelativeSelector, SelectorList, SimpleSelector,
};
use crate::limits::ParseLimits;
use crate::selectors::pseudo::PseudoElement;
use crate::selectors::specificity::Specificity;
use crate::syntax::component::BlockKind;
use crate::syntax::{trim_whitespace, ComponentValue, Function, Span, TokenKind};
use crate::tree::Direction;
use crate::values::serialize::format_number;

const SELECTORS_URL: &str = "https://www.w3.org/TR/selectors-4/";

/// Parses a selector list from already-trimmed component values.
pub(crate) fn parse_selector_list(
    values: &[ComponentValue],
    limits: &ParseLimits,
) -> Result<SelectorList, Diagnostic> {
    parse_list_at(values, limits, 0)
}

fn parse_list_at(
    values: &[ComponentValue],
    limits: &ParseLimits,
    depth: usize,
) -> Result<SelectorList, Diagnostic> {
    if depth > limits.max_nesting_depth {
        return Err(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            span_of(values),
            "selector nesting limit exceeded",
            Some(SELECTORS_URL),
        ));
    }
    let mut selectors = Vec::new();
    for chunk in split_on(values, |v| v.is_token(&TokenKind::Comma)) {
        selectors.push(parse_complex(trim_whitespace(chunk), limits, depth)?);
    }
    if selectors.is_empty() {
        return Err(invalid(span_of(values), "empty selector"));
    }
    Ok(SelectorList { selectors })
}

/// The token kind at `idx`, if the value there is a token.
fn kind_at(values: &[ComponentValue], idx: usize) -> Option<&TokenKind> {
    match values.get(idx) {
        Some(ComponentValue::Token(t)) => Some(&t.kind),
        _ => None,
    }
}

fn set_combinator(
    pending: &mut Option<Combinator>,
    combinator: Combinator,
    span: Span,
) -> Result<(), Diagnostic> {
    match pending {
        Some(existing) if *existing != Combinator::Descendant => {
            Err(invalid(span, "two combinators in a row"))
        }
        _ => {
            *pending = Some(combinator);
            Ok(())
        }
    }
}

fn parse_complex(
    values: &[ComponentValue],
    limits: &ParseLimits,
    depth: usize,
) -> Result<ComplexSelector, Diagnostic> {
    if values.is_empty() {
        return Err(invalid(Span::default(), "empty selector"));
    }
    let span = span_of(values);
    let mut compounds: Vec<CompoundSelector> = Vec::new();
    let mut combinators: Vec<Combinator> = Vec::new();
    let mut pending: Option<Combinator> = None;
    let mut pseudo_element: Option<PseudoElement> = None;
    let mut i = 0;

    while i < values.len() {
        let v = &values[i];
        if v.is_whitespace() {
            if pending.is_none() {
                pending = Some(Combinator::Descendant);
            }
            i += 1;
            continue;
        }
        let delim_combinator = match kind_at(values, i) {
            Some(TokenKind::Delim('>')) => Some(Combinator::Child),
            Some(TokenKind::Delim('+')) => Some(Combinator::NextSibling),
            Some(TokenKind::Delim('~')) => Some(Combinator::SubsequentSibling),
            _ => None,
        };
        if let Some(c) = delim_combinator {
            if compounds.is_empty() {
                return Err(invalid(v.span(), "selector starts with a combinator"));
            }
            set_combinator(&mut pending, c, v.span())?;
            i += 1;
            continue;
        }

        if !compounds.is_empty() {
            combinators.push(pending.take().unwrap_or(Combinator::Descendant));
        }
        let (compound, next, pe) = parse_compound(values, i, limits, depth)?;
        compounds.push(compound);
        if compounds.len() > limits.max_selector_complexity {
            return Err(Diagnostic::error(
                DiagnosticCode::LimitExceeded,
                span,
                "selector complexity limit exceeded",
                Some(SELECTORS_URL),
            ));
        }
        i = next;
        if let Some(p) = pe {
            if values[i..].iter().any(|v| !v.is_whitespace()) {
                return Err(invalid(span, "a pseudo-element must end the selector"));
            }
            pseudo_element = Some(p);
            break;
        }
    }

    if compounds.is_empty() {
        return Err(invalid(span, "selector has no compound selector"));
    }
    if pending.is_some() {
        return Err(invalid(span, "selector ends with a combinator"));
    }

    if matches!(pseudo_element, Some(PseudoElement::Slotted)) && compounds.len() > 1 {
        return Err(unsupported(span, "::slotted() must be the whole selector"));
    }

    let mut selector = ComplexSelector {
        compounds,
        combinators,
        pseudo_element,
        specificity: Specificity::default(),
        span,
    };
    selector.specificity = Specificity::of_complex(&selector);
    Ok(selector)
}

/// Parses one compound selector starting at `start`. Returns the compound, the
/// index after it, and a pseudo-element if the compound ends with one.
fn parse_compound(
    values: &[ComponentValue],
    start: usize,
    limits: &ParseLimits,
    depth: usize,
) -> Result<(CompoundSelector, usize, Option<PseudoElement>), Diagnostic> {
    let mut simple = Vec::new();
    let mut i = start;
    let mut pseudo: Option<PseudoElement> = None;

    while i < values.len() {
        let v = &values[i];
        if v.is_whitespace() {
            break;
        }
        if matches!(
            kind_at(values, i),
            Some(TokenKind::Delim('>') | TokenKind::Delim('+') | TokenKind::Delim('~'))
        ) {
            break;
        }
        match v {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::Delim('*') => {
                    if i != start {
                        return Err(invalid(t.span, "universal selector must come first"));
                    }
                    simple.push(SimpleSelector::Universal);
                    i += 1;
                }
                TokenKind::Delim('&') => {
                    // Resolved against the parent rule by `resolve_nesting`.
                    simple.push(SimpleSelector::Nesting);
                    i += 1;
                }
                TokenKind::Ident(name) => {
                    if i != start {
                        return Err(invalid(t.span, "type selector must come first"));
                    }
                    if kind_at(values, i + 1) == Some(&TokenKind::Delim('|')) {
                        return Err(unsupported(
                            t.span,
                            "namespace prefixes are not supported yet",
                        ));
                    }
                    simple.push(SimpleSelector::Type(name.clone()));
                    i += 1;
                }
                TokenKind::Hash { value, is_id } => {
                    if !is_id {
                        return Err(invalid(t.span, "hash selector is not a valid identifier"));
                    }
                    simple.push(SimpleSelector::Id(value.clone()));
                    i += 1;
                }
                TokenKind::Delim('.') => match kind_at(values, i + 1) {
                    Some(TokenKind::Ident(name)) => {
                        simple.push(SimpleSelector::Class(name.clone()));
                        i += 2;
                    }
                    _ => return Err(invalid(t.span, "expected class name after '.'")),
                },
                TokenKind::Colon => {
                    if kind_at(values, i + 1) == Some(&TokenKind::Colon) {
                        if let Some(ComponentValue::Function(f)) = values.get(i + 2) {
                            let (argument, element) = functional_pseudo_element(f, limits, depth)?;
                            if !argument.is_empty() && !simple.is_empty() {
                                return Err(invalid(
                                    t.span,
                                    "::slotted() must be the whole selector",
                                ));
                            }
                            simple.extend(argument);
                            pseudo = Some(element);
                            i += 3;
                            break;
                        }
                        let name = match kind_at(values, i + 2) {
                            Some(TokenKind::Ident(name)) => name.clone(),
                            _ => return Err(invalid(t.span, "expected pseudo-element name")),
                        };
                        pseudo = Some(pseudo_element(&name, t.span)?);
                        i += 3;
                        break;
                    }
                    match values.get(i + 1) {
                        Some(ComponentValue::Token(next)) => match &next.kind {
                            TokenKind::Ident(name) => {
                                if matches!(name.to_ascii_lowercase().as_str(), "before" | "after")
                                {
                                    // Legacy single-colon forms of the pseudo-elements.
                                    pseudo = Some(pseudo_element(name, next.span)?);
                                    i += 2;
                                    break;
                                }
                                simple.extend(pseudo_class(name, next.span)?);
                                i += 2;
                            }
                            _ => return Err(invalid(t.span, "expected pseudo-class name")),
                        },
                        Some(ComponentValue::Function(f)) => {
                            simple.push(functional_pseudo(f, limits, depth)?);
                            i += 2;
                        }
                        _ => return Err(invalid(t.span, "expected pseudo-class name")),
                    }
                }
                _ => return Err(invalid(t.span, "unexpected token in selector")),
            },
            ComponentValue::Block(b) if b.kind == BlockKind::Square => {
                simple.push(SimpleSelector::Attribute(parse_attribute(
                    &b.value, b.span,
                )?));
                i += 1;
            }
            ComponentValue::Function(f) => {
                return Err(unsupported(
                    f.span,
                    format!("selector function {}() is not supported here", f.name),
                ))
            }
            other => return Err(invalid(other.span(), "unexpected block in selector")),
        }
    }

    if pseudo.is_some() && simple.is_empty() {
        // `::before` on its own means `*::before`.
        simple.push(SimpleSelector::Universal);
    }
    if simple.is_empty() {
        return Err(invalid(
            span_of(&values[start..i.max(start)]),
            "empty compound selector",
        ));
    }
    Ok((CompoundSelector { simple }, i, pseudo))
}

/// Replaces every `&` in a nested selector list with the parent selector list
/// (CSS Nesting). A complex selector with no `&` gets an implicit `& ` descendant
/// prefix. The result matches on its own, so the matcher needs no nesting case.
pub(crate) fn resolve_nesting(child: &SelectorList, parent: &SelectorList) -> SelectorList {
    let selectors = child
        .selectors
        .iter()
        .map(|complex| {
            let mut had_nesting = false;
            let mut compounds: Vec<CompoundSelector> = complex
                .compounds
                .iter()
                .map(|compound| CompoundSelector {
                    simple: compound
                        .simple
                        .iter()
                        .map(|simple| replace_nesting(simple, parent, &mut had_nesting))
                        .collect(),
                })
                .collect();
            let mut combinators = complex.combinators.clone();
            if !had_nesting {
                compounds.insert(
                    0,
                    CompoundSelector {
                        simple: vec![SimpleSelector::Is(parent.clone())],
                    },
                );
                combinators.insert(0, Combinator::Descendant);
            }
            let mut out = ComplexSelector {
                compounds,
                combinators,
                pseudo_element: complex.pseudo_element.clone(),
                specificity: Specificity::default(),
                span: complex.span,
            };
            out.specificity = Specificity::of_complex(&out);
            out
        })
        .collect();
    SelectorList { selectors }
}

fn replace_nesting(
    simple: &SimpleSelector,
    parent: &SelectorList,
    had_nesting: &mut bool,
) -> SimpleSelector {
    match simple {
        SimpleSelector::Nesting => {
            *had_nesting = true;
            SimpleSelector::Is(parent.clone())
        }
        SimpleSelector::Is(list) => SimpleSelector::Is(resolve_inner(list, parent, had_nesting)),
        SimpleSelector::Not(list) => SimpleSelector::Not(resolve_inner(list, parent, had_nesting)),
        SimpleSelector::Where(list) => {
            SimpleSelector::Where(resolve_inner(list, parent, had_nesting))
        }
        other => other.clone(),
    }
}

/// Resolves `&` inside a functional argument. Such a list never gets the
/// implicit prefix, so it only replaces `&` in place.
fn resolve_inner(
    list: &SelectorList,
    parent: &SelectorList,
    had_nesting: &mut bool,
) -> SelectorList {
    let selectors = list
        .selectors
        .iter()
        .map(|complex| {
            let mut out = complex.clone();
            for compound in &mut out.compounds {
                for simple in &mut compound.simple {
                    *simple = replace_nesting(simple, parent, had_nesting);
                }
            }
            out.specificity = Specificity::of_complex(&out);
            out
        })
        .collect();
    SelectorList { selectors }
}

/// True if a selector list contains `&` anywhere. Used to reject `&` in a
/// top-level rule, where it has no meaning in this engine.
pub(crate) fn contains_nesting(list: &SelectorList) -> bool {
    list.selectors.iter().any(|c| {
        c.compounds.iter().any(|comp| {
            comp.simple.iter().any(|s| match s {
                SimpleSelector::Nesting => true,
                SimpleSelector::Is(l) | SimpleSelector::Not(l) | SimpleSelector::Where(l) => {
                    contains_nesting(l)
                }
                _ => false,
            })
        })
    })
}

fn pseudo_element(name: &str, span: Span) -> Result<PseudoElement, Diagnostic> {
    match name.to_ascii_lowercase().as_str() {
        "before" => Ok(PseudoElement::Before),
        "after" => Ok(PseudoElement::After),
        "marker" => Ok(PseudoElement::Marker),
        "placeholder" => Ok(PseudoElement::Placeholder),
        "selection" => Ok(PseudoElement::Selection),
        "backdrop" => Ok(PseudoElement::Backdrop),
        "file-selector-button" => Ok(PseudoElement::FileSelectorButton),
        _ => Err(unsupported(
            span,
            format!("pseudo-element ::{name} is not supported yet"),
        )),
    }
}

fn nth_plain(last: bool, of_type: bool) -> SimpleSelector {
    SimpleSelector::Nth(NthSelector {
        a: 0,
        b: 1,
        of_type,
        last,
        of: None,
    })
}

/// Simple pseudo-classes without arguments. Some expand to several simple
/// selectors (`:only-child` is `:first-child:last-child`).
fn pseudo_class(name: &str, span: Span) -> Result<Vec<SimpleSelector>, Diagnostic> {
    let one = |p: PseudoClass| Ok(vec![SimpleSelector::PseudoClass(p)]);
    match name.to_ascii_lowercase().as_str() {
        "root" => one(PseudoClass::Root),
        "hover" => one(PseudoClass::Hover),
        "active" => one(PseudoClass::Active),
        "focus" => one(PseudoClass::Focus),
        "empty" => one(PseudoClass::Empty),
        "enabled" => one(PseudoClass::Enabled),
        "disabled" => one(PseudoClass::Disabled),
        "checked" => one(PseudoClass::Checked),
        "required" => one(PseudoClass::Required),
        "optional" => one(PseudoClass::Optional),
        "read-only" => one(PseudoClass::ReadOnly),
        "read-write" => one(PseudoClass::ReadWrite),
        "placeholder-shown" => one(PseudoClass::PlaceholderShown),
        "link" => one(PseudoClass::Link),
        "any-link" => one(PseudoClass::AnyLink),
        "visited" => one(PseudoClass::Visited),
        "focus-visible" => one(PseudoClass::FocusVisible),
        "focus-within" => one(PseudoClass::FocusWithin),
        "target" => one(PseudoClass::Target),
        "valid" => one(PseudoClass::Valid),
        "invalid" => one(PseudoClass::Invalid),
        "user-valid" => one(PseudoClass::UserValid),
        "user-invalid" => one(PseudoClass::UserInvalid),
        "in-range" => one(PseudoClass::InRange),
        "out-of-range" => one(PseudoClass::OutOfRange),
        "default" => one(PseudoClass::Default),
        "indeterminate" => one(PseudoClass::Indeterminate),
        "open" => one(PseudoClass::Open),
        "closed" => one(PseudoClass::Closed),
        "modal" => one(PseudoClass::Modal),
        "popover-open" => one(PseudoClass::PopoverOpen),
        "current" => one(PseudoClass::Current),
        "past" => one(PseudoClass::Past),
        "future" => one(PseudoClass::Future),
        "defined" => one(PseudoClass::Defined),
        "local-link" => one(PseudoClass::LocalLink),
        "playing" => one(PseudoClass::Playing),
        "paused" => one(PseudoClass::Paused),
        "fullscreen" => one(PseudoClass::Fullscreen),
        "autofill" => one(PseudoClass::Autofill),
        "scope" => Ok(vec![SimpleSelector::Scope]),
        "host" => Ok(vec![SimpleSelector::Host(None)]),
        "first-child" => Ok(vec![nth_plain(false, false)]),
        "last-child" => Ok(vec![nth_plain(true, false)]),
        "only-child" => Ok(vec![nth_plain(false, false), nth_plain(true, false)]),
        "first-of-type" => Ok(vec![nth_plain(false, true)]),
        "last-of-type" => Ok(vec![nth_plain(true, true)]),
        "only-of-type" => Ok(vec![nth_plain(false, true), nth_plain(true, true)]),
        _ => Err(unsupported(
            span,
            format!("pseudo-class :{name} is not supported yet"),
        )),
    }
}

/// Functional pseudo-classes: `:nth-*()`, `:not()`, `:is()`, `:where()`,
/// `:lang()`, and `:dir()`.
fn functional_pseudo(
    f: &Function,
    limits: &ParseLimits,
    depth: usize,
) -> Result<SimpleSelector, Diagnostic> {
    let name = f.name.to_ascii_lowercase();
    match name.as_str() {
        "nth-child" => parse_nth(f, limits, depth, false, true),
        "nth-last-child" => parse_nth(f, limits, depth, true, true),
        "nth-of-type" => parse_nth(f, limits, depth, false, false),
        "nth-last-of-type" => parse_nth(f, limits, depth, true, false),
        "not" | "is" | "where" => {
            let list = parse_list_at(trim_whitespace(&f.value), limits, depth + 1)?;
            if list.selectors.iter().any(|s| s.pseudo_element.is_some()) {
                return Err(invalid(
                    f.span,
                    "pseudo-elements are not allowed in this argument",
                ));
            }
            Ok(match name.as_str() {
                "not" => SimpleSelector::Not(list),
                "is" => SimpleSelector::Is(list),
                _ => SimpleSelector::Where(list),
            })
        }
        "host" => {
            let list = parse_list_at(trim_whitespace(&f.value), limits, depth + 1)?;
            Ok(SimpleSelector::Host(Some(list)))
        }
        "host-context" => {
            let list = parse_list_at(trim_whitespace(&f.value), limits, depth + 1)?;
            Ok(SimpleSelector::HostContext(list))
        }
        "state" => match trim_whitespace(&f.value) {
            [ComponentValue::Token(t)] => match &t.kind {
                TokenKind::Ident(name) => Ok(SimpleSelector::CustomState(name.clone())),
                _ => Err(invalid(f.span, ":state() takes a state name")),
            },
            _ => Err(invalid(f.span, ":state() takes one state name")),
        },
        "has" => parse_has(f, limits, depth),
        "lang" => {
            let args = trim_whitespace(&f.value);
            match args {
                [ComponentValue::Token(t)] => match &t.kind {
                    TokenKind::Ident(s) | TokenKind::String(s) => {
                        Ok(SimpleSelector::Lang(s.clone()))
                    }
                    _ => Err(invalid(t.span, ":lang() expects a language tag")),
                },
                _ => Err(invalid(f.span, ":lang() expects one language tag")),
            }
        }
        "dir" => {
            let args = trim_whitespace(&f.value);
            match args {
                [ComponentValue::Token(t)] => match &t.kind {
                    TokenKind::Ident(s) if s.eq_ignore_ascii_case("ltr") => {
                        Ok(SimpleSelector::Dir(Direction::Ltr))
                    }
                    TokenKind::Ident(s) if s.eq_ignore_ascii_case("rtl") => {
                        Ok(SimpleSelector::Dir(Direction::Rtl))
                    }
                    _ => Err(invalid(t.span, ":dir() expects ltr or rtl")),
                },
                _ => Err(invalid(f.span, ":dir() expects ltr or rtl")),
            }
        }
        _ => Err(unsupported(
            f.span,
            format!(":{}() is not supported yet", f.name),
        )),
    }
}

/// Parses the arguments of an `:nth-*()` pseudo-class. `:nth-child` and
/// `:nth-last-child` accept `of S`.
fn parse_nth(
    f: &Function,
    limits: &ParseLimits,
    depth: usize,
    last: bool,
    allow_of: bool,
) -> Result<SimpleSelector, Diagnostic> {
    let args = trim_whitespace(&f.value);
    let of_index = args.iter().position(|v| match v {
        ComponentValue::Token(t) => {
            matches!(&t.kind, TokenKind::Ident(s) if s.eq_ignore_ascii_case("of"))
        }
        _ => false,
    });
    let (formula, of) = match of_index {
        Some(k) if allow_of => {
            let right = trim_whitespace(&args[k + 1..]);
            if right.is_empty() {
                return Err(invalid(f.span, "expected a selector after 'of'"));
            }
            (&args[..k], Some(parse_list_at(right, limits, depth + 1)?))
        }
        _ => (args, None),
    };
    let text =
        components_text(formula).ok_or_else(|| invalid(f.span, "invalid An+B expression"))?;
    let (a, b) = parse_anb(&text).ok_or_else(|| invalid(f.span, "invalid An+B expression"))?;
    Ok(SimpleSelector::Nth(NthSelector {
        a,
        b,
        of_type: !allow_of,
        last,
        of,
    }))
}

/// Writes component values back as text for `An+B` parsing. A number that
/// follows a name without a space keeps its sign: `2n+1` tokenizes as a
/// dimension followed by an unsigned number, so a `+` is restored here.
fn components_text(values: &[ComponentValue]) -> Option<String> {
    let mut out = String::new();
    for v in values {
        match v {
            ComponentValue::Token(t) => match &t.kind {
                TokenKind::Whitespace => out.push(' '),
                TokenKind::Ident(s) => out.push_str(s),
                TokenKind::Delim(c) => out.push(*c),
                TokenKind::Number { value, .. } => {
                    push_sign_if_needed(&mut out, *value);
                    out.push_str(&format_number(*value));
                }
                TokenKind::Dimension { value, unit, .. } => {
                    push_sign_if_needed(&mut out, *value);
                    out.push_str(&format_number(*value));
                    out.push_str(unit);
                }
                _ => return None,
            },
            _ => return None,
        }
    }
    Some(out)
}

fn push_sign_if_needed(out: &mut String, value: f64) {
    let after_name = out
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_alphanumeric());
    if value >= 0.0 && after_name {
        out.push('+');
    }
}

/// Parses `An+B`, `odd`, `even`, or an integer (spec 12.2).
fn parse_anb(text: &str) -> Option<(i64, i64)> {
    let s: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    match s.as_str() {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    match s.find('n') {
        Some(idx) => {
            let a_str = &s[..idx];
            let b_str = &s[idx + 1..];
            let a = match a_str {
                "" | "+" => 1,
                "-" => -1,
                other => other.parse::<i64>().ok()?,
            };
            let b = if b_str.is_empty() {
                0
            } else if b_str.starts_with('+') || b_str.starts_with('-') {
                b_str.parse::<i64>().ok()?
            } else {
                return None;
            };
            Some((a, b))
        }
        None => s.parse::<i64>().ok().map(|b| (0, b)),
    }
}

/// Parses the inside of `[ ... ]`: `name`, or `name OP value` where OP is one
/// of `=`, `~=`, `|=`, `^=`, `$=`, `*=`.
fn parse_attribute(values: &[ComponentValue], span: Span) -> Result<AttributeSelector, Diagnostic> {
    let v = trim_whitespace(values);
    let name = match v.first() {
        Some(ComponentValue::Token(t)) => match &t.kind {
            TokenKind::Ident(n) => n.clone(),
            _ => return Err(invalid(span, "expected attribute name")),
        },
        _ => return Err(invalid(span, "expected attribute name")),
    };

    let rest = trim_whitespace(&v[1..]);
    if rest.is_empty() {
        return Ok(AttributeSelector {
            name,
            matcher: None,
        });
    }
    let (operator, value_start) = match kind_at(rest, 0) {
        Some(TokenKind::Delim('=')) => (AttributeOperator::Equals, 1),
        Some(TokenKind::Delim(c @ ('~' | '|' | '^' | '$' | '*')))
            if kind_at(rest, 1) == Some(&TokenKind::Delim('=')) =>
        {
            let op = match *c {
                '~' => AttributeOperator::Includes,
                '|' => AttributeOperator::DashMatch,
                '^' => AttributeOperator::Prefix,
                '$' => AttributeOperator::Suffix,
                _ => AttributeOperator::Substring,
            };
            (op, 2)
        }
        _ => return Err(invalid(span, "malformed attribute selector")),
    };
    let value = match trim_whitespace(&rest[value_start..]) {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(s) | TokenKind::String(s) => s.clone(),
            _ => {
                return Err(invalid(
                    t.span,
                    "attribute value must be an identifier or string",
                ))
            }
        },
        _ => {
            return Err(unsupported(
                span,
                "attribute flags (such as 'i') are not supported yet",
            ))
        }
    };
    Ok(AttributeSelector {
        name,
        matcher: Some(AttributeMatcher { operator, value }),
    })
}

fn invalid(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidSelector,
        span,
        message,
        Some(SELECTORS_URL),
    )
}

fn unsupported(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::UnsupportedSelector,
        span,
        message,
        Some(SELECTORS_URL),
    )
}

/// Span covering all values, or an empty span for an empty slice.
pub(crate) fn span_of(values: &[ComponentValue]) -> Span {
    match (values.first(), values.last()) {
        (Some(a), Some(b)) => a.span().join(b.span()),
        _ => Span::default(),
    }
}

/// `:has(relative-selector-list)`. Each entry may start with a combinator, which
/// relates its subject to the anchor (a descendant when none is written).
fn parse_has(
    f: &Function,
    limits: &ParseLimits,
    depth: usize,
) -> Result<SimpleSelector, Diagnostic> {
    if depth + 1 > limits.max_nesting_depth {
        return Err(invalid(f.span, ":has() nesting limit exceeded"));
    }
    let mut relatives = Vec::new();
    for part in split_on(&f.value, |v| v.is_token(&TokenKind::Comma)) {
        let mut part = trim_whitespace(part);
        let leading = match part.first() {
            Some(ComponentValue::Token(t)) => match t.kind {
                TokenKind::Delim('>') => Some(Combinator::Child),
                TokenKind::Delim('+') => Some(Combinator::NextSibling),
                TokenKind::Delim('~') => Some(Combinator::SubsequentSibling),
                _ => None,
            },
            _ => None,
        };
        let leading = match leading {
            Some(c) => {
                part = trim_whitespace(&part[1..]);
                c
            }
            None => Combinator::Descendant,
        };
        if part.is_empty() {
            return Err(invalid(f.span, ":has() needs a selector"));
        }
        let selector = parse_complex(part, limits, depth + 1)?;
        if selector.pseudo_element.is_some() {
            return Err(invalid(f.span, "pseudo-elements are not allowed in :has()"));
        }
        relatives.push(RelativeSelector { leading, selector });
    }
    if relatives.is_empty() {
        return Err(invalid(f.span, ":has() needs a selector"));
    }
    Ok(SimpleSelector::Has(relatives))
}

/// Functional pseudo-elements: `::part(name)` and `::slotted(compound)`
/// (Shadow DOM 1 sections 4 and 5). Returns the simple selectors that the
/// pseudo-element adds to the compound, and the pseudo-element.
fn functional_pseudo_element(
    f: &Function,
    limits: &ParseLimits,
    depth: usize,
) -> Result<(Vec<SimpleSelector>, PseudoElement), Diagnostic> {
    match f.name.to_ascii_lowercase().as_str() {
        "part" => match trim_whitespace(&f.value) {
            [ComponentValue::Token(t)] => match &t.kind {
                TokenKind::Ident(name) => Ok((Vec::new(), PseudoElement::Part(name.clone()))),
                _ => Err(invalid(f.span, "::part() takes a part name")),
            },
            _ => Err(invalid(f.span, "::part() takes one part name")),
        },
        "slotted" => {
            let args = trim_whitespace(&f.value);
            let (compound, end, pseudo) = parse_compound(args, 0, limits, depth)?;
            if end != args.len() || pseudo.is_some() {
                return Err(invalid(f.span, "::slotted() takes one compound selector"));
            }
            Ok((compound.simple, PseudoElement::Slotted))
        }
        "highlight" => match trim_whitespace(&f.value) {
            [ComponentValue::Token(t)] => match &t.kind {
                TokenKind::Ident(name) => Ok((Vec::new(), PseudoElement::Highlight(name.clone()))),
                _ => Err(invalid(f.span, "::highlight() takes a highlight name")),
            },
            _ => Err(invalid(f.span, "::highlight() takes one highlight name")),
        },
        other => Err(unsupported(
            f.span,
            format!("pseudo-element ::{other}() is not supported yet"),
        )),
    }
}
