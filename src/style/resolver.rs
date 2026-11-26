// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Style resolution (spec 8.2, 16.1).
//!
//! Per target (an element, or one pseudo-element), the resolver collects the
//! matching rules into a [`Cascaded`] set. It then computes values in
//! dependency order: custom properties first, then font-size, color,
//! font-weight, and the rest. Each step applies inheritance and CSS-wide
//! keywords. `var()` is substituted at computed-value time (spec 13.7), and
//! shorthands are expanded before the cascade (spec 13.6).
//!
//! Media conditions are evaluated once per resolve call, and only rules whose
//! conditions hold take part in matching.
//!
//! Tree traversal uses an explicit stack, so deep trees cannot overflow the call
//! stack. Incremental restyle is in [`crate::style::invalidation`].

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Combinator, ComplexSelector, Declaration, MediaQuery, PseudoClass, Rule, SelectorList,
    SimpleSelector, Stylesheet,
};
use crate::cascade::order::DeclarationOrder;
use crate::cascade::origin::Origin;
use crate::cascade::CascadeKey;
use crate::env::{media, Environment};
use crate::limits::ParseLimits;
use crate::properties::registry::{ids, spec, PROPERTIES};
use crate::properties::shorthand::{expand, shorthand_index, SHORTHANDS};
use crate::properties::{lookup, parse_property_value, CssWide, PropertyId, SpecifiedValue};
use crate::selectors::matching::matches_complex;
use crate::selectors::{PseudoElement, Specificity};
use crate::style::computed::{compute_value, ComputeContext, ComputedStyle, StyleMap};
use crate::style::custom::{
    contains_var, resolve_customs, substitute, CustomPropertyValue, MapResolver,
};
use crate::syntax::{trim_whitespace, ComponentValue, TokenKind};
use crate::tree::{NodeId, NodeKind, StyleTree};
use crate::values::{Color, Value};

/// Identifies a stylesheet added to a resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SheetId(pub u32);

/// Resolver configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResolverConfig {
    /// Limits applied during matching and substitution (spec 20.3).
    pub limits: ParseLimits,
}

/// What a declaration sets.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DeclTarget {
    /// A registered property.
    Prop(PropertyId),
    /// A custom property, by name.
    Custom(String),
}

