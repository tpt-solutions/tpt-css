# Spec trace

This file maps [spec.txt](spec.txt) sections to modules, algorithms, and tests,
as spec 22.5 asks. The external conformance column is empty until the external
suites run in CI from a separate checkout (spec 22.2). Nothing here has been
run against them yet.

Status key: **done** = implemented and tested. **partial** = implemented with
the gap named in the last column. **todo** = not started.

Design decisions and deviations are in [DESIGN.md](DESIGN.md).

## Syntax (spec 10)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 10.1, CSS Syntax 3 (consume a token) | consume a token | `syntax/tokenizer.rs` `consume_token` | `tests/syntax` | not run | done | Comment preservation is not implemented. |
| 10.1, CSS Syntax 3 (escapes) | consume an escaped code point | `syntax/tokenizer.rs` `consume_escape` | `tests/syntax` (`hex_escapes_and_replacement_characters`) | not run | done | |
| 10.1, CSS Syntax 3 (url tokens) | consume a url token, bad url | `syntax/tokenizer.rs` `consume_url` | `tests/syntax` (`unquoted_url_and_bad_url`) | not run | done | |
| 10.1, CSS Syntax 3 (consume a numeric token) | consume a number | `syntax/tokenizer.rs` `consume_number` | `tests/syntax`, `tests/golden` | not run | done | Values are `f64` (spec 25.1). |
| 10.2, CSS Syntax 3 (component values) | consume a component value | `syntax/component.rs` `build_component_values` | `tests/syntax` | not run | done | The build is iterative. Nesting beyond the limit fails the whole sheet. |
| 10.3 | source spans | `syntax/span.rs` | `tests/syntax` (`crlf_is_one_whitespace_token...`) | n/a | done | Spans are byte offsets, not line and column. |

## Parse (spec 8.1, 11, 16.3)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 8.1 | parse a stylesheet | `parse/stylesheet.rs` | `tests/parse` | not run | done | Quirks mode is not modeled. |
| CSS Syntax 3 (rule lists) | consume a list of rules, qualified rule | `parse/rules.rs` | `tests/parse` | not run | partial | Nested style rules (CSS nesting) are M2. |
| CSS Syntax 3 (declarations) | consume a declaration | `parse/declarations.rs` | `tests/parse` (`an_invalid_declaration_does_not_abort...`) | not run | done | Custom properties are kept and resolved (M1). |
| 11.2 | at-rules: `@media` | `parse/rules.rs`, `parse/media.rs` | `tests/cascade/media.rs`, `tests/parse` | not run | partial | `@media` is parsed and evaluated. Other at-rules are preserved as unknown (M2). |
| 16.3 | media query prelude | `parse/media.rs` | `tests/cascade/media.rs` (`a_malformed_media_prelude...`) | not run | done | Unknown features parse as `Unknown` and never match. |
| 19.3 | diagnostics | `parse/diagnostics.rs` | `tests/parse` | n/a | done | |
| 19.2 | limits | `limits.rs` | `tests/parse` (`selector_complexity...`, `declaration_limit...`, `deep_nesting...`), `tests/syntax` | n/a | partial | `max_calc_depth` is a fixed constant (`values::calc::DEFAULT_CALC_DEPTH`), not configurable. A stylesheet count limit is not enforced. |

