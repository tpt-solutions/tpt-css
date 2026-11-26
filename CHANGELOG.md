# Changelog

All notable changes to tpt-css are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The crate is not yet
published (`publish = false` in `Cargo.toml`), so there are no released versions.

## Unreleased

### Added

- `examples/` with `minimal`, `dark_mode`, `diagnostics`, and `cssom_edit`
  (feature `cssom`). `tests/examples.rs` checks their output.
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
