# pramana

**pramana** (Sanskrit: proof/measure/evidence) -- Statistics and probability library for the AGNOS ecosystem.

> ⚠ **Being ported to Cyrius as 2.0.0.**
> - Until the port lands, this README describes the **Rust 1.3.0** crate. That crate is on crates.io, and its source
>   is frozen in [`rust-old/`](rust-old/) as the parity reference, read and never run: Python's models of it
>   generate the golden corpus (ADR 010).
> - The plan of record is [`docs/development/pramana-port-plan.md`](docs/development/pramana-port-plan.md), and
>   progress is in [`docs/development/state.md`](docs/development/state.md).
> - Rust users: stay on `pramana = "1"`. The full Cyrius README arrives with the 2.0.0 release.

[![CI](https://github.com/MacCracken/pramana/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/pramana/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/pramana.svg)](https://crates.io/crates/pramana)
[![docs.rs](https://docs.rs/pramana/badge.svg)](https://docs.rs/pramana)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

## Modules

| Module | Description |
|--------|-------------|
| `distribution` | Probability distributions (Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli, Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull, Multivariate Normal) |
| `descriptive` | Descriptive statistics, KDE, correlation matrix, PCA |
| `hypothesis` | Hypothesis testing (t-tests, chi-squared) and confidence intervals |
| `regression` | Linear, polynomial, and logistic regression |
| `bayesian` | Bayesian inference and naive Bayes classification |
| `combinatorics` | Factorials, permutations, combinations, Stirling approximation |
| `monte_carlo` | Monte Carlo integration, Metropolis-Hastings MCMC, Gibbs sampling |
| `markov` | Markov chains, Hidden Markov Models (Forward, Viterbi, Baum-Welch) |
| `timeseries` | Time series: moving average, exponential smoothing, autocorrelation, ARIMA |
| `bridge` | Primitive-value bridges for other AGNOS science crates |

### Feature flags

| Feature | Enables |
|---------|---------|
| `ai` | Natural-language statistical queries via hoosh (`ai::StatQuery`) |
| `soorat-compat` | Visualization data structures for soorat (`integration::soorat`) |
| `logging` | Reserved no-op (kept for compatibility) |

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

The Cyrius port builds from the root (CONTRIBUTING.md):

```bash
cyrius deps
python3 -I -B scripts/golden/gen.py . && python3 -I -B scripts/golden/validate.py . --write-cr   # the golden corpus
cyrius build src/main.cyr build/pramana
cyrius test
```

The Rust 1.3.0 crate's source stays in `rust-old/` as the parity reference until 2.0.0. Nothing runs it: the golden
corpus comes from Python's models of it (ADR 010).

## MSRV

Rust 1.100 (edition 2024)

## License

GPL-3.0-only. See [LICENSE](LICENSE).
