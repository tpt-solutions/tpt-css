# Spec trace

This file maps [spec.txt](spec.txt) sections to modules, algorithms, and tests,
as spec 22.5 asks. The external conformance column is empty until the external
suites run in CI from a separate checkout (spec 22.2). Nothing here has been
run against them yet.

Status key: **done** = implemented and tested in M0. **partial** = implemented
with the gap named in the last column. **todo** = not started.

## Syntax (spec 10)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 10.1, CSS Syntax 3 §4.3.1 | consume a token | `syntax/tokenizer.rs` `consume_token` | `tests/syntax` | not run | done | Comment preservation is not implemented. |
| 10.1, CSS Syntax 3 §4.3.7 | consume an escaped code point | `syntax/tokenizer.rs` `consume_escape` | `tests/syntax` (`hex_escapes_and_replacement_characters`) | not run | done | |
| 10.1, CSS Syntax 3 §4.3.4–4.3.6 | consume a url token, bad url | `syntax/tokenizer.rs` `consume_url` | `tests/syntax` (`unquoted_url_and_bad_url`) | not run | done | |
| 10.1, CSS Syntax 3 §4.3.3 | consume a number | `syntax/tokenizer.rs` `consume_number` | `tests/syntax` | not run | done | Values are `f64` (spec 25.1). |
| 10.2, CSS Syntax 3 §5.3 | consume a component value | `syntax/component.rs` `build_component_values` | `tests/syntax` | not run | done | The build is iterative. Nesting beyond the limit fails the whole sheet. |
| 10.3 | source spans | `syntax/span.rs` | `tests/syntax` (`crlf_is_one_whitespace_token...`) | n/a | done | Spans are byte offsets, not line and column. |

## Parse (spec 8.1, 11)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 8.1 | parse a stylesheet | `parse/stylesheet.rs` | `tests/parse` | not run | done | Quirks mode is not modeled. |
| CSS Syntax 3 §5.4.1, §5.4.3 | consume a list of rules, qualified rule | `parse/rules.rs` | `tests/parse` | not run | partial | Nested rules are M2. |
| CSS Syntax 3 §5.4.4 | consume a declaration | `parse/declarations.rs` | `tests/parse` (`an_invalid_declaration_does_not_abort...`) | not run | done | Custom properties are kept but not used (M1). |
| 11.2 | at-rule skeleton | `parse/rules.rs` `consume_at_rule` | `tests/parse` (`unknown_at_rules_are_preserved...`) | not run | partial | Only unknown at-rules are preserved. |
| 19.3 | diagnostics | `parse/diagnostics.rs` | `tests/parse` | n/a | done | |
| 19.2 | limits | `limits.rs` | `tests/parse` (`selector_complexity...`, `declaration_limit...`, `deep_nesting...`), `tests/syntax` | n/a | partial | `max_var_depth`, `max_calc_depth`, and a stylesheet count limit are not enforced yet. |

## Selectors (spec 12)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 12.1, Selectors 4 | parse a selector list (Tier 0) | `parse/selectors.rs` | `tests/parse` | not run | partial | Namespaces, `+`, `~`, pseudo-elements, and functional pseudo-classes are M1. |
| 12.2 Tier 0 | `*`, type, `#id`, `.class`, `[attr]`, `[attr=value]`, descendant, child, `:root` | `parse/selectors.rs`, `selectors/matching.rs` | `tests/parse`, `tests/selectors` | not run | done | Attribute values are case-sensitive in the core. |
| 12.2 state | `:hover`, `:active`, `:focus` | `selectors/matching.rs` | `tests/selectors` (`state_pseudo_classes...`) | not run | done | |
| 12.3 | specificity | `selectors/specificity.rs` | `tests/parse` (`specificity_follows_spec_12_3`) | not run | done | `:nth-child(… of S)` and `:is()`, `:not()` arrive in M1. |
| 12.4 | right-to-left matching | `selectors/matching.rs` `matches_complex` | `tests/selectors` | not run | partial | No candidate filtering by id, class, or type yet, and no bloom filter. |
| 12.4 | step budget (spec 20.3) | `selectors/matching.rs` | `tests/selectors` (`a_zero_step_budget_fails_closed`) | n/a | done | |

