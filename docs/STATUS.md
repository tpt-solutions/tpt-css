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
| Modern colors (`color-mix()`, relative color syntax, `light-dark()`) | Done, with some hue methods and wide-gamut spaces missing |
| CSSOM editing (feature `cssom`) | Done |
| Incremental restyle | Done for local changes; some invalidation is conservative |
| Long-tail properties | Partly done (see TODO.md section 6) |
| Layout, painting, JavaScript, DOM | Out of scope for this crate |
| DOM adapter | Not built |

## Milestones

The engine was built in milestones named after the specification
([spec.txt](../spec.txt), section 24).

- **M0, syntax and basic cascade:** complete.
- **M1, practical CSS:** mostly complete. Some shorthands and properties are
  partly done.
- **M2, modern cascade:** complete.
- **M3, incremental style and CSSOM:** partly done. Style sharing is designed
  but not built. Invalidation for `:has()`, media, and container queries is
  conservative.
- **M4, long-tail compatibility:** partly done. Background images, counters,
  and several long-tail properties are missing.

## Known gaps

- Pass-through values for `box-shadow`, `transform`, `transition`, `flex`, and
  `grid` are not parsed.
- A misspelled property name produces no diagnostic. It is ignored silently.
- No DOM adapter exists, so there is no end-to-end web test.
- The fuzz targets run in CI on Linux, but the job has not been confirmed green
  on a runner.
