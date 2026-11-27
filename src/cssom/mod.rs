// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! CSSOM mutation and deterministic serialization (spec 18), behind the `cssom`
//! feature.
//!
//! [`CssomSheet`] holds rules the way a script would edit them: insert, delete,
//! change a selector, change a declaration. Each mutation is checked, so an
//! invalid index or text is an error and leaves the sheet unchanged. Serialization
//! is canonical: the same rules give the same text.
//!
//! A mutated sheet changes the cascade. Convert it with [`CssomSheet::to_stylesheet`],
//! then replace the sheet in the resolver (`remove_stylesheet`, `add_stylesheet`)
//! and report [`crate::StyleChange::StyleSheetChanged`]. The style engine does not
//! keep a live link to the CSSOM object.
//!
//! Limits: serialization is canonical, not a byte-for-byte copy of the author's
//! text (spec 25.8). Rules whose features this engine does not model (for
//! example `@font-face` descriptors or `@keyframes` steps) are serialized from
//! their parsed form, and some formatting is lost.

use crate::ast::media::{ContrastKind, UpdateKind};
use crate::ast::selector::{
    AttributeOperator, Combinator, ComplexSelector, PseudoClass, SelectorList, SimpleSelector,
};
use crate::ast::{MediaFeature, MediaQuery, MediaType, Rule, Stylesheet};
use crate::parse::{parse_declaration_list, parse_stylesheet, ParseOptions};
use crate::properties::registry::lookup;
use crate::selectors::PseudoElement;
use crate::style::custom::components_text;
use crate::values::serialize::format_number;
use crate::StyleChange;

/// An error from a CSSOM mutation. The sheet is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssomError {
    /// The index is past the end of the rule list.
    IndexSize,
    /// The text does not parse as a rule or declaration.
    Syntax,
    /// The rule at the index is not a style rule, so it has no selector or
    /// declarations.
    NotAStyleRule,
}

/// A stylesheet as a list of rules, with CSSOM-style mutation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CssomSheet {
    rules: Vec<Rule>,
}

impl CssomSheet {
    /// Parses a stylesheet. Parse diagnostics are dropped; they are available
    /// from [`crate::parse_stylesheet`] when needed.
    pub fn parse(text: &str) -> CssomSheet {
        CssomSheet {
            rules: parse_stylesheet(text, ParseOptions::default())
                .stylesheet
                .rules,
        }
    }

    /// Wraps a parsed stylesheet.
    pub fn from_stylesheet(sheet: Stylesheet) -> CssomSheet {
        CssomSheet { rules: sheet.rules }
    }

    /// The rules, in order.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// The number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// True if there are no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Inserts a rule parsed from `text` at `index` (CSSOM `insertRule`). Returns
    /// the index, and the change to report to the resolver.
    pub fn insert_rule(
        &mut self,
        text: &str,
        index: usize,
    ) -> Result<(usize, StyleChange), CssomError> {
        if index > self.rules.len() {
            return Err(CssomError::IndexSize);
        }
        let mut parsed = parse_stylesheet(text, ParseOptions::default())
            .stylesheet
            .rules;
        if parsed.len() != 1 {
            return Err(CssomError::Syntax);
        }
        self.rules.insert(index, parsed.remove(0));
        Ok((index, StyleChange::StyleSheetChanged))
    }

    /// Deletes the rule at `index` (CSSOM `deleteRule`).
    pub fn delete_rule(&mut self, index: usize) -> Result<StyleChange, CssomError> {
        if index >= self.rules.len() {
            return Err(CssomError::IndexSize);
        }
        self.rules.remove(index);
        Ok(StyleChange::StyleSheetChanged)
    }

    /// The selector text of the style rule at `index` (CSSOM `selectorText`).
    pub fn selector_text(&self, index: usize) -> Result<String, CssomError> {
        match self.rules.get(index) {
            Some(Rule::Style(style)) => Ok(serialize_selector_list(&style.selectors)),
            Some(_) => Err(CssomError::NotAStyleRule),
            None => Err(CssomError::IndexSize),
        }
    }

    /// Replaces the selectors of the style rule at `index`. The new text must
    /// parse as a selector list.
    pub fn set_selector_text(
        &mut self,
        index: usize,
        text: &str,
    ) -> Result<StyleChange, CssomError> {
        let list = parse_selector_text(text)?;
        match self.rules.get_mut(index) {
            Some(Rule::Style(style)) => {
                style.selectors = list;
                Ok(StyleChange::StyleSheetChanged)
            }
            Some(_) => Err(CssomError::NotAStyleRule),
            None => Err(CssomError::IndexSize),
        }
    }

