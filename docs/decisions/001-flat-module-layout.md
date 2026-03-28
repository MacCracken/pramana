# 1. Flat Module Layout

Date: 2026-03-28
Status: Accepted

## Context

Pramana is a statistics and probability library with 12 source modules (14 files
including `lib.rs` and the feature-gated `ai.rs`). Rust crates of this size can
use either a flat layout (one `.rs` file per concern beside `lib.rs`) or nested
module directories (`distribution/mod.rs`, `distribution/normal.rs`, etc.).

Nested layouts add indirection and split context across many small files. For a
library where each concern (distributions, hypothesis testing, regression, etc.)
fits comfortably in a single file, the added structure is overhead without
benefit.

## Decision

Use a flat module layout: every public concern lives in a single file directly
under `src/`. Internal helpers (`math.rs`) follow the same pattern but are
`pub(crate)`.

```
src/
  lib.rs
  error.rs
  rng.rs
  distribution.rs
  descriptive.rs
  hypothesis.rs
  regression.rs
  bayesian.rs
  combinatorics.rs
  monte_carlo.rs
  markov.rs
  timeseries.rs
  math.rs          -- pub(crate)
  ai.rs            -- feature-gated
```

## Consequences

**Positive**

- Easy navigation -- every concern is one `rg` or editor tab away.
- Minimal boilerplate -- no `mod.rs` files, no re-export chains.
- Matches the mental model of the library's public API surface.

**Negative**

- If a module grows past ~800 lines it becomes harder to navigate. At that point
  it should be split into a subdirectory (e.g., `distribution/`).
- Flat layout does not enforce sub-module visibility boundaries.
