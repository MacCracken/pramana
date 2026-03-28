# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

- Polynomial regression (QR-based least squares, Horner evaluation, R²)
- 7 new probability distributions: Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull
- Multivariate normal distribution with Cholesky-based sampling, `pdf`, `log_pdf`
- Shared math module with `ln_gamma`, `ln_beta`, regularized incomplete beta/gamma functions
- Doc-tests for crate-level examples
- Benchmarks for all modules (17 total, up from 4)

### Changed

- Hypothesis test functions (`t_test_one_sample`, `t_test_two_sample`, `chi_squared_test`) now accept configurable `alpha` significance level parameter
- `#[non_exhaustive]` added to all public structs with validated constructors (prevents bypassing validation via struct literals from external crates)
- Poisson sampling uses normal approximation for lambda > 30 (fixes potential infinite loop)
- Gamma sampling uses iterative Ahrens-Dieter boost (was recursive, risked stack overflow for small alpha)
- Deduplicated erf/erfc implementations into shared `math` module

### Fixed

- Dead variable `f` in naive Bayes classifier loop
- Missing serde roundtrip tests for Exponential, Poisson, Binomial, Bernoulli

## [0.1.0] - 2026-03-26

### Added

- Initial release
- Probability distributions: Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli
- Descriptive statistics: mean, median, mode, variance, std_dev, skewness, kurtosis, percentiles
- Hypothesis testing: one-sample t-test, two-sample t-test, chi-squared test
- Linear regression with R-squared
- Bayesian inference: Bayes theorem, naive Bayes classifier
- Combinatorics: factorial, permutations, combinations, Stirling approximation
- Monte Carlo: integration, pi estimation, deterministic SimpleRng
- Markov chains: transition matrix, steady-state, simulation
- Time series: moving average, exponential smoothing, autocorrelation
- Serde support for all public types
- Criterion benchmarks
