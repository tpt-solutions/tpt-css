# tpt-css design notes

This file records the decisions made while implementing M0, the answers to the
open questions in [spec.txt](spec.txt) section 25, and every place where the
code deliberately differs from the sketches in the spec. The spec is still the
source of truth. When this file and the spec disagree, this file says why.

## 1. Pipeline

```text
CSS text
  -> syntax::tokenize            tokens with byte spans
  -> syntax::build_component_values   component values (iterative stack)
  -> parse::rules / declarations / selectors   stylesheet AST
  -> selectors::matching         right-to-left matching, step budget
  -> cascade                     origin, importance, inline, specificity, order
  -> properties::parse           grammar, CSS-wide keywords
  -> style::resolver             inheritance and computed values
  -> style::StyleMap             node id -> ComputedStyle
```

## 2. Decisions for spec section 25 (open questions)

| Spec | Question | Decision | Status |
| --- | --- | --- | --- |
| 25.1 | Numeric type | `f64` for all computed numbers. Source text is not kept for serialization. A decimal or rational type can replace it later behind `values::Value`. | Decided for M0 |
| 25.2 | Atom interning | No interner in M0. Names are owned `String`s and keywords are `&'static str` from grammar tables. Revisit in M1, when selector matching needs hot-path comparisons. | Decided, deferred |
| 25.3 | `StyleTree` return types | Slices for `children`, references for `attributes` and `state`, and `Option<&[Declaration]>` for inline style. Iterators and arena handles were rejected because slices are simpler and enough for M0. | Decided |
| 25.4 | Pseudo-elements | Side tables keyed by node plus pseudo-element, not generated nodes in the tree. `NodeKind::Pseudo` exists so generated nodes can be added later without breaking the contract. | Decided, implemented in M1 |
| 25.5 | Font metrics | The CSS engine does only font-relative arithmetic (`em`, `%` in `font-size`, `rem`). Metrics, shaping, and font loading belong to the layout or font system. | Decided |
| 25.6 | Container size requests | Size containers only. The style engine asks a `ContainerSizeProvider` during restyle, and the host guarantees that the container's size was computed by an earlier layout pass. No cycle is possible because container queries never change the size of their own container. | Decided, implemented in M2 |
| 25.7 | Animation interpolation | A separate crate (`tpt-animate`, name provisional). `tpt-css` only places animation and transition declarations in the cascade (spec 13.2). | Decided |
| 25.8 | CSSOM round-trip fidelity | Canonical, deterministic serialization in M3. Byte-for-byte round trips of the author's text are not required. | Decided, implemented in M3 |
| 25.9 | Web UA stylesheet location | `tpt-html`, not `tpt-css`. The core has no HTML names (spec 4.3). | Decided |
| 25.10 | Dependency policy | Already resolved by the spec. Zero dependencies in the core. Every allowed dependency must offer MIT. `deny.toml` enforces this. | Resolved |

## 3. Deviations from the spec sketches

Each deviation is deliberate and is listed here so it can be reviewed.

1. **`ParseOptions` has no `origin`, `quirks`, `preserve_spans`, or
   `preserve_comments`.** Origin is passed to `add_stylesheet`, which is
   where it is used. Spans are always kept. Quirks mode is an HTML concept and
   does not belong in the core. Comment preservation is not implemented yet.
2. **`Stylesheet` has no `diagnostics` field.** Diagnostics are returned in
   `ParsedStylesheet`, so the AST holds only syntax.
3. **`Stylesheet` has no `namespaces` or `layers` field yet.** Namespaces are
   M1 and layers are M2.
4. **Registry rows have no `id` field.** The id is the row index, and
   `properties::registry::ids` names the rows the engine uses. A unit test checks
   that every named id matches its row.
5. **Registry rows have no `animatable` or `layout_effect` field.** They are
   added in M2 and M3.
6. **`ComputedStyle::font_size()` returns `f64` px, not `Length`.** Layout does
   not need the unit once everything is absolute.
