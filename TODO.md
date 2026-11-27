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
- [x] Decide Rust toolchain version and add `rust-toolchain.toml` (MSRV). Decided: `stable` for development, MSRV 1.75 in `Cargo.toml`. CI runs the MSRV job. Verified locally with Rust 1.75.0: the default `cargo test` passes. The CI MSRV job runs that same command.
- [x] Create `Cargo.toml` with dual license field `MIT OR Apache-2.0`, zero dependencies (spec 6, 7, 23.2)
- [x] Create `src/lib.rs` with `#![forbid(unsafe_code)]` (spec 23.3)
- [x] Add every first-party file header `// SPDX-License-Identifier: MIT OR Apache-2.0` (spec 23.2). Checked: all 50 `.rs` files.
- [x] Decide first commit scope and make it (spec 26). Scope: the M0 foundation as one commit on branch `m0-foundation`, not pushed.
- [x] Add CI workflow: fmt, clippy, test, feature matrix, MSRV (1.75), deny, unsafe check, the spec trace check (inside `cargo test`), and fuzz smoke (spec 23.3). Written in `.github/workflows/ci.yml`. First run on GitHub Actions (run 45815543062) is green. The fuzz smoke job runs each target in `fuzz/` for 60 seconds on Linux with nightly.

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
- [x] Optional comment preservation for tooling. `ParseOptions::preserve_comments` keeps the source span of each comment in `ParsedStylesheet::comments`. Off by default (DESIGN.md decision 44).

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
- [x] `selectors/matching.rs` and the candidate index: right-to-left matching, filtering of selectors by subject id, class, and type (`src/style/rule_index.rs`), and a per-pass sibling index for `:nth-child()` and `+` (`SiblingCache`). `~` and `:nth-of-type()` use the sibling index too. `:nth-child(... of S)` uses a per-parent index of the siblings that match S, built once per pass (decision 34). Measured: an 8,000-sibling list with structural rules resolves in linear time (`tests/integration/perf.rs`).
- [x] Selector complexity bound (spec 19.2)

### Cascade (spec 13)
- [x] `cascade/origin.rs`: `Origin`, `Importance` (spec 13.2)
- [x] `cascade/order.rs`: source order and `DeclarationOrder`
- [x] `cascade/specificity.rs`: merged into `selectors/specificity.rs`, which the cascade uses (DESIGN.md 5)
- [x] `cascade/collect.rs`: collect matched declarations. Collection lives in `style/resolver.rs` (DESIGN.md 5).
- [x] Cascade key struct (spec 13.3) and origin/importance ordering
- [x] Inline style enters cascade by origin/importance/specificity/order (spec 13.5)
- [x] Shorthand expansion to longhands for cascade (spec 13.6, 15.3). Done in M1 for `margin`, `padding`, `border` (and sides, width, style, color), and `background` (color only).
- [x] CSS-wide keywords `initial`, `inherit`, `unset` (spec 13.8). `revert` and `revert-layer` are M2.

### Values (spec 14, basic)
- [~] `values/number.rs`, `values/length.rs` (px, em, rem, %). `length.rs` is done. Numbers are `Value::Number` and the tokenizer. There is no `number.rs`.
- [x] `values/color.rs`: all 148 named colors, `transparent`, hex (3, 4, 6, 8 digits), `rgb()`, `rgba()`, `hsl()`, `hsla()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color()`, `color-mix()`, and relative color syntax (see section 6).
- [x] `values/serialize.rs`: deterministic serialization (canonical form, not yet spec-exact CSSOM)

