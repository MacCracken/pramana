# Security Policy

## Scope

Pramana is a statistics and probability library providing distributions, hypothesis testing, regression, Bayesian inference, Monte Carlo methods, MCMC, Markov chains, HMMs, time series analysis, KDE, PCA, and natural language queries for Rust. The core library performs no I/O and contains no `unsafe` code.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Numerical stability | Catastrophic cancellation, overflow | IEEE 754 f64; documented precision limits |
| Distribution parameters | Invalid lambda, negative std_dev | Returns `Err(InvalidParameter)` |
| Empty samples | Division by zero on empty data | Returns `Err(InvalidSample)` |
| Iterative methods | Non-convergence (IRLS, power iteration, Baum-Welch) | max_iter bounds; returns `Err(ConvergenceFailure)` |
| Markov chains | Invalid transition matrix | Row-sum validation; returns `Err(InvalidParameter)` |
| HMMs | Invalid stochastic matrices | Full validation on construction |
| MCMC | Non-convergence on multimodal targets | Consumer responsibility; burn-in parameter |
| Monte Carlo | Seed predictability | SimpleRng is for reproducibility, not cryptography |
| Matrix operations | Singular/non-positive-definite matrices | Cholesky/eigendecomposition errors propagated |
| Serde deserialization | Crafted JSON | Enum validation via serde derive |
| AI client (opt-in) | Network I/O to hoosh | Feature-gated (`ai`); not compiled by default |
| Dependencies | Supply chain compromise | cargo-deny, cargo-audit in CI; minimal deps |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x | Yes |
| 0.1.x | Security fixes only |

## Reporting

- Contact: **security@agnos.dev**
- Do not open public issues for security vulnerabilities
- 48-hour acknowledgement SLA
- 90-day coordinated disclosure

## Design Principles

- Zero `unsafe` code
- No `unwrap()` or `panic!()` in library code -- all errors via `Result`
- All public types are `Send + Sync` (compile-time verified)
- No network I/O in core library (AI client is opt-in via feature flag)
- Minimal dependency surface (core depends only on hisab, serde, thiserror, tracing)
- SimpleRng is NOT cryptographically secure -- documented clearly