7. **Computed lengths are clamped to the finite `f64` range.** Inheritance can
   compound `em` across a deep tree, and an infinite length would leak into
   layout. NaN becomes 0 and infinities clamp to `f64::MAX`. This is a
   deliberate choice for hostile input (spec 19.1, 20.3).
8. **A component nesting violation drops the whole stylesheet.** Component
   values are built for the whole input at once. Recovering per rule would need
   the builder to be restartable, which is M1 work.
9. **Matching uses a step budget.** When it runs out, the selector does not
   match. This fails closed (spec 20.3).
10. **`StyleResolver::resolve` takes `&mut self`** as in spec 8.2, even though M0
    keeps no caches. The signature is kept so that M3 caches do not change the
    API.
11. **No `cascade/collect.rs`, `cascade/specificity.rs`, `style/inheritance.rs`,
    `style/custom.rs`, or `values/number.rs` files.** Collection and inheritance
    live in `style/resolver.rs`. Specificity lives in `selectors/specificity.rs`,
    which the cascade uses. Numbers are handled by the `Value::Number` variant
    and by the tokenizer. The file layout can be split later when those modules
    grow. Some items in the spec layout were never needed.
12. **Initial value of `min-width` and `min-height` is `0px`.** The CSS Sizing 3
    initial value is `auto`, but `auto` is not accepted in M0. This keeps the
    CSS 2.1 behavior until layout needs the flexbox form (M1 or later).
13. **Initial value of `font-family` is `serif`.** The spec leaves the initial
    value to the user agent. The core needs a concrete generic.
14. **Initial value of `color` is opaque black.** Same reason as above.
15. **`border-*-width` computes to 0 when the matching style is `none` or
    `hidden`** (CSS Backgrounds 3, border-width computed value). This is the only cross-property
    dependency in M0 computation.
16. **`bolder` and `lighter` thresholds.** These follow the table in CSS Fonts 4
    (relative weights) as recorded in `style/computed.rs`. The table is not yet checked
    against the spec text, and [SPEC_TRACE.md](SPEC_TRACE.md) records that.

## 4. Known M0 gaps

These are not bugs. They are features that the milestones put later.

- Custom properties (`--name`) are parsed and kept, but they do not cascade or
  substitute (M1).
- `var()`, `calc()`, shorthands (`margin`, `padding`, `border`, `font`, and
  others), `content`, and `revert` / `revert-layer` are not supported. A value
  that uses them is dropped with no effect.
- Pseudo-elements, `:nth-*`, `:not()`, `:is()`, `:where()`, sibling
  combinators, attribute operators other than `=`, and namespaces are not
  supported. Selectors that use them are dropped and report
  `UnsupportedSelector`.
- Media queries, `@supports`, `@layer`, `@scope`, `@container`, `@property`,
  `@keyframes`, and `@font-face` are preserved as unknown at-rules and have no
  effect.
- Named colors are limited to the 16 basic colors and `orange`.
- Invalidation, restyle hints, and style sharing (M3) are not implemented.
  `resolve` always styles the whole tree.

## 5. Module map

Where the spec sketch names a file that does not exist, the functionality lives
in the module listed here.

| Functionality | Module |
| --- | --- |
| Tokens, spans, component values | `src/syntax/` |
| Stylesheet, rule, declaration, and selector AST | `src/ast/` |
| Parsing and diagnostics | `src/parse/` |
| Selector specificity, matching, pseudo-element identities | `src/selectors/` |
| Cascade origin, order, and key | `src/cascade/` (collection in `src/style/resolver.rs`) |
| Length, color, and value serialization | `src/values/` |
| Property registry and grammar parsing | `src/properties/` |
| Computed style, computed-value rules, style map | `src/style/computed.rs` |
| Inheritance, CSS-wide keywords, and resolution | `src/style/resolver.rs` |
| Abstract tree contract | `src/tree/` |
| Environment | `src/env/mod.rs` |
| Resource limits | `src/limits.rs` |

## 6. M1 decisions and deviations

M1 adds Tier 1 selectors, units and math functions, custom properties and
`var()`, shorthands, `@media`, pseudo-element styles, the M1 property set, and
the invalidation foundation. Section 4 lists the M0 gaps; the list below replaces
it for M1.

