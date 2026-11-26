# tpt-css TODO

Project: tpt-css, a clean-room CSS engine in Rust.
License: dual `MIT OR Apache-2.0`. Copyright TPT Solutions.
Source of truth: [spec.txt](spec.txt). Section numbers below refer to it.

Legend: `[ ]` not started, `[~]` in progress, `[x]` done.

---

## 0. Repo setup

- [x] `git init` in project folder
- [ ] Add `LICENSE-MIT` (copyright TPT Solutions) (spec 23.2)
- [ ] Add `LICENSE-APACHE` (full Apache-2.0 text) (spec 23.2)
- [ ] Add `NOTICE` with TPT Solutions copyright (spec 23.2)
- [ ] Add `README.md` (spec 23.2)
- [ ] Add `CLEAN_ROOM.md` with provenance rules and contributor log (spec 23.1)
- [ ] Add `DESIGN.md` (spec 6)
- [ ] Add `SPEC_TRACE.md` skeleton, section -> module -> tests (spec 22.5)
- [ ] Add `deny.toml` with license and ban rules (spec 23.3). Deny Apache-only licenses for dependencies, allow only dependencies that offer MIT (`MIT` or `MIT OR Apache-2.0`)
- [ ] Add `.gitignore` for Rust (`target/`)
- [ ] Decide Rust toolchain version and add `rust-toolchain.toml` (MSRV)
- [ ] Create `Cargo.toml` with dual license field `MIT OR Apache-2.0`, zero dependencies (spec 6, 7, 23.2)
- [ ] Create `src/lib.rs` with `#![forbid(unsafe_code)]` (spec 23.3)
- [ ] Add every first-party file header `// SPDX-License-Identifier: MIT OR Apache-2.0` (spec 23.2)
- [ ] Decide first commit scope and make it (spec 26)
- [ ] Add CI workflow: fmt, clippy, test, feature matrix, deny, fuzz smoke, spec trace, unsafe check (spec 23.3)

## 1. Open design decisions (resolve in M0, spec 25)

- [ ] 25.1 Numeric type: `f64` first, or decimal/rational now?
- [ ] 25.2 Atom interning: custom deterministic interner, or approved primitive?
- [ ] 25.3 `StyleTree` return type: slices, iterators, or arena handles?
- [ ] 25.4 Pseudo-elements: generated nodes in the tree, or side tables keyed by node + pseudo?
- [ ] 25.5 Font metrics: what belongs in CSS vs. layout/font system?
- [ ] 25.6 Container query size requests without layout cycles
- [ ] 25.7 Animation interpolation: `tpt-css`, `tpt-layout`, or separate crate?
- [ ] 25.8 CSSOM round-trip fidelity for M3
- [ ] 25.9 Web UA stylesheet location: `tpt-css`, `tpt-dom`, or `tpt-html`?
- [ ] 25.10 Dependency policy: no third-party core dependencies; no Apache-only dependencies; any dependency must offer MIT

## 2. M0: syntax + basic cascade (spec 24, 26)

### Syntax layer (spec 10)
- [ ] `syntax/token.rs`: token kinds (spec 10.1)
- [ ] `syntax/tokenizer.rs`: CSS Syntax 3 tokenizer, escapes, newline normalization, bad strings and URLs
- [ ] `syntax/component.rs`: component values, simple blocks `()` `[]` `{}`, functions
- [ ] `syntax/span.rs`: `Span { source, start, end }` on meaningful nodes (spec 10.3)
- [ ] Bounded input length and token count (spec 19.2)
- [ ] Optional comment preservation for tooling

### AST (spec 11)
- [ ] `ast/stylesheet.rs`, `ast/rule.rs`, `ast/declaration.rs`, `ast/selector.rs`, `ast/value.rs`
- [ ] Unknown at-rules preserved but ignored by resolution

### Parser (spec 8.1, 11, 19)
- [ ] `parse/stylesheet.rs`: `parse_stylesheet(input, ParseOptions) -> ParsedStylesheet`
- [ ] `parse/declarations.rs`: declarations, custom property names (`--`)
- [ ] `parse/rules.rs`: style rules and at-rule skeleton
- [ ] `parse/diagnostics.rs`: `Diagnostic`, severity, codes, spec URL (spec 19.3)
- [ ] Spec-defined error recovery: invalid declarations do not abort the sheet
- [ ] `limits.rs`: `ParseLimits` with configurable defaults (spec 19.2)

### Selectors (spec 12)
- [ ] `parse/selectors.rs`: Tier 0 selectors: `*`, type, `#id`, `.class`, `[attr]`, `[attr=value]`, descendant, child, `:root`
- [ ] State selectors `:hover`, `:active`, `:focus` (adapter-provided state)
- [ ] `selectors/specificity.rs`: explicit `Specificity { a, b, c }` computed at parse time (spec 12.3)
- [ ] `selectors/matching.rs`: right-to-left matching, candidate filtering by id/class/type
- [ ] Selector complexity bound (spec 19.2)