## Selectors (spec 12)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 12.2 Tier 0 | `*`, type, `#id`, `.class`, `[attr]`, `[attr=value]`, descendant, child, `:root` | `parse/selectors.rs`, `selectors/matching.rs` | `tests/parse`, `tests/selectors` | not run | done | Attribute values are case-sensitive in the core. |
| 12.2 Tier 1 | attribute operators `~=, \|=, ^=, $=, *=` | `parse/selectors.rs`, `selectors/matching.rs` | `tests/selectors/tier1.rs` | not run | done | Attribute flags (`i`, `s`) and namespaces are not supported. |
| 12.2 Tier 1 | `:first-child`, `:last-child`, `:only-child`, `:nth-*`, `:first-of-type`, `:last-of-type`, `:only-of-type`, `:nth-of-type`, `:nth-last-of-type` | `selectors/matching.rs` `nth_matches` | `tests/selectors/tier1.rs` | not run | done | Sibling lookups are O(siblings) per node (M3 index). |
| 12.2 Tier 1 | `:nth-child(An+B of S)` | `selectors/matching.rs` | `tests/selectors/tier1.rs` (`nth_child_of_a_selector...`) | not run | done | |
| 12.2 Tier 1 | `:not()`, `:is()`, `:where()` | `selectors/matching.rs` | `tests/selectors/tier1.rs`, `tests/selectors/main.rs` | not run | done | `:has()` is M2. |
| 12.2 Tier 1 | `:empty` | `selectors/matching.rs` `is_empty` | `tests/selectors/tier1.rs` | not run | done | |
| 12.2 Tier 1 | `:lang()`, `:dir()` | `selectors/matching.rs` | `tests/selectors/tier1.rs` | not run | done | Language and direction come from the tree adapter. |
| 12.2 Tier 1 | form states: `:enabled`, `:disabled`, `:checked`, `:required`, `:optional`, `:read-only`, `:read-write`, `:placeholder-shown` | `selectors/matching.rs` | `tests/selectors/tier1.rs` | not run | done | The adapter sets the state bits. |
| 12.2 state | `:hover`, `:active`, `:focus` | `selectors/matching.rs` | `tests/selectors` | not run | done | |
| 12.1 | sibling combinators `+`, `~` | `selectors/matching.rs` | `tests/selectors/tier1.rs` | not run | done | |
| 12.3 | specificity | `selectors/specificity.rs` | `tests/parse`, `tests/selectors/tier1.rs` | not run | done | |
| 12.4 | right-to-left matching | `selectors/matching.rs` `matches_complex` | `tests/selectors` | not run | partial | No candidate filtering by id, class, or type, and no bloom filter (M3). |
| 12.4 | step budget (spec 20.3) | `selectors/matching.rs` | `tests/selectors` (`a_zero_step_budget_fails_closed`) | n/a | done | |
| 12.5 | pseudo-elements `::before`, `::after`, `::marker`, `::placeholder` | `parse/selectors.rs`, `style/resolver.rs` | `tests/selectors/tier1.rs`, `tests/style/m1.rs` | not run | done | Other pseudo-elements are M2 and M4. |

## Cascade (spec 13)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 13.2, Cascade 5 (origins) | origin and importance order | `cascade/origin.rs` `precedence` | `tests/cascade` | not run | done | Animation and transition ranks have no importance. |
| 13.3 | cascade key | `cascade/mod.rs` `CascadeKey` | `tests/cascade` | not run | partial | Layers are in the key. Scope proximity (Cascade 6) is not implemented. |
| 13.3 | declaration order | `cascade/order.rs` | `tests/cascade` (`later_declaration_wins...`) | not run | done | |
| 13.5 | inline style | `style/resolver.rs` `cascade` | `tests/cascade` (`inline_*`) | not run | done | |
| 13.6 | shorthand expansion | `properties/shorthand.rs` | `tests/style/m1.rs` (shorthand tests) | not run | partial | `margin`, `padding`, `border` (and sides), `border-width/style/color`, `background` (color). `font`, `list-style`, `text-decoration` not implemented. |
| 13.7 | custom properties, cascade, substitution, cycles | `style/custom.rs`, `style/resolver.rs` | `tests/style/m1.rs` (custom property tests) | not run | done | `@property` registration is M2. |
| 13.8 | `initial`, `inherit`, `unset` | `style/resolver.rs` `specified_value` | `tests/style` | not run | done | `revert` and `revert-layer` are M2. |
| 13.4 | cascade layers and layer ranks | `cascade/mod.rs` `layer_rank`, `style/resolver.rs` `flatten` | `tests/cascade/layers.rs` | not run | done | Layer order is the order of first declaration. |

