// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Style resolution (spec 8.2, 13, 16.1).
//!
//! Per target (an element, or one pseudo-element), the resolver collects every
//! candidate declaration into a [`Cascaded`] set. It then picks winners per
//! property, handling `revert` and `revert-layer` (spec 13.8), and computes
//! values in dependency order: custom properties first, then font-size, color,
//! font-weight, and the rest. Each step applies inheritance and CSS-wide
//! keywords. `var()` is substituted at computed-value time (spec 13.7).
//!
//! Rules are flattened once, at `add_stylesheet`. Each flat rule records its
//! `@media` and `@container` conditions, its cascade layer, and whether a selector
//! uses `:visited`. Media conditions are evaluated once per resolve call.
//! Container conditions are evaluated per element, through the host's
//! [`ContainerSizeProvider`].
//!
//! Tree traversal uses an explicit stack, so deep trees cannot overflow the call
//! stack. Incremental restyle is in [`crate::style::invalidation`].

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Combinator, ComplexSelector, Declaration, FontFaceRule, KeyframesRule, MediaFeature,
    MediaQuery, PseudoClass, Rule, SelectorList, SimpleSelector, Stylesheet,
};
use crate::cascade::order::DeclarationOrder;
use crate::cascade::origin::Origin;
use crate::cascade::{layer_rank, CascadeKey, UNLAYERED_IMPORTANT, UNLAYERED_NORMAL};
use crate::env::container::{features_match, ContainerSizeProvider};
use crate::env::{media, Environment};
use crate::limits::ParseLimits;
use crate::properties::registry::{ids, lookup, spec, PROPERTIES};
use crate::properties::shorthand::{expand, shorthand_index, SHORTHANDS};
use crate::properties::{parse_property_value, CssWide, PropertyId, SpecifiedValue};
use crate::selectors::matching::matches_complex;
use crate::selectors::{PseudoElement, Specificity};
use crate::style::computed::{compute_value, ComputeContext, ComputedStyle, StyleMap};
use crate::style::custom::{
    contains_var, resolve_customs, substitute, CustomPropertyValue, MapResolver, RegisteredProperty,
};
use crate::syntax::{trim_whitespace, ComponentValue, TokenKind};
use crate::tree::{NodeId, NodeKind, StyleTree};
use crate::values::{Color, Value};

/// Identifies a stylesheet added to a resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SheetId(pub u32);

/// Whether `:visited` is honored (spec 20.2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VisitedPolicy {
    /// Web semantics. A selector with `:visited` applies only to the color
    /// properties (`color`, `background-color`, `border-*-color`), so that a
    /// page cannot learn history from other properties.
    #[default]
    Restricted,
    /// AppFront mode. A selector with `:visited` never matches.
    Disabled,
}

/// Resolver configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResolverConfig {
    /// Limits applied during matching and substitution (spec 20.3).
    pub limits: ParseLimits,
    /// Visited-link policy (spec 20.2).
    pub visited: VisitedPolicy,
}

/// A `@container` condition attached to a rule.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ContainerCondition {
    pub name: Option<String>,
    pub features: Vec<MediaFeature>,
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

/// A style rule after flattening: its nesting conditions are attached, not
/// nested.
#[derive(Debug, Clone)]
pub(crate) struct CompiledRule {
    pub selectors: SelectorList,
    pub decls: Vec<CompiledDecl>,
    /// `@media` query lists from enclosing rules. Every list must match.
    pub conditions: Vec<Vec<MediaQuery>>,
    /// `@container` conditions from enclosing rules. Every condition must match.
    pub containers: Vec<ContainerCondition>,
    /// The cascade layer index, or `None` if unlayered.
    pub layer: Option<usize>,
    /// Per selector: whether it contains `:visited`.
    pub visited: Vec<bool>,
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

/// The winning declaration for one property or custom property, with its key.
#[derive(Debug, Clone)]
pub(crate) struct Winner {
    pub value: SpecifiedValue,
    pub key: CascadeKey,
}

/// All candidate declarations for one target: an element, or one pseudo-element.
/// Winners are picked later, because `revert` needs the lower candidates.
#[derive(Debug, Clone)]
pub(crate) struct Cascaded {
    pub props: Vec<Vec<Winner>>,
    pub custom: BTreeMap<String, Vec<Winner>>,
}

impl Cascaded {
    pub(crate) fn new() -> Cascaded {
        Cascaded {
            props: vec![Vec::new(); PROPERTIES.len()],
            custom: BTreeMap::new(),
        }
    }