### Decisions

1. **Pseudo-elements are side tables.** `StyleMap::get_pseudo` looks up a
   `(node, pseudo)` key. A pseudo-element is generated only when its `content` is
   not `none` or `normal`. This resolves spec 25.4 in favor of side tables.
2. **Custom properties are resolved per element at computed-value time.** Each
   custom property is resolved with a memo and a stack of names in progress. A
   cycle makes every property in it invalid. An invalid or missing property is
   absent from the element's map, so a `var()` that reads it uses its fallback.
3. **`var()` is substituted before parsing.** A declaration that contains `var()`
   is kept as tokens and parsed after substitution. For a shorthand, the whole
   value is kept, expanded after substitution, and each longhand takes its own
   part. This follows the pending-substitution approach of spec 15.3.
4. **Shorthands are expanded before the cascade.** Each longhand then competes
   on its own. A shorthand reset (for example a `border` without a color) sets
   the missing longhands to their initial values.
5. **Math functions keep units until computed time.** The parser type-checks each
   expression (`10px + 2` is invalid). At computed time, lengths become px and
   linear forms collapse to `Linear { px, percent }`. Percentages stay symbolic
   for layout, except in `font-size`.
6. **Non-negative clamping waits for percentages.** `calc(50% - 10px)` on a
   non-negative property keeps its negative px part, because layout resolves the
   percentage first.
7. **Media conditions are evaluated per resolve call.** `active_rules` computes
   one boolean per rule. Matching then runs only for active rules. Unknown
   features never match, and `not` inverts the whole query.
8. **Invalidation is conservative.** Attribute, state, and inline changes restyle
   the changed element and its subtree. If any rule depends on sibling structure,
   the parent's subtree is also restyled, which covers every sibling. Stylesheet
   and tree changes restyle everything. A precise dependency index is M3 work.
9. **`restyle` returns a copy of the cached map.** This keeps the API of spec 8.2
   at the cost of one clone per call. A shared map is M3 work. (Superseded by decision 16 in section 8.)
10. **`StyleTree::text` has a default.** Adapters that do not supply text get the
    empty string, so `:empty` treats them as empty.

### Deviations and limits

- `calc()` nesting uses a fixed `DEFAULT_CALC_DEPTH` (32), not a `ParseLimits`
  field. Making it configurable is a small follow-up.
- `content` accepts strings only. `attr()` and counters are not supported.
- `background` covers only `background-color`. A declaration with an image is
  dropped. `font`, `list-style`, and `text-decoration` are not implemented.
- Named colors are still the 16 basic colors plus `orange`.
  `lab()`, `lch()`, `oklab()`, `oklch()`, and `color-mix()` are not implemented.
- Angle, time, and resolution units are accepted only where they matter here
  (the hue of `hsl()` and `hwb()`).
- Structural pseudo-classes scan the sibling list. A wide parent costs
  O(siblings) per child, so a child list of n elements costs O(n²) in the worst
  case. An index is M3 work.

### Still not implemented at the end of M1

- Shorthands `font`, `list-style`, `text-decoration`, and the full `background`.
- Named color table (148 names), and the modern color spaces.
- Tier 2 selectors (`:has()`, `:focus-visible`, and the rest), `@layer`,
  `@supports`, `@container`, `@scope`, `@property`, nesting, and `revert`.
- Precise invalidation (M3) and CSSOM (M3).

## 7. M2 decisions and deviations

M2 adds cascade layers, `revert` and `revert-layer`, `all`, `@supports`, CSS
nesting, `@container` with a host-supplied size provider, `@property`, stored
`@keyframes` and `@font-face`, `light-dark()`, the modern state pseudo-classes,
and the visited-link policy. The items still missing are listed at the end.

### Decisions

1. **Candidates are kept until resolution.** The cascade collects every candidate
   declaration per property, and winners are picked in `resolve_winner`. `revert`
   needs the lower-origin candidates, which a single pre-picked winner cannot
   provide.
