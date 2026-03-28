# Architecture Overview

## Module Map

```
pramana/
  src/
    lib.rs            -- crate root, re-exports
    error.rs          -- PramanaError (thiserror)
    rng.rs            -- Rng trait + SimpleRng (xorshift64)
    distribution.rs   -- Distribution trait + Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli
    descriptive.rs    -- Descriptive statistics (mean, median, mode, variance, percentiles, etc.)
    hypothesis.rs     -- Hypothesis testing (t-tests, chi-squared)
    regression.rs     -- Linear regression
    bayesian.rs       -- Bayesian inference + naive Bayes
    combinatorics.rs  -- Factorial, permutations, combinations
    monte_carlo.rs    -- Monte Carlo integration + pi estimation
    markov.rs         -- Markov chains
    timeseries.rs     -- Moving average, exponential smoothing, autocorrelation
```

## Dependencies

- **hisab** -- mathematical primitives (constants, numerical utilities)
- **serde** -- serialization for all public types
- **thiserror** -- error types
- **tracing** -- structured logging

## Data Flow

All functions operate on `f64` data. No I/O, no async, no allocations beyond return values.

## Consumers

Any AGNOS component needing statistical analysis: daimon (anomaly detection), aegis (security metrics), hoosh (model performance tracking), phylax (threat scoring), and consumer apps.
