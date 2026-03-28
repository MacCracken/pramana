# Architecture Overview

Version: 1.0.0

## Module Map

```
pramana/
  src/
    lib.rs            -- crate root, public re-exports
    error.rs          -- PramanaError (thiserror)
    rng.rs            -- Rng trait + SimpleRng (xorshift64)
    math.rs           -- pub(crate) special functions (erf, ln_gamma, incomplete beta/gamma)
    distribution.rs   -- Distribution trait + Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli, etc.
    descriptive.rs    -- Descriptive statistics (mean, median, mode, variance, percentiles, skewness, kurtosis)
    hypothesis.rs     -- Hypothesis testing (t-tests, chi-squared, p-values)
    regression.rs     -- Linear (OLS) and logistic (IRLS) regression
    bayesian.rs       -- Bayesian inference, conjugate priors, naive Bayes classifier
    combinatorics.rs  -- Factorial, permutations, combinations
    monte_carlo.rs    -- Monte Carlo integration + pi estimation
    markov.rs         -- Markov chains, HMM (forward/backward, Baum-Welch, Viterbi)
    timeseries.rs     -- Moving average, EMA, autocorrelation, ARIMA
    ai.rs             -- Hoosh integration (feature-gated behind `ai`)
```

## Dependencies

| Crate     | Role                                | Required |
|-----------|-------------------------------------|----------|
| hisab     | Linear algebra (Cholesky, eigen, matrix ops) via `num` feature | Yes |
| serde     | Serialization for all public types  | Yes      |
| thiserror | `PramanaError` derive               | Yes      |
| tracing   | Structured logging                  | Yes      |
| hoosh     | AI/LLM client                       | No (`ai` feature) |
| tokio     | Async runtime for hoosh             | No (`ai` feature) |

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
allocations beyond return values in the default configuration. The `ai` module
is the sole exception -- it performs async network calls via hoosh.

## Consumers

AGNOS components needing statistical analysis:

- **daimon** -- anomaly detection, scoring distributions
- **aegis** -- security metrics, hypothesis testing
- **hoosh** -- model performance tracking, evaluation statistics
- **phylax** -- threat scoring, Bayesian classification
- Consumer applications using AGNOS as a framework