### Cascade (spec 13)
- [ ] `cascade/origin.rs`: `Origin`, `Importance` (spec 13.2)
- [ ] `cascade/order.rs`: source order and `DeclarationOrder`
- [ ] `cascade/specificity.rs`
- [ ] `cascade/collect.rs`: collect matched declarations
- [ ] Cascade key struct (spec 13.3) and origin/importance ordering
- [ ] Inline style enters cascade by origin/importance/specificity/order (spec 13.5)
- [ ] Shorthand expansion to longhands for cascade (spec 13.6, 15.3)
- [ ] CSS-wide keywords `initial`, `inherit`, `unset` (spec 13.8)

### Values (spec 14, basic)
- [ ] `values/number.rs`, `values/length.rs` (px, em, rem, %)
- [ ] `values/color.rs`: named, `transparent`, `#rgb`, `#rrggbb`, `rgb()`, `rgba()`
- [ ] `values/serialize.rs`: deterministic serialization

### Properties and style (spec 15, 16)
- [ ] `properties/registry.rs`: data-driven `PropertySpec` (spec 15.1)
- [ ] Initial M0 property set (spec 15.2, 37 properties)
- [ ] `properties/aliases.rs`: explicit alias registry, no string stripping (spec 15.4)
- [ ] `style/resolver.rs`: 14-step pipeline (spec 16.1)
- [ ] `style/inheritance.rs`
- [ ] `style/computed.rs`: `ComputedStyle` with typed accessors and property-ID storage (spec 8.3)
- [ ] `StyleMap` with `get`, `get_pseudo`, `custom_property` (spec 8.3)
- [ ] Font-size computed first (spec 16.1)

### Tree contract (spec 9)
- [ ] `tree/node.rs`: `StyleTree` trait, `NodeId` generational id (spec 9)
- [ ] `tree/attributes.rs`, `tree/state.rs` (`ElementState`), `tree/scope.rs`, `tree/contract.rs`
- [ ] No HTML-specific names in core (spec 4.3)

### Tests and fixtures (spec 22)
- [ ] Golden test harness
- [ ] `tests/syntax/` tokenizer golden tests
- [ ] `tests/parse/` stylesheet and declaration parser tests
- [ ] `tests/selectors/` selector parse and match tests, specificity tests
- [ ] `tests/cascade/` origin and order tests
- [ ] `tests/style/` inheritance and initial value tests
- [ ] `tests/integration/` same resolver on AppFront-style tree and DOM-adapter-style tree (spec 26)
- [ ] `fixtures/mit/` own fixtures, written from spec behavior (spec 22.2)
- [ ] `fixtures/golden/`

### M0 acceptance (spec 24)
- [ ] Parse a stylesheet
- [ ] Match basic selectors against an abstract tree
- [ ] Compute styles for a static tree
- [ ] Deterministic output
- [ ] No DOM dependency
- [ ] No third-party core dependency
- [ ] No unsafe

## 3. M1: practical CSS (spec 24)

- [ ] Full Tier 1 selectors: attribute operators, `:first-child`, `:last-child`, `:only-child`, `:nth-*`, `:first-of-type` and related, `:empty`, `:not()`, `:is()`, `:where()`, `:lang()`, `:dir()`, form states
- [ ] Sibling combinators `+` and `~`
- [ ] Pseudo-elements `::before`, `::after`, `::marker`, `::placeholder` (spec 12.5)
- [ ] `content` property
- [ ] Custom properties with cascade (spec 13.7)
- [ ] `var()` with fallbacks (spec 14.6)
- [ ] Custom property cycle detection and `max_var_depth` (spec 13.7, 19.2)
- [ ] `calc()`, `min()`, `max()`, `clamp()` with unit algebra and simplification (spec 14.4)
- [ ] Media queries, `@media`, evaluation separate from matching (spec 16.3)
- [ ] Remaining basic colors: `#rgba`, `#rrggbbaa`, `hsl()`, `hsla()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color-mix()` (spec 14.5)
- [ ] Tier 0 and Tier 1 units (spec 14.3)
- [ ] Shorthands: `background`, `border`, `margin`, `padding`, `font`, `list-style`, `text-decoration`
- [ ] M1 property set (spec 15.2)
- [ ] Invalidation foundation (spec 17)
- [ ] Fixtures: selector, cascade, var, calc
- [ ] Style a nontrivial AppFront document
- [ ] Style a static web-like fixture tree

## 4. M2: modern cascade (spec 24)