2. **Layers are ranks in the cascade key.** Normal declarations rank later layers
   higher. Important declarations rank earlier layers higher, and unlayered
   important declarations rank lowest (spec 13.4). The key order is origin and
   importance, then layer, then inline, then specificity, then order.
3. **Layer order is the order of first declaration.** A statement such as
   `@layer a, b;` fixes it. Nested layers are named with dots, so `a.b` declared
   inside `a` is the same layer as a top-level `a.b`. Anonymous blocks each get
   their own layer.
4. **`revert` pools by origin, `revert-layer` by layer.** `revert` takes the best
   candidate of a lower origin. `revert-layer` takes the best candidate of the same
   origin in a lower layer. With no candidate left, both are unset. For
   `revert-layer` in unlayered rules, the last named layer counts as the lower one.
5. **`@supports` is evaluated at parse time.** Its condition depends only on the
   engine's capabilities, so the rule is kept or dropped then. Support is checked
   against the real property registry, grammar, and selector parser.
6. **Nesting is resolved at parse time.** A nested selector has its `&` replaced by
   `:is(parent)`. A selector with no `&` gets an implicit descendant prefix. The
   resolved rule then matches on its own, so the matcher has no nesting case.
   Specificity follows from `:is()`, which takes the most specific argument, as
   CSS Nesting specifies.
7. **Container queries need a provider.** The host implements
   `ContainerSizeProvider`. A condition is evaluated against the nearest ancestor
   that is a container, and whose name matches if the query has one. Without a
   provider, container rules never apply. While a provider is set, restyle
   recomputes everything, because size changes are not tracked by invalidation.
8. **`@property` values are checked against `syntax`.** The checked syntaxes are
   `*`, `<length>`, `<number>`, `<integer>`, `<percentage>`, `<length-percentage>`,
   and `<color>`. Other syntaxes are accepted without a check. A value that fails
   the check uses the registered initial value. `inherits` decides whether an
   unset registered property inherits or takes its initial value.
9. **`all` skips `direction` and `unicode-bidi`**, as CSS Cascade 4 specifies. It
   accepts only CSS-wide keywords.
10. **`light-dark()` is resolved at computed time** from the environment's color
    scheme. The specified value keeps both colors.
11. **`:visited` follows the spec 20.2 policy.** In `Restricted` mode (the default),
    a selector with `:visited` applies only to the color properties. In `Disabled`
    mode it never matches. Custom properties are always excluded from `:visited`.
12. **Animation origin is a cascade input only.** `add_stylesheet` with
    `Origin::Animation` gives the usual precedence. Keyframes are parsed and stored,
    but nothing drives an animation yet.

### Still not implemented at the end of M2

- Cascade scope proximity, `@scope`, `:scope`, and `:has()` (all M2 items, still open).
  `::slotted()`, `::part()`, and `:host` are M4.
- Keyframe interpolation and the animation and transition drivers (spec 3 places
  drivers outside the engine).