### Properties and style (spec 15, 16)
- [x] `properties/registry.rs`: data-driven `PropertySpec` (spec 15.1)
- [x] Initial M0 property set (spec 15.2, 37 properties)
- [x] `properties/aliases.rs`: explicit alias registry, no string stripping (spec 15.4). One alias (`word-wrap` for `overflow-wrap`), and a test that vendor prefixes are not stripped (`tests/style/m3.rs`).
- [x] `style/resolver.rs`: the cascade pipeline, with custom properties and `var()` (steps 5–8) in place (spec 16.1). See `style/custom.rs`.
- [x] Inheritance is in `style/resolver.rs` (DESIGN.md 5). There is no `style/inheritance.rs`.
- [x] `style/computed.rs`: `ComputedStyle` with typed accessors and property-ID storage (spec 8.3)
- [x] `StyleMap` with `get`, `get_pseudo`, and `custom_property` (spec 8.3).
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
- [~] `content` property. Strings, `attr(name)` (plain name, mixed with strings), `normal`, and `none` are done. `attr()` reads the element's attribute at computed-value time, and a change to that attribute restyles the element. Missing attribute: empty string. `attr()` takes the `string` type and a string fallback (DESIGN.md decision 43). Typed `attr()` is done too (DESIGN.md decision 64): units, `type(<number>)`, `<integer>`, `<length>`, `<percentage>`, `<length-percentage>`, `<color>`, `<time>`, `<angle>`, and `<url>` (`attr(name url)`), in any property. Other `attr()` syntaxes are not done. Counters are done in `content` (`counter()` and `counters()`, decision 38). Tests: `tests/style/m1.rs`, `tests/integration/invalidation.rs`, `tests/integration/counters.rs`.
- [x] Custom properties with cascade (spec 13.7). Inherited; `initial` removes them.
- [x] `var()` with fallbacks (spec 14.6)
- [x] Custom property cycle detection and `max_var_depth` (spec 13.7, 19.2). A cycle makes its members invalid.
- [x] `calc()`, `min()`, `max()`, `clamp()` with unit algebra and simplification (spec 14.4). Type-checked at parse time. Percentages stay symbolic except in `font-size`. Trig, `round()`, and the other M4 functions are not implemented.
- [x] Media queries, `@media`, evaluation separate from matching (spec 16.3). Media type, `and`, `not`, lists, nesting, and the width, height, aspect-ratio, orientation, color-scheme, reduced-motion, hover, and pointer features.
- [x] Remaining basic colors: `#rgba`, `#rrggbbaa`, `hsl()`, `hsla()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, and `color-mix()` are done (spec 14.5). See section 6 for the open color items.
- [~] Tier 0 and Tier 1 units (spec 14.3). Length units (px, em, rem, vw, vh, vmin, vmax, cm, mm, Q, in, pt, pc) and percent are done. Angle units (`deg`, `grad`, `rad`, `turn`) and time units (`s`, `ms`) are typed values now, used by `transform` and the transition and animation durations and delays (DESIGN.md decision 51). Resolution units (`dpi`, `dpcm`, `dppx`) are accepted only in media queries.
- [~] Shorthands: `margin`, `padding`, `border` (with sides, width, style, color), `background` (one image and one color, in any order, with `none` and `transparent` for the omitted ones; layers are supported since DESIGN.md decision 65), `font` (style, weight, stretch, small-caps, size, line height, family; no system fonts), `list-style` (type, position, and `none` for the image), and `text-decoration` (line, style, color, thickness, skip-ink). `text-decoration-thickness` is a longhand (`auto`, `from-font`, or a non-negative length or percentage). `text-decoration-skip-ink` is `auto`, `all`, or `none`; the shorthand can name only `all`, because `none` is the line value and `auto` is the thickness. `font-stretch` is a longhand (the nine keywords, or a non-negative percentage) and is inherited. `font-variant-caps` is a longhand (the CSS Fonts 4 caps keywords). `font-variant` is a shorthand over that one longhand: `normal`, `none` (which means `normal` here), or a caps keyword. Its other sub-values (ligatures, numeric, east-asian, alternates, position, emoji) are not registered, so a declaration that names them is dropped. Not done: `background` with images, and those other `font-variant-*` longhands (spec 15.3). In the `font` shorthand, a percentage stretch is not accepted, because `font-size` also takes percentages and the two cannot be told apart.
- [~] M1 property set (spec 15.2). Added: `overflow`, `position`, `top`, `right`, `bottom`, `left`, `z-index`, `opacity`, `content`, `vertical-align`, `white-space`, `direction`, `unicode-bidi`, and the long-tail rows listed in section 6. Plus the M0 set and the shorthands above. Background images are not implemented.
- [~] Invalidation foundation (spec 17). Attribute, state, and inline changes restyle the subtree. Sibling-sensitive rules restyle the parent's subtree. Stylesheet and tree changes restyle everything. There is no dependency index yet (M3). Tests: `tests/integration/invalidation.rs`.
- [x] Fixtures: selector, cascade, var, calc. Tests in `tests/selectors`, `tests/style/m1.rs`, `tests/cascade/media.rs`. Own fixture: `fixtures/mit/web-like.css`.
- [x] Style a nontrivial AppFront document. `tests/integration/appfront.rs`: layers, custom-property theme, custom states, `:has()`, a media rule, and an incremental restyle.
- [x] Style a static web-like fixture tree (`tests/integration/web_like.rs`)

## 4. M2: modern cascade (spec 24)

- [x] `@layer` with first-declaration order and layer cascade rules (spec 13.4). Tests: `tests/cascade/layers.rs`.
- [x] `@supports` using engine's own capabilities, `selector()`, `not`, `and`, `or` (spec 16.4). Evaluated at parse time.
- [x] `@scope`, `:scope`, Cascade 6 scope proximity (spec 13.3). Proximity ranks after specificity, following the sorting order of the current Cascade 6 draft. `spec.txt` 13.3 now uses the same order (corrected 2026-11-27). Unscoped rules have infinite proximity (`tests/style/m3.rs`).
- [x] CSS nesting and `&` (spec 12.3).
- [x] `@container` foundation and `ContainerSizeProvider` (spec 16.5).
- [x] `@property` and registered custom properties (spec 13.7).
- [x] `revert` and `revert-layer` (spec 13.8).
- [x] `all` shorthand (spec 13.8).
- [x] Animation and transition cascade origins (spec 13.2). No animation driver exists (spec 3).
- [x] Keyframe style resolution (`@keyframes`). `style::keyframes::value_at` interpolates numbers, lengths, percentages, and colors; other values change at the midpoint. The host supplies the progress; there is no timeline.
- [x] `@font-face` parsing.
- [~] Modern color functions: `color()`, relative color syntax, `light-dark()`. `light-dark()` and `color()` in `srgb` and `srgb-linear` are done, and so is relative `color(from ...)` for those two spaces. Relative color syntax (CSS Color 5) is done for `rgb`, `hsl`, `hwb`, `lab`, `lch`, `oklab`, and `oklch` (`tests/style/m3.rs`).
- [x] Viewport and container units (spec 14.3). `cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, `cqmax` resolve against the nearest size container.
- [x] `:has()`. Relative selectors in all four combinator forms (`tests/selectors/m3.rs`).
- [x] `:host`, `:host()`, `:host-context()`, `::part()`, `::slotted()`, and `:state()` (spec 12.2 Tier 2, Shadow DOM 1, CSS Custom State). Shadow sheets are scoped to a shadow root (`add_shadow_stylesheet`), and the host is reached through `:host span`. Slotted elements inherit from their slot. `:state()` matches the custom states the host reports. `tests/style/m3.rs`.
- [x] Modern state selectors.
- [x] Modern pseudo-elements: `::selection`, `::backdrop`, `::file-selector-button`.
- [x] Layer and scope behavior tests (spec 22.1). Layers: order, nesting, `revert-layer`, and unlayered precedence (`tests/cascade/layers.rs`). Scope: root, limit, `:scope`, and proximity ordering after specificity (`tests/selectors/m3.rs`, `tests/style/m3.rs`).
- [x] Registered custom property tests.
- [x] Container query integration test with size provider.
- [x] Animation declarations participate in cascade test.

