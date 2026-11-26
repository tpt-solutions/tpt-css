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
    `hidden`** (CSS Backgrounds 3 section 4.3). This is the only cross-property
    dependency in M0 computation.
16. **`bolder` and `lighter` thresholds.** These follow the table in CSS Fonts 4
    section 2.2 as recorded in `style/computed.rs`. The table is not yet checked
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