## Values and properties (spec 14, 15)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 14.3 | length units: px, em, rem, vw, vh, vmin, vmax, cm, mm, Q, in, pt, pc, and percent | `values/length.rs`, `style/computed.rs` `to_px` | `tests/style/m1.rs`, `tests/style` | not run | done | Viewport units use the environment viewport. Viewport units in media queries are not valid and fall back. |
| 14.3 | other units (`ex`, `ch`, `cap`, `ic`, `lh`, `rlh`, `fr`) | — | — | — | todo | Font-relative units beyond `em` and `rem` are not implemented. `fr` belongs to grid, which is not started. |
| 14.4 | `calc()`, `min()`, `max()`, `clamp()`, and the numeric functions (`round`, `mod`, `rem`, `abs`, `sign`, `hypot`, `sqrt`, `pow`, `log`, `exp`, trig, `atan2`, `progress`) | `values/calc.rs` | `tests/style/m1.rs`, `tests/style/m3.rs` | not run | done | Type-checked at parse time. Trig takes unitless radians, since angle units are not parsed as values. The depth limit is a fixed constant. |
| 14.5 | colors: named (all 148), `transparent`, hex (3, 4, 6, and 8 digits), `rgb()`, `hsl()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color()` (srgb, srgb-linear), `color-mix()` (srgb) | `values/color.rs` | `tests/style`, `tests/style/m3.rs` | not run | partial | Every space converts to sRGB at parse time. Relative color syntax works for rgb, hsl, hwb, lab, lch, oklab, and oklch. Relative `color()` works for srgb and srgb-linear. `color-mix()` works in eight spaces. `xyz` and wide-gamut spaces are not implemented. |
| 14.5 | modern color spaces (`lab`, `lch`, `oklab`, `oklch`) | `values/color.rs` | `tests/style/m3.rs` | not run | done | Converted to sRGB at parse time; out-of-gamut values are clamped. |
| 14.2 | deterministic serialization | `values/serialize.rs`, `values/calc.rs` `serialize` | `tests/style` | not run | partial | Canonical form only. Spec-exact CSSOM serialization is M3. |
| 14.6 | `var()` | `style/custom.rs` `substitute` | `tests/style/m1.rs` | not run | done | `env()` and `attr()` are not implemented. |
| 15.1 | property registry | `properties/registry.rs` | `tests/style` | n/a | done | `animatable` and `layout_effect` fields are M2 and M3. |
| 15.2 | M0 property set (37) and M1 additions (13) | `properties/registry.rs` | `tests/style` (`registry_has_...`) | n/a | done | `font`, `list-style`, `text-decoration` are not in the set yet. |
| 15.3 | shorthand metadata and expansion | `properties/shorthand.rs` | `tests/style/m1.rs` | n/a | partial | See 13.6. |
| 15.4 | aliases | — | — | — | todo | Vendor-prefixed aliases are not registered. |
| 14.1 | grammar parsing | `properties/parse.rs` | `tests/style` | n/a | done | |

## Computed style and resolution (spec 8.2, 8.3, 16, 17)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 16.1 steps 1–4, 9–13 | collect, cascade, CSS-wide, compute, inherit, store | `style/resolver.rs` | `tests/style`, `tests/cascade`, `tests/integration` | not run | done | |
| 16.1 steps 5–8 | custom properties and `var()` | `style/resolver.rs`, `style/custom.rs` | `tests/style/m1.rs` | not run | done | |
| 16.1 step 12 | pseudo-element styles | `style/resolver.rs` `style_tree` | `tests/style/m1.rs`, `tests/integration/web_like.rs` | not run | done | |
| 16.1 font-size first | `compute_order` | `style/resolver.rs` | `tests/style` (`em_in_font_size...`) | n/a | done | |
| 8.3 | `StyleMap` `get`, `get_pseudo`, `custom_property` | `style/computed.rs` | `tests/integration`, `tests/style/m1.rs` | n/a | done | |
| 8.3 | typed accessors | `style/computed.rs` | `tests/style` | n/a | partial | `font_size()` returns px as `f64`. `display()` returns the keyword string. |
| 4.4 | keep layout-dependent values unresolved | `style/computed.rs` `compute_value` | `tests/style`, `tests/style/m1.rs` | n/a | done | |
| 16.2 | environment | `env/mod.rs` | `tests/integration`, `tests/cascade/media.rs` | n/a | partial | Viewport, root font size, and media features. Containers, fonts, URLs, `env()` are later. |
| 16.3 | media query evaluation | `env/media.rs` | `tests/cascade/media.rs` | not run | done | Media features: width, height, aspect-ratio, orientation, prefers-color-scheme, prefers-reduced-motion, hover, pointer. `resolution`, `update`, and other features are unknown and never match. |
| 17 | invalidation (foundation) | `style/invalidation.rs` | `tests/integration/invalidation.rs` | n/a | partial | Attribute, state, and inline changes restyle the subtree. Sibling-sensitive rules restyle the parent subtree. Sheet and tree changes restyle everything. No dependency index (M3). |
| 9 | tree contract | `tree/` | `tests/integration`, all suites via `tests/common` | n/a | done | Scope and shadow fields are defined but unused until M2 and M4. `text` (for `:empty`) has a default. |
| 4.3 | no HTML names in core | `src/` | case-sensitive search of `src/` | n/a | done | |
| 20.1 | no network in core | `src/` | by construction (no I/O) | n/a | done | |
| 19.1 | no panics on hostile input | all | `tests/integration` (`hostile_input_never_panics`) | n/a | partial | Randomized test, not coverage-guided fuzzing. Fuzz targets are not written. |
| 21.3 | determinism | `style/computed.rs` (`BTreeMap`), `style/resolver.rs` | `tests/style` (`styling_the_same_tree_twice...`) | n/a | done | |

