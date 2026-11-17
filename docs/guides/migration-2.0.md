# Migrating from pramana 1.x (Rust) to 2.0 (Cyrius)

2.0.0 is pramana 1.3.0 ported to Cyrius. Every function computes what 1.3.0 computed, in 1.3.0's operation order, and
the golden corpus proves it row by row (`tests/parity.tcyr`). Some rows differ on purpose: they are more accurate, as
row 12 and 13 below set out. What changes is the shape of the API, which
[ADR 007](../decisions/007-port-api-divergences.md) records row by row. This guide takes each row in turn. The
appendix maps every public item of 1.3.0 to its 2.0 name.

A Rust project that can't move yet stays on `pramana = "1"`: crates.io keeps 1.3.x.

## Names

| 1.3.0 | 2.0 |
|---|---|
| `descriptive::mean(&data)` | `pmn_mean(data, n)`: a free function is `pmn_<name>` |
| `Normal::new(m, s)` | `PmnNormal_new(&out, m, s)`: a type is `Pmn<Name>`, its constructor `Pmn<Name>_new` |
| `normal.pdf(x)` | `normal.pdf(x)`: methods keep their names |
| `PramanaError::InvalidParameter` | `PMN_ERR_INVALID_PARAMETER`: constants and enum members are `PMN_…` |

Helpers whose names start with `_pmn_` are private, and so are struct fields not listed in a type's docs.

## The changes, by ADR 007 row

**1. Errors are codes.** `Result<T, PramanaError>` becomes a value-form `Result`: `var t, v = pmn_mean(data, n);`,
with `t == Err` and `v` the code on a refusal, else `v` the value. Functions that fill an out-parameter return the
code directly: `PMN_ERR_NONE` or a `PMN_ERR_*`. Each Rust variant has its code (`INVALID_PARAMETER`,
`INVALID_SAMPLE`, `CONVERGENCE_FAILURE`, `DIMENSION_MISMATCH`, `COMPUTATION_ERROR`). 2.0 adds `OUT_OF_MEMORY`, where
Rust would abort, and `DESERIALIZE`, for any document serde_json refuses. A code carries no detail string:
`pmn_err_name(code)` gives the variant's name and `pmn_err_message(code)` Rust's message up to its detail.

```cyrius
var t, v = pmn_median(data, n);
if (t == Err) { println(pmn_err_name(v)); }        # "InvalidSample" for an empty or NaN-bearing slice
```

**2. Constructors fill caller storage.** `let n = Normal::new(0.0, 1.0)?` becomes
`var n: PmnNormal; var e = PmnNormal_new(&n, 0.0, 1.0);`, with `out` untouched on a refusal.

**3. Vectors.** A result whose length follows from the inputs goes into your buffer, `(out, cap)`. Examples are
`moving_average`'s n − w + 1 values, `simulate`'s steps + 1 states and a Viterbi path. A buffer too small is
`INVALID_PARAMETER`, and so is one that overlaps an input. Other results own a freelist block, which you free with
the type's `_drop`. These are `MultivariateNormal`, `KernelDensity`, `PcaResult`, the regression and ARIMA models,
`MarkovChain`, `HiddenMarkovModel`, `McmcResult`, the soorat shapes, `QueryResult` and `StatQuery`. A dropped value
refuses every call, and `_clone` makes a copy with its own block. Slices are a pointer and a count. A
`&[Vec<f64>]` argument is an array of 16-byte (pointer, count) records, and a `Vec<Vec<f64>>` matrix a flat
row-major array with its rows and columns.

**4. u128.** `factorial`, `permutations` and `combinations` write a `PmnU128 { lo, hi }`:
`pmn_combinations(30, 15, &u)`.

**5. Closures.** An `impl Fn` becomes a function pointer and a context word. The integrand is
`f(ctx, x: f64): f64`, MH's log-density `log_target(ctx, xs, dim): f64` and the KS test's CDF `cdf(ctx, x: f64): f64`.
Captured state goes through `ctx`.

```cyrius
fn scaled(ctx, x: f64): f64 { var k: f64 = load64(ctx); var r: f64 = k * x; return r; }
var k[8]; store64(&k, 2.0);
var e = pmn_monte_carlo_integrate(&out, &scaled, &k, 0.0, 1.0, 10000, &rng);
```

**6. RNGs.** `&mut impl Rng` becomes `rng: *R`, generic over the `PmnRng` trait, which `PmnSimpleRng` implements:
`normal.sample(&rng)`. Your own generator implements `PmnRng` (`fn next_f64(self): f64`). A Gibbs conditional is
`cond(ctx, state, dim, rng: *PmnRngRef): f64` and draws with `rng.o.next_f64()`. It is Rust's `&mut dyn Rng`, and
its entries are 16-byte (function, context) records.

**7. JSON.** Every public type has `x.to_json(sb)` and `PmnX_from_json(&out, text, len)`. The text is serde_json's,
byte for byte, and every document 1.3.0 wrote reads back to the same bits. Each decoder goes through the type's
validating constructor, as Rust's `try_from` did.

**8. PramanaError's JSON.** An error code is written as its variant with an empty message
(`{"InvalidParameter":""}`), which 1.x reads. Reading one keeps the variant and drops the message.

