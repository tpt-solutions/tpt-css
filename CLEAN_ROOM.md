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

## Notes on the log

- The log names roles, not personal details. Add the contributor's handle
  only if they agree to it.
- "AI-assisted" rows must say which parts were generated and must be reviewed by
  a person who knows the specification before they are merged.
