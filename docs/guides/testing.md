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

| Module         | Unit Tests | Notes                                  |
|----------------|------------|----------------------------------------|
| distribution   | ~40        | PDF, CDF, sampling, edge cases         |
| descriptive    | ~20        | Mean, median, variance, percentiles    |
| hypothesis     | ~20        | t-tests, chi-squared, p-values         |
| regression     | ~15        | OLS, logistic, residuals               |
| markov         | ~15        | Transition, steady state, Baum-Welch   |
| bayesian       | ~12        | Prior update, naive Bayes              |
| monte_carlo    | ~10        | Integration, pi estimation             |
| timeseries     | ~10        | MA, EMA, autocorrelation, ARIMA        |
| combinatorics  | ~10        | Factorial, permutations, combinations  |
| rng            | ~8         | Uniformity, period, reproducibility    |
| math           | ~5         | erf, ln_gamma, incomplete beta/gamma   |
| error          | ~4         | Display, From impls                    |
| ai             | ~5         | Requires `--all-features`              |
| **Subtotal**   | **~174**   |                                        |

| Category       | Count | Command                        |
|----------------|-------|--------------------------------|
| Unit           | 194   | `cargo test --lib`             |
| Integration    | 15    | `cargo test --test '*'`        |
| Doc            | 3     | `cargo test --doc`             |
| **Total**      | **212** |                              |

## Coverage

Target: 80% line coverage.

```bash
make coverage    # runs cargo-llvm-cov, opens HTML report
```

## Benchmarks

23 benchmarks via Criterion:

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