**9. HashMaps.** 2.0 has no HashMap, so lists stand in sorted by key, bytewise, where 1.3.0's iteration order was
random. `StatQuery::query`'s columns are 32-byte records (name pointer, name length, values pointer, count), and the
system prompt summarizes them in name order. A `QueryResult`'s values are kept, written and printed in key order,
and a key given twice is refused. `query` is synchronous.

**10. Features.** There are no cargo features. The core bundle `dist/pramana.cyr` carries everything but the AI
client, and dead-code elimination drops what you don't call. `soorat-compat` is in the core. `ai` is the `[lib.ai]`
profile, `dist/pramana-ai.cyr`, whose consumer includes `lib/sandhi.cyr` itself. `logging` was a no-op and is gone.

**11. The same bits on every target.** 1.3.0's results depended on the host's glibc. 2.0's are identical on x86_64 ELF,
PE and aarch64.

**12. Correctly rounded primitives.** exp, ln, log1p, expm1, pow, sin, cos, tan, atan and log2 are correctly rounded,
where glibc misrounds some calls at the last ulp (expm1 one in twenty, the others fewer). A value that depends on one of those can move by an
ulp or more: `*.cr.tsv` lists every golden row where that happens (ADR 009).

**13. The special functions.** erf and erfc are correctly rounded, Φ is within 1 ulp, lnΓ within 2, the incomplete
gamma within 4 and the incomplete beta within 8. 1.3.0 was up to 1,590, 4.3×10¹³, 485 and 583 ulp off. p-values,
cdfs and quantiles built on them move with them, and `*.ref.tsv` lists those rows.

**14. Sorts are stable.** `median`, `percentile`, `quartiles` and `iqr` pick the k-th element of a stable order, so
with −0 and +0 together the zero's sign follows your input order. 1.3.0's unstable selection made it arbitrary.
Eigenvalue ties in the Jacobi keep their order, and so do the KS test's.

**15. Debug.** Rust's `{:?}` and `{:#?}` become `x.debug(sb, alternate)` with `alternate` 0 or 1, the same text. A
String field escapes ASCII as Rust's `str` Debug does, and leaves non-ASCII bytes as they are.

**16. Documents bayan refuses.** serde_json skips an unknown member unread, so it accepts a document whose skipped
member holds a duplicate key, nests past 128 levels or holds a lone surrogate. 2.0's reader (bayan) refuses the whole
document. Every other document reads as 1.3.0 read it.

**17. NaN eigenvalues in PCA.** Overflowing data can make an eigenvalue NaN. Its sign is canonicalized before the
sort, so the component order is the same on every target (1.3.0's followed the hardware's NaN sign).

**18. Counts past 2⁶³.** A `usize` argument at or past 2⁶³ reads negative as a Cyrius i64 and is refused with
`INVALID_PARAMETER`: a run length, a sample count, a burn-in, a bin count or a lag. In 1.3.0 some of these refused and
others started a run of 2⁶³ steps.

## The AI client

hoosh 1.3.0's client sends no tools and returns no tool calls. So 1.3.0's `StatQuery::query` always posted the
question with the data summary and returned the model's direct answer, with operation `direct_answer` and no values.
2.0 keeps that exactly. Its five tools (`describe`, `correlate`, `regress`, `t_test`, `forecast`) are ported and
tested; sending them is planned for 2.1 (the roadmap). The HooshClient is yours to give: `PmnStatQuery_new(&q, base,
nbase, model, nmodel, &pmn_ai_sandhi_transport, 0)` posts through sandhi, and any `transport(ctx, url, nurl, body,
nbody, resp)` can stand in for it, a recorded reply in tests among them. It fills `resp` (32 bytes since 2.0.1): the
status, the body's pointer and length, and a fourth word that is 1 when the body is a freelist block of its own,
which `query` frees once it has read it. sandhi's transport hands its body over that way; a transport that keeps its
body leaves the word 0.

## Floats in arithmetic

Cyrius checks f64 arithmetic by type. Bind every call's result to a `: f64` local before you use it in an operator,
and copy a value out of a Result the same way (`var x: f64 = v;`). The compiler warns otherwise.

## Appendix: every public item of 1.3.0 and its 2.0 form

