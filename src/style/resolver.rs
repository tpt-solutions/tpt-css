// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Style resolution (spec 8.2, 16.1).
//!
//! M0 pipeline per element: collect matching rules and inline declarations,
//! cascade per property, then compute in dependency order (font-size, color,
//! font-weight, then everything else), applying inheritance and CSS-wide
//! keywords. Pseudo-elements, var(), shorthands, and invalidation are later
//! milestones.
//!
//! Tree traversal uses an explicit stack, so deep trees cannot overflow the
//! call stack.

use crate::ast::{Declaration, Rule, SelectorList, Stylesheet};
use crate::cascade::order::DeclarationOrder;
use crate::cascade::origin::Origin;
use crate::cascade::CascadeKey;
use crate::env::Environment;
use crate::limits::ParseLimits;
use crate::properties::registry::{spec, PROPERTIES};
use crate::properties::{ids, lookup, parse_property_value, CssWide, PropertyId, SpecifiedValue};
use crate::selectors::matching::match_selector_list;
use crate::selectors::specificity::Specificity;
use crate::style::computed::{compute_value, ComputeContext, ComputedStyle, StyleMap};
use crate::tree::{NodeId, NodeKind, StyleTree};
use crate::values::{Color, Value};

/// Identifies a stylesheet added to a resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SheetId(pub u32);

/// Resolver configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResolverConfig {
    /// Limits applied during selector matching (spec 20.3).
    pub limits: ParseLimits,
}

/// A declaration after property lookup and value parsing.
#[derive(Debug, Clone)]
struct CompiledDecl {
    prop: PropertyId,
    value: SpecifiedValue,
    important: bool,
}

#[derive(Debug, Clone)]
struct CompiledRule {
    selectors: SelectorList,
    decls: Vec<CompiledDecl>,
}

#[derive(Debug, Clone)]
struct LoadedSheet {
    id: SheetId,
    origin: Origin,
    rules: Vec<CompiledRule>,
}

/// The winning declaration for one property on one element.
#[derive(Debug, Clone)]
struct Winner {
    value: SpecifiedValue,
    key: CascadeKey,
}

/// Work item for the iterative traversal.
struct Frame {
    node: NodeId,
    /// Computed style of the nearest element ancestor, if any.
    parent: Option<ComputedStyle>,
    /// Root font size for `rem`. `None` means this element is the root.
    rem_base: Option<f64>,
}

/// Resolves computed styles for trees that implement [`StyleTree`].
#[derive(Debug, Default)]
pub struct StyleResolver {
    config: ResolverConfig,
    sheets: Vec<LoadedSheet>,
    next_sheet: u32,
}

impl StyleResolver {
    /// Creates an empty resolver.
    pub fn new(config: ResolverConfig) -> StyleResolver {
        StyleResolver {
            config,
            sheets: Vec::new(),
            next_sheet: 0,
        }
    }

    /// Adds a parsed stylesheet with the given origin (spec 8.2).
    ///
    /// Declarations that name unknown properties or have invalid values are
    /// ignored here, as the parser already reported them.
    pub fn add_stylesheet(&mut self, sheet: Stylesheet, origin: Origin) -> SheetId {
        let id = SheetId(self.next_sheet);
        self.next_sheet += 1;
        let rules = sheet
            .rules
            .into_iter()
            .filter_map(|rule| match rule {
                Rule::Style(style) => Some(CompiledRule {
                    selectors: style.selectors,
                    decls: style
                        .declarations
                        .declarations
                        .iter()
                        .filter_map(compile_declaration)
                        .collect(),
                }),
                Rule::Unknown(_) => None,
            })
            .collect();
        self.sheets.push(LoadedSheet { id, origin, rules });
        id
    }

    /// Removes a stylesheet. Unknown ids are ignored.
    pub fn remove_stylesheet(&mut self, id: SheetId) {
        self.sheets.retain(|s| s.id != id);
    }

