# Changelog

All notable changes to tpt-css are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The crate is not yet
published (`publish = false` in `Cargo.toml`), so there are no released versions.

## Unreleased

### Added

- `examples/` with `minimal`, `dark_mode`, `diagnostics`, `media_and_container`,
  and `cssom_edit` (feature `cssom`). `tests/examples.rs` checks their output.
- `UnknownProperty` diagnostic at `info` severity for a property name that is not
  in the registry, is not a shorthand, and is not a custom property (DESIGN.md 36).
- Pass-through properties `box-shadow`, `transform`, `transition`, `flex`, and
  `grid`, kept as canonical text (`Value::Raw`, DESIGN.md 37).
- `background-image`: `none`, or a list of `url()` and image functions, kept as
  text (DESIGN.md 37).
- `tpt-css` command line: `check` and `compute` (`docs/CLI.md`, DESIGN.md 40). It has
  no dependencies, and `compute` takes `--css` for extra stylesheets.
- WASM playground in `playground/` (DESIGN.md 40), built in CI for `wasm32-unknown-unknown`,
  with an Inspect button for the winning declarations (DESIGN.md 50).
- Computed-style snapshots for `fixtures/mit/` (DESIGN.md 41) and a generated support
  table in `docs/SUPPORT.md` (DESIGN.md 42). CI fails when either is stale.
- `background` shorthand: one image and one color (DESIGN.md 43).
- `attr()` with the `string` type and a string fallback (DESIGN.md 43).
- Implicit `list-item` counter for `display: list-item` elements (DESIGN.md 43).
- `ParseOptions::preserve_comments` and `ParsedStylesheet::comments` (DESIGN.md 44).
- `:current()`, `:past()`, and `:future()` functional forms (DESIGN.md 45).
- Media-state pseudo-classes `:muted`, `:buffering`, `:stalled`, `:seeking`, and
  `:volume-locked` (DESIGN.md 49).
- `color-mix()` hue methods `increasing` and `decreasing` (DESIGN.md 48).
- `color-mix()` and `color()` in `display-p3`, `rec2020`, `a98-rgb`, and
  `prophoto-rgb`, clipped to sRGB in the result (DESIGN.md 53, 58).
- `font-variant-ligatures`, `font-variant-numeric`, `font-variant-east-asian`,
  `font-variant-position`, `font-variant-emoji`, and `font-variant-alternates`, and
  the full `font-variant` shorthand over them (DESIGN.md 59).
- A seeded smoke test of the six fuzz-target bodies, and a test that every initial
  value round-trips through its grammar (DESIGN.md 60, 61).
- Typed time and angle values, `transition-duration`, `transition-delay`,
  `animation-duration`, and `animation-delay` (DESIGN.md 51).
- `tpt-css inspect <tree.json>` and the playground's Inspect mode: the winning
  declaration of each property, with origin, layer, and specificity (DESIGN.md 50).
- `ResolverConfig::share_styles` (off by default): sibling style sharing (DESIGN.md 35, 46).
- Test support: a small markup reader (`tests/common/markup.rs`) and an end-to-end
  web test under `--features web` (DESIGN.md 39). Not an engine parser.
- Counters: `counter-reset`, `counter-set`, `counter-increment`, and the
  `counter()` and `counters()` functions in `content`, with the decimal, alphabetic,
  and roman styles (DESIGN.md 38). A rule that uses counters makes `restyle` do a
  full resolve.
- `examples/common/mod.rs`: a shared arena tree that implements `StyleTree`.
- Issue templates for bug reports, specification gaps, and feature requests.
  Contributions are made through issues.
- `docs/STATUS.md`: what works, what is partly done, and what is missing. The
  milestone status moved out of the README.
- `.githooks/pre-commit`: runs `cargo fmt --check` and clippy. Enable it with
  `git config core.hooksPath .githooks`.
- `.github/dependabot.yml`: weekly updates for GitHub Actions and the `fuzz/`
  dependencies.
- CI: a `doc` job (`cargo doc` with warnings as errors) and a clippy matrix
  across the feature set. The `deny` job now also checks advisories.

### Changed

- `background` takes comma-separated layers, with six per-layer longhands
  (`background-position`, `background-size`, `background-repeat`,
  `background-attachment`, `background-origin`, `background-clip`) (DESIGN.md 65).
- Typed `attr()` in any property: units and `type()` syntaxes, with fallbacks, read at
  computed-value time (DESIGN.md 64).
- `filter` is a typed property: filter functions with their defaults, and a list
  that names a `url()` kept as text (DESIGN.md 63).
- `transition`, `animation`, and `flex` are typed shorthands with typed longhands
  (`transition-property`, `transition-timing-function`, `animation-name`,
  `animation-timing-function`, `animation-iteration-count`, `animation-direction`,
  `animation-fill-mode`, `animation-play-state`, `flex-grow`, `flex-shrink`, and
  `flex-basis`), not pass-through text (DESIGN.md 62). Only `grid` is pass-through.
- `font-variant` takes the ligature, numeric, and other sub-values, which made the
  shorthand invalid before. `font-variant: none` now sets ligatures to `none` and the
  rest to `normal`, as the spec says (DESIGN.md 59).
- `box-shadow` and `transform` are typed values, not pass-through text. Hosts that
  read their `Value::Raw` text must read the typed values now (DESIGN.md 51).
- An environment change restyles only the elements it can reach, instead of
  everything (DESIGN.md 52). `StyleResolver::restyle` results are unchanged.
- `:dir()` falls back to `ltr` when no ancestor has a direction (DESIGN.md 47).
- The README example is accurate: its snippet points to `examples/minimal.rs`.
- `Cargo.toml`: the `cssom` feature comment no longer calls it a placeholder.

### Fixed

- Nested comma lists could grow exponentially with nesting depth (a fuzzer
  out-of-memory on a 266-byte input, about 1.9 s to parse). A nested rule whose
  resolved selector list exceeds the new `max_resolved_selector_size` limit
  (default 1024 nodes) is now dropped with a `LimitExceeded` diagnostic.
- Three broken rustdoc links that failed `cargo doc` with warnings as errors.
- The `fuzz-smoke` CI job failed at build time on Linux: cargo-fuzz defaults to the
  musl target, which the runner lacks. The job now passes the gnu target explicitly.