- [ ] `@layer` with first-declaration order and layer cascade rules (spec 13.4)
- [ ] `@supports` using engine's own capabilities, `selector()`, `not` (spec 16.4)
- [ ] `@scope`, `:scope`, Cascade 6 scope proximity (spec 13.3)
- [ ] CSS nesting and `&` (spec 12.3)
- [ ] `@container` foundation and `ContainerSizeProvider` (spec 16.5)
- [ ] `@property` and registered custom properties (spec 13.7)
- [ ] `revert` and `revert-layer` (spec 13.8)
- [ ] `all` shorthand (spec 13.8)
- [ ] Animation and transition cascade origins (spec 13.2)
- [ ] Keyframe style resolution (`@keyframes`)
- [ ] `@font-face` parsing
- [ ] Modern color functions: `color()`, relative color syntax, `light-dark()`
- [ ] Viewport and container units (spec 14.3)
- [ ] `:has()`, `:host`, `:host-context()`, `::slotted()`, `::part()`, `:state()` (spec 12.2 Tier 2)
- [ ] Modern state selectors: `:focus-visible`, `:focus-within`, `:target`, `:any-link`, `:link`, `:visited`, form validity states, `:open`, `:closed`, `:modal`, `:popover-open`
- [ ] Modern pseudo-elements: `::selection`, `::backdrop`, `::file-selector-button`
- [ ] Layer and scope behavior tests (spec 22.1)
- [ ] Registered custom property tests
- [ ] Container query integration test with size provider
- [ ] Animation declarations participate in cascade test

## 5. M3: incremental style + CSSOM (spec 24)

- [ ] Selector dependency graph (spec 12.4, 17.2)
- [ ] Custom property dependency graph (spec 17.2)
- [ ] `StyleChange` enum and `RestyleHint` (spec 17.1, 17.2)
- [ ] `invalidate` and `restyle` on `StyleResolver` (spec 8.2)
- [ ] Style sharing with safe sharing key (spec 17.3)
- [ ] `:has()` invalidation graph (spec 12.4)
- [ ] Media and container query invalidation (spec 16.3, 16.5)
- [ ] `cssom/`: `CSSStyleSheet`, `CSSRuleList`, rule types, `CSSStyleDeclaration` (spec 18), behind `cssom` feature
- [ ] Mutation APIs with correct invalidation: `insert_rule`, `delete_rule`, `selector_text`, `set_property`, `remove_property` (spec 18)
- [ ] Deterministic CSSOM serialization (spec 18, 7 `serialize`)
- [ ] `getComputedStyle` support contract
- [ ] Devtools-facing inspection data (spec 7 `diagnostics`)
- [ ] Visited-link privacy policy: `VisitedPolicy` (spec 20.2)
- [ ] Web profile integration tests (`--features web`)
- [ ] Acceptance: incremental restyle, no full-document restyle for simple local changes

## 6. M4: long-tail web compatibility (spec 24)

- [ ] Long-tail selectors (spec 12.2 Tier 3)
- [ ] Long-tail properties and advanced values (spec 15.2)
- [ ] Advanced colors and media features (spec 14.5, 16.3)
- [ ] Shadow DOM edge cases (spec 9.3)
- [ ] `::part`, `::slotted`, `::highlight()` and related pseudos
- [ ] Serialization edge cases
- [ ] Remaining calc functions: `round()`, `mod()`, `rem()`, trig, `pow()`, `sqrt()`, `log()`, `exp()`, `sign()`, `progress()` (spec 14.4)
- [ ] External WPT CSS subsuite runs in CI from separate checkout (spec 22.2)
- [ ] Record pass/fail deltas in `SPEC_TRACE.md` (spec 22.2)

## 7. Cross-cutting (every milestone)

- [ ] Each algorithm cites its spec section (spec 2.5)
- [ ] Each supported property, selector, cascade rule has tests (spec 2.5)
- [ ] `SPEC_TRACE.md` updated (spec 22.5)
- [ ] `CLEAN_ROOM.md` provenance log updated for each contributor (spec 23.1)
- [ ] Fuzz targets: tokenizer, stylesheet, selector, declaration, var, calc, color (spec 22.3)
- [ ] Property-based tests: specificity, cascade stability, shorthand round trip, calc invariants, `:is`/`:where` equivalence, cycle detection, serialization determinism (spec 22.4)
- [ ] Performance targets measured: tokenizer throughput, static tree restyle (spec 21.1)
- [ ] Bounds enforced: tokens, nesting, selector work, var depth, calc depth, stylesheet and rule counts (spec 20.3)
- [ ] No panics on hostile input (spec 19.1)
- [ ] Determinism: no hash-map iteration in cascade (spec 21.3)
- [ ] `cargo deny check licenses` and `bans` pass
- [ ] No Apache-only dependencies in the dependency tree (spec 23.2)
- [ ] No `unsafe` (spec 23.3)
- [ ] Each commit keeps `cargo test`, `cargo test --no-default-features`, `cargo test --features web` green