    /// Sets a declaration on the style rule at `index`, replacing an existing one
    /// of the same name (CSSOM `setProperty`). `value` is the value text; a
    /// trailing `!important` in it is honored.
    pub fn set_property(
        &mut self,
        index: usize,
        name: &str,
        value: &str,
    ) -> Result<StyleChange, CssomError> {
        let parsed = parse_declaration_list(&format!("{name}: {value}"), ParseOptions::default());
        let [decl] = parsed.declarations.as_slice() else {
            return Err(CssomError::Syntax);
        };
        let decl = decl.clone();
        match self.rules.get_mut(index) {
            Some(Rule::Style(style)) => {
                let decls = &mut style.declarations.declarations;
                match decls.iter().position(|d| d.property == decl.property) {
                    Some(pos) => decls[pos] = decl,
                    None => decls.push(decl),
                }
                Ok(StyleChange::StyleSheetChanged)
            }
            Some(_) => Err(CssomError::NotAStyleRule),
            None => Err(CssomError::IndexSize),
        }
    }

    /// Removes a declaration from the style rule at `index` (CSSOM
    /// `removeProperty`). Returns whether one was removed.
    pub fn remove_property(
        &mut self,
        index: usize,
        name: &str,
    ) -> Result<(bool, StyleChange), CssomError> {
        match self.rules.get_mut(index) {
            Some(Rule::Style(style)) => {
                let decls = &mut style.declarations.declarations;
                let before = decls.len();
                decls.retain(|d| d.property != name);
                Ok((decls.len() != before, StyleChange::StyleSheetChanged))
            }
            Some(_) => Err(CssomError::NotAStyleRule),
            None => Err(CssomError::IndexSize),
        }
    }

    /// The value of a declaration on the style rule at `index`, as text.
    pub fn property(&self, index: usize, name: &str) -> Option<String> {
        match self.rules.get(index) {
            Some(Rule::Style(style)) => style
                .declarations
                .declarations
                .iter()
                .rev()
                .find(|d| d.property == name)
                .map(|d| components_text(&d.value)),
            _ => None,
        }
    }

    /// The sheet as a stylesheet for the resolver.
    pub fn to_stylesheet(&self) -> Stylesheet {
        Stylesheet {
            rules: self.rules.clone(),
        }
    }

