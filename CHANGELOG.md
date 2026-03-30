# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [1.2.0]

### Added
- **bridge/bodh** — psychology cross-crate bridges: `item_variances_to_alpha` (psychometric reliability), `z_rates_to_d_prime` (signal detection), `trait_scores_to_correlation` (construct relationships), `learning_trials_to_power_law` (learning curve fitting), `response_times_to_hick_params` (Hick's law coefficient extraction)

## [1.1.0]

### Added
- **bridge** — cross-crate primitive-value bridges for badal (ensemble spread to confidence, observation error to likelihood), kimiya (yields to mean/variance, replicates to t-statistic), ushma (uncertainty to Gaussian, sensor noise to std dev)
- **integration/soorat** — feature-gated `soorat-compat` module with visualization data structures: `DistributionCurve` (PDF/CDF plots), `McmcTrace` (sample chains), `RegressionFit` (predicted+confidence bands), `HistogramData` (binned from raw data), `CorrelationMatrix` (NxN heatmap)

### Updated
- hoosh 1.0.0 -> 1.1.0, majra 0.21.3 -> 1.0.2, iri-string 0.7.11 -> 0.7.12, zerocopy 0.8.47 -> 0.8.48

## [1.0.0] - 2026-03-28

### Added

- **AI**: Natural language statistical queries via hoosh (`ai` feature flag) — describe, correlate, regress, t-test, and forecast tools with LLM-driven dispatch
- **Distributions**: Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull, Multivariate Normal (Cholesky-based sampling, log-PDF)
- **Regression**: Polynomial regression (QR-based least squares, Horner evaluation, R²), logistic regression (IRLS, L2 regularization, probability/class prediction)
- **Hypothesis testing**: One-way ANOVA (F-test, full SS decomposition), Kolmogorov-Smirnov test (one-sample and two-sample), confidence intervals (mean, two-means, proportion)
- **Quantile functions**: t-distribution (bisection) and standard normal (Acklam approximation)
- **Time series**: ARIMA (differencing/integration, Yule-Walker AR fitting, forecasting)
- **Descriptive**: Kernel density estimation (Gaussian, Epanechnikov, Uniform, Triangular; Silverman bandwidth), correlation matrix (Pearson), Principal Component Analysis (eigendecomposition)
- **Monte Carlo**: Metropolis-Hastings MCMC (Gaussian random walk, burn-in) and Gibbs sampling
- **Markov**: Hidden Markov Models (Forward algorithm, Viterbi decoding, Baum-Welch EM)
- **Math**: Shared crate-internal module with `ln_gamma`, `ln_beta`, regularized incomplete beta/gamma functions, erf/erfc
- Doc-tests for crate-level examples
- Benchmarks for all modules (23 benchmarks)
- 212 tests (194 unit + 15 integration + 3 doc-tests)

### Changed

- Hypothesis test functions (`t_test_one_sample`, `t_test_two_sample`, `chi_squared_test`) now accept configurable `alpha` significance level
- `#[non_exhaustive]` on all public structs with validated constructors
- Poisson sampling uses normal approximation for lambda > 30
- Gamma sampling uses iterative Ahrens-Dieter boost (no recursion)
- Dependencies use public crates.io versions (hisab, hoosh)

### Fixed

- Dead variable in naive Bayes classifier loop
- Missing serde roundtrip tests for Exponential, Poisson, Binomial, Bernoulli
- Potential stack overflow in Gamma sampling for small alpha
- Potential infinite loop in Poisson sampling for large lambda

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