- `color()`, relative color syntax, and `lab()`/`lch()`/`oklab()`/`oklch()`.
- Container units (`cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, `cqmax`).
- `revert` in custom properties (treated as unset).
- Style sharing and dependency-indexed invalidation (M3).

## 8. M2 leftovers, M3, and M4 decisions

This section covers the work that finished the open M2 items, and the M3 and M4
items that could be done without an external conformance suite. The items that
remain open are listed at the end.

### Decisions

1. **Calc numeric functions are nodes in the tree.** `CalcNode::Func` carries the
   function and its arguments. Arguments are type-checked as the tree is built:
   `round`, `mod`, `rem`, `abs`, and `hypot` take one kind (number or dimension),
   while `sign` and the others take numbers and return numbers. `round()` takes a
   strategy (`nearest` is the default; ties round toward positive infinity). A
   function whose inputs are constants folds to a number. An undefined result (for
   example `mod(x, 0)`) stays a tree rather than becoming an error.
2. **Container units resolve against the nearest container.** The style engine
   asks the `ContainerSizeProvider` for the nearest ancestor that reports a size.
   `cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, and `cqmax` fall back to the viewport when
   no container exists.
3. **Color spaces convert to sRGB at parse time.** `lab()` and `lch()` go through
   XYZ (D50) and the Bradford-adapted matrix. `oklab()` and `oklch()` use the
   published OKLab matrices. Out-of-gamut values are clamped. `color()` supports
   `srgb` and `srgb-linear`. `color-mix()` supports `in srgb` with optional
   percentages; interpolation in other spaces is not implemented and is dropped.
4. **`:has()` is a relative selector matched against an anchor.** The matcher's
   leftmost compound is checked against the anchor with the leading combinator's
   relation. Candidates come from the element's children, descendants, or
   following siblings, as the combinator requires. A rule that uses `:has()` can
   depend on any ancestor, so an attribute change restyles everything (see 9).
5. **`@scope` finds the nearest root, then checks the limit.** The root is the
   nearest ancestor-or-self that matches the root selector (or the root element
   when there is none). The walk stops at a limit element. `:scope` refers to the
   innermost root. Cascade scope proximity (Cascade 6) is not implemented, so
   scoped rules rank by specificity alone.
6. **Keyframes interpolate per property.** Numbers, pixel lengths, percentages,
   and colors interpolate linearly. Other values change at the midpoint. The host
   supplies the progress; there is no timeline (spec 3).
7. **Inspection reports the highest candidate.** `inspect()` gives the declaration
   that wins each property, with its origin, layer, specificity, and source. It
   does not follow `revert`; the computed value gives the final answer.
8. **`getComputedStyle` is a registry-order list.** `property_values()` returns
   every registered longhand with its serialized computed value, including the ones
   left at their initial value.
9. **Invalidation is more precise, and still conservative where it must be.** The
   restyle stops at an element whose computed style is unchanged, when every rule
   matches only its subject. Such an element's children cannot change, so they are
   skipped. A rule with descendant, sibling, `:has()`, `:scope`, `@scope`, or
   container conditions disables this cutoff, and an attribute change then restyles
   the affected subtree (or everything, for `:has()` and `@scope`).
10. **CSSOM is a checked mutation layer.** `CssomSheet` applies `insert_rule`,
    `delete_rule`, `selector_text`, `set_property`, and `remove_property`. Each
    mutation validates its input and leaves the sheet unchanged on an error. The
    serializer is canonical: parsing the output and serializing again gives the same
    text. A mutated sheet reaches the resolver through `to_stylesheet()` and a
    sheet replacement, reported as `StyleChange::StyleSheetChanged`.
11. **Equality of styles is structural.** The cutoff compares whole computed styles,
    so a change that leaves every computed value equal stops the restyle.

### Measured (release build, this machine)

These numbers come from the ignored tests in `tests/integration/perf.rs`, run with
`cargo test --release --test integration -- --ignored --nocapture`. They are
engineering measurements, not claims about other machines (spec 21.1).

- Tokenizer: about 9 to 10 million tokens per second (two runs).
- Static tree of 10,000 elements: full resolve about 45 to 50 ms (two runs). A local restyle that
  changes one element with a subject-only rule restyles 1 element and takes about
  34 µs, since `restyle` returns a reference into the cache (decision 16). It was
  16 ms when the map was copied, and 32 ms when it was copied twice.

### Later additions (this batch)

12. **Scope proximity ranks after specificity**, following the sorting order of the
    current Cascade 6 draft: origin and importance, context, style attribute,
    layers, specificity, scope proximity, then order of appearance. A scoped
    rule's proximity is the number of ancestor steps from the element up to its
    scope root. An unscoped rule has infinite proximity, so among equally specific
    rules a scoped one wins whatever the source order. `spec.txt` 13.3 lists the
    proximity field before specificity. That disagrees with the draft, and
    `spec.txt` needs correcting. The code follows the draft.
13. **Calc nesting is checked against `max_calc_depth`** where declarations enter
    the engine: at stylesheet parse (both top-level and nested rules) and after
    `var()` substitution at computed time. The value parser's own bound stays at
    32, so a configured value above 32 does not raise it.
14. **Stylesheet count is bounded** by `max_stylesheets`. `try_add_stylesheet`
    reports the limit; `add_stylesheet` keeps its signature and returns an id that
    matches no sheet when the limit is reached.
15. **Media features added:** `prefers-contrast`, `dynamic-range`, `update`, and
    `resolution` (converted to `dppx`, with `dpi` and `dpcm` supported). The
    environment gained the fields these read, with defaults that match a screen
    at 1 dppx with no contrast preference. `MediaEnvironment` is no longer `Eq`,
    because the pixel ratio is a float.

16. **`resolve` and `restyle` return a reference into the cache.** Neither copies the
    map. A caller that keeps a result across a later call clones it. Before this
    change, `resolve` copied the map on the way out. A one-element local restyle on
    10,000 elements takes about 34 µs in release, down from about 16 ms when the
    map was copied.
17. **Relative color syntax (CSS Color 5)** works for `rgb`, `hsl`, `hwb`, `lab`,
    `lch`, `oklab`, and `oklch`. The origin color supplies the channel values in
    its own space, and the keywords bind to them. Each channel is one component
    value: a keyword, `none`, a number, a percentage, or a `calc()`, `min()`,
    `max()`, or `clamp()` over those. The alpha defaults to the origin's alpha.
    The forward conversions from sRGB are the inverses of the existing ones, and
    the tests check that each color round-trips through each space.
18. **Attribute-named invalidation.** `StyleChange::AttributeNamed(node, name)`
    restyles only when a selector can read the attribute: `#id` and `.class`, an
    attribute selector, or one of those inside `:not()`, `:is()`, `:where()`,
    `:has()`, and `@scope` roots and limits. `style`, `lang`, and `dir` are always
    relevant, since style reads them directly. The name set is computed on demand
    and reset when sheets change. `StyleChange` is no longer `Copy`, because the
    name is owned.

19. **`color-mix()` interpolates in eight spaces**: `srgb`, `srgb-linear`, `hsl`,
    `hwb`, `lab`, `lch`, `oklab`, and `oklch`. Alpha is premultiplied for the
    non-hue channels, so a transparent color does not tint the result. Hue
    interpolation takes the `shorter` arc by default, or the `longer` arc when asked.
    Other hue methods, and `xyz` and wide-gamut spaces, are not implemented. A
    `longer` method in a space without a hue is rejected.
20. **Relative `color(from ...)`** works for `srgb` and `srgb-linear`. The keywords
    `r`, `g`, and `b` stand for the origin's channels in that space.
21. **Long-tail state pseudo-classes** `:current`, `:past`, `:future`, `:defined`,
    `:local-link`, `:playing`, `:paused`, `:fullscreen`, and `:autofill` match the
    state bits the host sets. Nothing sets them by default, so `:defined` matches
    nothing until the host reports the elements it considers defined. The
    functional forms, `:current(selector)` and the others, are not implemented.

### Still open

- Scope proximity is implemented (decision 12). The order matches the Cascade 6 draft. `spec.txt` 13.3 still needs correcting.
- Color spaces beyond the ones in decision 19 (`xyz`, `display-p3`, and the others),
  and the hue methods other than `shorter` and `longer`.
- `StyleChange::AttributeChanged` (no name) is still conservative. Use
  `AttributeNamed` to skip attributes that no selector reads.
- Container units across nested containers: only the nearest size container is used.
- Shadow DOM: `::part()`, `::slotted()`, `::highlight()`, `:host`, `:host-context()`.
- Style sharing, a dependency graph for custom properties, and a dependency index
  for selectors (the cutoff and the conservative rules are the current approach).
  `AttributeNamed` to skip attributes that no selector reads.
- Fuzz targets and coverage-guided fuzzing. The randomized tests are seeded and
  deterministic, which is not the same thing.
- The WPT subsuites, which need a separate checkout (spec 22.2).
- Calc depth: the value parser keeps its own fixed bound of 32. The configured `max_calc_depth` is enforced on top of it (decision 13).