## Milestones

| Milestone | Status |
| --- | --- |
| M0 | Done, with the partial items above. Merged to `master`. |
| M1 | Largely done on branch `m1-practical-css`. Open items are in TODO.md section 3 (shorthands `font`, `list-style`, `text-decoration`; full named colors; modern color spaces; invalidation index; an AppFront document fixture). |
| M2 | Largely done on branch `m2-modern-cascade`. Open items are in TODO.md section 4 (`@scope` and scope proximity, `:has()`, `color()`, container units, keyframe interpolation). |
| M3 | Partly done. CSSOM (feature `cssom`), devtools inspection, the `getComputedStyle` contract, and the equality-cutoff restyle are in. Open: dependency graph, style sharing, custom property dependency graph. See TODO.md section 5. |
| M4 | Partly done. Numeric math functions, the full color set and modern color spaces, and keyframe interpolation are in. The long-tail selectors, properties, and shadow-tree items are open. See TODO.md section 6. |

## M2 additions

| Spec | Algorithm | Module | Tests | Status | Known gaps |
| --- | --- | --- | --- | --- | --- |
| 13.4 | cascade layers, layer ranks | `cascade/mod.rs` `layer_rank`, `style/resolver.rs` `flatten` | `tests/cascade/layers.rs` | done | Layer order is the order of first declaration. |
| 13.8 | `revert`, `revert-layer` | `style/resolver.rs` `resolve_winner` | `tests/cascade/layers.rs` | done | `revert` in custom properties is unset. |
| 13.8 | `all` | `style/resolver.rs` `compile_declaration` | `tests/style/m2.rs` | done | |
| 16.4 | `@supports` | `parse/supports.rs` | `tests/style/m2.rs`, `tests/parse/m2.rs` | done | `font-tech()`, `font-format()` report false. |
| 12.3 | CSS nesting, `&` | `parse/selectors.rs` `resolve_nesting`, `parse/rules.rs` | `tests/style/m2.rs`, `tests/parse/m2.rs` | done | Nested rules are resolved at parse time. |
| 16.5 | `@container`, `ContainerSizeProvider`, container units | `env/container.rs`, `style/resolver.rs` `containers_match` | `tests/integration/container.rs`, `tests/style/m3.rs` | not run | done | Only the nearest size container is used. |
| 15.1 | `@property` | `style/custom.rs` `syntax_matches`, `resolve_customs` | `tests/style/m2.rs` | done | Only the listed syntaxes are checked. |
| 13.2 (animation origin) | `Origin::Animation` in the cascade | `cascade/origin.rs` | `tests/style/m2.rs` | partial | No animation driver; keyframes are stored, not interpolated. |
| 14.5 | `light-dark()` | `properties/parse.rs`, `style/computed.rs` | `tests/style/m2.rs` | not run | done | Resolved at computed time from the color scheme. |
| 12.2 Tier 2 | state selectors, `:has()`, `@scope`, `:scope` | `selectors/matching.rs`, `style/resolver.rs` `scope_for` | `tests/style/m2.rs`, `tests/selectors/m3.rs` | not run | done | `:host`, `:host-context()`, `::slotted()`, and `::part()` are not implemented (M4). |
| 20.2 | visited policy | `style/resolver.rs` `VisitedPolicy` | `tests/style/m2.rs` | done | |