    /// Computes styles for every element in `tree` (spec 8.2).
    pub fn resolve<T: StyleTree>(&mut self, tree: &T, env: &Environment) -> StyleMap {
        let mut map = StyleMap::default();
        let mut stack = vec![Frame {
            node: tree.root(),
            parent: None,
            rem_base: None,
        }];

        while let Some(frame) = stack.pop() {
            match tree.kind(frame.node) {
                NodeKind::Element => {
                    let root_fs = frame.rem_base.unwrap_or(env.root_font_size);
                    let style =
                        self.compute_element(tree, frame.node, frame.parent.as_ref(), root_fs, env);
                    let child_rem = Some(frame.rem_base.unwrap_or(style_font_size(&style)));
                    for &child in tree.children(frame.node).iter().rev() {
                        stack.push(Frame {
                            node: child,
                            parent: Some(style.clone()),
                            rem_base: child_rem,
                        });
                    }
                    map.insert(frame.node, style);
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
        map
    }

    fn compute_element<T: StyleTree>(
        &self,
        tree: &T,
        node: NodeId,
        parent: Option<&ComputedStyle>,
        root_font_size: f64,
        env: &Environment,
    ) -> ComputedStyle {
        let winners = self.cascade(tree, node);

        let parent_font_size = parent.map_or(env.root_font_size, style_font_size);
        let parent_color = parent.map_or(Color::BLACK, |p| p.color());
        let parent_weight = parent.map_or(400.0, |p| p.font_weight());
        let mut ctx = ComputeContext {
            parent_font_size,
            root_font_size,
            font_size: parent_font_size,
            parent_color,
            color: parent_color,
            parent_weight,
        };

        let mut values: Vec<Value> = PROPERTIES.iter().map(|p| p.initial.to_value()).collect();
        for idx in compute_order() {
            let id = PropertyId(idx as u16);
            let specified = specified_value(idx, winners[idx].as_ref().map(|w| &w.value), parent);
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

        // CSS Backgrounds 3, border-width computed value: border-width computes to 0 when the
        // style is none or hidden.
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

        ComputedStyle::from_values(values)
    }

    /// Collects the winning declaration for each property (spec 13).
    fn cascade<T: StyleTree>(&self, tree: &T, node: NodeId) -> Vec<Option<Winner>> {
        let mut winners: Vec<Option<Winner>> = vec![None; PROPERTIES.len()];
        let max_steps = self.config.limits.max_match_steps;

        for (sheet_pos, sheet) in self.sheets.iter().enumerate() {
            for (rule_idx, rule) in sheet.rules.iter().enumerate() {
                let Some(specificity) = match_selector_list(tree, node, &rule.selectors, max_steps)
                else {
                    continue;
                };
                for (decl_idx, decl) in rule.decls.iter().enumerate() {
                    let key = CascadeKey {
                        origin: sheet.origin,
                        important: decl.important,
                        inline: false,
                        specificity,
                        order: DeclarationOrder {
                            sheet: sheet_pos as u32,
                            rule: rule_idx as u32,
                            declaration: decl_idx as u32,
                        },
                    };
                    offer(&mut winners, decl.prop, &decl.value, key);
                }
            }
        }

        if let Some(inline) = tree.inline_style(node) {
            for (i, decl) in inline.iter().enumerate() {
                if let Some(compiled) = compile_declaration(decl) {
                    let key = CascadeKey {
                        origin: Origin::Author,
                        important: compiled.important,
                        inline: true,
                        specificity: Specificity::default(),
                        order: DeclarationOrder::inline(i as u32),
                    };
                    offer(&mut winners, compiled.prop, &compiled.value, key);
                }
            }
        }
        winners
    }
}

/// Order in which properties are computed: font-size first (its `em` depends
/// on the parent), then color (for `currentcolor`), then font-weight (for the
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

fn style_font_size(style: &ComputedStyle) -> f64 {
    style.font_size()
}

fn offer(
    winners: &mut [Option<Winner>],
    prop: PropertyId,
    value: &SpecifiedValue,
    key: CascadeKey,
) {
    let slot = &mut winners[prop.index()];
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

/// Resolves a winning declaration to a specified value (spec 13.8).
fn specified_value(
    idx: usize,
    winner: Option<&SpecifiedValue>,
    parent: Option<&ComputedStyle>,
) -> Value {
    let info = &PROPERTIES[idx];
    let initial = || info.initial.to_value();
    let inherit = || match parent {
        Some(p) => p.raw(PropertyId(idx as u16)).clone(),
        None => initial(),
    };
    match winner {
        None => {
            if info.inherited {
                inherit()
            } else {
                initial()
            }
        }
        Some(SpecifiedValue::Value(v)) => v.clone(),
        Some(SpecifiedValue::CssWide(CssWide::Initial)) => initial(),
        Some(SpecifiedValue::CssWide(CssWide::Inherit)) => inherit(),
        Some(SpecifiedValue::CssWide(CssWide::Unset)) => {
            if info.inherited {
                inherit()
            } else {
                initial()
            }
        }
    }
}

/// Looks up and parses a declaration. Returns `None` for unknown properties,
/// invalid values, and custom properties (M1).
fn compile_declaration(decl: &Declaration) -> Option<CompiledDecl> {
    if decl.property.starts_with("--") {
        return None;
    }
    let prop = lookup(&decl.property)?;
    let value = parse_property_value(spec(prop), &decl.value).ok()?;
    Some(CompiledDecl {
        prop,
        value,
        important: decl.important,
    })
}
