# pramana

**pramana** (Sanskrit: proof/measure/evidence) -- Statistics and probability library for the AGNOS ecosystem.

[![CI](https://github.com/MacCracken/pramana/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/pramana/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/pramana.svg)](https://crates.io/crates/pramana)
[![docs.rs](https://docs.rs/pramana/badge.svg)](https://docs.rs/pramana)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

## Modules

| Module | Description |
|--------|-------------|
| `distribution` | Probability distributions (Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli, Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull) |
| `descriptive` | Descriptive statistics (mean, median, mode, variance, skewness, kurtosis, percentiles) |
| `hypothesis` | Hypothesis testing (t-tests, chi-squared) |
| `regression` | Linear regression with R-squared |
| `bayesian` | Bayesian inference and naive Bayes classification |
| `combinatorics` | Factorials, permutations, combinations, Stirling approximation |
| `monte_carlo` | Monte Carlo integration and simulation |
| `markov` | Markov chains with steady-state analysis |
| `timeseries` | Time series analysis (moving average, exponential smoothing, autocorrelation) |

## Quick Start

```rust
use pramana::{descriptive, distribution::{Normal, Distribution}, monte_carlo::SimpleRng};

// Descriptive statistics
let data = [1.0, 2.0, 3.0, 4.0, 5.0];
let m = descriptive::mean(&data).unwrap();
let s = descriptive::std_dev(&data).unwrap();

// Fit and sample from a normal distribution
let normal = Normal::new(m, s).unwrap();
let mut rng = SimpleRng::new(42);
let sample = normal.sample(&mut rng);
```

## Building

```bash
cargo build
cargo test
make check    # fmt + clippy + test + audit
make bench    # criterion benchmarks with history
```

## MSRV

Rust 1.89

## License

GPL-3.0-only. See [LICENSE](LICENSE).
