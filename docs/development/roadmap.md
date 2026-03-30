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
- [x] ADRs, threat model, dependency watch, testing guide

## v1.1 — Performance, Platform & Numerical Hardening

### Performance
- [ ] SIMD-accelerated descriptive statistics (leverage hisab parallel feature)
- [ ] Parallel MCMC (multiple chains)
- [ ] Streaming/online statistics (Welford's algorithm for mean/variance)
- [ ] Memory-mapped large dataset support

### Platform
- [ ] WASM target support (no-std compatible core)
- [ ] C FFI bindings
- [ ] Python bindings (via PyO3)

### AI enhancements
- [ ] Multi-turn statistical analysis sessions
- [ ] Automated exploratory data analysis (auto-EDA)
- [ ] Natural language report generation
- [ ] Anomaly detection tool for AI dispatch

### New distributions
- [ ] Log-Normal distribution
- [ ] Negative Binomial distribution
- [ ] Hypergeometric distribution
- [ ] Geometric distribution
- [ ] Dirichlet distribution (multivariate generalization of Beta)
- [ ] Multinomial distribution

### Numerical improvements
- [ ] Scaled forward algorithm for HMMs (prevent underflow on long sequences)
- [ ] MA(q) support in ARIMA via conditional sum-of-squares (currently AR-only)
- [ ] Adaptive bandwidth selection for KDE (leave-one-out cross-validation)
- [ ] Improved Poisson CDF for large lambda (use incomplete gamma instead of summation)
- [ ] Standardize epsilon constants across modules (see threat-model.md)

### Testing
- [ ] Property-based tests via proptest or quickcheck
- [ ] Numerical accuracy benchmarks against R/scipy reference values
- [ ] 90%+ line coverage

## v1.2 — Non-Parametric Methods & Robust Statistics

### Non-parametric tests
- [ ] Mann-Whitney U test (two-sample rank test)
- [ ] Wilcoxon signed-rank test (paired samples)
- [ ] Kruskal-Wallis test (non-parametric ANOVA)
- [ ] Friedman test (repeated measures)
- [ ] Runs test (randomness)
- [ ] Fisher's exact test (2x2 contingency tables)

### Robust statistics
- [ ] Median absolute deviation (MAD)
- [ ] Trimmed mean
- [ ] Winsorized mean
- [ ] Huber M-estimator
- [ ] Spearman rank correlation
- [ ] Kendall's tau

### Confidence intervals
- [ ] Bootstrap confidence intervals (percentile, BCa)
- [ ] Wilson score interval for proportions (replaces Wald for small n)

## v1.3 — Advanced Regression & Model Selection

### Regression
- [ ] Multiple linear regression (matrix form, arbitrary predictors)
- [ ] Ridge regression (L2-penalized OLS)
- [ ] LASSO regression (L1 penalty, coordinate descent)
- [ ] Elastic net
- [ ] Generalized linear models (GLM) framework (Gaussian, Poisson, Binomial families)
- [ ] Regression diagnostics: leverage, Cook's distance, VIF

### Model selection
- [ ] AIC (Akaike Information Criterion)
- [ ] BIC (Bayesian Information Criterion)
- [ ] Adjusted R-squared
- [ ] Cross-validation (k-fold, leave-one-out)
- [ ] Stepwise regression (forward/backward)

## v1.4 — Time Series & Signal Processing

### Time series
- [ ] Seasonal ARIMA (SARIMA)
- [ ] Holt-Winters (triple exponential smoothing)
- [ ] Seasonal decomposition (STL)
- [ ] Granger causality test
- [ ] Augmented Dickey-Fuller test (stationarity)
- [ ] Ljung-Box test (autocorrelation significance)
- [ ] Change point detection

### Signal processing
- [ ] Spectral density estimation (periodogram, Welch's method) — leverage hisab FFT
- [ ] Cross-correlation function
- [ ] Coherence

## v1.5 — Bayesian & Probabilistic Graphical Models

### Bayesian
- [ ] Conjugate prior families (Beta-Binomial, Normal-Normal, Gamma-Poisson)
- [ ] Bayesian linear regression (posterior predictive)
- [ ] Hamiltonian Monte Carlo (HMC / NUTS)
- [ ] Variational inference (mean-field approximation)

### Graphical models
- [ ] Bayesian networks (DAG structure, exact inference)
- [ ] Factor analysis (via EM)
- [ ] Gaussian mixture models (EM fitting)
- [ ] Latent Dirichlet Allocation (topic modeling)

## v1.6 — Multivariate & Dimensionality Reduction

### Multivariate
- [ ] Multivariate hypothesis tests (Hotelling's T-squared, MANOVA)
- [ ] Canonical correlation analysis
- [ ] Discriminant analysis (LDA, QDA)
- [ ] Mahalanobis distance (standalone utility)

### Dimensionality reduction
- [ ] Incremental PCA (streaming/online)
- [ ] Sparse PCA
- [ ] Independent Component Analysis (ICA)
- [ ] t-SNE (via Barnes-Hut approximation)
- [ ] UMAP

## Cross-Crate Bridges

- [ ] `bridge.rs` module — primitive-value conversions for cross-crate statistics
- [ ] **badal bridge**: ensemble forecast spread → confidence intervals; observation error → likelihood weight
- [ ] **kimiya bridge**: reaction yield measurements → mean/variance; replicate data → hypothesis test p-value
- [ ] **ushma bridge**: measurement uncertainty (±K) → Gaussian error model; sensor noise → Kalman filter parameters

## Soorat Integration

- [ ] `integration/soorat.rs` module — feature-gated `soorat-compat`
- [ ] **Distribution curve**: PDF/CDF sample points for line plot rendering
- [ ] **MCMC trace**: sample chain positions for scatter/line rendering
- [ ] **Regression fit**: predicted vs actual points with confidence band for line+ribbon rendering
- [ ] **Histogram data**: bin edges and counts for bar chart rendering
- [ ] **Correlation matrix**: NxN values for heatmap rendering
