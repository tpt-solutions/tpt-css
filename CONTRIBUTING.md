# Contributing to tpt-css

Contributions are made through GitHub issues. Please open an issue to report a
bug, point out a gap in the specification coverage, or propose a feature. The
maintainers do the implementation work from there.

## Opening an issue

Use the issue templates. Each one asks for what is needed to act on the report:

- **Bug report:** the CSS input, the tree shape if it matters, the output you
  expected, and the output you got. A minimal stylesheet and tree is ideal.
- **Specification gap:** the specification section (for example `spec 13.3` or
  a CSS Cascade 5 section), the behavior you expected, and a link to the text.
  If the engine is wrong against the spec, say so and cite the sentence.
- **Feature request:** the CSS feature, the spec it comes from, and the use case
  it serves.

Search existing issues first. Check [TODO.md](TODO.md) too: many gaps are
already listed there, with their status.

## The clean-room rule

Reports must not include implementation code copied from other CSS engines,
or code from copyleft sources. Describe the behavior and cite the public
specification instead. The rules are in [CLEAN_ROOM.md](CLEAN_ROOM.md).

## Building and testing locally

You do not need to build the project to file an issue. If you want to check a
behavior yourself, the toolchain is pinned in `rust-toolchain.toml`, and the
minimum supported Rust version is 1.75.

```sh
cargo test
cargo run --example minimal
```

To run the format and lint checks before each commit, enable the hook once per
clone:

```sh
git config core.hooksPath .githooks
```

Before you open an issue about a failing test, run `cargo test` and include
the failing test name and the output.

## Background reading

- [README.md](README.md): what the engine does and does not do.
- [DESIGN.md](DESIGN.md): design decisions and deviations from the specification.
- [spec.txt](spec.txt): the design specification the engine follows.
