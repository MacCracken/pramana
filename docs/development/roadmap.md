# Roadmap

Planned work only. Done work is in the [CHANGELOG](../../CHANGELOG.md): 0.1.0–1.3.0 are the Rust releases, and 2.0.0
is the Cyrius port of 1.3.0 (M0–M8 of the [port plan](pramana-port-plan.md)). Live state is in [`state.md`](state.md).

**Versions.**
- **2.0.x** finishes the port: the release's loose ends, verification, and stand-ins converted as their libraries
  ship. It adds no public API, and a value changes only where 2.0.0 does not yet match 1.3.0.
- **2.1.0 onward** adds features. A minor that moves a value 2.0.0 returns gives the row in ADR 007 (it diverges
  from 1.3.0) and carries the reference rows that prove the new value.

**New work's proof.** A function 1.3.0 doesn't have has no golden row to match. It is held to a Python model, with
MPFR, exact arithmetic or NIST StRD as the reference (ADR 010). Its Class-C bound is set by
[`port-parity.md`](port-parity.md)'s rule before its first comparison. The elementary functions stay correctly
rounded, ADR 009's bounds are never loosened, and the bits stay the same on every target.

## 2.0.x — finishing the port

### The 2.0.0 release
- [ ] The zugot recipe. `~/Repos/zugot/marketplace/pramana.cyml` is at 2.0.0, with its sha256 still a TODO: fill it
      from the release tarball's `SHA256SUMS`. Switching the recipe from the Rust 1.3.0 build to the Cyrius one is the
      maintainer's call (port plan §12)
- [ ] The 1.3.x deprecation notice on crates.io, pointing Rust users at 2.0.0 and
      [`migration-2.0.md`](../guides/migration-2.0.md). This is the maintainer's (port plan §12). bodh, jivanu and
      sangha stay on `pramana = "1"` until they port

### Security
- [ ] A Cyrius threat model. The 38 JSON readers and the `ai` HTTP client are the input surface. The Rust 1.3.0 model
      is at `rust-old/docs/development/threat-model.md`

### Verification
- [ ] Make the vendored-source checks real in CI. `vendor_crmath.py`, `vendor_fmtf.py`, `vendor_coremath.py` and
      `vendor_lgamma.py --check` all pass as unverified there, because CI has none of their sources: `../kimiya`,
      `../impetus`, CORE-MATH's source in `build/coremath/`, and libm 0.2.16's source. CI should fetch each at its
      pinned revision
- [ ] Run `scripts/crverify/run.sh` (the port against MPFR at 1,024 bits, on all three targets) in CI, or on a
      schedule if it is too slow for every push. Today it runs by hand
- [ ] `pow` on a signaling NaN: `pow(sNaN, ±0)` and `pow(1, sNaN)` give 1.0, where glibc, so 1.3.0, quiets it to a NaN
      as IEEE 754 asks (2.0.1's audit). A signaling NaN only comes from raw bits; decide between an ADR 007 row and
      matching 1.3.0

### Stand-ins, converted as their libraries ship (ADR 008)
The conversion ledger is the port plan's §10: 16 rows, all open, with 25 `UPSTREAM` notes in `src/`. Each row converts
when its library ships, and its parity rows are re-run then. A conversion that would move a row's bits becomes a 2.x
item, with its ADR row.
- [ ] ganita: the correctly rounded tier (L1), the `_into` Cholesky solve (L4), and the special functions if ganita
      takes them (L8)
- [ ] hisab: the polynomial least squares (L5), and `eigen_symmetric` with a tolerance and caller scratch (L6)
- [ ] The cyrius stdlib:
  - Rust's NaN rule for min and max, a trunc-based fract, and an unsigned u64 → f64 (L2);
  - a stable sort on raw words (L3);
  - `#derive` JSON with a validation hook (L7);
  - `#derive(Debug)` and a shortest printer, with bayan (L10);
  - a scalar fma (L12);
  - `vec_push_a` and `str_builder` forms that return −1 instead of aborting (L13, L14);
  - a word-wise `memcpy` (L15);
  - a float parser that takes any exponent (L16: `_f64_parse_n` drops exponent digits past six).
- [ ] bayan: u64 nodes, signed zero and strict strings (L11)
- [ ] hoosh: a client bundle (L9)
- [ ] The seven requests filed with no stand-in (port plan §10). When each one ships, its workaround is retired:
  - the closure-capture rule (port plan §8.3, cyrius);
  - the `include "lib/alloc.cyr"` in files that `cyrius lint` compiles alone (cyrius);
  - the freelist's mmap per large block: 36 µs per MVN new and drop at d = 32 (cyrius);
  - bayan's opt-in for unread members and its nesting cap. 35 of the 1,495 serde_json documents that 1.3.0 reads are
    refused until it ships (ADR 007 row 16), so the fix restores 1.3.0's behavior;
  - sandhi's static data: `tests/ai_sandhi.tcyr` builds the transport in a binary of its own until then;
  - sandhi's `Host` header for a non-default port, built on the default heap whatever the arena: about 424 bytes an
    exchange are left on the consumer's heap until then (`tests/ai_sandhi.tcyr`'s heap bound).

