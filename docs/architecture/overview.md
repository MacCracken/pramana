# Architecture Overview

Version: 1.2.0 (MSRV 1.100)

## Module Map

```
pramana/
  src/
    lib.rs            -- crate root, public re-exports
    error.rs          -- PramanaError (thiserror)
    rng.rs            -- Rng trait + SimpleRng (xorshift64)
    math.rs           -- pub(crate) special functions (fdlibm erf/erfc, ln_gamma, Stirling remainder,
                         incomplete beta/gamma returning both tails)
    distribution.rs   -- Distribution trait + Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli, etc.
    descriptive.rs    -- Descriptive statistics (mean, median, mode, variance, percentiles, skewness, kurtosis)
    hypothesis.rs     -- Hypothesis testing (t-tests, chi-squared, p-values)
    regression.rs     -- Linear (OLS) and logistic (IRLS) regression
    bayesian.rs       -- Bayesian inference, conjugate priors, naive Bayes classifier
    combinatorics.rs  -- Factorial, permutations, combinations
    monte_carlo.rs    -- Monte Carlo integration + pi estimation
    markov.rs         -- Markov chains, HMM (forward/backward, Baum-Welch, Viterbi)
    timeseries.rs     -- Moving average, EMA, autocorrelation, ARIMA
    bridge.rs         -- Primitive-value bridges for other AGNOS science crates
    integration/      -- soorat visualization data (feature-gated behind `soorat-compat`)
    ai.rs             -- Hoosh integration (feature-gated behind `ai`)
```

## Dependencies

| Crate     | Role                                | Required |
|-----------|-------------------------------------|----------|
| hisab     | Linear algebra (Cholesky, eigen, matrix ops) via `num` feature | Yes |
| serde     | Serialization for all public types  | Yes      |
| thiserror | `PramanaError` derive               | Yes      |
| hoosh     | AI/LLM client                       | No (`ai` feature) |
| tokio     | Async runtime for hoosh             | No (`ai` feature) |
| serde_json| Tool-call arguments for hoosh       | No (`ai` feature) |

## Data Flow

```
caller data (&[f64], params)
  |
  v
pramana public API (distribution, hypothesis, regression, ...)
  |
  +---> math.rs (special functions, pub(crate))
  +---> hisab (matrix solve, decomposition)
  |
  v
Result<T, PramanaError>
```

All core functions operate on `f64` data. No I/O, no async, and no heap
allocations beyond return values and internal scratch buffers in the default
configuration. The `ai` module is the sole exception -- it performs async
network calls via hoosh.

## Invariants

- Every constructor validates its parameters (finite, in range) and every
  validated type deserializes through that constructor (`#[serde(try_from)]`),
  so serde input cannot create an invalid distribution, chain or model.
- No panics on any input: NaN data is rejected where it would otherwise poison
  a sort or a p-value; sizes that could overflow or over-allocate are checked
  (`checked_add`, `try_reserve`).
- `#![forbid(unsafe_code)]`.

## Consumers

AGNOS components needing statistical analysis:

- **daimon** -- anomaly detection, scoring distributions
- **aegis** -- security metrics, hypothesis testing
- **hoosh** -- model performance tracking, evaluation statistics
- **phylax** -- threat scoring, Bayesian classification
- Consumer applications using AGNOS as a framework
