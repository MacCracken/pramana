# Security Policy

## Scope

Pramana is a pure statistics and probability library providing distributions, hypothesis testing, Bayesian inference, Monte Carlo methods, and Markov chains for Rust. The core library performs no I/O and contains no `unsafe` code.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Numerical stability | Catastrophic cancellation, overflow | IEEE 754 f64; documented precision limits |
| Distribution parameters | Invalid lambda, negative std_dev | Returns `Err(InvalidParameter)` |
| Empty samples | Division by zero on empty data | Returns `Err(InvalidSample)` |
| Iterative methods | Non-convergence | max_iter bounds; returns `Err(ConvergenceFailure)` |
| Markov chains | Invalid transition matrix | Row-sum validation; returns `Err(InvalidParameter)` |
| Monte Carlo | Seed predictability | SimpleRng is for reproducibility, not cryptography |
| Serde deserialization | Crafted JSON | Enum validation via serde derive |
| Dependencies | Supply chain compromise | cargo-deny, cargo-audit in CI; minimal deps |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x | Yes |

## Reporting

- Contact: **security@agnos.dev**
- Do not open public issues for security vulnerabilities
- 48-hour acknowledgement SLA
- 90-day coordinated disclosure

## Design Principles

- Zero `unsafe` code
- No `unwrap()` or `panic!()` in library code -- all errors via `Result`
- All public types are `Send + Sync` (compile-time verified)
- No network I/O in core library
- Minimal dependency surface (core depends only on hisab, serde, thiserror, tracing)
- SimpleRng is NOT cryptographically secure -- documented clearly
