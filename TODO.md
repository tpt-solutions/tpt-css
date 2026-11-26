# tpt-css TODO

Project: tpt-css, a clean-room CSS engine in Rust.
License: dual `MIT OR Apache-2.0`. Copyright TPT Solutions.
Source of truth: [spec.txt](spec.txt). Section numbers below refer to it.
Decisions and deviations: [DESIGN.md](DESIGN.md). Spec traceability: [SPEC_TRACE.md](SPEC_TRACE.md).

Legend: `[ ]` not started, `[~]` in progress or partly done (the note says what is missing), `[x]` done.

---

## 0. Repo setup

- [x] `git init` in project folder
- [x] Add `LICENSE-MIT` (copyright TPT Solutions) (spec 23.2)
- [x] Add `LICENSE-APACHE` (full Apache-2.0 text) (spec 23.2)
- [x] Add `NOTICE` with TPT Solutions copyright (spec 23.2)
- [x] Add `README.md` (spec 23.2)
- [x] Add `CLEAN_ROOM.md` with provenance rules and contributor log (spec 23.1)
- [x] Add `DESIGN.md` (spec 6)
- [x] Add `SPEC_TRACE.md` skeleton, section -> module -> tests (spec 22.5)
- [x] Add `deny.toml` with license and ban rules (spec 23.3). Deny Apache-only licenses for dependencies, allow only dependencies that offer MIT (`MIT` or `MIT OR Apache-2.0`). `cargo deny check licenses bans` passes.
- [x] Add `.gitignore` for Rust (`target/`)
- [x] Decide Rust toolchain version and add `rust-toolchain.toml` (MSRV). Decided: `stable` for development, MSRV 1.75 in `Cargo.toml`. CI runs the MSRV job. The MSRV job has not been run locally (1.75 is not installed here).
- [x] Create `Cargo.toml` with dual license field `MIT OR Apache-2.0`, zero dependencies (spec 6, 7, 23.2)
- [x] Create `src/lib.rs` with `#![forbid(unsafe_code)]` (spec 23.3)
- [x] Add every first-party file header `// SPDX-License-Identifier: MIT OR Apache-2.0` (spec 23.2). Checked: all 50 `.rs` files.
- [x] Decide first commit scope and make it (spec 26). Scope: the M0 foundation as one commit on branch `m0-foundation`, not pushed.
- [~] Add CI workflow: fmt, clippy, test, feature matrix, deny, fuzz smoke, spec trace, unsafe check (spec 23.3). Written in `.github/workflows/ci.yml` but not run on a real runner. Fuzz smoke and spec trace check are missing because their targets and tools do not exist yet.

## 1. Open design decisions (resolve in M0, spec 25)

All decisions are recorded in DESIGN.md section 2.

- [x] 25.1 Numeric type: `f64` first, or decimal/rational now? Decided: `f64` for M0.
- [x] 25.2 Atom interning: custom deterministic interner, or approved primitive? Decided: no interner in M0, owned strings and `'static` keyword tables. Revisit in M1.
- [x] 25.3 `StyleTree` return type: slices, iterators, or arena handles? Decided: slices for children, references for attributes and state.
- [x] 25.4 Pseudo-elements: generated nodes in the tree, or side tables keyed by node + pseudo? Decided: side tables. Implemented in M1.
- [x] 25.5 Font metrics: what belongs in CSS vs. layout/font system? Decided: CSS does only font-relative arithmetic.
- [x] 25.6 Container query size requests without layout cycles. Decided: size containers only, host-ordered. Implemented in M2.
- [x] 25.7 Animation interpolation: `tpt-css`, `tpt-layout`, or separate crate? Decided: separate crate.
- [x] 25.8 CSSOM round-trip fidelity for M3. Decided: canonical serialization, not byte-preserving.
- [x] 25.9 Web UA stylesheet location: `tpt-css`, `tpt-dom`, or `tpt-html`? Decided: `tpt-html`.
- [x] 25.10 Dependency policy: no third-party core dependencies; no Apache-only dependencies; any dependency must offer MIT

## 2. M0: syntax + basic cascade (spec 24, 26)

### Syntax layer (spec 10)
- [x] `syntax/token.rs`: token kinds (spec 10.1)
- [x] `syntax/tokenizer.rs`: CSS Syntax 3 tokenizer, escapes, newline normalization, bad strings and URLs
- [x] `syntax/component.rs`: component values, simple blocks `()` `[]` `{}`, functions
- [x] `syntax/span.rs`: `Span { source, start, end }` on meaningful nodes (spec 10.3)
- [x] Bounded input length and token count (spec 19.2)
- [ ] Optional comment preservation for tooling

### AST (spec 11)
- [~] `ast/stylesheet.rs`, `ast/rule.rs`, `ast/declaration.rs`, `ast/selector.rs`, `ast/value.rs`. `value.rs` is not created: declarations keep their `ComponentValue`s directly.
- [x] Unknown at-rules preserved but ignored by resolution

