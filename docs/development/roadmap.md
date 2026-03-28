# Roadmap

## Completed

### v0.1.0 (2026-03-26)
- [x] Error types (PramanaError)
- [x] Rng trait + SimpleRng
- [x] Distributions: Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli
- [x] Descriptive statistics
- [x] Hypothesis testing (t-tests, chi-squared)
- [x] Linear regression
- [x] Bayesian inference + naive Bayes
- [x] Combinatorics
- [x] Monte Carlo integration
- [x] Markov chains
- [x] Time series (moving average, exponential smoothing, autocorrelation)
- [x] Serde on all public types
- [x] Criterion benchmarks
- [x] Integration tests

### v1.0.0 (2026-03-28)
- [x] Additional distributions: Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull
- [x] Multivariate normal distribution (Cholesky-based sampling, log-PDF)
- [x] Shared math foundation (ln_gamma, incomplete beta/gamma functions)
- [x] Configurable alpha for hypothesis tests
- [x] Scaffold hardening (struct invariants, dedup erf, Poisson/Gamma sampling fixes)
- [x] Polynomial regression (QR-based least squares via hisab, Horner evaluation)
- [x] Logistic regression (IRLS with L2 regularization, sigmoid, proba/class prediction)
- [x] Confidence intervals (mean, two-means, proportion; t-quantile, z-quantile)
- [x] One-way ANOVA (F-test with full SS decomposition)
- [x] Kolmogorov-Smirnov test (one-sample and two-sample)
- [x] Kernel density estimation (4 kernels, Silverman bandwidth)
- [x] MCMC: Metropolis-Hastings and Gibbs sampling
- [x] Hidden Markov Models (Forward, Viterbi, Baum-Welch)
- [x] ARIMA time series (differencing, Yule-Walker AR fitting, forecasting)
- [x] Correlation matrix (Pearson)
- [x] Principal Component Analysis (eigendecomposition via hisab)
- [x] AI: natural language statistical queries via hoosh (feature-gated)
- [x] Public crate dependencies (hisab, hoosh from crates.io)

## Backlog

- [ ] Additional distribution families: Log-Normal, Negative Binomial, Hypergeometric
- [ ] Multivariate regression (multiple predictors)
- [ ] Non-parametric tests: Mann-Whitney U, Wilcoxon signed-rank
- [ ] Cross-validation and model selection (AIC, BIC)
- [ ] Bootstrap confidence intervals
- [ ] Seasonal ARIMA (SARIMA)
- [ ] Factor analysis
- [ ] Bayesian networks
- [ ] GPU-accelerated operations (via hisab parallel feature)
- [ ] WASM target support

## v1.0 Criteria

- ~~All backlog items complete~~ Core backlog complete
- 90%+ test coverage
- Comprehensive documentation with examples
- Battle-tested in at least 3 AGNOS consumers
- No known numerical stability issues
