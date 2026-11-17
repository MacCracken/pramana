# pramana

**pramana** (Sanskrit: proof, measure, evidence) is the statistics and probability library of AGNOS, in Cyrius.

[![CI](https://github.com/MacCracken/pramana/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/pramana/actions/workflows/ci.yml)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

2.0.0 is the Cyrius port of the Rust crate 1.3.0. Rust users can stay on `pramana = "1"` on crates.io, which keeps
1.3.x, and [`docs/guides/migration-2.0.md`](docs/guides/migration-2.0.md) maps every change.

## What it does

| Area | In 2.0 |
|---|---|
| Distributions | Normal, Uniform, Exponential, Bernoulli, Cauchy, Weibull, Gamma, χ², Beta, Student-t, F, Poisson, Binomial (caller storage, the `PmnDistribution` trait), MultivariateNormal (owned) |
| Special functions | erf, erfc, Φ, lnΓ, the incomplete gamma and beta, ln B |
| Descriptive statistics | mean, median, mode, variance, the moments, percentiles and quartiles, KDE, the correlation matrix, PCA |
| Hypothesis tests | one- and two-sample t, χ², one-way ANOVA, Kolmogorov–Smirnov, confidence intervals, the t and z quantiles |
| Regression | linear, polynomial, logistic |
| Time series | the moving average, exponential smoothing, autocorrelation, differencing and integration, ARIMA(p, d, 0) |
| Bayesian | Bayes' theorem, the update, naive Bayes |
| Markov | Markov chains (walks, the steady state), hidden Markov models (forward, Viterbi, Baum–Welch) |
| Monte Carlo | integration, π, Metropolis–Hastings, Gibbs sampling |
| Combinatorics, bridges | factorials, permutations and combinations in 128 bits, Stirling; the primitive bridges for other AGNOS crates |
| soorat | the visualization data shapes and `HistogramData::from_data` |
| `[lib.ai]` | natural-language queries through hoosh (`StatQuery`), with its five tools |

Every public type has its serde_json codec (the same text as 1.3.0, byte for byte) and Rust's derived Debug.

## Accuracy

Accuracy is part of the contract ([ADR 009](docs/decisions/009-correct-rounding-and-fixed-accuracy-bounds.md)):
- the elementary functions (exp, ln, log1p, expm1, pow, sin, cos, tan, atan, log2) are **correctly rounded**;
- erf and erfc are **correctly rounded**;
- Φ is within 1 ulp, lnΓ within 2, the incomplete gamma within 4 and the incomplete beta within 8, each over its
  stated domain;
- the same inputs give **the same bits on every target** (x86_64 ELF, PE and aarch64).

1.3.0 called glibc, and so carried its last-ulp misrounds and its own special functions. Its lnΓ was up to 4.3×10¹³
ulp off beside 1 and 2, and its incomplete gamma and beta up to 485 and 583. 2.0 matches 1.3.0 bit for bit
everywhere else, and is more accurate where 1.3.0 was wrong: [`docs/development/port-parity.md`](docs/development/port-parity.md)
has the measurements.

## Quick start

```cyrius
include "dist/pramana.cyr"

fn main(): i64 {
    alloc_init();
    var data = alloc(5 * 8);
    for (var i = 0; i < 5; i = i + 1) { store64(data + i * 8, f64_from(i + 1)); }
    var tm, m = pmn_mean(data, 5);           # (Ok or Err, the value or a PMN_ERR_* code)
    var ts, s = pmn_std_dev(data, 5);
    if (tm == Err || ts == Err) { return 1; }
    var normal: PmnNormal;                   # constructors fill caller storage
    if (PmnNormal_new(&normal, m, s) != PMN_ERR_NONE) { return 1; }
    var rng: PmnSimpleRng;
    PmnSimpleRng_new(&rng, 42);
    var x: f64 = normal.sample(&rng);
    return 0;
}

var rc = main();
sys_exit_group(rc);
```

More in [`docs/examples/`](docs/examples/): hypothesis tests, Monte Carlo and MCMC, Markov chains and HMMs, and a
StatQuery. CI compiles and runs each one.

## Using it

- **The core bundle**, [`dist/pramana.cyr`](dist/pramana.cyr). Its header includes the stdlib leaves it needs, and
  `dist/pramana.deps` lists them. Dead-code elimination drops what a consumer doesn't call.
- **The `[lib.ai]` bundle**, [`dist/pramana-ai.cyr`](dist/pramana-ai.cyr): the core plus the AI client. A consumer
  includes `lib/sandhi.cyr` itself ([D8](docs/development/pramana-port-plan.md)).
- **Conventions.** Public names are `pmn_*` (functions), `Pmn<Type>` (structs) and `PMN_*` (constants). Errors are
  `PMN_ERR_*` codes, and owned results have a `_drop`. Results whose length follows from the inputs go into caller
  buffers `(out, cap)`. Callbacks are a function pointer and a context. The migration guide has each rule.

## Building and testing

```sh
cyrius deps                                              # the pinned stdlib (cyrius.cyml)
python3 -I -B scripts/golden/gen.py .                    # the golden corpus: Python's models of 1.3.0 (ADR 010)
python3 -I -B scripts/golden/validate.py . --write-cr    # every row checked; the correctly rounded overlays
cyrius build src/main.cyr build/pramana
cyrius test                                              # the unit tests and the parity replay
cyrius bench
```

The golden corpus (`tests/golden/`, 369,563 rows in 20 modules) is generated, never committed. Its sha256s are in
`scripts/golden/digests.txt`. Python's models of 1.3.0 write it, with glibc through ctypes for 1.3.0's bits, MPFR for
the correctly rounded values and NIST StRD for the reference sets. `tests/parity.tcyr` replays it against the port,
and every row passes at 0 ulp on all three targets. [CONTRIBUTING.md](CONTRIBUTING.md) has the process, and
[`docs/development/state.md`](docs/development/state.md) the current state.

`rust-old/` holds 1.3.0's source, frozen as the parity reference. It is read as text and never built
([ADR 010](docs/decisions/010-python-proof-rust-tooling-removed.md)).

## License

GPL-3.0-only. See [LICENSE](LICENSE).