    fn offer(&mut self, target: &DeclTarget, value: &SpecifiedValue, key: CascadeKey) {
        let winner = Winner {
            value: value.clone(),
            key,
        };
        match target {
            DeclTarget::Prop(id) => self.props[id.index()].push(winner),
            DeclTarget::Custom(name) => self.custom.entry(name.clone()).or_default().push(winner),
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

/// Context carried down while flattening a stylesheet.
#[derive(Debug, Clone, Default)]
struct FlattenCtx {
    conditions: Vec<Vec<MediaQuery>>,
    containers: Vec<ContainerCondition>,
    layer: Option<usize>,
    layer_path: Option<String>,
}

/// Resolves computed styles for trees that implement [`StyleTree`].
///
/// The resolver keeps the last result so that [`StyleResolver::restyle`] can
/// recompute only the parts of the tree that changed. See
/// [`crate::style::invalidation`].
#[derive(Default)]
pub struct StyleResolver {
    pub(crate) config: ResolverConfig,
    pub(crate) sheets: Vec<LoadedSheet>,
    pub(crate) next_sheet: u32,
    pub(crate) cache: Option<Cache>,
    pub(crate) dirty: BTreeSet<NodeId>,
    pub(crate) full_dirty: bool,
    pub(crate) last_restyled: usize,
    /// Cascade layers in order of first declaration (spec 13.4).
    pub(crate) layers: BTreeMap<String, usize>,
    pub(crate) layer_count: usize,
    pub(crate) anon_layers: usize,
    /// `@property` registrations (later ones replace earlier ones).
    pub(crate) registered: BTreeMap<String, RegisteredProperty>,
    keyframes: BTreeMap<String, KeyframesRule>,
    font_faces: Vec<FontFaceRule>,
    pub(crate) containers: Option<Box<dyn ContainerSizeProvider>>,
}

impl std::fmt::Debug for StyleResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StyleResolver")
            .field("sheets", &self.sheets.len())
            .field("layers", &self.layer_count)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl StyleResolver {
    /// Creates an empty resolver.
    pub fn new(config: ResolverConfig) -> StyleResolver {
        StyleResolver {
            config,
            ..StyleResolver::default()
        }
    }

    /// Sets the container size provider used by `@container` (spec 16.5). Sizes
    /// are read at the next [`StyleResolver::resolve`]. Restyle recomputes all
    /// styles while a provider is set.
    pub fn set_container_provider(&mut self, provider: Box<dyn ContainerSizeProvider>) {
        self.containers = Some(provider);
        self.full_dirty = true;
    }

    /// The `@keyframes` rule with this name, if one was parsed. Interpolation is
    /// not implemented yet, so this gives the rule as written.
    pub fn keyframes(&self, name: &str) -> Option<&KeyframesRule> {
        self.keyframes.get(name)
    }

    /// The `@font-face` rules that were parsed, in source order.
    pub fn font_faces(&self) -> &[FontFaceRule] {
        &self.font_faces
    }

    /// Adds a parsed stylesheet with the given origin (spec 8.2).
    ///
    /// Declarations that name unknown properties or have invalid values are
    /// ignored here, since the parser already reported them. Rules inside
    /// `@media`, `@container`, and `@layer` keep their conditions and layer.
    pub fn add_stylesheet(&mut self, sheet: Stylesheet, origin: Origin) -> SheetId {
        let id = SheetId(self.next_sheet);
        self.next_sheet += 1;
        let mut rules = Vec::new();
        self.flatten(sheet.rules, &FlattenCtx::default(), &mut rules);
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

    /// Whether the `@container` conditions of a rule hold for `node`.
    fn containers_match<T: StyleTree>(
        &self,
        tree: &T,
        node: NodeId,
        conditions: &[ContainerCondition],
    ) -> bool {
        if conditions.is_empty() {
            return true;
        }
        let Some(provider) = self.containers.as_deref() else {
            return false;
        };
        let root_fs = self.cache.as_ref().map_or(16.0, |c| c.env.root_font_size);
        conditions.iter().all(|cond| {
            let mut current = tree.parent(node);
            while let Some(ancestor) = current {
                if let Some(info) = provider.container(ancestor) {
                    let named = cond
                        .name
                        .as_ref()
                        .map_or(true, |n| info.name.as_deref() == Some(n.as_str()));
                    if named {
                        return features_match(&cond.features, &info, root_fs);
                    }
                }
                current = tree.parent(ancestor);
            }
            false
        })
    }

    /// Collects every candidate declaration for each target of `node` (spec 13).
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
                if !active[sheet_pos][rule_idx]
                    || !self.containers_match(tree, node, &rule.containers)
                {
                    continue;
                }
                for (sel_idx, selector) in rule.selectors.selectors.iter().enumerate() {
                    let uses_visited = rule.visited.get(sel_idx).copied().unwrap_or(false);
                    if uses_visited && self.config.visited == VisitedPolicy::Disabled {
                        continue;
                    }
                    let mut budget = max_steps;
                    if !matches_complex(tree, node, selector, &mut budget) {
                        continue;
                    }
                    let cas = out
                        .entry(selector.pseudo_element.clone())
                        .or_insert_with(Cascaded::new);
                    for (decl_idx, decl) in rule.decls.iter().enumerate() {
                        if uses_visited && visited_restricted(&decl.target) {
                            continue;
                        }
                        let key = CascadeKey {
                            origin: sheet.origin,
                            important: decl.important,
                            layer: layer_key(rule.layer, decl.important),
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
                        layer: layer_key(None, compiled.important),
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

    /// Returns the layer index for `name`, registering it on first use. The
    /// order is the order of first declaration (spec 13.4).
    fn layer_index(&mut self, name: &str) -> usize {
        if let Some(&index) = self.layers.get(name) {
            return index;
        }
        let index = self.layer_count;
        self.layer_count += 1;
        self.layers.insert(name.to_string(), index);
        index
    }

    /// Flattens a rule list into `out`, recording the nesting conditions on each
    /// style rule. Registrations (`@layer`, `@property`) take effect in document
    /// order.
    fn flatten(&mut self, rules: Vec<Rule>, ctx: &FlattenCtx, out: &mut Vec<CompiledRule>) {
        for rule in rules {
            match rule {
                Rule::Style(style) => {
                    let decls = style
                        .declarations
                        .declarations
                        .iter()
                        .flat_map(compile_declaration)
                        .collect();
                    let visited = style
                        .selectors
                        .selectors
                        .iter()
                        .map(selector_has_visited)
                        .collect();
                    let structural = style.selectors.selectors.iter().any(selector_is_structural);
                    out.push(CompiledRule {
                        selectors: style.selectors,
                        decls,
                        conditions: ctx.conditions.clone(),
                        containers: ctx.containers.clone(),
                        layer: ctx.layer,
                        visited,
                        structural,
                    });
                }
                Rule::Media(media_rule) => {
                    let mut child = ctx.clone();
                    child.conditions.push(media_rule.queries);
                    self.flatten(media_rule.rules, &child, out);
                }
                Rule::Supports(supports) => {
                    if supports.supported {
                        self.flatten(supports.rules, ctx, out);
                    }
                }
                Rule::Container(container) => {
                    let mut child = ctx.clone();
                    child.containers.push(ContainerCondition {
                        name: container.name,
                        features: container.features,
                    });
                    self.flatten(container.rules, &child, out);
                }
                Rule::Layer(layer) => match layer.rules {
                    None => {
                        for name in layer.names {
                            let full = join_layer(ctx.layer_path.as_deref(), &name);
                            self.layer_index(&full);
                        }
                    }
                    Some(inner) => {
                        let name = layer.names.into_iter().next().unwrap_or_default();
                        let full = if name.is_empty() {
                            self.anon_layers += 1;
                            format!("\u{1}anonymous-{}", self.anon_layers)
                        } else {
                            join_layer(ctx.layer_path.as_deref(), &name)
                        };
                        let index = self.layer_index(&full);
                        let mut child = ctx.clone();
                        child.layer = Some(index);
                        child.layer_path = Some(full);
                        self.flatten(inner, &child, out);
                    }
                },
                Rule::Property(property) => {
                    self.registered.insert(
                        property.name,
                        RegisteredProperty {
                            syntax: property.syntax,
                            inherits: property.inherits,
                            initial: property.initial,
                        },
                    );
                }
                Rule::Keyframes(keyframes) => {
                    self.keyframes.insert(keyframes.name.clone(), keyframes);
                }
                Rule::FontFace(font_face) => self.font_faces.push(font_face),
                Rule::Unknown(_) => {}
            }
        }
    }
}

/// The name of a layer nested under `parent`.
fn join_layer(parent: Option<&str>, name: &str) -> String {
    match parent {
        Some(p) => format!("{p}.{name}"),
        None => name.to_string(),
    }
}

/// The layer rank for a declaration in `layer` (see [`layer_rank`]).
fn layer_key(layer: Option<usize>, important: bool) -> u32 {
    match layer {
        Some(index) => layer_rank(index, important),
        None if important => UNLAYERED_IMPORTANT,
        None => UNLAYERED_NORMAL,
    }
}

/// `:visited` restricts a selector to color properties (spec 20.2).
fn visited_restricted(target: &DeclTarget) -> bool {
    match target {
        DeclTarget::Prop(id) => {
            let name = spec(*id).name;
            !(name == "color"
                || name == "background-color"
                || (name.starts_with("border-") && name.ends_with("-color")))
        }
        DeclTarget::Custom(_) => true,
    }
}

/// True if a selector contains `:visited`, including inside `:is()`, `:not()`,
/// and `:where()`.
fn selector_has_visited(selector: &ComplexSelector) -> bool {
    selector.compounds.iter().any(|compound| {
        compound.simple.iter().any(|simple| match simple {
            SimpleSelector::PseudoClass(PseudoClass::Visited) => true,
            SimpleSelector::Is(list) | SimpleSelector::Not(list) | SimpleSelector::Where(list) => {
                list.selectors.iter().any(selector_has_visited)
            }
            _ => false,
        })
    })
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
                    &resolver.registered,
                );

                map.clear_pseudo(frame.node);
                for (target, cas) in targets {
                    if let Some(pseudo) = target {
                        let pstyle = compute_style(
                            &cas,
                            Some(&style),
                            root_fs,
                            env,
                            max_var_depth,
                            &resolver.registered,
                        );
                        if is_generated(&pseudo, &pstyle) {
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

/// A pseudo-element is generated when its `content` is not `none` or `normal`,
/// or always for the pseudo-elements that have no `content` (`::selection`,
/// `::backdrop`, `::file-selector-button`, CSS Pseudo-Elements 4).
fn is_generated(pseudo: &PseudoElement, style: &ComputedStyle) -> bool {
    match pseudo {
        PseudoElement::Selection | PseudoElement::Backdrop | PseudoElement::FileSelectorButton => {
            true
        }
        _ => !matches!(
            style.raw(ids::CONTENT),
            Value::Keyword("none") | Value::Keyword("normal")
        ),
    }
}

/// The winning candidate for a custom property (revert is treated as unset).
fn custom_declared(candidates: &[Winner]) -> Option<SpecifiedValue> {
    let winner = candidates.iter().max_by(|a, b| a.key.cmp(&b.key))?;
    Some(match &winner.value {
        SpecifiedValue::CssWide(CssWide::Revert | CssWide::RevertLayer) => {
            SpecifiedValue::CssWide(CssWide::Unset)
        }
        other => other.clone(),
    })
}

/// Computes the style of one target from its candidate declarations.
fn compute_style(
    cas: &Cascaded,
    parent: Option<&ComputedStyle>,
    root_fs: f64,
    env: &Environment,
    max_var_depth: usize,
    registered: &BTreeMap<String, RegisteredProperty>,
) -> ComputedStyle {
    let declared: BTreeMap<String, SpecifiedValue> = cas
        .custom
        .iter()
        .filter_map(|(name, cands)| custom_declared(cands).map(|v| (name.clone(), v)))
        .collect();
    let customs = resolve_customs(
        &declared,
        parent.map(|p| p.customs()),
        registered,
        max_var_depth,
    );

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
        dark: env.media.prefers_dark,
    };

    let mut values: Vec<Value> = PROPERTIES.iter().map(|p| p.initial.to_value()).collect();
    for idx in compute_order() {
        let id = PropertyId(idx as u16);
        let specified = specified_value(idx, &cas.props[idx], parent, &customs, max_var_depth);
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

/// Picks the value that applies to one property from its candidates (spec 13.3,
/// 13.8). `revert` rolls back to the candidates of lower origins, and
/// `revert-layer` to the candidates of the same origin in lower layers. When no
/// candidate is left, the result is unset. `None` also means unset.
fn resolve_winner(
    candidates: &[Winner],
    idx: usize,
    customs: &BTreeMap<String, CustomPropertyValue>,
    max_var_depth: usize,
) -> Option<SpecifiedValue> {
    let mut pool: Vec<&Winner> = candidates.iter().collect();
    loop {
        let winner = pool.iter().copied().max_by(|a, b| a.key.cmp(&b.key))?;
        match &winner.value {
            SpecifiedValue::CssWide(CssWide::Revert) => {
                let origin = winner.key.origin.index();
                pool.retain(|c| c.key.origin.index() < origin);
            }
            SpecifiedValue::CssWide(CssWide::RevertLayer) => {
                let (origin, layer) = (winner.key.origin, winner.key.layer);
                pool.retain(|c| c.key.origin == origin && c.key.layer < layer);
            }
            SpecifiedValue::Unparsed { tokens, shorthand } => {
                return resolve_unparsed(idx, tokens, *shorthand, customs, max_var_depth);
            }
            other => return Some(other.clone()),
        }
    }
}

/// Resolves the winning declaration for property `idx` to a value (spec 13.8).
fn specified_value(
    idx: usize,
    candidates: &[Winner],
    parent: Option<&ComputedStyle>,
    customs: &BTreeMap<String, CustomPropertyValue>,
    max_var_depth: usize,
) -> Value {
    let info = &PROPERTIES[idx];
    let concrete = resolve_winner(candidates, idx, customs, max_var_depth)
        .unwrap_or(SpecifiedValue::CssWide(CssWide::Unset));
    let initial = || info.initial.to_value();
    let inherit = || match parent {
        Some(p) => p.raw(PropertyId(idx as u16)).clone(),
        None => initial(),
    };
    match concrete {
        SpecifiedValue::Value(v) => v,
        SpecifiedValue::CssWide(CssWide::Initial) => initial(),
        SpecifiedValue::CssWide(CssWide::Inherit) => inherit(),
        SpecifiedValue::CssWide(_) => {
            if info.inherited {
                inherit()
            } else {
                initial()
            }
        }
        // Resolution never returns an unparsed value.
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
/// came from if any. `all` sets every property except `direction` and
/// `unicode-bidi` (CSS Cascade 4), and accepts only CSS-wide keywords.
/// Unknown properties and invalid values yield nothing.
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

    if decl.property == "all" {
        let Some(wide) = css_wide_only(&decl.value) else {
            return Vec::new();
        };
        let excluded = [lookup("direction"), lookup("unicode-bidi")];
        return (0..PROPERTIES.len())
            .map(|i| PropertyId(i as u16))
            .filter(|id| !excluded.contains(&Some(*id)))
            .map(|id| CompiledDecl {
                target: DeclTarget::Prop(id),
                value: SpecifiedValue::CssWide(wide),
                important,
            })
            .collect();
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
                "revert" => Some(CssWide::Revert),
                "revert-layer" => Some(CssWide::RevertLayer),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