### Parser (spec 8.1, 11, 19)
- [x] `parse/stylesheet.rs`: `parse_stylesheet(input, ParseOptions) -> ParsedStylesheet`
- [x] `parse/declarations.rs`: declarations, custom property names (`--`)
- [x] `parse/rules.rs`: style rules and at-rule skeleton
- [x] `parse/diagnostics.rs`: `Diagnostic`, severity, codes, spec URL (spec 19.3)
- [x] Spec-defined error recovery: invalid declarations do not abort the sheet. Exception: a nesting-depth violation drops the whole sheet (DESIGN.md 3.8).
- [x] `limits.rs`: `ParseLimits` with configurable defaults (spec 19.2)

### Selectors (spec 12)
- [x] `parse/selectors.rs`: Tier 0 selectors: `*`, type, `#id`, `.class`, `[attr]`, `[attr=value]`, descendant, child, `:root`
- [x] State selectors `:hover`, `:active`, `:focus` (adapter-provided state)
- [x] `selectors/specificity.rs`: explicit `Specificity { a, b, c }` computed at parse time (spec 12.3)
- [~] `selectors/matching.rs`: right-to-left matching is done. Candidate filtering by id, class, and type is not done.
- [x] Selector complexity bound (spec 19.2)

### Cascade (spec 13)
- [x] `cascade/origin.rs`: `Origin`, `Importance` (spec 13.2)
- [x] `cascade/order.rs`: source order and `DeclarationOrder`
- [x] `cascade/specificity.rs`: merged into `selectors/specificity.rs`, which the cascade uses (DESIGN.md 5)
- [x] `cascade/collect.rs`: collect matched declarations. Collection lives in `style/resolver.rs` (DESIGN.md 5).
- [x] Cascade key struct (spec 13.3) and origin/importance ordering
- [x] Inline style enters cascade by origin/importance/specificity/order (spec 13.5)
- [ ] Shorthand expansion to longhands for cascade (spec 13.6, 15.3). No shorthands are registered yet (M1).
- [x] CSS-wide keywords `initial`, `inherit`, `unset` (spec 13.8). `revert` and `revert-layer` are M2.

### Values (spec 14, basic)
- [~] `values/number.rs`, `values/length.rs` (px, em, rem, %). `length.rs` is done. Numbers are `Value::Number` and the tokenizer. There is no `number.rs`.
- [~] `values/color.rs`: named, `transparent`, `#rgb`, `#rrggbb`, `rgb()`, `rgba()`. Named colors are partial: the 16 basic colors and `orange`. The full table is pending.
- [x] `values/serialize.rs`: deterministic serialization (canonical form, not yet spec-exact CSSOM)

### Properties and style (spec 15, 16)
- [x] `properties/registry.rs`: data-driven `PropertySpec` (spec 15.1)
- [x] Initial M0 property set (spec 15.2, 37 properties)
- [ ] `properties/aliases.rs`: explicit alias registry, no string stripping (spec 15.4). Not started (M1).
- [~] `style/resolver.rs`: 14-step pipeline (spec 16.1). Steps 1–4 and 9–13 are done. Steps 5–8 (custom properties, `var()`) are M1.
- [x] `style/inheritance.rs`: inheritance is in `style/resolver.rs` (DESIGN.md 5)
- [x] `style/computed.rs`: `ComputedStyle` with typed accessors and property-ID storage (spec 8.3)
- [~] `StyleMap` with `get`, `get_pseudo`, `custom_property` (spec 8.3). `get` is done. `get_pseudo` and `custom_property` are M1.
- [x] Font-size computed first (spec 16.1)

### Tree contract (spec 9)
- [x] `tree/node.rs`: `StyleTree` trait, `NodeId` generational id (spec 9)
- [x] `tree/attributes.rs`, `tree/state.rs` (`ElementState`), `tree/scope.rs`, `tree/contract.rs`
- [x] No HTML-specific names in core (spec 4.3). Checked with a case-sensitive search of `src/`.

### Tests and fixtures (spec 22)
- [x] Golden test harness
- [x] `tests/syntax/` tokenizer golden tests (and component value tests)
- [x] `tests/parse/` stylesheet and declaration parser tests
- [x] `tests/selectors/` selector parse and match tests, specificity tests
- [x] `tests/cascade/` origin and order tests
- [x] `tests/style/` inheritance and initial value tests
- [x] `tests/integration/` same resolver on AppFront-style tree and DOM-adapter-style tree (spec 26)
- [x] `fixtures/mit/` own fixtures, written from spec behavior (spec 22.2)
- [x] `fixtures/golden/`

### M0 acceptance (spec 24)
- [x] Parse a stylesheet
- [x] Match basic selectors against an abstract tree
- [x] Compute styles for a static tree
- [x] Deterministic output
- [x] No DOM dependency
- [x] No third-party core dependency
- [x] No unsafe