## 2.1.0 — accuracy, and the AI client's tools

These are the port's findings. Each is parity-bound in 2.0.0, which keeps 1.3.0's formula, and is measured against
MPFR or NIST StRD in [`port-parity.md`](port-parity.md) §9–§11. Fixing one moves its rows away from 1.3.0's.

### Quantiles (M4 part b2; the T1 candidate of port plan §12)
- [ ] `z_quantile` (Acklam's approximation and one Halley step) is within 0.5 ulp on 2,427 of the 3,215 values of p
      below ½. It is up to 387 ulp off near ½, where Φ(x) − p cancels, and up to 9.4×10⁶ ulp off above ½: at
      p = 0.999999999999969, most of the tail is Φ's rounding next to 1. The reference is MPFR at 1,024 bits. The fix
      is a Halley step on 1 − Φ above ½, and in double-double near ½
- [ ] `t_quantile` stops its bisection at 1e-12 relative, so it is up to 1.5×10⁶ ulp from the true quantile
      (p = 0.5005, df = 29). 119 of the 120 (p, df) pairs are beyond 1 ulp. The fix is a Newton step on the T1 CDF
      after the bisection

### Sums and fits (D4; NIST StRD)
- [ ] Compensated or shifted sums for the means and the sums of squares (M4 part c; port-parity §10.3).
      `one_way_anova` keeps 6.5–8.8 digits on the average-difficulty sets. On SmLs07–09, whose values sit near 10¹²,
      it keeps −0.9 to 4.3. The population variance keeps 8.3 digits on NumAcc4
- [ ] A centered, scaled Householder QR for the polynomial fit (M5 part d; port-parity §11.2). hisab's Gram–Schmidt on
      the raw Vandermonde matrix keeps 4.0 digits of Filip's worst coefficient (degree 10), and 6.5–10.9 digits of
      Wampler1–5's (degree 5)

### Distributions (M3; port-parity §9)
- [ ] Weibull: the variance cancels for large k (1.3.0 and 2.0.0 are both noise from k = 10¹⁰), and the moments
      overflow where λ·Γ is finite. The fix is a form on lnΓ differences and logs (part a3)
- [ ] Gamma's and χ²'s pdf, an exp of a sum, is up to 1.36×10⁶ ulp off at α = 10⁵. T1's incomplete gamma already holds
      the prefactor x^α·e^(−x)/Γ(α) in double-double (part b1)
- [ ] Poisson's pmf loses about 16 bits in Loader's kernel at λ = 10⁻³ and k ≥ 40. The direct form,
      a·ln λ − λ − lnΓ(a), does not cancel there (part c1)
- [ ] Binomial's pmf is NaN at p = 5e-324 for k ≥ 1, because bd0's u overflows (part c2)
- [ ] MultivariateNormal's density loses digits with the covariance's condition number: 2×10⁻⁸ relative at
      κ ≈ 2×10⁸. The fix is a refined solve (part d2)

### Tolerances
- [ ] Standardize the epsilon constants across modules. The 1.3.0 threat model's precision table lists them,
      among them IRLS convergence 1e-8, the Markov steady state 1e-10, stochastic row sums 1e-6, MVN symmetry 1e-10
      and Bayes coherence 1e-12

### The AI client's tools (M7 part b's finding)
- [ ] hoosh 1.3.0's client drops `tools` and returns no tool calls, so 1.3.0's `StatQuery::query` always answers
      directly, and 2.0.0 does the same. The five tools are ported and proven on golden rows, but nothing can reach
      them yet. To make them reachable:
  - send the five tools in OpenAI's tool-calling form;
  - run the returned call through `_pmn_ai_execute_tool`;
  - send the follow-up request.

## 2.2.0 — distributions
- [ ] Log-Normal
- [ ] Negative Binomial
- [ ] Hypergeometric
- [ ] Geometric
- [ ] Dirichlet (the multivariate generalization of Beta)
- [ ] Multinomial

## 2.3.0 — non-parametric and robust statistics

### Non-parametric tests
- [ ] Mann-Whitney U (a two-sample rank test)
- [ ] Wilcoxon signed-rank (paired samples)
- [ ] Kruskal-Wallis (non-parametric ANOVA)
- [ ] Friedman (repeated measures)
- [ ] The runs test (randomness)
- [ ] Fisher's exact test (2×2 contingency tables)

### Robust statistics
- [ ] Median absolute deviation (MAD)
- [ ] Trimmed mean
- [ ] Winsorized mean
- [ ] Huber M-estimator
- [ ] Spearman rank correlation
- [ ] Kendall's tau

### Intervals and densities
- [ ] Bootstrap confidence intervals (percentile, BCa)
- [ ] The Wilson score interval for proportions, which replaces Wald for small n
- [ ] Adaptive bandwidth selection for KDE (leave-one-out cross-validation)

## 2.4.0 — regression and model selection

### Regression
- [ ] Multiple linear regression (matrix form, arbitrary predictors), with a fit without an intercept. These make NIST
      StRD's NoInt1/2 and Longley measurable; 1.3.0 can fit neither (port-parity §11.2)
- [ ] Ridge regression (L2-penalized OLS)
- [ ] LASSO regression (L1 penalty, coordinate descent)
- [ ] Elastic net
- [ ] A generalized linear model (GLM) framework: Gaussian, Poisson and Binomial families
- [ ] Regression diagnostics: leverage, Cook's distance, VIF

### Model selection
- [ ] AIC (Akaike Information Criterion)
- [ ] BIC (Bayesian Information Criterion)
- [ ] Adjusted R²
- [ ] Cross-validation (k-fold, leave-one-out)
- [ ] Stepwise regression (forward and backward)

## 2.5.0 — time series and signal processing

### Time series
- [ ] MA(q) in ARIMA, by conditional sum of squares. ARIMA is AR-only today
- [ ] Seasonal ARIMA (SARIMA)
- [ ] Holt-Winters (triple exponential smoothing)
- [ ] Seasonal decomposition (STL)
- [ ] Granger causality test
- [ ] Augmented Dickey-Fuller test (stationarity)
- [ ] Ljung-Box test (autocorrelation significance)
- [ ] Change point detection

### Signal processing
These need an FFT from an AGNOS library. Since the port, pramana has no hisab dependency (port plan §8.2).
- [ ] Spectral density estimation (periodogram, Welch's method)
- [ ] Cross-correlation function
- [ ] Coherence

## 2.6.0 — Bayesian inference and graphical models

### Bayesian
- [ ] Conjugate prior families (Beta-Binomial, Normal-Normal, Gamma-Poisson)
- [ ] Bayesian linear regression (posterior predictive)
- [ ] Hamiltonian Monte Carlo (HMC / NUTS)
- [ ] Variational inference (mean-field approximation)

### Graphical models
- [ ] Bayesian networks (DAG structure, exact inference)
- [ ] Factor analysis (by EM)
- [ ] Gaussian mixture models (EM fitting)
- [ ] Latent Dirichlet Allocation (topic modeling)

## 2.7.0 — multivariate analysis and dimensionality reduction

### Multivariate
- [ ] Multivariate hypothesis tests (Hotelling's T², MANOVA)
- [ ] Canonical correlation analysis
- [ ] Discriminant analysis (LDA, QDA)
- [ ] Mahalanobis distance as a standalone utility, on MultivariateNormal's Cholesky factor

### Dimensionality reduction
- [ ] Incremental PCA (streaming, online)
- [ ] Sparse PCA
- [ ] Independent Component Analysis (ICA)
- [ ] t-SNE (Barnes-Hut approximation)
- [ ] UMAP

## Later in 2.x (unscheduled)

### Performance
The 2.0.0 record ([`benchmarks-rust-v-cyrius.md`](../benchmarks-rust-v-cyrius.md), M8 part b) puts the port at a
geometric mean of 10.0× Rust 1.3.0's time, over a range of 1.8–66.3×. Most of that is correct rounding, the T1
functions and Cyrius's codegen. Speed is measured, not gated. A speedup that leaves every bit as it was can ship in
any patch.
- [ ] The rows dominated by the T1 functions: `chi_squared_test_20` at 66×, `binomial_cdf` at 62×, `poisson_cdf` at
      59× and `normal_cdf` at 52×
- [ ] Streaming (online) statistics as a public accumulator, by Welford's method
- [ ] Parallel MCMC: several chains, with one RNG stream each
- [ ] SIMD descriptive statistics. These change the summation order, which moves bits away from 1.3.0's, so an ADR
      comes first
- [ ] Memory-mapped large datasets

### Platform
- [ ] C FFI bindings. `cyrius header` already prints the C prototypes of the public functions
- [ ] Python bindings over the C FFI. The 1.x plan used PyO3
- [ ] A WASM build. Cyrius has no wasm target (`cyrius build` makes x86_64, aarch64, PE and agnos), so a request to
      cyrius comes first

### AI, after 2.1.0's tools
- [ ] Multi-turn statistical analysis sessions
- [ ] Automated exploratory data analysis (auto-EDA)
- [ ] Natural-language report generation
- [ ] An anomaly detection tool for the AI dispatch

### Testing and observability
- [ ] Property-based tests, as `cyrius fuzz` harnesses beyond the JSON readers. The 1.x plan used proptest
- [ ] 90% line coverage, after 2.0.x's gate at 80%
- [ ] sakshi spans, if a consumer asks (port plan §4.2)
