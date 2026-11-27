# Status

What the engine does today, what is partly done, and what is missing. The
detailed checklist is in [TODO.md](../TODO.md). Spec-by-spec traceability is in
[SPEC_TRACE.md](../SPEC_TRACE.md).

## Summary

| Area | State |
| --- | --- |
| Tokenizer, parser, and diagnostics | Done |
| Selectors (Tier 1, `:has()`, `:is()`, `:where()`, shadow selectors) | Done; long-tail (Tier 3) pseudo-classes partly done |
| Cascade: origins, layers, scope proximity, `revert` | Done |
| Custom properties, `var()`, `calc()` and math functions | Done |
| Media and container queries | Done (container size comes from the host) |
| Modern colors (`color-mix()`, relative color syntax, `light-dark()`, wide-gamut RGB spaces) | Done; spaces outside CSS Color 4 (for example `jzazbz`) are missing |
| CSSOM editing (feature `cssom`) | Done |
| Incremental restyle | Done for local changes; some invalidation is conservative |
| Long-tail properties | Partly done (see TODO.md section 6) |
| Layout, painting, JavaScript, DOM | Out of scope for this crate |
| DOM adapter | Test-only: an in-memory tree and a small markup reader under `tests/common`. No product adapter |
| Command line | `tpt-css check`, `compute`, and `inspect` (`docs/CLI.md`) |
| WASM playground | Static page and module in `playground/`. Not opened in a browser here |
| Support table | Generated in `docs/SUPPORT.md`, checked by a test |

## Milestones

The engine was built in milestones named after the specification
([spec.txt](../spec.txt), section 24).

- **M0, syntax and basic cascade:** complete.
- **M1, practical CSS:** mostly complete. Some shorthands and properties are
  partly done.
- **M2, modern cascade:** complete.
- **M3, incremental style and CSSOM:** partly done. Sibling style sharing is
  built behind `ResolverConfig::share_styles` and is off by default. Sharing
  across parents and for pseudo-elements is not done. Invalidation for `:has()`
  and container queries is conservative.
- **M4, long-tail compatibility:** partly done. Counters and background layers
  are done. Background images are kept as text, and some long-tail properties
  are missing.

## Known gaps

- The `grid` property is pass-through: it is kept as text, not parsed. `filter`
  is typed, except that a list naming a `url()` is kept as text. `transition`,
  `animation`, and `flex` are typed shorthands, and `box-shadow` and `transform`
  are typed too.
- A property name the engine does not register (a misspelling, for example)
  produces an `UnknownProperty` diagnostic at `info` severity and is ignored.
- The only DOM adapter is test support. The end-to-end web test (`--features web`) reads a small markup subset, not full HTML.
- The fuzz targets run in CI on Linux, but the job has not been confirmed green
  on a runner.
