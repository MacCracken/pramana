# Threat Model

Last reviewed: 2026-03-28

## Trust Boundaries

Pramana is a **library crate**. It trusts its caller completely -- it does not
validate that callers have authorization to perform statistical computations. All
input validation is for correctness (e.g., non-negative variance) rather than
security.

The `ai` feature opens a network boundary via hoosh. Hoosh owns TLS, auth token
handling, and request signing; pramana passes data to hoosh's API and trusts its
responses.

## Attack Surface

| Surface              | Input Type       | Risk                                    | Mitigation                              |
|----------------------|------------------|-----------------------------------------|-----------------------------------------|
| Distribution params  | `f64` values     | NaN/Inf propagation                     | Param validation, error returns         |
| Hypothesis tests     | `&[f64]` slices  | Empty/single-element slices             | Length checks, descriptive errors        |
| Regression (OLS)     | `&[f64]` pairs   | Singular matrices                       | Cholesky failure -> `PramanaError`       |
| Regression (logistic)| `&[f64]` + labels| Separable data, non-convergence         | L2 regularization, iteration cap        |
| MCMC                 | Closure + params | Infinite loops in user-supplied closure | Iteration cap, tracing warnings         |
| HMM (Baum-Welch)     | Observation seqs | Underflow in forward/backward           | Log-space computation                   |
| ARIMA / timeseries   | `&[f64]` series  | Empty series, zero variance             | Length/variance checks                  |
| KDE                  | `&[f64]` samples | Bandwidth -> 0 or Inf                   | Silverman default, validation           |
| PCA                  | Matrix via hisab | Degenerate eigenvalues                  | Handled by hisab's eigen decomposition  |
| AI client            | Network I/O      | Prompt injection, data exfiltration     | Caller's responsibility; hoosh handles TLS |

## Panic Sites

**Zero.** All fallible operations return `Result<T, PramanaError>`. The crate
uses no `unwrap()`, `expect()`, or `panic!()` in library code.

## Unsafe Code

**Zero.** No `unsafe` blocks anywhere in the crate.

## Supply Chain

- `cargo audit` -- run in CI on every push; advisory database checked.
- `cargo deny check` -- license compliance (GPL-3.0 compatible deps only),
  duplicate crate detection, advisory checks.
- Minimal dependency tree by design (5 runtime deps in default config).

## Numerical Precision

Epsilon values used across modules:

| Context                | Epsilon       | Rationale                              |
|------------------------|---------------|----------------------------------------|
| Float comparison       | `1e-10`       | General-purpose f64 near-equality      |
| Convergence (IRLS)     | `1e-8`        | Relative log-likelihood change         |
| Convergence (Baum-Welch)| `1e-6`       | Log-likelihood plateau detection       |
| Distribution CDF       | `1e-12`       | Tighter bound for cumulative probs     |
| Special functions      | `1e-14`       | Continued fraction / series truncation |