## 5. M3: incremental style + CSSOM (spec 24)

- [~] Selector dependency graph (spec 12.4, 17.2). Not built, by decision (DESIGN.md decision 54). The restyle uses two conservative flags per rule, `structural` and `needs_ancestors`, and the environment path matches the affected rules directly. Revisit only if a profile shows the flags are too coarse.
- [~] Custom property dependency graph (spec 17.2). Not built, by decision (DESIGN.md decision 54). Custom properties are inherited, so a change alters the map of every descendant, and the equality cutoff already stops the restyle where the computed values end up unchanged. A graph from property to reader would not remove that work. Revisit if a profile shows custom-property-heavy trees are slow.
- [x] `StyleChange` enum and `RestyleHint` (spec 17.1, 17.2). The `StyleChange` enum, the dirty set, and the structural flag carry the hint semantics: a changed element restyles itself and its subtree, and the parent covers later siblings. No separate `RestyleHint` type is added (DESIGN.md decision 55). `AttributeNamed` skips an attribute that no selector reads and style does not read directly. `AttributeChanged`, which has no name, stays conservative.
- [~] `invalidate` and `restyle` on `StyleResolver` (spec 8.2). Done, with an equality cutoff: an element whose computed style is unchanged stops the restyle, when every rule is subject-only. `resolve` and `restyle` return a reference into the cache and copy nothing.
- [~] Style sharing with safe sharing key (spec 17.3). Sharing across parents and for pseudo-elements is deferred until a profile, by decision (DESIGN.md decision 57). Built behind `ResolverConfig::share_styles`, off by default (DESIGN.md decision 35). Siblings reuse a computed style when their winning declarations, container size, and (only when `attr()` is used) attributes are equal. Not shared: elements with pseudo-elements, slotted elements, and every element when a rule uses counters. Measured: 10,000 identical siblings, 102 ms to 71 ms in release (`sharing_speed`). Not done: a sampling profile on Linux CI, which the design asks for before turning it on by default; sharing across parents; and sharing pseudo-element styles.
- [~] `:has()` invalidation graph (spec 12.4). Conservative by decision (DESIGN.md decision 56): any attribute change restyles everything when a `:has()` rule exists. A finer graph needs a restyle that recomputes an ancestor without its subtree, and the equality cutoff does not allow that for rules that are not subject-only.
- [~] Media and container query invalidation (spec 16.3, 16.5). An environment change restyles only the elements that a rule which stopped or started to match, or that reads a changed viewport, root font size, or color scheme, can reach (DESIGN.md decision 52). A container provider still restyles everything (DESIGN.md decision 56).
- [x] `cssom/`: `CssomSheet`, rule and declaration editing, behind the `cssom` feature (spec 18). Tests: `tests/cssom/main.rs`. Rule types are represented by the rule AST, not by separate CSSOM interface types.
- [x] Mutation APIs with correct invalidation: `insert_rule`, `delete_rule`, `selector_text`, `set_selector_text`, `set_property`, `remove_property`. A mutation returns `StyleSheetChanged`; the host replaces the sheet in the resolver.
- [x] Deterministic CSSOM serialization (spec 18, 7 `serialize`). Canonical, not byte-preserving (spec 25.8).
- [x] `getComputedStyle` support contract (`ComputedStyle::property_values`).
- [x] Devtools-facing inspection data (spec 7 `diagnostics`), via `StyleResolver::inspect`.
- [x] Visited-link privacy policy: `VisitedPolicy` (spec 20.2).
- [x] Web profile integration tests (`--features web`). The `web` feature enables the CSSOM tests and `tests/integration/web_markup.rs`, an end-to-end test from markup (with `<style>` sheets) to computed styles. The markup reader is test support, not an engine parser (DESIGN.md decision 39).
- [~] Acceptance: incremental restyle, no full-document restyle for simple local changes. Met for subject-only rules: a local change restyles one element (`tests/integration/perf.rs`, `tests/integration/invalidation.rs`). A one-element restyle on 10,000 elements takes about 34 µs (measured, release). `resolve` and `restyle` both return a reference, so no call copies the map. Callers that keep a result clone it.

