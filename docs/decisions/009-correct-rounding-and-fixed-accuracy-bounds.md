# 9. Correct rounding, and fixed accuracy bounds for the special functions

Date: 2026-11-15
Status: Accepted (the bounds are commitments; only the maintainer can narrow a domain, by amending this ADR)

## Context

The ecosystem's accuracy policy is "better than glibc" (the maintainer, 2026-11-09; kimiya ADR 0006). The sibling
ports meet it with correctly rounded elementary functions, checked at 0 ulp against MPFR-corrected rows.

For pramana that is not enough. The measured baseline of Rust 1.3.0 against MPFR 4.2.2 (plan §3.2,
`scripts/port-baseline/`) shows that glibc's primitives are within 0.75 ulp, but pramana's own special functions are
not. Maximum errors:

| Function | Max error |
|---|---|
| `erfc` | 2.49 ulp |
| `normal_cdf`, in the tails | 1,590 ulp |
| `ln_gamma` on [0.5, 10] | 1.2×10⁶ ulp |
| `ln_gamma` beside its roots at 1 and 2 | 4.3×10¹³ ulp |
| `gamma_inc` | 246 ulp |
| `beta_inc` | 583 ulp |

These errors come from the algorithms (Lanczos, argument rounding before erfc, large-exponent prefactors, 1 − P
complements), not from glibc. Every CDF and p-value inherits them.

## Decision

1. **The elementary functions are correctly rounded** on every target: exp, ln, log1p, expm1, pow, sin, cos, tan,
   atan, log2.
   - kimiya's two-phase implementations are vendored, with atan computed as the correctly rounded atan2(x, 1).
   - log1p, tan and log2 are new, each verified on ≥ 1M MPFR arguments with 0 mismatches on three targets.
   - Where glibc misrounds, the golden expectation is Rust's operation sequence on correctly rounded primitives
     (`*.cr.tsv`), compared at 0 ulp. Matching glibc's misround is a failure.
2. **The special functions are ported twice inside 2.0.0.**
   - **M2 part a** ports `math.rs` in Rust's operation order (T0). This proves the transcription.
   - **M2 part b** replaces five functions with more accurate algorithms (T1). Their bounds are **fixed now** and
     measured against MPFR or exact rationals (`*.ref.tsv`):

| Function | Bound | Committed domain | Outside it |
|---|---|---|---|
| erf, erfc | **correctly rounded** (CORE-MATH's binary64 algorithms, ported without FMA) | every finite x | — |
| `normal_cdf`, `normal_sf` | ≤ 1 ulp | every finite z | — |
| `ln_gamma` | ≤ 2 ulp | finite x > 0, including beside 1 and 2 | x < 0 (reflection only): ≤ 2 ulp of max(\|lnΓ\|, 1) |
| `gamma_inc` P, Q | ≤ 4 ulp | a ∈ [1e-3, 1e5], x > 0 | a ≤ 1e9: no row less accurate than Rust; a > 1e9: Rust's Wilson–Hilferty kept as it is |
| `beta_inc` I, 1 − I | ≤ 8 ulp | a, b ∈ [1e-3, 1e5] | no row less accurate than Rust |

3. **Fallback for erf and erfc.** If the FMA-free CORE-MATH port does not verify within its time slot (probe P9), erf
   and erfc ship at ≤ 1 ulp, through one correctly rounded exp of a double-double argument. The port then moves to
   2.x, and this ADR is amended with the bound that shipped.
4. **Misses never loosen a bound.** A row that misses is a defect: the algorithm gets more precision. Only the
   maintainer can narrow a domain, by amending this ADR.
5. **Scope fence.** These five functions only, in `src/special.cyr`. Elsewhere Rust's operation order is the
   definition. Candidates outside the fence (compensated sums, a better polynomial fit, the Wilson–Hilferty region)
   are measured, starting with NIST StRD at M4/M5, and filed for 2.1.

**Verification.**
- Errors are in ulps of the correctly rounded value. Zero and subnormal results are measured in units of 2⁻¹⁰⁷⁴.
- Arguments:
  - M2's grids across every branch point, and plan §3.2's sets;
  - ≥ 1M seeded MPFR or exact arguments for erf, erfc, `normal_cdf` and `ln_gamma`;
  - ≥ 50k for `gamma_inc` and `beta_inc`, weighted toward x ≈ a, the deep tails and small shapes.
- Every target: x86_64 ELF, x86_64 PE and aarch64.

## Consequences

- **Positive.** pramana's CDFs and p-values become more accurate than Rust's by one to five orders of magnitude,
  depending on the function and region, and identical on every target.
- **Negative.**
  - M2 roughly doubles.
  - The correctly rounded functions cost more per call: kimiya measured 39–138 ns, against glibc's ~5–20 ns.
  - The FMA-free erf/erfc pays for exact products in software.
  - pramana owns several thousand more lines of numerical code (kimiya's correctly rounded functions alone are about
    6,800 with their tables) until ganita ships a correctly rounded tier and special functions (ledger L1, L8).
- **Neutral.**
  - Downstream golden rows are generated once, after M2 part b, from Rust's formulas with the special functions
    taken from the reference values.
  - Results differ from Rust 1.3.0 in the last digits wherever Rust was less accurate. ADR 007 rows 12–13 name this.

## Alternatives considered

- **T0 only** (Rust's algorithms on correctly rounded primitives): leaves every error above in place. Fixing them in
  2.1 would remap every distribution and hypothesis row a second time.
- **Bounds set by prototypes** rather than fixed: declined by the maintainer, who wanted commitments.
- **Correctly rounded `ln_gamma` and incomplete gamma/beta:** no binary64 implementation exists to port (CORE-MATH
  excludes lgamma). The incomplete functions are research-grade.