/// A declaration after lookup and value parsing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompiledDecl {
    pub target: DeclTarget,
    pub value: SpecifiedValue,
    pub important: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledRule {
    pub selectors: SelectorList,
    pub decls: Vec<CompiledDecl>,
    /// Media conditions from enclosing `@media` rules. Each entry is a query list,
    /// and every entry must match for the rule to apply.
    pub conditions: Vec<Vec<MediaQuery>>,
    /// True if a selector depends on sibling structure, so a change to one node
    /// may affect its siblings.
    pub structural: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedSheet {
    pub id: SheetId,
    pub origin: Origin,
    pub rules: Vec<CompiledRule>,
}

/// The winning declaration for one property or custom property.
#[derive(Debug, Clone)]
pub(crate) struct Winner {
    pub value: SpecifiedValue,
    pub key: CascadeKey,
}

/// Winning declarations for one target: an element, or one pseudo-element.
#[derive(Debug, Clone)]
pub(crate) struct Cascaded {
    pub props: Vec<Option<Winner>>,
    pub custom: BTreeMap<String, Winner>,
}

impl Cascaded {
    pub(crate) fn new() -> Cascaded {
        Cascaded {
            props: vec![None; PROPERTIES.len()],
            custom: BTreeMap::new(),
        }
    }

    /// Offers a declaration to this target. The higher cascade key wins (spec 13.3).
    fn offer(&mut self, target: &DeclTarget, value: &SpecifiedValue, key: CascadeKey) {
        match target {
            DeclTarget::Prop(id) => {
                let slot = &mut self.props[id.index()];
                let replace = match slot {
                    None => true,
                    Some(current) => key > current.key,
                };
                if replace {
                    *slot = Some(Winner {
                        value: value.clone(),
                        key,
                    });
                }
            }
            DeclTarget::Custom(name) => {
                let replace = match self.custom.get(name) {
                    None => true,
                    Some(current) => key > current.key,
                };
                if replace {
                    self.custom.insert(
                        name.clone(),
                        Winner {
                            value: value.clone(),
                            key,
                        },
                    );
                }
            }
        }
    }
}

/// Work item for the iterative traversal.
pub(crate) struct Frame {
    pub node: NodeId,
    /// Computed style of the nearest element ancestor, if any.
    pub parent: Option<ComputedStyle>,
    /// Root font size for `rem`. `None` means this element is the root.
    pub rem_base: Option<f64>,
}

/// The last full result, with the environment it was computed for.
#[derive(Debug, Clone)]
pub(crate) struct Cache {
    pub map: StyleMap,
    pub env: Environment,
}

/// Resolves computed styles for trees that implement [`StyleTree`].
///
/// The resolver keeps the last result so that [`StyleResolver::restyle`] can
/// recompute only the parts of the tree that changed. See
/// [`crate::style::invalidation`].
#[derive(Debug, Default)]
pub struct StyleResolver {
    pub(crate) config: ResolverConfig,
    pub(crate) sheets: Vec<LoadedSheet>,
    pub(crate) next_sheet: u32,
    pub(crate) cache: Option<Cache>,
    pub(crate) dirty: BTreeSet<NodeId>,
    pub(crate) full_dirty: bool,
    pub(crate) last_restyled: usize,
}

impl StyleResolver {
    /// Creates an empty resolver.
    pub fn new(config: ResolverConfig) -> StyleResolver {
        StyleResolver {
            config,
            ..StyleResolver::default()
        }
    }

    /// Adds a parsed stylesheet with the given origin (spec 8.2).
    ///
    /// Declarations that name unknown properties or have invalid values are
    /// ignored here, since the parser already reported them. Rules inside
    /// `@media` keep their conditions and apply only when those hold.
    pub fn add_stylesheet(&mut self, sheet: Stylesheet, origin: Origin) -> SheetId {
        let id = SheetId(self.next_sheet);
        self.next_sheet += 1;
        let mut rules = Vec::new();
        flatten_rules(sheet.rules, &[], &mut rules);
        self.sheets.push(LoadedSheet { id, origin, rules });
        self.full_dirty = true;
        id
    }

    /// Removes a stylesheet. Unknown ids are ignored.
    pub fn remove_stylesheet(&mut self, id: SheetId) {
        self.sheets.retain(|s| s.id != id);
        self.full_dirty = true;
    }

    /// Computes styles for every element in `tree` (spec 8.2).
    pub fn resolve<T: StyleTree>(&mut self, tree: &T, env: &Environment) -> StyleMap {
        let active = self.active_rules(env);
        let mut map = StyleMap::default();
        let max_var_depth = self.config.limits.max_var_depth;
        let start = Frame {
            node: tree.root(),
            parent: None,
            rem_base: None,
        };
        self.last_restyled = style_tree(self, tree, start, env, &active, max_var_depth, &mut map);
        self.cache = Some(Cache {
            map: map.clone(),
            env: *env,
        });
        self.dirty.clear();
        self.full_dirty = false;
        map
    }

    /// Whether each rule's media conditions hold in `env`, indexed by sheet, then rule.
    pub(crate) fn active_rules(&self, env: &Environment) -> Vec<Vec<bool>> {
        self.sheets
            .iter()
            .map(|sheet| {
                sheet
                    .rules
                    .iter()
                    .map(|rule| rule.conditions.iter().all(|q| media::list_matches(q, env)))
                    .collect()
            })
            .collect()
    }

    /// Collects the cascaded declarations for each target of `node` (spec 13).
    pub(crate) fn cascade<T: StyleTree>(
        &self,
        tree: &T,
        node: NodeId,
        active: &[Vec<bool>],
    ) -> BTreeMap<Option<PseudoElement>, Cascaded> {
        let mut out: BTreeMap<Option<PseudoElement>, Cascaded> = BTreeMap::new();
        let max_steps = self.config.limits.max_match_steps;

        for (sheet_pos, sheet) in self.sheets.iter().enumerate() {
            for (rule_idx, rule) in sheet.rules.iter().enumerate() {
                if !active[sheet_pos][rule_idx] {
                    continue;
                }
                for selector in &rule.selectors.selectors {
                    let mut budget = max_steps;
                    if !matches_complex(tree, node, selector, &mut budget) {
                        continue;
                    }
                    let cas = out
                        .entry(selector.pseudo_element.clone())
                        .or_insert_with(Cascaded::new);
                    for (decl_idx, decl) in rule.decls.iter().enumerate() {
                        let key = CascadeKey {
                            origin: sheet.origin,
                            important: decl.important,
                            inline: false,
                            specificity: selector.specificity,
                            order: DeclarationOrder {
                                sheet: sheet_pos as u32,
                                rule: rule_idx as u32,
                                declaration: decl_idx as u32,
                            },
                        };
                        cas.offer(&decl.target, &decl.value, key);
                    }
                }
            }
        }

        if let Some(inline) = tree.inline_style(node) {
            let cas = out.entry(None).or_insert_with(Cascaded::new);
            let mut index = 0u32;
            for decl in inline {
                for compiled in compile_declaration(decl) {
                    let key = CascadeKey {
                        origin: Origin::Author,
                        important: compiled.important,
                        inline: true,
                        specificity: Specificity::default(),
                        order: DeclarationOrder::inline(index),
                    };
                    cas.offer(&compiled.target, &compiled.value, key);
                    index += 1;
                }
            }
        }
        out
    }
}

/// Flattens `@media` groups into a list of style rules, keeping each group's
/// conditions.
fn flatten_rules(rules: Vec<Rule>, conditions: &[Vec<MediaQuery>], out: &mut Vec<CompiledRule>) {
    for rule in rules {
        match rule {
            Rule::Style(style) => {
                let decls = style
                    .declarations
                    .declarations
                    .iter()
                    .flat_map(compile_declaration)
                    .collect();
                let structural = style.selectors.selectors.iter().any(selector_is_structural);
                out.push(CompiledRule {
                    selectors: style.selectors,
                    decls,
                    conditions: conditions.to_vec(),
                    structural,
                });
            }
            Rule::Media(media_rule) => {
                let mut nested = conditions.to_vec();
                nested.push(media_rule.queries);
                flatten_rules(media_rule.rules, &nested, out);
            }
            Rule::Unknown(_) => {}
        }
    }
}

/// True if a selector depends on sibling structure.
pub(crate) fn selector_is_structural(selector: &ComplexSelector) -> bool {
    let sibling_combinator = selector
        .combinators
        .iter()
        .any(|c| matches!(c, Combinator::NextSibling | Combinator::SubsequentSibling));
    if sibling_combinator {
        return true;
    }
    selector.compounds.iter().any(|compound| {
        compound.simple.iter().any(|simple| match simple {
            SimpleSelector::Nth(_) | SimpleSelector::PseudoClass(PseudoClass::Empty) => true,
            SimpleSelector::Not(list) | SimpleSelector::Is(list) | SimpleSelector::Where(list) => {
                list.selectors.iter().any(selector_is_structural)
            }
            _ => false,
        })
    })
}

/// Resolves the subtree rooted at `start.node` into `map`. Returns the number of
/// elements styled.
pub(crate) fn style_tree<T: StyleTree>(
    resolver: &StyleResolver,
    tree: &T,
    start: Frame,
    env: &Environment,
    active: &[Vec<bool>],
    max_var_depth: usize,
    map: &mut StyleMap,
) -> usize {
    let mut styled = 0;
    let mut stack = vec![start];
    while let Some(frame) = stack.pop() {
        match tree.kind(frame.node) {
            NodeKind::Element => {
                let root_fs = frame.rem_base.unwrap_or(env.root_font_size);
                let mut targets = resolver.cascade(tree, frame.node, active);
                let element_cas = targets.remove(&None).unwrap_or_else(Cascaded::new);
                let style = compute_style(
                    &element_cas,
                    frame.parent.as_ref(),
                    root_fs,
                    env,
                    max_var_depth,
                );

                map.clear_pseudo(frame.node);
                for (target, cas) in targets {
                    if let Some(pseudo) = target {
                        let pstyle = compute_style(&cas, Some(&style), root_fs, env, max_var_depth);
                        if is_generated(&pstyle) {
                            map.insert_pseudo(frame.node, pseudo, pstyle);
                        }
                    }
                }

                let child_rem = Some(frame.rem_base.unwrap_or(style.font_size()));
                for &child in tree.children(frame.node).iter().rev() {
                    stack.push(Frame {
                        node: child,
                        parent: Some(style.clone()),
                        rem_base: child_rem,
                    });
                }
                map.insert(frame.node, style);
                styled += 1;
            }
            NodeKind::Text | NodeKind::Comment => {}
            NodeKind::Document | NodeKind::Fragment | NodeKind::Pseudo(_) => {
                // Non-element containers do not provide inherited values.
                for &child in tree.children(frame.node).iter().rev() {
                    stack.push(Frame {
                        node: child,
                        parent: None,
                        rem_base: frame.rem_base,
                    });
                }
            }
        }
    }
    styled
}

/// A pseudo-element is generated only when its `content` is not `none` or `normal`.
fn is_generated(style: &ComputedStyle) -> bool {
    !matches!(
        style.raw(ids::CONTENT),
        Value::Keyword("none") | Value::Keyword("normal")
    )
}

/// Computes the style of one target from its cascaded declarations.
fn compute_style(
    cas: &Cascaded,
    parent: Option<&ComputedStyle>,
    root_fs: f64,
    env: &Environment,
    max_var_depth: usize,
) -> ComputedStyle {
    let declared: BTreeMap<String, SpecifiedValue> = cas
        .custom
        .iter()
        .map(|(name, w)| (name.clone(), w.value.clone()))
        .collect();
    let customs = resolve_customs(&declared, parent.map(|p| p.customs()), max_var_depth);

    let parent_font_size = parent.map_or(env.root_font_size, |p| p.font_size());
    let parent_color = parent.map_or(Color::BLACK, |p| p.color());
    let parent_weight = parent.map_or(400.0, |p| p.font_weight());
    let mut ctx = ComputeContext {
        parent_font_size,
        root_font_size: root_fs,
        font_size: parent_font_size,
        parent_color,
        color: parent_color,
        parent_weight,
        viewport_width: env.viewport.width,
        viewport_height: env.viewport.height,
    };

    let mut values: Vec<Value> = PROPERTIES.iter().map(|p| p.initial.to_value()).collect();
    for idx in compute_order() {
        let id = PropertyId(idx as u16);
        let specified = specified_value(
            idx,
            cas.props[idx].as_ref(),
            parent,
            &customs,
            max_var_depth,
        );
        let computed = compute_value(id, spec(id).grammar, specified, &ctx);
        if id == ids::FONT_SIZE {
            if let Value::Length { value, .. } = computed {
                ctx.font_size = value;
            }
        }
        if id == ids::COLOR {
            if let Value::Color(c) = computed {
                ctx.color = c;
            }
        }
        values[idx] = computed;
    }

    // CSS Backgrounds 3, border-width computed value: zero when the style is none
    // or hidden.
    for side in 0..4 {
        let style_idx = ids::BORDER_TOP_STYLE.index() + side;
        let width_idx = ids::BORDER_TOP_WIDTH.index() + side;
        if matches!(
            values[style_idx],
            Value::Keyword("none") | Value::Keyword("hidden")
        ) {
            values[width_idx] = Value::px(0.0);
        }
    }

    ComputedStyle::from_values(values, customs)
}

/// Order in which properties are computed: font-size first (its `em` depends on
/// the parent), then color (for `currentcolor`), then font-weight (for the
/// relative keywords), then the rest in registry order.
fn compute_order() -> impl Iterator<Item = usize> {
    let first = [
        ids::FONT_SIZE.index(),
        ids::COLOR.index(),
        ids::FONT_WEIGHT.index(),
    ];
    first
        .into_iter()
        .chain((0..PROPERTIES.len()).filter(move |i| !first.contains(i)))
}

/// Resolves the winning declaration for property `idx` to a value (spec 13.8).
fn specified_value(
    idx: usize,
    winner: Option<&Winner>,
    parent: Option<&ComputedStyle>,
    customs: &BTreeMap<String, CustomPropertyValue>,
    max_var_depth: usize,
) -> Value {
    let info = &PROPERTIES[idx];
    let concrete = match winner.map(|w| &w.value) {
        None => SpecifiedValue::CssWide(CssWide::Unset),
        Some(SpecifiedValue::Unparsed { tokens, shorthand }) => {
            resolve_unparsed(idx, tokens, *shorthand, customs, max_var_depth)
                .unwrap_or(SpecifiedValue::CssWide(CssWide::Unset))
        }
        Some(other) => other.clone(),
    };
    let initial = || info.initial.to_value();
    let inherit = || match parent {
        Some(p) => p.raw(PropertyId(idx as u16)).clone(),
        None => initial(),
    };
    match concrete {
        SpecifiedValue::Value(v) => v,
        SpecifiedValue::CssWide(CssWide::Initial) => initial(),
        SpecifiedValue::CssWide(CssWide::Inherit) => inherit(),
        SpecifiedValue::CssWide(CssWide::Unset) => {
            if info.inherited {
                inherit()
            } else {
                initial()
            }
        }
        // Substitution produces a parsed value, never another unparsed one.
        SpecifiedValue::Unparsed { .. } => initial(),
    }
}

/// Substitutes `var()`, then parses the result for property `idx`. Returns
/// `None` when the value is invalid at computed-value time (spec 13.7 step 8).
fn resolve_unparsed(
    idx: usize,
    tokens: &[ComponentValue],
    shorthand: Option<usize>,
    customs: &BTreeMap<String, CustomPropertyValue>,
    max_var_depth: usize,
) -> Option<SpecifiedValue> {
    let mut resolver = MapResolver(customs);
    let substituted = substitute(tokens, &mut resolver, 0, max_var_depth).ok()?;
    let id = PropertyId(idx as u16);
    match shorthand {
        Some(sid) => {
            let name = spec(id).name;
            expand(sid, &substituted)
                .ok()?
                .into_iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v)
        }
        None => parse_property_value(spec(id), &substituted).ok(),
    }
}

