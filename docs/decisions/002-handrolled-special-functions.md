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

## Amendment (2026-11-14)

The P(-1) hardening sweep found the original implementations too imprecise or
truncated for statistical use, and replaced them while keeping the decision
(no new dependencies):

| Function | Change | Why |
|----------|--------|-----|
| `erf` / `erfc` | Port of FreeBSD msun `s_erf.c` (fdlibm, via the MIT-licensed `libm` crate; Sun notice preserved in `math.rs`) | A&S 7.1.26 has 1.5e-7 absolute error and ~40% relative error in the tails (e.g. `Φ(-6)` off by 0.4%), which leaks straight into p-values |
| Incomplete gamma | Returns `(P, Q)` with each tail computed directly; iteration budget `200 + 20·√a` (was a fixed 200); Stirling/`ln_1p` prefactor; Wilson–Hilferty above `a = 1e9` | `ChiSquared(1e6).cdf(1e6)` returned 0.11 instead of 0.50 (series truncated); `1 - P` lost tiny upper tails |
| Incomplete beta | Returns `(I, 1 - I)`; callers pass `x` and `1 - x` separately; same iteration budget; Loader/TOMS 708-style `bd0` prefactor when both shapes ≥ 10 | Upper-tail F p-values rounded to 0; large-shape prefactor cancellation (~1e-10 relative error at n = 10⁶) |
| `ln_beta` | Stirling-remainder form for large arguments (R's `lbeta` scheme) | `lnΓ(a) + lnΓ(b) - lnΓ(a+b)` cancels for large `a` (Student-t with large df) |
| New helpers | `stirling_error`, `bd0`, `ln_gamma_kernel`, `normal_cdf`, `normal_sf` | O(1), cancellation-free Poisson/Binomial PMFs and normal tails |

Reference values in the tests come from glibc `erf`/`erfc`, closed forms
(Cauchy, `t₂`, `F(2, ν)`, `I_x(a, 1)`), and 50-digit decimal Poisson sums.

