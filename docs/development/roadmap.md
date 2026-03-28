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

## In Progress

### v0.2.0
- [x] Additional distributions: Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull
- [x] Shared math foundation (ln_gamma, incomplete beta/gamma functions)
- [x] Configurable alpha for hypothesis tests
- [x] Scaffold hardening (struct invariants, dedup erf, Poisson/Gamma sampling fixes)
- [x] Multivariate normal distribution (Cholesky-based sampling, log-PDF)
- [x] Polynomial regression (QR-based least squares via hisab, Horner evaluation)
- [x] Logistic regression (IRLS with L2 regularization, sigmoid, proba/class prediction)
- [x] Confidence intervals (mean, two-means, proportion; t-quantile via bisection, z-quantile via Acklam)
- [x] One-way ANOVA (F-test with full SS decomposition)
- [x] Kolmogorov-Smirnov test (one-sample and two-sample, Kolmogorov distribution p-value)
- [x] Kernel density estimation (Gaussian, Epanechnikov, Uniform, Triangular kernels; Silverman bandwidth)
- [ ] MCMC (Metropolis-Hastings, Gibbs sampling)
- [ ] Hidden Markov Models
- [ ] ARIMA time series
- [ ] Correlation matrix
- [ ] Principal Component Analysis
- [ ] AI feature: natural language statistical queries via hoosh

## v1.0 Criteria

- All backlog items complete
- 90%+ test coverage
- Comprehensive documentation with examples
- Battle-tested in at least 3 AGNOS consumers
- No known numerical stability issues