/// Looks up and parses a declaration, expanding shorthands (spec 13.6, 15.3).
///
/// A value that contains `var()` stays unparsed, along with the shorthand it
/// came from if any. Unknown properties and invalid values yield nothing.
pub(crate) fn compile_declaration(decl: &Declaration) -> Vec<CompiledDecl> {
    let important = decl.important;
    if decl.property.starts_with("--") {
        let value = match css_wide_only(&decl.value) {
            Some(wide) => SpecifiedValue::CssWide(wide),
            None => SpecifiedValue::Unparsed {
                tokens: decl.value.clone(),
                shorthand: None,
            },
        };
        return vec![CompiledDecl {
            target: DeclTarget::Custom(decl.property.clone()),
            value,
            important,
        }];
    }

    let has_var = contains_var(&decl.value);
    if let Some(sid) = shorthand_index(&decl.property) {
        if has_var {
            return SHORTHANDS[sid]
                .longhands
                .iter()
                .filter_map(|name| lookup(name))
                .map(|id| CompiledDecl {
                    target: DeclTarget::Prop(id),
                    value: SpecifiedValue::Unparsed {
                        tokens: decl.value.clone(),
                        shorthand: Some(sid),
                    },
                    important,
                })
                .collect();
        }
        return match expand(sid, &decl.value) {
            Ok(list) => list
                .into_iter()
                .filter_map(|(name, value)| {
                    lookup(name).map(|id| CompiledDecl {
                        target: DeclTarget::Prop(id),
                        value,
                        important,
                    })
                })
                .collect(),
            Err(_) => Vec::new(),
        };
    }

    let Some(id) = lookup(&decl.property) else {
        return Vec::new();
    };
    if has_var {
        return vec![CompiledDecl {
            target: DeclTarget::Prop(id),
            value: SpecifiedValue::Unparsed {
                tokens: decl.value.clone(),
                shorthand: None,
            },
            important,
        }];
    }
    match parse_property_value(spec(id), &decl.value) {
        Ok(value) => vec![CompiledDecl {
            target: DeclTarget::Prop(id),
            value,
            important,
        }],
        Err(_) => Vec::new(),
    }
}

/// A value that is exactly one CSS-wide keyword.
fn css_wide_only(values: &[ComponentValue]) -> Option<CssWide> {
    match trim_whitespace(values) {
        [ComponentValue::Token(t)] => match &t.kind {
            TokenKind::Ident(s) => match s.to_ascii_lowercase().as_str() {
                "initial" => Some(CssWide::Initial),
                "inherit" => Some(CssWide::Inherit),
                "unset" => Some(CssWide::Unset),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
