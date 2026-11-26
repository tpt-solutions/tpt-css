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
- WASM playground in `playground/` (DESIGN.md 40), built in CI for `wasm32-unknown-unknown`.
- Computed-style snapshots for `fixtures/mit/` (DESIGN.md 41) and a generated support
  table in `docs/SUPPORT.md` (DESIGN.md 42). CI fails when either is stale.
- `background` shorthand: one image and one color (DESIGN.md 43).
- `attr()` with the `string` type and a string fallback (DESIGN.md 43).
- Implicit `list-item` counter for `display: list-item` elements (DESIGN.md 43).
- `ParseOptions::preserve_comments` and `ParsedStylesheet::comments` (DESIGN.md 44).
- `:current()`, `:past()`, and `:future()` functional forms (DESIGN.md 45).
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

- The README example is accurate: its snippet points to `examples/minimal.rs`.
- `Cargo.toml`: the `cssom` feature comment no longer calls it a placeholder.

### Fixed

- Three broken rustdoc links that failed `cargo doc` with warnings as errors.
