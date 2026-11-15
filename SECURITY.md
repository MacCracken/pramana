# Security Policy

## Scope

Pramana is a statistics and probability library: distributions, hypothesis testing, regression, Bayesian inference,
Monte Carlo methods, MCMC, Markov chains, HMMs, time series, KDE, PCA, and opt-in natural-language queries.

The 2.0 line is a Cyrius library, being ported from the Rust crate 1.3.0 (`docs/development/pramana-port-plan.md`).
The core library performs no I/O. The frozen Rust crate in `rust-old/` and its hidden `oracle` feature are port
tooling, not public API.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Numerical stability | Catastrophic cancellation, overflow | Correctly rounded elementary functions; the special functions' bounds fixed and measured against MPFR (ADR 009) |
| Distribution parameters | Invalid lambda, negative std_dev, NaN | Constructors refuse them with `PMN_ERR_INVALID_PARAMETER` |
| Empty or NaN samples | Division by zero, unordered sorts | Refused with `PMN_ERR_INVALID_SAMPLE` before any sort |
| Iterative methods | Non-convergence (IRLS, power iteration, Baum–Welch) | Iteration bounds; `PMN_ERR_CONVERGENCE_FAILURE` |
| Markov chains, HMMs | Invalid stochastic matrices | Full validation on construction and on decode |
| MCMC | Non-convergence on multimodal targets | Consumer responsibility; burn-in parameter |
| Monte Carlo | Seed predictability | `PmnSimpleRng` is for reproducibility, not cryptography |
| Matrix operations | Singular or non-positive-definite matrices | Cholesky and eigen failures returned as error codes |
| JSON decoding | Crafted documents | Hand codecs that route every decode through the validating constructor (ADR 007, port plan D5); a fuzz harness for every reader (M8) |
| Memory | Unbounded growth on the bump heap | Caller buffers, freelist-owned objects with `_drop`, a work-table pool; a soak test per milestone |
| AI client (opt-in) | Network I/O to hoosh; untrusted model output | Outside the core bundle; tool arguments bounded (forecast steps, AR order) |
| Dependencies | Supply chain compromise | `cyrius deps --locked` in CI; minimal stdlib dependencies |

## Supported Versions

| Version | Language | Status |
|---|---|---|
| 2.0.x | Cyrius | In development: the port of 1.3.0 |
| 1.3.x | Rust | Supported. The final Rust line, frozen in `rust-old/`. Security fixes ship as 1.3.x |
| < 1.3 | Rust | Not supported |

## Reporting

- Contact: **security@agnos.dev**
- Do not open public issues for security vulnerabilities
- 48-hour acknowledgement SLA
- 90-day coordinated disclosure

Vulnerabilities in dependencies (the Cyrius stdlib, ganita, bayan, sandhi, or the Rust crates of the 1.3.x line)
should be reported to the respective upstream projects, and flagged here if they affect pramana users.

## Design Principles

- No aborts in library code: no overflow-abort operators, `panic`, `assert_fatal` or `result_unwrap`. Every error
  is a value-form `Result` with a code
- No network I/O in the core library (the `ai` client stays outside the core bundle)
- Decoding a validated type re-runs its constructor
- Results are identical on every target (x86_64 ELF, x86_64 PE, aarch64)
- `PmnSimpleRng` is NOT cryptographically secure — documented clearly