## Cascade (spec 13)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 13.2, Cascade 5 §6.1 | origin and importance order | `cascade/origin.rs` `precedence` | `tests/cascade` (`origin_and_importance_follow_spec_13_2`, `ua_important_beats_author_important`) | not run | done | Animation and transition ranks have no importance. |
| 13.3 | cascade key | `cascade/mod.rs` `CascadeKey` | `tests/cascade` | not run | partial | Layer and scope proximity fields are M2. |
| 13.3 | declaration order | `cascade/order.rs` | `tests/cascade` (`later_declaration_wins...`) | not run | done | |
| 13.5 | inline style | `style/resolver.rs` `cascade` | `tests/cascade` (`inline_*`) | not run | done | |
| 13.6 | shorthand expansion | — | — | — | todo | M1. No shorthands are registered. |
| 13.8 | `initial`, `inherit`, `unset` | `style/resolver.rs` `specified_value` | `tests/style` (`initial_keyword...`, `unset_...`, `explicit_inherit...`) | not run | done | `revert` and `revert-layer` are M2. |
| 13.7 | custom properties | `parse/declarations.rs` (parse only) | — | — | todo | Cascade and substitution are M1. |
| 13.4 | layers | — | — | — | todo | M2. |

## Values and properties (spec 14, 15)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 14.3 Tier 0 | lengths px, em, rem, % | `values/length.rs`, `style/computed.rs` | `tests/style` (`em_in_font_size...`, `rem_uses_the_root...`, `percentages_are_kept...`) | not run | done | Tier 1 and Tier 2 units are M1 and M2. |
| 14.5 Tier 0 | colors: named, transparent, `#rgb`, `#rrggbb`, `rgb()`, `rgba()` | `values/color.rs` | `tests/style` (color tests) | not run | partial | The named color table has 16 basic colors and `orange` only. The full 148-name table is pending. |
| 14.2 | deterministic serialization | `values/serialize.rs` | `tests/style` (`color_serialization_is_canonical`) | not run | partial | Canonical form only. Spec-exact CSSOM serialization is M3. |
| 15.1 | property registry | `properties/registry.rs` | `tests/style` (`registry_has_37_properties...`) | n/a | done | `animatable` and `layout_effect` fields are M2 and M3. |
| 15.2 | M0 property set (37) | `properties/registry.rs` | `tests/style` | n/a | done | |
| 15.4 | aliases | — | — | — | todo | M1. |
| 14.1 | grammar parsing | `properties/parse.rs` | `tests/style` (`length_grammar...`, `font_family_lists...`, etc.) | n/a | done | |

## Computed style and resolution (spec 8.3, 16)

| Spec | Algorithm | Module | Tests | External | Status | Known gaps |
| --- | --- | --- | --- | --- | --- | --- |
| 16.1 steps 1–4, 9–13 | collect, cascade, CSS-wide, compute, inherit, store | `style/resolver.rs` | `tests/style`, `tests/cascade`, `tests/integration` | not run | done | |
| 16.1 steps 5–8 | custom properties and `var()` | — | — | — | todo | M1. |
| 16.1 step 12 | pseudo-element styles | — | — | — | todo | M1. |
| 16.1 font-size first | `compute_order` | `style/resolver.rs` | `tests/style` (`em_in_font_size...`) | n/a | done | |
| 8.3 | `StyleMap` `get` | `style/computed.rs` | `tests/integration` | n/a | done | |
| 8.3 | `StyleMap` `get_pseudo`, `custom_property` | — | — | — | todo | M1. |
| 8.3 | typed accessors | `style/computed.rs` | `tests/style` | n/a | partial | `font_size()` returns px as `f64`. `display()` returns the keyword string. |
| 4.4 | keep layout-dependent values unresolved | `style/computed.rs` `compute_value` | `tests/style` | n/a | done | |
| 16.2 | environment | `env/mod.rs` | `tests/integration` | n/a | partial | Viewport and root font size only. Media, containers, fonts, URLs, and `env()` are later. |
| 9 | tree contract | `tree/` | `tests/integration` (`the_tree_contract...`), all suites via `tests/common` | n/a | done | Scope and shadow fields are defined but unused until M2 and M4. |
| 4.3 | no HTML names in core | `src/` | grep check, see README | n/a | done | |
| 20.1 | no network in core | `src/` | by construction (no I/O) | n/a | done | |
| 19.1 | no panics on hostile input | all | `tests/integration` (`hostile_input_never_panics`) | n/a | partial | Randomized test, not coverage-guided fuzzing. Fuzz targets are not written. |
| 21.3 | determinism | `style/computed.rs` (`BTreeMap`), `style/resolver.rs` (`Vec` winners) | `tests/style` (`styling_the_same_tree_twice...`) | n/a | done | |

## Milestones

| Milestone | Status |
| --- | --- |
| M0 | Implemented, with the partial items above. |
| M1 | Not started. |
| M2 | Not started. |
| M3 | Not started. |
| M4 | Not started. |