Generated from `python3 -I -B scripts/surface_parity.py` (CI's `--check` keeps it whole): each item of a
1.3.0 module, by `module::Item`, `Item.method` or `Item#Derive`, and the Cyrius names that port it.

### `error.rs`

| 1.3.0 | 2.0 |
|---|---|
| `error::PramanaError` | enum PmnError, fn pmn_err_name |
| `error::PramanaError#Clone` | a code is a word: copying it clones it (D6) |
| `error::PramanaError#Debug` | fn pmn_err_debug |
| `error::PramanaError#Deserialize` | fn pmn_err_from_json |
| `error::PramanaError#Error` | fn pmn_err_message |
| `error::PramanaError#Serialize` | fn pmn_err_to_json |

### `rng.rs`

| 1.3.0 | 2.0 |
|---|---|
| `rng::Rng` | trait PmnRng |
| `rng::Rng.next_f64` | method PmnRng.next_f64 |
| `rng::SimpleRng` | struct PmnSimpleRng |
| `rng::SimpleRng#Clone` | a one-word struct: copying it clones it |
| `rng::SimpleRng#Debug` | method PmnSimpleRng.debug |
| `rng::SimpleRng#Deserialize` | fn PmnSimpleRng_from_json |
| `rng::SimpleRng#Serialize` | method PmnSimpleRng.to_json |
| `rng::SimpleRng.new` | fn PmnSimpleRng_new |
| `rng::SimpleRng.next_f64` | method PmnSimpleRng.next_f64 |

### `combinatorics.rs`

| 1.3.0 | 2.0 |
|---|---|
| `combinatorics::combinations` | fn pmn_combinations |
| `combinatorics::factorial` | fn pmn_factorial, struct PmnU128 |
| `combinatorics::permutations` | fn pmn_permutations |
| `combinatorics::stirling_approx` | fn pmn_stirling_approx |

### `bridge.rs`

| 1.3.0 | 2.0 |
|---|---|
| `bridge::ensemble_spread_to_confidence` | fn pmn_ensemble_spread_to_confidence |
| `bridge::item_variances_to_alpha` | fn pmn_item_variances_to_alpha |
| `bridge::learning_trials_to_power_law` | fn pmn_learning_trials_to_power_law |
| `bridge::observation_error_to_likelihood` | fn pmn_observation_error_to_likelihood |
| `bridge::replicates_to_t_statistic` | fn pmn_replicates_to_t_statistic |
| `bridge::response_times_to_hick_params` | fn pmn_response_times_to_hick_params |
| `bridge::sensor_noise_to_std_dev` | fn pmn_sensor_noise_to_std_dev |
| `bridge::trait_scores_to_correlation` | fn pmn_trait_scores_to_correlation |
| `bridge::uncertainty_to_gaussian` | fn pmn_uncertainty_to_gaussian |
| `bridge::yields_to_mean_variance` | fn pmn_yields_to_mean_variance |
| `bridge::z_rates_to_d_prime` | fn pmn_z_rates_to_d_prime |

### `distribution.rs`

| 1.3.0 | 2.0 |
|---|---|
| `distribution::Bernoulli` | struct PmnBernoulli |
| `distribution::Bernoulli#Clone` | a struct of one f64: copying it clones it |
| `distribution::Bernoulli#Debug` | method PmnBernoulli.debug |
| `distribution::Bernoulli#Deserialize` | fn PmnBernoulli_from_json |
| `distribution::Bernoulli#Serialize` | method PmnBernoulli.to_json |
| `distribution::Bernoulli.cdf` | method PmnBernoulli.cdf |
| `distribution::Bernoulli.mean` | method PmnBernoulli.mean |
| `distribution::Bernoulli.new` | fn PmnBernoulli_new |
| `distribution::Bernoulli.pdf` | method PmnBernoulli.pdf |
| `distribution::Bernoulli.sample` | method PmnBernoulli.sample |
| `distribution::Bernoulli.variance` | method PmnBernoulli.variance |
| `distribution::Beta` | struct PmnBeta |
| `distribution::Beta#Clone` | a struct of two f64s: copying it clones it |
| `distribution::Beta#Debug` | method PmnBeta.debug |
| `distribution::Beta#Deserialize` | fn PmnBeta_from_json |
| `distribution::Beta#Serialize` | method PmnBeta.to_json |
| `distribution::Beta.cdf` | method PmnBeta.cdf |
| `distribution::Beta.mean` | method PmnBeta.mean |
| `distribution::Beta.new` | fn PmnBeta_new |
| `distribution::Beta.pdf` | method PmnBeta.pdf |
| `distribution::Beta.sample` | method PmnBeta.sample |
| `distribution::Beta.variance` | method PmnBeta.variance |
| `distribution::Binomial` | struct PmnBinomial |
| `distribution::Binomial#Clone` | a struct of a u64 word and an f64: copying it clones it |
| `distribution::Binomial#Debug` | method PmnBinomial.debug |
| `distribution::Binomial#Deserialize` | fn PmnBinomial_from_json |
| `distribution::Binomial#Serialize` | method PmnBinomial.to_json |
| `distribution::Binomial.cdf` | method PmnBinomial.cdf |
| `distribution::Binomial.mean` | method PmnBinomial.mean |
| `distribution::Binomial.new` | fn PmnBinomial_new |
| `distribution::Binomial.pdf` | method PmnBinomial.pdf |
| `distribution::Binomial.sample` | method PmnBinomial.sample |
| `distribution::Binomial.variance` | method PmnBinomial.variance |
| `distribution::Cauchy` | struct PmnCauchy |
| `distribution::Cauchy#Clone` | a struct of two f64s: copying it clones it |
| `distribution::Cauchy#Debug` | method PmnCauchy.debug |
| `distribution::Cauchy#Deserialize` | fn PmnCauchy_from_json |
| `distribution::Cauchy#Serialize` | method PmnCauchy.to_json |
| `distribution::Cauchy.cdf` | method PmnCauchy.cdf |
| `distribution::Cauchy.mean` | method PmnCauchy.mean |
| `distribution::Cauchy.new` | fn PmnCauchy_new |
| `distribution::Cauchy.pdf` | method PmnCauchy.pdf |
| `distribution::Cauchy.sample` | method PmnCauchy.sample |
| `distribution::Cauchy.variance` | method PmnCauchy.variance |
| `distribution::ChiSquared` | struct PmnChiSquared |
| `distribution::ChiSquared#Clone` | a struct of one f64: copying it clones it |
| `distribution::ChiSquared#Debug` | method PmnChiSquared.debug |
| `distribution::ChiSquared#Deserialize` | fn PmnChiSquared_from_json |
| `distribution::ChiSquared#Serialize` | method PmnChiSquared.to_json |
| `distribution::ChiSquared.cdf` | method PmnChiSquared.cdf |
| `distribution::ChiSquared.mean` | method PmnChiSquared.mean |
| `distribution::ChiSquared.new` | fn PmnChiSquared_new |
| `distribution::ChiSquared.pdf` | method PmnChiSquared.pdf |
| `distribution::ChiSquared.sample` | method PmnChiSquared.sample |
| `distribution::ChiSquared.variance` | method PmnChiSquared.variance |
| `distribution::Distribution` | trait PmnDistribution |
| `distribution::Distribution.cdf` | method PmnDistribution.cdf |
| `distribution::Distribution.mean` | method PmnDistribution.mean |
| `distribution::Distribution.pdf` | method PmnDistribution.pdf |
| `distribution::Distribution.sample` | method PmnDistribution.sample |
| `distribution::Distribution.variance` | method PmnDistribution.variance |
| `distribution::Exponential` | struct PmnExponential |
| `distribution::Exponential#Clone` | a struct of one f64: copying it clones it |
| `distribution::Exponential#Debug` | method PmnExponential.debug |
| `distribution::Exponential#Deserialize` | fn PmnExponential_from_json |
| `distribution::Exponential#Serialize` | method PmnExponential.to_json |
| `distribution::Exponential.cdf` | method PmnExponential.cdf |
| `distribution::Exponential.mean` | method PmnExponential.mean |
| `distribution::Exponential.new` | fn PmnExponential_new |
| `distribution::Exponential.pdf` | method PmnExponential.pdf |
| `distribution::Exponential.sample` | method PmnExponential.sample |
| `distribution::Exponential.variance` | method PmnExponential.variance |
| `distribution::FDistribution` | struct PmnFDistribution |
| `distribution::FDistribution#Clone` | a struct of two f64s: copying it clones it |
| `distribution::FDistribution#Debug` | method PmnFDistribution.debug |
| `distribution::FDistribution#Deserialize` | fn PmnFDistribution_from_json |
| `distribution::FDistribution#Serialize` | method PmnFDistribution.to_json |
| `distribution::FDistribution.cdf` | method PmnFDistribution.cdf |
| `distribution::FDistribution.mean` | method PmnFDistribution.mean |
| `distribution::FDistribution.new` | fn PmnFDistribution_new |
| `distribution::FDistribution.pdf` | method PmnFDistribution.pdf |
| `distribution::FDistribution.sample` | method PmnFDistribution.sample |
| `distribution::FDistribution.variance` | method PmnFDistribution.variance |
| `distribution::Gamma` | struct PmnGamma |
| `distribution::Gamma#Clone` | a struct of two f64s: copying it clones it |
| `distribution::Gamma#Debug` | method PmnGamma.debug |
| `distribution::Gamma#Deserialize` | fn PmnGamma_from_json |
| `distribution::Gamma#Serialize` | method PmnGamma.to_json |
| `distribution::Gamma.cdf` | method PmnGamma.cdf |
| `distribution::Gamma.mean` | method PmnGamma.mean |
| `distribution::Gamma.new` | fn PmnGamma_new |
| `distribution::Gamma.pdf` | method PmnGamma.pdf |
| `distribution::Gamma.sample` | method PmnGamma.sample |
| `distribution::Gamma.variance` | method PmnGamma.variance |
| `distribution::MultivariateNormal` | struct PmnMultivariateNormal, fn PmnMultivariateNormal_drop |
| `distribution::MultivariateNormal#Clone` | fn PmnMultivariateNormal_clone |
| `distribution::MultivariateNormal#Debug` | method PmnMultivariateNormal.debug |
| `distribution::MultivariateNormal#Deserialize` | fn PmnMultivariateNormal_from_json |
| `distribution::MultivariateNormal#Serialize` | method PmnMultivariateNormal.to_json |
| `distribution::MultivariateNormal.covariance_matrix` | method PmnMultivariateNormal.covariance_matrix |
| `distribution::MultivariateNormal.dim` | method PmnMultivariateNormal.dim |
| `distribution::MultivariateNormal.log_pdf` | method PmnMultivariateNormal.log_pdf |
| `distribution::MultivariateNormal.mean_vec` | method PmnMultivariateNormal.mean_vec |
| `distribution::MultivariateNormal.new` | fn PmnMultivariateNormal_new |
| `distribution::MultivariateNormal.pdf` | method PmnMultivariateNormal.pdf |
| `distribution::MultivariateNormal.sample` | method PmnMultivariateNormal.sample |
| `distribution::Normal` | struct PmnNormal |
| `distribution::Normal#Clone` | a struct of two f64s: copying it clones it |
| `distribution::Normal#Debug` | method PmnNormal.debug |
| `distribution::Normal#Deserialize` | fn PmnNormal_from_json |
| `distribution::Normal#Serialize` | method PmnNormal.to_json |
| `distribution::Normal.cdf` | method PmnNormal.cdf |
| `distribution::Normal.mean` | method PmnNormal.mean |
| `distribution::Normal.new` | fn PmnNormal_new |
| `distribution::Normal.pdf` | method PmnNormal.pdf |
| `distribution::Normal.sample` | method PmnNormal.sample |
| `distribution::Normal.variance` | method PmnNormal.variance |
| `distribution::Poisson` | struct PmnPoisson |
| `distribution::Poisson#Clone` | a struct of one f64: copying it clones it |
| `distribution::Poisson#Debug` | method PmnPoisson.debug |
| `distribution::Poisson#Deserialize` | fn PmnPoisson_from_json |
| `distribution::Poisson#Serialize` | method PmnPoisson.to_json |
| `distribution::Poisson.cdf` | method PmnPoisson.cdf |
| `distribution::Poisson.mean` | method PmnPoisson.mean |
| `distribution::Poisson.new` | fn PmnPoisson_new |
| `distribution::Poisson.pdf` | method PmnPoisson.pdf |
| `distribution::Poisson.sample` | method PmnPoisson.sample |
| `distribution::Poisson.variance` | method PmnPoisson.variance |
| `distribution::StudentT` | struct PmnStudentT |
| `distribution::StudentT#Clone` | a struct of one f64: copying it clones it |
| `distribution::StudentT#Debug` | method PmnStudentT.debug |
| `distribution::StudentT#Deserialize` | fn PmnStudentT_from_json |
| `distribution::StudentT#Serialize` | method PmnStudentT.to_json |
| `distribution::StudentT.cdf` | method PmnStudentT.cdf |
| `distribution::StudentT.mean` | method PmnStudentT.mean |
| `distribution::StudentT.new` | fn PmnStudentT_new |
| `distribution::StudentT.pdf` | method PmnStudentT.pdf |
| `distribution::StudentT.sample` | method PmnStudentT.sample |
| `distribution::StudentT.variance` | method PmnStudentT.variance |
| `distribution::Uniform` | struct PmnUniform |
| `distribution::Uniform#Clone` | a struct of two f64s: copying it clones it |
| `distribution::Uniform#Debug` | method PmnUniform.debug |
| `distribution::Uniform#Deserialize` | fn PmnUniform_from_json |
| `distribution::Uniform#Serialize` | method PmnUniform.to_json |
| `distribution::Uniform.cdf` | method PmnUniform.cdf |
| `distribution::Uniform.mean` | method PmnUniform.mean |
| `distribution::Uniform.new` | fn PmnUniform_new |
| `distribution::Uniform.pdf` | method PmnUniform.pdf |
| `distribution::Uniform.sample` | method PmnUniform.sample |
| `distribution::Uniform.variance` | method PmnUniform.variance |
| `distribution::Weibull` | struct PmnWeibull |
| `distribution::Weibull#Clone` | a struct of two f64s: copying it clones it |
| `distribution::Weibull#Debug` | method PmnWeibull.debug |
| `distribution::Weibull#Deserialize` | fn PmnWeibull_from_json |
| `distribution::Weibull#Serialize` | method PmnWeibull.to_json |
| `distribution::Weibull.cdf` | method PmnWeibull.cdf |
| `distribution::Weibull.mean` | method PmnWeibull.mean |
| `distribution::Weibull.new` | fn PmnWeibull_new |
| `distribution::Weibull.pdf` | method PmnWeibull.pdf |
| `distribution::Weibull.sample` | method PmnWeibull.sample |
| `distribution::Weibull.variance` | method PmnWeibull.variance |

### `descriptive.rs`

| 1.3.0 | 2.0 |
|---|---|
| `descriptive::Kernel` | enum PmnKernel, fn pmn_kernel_name |
| `descriptive::Kernel#Clone` | a kernel is a word: copying it clones it |
| `descriptive::Kernel#Debug` | fn pmn_kernel_debug |
| `descriptive::Kernel#Deserialize` | fn pmn_kernel_from_json |
| `descriptive::Kernel#Serialize` | fn pmn_kernel_to_json |
| `descriptive::KernelDensity` | struct PmnKernelDensity, fn PmnKernelDensity_drop |
| `descriptive::KernelDensity#Clone` | fn PmnKernelDensity_clone |
| `descriptive::KernelDensity#Debug` | method PmnKernelDensity.debug |
| `descriptive::KernelDensity#Deserialize` | fn PmnKernelDensity_from_json |
| `descriptive::KernelDensity#Serialize` | method PmnKernelDensity.to_json |
| `descriptive::KernelDensity.evaluate` | method PmnKernelDensity.evaluate |
| `descriptive::KernelDensity.evaluate_grid` | method PmnKernelDensity.evaluate_grid |
| `descriptive::KernelDensity.new` | fn PmnKernelDensity_new |
| `descriptive::KernelDensity.silverman` | fn PmnKernelDensity_silverman |
| `descriptive::PcaResult` | struct PmnPcaResult, fn PmnPcaResult_drop, method PmnPcaResult.component, method PmnPcaResult.eigenvalues, method PmnPcaResult.explained_variance_ratio |
| `descriptive::PcaResult#Clone` | fn PmnPcaResult_clone |
| `descriptive::PcaResult#Debug` | method PmnPcaResult.debug |
| `descriptive::PcaResult#Deserialize` | fn PmnPcaResult_from_json |
| `descriptive::PcaResult#Serialize` | method PmnPcaResult.to_json |
| `descriptive::correlation_matrix` | fn pmn_correlation_matrix |
| `descriptive::iqr` | fn pmn_iqr |
| `descriptive::kurtosis` | fn pmn_kurtosis |
| `descriptive::max` | fn pmn_max |
| `descriptive::mean` | fn pmn_mean |
| `descriptive::median` | fn pmn_median, fn _pmn_select_pair |
| `descriptive::min` | fn pmn_min |
| `descriptive::mode` | fn pmn_mode |
| `descriptive::pca` | fn pmn_pca, fn _pmn_eigen_symmetric |
| `descriptive::percentile` | fn pmn_percentile |
| `descriptive::quartiles` | fn pmn_quartiles |
| `descriptive::range` | fn pmn_range |
| `descriptive::skewness` | fn pmn_skewness |
| `descriptive::std_dev` | fn pmn_std_dev |
| `descriptive::sum` | fn pmn_sum |
| `descriptive::variance` | fn pmn_variance |

### `hypothesis.rs`

| 1.3.0 | 2.0 |
|---|---|
| `hypothesis::AnovaResult` | struct PmnAnovaResult |
| `hypothesis::AnovaResult#Clone` | a plain struct: copying it clones it |
| `hypothesis::AnovaResult#Debug` | method PmnAnovaResult.debug |
| `hypothesis::AnovaResult#Deserialize` | fn PmnAnovaResult_from_json |
| `hypothesis::AnovaResult#Serialize` | method PmnAnovaResult.to_json |
| `hypothesis::ConfidenceInterval` | struct PmnConfidenceInterval |
| `hypothesis::ConfidenceInterval#Clone` | a plain struct: copying it clones it |
| `hypothesis::ConfidenceInterval#Debug` | method PmnConfidenceInterval.debug |
| `hypothesis::ConfidenceInterval#Deserialize` | fn PmnConfidenceInterval_from_json |
| `hypothesis::ConfidenceInterval#Serialize` | method PmnConfidenceInterval.to_json |
| `hypothesis::KsResult` | struct PmnKsResult |
| `hypothesis::KsResult#Clone` | a plain struct: copying it clones it |
| `hypothesis::KsResult#Debug` | method PmnKsResult.debug |
| `hypothesis::KsResult#Deserialize` | fn PmnKsResult_from_json |
| `hypothesis::KsResult#Serialize` | method PmnKsResult.to_json |
| `hypothesis::TestResult` | struct PmnTestResult, fn PmnTestResult_drop |
| `hypothesis::TestResult#Clone` | fn PmnTestResult_clone |
| `hypothesis::TestResult#Debug` | method PmnTestResult.debug |
| `hypothesis::TestResult#Deserialize` | fn PmnTestResult_from_json |
| `hypothesis::TestResult#Serialize` | method PmnTestResult.to_json |
| `hypothesis::chi_squared_test` | fn pmn_chi_squared_test |
| `hypothesis::ci_mean` | fn pmn_ci_mean, fn _pmn_t_quantile |
| `hypothesis::ci_proportion` | fn pmn_ci_proportion, fn _pmn_z_quantile |
| `hypothesis::ci_two_means` | fn pmn_ci_two_means |
| `hypothesis::ks_one_sample` | fn pmn_ks_one_sample |
| `hypothesis::ks_two_sample` | fn pmn_ks_two_sample, fn _pmn_kolmogorov_sf |
| `hypothesis::one_way_anova` | fn pmn_one_way_anova |
| `hypothesis::t_test_one_sample` | fn pmn_t_test_one_sample |
| `hypothesis::t_test_two_sample` | fn pmn_t_test_two_sample, fn _pmn_welch |

### `regression.rs`

| 1.3.0 | 2.0 |
|---|---|
| `regression::LinearModel` | struct PmnLinearModel |
| `regression::LinearModel#Clone` | a plain struct: copying it clones it |
| `regression::LinearModel#Debug` | method PmnLinearModel.debug |
| `regression::LinearModel#Deserialize` | fn PmnLinearModel_from_json |
| `regression::LinearModel#Serialize` | method PmnLinearModel.to_json |
| `regression::LogisticModel` | struct PmnLogisticModel, fn PmnLogisticModel_drop, method PmnLogisticModel.coefficients |
| `regression::LogisticModel#Clone` | fn PmnLogisticModel_clone |
| `regression::LogisticModel#Debug` | method PmnLogisticModel.debug |
| `regression::LogisticModel#Deserialize` | fn PmnLogisticModel_from_json |
| `regression::LogisticModel#Serialize` | method PmnLogisticModel.to_json |
| `regression::PolynomialModel` | struct PmnPolynomialModel, fn PmnPolynomialModel_drop, method PmnPolynomialModel.coefficients |
| `regression::PolynomialModel#Clone` | fn PmnPolynomialModel_clone |
| `regression::PolynomialModel#Debug` | method PmnPolynomialModel.debug |
| `regression::PolynomialModel#Deserialize` | fn PmnPolynomialModel_from_json |
| `regression::PolynomialModel#Serialize` | method PmnPolynomialModel.to_json |
| `regression::linear_regression` | fn pmn_linear_regression |
| `regression::logistic_regression` | fn pmn_logistic_regression, fn _pmn_sigmoid |
| `regression::polynomial_regression` | fn pmn_polynomial_regression, fn _pmn_least_squares_poly |
| `regression::predict` | fn pmn_predict |
| `regression::predict_logistic_class` | fn pmn_predict_logistic_class |
| `regression::predict_logistic_proba` | fn pmn_predict_logistic_proba |
| `regression::predict_poly` | fn pmn_predict_poly |
| `regression::residuals` | fn pmn_residuals |
| `regression::residuals_poly` | fn pmn_residuals_poly |

### `timeseries.rs`

| 1.3.0 | 2.0 |
|---|---|
| `timeseries::ArimaModel` | struct PmnArimaModel, fn PmnArimaModel_drop, method PmnArimaModel.ar_coefficients |
| `timeseries::ArimaModel#Clone` | fn PmnArimaModel_clone |
| `timeseries::ArimaModel#Debug` | method PmnArimaModel.debug |
| `timeseries::ArimaModel#Deserialize` | fn PmnArimaModel_from_json |
| `timeseries::ArimaModel#Serialize` | method PmnArimaModel.to_json |
| `timeseries::arima_fit` | fn pmn_arima_fit |
| `timeseries::arima_forecast` | fn pmn_arima_forecast |
| `timeseries::autocorrelation` | fn pmn_autocorrelation |
| `timeseries::difference` | fn pmn_difference |
| `timeseries::exponential_smoothing` | fn pmn_exponential_smoothing |
| `timeseries::integrate` | fn pmn_integrate |
| `timeseries::moving_average` | fn pmn_moving_average |

### `bayesian.rs`

| 1.3.0 | 2.0 |
|---|---|
| `bayesian::BayesianUpdate` | struct PmnBayesianUpdate |
| `bayesian::BayesianUpdate#Clone` | a plain struct: copying it clones it |
| `bayesian::BayesianUpdate#Debug` | method PmnBayesianUpdate.debug |
| `bayesian::BayesianUpdate#Deserialize` | fn PmnBayesianUpdate_from_json |
| `bayesian::BayesianUpdate#Serialize` | method PmnBayesianUpdate.to_json |
| `bayesian::BayesianUpdate.new` | fn PmnBayesianUpdate_new |
| `bayesian::BayesianUpdate.posterior` | method PmnBayesianUpdate.posterior |
| `bayesian::bayes_theorem` | fn pmn_bayes_theorem |
| `bayesian::naive_bayes_classify` | fn pmn_naive_bayes_classify |

### `markov.rs`

| 1.3.0 | 2.0 |
|---|---|
| `markov::HiddenMarkovModel` | struct PmnHiddenMarkovModel, fn PmnHiddenMarkovModel_drop, method PmnHiddenMarkovModel.transition, method PmnHiddenMarkovModel.emission, method PmnHiddenMarkovModel.initial |
| `markov::HiddenMarkovModel#Clone` | fn PmnHiddenMarkovModel_clone |
| `markov::HiddenMarkovModel#Debug` | method PmnHiddenMarkovModel.debug |
| `markov::HiddenMarkovModel#Deserialize` | fn PmnHiddenMarkovModel_from_json |
| `markov::HiddenMarkovModel#Serialize` | method PmnHiddenMarkovModel.to_json |
| `markov::HiddenMarkovModel.baum_welch` | method PmnHiddenMarkovModel.baum_welch |
| `markov::HiddenMarkovModel.forward_log_likelihood` | method PmnHiddenMarkovModel.forward_log_likelihood |
| `markov::HiddenMarkovModel.new` | fn PmnHiddenMarkovModel_new |
| `markov::HiddenMarkovModel.num_states` | method PmnHiddenMarkovModel.num_states |
| `markov::HiddenMarkovModel.num_symbols` | method PmnHiddenMarkovModel.num_symbols |
| `markov::HiddenMarkovModel.viterbi` | method PmnHiddenMarkovModel.viterbi |
| `markov::MarkovChain` | struct PmnMarkovChain, fn PmnMarkovChain_drop, method PmnMarkovChain.transition_matrix |
| `markov::MarkovChain#Clone` | fn PmnMarkovChain_clone |
| `markov::MarkovChain#Debug` | method PmnMarkovChain.debug |
| `markov::MarkovChain#Deserialize` | fn PmnMarkovChain_from_json |
| `markov::MarkovChain#Serialize` | method PmnMarkovChain.to_json |
| `markov::MarkovChain.new` | fn PmnMarkovChain_new |
| `markov::MarkovChain.num_states` | method PmnMarkovChain.num_states |
| `markov::MarkovChain.simulate` | method PmnMarkovChain.simulate |
| `markov::MarkovChain.steady_state` | method PmnMarkovChain.steady_state |
| `markov::MarkovChain.step` | method PmnMarkovChain.step |

### `monte_carlo.rs`

| 1.3.0 | 2.0 |
|---|---|
| `monte_carlo::EstimateResult` | struct PmnEstimateResult |
| `monte_carlo::EstimateResult#Clone` | a plain struct: copying it clones it |
| `monte_carlo::EstimateResult#Debug` | method PmnEstimateResult.debug |
| `monte_carlo::EstimateResult#Deserialize` | fn PmnEstimateResult_from_json |
| `monte_carlo::EstimateResult#Serialize` | method PmnEstimateResult.to_json |
| `monte_carlo::McmcResult` | struct PmnMcmcResult, fn PmnMcmcResult_drop, method PmnMcmcResult.n_samples, method PmnMcmcResult.sample, method PmnMcmcResult.sample_len, method PmnMcmcResult.samples |
| `monte_carlo::McmcResult#Clone` | fn PmnMcmcResult_clone |
| `monte_carlo::McmcResult#Debug` | method PmnMcmcResult.debug |
| `monte_carlo::McmcResult#Deserialize` | fn PmnMcmcResult_from_json |
| `monte_carlo::McmcResult#Serialize` | method PmnMcmcResult.to_json |
| `monte_carlo::gibbs_sampling` | fn pmn_gibbs_sampling, struct PmnRngRef |
| `monte_carlo::metropolis_hastings` | fn pmn_metropolis_hastings |
| `monte_carlo::monte_carlo_integrate` | fn pmn_monte_carlo_integrate |
| `monte_carlo::monte_carlo_pi` | fn pmn_monte_carlo_pi |

### `integration/soorat.rs`

| 1.3.0 | 2.0 |
|---|---|
| `integration/soorat::CorrelationMatrix` | struct PmnCorrelationMatrix, fn PmnCorrelationMatrix_new, fn PmnCorrelationMatrix_drop |
| `integration/soorat::CorrelationMatrix#Clone` | fn PmnCorrelationMatrix_clone |
| `integration/soorat::CorrelationMatrix#Debug` | method PmnCorrelationMatrix.debug |
| `integration/soorat::CorrelationMatrix#Deserialize` | fn PmnCorrelationMatrix_from_json |
| `integration/soorat::CorrelationMatrix#PartialEq` | fn PmnCorrelationMatrix_eq |
| `integration/soorat::CorrelationMatrix#Serialize` | method PmnCorrelationMatrix.to_json |
| `integration/soorat::DistributionCurve` | struct PmnDistributionCurve, fn PmnDistributionCurve_new, fn PmnDistributionCurve_drop |
| `integration/soorat::DistributionCurve#Clone` | fn PmnDistributionCurve_clone |
| `integration/soorat::DistributionCurve#Debug` | method PmnDistributionCurve.debug |
| `integration/soorat::DistributionCurve#Deserialize` | fn PmnDistributionCurve_from_json |
| `integration/soorat::DistributionCurve#PartialEq` | fn PmnDistributionCurve_eq |
| `integration/soorat::DistributionCurve#Serialize` | method PmnDistributionCurve.to_json |
| `integration/soorat::HistogramData` | struct PmnHistogramData, fn PmnHistogramData_new, fn PmnHistogramData_drop |
| `integration/soorat::HistogramData#Clone` | fn PmnHistogramData_clone |
| `integration/soorat::HistogramData#Debug` | method PmnHistogramData.debug |
| `integration/soorat::HistogramData#Deserialize` | fn PmnHistogramData_from_json |
| `integration/soorat::HistogramData#PartialEq` | fn PmnHistogramData_eq |
| `integration/soorat::HistogramData#Serialize` | method PmnHistogramData.to_json |
| `integration/soorat::HistogramData.from_data` | fn PmnHistogramData_from_data |
| `integration/soorat::McmcTrace` | struct PmnMcmcTrace, fn PmnMcmcTrace_new, fn PmnMcmcTrace_drop |
| `integration/soorat::McmcTrace#Clone` | fn PmnMcmcTrace_clone |
| `integration/soorat::McmcTrace#Debug` | method PmnMcmcTrace.debug |
| `integration/soorat::McmcTrace#Deserialize` | fn PmnMcmcTrace_from_json |
| `integration/soorat::McmcTrace#PartialEq` | fn PmnMcmcTrace_eq |
| `integration/soorat::McmcTrace#Serialize` | method PmnMcmcTrace.to_json |
| `integration/soorat::RegressionFit` | struct PmnRegressionFit, fn PmnRegressionFit_new, fn PmnRegressionFit_drop |
| `integration/soorat::RegressionFit#Clone` | fn PmnRegressionFit_clone |
| `integration/soorat::RegressionFit#Debug` | method PmnRegressionFit.debug |
| `integration/soorat::RegressionFit#Deserialize` | fn PmnRegressionFit_from_json |
| `integration/soorat::RegressionFit#PartialEq` | fn PmnRegressionFit_eq |
| `integration/soorat::RegressionFit#Serialize` | method PmnRegressionFit.to_json |

### `ai.rs`

| 1.3.0 | 2.0 |
|---|---|
| `ai::QueryResult` | struct PmnQueryResult, fn PmnQueryResult_new, fn PmnQueryResult_drop, method PmnQueryResult.value |
| `ai::QueryResult#Clone` | fn PmnQueryResult_clone |
| `ai::QueryResult#Debug` | method PmnQueryResult.debug |
| `ai::QueryResult#Deserialize` | fn PmnQueryResult_from_json |
| `ai::QueryResult#Serialize` | method PmnQueryResult.to_json |
| `ai::StatQuery` | struct PmnStatQuery, fn PmnStatQuery_drop, fn pmn_ai_sandhi_transport |
| `ai::StatQuery.new` | fn PmnStatQuery_new |
| `ai::StatQuery.query` | method PmnStatQuery.query |