    /// The canonical text of the sheet. Rules are separated by newlines.
    pub fn serialize(&self) -> String {
        self.rules
            .iter()
            .map(serialize_rule)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn parse_selector_text(text: &str) -> Result<SelectorList, CssomError> {
    let sheet = parse_stylesheet(&format!("{text} {{ }}"), ParseOptions::default()).stylesheet;
    match sheet.rules.into_iter().next() {
        Some(Rule::Style(style)) => Ok(style.selectors),
        _ => Err(CssomError::Syntax),
    }
}

fn serialize_rule(rule: &Rule) -> String {
    match rule {
        Rule::Style(style) => format!(
            "{} {{ {} }}",
            serialize_selector_list(&style.selectors),
            serialize_declarations(&style.declarations.declarations)
        ),
        Rule::Media(media) => format!(
            "@media {} {{\n{}\n}}",
            media
                .queries
                .iter()
                .map(serialize_media_query)
                .collect::<Vec<_>>()
                .join(", "),
            indent(
                &media
                    .rules
                    .iter()
                    .map(serialize_rule)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        ),
        Rule::Layer(layer) => match &layer.rules {
            None => format!("@layer {};", layer.names.join(", ")),
            Some(rules) => format!(
                "@layer {} {{\n{}\n}}",
                layer.names.first().map(String::as_str).unwrap_or(""),
                indent(
                    &rules
                        .iter()
                        .map(serialize_rule)
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            ),
        },
        Rule::Supports(supports) => format!(
            "@supports {} {{\n{}\n}}",
            if supports.supported {
                "(supported)"
            } else {
                "(unsupported)"
            },
            indent(
                &supports
                    .rules
                    .iter()
                    .map(serialize_rule)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        ),
        Rule::Container(container) => format!(
            "@container{} {} {{\n{}\n}}",
            container
                .name
                .as_deref()
                .map(|n| format!(" {n}"))
                .unwrap_or_default(),
            container
                .features
                .iter()
                .map(|f| format!("({})", serialize_feature(f)))
                .collect::<Vec<_>>()
                .join(" and "),
            indent(
                &container
                    .rules
                    .iter()
                    .map(serialize_rule)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        ),
        Rule::Scope(scope) => {
            let mut prelude = String::new();
            if let Some(root) = &scope.root {
                prelude.push_str(&format!("({})", serialize_selector_list(root)));
            }
            if let Some(limit) = &scope.limit {
                prelude.push_str(&format!(" to ({})", serialize_selector_list(limit)));
            }
            format!(
                "@scope {} {{\n{}\n}}",
                prelude.trim(),
                indent(
                    &scope
                        .rules
                        .iter()
                        .map(serialize_rule)
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            )
        }
        Rule::Property(property) => format!(
            "@property {} {{ syntax: {}; inherits: {}; }}",
            property.name,
            property
                .syntax
                .as_deref()
                .map(|s| format!("\"{s}\""))
                .unwrap_or_else(|| "\"*\"".into()),
            property.inherits
        ),
        Rule::Keyframes(keyframes) => format!("@keyframes {} {{ }}", keyframes.name),
        Rule::FontFace(_) => "@font-face { }".to_string(),
        Rule::Unknown(unknown) => format!("@{};", unknown.name),
    }
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn serialize_declarations(decls: &[crate::ast::Declaration]) -> String {
    decls
        .iter()
        .map(|d| {
            let mut s = format!("{}: {}", d.property, components_text(&d.value));
            if d.important {
                s.push_str(" !important");
            }
            s
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn serialize_media_query(query: &MediaQuery) -> String {
    let mut parts = Vec::new();
    if query.negated {
        parts.push("not".to_string());
    }
    parts.push(match &query.media_type {
        MediaType::All => "all".to_string(),
        MediaType::Screen => "screen".to_string(),
        MediaType::Print => "print".to_string(),
        MediaType::Unknown(name) => name.clone(),
    });
    for feature in &query.features {
        parts.push(format!("and ({})", serialize_feature(feature)));
    }
    parts.join(" ")
}

fn serialize_feature(feature: &MediaFeature) -> String {
    use crate::ast::Comparison;
    let len = |v: f64, u: crate::values::LengthUnit| {
        format!("{}{}", format_number(v), crate::values::calc::unit_name(u))
    };
    let cmp_name = |base: &str, cmp: Comparison| match cmp {
        Comparison::Equal => base.to_string(),
        Comparison::Min => format!("min-{base}"),
        Comparison::Max => format!("max-{base}"),
    };
    match feature {
        MediaFeature::Width(cmp, v, u) => format!("{}: {}", cmp_name("width", *cmp), len(*v, *u)),
        MediaFeature::Height(cmp, v, u) => format!("{}: {}", cmp_name("height", *cmp), len(*v, *u)),
        MediaFeature::AspectRatio(cmp, r) => {
            format!("{}: {}", cmp_name("aspect-ratio", *cmp), format_number(*r))
        }
        MediaFeature::Landscape(l) => {
            format!("orientation: {}", if *l { "landscape" } else { "portrait" })
        }
        MediaFeature::PrefersDark(d) => format!(
            "prefers-color-scheme: {}",
            if *d { "dark" } else { "light" }
        ),
        MediaFeature::PrefersReducedMotion(r) => {
            format!(
                "prefers-reduced-motion: {}",
                if *r { "reduce" } else { "no-preference" }
            )
        }
        MediaFeature::Hover(h) => format!("hover: {}", if *h { "hover" } else { "none" }),
        MediaFeature::Pointer(p) => format!(
            "pointer: {}",
            match p {
                crate::ast::PointerKind::None => "none",
                crate::ast::PointerKind::Coarse => "coarse",
                crate::ast::PointerKind::Fine => "fine",
            }
        ),
        MediaFeature::PrefersContrast(kind) => format!(
            "prefers-contrast: {}",
            match kind {
                ContrastKind::NoPreference => "no-preference",
                ContrastKind::More => "more",
                ContrastKind::Less => "less",
                ContrastKind::Custom => "custom",
            }
        ),
        MediaFeature::DynamicRange(high) => {
            format!("dynamic-range: {}", if *high { "high" } else { "standard" })
        }
        MediaFeature::Update(kind) => format!(
            "update: {}",
            match kind {
                UpdateKind::None => "none",
                UpdateKind::Slow => "slow",
                UpdateKind::Fast => "fast",
            }
        ),
        MediaFeature::Resolution(cmp, dppx) => {
            format!(
                "{}: {}dppx",
                cmp_name("resolution", *cmp),
                format_number(*dppx)
            )
        }
        MediaFeature::Unknown(name) => name.clone(),
    }
}

/// Serializes a selector list, comma-separated.
pub fn serialize_selector_list(list: &SelectorList) -> String {
    list.selectors
        .iter()
        .map(serialize_complex)
        .collect::<Vec<_>>()
        .join(", ")
}

fn serialize_complex(selector: &ComplexSelector) -> String {
    // `::slotted(compound)` keeps its argument in the compound, so it is written as
    // the pseudo-element alone.
    if let (Some(PseudoElement::Slotted), [compound]) =
        (&selector.pseudo_element, selector.compounds.as_slice())
    {
        let argument: String = compound.simple.iter().map(serialize_simple).collect();
        return format!("::slotted({argument})");
    }
    let mut out = String::new();
    for (i, compound) in selector.compounds.iter().enumerate() {
        if i > 0 {
            out.push_str(match selector.combinators[i - 1] {
                Combinator::Descendant => " ",
                Combinator::Child => " > ",
                Combinator::NextSibling => " + ",
                Combinator::SubsequentSibling => " ~ ",
            });
        }
        for simple in &compound.simple {
            out.push_str(&serialize_simple(simple));
        }
    }
    if let Some(pe) = &selector.pseudo_element {
        out.push_str(&serialize_pseudo_element(pe));
    }
    out
}

fn serialize_pseudo_element(pe: &PseudoElement) -> String {
    match pe {
        PseudoElement::Before => "::before".into(),
        PseudoElement::After => "::after".into(),
        PseudoElement::Marker => "::marker".into(),
        PseudoElement::Placeholder => "::placeholder".into(),
        PseudoElement::Selection => "::selection".into(),
        PseudoElement::Backdrop => "::backdrop".into(),
        PseudoElement::FileSelectorButton => "::file-selector-button".into(),
        PseudoElement::Part(n) => format!("::part({n})"),
        PseudoElement::Slotted => "::slotted()".into(),
        PseudoElement::Highlight(n) => format!("::highlight({n})"),
        PseudoElement::Unknown(n) => format!("::{n}"),
    }
}

fn serialize_simple(simple: &SimpleSelector) -> String {
    match simple {
        SimpleSelector::Universal => "*".into(),
        SimpleSelector::Type(name) => name.clone(),
        SimpleSelector::Id(id) => format!("#{id}"),
        SimpleSelector::Class(class) => format!(".{class}"),
        SimpleSelector::Attribute(attr) => match &attr.matcher {
            None => format!("[{}]", attr.name),
            Some(m) => {
                let op = match m.operator {
                    AttributeOperator::Equals => "=",
                    AttributeOperator::Includes => "~=",
                    AttributeOperator::DashMatch => "|=",
                    AttributeOperator::Prefix => "^=",
                    AttributeOperator::Suffix => "$=",
                    AttributeOperator::Substring => "*=",
                };
                format!("[{}{}\"{}\"]", attr.name, op, m.value.replace('"', "\\\""))
            }
        },
        SimpleSelector::PseudoClass(pseudo) => format!(":{}", pseudo_class_name(*pseudo)),
        SimpleSelector::Nth(nth) => {
            let name = match (nth.last, nth.of_type) {
                (false, false) => "nth-child",
                (true, false) => "nth-last-child",
                (false, true) => "nth-of-type",
                (true, true) => "nth-last-of-type",
            };
            let of = nth
                .of
                .as_ref()
                .map(|l| format!(" of {}", serialize_selector_list(l)))
                .unwrap_or_default();
            format!(":{name}({}{of})", anb_text(nth.a, nth.b))
        }
        SimpleSelector::Not(list) => format!(":not({})", serialize_selector_list(list)),
        SimpleSelector::Is(list) => format!(":is({})", serialize_selector_list(list)),
        SimpleSelector::Where(list) => format!(":where({})", serialize_selector_list(list)),
        SimpleSelector::TimeState { state, list } => {
            let name = match state {
                PseudoClass::Past => "past",
                PseudoClass::Future => "future",
                _ => "current",
            };
            format!(":{name}({})", serialize_selector_list(list))
        }
        SimpleSelector::Lang(tag) => format!(":lang({tag})"),
        SimpleSelector::Dir(dir) => format!(
            ":dir({})",
            match dir {
                crate::tree::Direction::Ltr => "ltr",
                crate::tree::Direction::Rtl => "rtl",
            }
        ),
        SimpleSelector::Has(relatives) => format!(
            ":has({})",
            relatives
                .iter()
                .map(|r| {
                    let lead = match r.leading {
                        Combinator::Descendant => "",
                        Combinator::Child => "> ",
                        Combinator::NextSibling => "+ ",
                        Combinator::SubsequentSibling => "~ ",
                    };
                    format!("{lead}{}", serialize_complex(&r.selector))
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        SimpleSelector::Scope => ":scope".into(),
        SimpleSelector::CustomState(name) => format!(":state({name})"),
        SimpleSelector::Host(None) => ":host".into(),
        SimpleSelector::Host(Some(list)) => format!(":host({})", serialize_selector_list(list)),
        SimpleSelector::HostContext(list) => {
            format!(":host-context({})", serialize_selector_list(list))
        }
        SimpleSelector::Nesting => "&".into(),
    }
}

/// The CSS name of a pseudo-class (without the colon).
fn pseudo_class_name(pseudo: PseudoClass) -> &'static str {
    match pseudo {
        PseudoClass::Root => "root",
        PseudoClass::Hover => "hover",
        PseudoClass::Active => "active",
        PseudoClass::Focus => "focus",
        PseudoClass::Empty => "empty",
        PseudoClass::Enabled => "enabled",
        PseudoClass::Disabled => "disabled",
        PseudoClass::Checked => "checked",
        PseudoClass::Required => "required",
        PseudoClass::Optional => "optional",
        PseudoClass::ReadOnly => "read-only",
        PseudoClass::ReadWrite => "read-write",
        PseudoClass::PlaceholderShown => "placeholder-shown",
        PseudoClass::Link => "link",
        PseudoClass::AnyLink => "any-link",
        PseudoClass::Visited => "visited",
        PseudoClass::FocusVisible => "focus-visible",
        PseudoClass::FocusWithin => "focus-within",
        PseudoClass::Target => "target",
        PseudoClass::Valid => "valid",
        PseudoClass::Invalid => "invalid",
        PseudoClass::UserValid => "user-valid",
        PseudoClass::UserInvalid => "user-invalid",
        PseudoClass::InRange => "in-range",
        PseudoClass::OutOfRange => "out-of-range",
        PseudoClass::Default => "default",
        PseudoClass::Indeterminate => "indeterminate",
        PseudoClass::Open => "open",
        PseudoClass::Closed => "closed",
        PseudoClass::Modal => "modal",
        PseudoClass::PopoverOpen => "popover-open",
        PseudoClass::Current => "current",
        PseudoClass::Past => "past",
        PseudoClass::Future => "future",
        PseudoClass::Defined => "defined",
        PseudoClass::LocalLink => "local-link",
        PseudoClass::Playing => "playing",
        PseudoClass::Paused => "paused",
        PseudoClass::Fullscreen => "fullscreen",
        PseudoClass::Autofill => "autofill",
        PseudoClass::Muted => "muted",
        PseudoClass::Buffering => "buffering",
        PseudoClass::Stalled => "stalled",
        PseudoClass::Seeking => "seeking",
        PseudoClass::VolumeLocked => "volume-locked",
    }
}

/// Canonical `An+B` text.
fn anb_text(a: i64, b: i64) -> String {
    if a == 0 {
        return b.to_string();
    }
    let a_part = match a {
        1 => String::new(),
        -1 => "-".to_string(),
        other => other.to_string(),
    };
    match b {
        0 => format!("{a_part}n"),
        b if b > 0 => format!("{a_part}n+{b}"),
        b => format!("{a_part}n{b}"),
    }
}

/// Looks up a property for a host that wants to validate names before setting
/// them. Returns `false` for unknown names and for custom properties (`--x`),
/// which are always accepted.
pub fn is_known_property(name: &str) -> bool {
    name.starts_with("--") || lookup(&name.to_ascii_lowercase()).is_some()
}