## 6. M4: long-tail web compatibility (spec 24)

- [x] Long-tail selectors (spec 12.2 Tier 3). The state pseudo-classes are done: `:current`, `:past`, `:future`, `:defined`, `:local-link`, `:playing`, `:paused`, `:fullscreen`, `:autofill`, and the media states `:muted`, `:buffering`, `:stalled`, `:seeking`, `:volume-locked` (DESIGN.md decision 49). They match the state bits that the host sets, so `:defined` matches nothing until the host reports it. The functional forms `:current(selector)`, `:past(selector)`, and `:future(selector)` are done (`SimpleSelector::TimeState`, DESIGN.md decision 45). `:dir()` falls back to `ltr` when no ancestor has a direction (DESIGN.md decision 47). Tests: `tests/style/m3.rs`, `tests/selectors/tier1.rs`.
- [~] Long-tail properties and advanced values (spec 15.2). Added as registry rows, with keyword or length grammars: `cursor`, `pointer-events`, `user-select`, `appearance`, `resize`, `outline-style`, `text-transform`, `text-indent`, `overflow-wrap`, `word-break`, `hyphens`, `text-rendering`, `text-overflow`, `float`, `clear`, `object-fit`, `image-rendering`, `mix-blend-mode`, `isolation`, `backface-visibility`, `scroll-behavior`, `overscroll-behavior`, `list-style-type`, `list-style-position`, `list-style-image` (`none` only), `text-decoration-line`, `text-decoration-style`, `text-decoration-color`, `letter-spacing`, `word-spacing`, and `border-radius` with its four corner longhands, including the elliptical form (`a b` on a corner, `/` in the shorthand). An elliptical corner is `Value::Elliptical`; a circular one stays a plain length or percentage. Typed since decision 51: `box-shadow`, `transform`, `transition-duration`, `transition-delay`, `animation-duration`, and `animation-delay`. The `font-variant-*` longhands are typed too (decision 59), and so is `font-variant-alternates`. `transition`, `animation`, and `flex` are typed shorthands with typed longhands (DESIGN.md decision 62). `filter` is typed (DESIGN.md decision 63). Not done: the `grid` grammar and shorthand (it stays pass-through text), and resolution values.
- [~] Advanced colors and media features (spec 14.5, 16.3). Full named color set (148), `lab()`, `lch()`, `oklab()`, `oklch()`, `color()` (srgb), and `color-mix()` in `srgb`, `srgb-linear`, `xyz`, `xyz-d50`, `hsl`, `hwb`, `lab`, `lch`, `oklab`, and `oklch`, with premultiplied alpha and the `shorter` and `longer` hue methods. Relative color syntax for seven functions. Media features `resolution`, `update`, `dynamic-range`, and `prefers-contrast` are done. `xyz`, `xyz-d65`, and `xyz-d50` are done in `color()` and `color-mix()`. The hue methods `increasing` and `decreasing` are done (DESIGN.md decision 48). `color-mix()` in `display-p3`, `rec2020`, `a98-rgb`, and `prophoto-rgb` is done (decisions 53 and 58), and so is `color()` in those spaces. The results are clipped to sRGB. Not done: spaces outside CSS Color 4 (for example `jzazbz`).
- [~] Shadow DOM edge cases (spec 9.3). Shadow roots are styled as the host's flat-tree children, document selectors do not match inside a shadow tree, and slotted elements inherit from their slot. Not done: `::highlight()` inside shadow trees is not checked, shadow DOM event retargeting (not a style concern), and the engine does not build shadow trees; the host supplies them through the tree contract.
- [x] `::part`, `::slotted`, and `::highlight(name)` are parsed and matched, and `::highlight` styles are stored under their name. Their declarations are the element's own for `::part` and `::slotted`. The other pseudo-elements in the identity list are not parsed.
- [~] Serialization edge cases. Canonical serialization of values, selectors, and rules; the exact CSS spelling of every edge case is not checked.
- [x] Remaining calc functions: `round()`, `mod()`, `rem()`, trig (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`), `pow()`, `sqrt()`, `log()`, `exp()`, `sign()`, `abs()`, `hypot()`, `progress()` (spec 14.4). Trig takes unitless radians, since angle units are not parsed as values.
- [x] External WPT CSS subsuite (spec 22.2). Decided out of scope for this architecture: most CSS tests are reftests (render and compare pixels), and the rest are testharness tests that need a JavaScript DOM. The engine has neither. Revisit when a DOM adapter exists. Recorded in DESIGN.md decision 33.
- [x] Record pass/fail deltas in `SPEC_TRACE.md` (spec 22.2). Not applicable: there is no WPT run (see the WPT decision in DESIGN.md).

## 7. Cross-cutting (every milestone)

- [~] Each algorithm cites its spec section (spec 2.5). Most modules cite their section; not audited function by function.
- [~] Each supported property, selector, cascade rule has tests (spec 2.5). Covered broadly. Every keyword property is checked by one generic test (`every_keyword_property_accepts_its_keywords_in_any_case_and_no_others`), and `min-height` and `max-height` have their own. Not every property has a test that names it.
- [x] `SPEC_TRACE.md` updated (spec 22.5). A test (`tests/integration/spec_trace.rs`) checks that every file the trace names exists.
- [x] `CLEAN_ROOM.md` provenance log updated for each contributor (spec 23.1). Entries added for each batch. Each is recorded as not reviewed by a person; the review requirement was waived by the repository owner on 2026-11-28.
- [~] Fuzz targets: tokenizer, stylesheet, selector, declaration, calc, and color (spec 22.3). The bodies of the six targets also run as a seeded smoke test on every platform (`the_fuzz_target_bodies_do_not_panic_on_seeded_inputs`, DESIGN.md decision 60), with 4,000 cases each, and the libFuzzer job on Linux is still unconfirmed. Written in `fuzz/` as a separate package, so libfuzzer-sys is not in the engine's dependency tree. The targets type-check against the engine. The `fuzz-smoke` job in `ci.yml` runs them (Linux, nightly, 60 seconds each). Not yet confirmed green on a runner. The var and color-function targets are covered by the stylesheet target.
- [~] Property-based tests (spec 22.4). Seeded generators cover specificity additivity, `:is()` and `:where()` equivalence, incremental restyle against a full resolve, custom property cycle termination, declaration round trips, hostile text, every arity of the `margin` and `padding` shorthands, the `calc`, `min`, `max`, and `clamp` definitions, the `border-radius` corner arity, and `round`, `mod`, `rem`, and `hypot` (`tests/integration/property.rs`). The `flex` forms, `transition` layers, `list-style` in either order, `font`, `background`, every `text-decoration-line` subset, and the unitless math functions (`sqrt`, `pow`, `exp`, `log`, trig, `atan2`, `abs`, `sign`) are covered too.
- [~] Performance targets measured (spec 21.1). Tokenizer throughput, static-tree restyle, and wide sibling lists are measured (`tests/integration/perf.rs`, ignored by default; numbers in DESIGN.md section 8). A wide list with `:nth-child()` and `+` now resolves in linear time: 8,000 siblings took 50 ms against 389 ms before the sibling index (4.2 times the time for 4 times the siblings, where quadratic would give about 16). `~` and `:nth-of-type()` on wide lists are measured and scale linearly (ratios 3.8 and 4.3 for 4 times the siblings). `:nth-child(of S)` was close to quadratic (ratio 11.7 for 4 times the siblings) and is now linear (ratio 4.0) after the per-parent index (DESIGN.md section 8, decision 34).
- [~] Bounds enforced (spec 20.3). Tokens, nesting, selector work and complexity, rule and declaration counts, input size, var depth, custom property cycles, the step budget, and stylesheet count (`max_stylesheets`, via `try_add_stylesheet`). Calc nesting is checked against `max_calc_depth` at declaration parse and after `var()` substitution. The value parser recurses up to 256, so the configured limit is the effective one.
- [~] No panics on hostile input (spec 19.1). Seeded hostile-text tests; not a fuzzer.
- [x] Determinism: no hash-map iteration in cascade (spec 21.3).
- [x] `cargo deny check licenses` and `bans` pass locally. The `deny` job in `ci.yml` runs them; the CI result is not confirmed here.
- [x] No Apache-only dependencies in the dependency tree (spec 23.2). The core has no dependencies.
- [x] No `unsafe` (spec 23.3).
- [x] Each commit keeps `cargo test`, `cargo test --no-default-features`, `cargo test --features web`, and `cargo test --features cssom` green. Checked on the working tree.

## 8. Review follow-ups (review of 2026-11-27)

Source: a review of the repo for bugs, gaps, usability, adoption, and automation. The review found no failing tests; `cargo test`, `cargo fmt --check`, and `cargo clippy -D warnings` pass on the default features.

### Docs and correctness
- [x] Commit or revert the uncommitted `ci.yml` changes: the fuzz install step (`taiki-e/install-action@v2`), the clippy matrix, the `doc` job, and `advisories` in the `deny` job. Committed in 40ce7a5, so nothing is uncommitted now.
- [x] Fix stale claims about deny and fuzz CI in sections 0 and 7.
- [x] README example is accurate. Its snippet is marked `rust,ignore` and points to `examples/minimal.rs`, a complete program that `tests/examples.rs` checks.
- [x] `cargo doc --no-deps` with `-D warnings` passes on all three feature sets. Three broken links were fixed: two public docs linked to private items, and one used a path that did not resolve.
- [x] The spec 13.3 cascade key order is already corrected in `spec.txt` (specificity before scope proximity, line 1162). The section 4 note that said it still needed correcting was stale and is fixed.
- [x] Human review of the `CLEAN_ROOM.md` batches. Removed by the repository owner on 2026-11-28. No batch has been reviewed by a person, and `CLEAN_ROOM.md` says so for each one.

### Adoption
- [x] `examples/minimal.rs`: a small arena tree implementing `StyleTree`, styled with one stylesheet. `tests/examples.rs` asserts its output.
- [x] `examples/dark_mode.rs`: custom properties switched by `prefers-color-scheme`. Checked by `tests/examples.rs`.
- [x] `examples/diagnostics.rs`: prints each parse diagnostic with its severity and position. Checked by `tests/examples.rs`.
- [x] `examples/cssom_edit.rs` (feature `cssom`): CSSOM insert and set_property, serialized. Checked by `tests/examples.rs` under `--features cssom`.
- [x] `examples/media_and_container.rs`: media queries and container sizes. Checked by `tests/examples.rs`.
- [x] `examples/common/mod.rs`: the shared arena tree for the examples. Loaded once by `tests/examples.rs` so clippy accepts it.
- [x] Starter adapter template in `templates/adapter/`: a node store implementing `StyleTree`, with a test. It is a separate package that depends on the engine by path. Its test passes locally.
- [x] `CONTRIBUTING.md`: contributions are through issues only. Covers the issue templates, the clean-room rule, and the build commands.
- [x] Issue templates in `.github/ISSUE_TEMPLATE/` (bug report, specification gap, feature request). No pull request template, because contributions are issues only.
- [x] `CHANGELOG.md` (Unreleased section).
- [x] Milestone status moved out of `README.md` into `docs/STATUS.md`.
- [x] `SECURITY.md` was never created, so there is no file to remove. The item was dropped.

### Automation
- [x] CI: `doc` job, `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings`, across the feature matrix. Checked locally; not yet run on a runner.
- [x] CI: `clippy` job across the feature matrix (default, `--no-default-features`, `--features web`). Checked locally; not yet run on a runner.
- [x] `cargo deny check advisories` added to the `deny` job. Passes locally.
- [x] Dependabot for GitHub Actions and the `fuzz/` crate (`.github/dependabot.yml`). Not yet run on GitHub.
- [x] Pre-commit hook (`.githooks/pre-commit`) for `cargo fmt --check` and clippy. Enabled with `git config core.hooksPath .githooks`. Run once locally; the hook file's executable bit is not set on Windows.

### Innovative
- [x] `tpt-css` CLI: `check` prints diagnostics for a stylesheet; `compute` prints computed styles for a JSON fixture tree. Built as `src/bin/tpt-css` with no dependencies (DESIGN.md decisions 40 and 50, `docs/CLI.md`). `inspect` prints the winning declaration of each property. Tests: `tests/cli.rs`.
- [x] WASM playground: CSS in, diagnostics, computed styles, and winning declarations (`Inspect`, mode 3) out, on a static page (`playground/`, DESIGN.md decisions 40 and 50). The module is built for `wasm32-unknown-unknown` in CI. Its exports, including mode 3, were smoke-tested in Node. The page itself was not opened in a browser.
- [x] Generated support table from `properties/registry.rs`, in `docs/SUPPORT.md` (DESIGN.md decision 42). CI fails when it is stale. Not done: test names in the table, and publishing it from CI.
- [x] Computed-style snapshots for the fixtures, with a diff printed in CI when they change (`fixtures/snapshots/`, DESIGN.md decision 41).

### Usability findings
- [x] A misspelled property name (for example `colr: blue`) is dropped with no diagnostic. Decided (DESIGN.md decision 36): an `info` diagnostic with code `UnknownProperty` is reported, and the cascade still drops the declaration. Checked by `tests/examples.rs`.

### Missing features that block real pages
- [x] Parse `box-shadow`, `transform`, `transition`, `flex`, and `grid` as pass-through values, so a layout engine downstream can read them. `box-shadow` and `transform` are typed now (decision 51). `transition`, `animation`, and `flex` were typed later (decision 62). `grid` stays pass-through. Layout itself stays out of scope. Done as `Grammar::PassThrough` (decision 37). Their grammars are not checked here.
- [x] `background-image` and `content` counters (see section 3). `background-image` is a typed list of `none`, `url()`, and image functions, kept as text (decision 37). Counters are done (decision 38). Not done: images are not loaded, `list-item` is done (decision 43), and `counter-set`/`counter-reset` on `::marker` are not special-cased.
- [x] A DOM adapter, so the web profile has an end-to-end test (see section 5). Decided (DESIGN.md decision 39): no HTML parser in the engine. The adapter is `TestTree` (tests/common) fed by a small markup reader (tests/common/markup.rs), which gives the end-to-end test. A product DOM adapter waits for the `tpt-dom`/`tpt-html` crates.
