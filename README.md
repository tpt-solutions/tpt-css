# tpt-css

A clean-room CSS engine in Rust. It takes CSS text through tokens, a stylesheet
AST, selector matching, the cascade, and property parsing to computed styles,
over any tree that implements a small `StyleTree` trait.

**Status:** M0 foundation (syntax, Tier 0 selectors, cascade, 37 core properties,
style resolution). See [TODO.md](TODO.md) for the milestone plan and
[SPEC_TRACE.md](SPEC_TRACE.md) for what is implemented, tested, and still missing.

## Design

- **Spec first.** The design is in [spec.txt](spec.txt). Decisions and deviations
  are in [DESIGN.md](DESIGN.md).
- **Clean room.** Implemented from public CSS specifications only. See
  [CLEAN_ROOM.md](CLEAN_ROOM.md).
- **No dependencies.** The core has no third-party crates. `unsafe` is forbidden.
- **Not tied to a document model.** The engine styles an abstract tree. AppFront
  and DOM are adapters, and the core has no HTML names.
- **Deterministic.** Same input, same output. No hash-map order affects results.
- **Bounded.** Input size, token count, nesting, selector work, and rule count
  are all limited and configurable (`ParseLimits`).

## Example

```rust
use tpt_css::{parse_stylesheet, Environment, Origin, ParseOptions, StyleResolver};
use tpt_css::ResolverConfig;

// `tree` is any type that implements tpt_css::StyleTree.
let parsed = parse_stylesheet("p { color: red }", ParseOptions::default());
let mut resolver = StyleResolver::new(ResolverConfig::default());
resolver.add_stylesheet(parsed.stylesheet, Origin::Author);
let styles = resolver.resolve(&tree, &Environment::default());
```

## Build and test

```sh
cargo test                       # default features (appfront)
cargo test --no-default-features
cargo test --features web
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo deny check licenses bans
```

To regenerate the tokenizer golden files after an intended change:

```sh
TPT_UPDATE_GOLDEN=1 cargo test --test golden
```

Review the diff before committing the new files.

## Layout

```text
src/        engine (see DESIGN.md section 5 for the module map)
tests/      one test target per area, plus tests/common (a test tree)
fixtures/   own MIT fixtures (fixtures/mit) and golden files (fixtures/golden)
spec.txt    design specification
```

## License

Dual licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. Copyright TPT Solutions. See [NOTICE](NOTICE).

Every first-party source file carries the SPDX identifier
`MIT OR Apache-2.0`.