## 3. M1: practical CSS (spec 24)

- [x] Full Tier 1 selectors: attribute operators, `:first-child`, `:last-child`, `:only-child`, `:nth-*`, `:first-of-type` and related, `:empty`, `:not()`, `:is()`, `:where()`, `:lang()`, `:dir()`, form states. Tests: `tests/selectors/tier1.rs`.
- [x] Sibling combinators `+` and `~`
- [x] Pseudo-elements `::before`, `::after`, `::marker`, `::placeholder` (spec 12.5). Styles are side tables; `StyleMap::get_pseudo`.
- [~] `content` property. Strings, `normal`, and `none` only. `attr()` and counters are not supported.
- [x] Custom properties with cascade (spec 13.7). Inherited; `initial` removes them.
- [x] `var()` with fallbacks (spec 14.6)
- [x] Custom property cycle detection and `max_var_depth` (spec 13.7, 19.2). A cycle makes its members invalid.
- [x] `calc()`, `min()`, `max()`, `clamp()` with unit algebra and simplification (spec 14.4). Type-checked at parse time. Percentages stay symbolic except in `font-size`. Trig, `round()`, and the other M4 functions are not implemented.
- [x] Media queries, `@media`, evaluation separate from matching (spec 16.3). Media type, `and`, `not`, lists, nesting, and the width, height, aspect-ratio, orientation, color-scheme, reduced-motion, hover, and pointer features.
- [~] Remaining basic colors: `#rgba`, `#rrggbbaa`, `hsl()`, `hsla()`, `hwb()` done. `lab()`, `lch()`, `oklab()`, `oklch()`, and `color-mix()` are not implemented (spec 14.5).
- [~] Tier 0 and Tier 1 units (spec 14.3). Length units (px, em, rem, vw, vh, vmin, vmax, cm, mm, Q, in, pt, pc) and percent are done. Angle, time, and resolution units are accepted only for the hue of `hsl()`/`hwb()`.
- [~] Shorthands: `margin`, `padding`, `border` (with sides, width, style, color), `background` (color only). `font`, `list-style`, and `text-decoration` are not implemented (spec 15.3).
- [~] M1 property set (spec 15.2). Added: `overflow`, `position`, `top`, `right`, `bottom`, `left`, `z-index`, `opacity`, `content`, `vertical-align`, `white-space`, `direction`, `unicode-bidi`. Plus the M0 set and the shorthands above. `font`, `list-style`, `text-decoration`, and background images are not implemented.
- [~] Invalidation foundation (spec 17). Attribute, state, and inline changes restyle the subtree. Sibling-sensitive rules restyle the parent's subtree. Stylesheet and tree changes restyle everything. There is no dependency index yet (M3). Tests: `tests/integration/invalidation.rs`.
- [x] Fixtures: selector, cascade, var, calc. Tests in `tests/selectors`, `tests/style/m1.rs`, `tests/cascade/media.rs`. Own fixture: `fixtures/mit/web-like.css`.
- [ ] Style a nontrivial AppFront document. Not started. The M0 integration test covers only a small AppFront-style tree.
- [x] Style a static web-like fixture tree (`tests/integration/web_like.rs`)

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

- [~] Each algorithm cites its spec section (spec 2.5). Most modules cite their section. Not audited for every function.
- [~] Each supported property, selector, cascade rule has tests (spec 2.5). Covered for M0. Not every property has its own test.
- [x] `SPEC_TRACE.md` updated (spec 22.5)
- [x] `CLEAN_ROOM.md` provenance log updated for each contributor (spec 23.1). Entry added for the M0 batch; it needs human review (see the log).
- [ ] Fuzz targets: tokenizer, stylesheet, selector, declaration, var, calc, color (spec 22.3)
- [ ] Property-based tests: specificity, cascade stability, shorthand round trip, calc invariants, `:is`/`:where` equivalence, cycle detection, serialization determinism (spec 22.4)
- [ ] Performance targets measured: tokenizer throughput, static tree restyle (spec 21.1)
- [~] Bounds enforced: tokens, nesting, selector work, var depth, calc depth, stylesheet and rule counts (spec 20.3). Done: tokens, nesting, selector work, rule and declaration counts, input size. Not yet: var depth, calc depth, stylesheet count.
- [~] No panics on hostile input (spec 19.1). A seeded randomized test passes (`tests/integration`). It is not coverage-guided fuzzing.
- [x] Determinism: no hash-map iteration in cascade (spec 21.3)
- [ ] `cargo deny check licenses` and `bans` pass. Passes locally. Not yet run in CI on a runner.
- [x] No Apache-only dependencies in the dependency tree (spec 23.2). The core has no dependencies.
- [x] No `unsafe` (spec 23.3)
- [x] Each commit keeps `cargo test`, `cargo test --no-default-features`, `cargo test --features web` green. Checked on the working tree for the M0 batch.
