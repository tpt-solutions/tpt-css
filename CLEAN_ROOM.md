# Clean-room policy and provenance log

`tpt-css` is written clean-room, spec-first, as spec.txt section 23.1 requires.

## Rules

Contributors must not:

- copy code from other CSS engines
- port code from other CSS engines
- translate C++, Rust, or JS implementation code from other engines
- wrap existing CSS parsers or style engines
- paste implementation code from Stack Overflow or other copyleft sources
- use AI-generated code that reproduces non-MIT implementation code

Contributors should:

- read public specifications
- write their own algorithm notes
- write their own Rust implementation
- write their own MIT tests
- cite spec sections in doc comments
- add a row to the log below for each contributor or AI-assisted batch of work

External test suites (for example web-platform-tests) may be used as black-box
oracles. Their source or assets are not vendored into this tree (spec 22.2).

## Provenance log

| Date | Contributor | What was written | Sources | Review status |
| --- | --- | --- | --- | --- |
| 2026-11-26 | Repository owner, with an AI coding assistant | M0 foundation: syntax layer, AST, parser, Tier 0 selectors, cascade, Tier 0 values, property registry (37 properties), style resolution, tests, and project files. | `spec.txt` in this repository, and public CSS specifications at the URLs cited in the code (CSS Syntax 3, Selectors 4, CSS Cascade 5, CSS Values, CSS Color 4, CSS Fonts 4, CSS Box 4, CSS Backgrounds 3, CSS Display 3, CSS Sizing 3, CSS Text 3). No code from other engines was consulted or copied. | Not yet reviewed by a human. Must be reviewed before merge. |
| 2026-11-26 | Repository owner | `LICENSE-APACHE` is the standard, unmodified Apache-2.0 text, copied from a locally installed crate's `LICENSE-APACHE` file. `LICENSE-MIT` is the standard MIT text with the TPT Solutions copyright line. | Standard license texts. | Copied text, not new code. |
| 2026-11-26 | Repository owner, with an AI coding assistant | M1 batch: Tier 1 selectors, pseudo-elements, custom properties and `var()`, math functions, length units, colors (`hsl()`, `hwb()`, 4- and 8-digit hex), shorthands, `@media`, the M1 property set, and the invalidation foundation. | The same public CSS specifications as the M0 batch, plus the spec URLs cited in the code (Media Queries 5, Position 3, Overflow 3, Generated Content 3, Inline 3, Writing Modes 4). No code from other engines was consulted. | Not yet reviewed by a human. Must be reviewed before merge. |
| 2026-11-26 | Repository owner, with an AI coding assistant | M2 batch: cascade layers, `revert` and `revert-layer`, `all`, `@supports`, CSS nesting, `@container` with a size provider, `@property`, stored `@keyframes` and `@font-face`, `light-dark()`, modern state selectors, and the visited-link policy. | The same public CSS specifications, plus the spec URLs cited in the code (Cascade 4 and 5, Conditional 3, Nesting 1, Containment 3, Properties and Values, Animations 1). No code from other engines was consulted. | Not yet reviewed by a human. Must be reviewed before merge. |

| 2026-11-26 | Repository owner, with an AI coding assistant | M2 leftovers and M3/M4 batch: `:has()` and `@scope` matching, equality-cutoff restyle, `inspect()`, `getComputedStyle` property list, CSSOM editing and serialization (feature `cssom`), keyframe interpolation, numeric math functions, `lab()`/`lch()`/`oklab()`/`oklch()`/`color()`/`color-mix()` (srgb), the full 148 named colors, and the AppFront, property, perf, and spec-trace tests. | The same public CSS specifications, plus Color 4 (matrices for Lab, LCh, OKLab, and D50 Bradford adaptation), Values 4 (math functions), Animations 1 (interpolation), and Scoping 1. No code from other engines was consulted. | Not yet reviewed by a human. Must be reviewed before merge. |
| 2026-11-26 | Repository owner, with an AI coding assistant | Next batch: scope proximity (Cascade 6) in the cascade key, the calc nesting limit and stylesheet count limit, the `prefers-contrast`, `dynamic-range`, `update`, and `resolution` media features, a one-copy restyle, and shorthand and calc property tests. | The same public CSS specifications, plus Media Queries 5 (the four media features) and the key in `spec.txt` 13.3 for proximity ordering. No code from other engines was consulted. | Not yet reviewed by a human. Must be reviewed before merge. The unscoped-rule proximity ordering is a reading of Cascade 6 that needs checking against the spec text. |
| 2026-11-26 | Repository owner, with an AI coding assistant | Batch: relative color syntax (CSS Color 5) for seven functions with forward conversions from sRGB, attribute-named invalidation (`AttributeNamed`), `restyle` returning a reference into the cache, and scope proximity ordered after specificity to match the Cascade 6 draft. | Public CSS specifications: CSS Color 5 (relative syntax), the Cascade 6 draft at drafts.csswg.org (sorting order and proximity), and the Color 4 and OKLab matrices already in the repository. No code from other engines was consulted. | Not yet reviewed by a human. Must be reviewed before merge. The `spec.txt` 13.3 key needs correcting to match the draft. |

## Notes on the log

- The log names roles, not personal details. Add the contributor's handle
  only if they agree to it.
- "AI-assisted" rows must say which parts were generated and must be reviewed by
  a person who knows the specification before they are merged.
