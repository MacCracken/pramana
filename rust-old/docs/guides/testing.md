# Testing Guide

## Running Tests

```bash
# Default features only
cargo test

# All features (includes ai module)
cargo test --all-features

# Specific module
cargo test --lib distribution
cargo test --lib hypothesis

# Integration tests only
cargo test --test '*'

# Doc tests only
cargo test --doc
```

## Test Categories

| Category       | Count | Command                                  |
|----------------|-------|------------------------------------------|
| Unit           | 301   | `cargo test --all-features --lib`        |
| Integration    | 15    | `cargo test --test '*'`                  |
| Doc            | 3     | `cargo test --doc`                       |
| **Total**      | **319** |                                        |
| Oracle hooks   | 11    | `cargo test --features oracle --test oracle` (hidden tooling for the Cyrius port; `--all-features` runs them too, for 330) |

`ai` unit tests (tool-argument bounds, serde) need `--all-features`; the
`soorat-compat` tests need that feature too.

### Reference values

Numerical tests compare against independent references, never against the
implementation itself:

- glibc `erf`/`erfc` and `0.5·erfc(-z/√2)` for the normal CDF
- closed forms: Cauchy (`t₁`), `t₂`, `F(2, ν)`, `I_x(a, 1) = xᵃ`,
  `P(χ²₁ > s) = erfc(√(s/2))`, `P(χ²₂ > s) = e^{-s/2}`, Kolmogorov series
- 50-digit `decimal` Poisson sums for `Q(n, x)` (e.g. `Q(500000, 500000)`)
- brute-force path enumeration for HMM likelihoods

### Hardening regression tests

Each module ends with a "Hardening regression tests" block: one test per
fixed defect (panic, hang, wrong result, or validation gap), so a regression
fails with the original symptom in its name.

## Coverage

Target: 80% line coverage.

```bash
make coverage    # runs cargo-llvm-cov, opens HTML report
```

## Benchmarks

34 benchmarks via Criterion:

```bash
make bench                        # full suite
cargo bench -- "distribution"     # filter by name
./scripts/bench-history.sh        # record to bench-history/
```

## Testing Patterns

### Approximate Equality

All floating-point comparisons use `assert!((a - b).abs() < EPSILON)` or a
helper macro. Never use `assert_eq!` for `f64`.

### Serde Roundtrip

Every public type has a test that serializes to JSON and deserializes back,
asserting equality. This catches missing `Serialize`/`Deserialize` derives and
field rename issues.

### Statistical Property Tests

- **KDE integrates to 1**: numerical integration of the density over a wide
  range, assert within 0.01 of 1.0.
- **Baum-Welch monotonicity**: log-likelihood must not decrease between EM
  iterations.
- **Distribution mean/variance**: sample mean and variance from large draws
  converge to theoretical values within tolerance.
- **MCMC stationarity**: chain samples approximate the target distribution's
  moments after burn-in.

## Local CI

```bash
make check    # fmt, clippy, test, doc, audit, deny -- mirrors CI pipeline
```