## M3 and M4 additions

| Spec | Algorithm | Module | Tests | Status | Known gaps |
| --- | --- | --- | --- | --- | --- |
| 8.2, 17 | restyle with an equality cutoff | `style/invalidation.rs`, `style/resolver.rs` `style_tree` | `tests/integration/invalidation.rs`, `tests/integration/property.rs`, `tests/integration/perf.rs` | partial | No dependency index; `AttributeChanged` carries no name; the restyle copies the map. |
| 18 | CSSOM mutation and serialization | `cssom/mod.rs` (feature `cssom`) | `tests/cssom/main.rs` | done | Canonical serialization, not byte-preserving (spec 25.8). |
| 18 | `getComputedStyle` contract | `style/computed.rs` `property_values` | `tests/style/m3.rs` | done | |
| 7 `diagnostics` | devtools inspection | `style/inspect.rs` | `tests/style/m3.rs` | done | Reports the highest candidate; does not follow `revert`. |
| 22.3 | seeded hostile-input tests | (tests only) | `tests/integration/property.rs` | partial | Not a coverage-guided fuzzer; no fuzz targets. |
| 21.1 | measured throughput and restyle | (tests only) | `tests/integration/perf.rs` (ignored) | done | Numbers in DESIGN.md section 8, from one machine. |
| 22.5 | spec trace consistency | (tests only) | `tests/integration/spec_trace.rs` | done | Checks that named files exist, not that the claims are right. |
| 14.4 | `progress()`, `round()` and the rest | `values/calc.rs` | `tests/style/m3.rs` | done | |
| 14.5 | `color-mix()` (srgb) | `values/color.rs` | `tests/style/m3.rs` | done | Other interpolation spaces are not implemented. |
| 13.2, CSS Animations 1 | keyframe interpolation | `style/keyframes.rs` | `tests/style/m3.rs` | partial | No timeline; the host supplies the progress. |
| 12.2 Tier 2 | `:has()`, `:scope` | `selectors/matching.rs` `Matcher` | `tests/selectors/m3.rs` | done | Invalidation is conservative (restyle everything on attribute change). |
| CSS Cascade 6 | `@scope`, scope proximity | `style/resolver.rs` `scope_for` and `scope_proximity`, `cascade/mod.rs` `CascadeKey`, `parse/rules.rs` `scope_rule` | `tests/selectors/m3.rs`, `tests/style/m3.rs` | partial | Proximity ranks after specificity, following the current Cascade 6 draft. `spec.txt` 13.3 puts it before specificity and needs correcting. Unscoped rules have infinite proximity. |
| Media Queries 5 | \`prefers-contrast\`, \`dynamic-range\`, \`update\`, \`resolution\` | \`ast/media.rs\`, \`parse/media.rs\`, \`env/media.rs\` | \`tests/style/m3.rs\` | partial | Other media features are still unknown and never match. \`resolution\` takes \`dppx\`, \`x\`, \`dpi\`, and \`dpcm\`. |
| Values 4 | calc nesting limit | \`parse/declarations.rs\` \`within_math_depth\`, \`style/resolver.rs\` \`resolve_unparsed\` | \`tests/style/m3.rs\` | done | Checked at declaration parse and after \`var()\` substitution. The value parser recurses up to 256, so the configured limit is the effective one. |
| 20.3 | stylesheet count limit | \`style/resolver.rs\` \`try_add_stylesheet\`, \`limits.rs\` | \`tests/style/m3.rs\` | done | \`add_stylesheet\` returns an id matching no sheet when the limit is reached. |
| CSS Color 5 | CSS Color 5 | relative color syntax (`rgb(from ...)` and the other six functions, and `color(from ...)`) | `values/color.rs` `parse_relative`, `parse_relative_color_function`, `color_from_channels` | `tests/style/m3.rs` | partial | Round trips through every space are tested. `color(from ...)` covers srgb and srgb-linear only. |
| 8.2, 17 | attribute-named invalidation | `style/invalidation.rs` `attribute_is_read`, `collect_attribute_names` | `tests/integration/invalidation.rs` | done | An attribute is relevant if a selector reads it, including inside `:not`, `:is`, `:where`, `:has`, and `@scope`. `style`, `lang`, and `dir` are always relevant. |
| 12.2 Tier 3 | state pseudo-classes `:current`, `:past`, `:future`, `:defined`, `:local-link`, `:playing`, `:paused`, `:fullscreen`, `:autofill` | `tree/state.rs`, `selectors/matching.rs`, `parse/selectors.rs` | `tests/style/m3.rs` | partial | The host sets the state bits. Nothing sets them by default. The functional forms are not implemented. |
| CSS Color 4 | `color-mix()` interpolation | `values/color.rs` `mix_in`, `MixSpace` | `tests/style/m3.rs` | partial | Eight spaces, premultiplied alpha, and the `shorter` and `longer` hue methods. Other spaces and hue methods are not implemented. |
| CSS Text Decoration 3 | `text-decoration` (line, style, color) and the `text-decoration-*` longhands | `properties/shorthand.rs` `text_decoration`, `properties/parse.rs` `parse_decoration_line` | `tests/style/m3.rs` | partial | No `text-decoration-thickness` or skip-ink. The line is stored as a combination keyword. |
| CSS Lists 3 | `list-style` and `list-style-type`, `-position`, `-image` | `properties/shorthand.rs` `list_style`, `properties/registry.rs` | `tests/style/m3.rs` | partial | Counter styles and image URLs are not supported. `none` is the only image. |
| CSS Fonts 4 | `font` shorthand | `properties/shorthand.rs` `font` | `tests/style/m3.rs` | partial | No `font-variant`, `font-stretch`, or system font keywords. |
| CSS Text 3 | `text-transform`, `text-indent`, `overflow-wrap`, `word-break`, `hyphens`, `letter-spacing`, `word-spacing` | `properties/registry.rs` | `tests/style/m3.rs` | done | `letter-spacing` and `word-spacing` take lengths only. |
| 15.4 | explicit property aliases | `properties/aliases.rs` | `tests/style/m3.rs` | done | One alias. Prefixes are not stripped. |
| 21.2 | selector candidates and sibling index | `style/rule_index.rs`, `selectors/matching.rs` `SiblingCache` | `tests/style/m3.rs`, `tests/integration/perf.rs` | partial | `:nth-child(of S)` still tests each sibling. `~` and `:nth-of-type()` use the index. |
| Shadow DOM 1 | `:host`, `:host()`, `:host-context()` | `selectors/matching.rs` `is_host`, `host_or_ancestor_matches`, `upward`, `style/resolver.rs` `cascade` | `tests/style/m3.rs` | partial | `:state()` is not implemented. Specificity of the functional forms counts the argument. |
| Shadow DOM 1 | `::part()` and `::slotted()` | `selectors/matching.rs` `matches_part`, `is_slotted`, `parse/selectors.rs` `functional_pseudo_element` | `tests/style/m3.rs`, `tests/cssom/main.rs` | partial | Slotted elements inherit from the light-tree parent, not from the slot. `::slotted()` takes one compound. |
| 9.3 | shadow trees: host, shadow root, slot assignment | `tree/contract.rs` `shadow`, `shadow_root`, `assigned_slot`, `tree/scope.rs` `shadow_root_of`, `flat_parent` | `tests/style/m3.rs` | partial | The host must report the tree relations. Document rules do not match inside a shadow tree. |
| CSS Custom State | `:state(name)` | `selectors/matching.rs` `CustomState`, `parse/selectors.rs` `functional_pseudo` | `tests/style/m3.rs` | done | Names are case-sensitive. The host reports the states. |
| Shadow DOM 1 | slot inheritance | `style/resolver.rs` `inherited_parent` | `tests/style/m3.rs` | done | A slotted element inherits from its slot. |
| CSS Pseudo-Elements (highlight) | `::highlight(name)` | `parse/selectors.rs` `functional_pseudo_element`, `style/resolver.rs` `is_generated` | `tests/style/m3.rs` | partial | Parsed and stored under the name. Matching of highlighted ranges is the host's job. |
| CSS Backgrounds 3 | `border-radius` (1 to 4 values) and the corner longhands | `properties/registry.rs`, `properties/shorthand.rs` `radius_expand` | `tests/style/m3.rs` | partial | The elliptical `/` form is not supported. |
