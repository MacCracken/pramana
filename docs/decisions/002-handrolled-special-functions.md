# 2. Hand-Rolled Special Functions

Date: 2026-03-28
Status: Accepted

## Context

Several modules (distributions, hypothesis tests, regression) require special
mathematical functions: the error function (`erf`), the log-gamma function
(`ln_gamma`), the regularized incomplete beta function, and the regularized
incomplete gamma function.

Options considered:

1. **Pull in `statrs` or `special`** -- heavy transitive dependency trees;
   `statrs` alone brings in `nalgebra`. Conflicts with the goal of depending on
   hisab for linear algebra.
2. **Use hisab** -- hisab provides core linear algebra and constants but does not
   implement special functions.
3. **Hand-roll in `math.rs`** -- implement the four functions ourselves using
   well-known numerical recipes.

## Decision

Implement special functions in `math.rs` (marked `pub(crate)`) using classical
algorithms:

| Function          | Algorithm                         | Reference              |
|-------------------|-----------------------------------|------------------------|
| `erf` / `erfc`    | Abramowitz & Stegun 7.1.26        | A&S (1964), eq. 7.1.26 |
| `ln_gamma`        | Lanczos approximation (g = 7)     | Lanczos (1964)         |
| `regularized_incomplete_beta`  | Lentz continued fraction | DLMF 8.17.22  |
| `regularized_incomplete_gamma` | Lentz continued fraction + series | DLMF 8.11   |

## Consequences

**Positive**

- Zero additional dependencies beyond hisab.
- Full control over precision and edge-case handling.
- Functions are `pub(crate)`, so they do not pollute the public API.

**Negative**

- We own the numerical accuracy -- must maintain comprehensive tests against
  known reference values (SciPy, Mathematica, DLMF tables).
- Future special functions (e.g., Bessel) will need the same treatment.
