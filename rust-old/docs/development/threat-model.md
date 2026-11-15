# Threat Model

Last reviewed: 2026-11-14

## Trust Boundaries

Pramana is a **library crate**. It trusts its caller's *authority* completely
-- it does not check whether callers may perform statistical computations --
but it does **not** trust the *values* it is handed. Any `f64`, slice length,
count, or serde payload may be hostile or simply wrong, and must produce a
`PramanaError` (or a documented NaN/inf), never a panic, hang, or unbounded
allocation.

Two inputs deserve particular care:

- **Serde payloads.** Every validated type (distributions, `KernelDensity`,
  `MarkovChain`, `HiddenMarkovModel`, `BayesianUpdate`) deserializes through its
  validating constructor via `#[serde(try_from)]`. `MultivariateNormal`
  recomputes its cached Cholesky factor instead of trusting the serialized one.
  `SimpleRng` maps a zero state to 1.
- **LLM tool calls (`ai` feature).** Tool arguments are produced by a model and
  are untrusted: `steps` and `ar_order` are bounded (`MAX_FORECAST_STEPS`,
  `MAX_AR_ORDER`) before they size any computation or allocation. Hoosh owns
  TLS, auth token handling and request signing.

## Attack Surface

| Surface              | Input Type       | Risk                                    | Mitigation                              |
|----------------------|------------------|-----------------------------------------|-----------------------------------------|
| Distribution params  | `f64` values     | NaN/Inf propagation, invalid serde      | Finite/range validation in `new`; serde routes through `new` |
| Distribution CDF/PMF | `f64` argument   | O(x) loops on huge `x` (DoS)            | Poisson/Binomial CDFs via incomplete gamma/beta, PMFs via Stirling/`bd0` -- O(1)/O(√shape) |
| Samplers             | `n`, `lambda`    | O(n) loops for huge `n`                 | Binomial: Beta order-statistic splitting (O(log n)); Poisson: PTRS |
| Descriptive stats    | `&[f64]`         | NaN breaking sort (panic since Rust 1.81), O(n²) mode | NaN rejected; `total_cmp` selection; O(n log n) mode |
| Hypothesis tests     | `&[f64]` slices  | NaN p-value => `reject = false`; NaN alpha | Non-finite data and NaN alpha/confidence rejected |
| Regression (OLS)     | `&[f64]` pairs   | Cancellation on offset data; `degree + 1` overflow | Centered sums; overflow-free degree check |
| Regression (logistic)| `&[f64]` + labels| Separable data, non-convergence, empty model | L2 option, iteration cap, non-finite step guard, `predict` returns NaN for empty model |
| MCMC                 | Closure + params | `n_samples + burn_in` overflow, huge reservations, zero-density start | `checked_add`, `try_reserve_exact`, `-inf`/NaN-aware acceptance |
| Markov chain         | Public fields    | Out-of-range `state` (index panic)      | `step` is bounds-checked; serde validated |
| HMM                  | Observation seqs | Underflow on long sequences; inconsistent public fields | Rabiner-scaled forward/backward; shape check before indexing |
| ARIMA / timeseries   | `&[f64]` series  | Huge `steps` allocation                 | `try_reserve`; non-finite data rejected |
| KDE                  | `&[f64]` samples | Bandwidth -> 0, NaN or Inf              | Finite positive bandwidth, finite data, serde validated |
| PCA                  | Matrix via hisab | Degenerate eigenvalues, NaN sort        | Finite check; `total_cmp` ordering      |
| AI client            | Network I/O      | Prompt injection, oversized tool args   | Bounded tool arguments; caller's responsibility for data sensitivity |

## Panic Sites

**Zero** in library code. All fallible operations return
`Result<T, PramanaError>`; no `unwrap()`, `expect()`, `panic!()`, unchecked
arithmetic on caller-controlled sizes, or indexing that callers can push out of
range. Regression tests cover each previously reachable panic (NaN sort,
`MultivariateNormal`/`MarkovChain` deserialization, `permutations(u64::MAX, 0)`,
`polynomial_regression(.., usize::MAX)`, huge forecast/MCMC lengths).

## Unsafe Code

**Zero**, enforced with `#![forbid(unsafe_code)]`.

## Supply Chain

- `cargo audit` -- run in CI on every push; advisory database checked.
- `cargo deny check` -- license allow-list (only licenses actually present),
  duplicate crate detection, advisory checks, crates.io only.
- Minimal dependency tree by design: 3 direct runtime deps in the default
  configuration (`hisab`, `serde`, `thiserror`).

## Numerical Precision

| Context                    | Value              | Rationale                                     |
|----------------------------|--------------------|-----------------------------------------------|
| Special-function convergence | `1e-15` relative | Series / continued-fraction truncation        |
| Special-function iterations | `200 + 20·√shape`, cap 10⁶ | Converge in O(√shape); bounded worst case |
| Wilson–Hilferty switch     | shape > `1e9`      | Error far below 1e-10 there                   |
| Convergence (IRLS)         | `1e-8`             | Max coefficient change                        |
| Markov steady state        | `1e-10`            | L1 stationarity residual `‖πP − π‖₁`          |
| Stochastic row sums        | `1e-6`             | Markov / HMM parameter validation             |
| MVN symmetry               | `1e-10` relative   | Scale-independent symmetry check              |
| Bayes coherence            | `1e-12` relative   | `P(B|A)·P(A) ≤ P(B)`                          |

Baum-Welch runs exactly `max_iter` EM iterations (no early stopping).
