# pramana Rust → Cyrius port plan

> **Plan of record (draft, 2026-11-15)** for porting pramana to Cyrius. Rust **1.3.0** is the final Rust release, and its
> functionality is what the port carries. The Cyrius port ships as **2.0.0**. The method is the one the sibling ports
> converged on:
> - tanmatra's Rust oracle and parity classes ([its ADR 0006](../../../tanmatra/docs/adr/0006-cyrius-port-with-rust-oracle.md));
> - kimiya's accuracy policy, "correct rounding is the accuracy target" ([its ADR 0006](../../../kimiya/docs/adr/0006-correct-rounding-is-the-accuracy-target.md));
> - pravash's tooling, JSON codecs and gates ([its port plan](../../../pravash/docs/development/pravash-port-plan.md), shipped 2.0.0 on 2026-11-13).
>
> Where this plan says "as pravash", "as kimiya" or "as tanmatra", follow that document.
>
> **How to read it:**
> - §1.2 is the language review that every scope decision rests on.
> - §2 lists what is different about pramana.
> - §3 answers how accurate the Cyrius port can be against Rust. It starts from a measured baseline of Rust 1.3.0 against MPFR.
>
> ▶ **Status: every decision is taken (§0.2, 2026-11-15). M0 steps 0 and 1 are done:**
> - step 0, the Rust hooks, is committed as `94a7153`;
> - step 1, the `cyrius port` scaffold, is committed as `d8e761d`.
>
> **Step 2,** the repairs, is committed as `40a8269`, and **step 3**, the full `cyrius.cyml`, as `0e147d3`.
> **Steps 4–8** are committed (`53078cb`, `30eb41b`, `bb0f938`, `d7941bc`, `aec9861`). **Step 9,** the probes, is
> done: part 9a (P1, P5, P6) is committed (`96ae9d5`, and P6's decision, ADR 007 row 14, as `aa3dc1e`). Part 9b
> (P2, P3, P4) and the D7 amendment it led to (the RNG is generic) are committed as `4a051bf`. Part 9c (P7, P8) is
> committed as `155dce0`. After it, the maintainer amended D5 (serde_json's text, byte for byte) and accepted ADR 007
> row 16. Parts 9d-1 and 9d-2, P9's `ln_gamma` and `gamma_inc` prototypes, are committed (`998b64b`, `d424b90`,
> `2f572be`), and so is part 9d-3, `beta_inc` (`121e182`). Part 9e, the FMA-free CORE-MATH erf/erfc port, is
> committed (`ea7f1a0`): it is correctly rounded, so D3 = C holds without the fallback. So is part 9d-4, the
> `normal_cdf` prototype on it, within 0.5 ulp (`26f04f2`), and step 10, the upstream requests and the ledger
> (`aa5df35`): **M0 is complete.** **M1** is in progress: part a (the error codes) and part b1 (kimiya's correctly
> rounded primitives, vendored) are committed (`1f718c6`), and so is part b2, a new correctly rounded log1p
> (`f08f233`) and part b3, a new correctly rounded tan (`3257918`). Part b4, a new correctly rounded log2, is done
> but not committed; with it every class-C math row is at 0 ulp, and the primitives are verified on 11.6M MPFR rows. The maintainer took each
> recommendation and chose option C for D3. Nothing has been ported. The one artifact so far is the accuracy-baseline
> tool, `scripts/port-baseline/`, which produced §3.2.

| Thing | Value | Source |
|---|---|---|
| Rust source | **pramana 1.3.0**, tagged at `284eb18` (2026-11-14). `release.yml` runs `cargo publish` on any tag (the crates.io upload is not confirmed here). 4,454 code lines in `src/` (no tests, comments or blank lines); 14 modules plus `integration/soorat.rs`. 102 `pub fn`s, plus 65 trait methods (13 `Distribution` impls × 5). 2 public traits (`Distribution`, `Rng`), 2 public enums (`PramanaError`, `Kernel`). 38 `Serialize` types, 20 of them with validated deserialization (`#[serde(try_from)]` / `from`) | `Cargo.toml`, `git tag`, `grep` of `src/` |
| Features | default empty; `ai` (hoosh 1.3, tokio, serde_json); `soorat-compat`; `logging`, a reserved no-op since 1.3.0 | `Cargo.toml` |
| Rust health, 2026-11-14 on rustc 1.100.0 | All gates green: fmt; clippy `-D warnings` (all features, defaults); rustdoc `-D warnings`; audit; deny; publish dry-run. Tests: 301 unit + 15 integration + 3 doc = 319 with `--all-features`. 34 criterion benches | 1.3.0 hardening sweep, `CHANGELOG.md` |
| Correctness baseline | The 1.3.0 P(-1) audit. Every fix has a regression test. It covers the NaN sort panic, deserialize panics, O(x²) and O(n) hangs, the large-df χ² CDF, HMM underflow, KS ties, `integrate` d ≥ 2 and others | `CHANGELOG.md` 1.3.0 |
| Cyrius toolchain | **6.7.18** (installed and current) | `~/.cyrius/current` |
| ganita | **1.2.15**, bundled in the 6.7.18 stdlib | `~/.cyrius/versions/6.7.18/lib/ganita.cyr` |
| hisab (Cyrius) | **3.5.0** (siblings pin 3.4.1; H9, the allocator-scoped calls, closed in 3.4.2). §8.2 expects pramana to need **no** hisab in `[lib]` | `/home/macro/Repos/hisab` |
| Rust 1.3.0's dependencies | hisab **1.4.0** (`num`): `cholesky`, `cholesky_solve`, `least_squares_poly` (modified Gram–Schmidt QR), `eigen_symmetric` (Jacobi, though its doc says QR). `Cargo.lock` is **tracked** | `Cargo.lock`, hisab 1.4.0 source |
| Rust 1.3.0's math | Rust std f64 methods, which call **glibc 2.44** for: `ln` (53 calls), `exp` (32), `ln_1p` (11), `powf` (7), `cos` (3), `sin` (2), `exp_m1` (2), `tan` (1), `atan` (1), `log2` (1). Built-ins: `sqrt` (25), `powi` (17), `cbrt` (1; compiler_builtins, correctly rounded per pravash). pramana's own special functions are in `src/math.rs`: an fdlibm erf/erfc port, Lanczos `ln_gamma`, incomplete gamma/beta | `grep` of `src/` |
| Dependents | **bodh, jivanu, sangha**: all Rust 1.0.0, each with an optional `pramana = "1"`. They stay on crates.io 1.3.x. No Cyrius crate depends on pramana | `~/Repos/*/Cargo.toml` |
| `ai` counterpart | **hoosh 2.12.3 is Cyrius, but it is a gateway binary.** Its API is OpenAI-compatible `/v1/chat/completions` on :8088, and there is no client library (`hoosh/docs/development/rust-old-retirement.md:67`) | `/home/macro/Repos/hoosh` |
| Target | pramana **2.0.0**, Cyrius, carrying **every** module and feature (§4) | this document |

---

## 0. Decisions

### 0.1 Carried from the sibling ports

- **No deferral without a language check.** A Rust feature leaves the 2.0 scope only for a reason written against
  §1.2: a capability 6.7.18 lacks, a measured cost, or nothing to test against. The review below found no language
  reason to drop any pramana module or feature.
- **The Rust crate is frozen.** 1.3.0 moves to `rust-old/` and is never edited, except for the read-only `oracle`
  hooks (D1). A generator crate in `scripts/golden/` drives it by path. If a value 1.3.0 records is wrong, it is
  fixed in Rust as 1.3.x and the corpus is regenerated (as tanmatra).
- **No Rust 2.0.** API shape changes land in the Cyrius API only, and ADR 0007 accepts them (§4.3).
- **Upstream requests never block the port.** Every gap in ganita, hisab, bayan or the stdlib is filed in the owning
  repo with pramana's numbers. pramana ships a local stand-in marked `UPSTREAM(Ln)`, with a row in the conversion
  ledger (§10, `scripts/upstream_ledger.py --check` in CI). The stand-in is converted when the library ships.
- **Parity is functional, and accuracy beats glibc** (the maintainer, 2026-11-09: "WE WANT TO BE BETTER THAT glibc").
  - Every transcendental is correctly rounded and gives the same bits on every target.
  - The golden expectation where glibc misrounds is Rust 1.3.0's formula evaluated on correctly rounded primitives
    (`*.cr.tsv`), compared at **0 ulp**. A port value that matches glibc's misround is a failure.
  - §3 extends this to pramana's own special functions (D3).
- **Performance is measured, not gated.** Every Rust bench gets a Cyrius twin, and both columns are recorded per
  milestone in `docs/benchmarks-rust-v-cyrius.md`. A slow row is a finding to file, with its numbers.
- **Three targets at every milestone:** x86_64 ELF, x86_64 PE (wine) and aarch64 (qemu). pravash's PE `chdir` gap
  surfaced only when its release gate first ran every suite cross-target.

### 0.2 Decided 2026-11-15 (the maintainer took every recommendation, and option C for D3)

Each row gives the decision and the alternatives not chosen.

| # | Question | Decision | Not chosen |
|---|---|---|---|
| D1 | ✅ **The freeze point.** 1.3.0 is already tagged (`284eb18`, and so published by `release.yml`), without the `oracle` hooks the generator needs: the 14 `pub(crate)` functions of `math.rs`, the private samplers and quantiles, and `MultivariateNormal`'s private factorization | Land the read-only `oracle` feature on main after the tag, as unreleased tooling (as impetus did). It moves into `rust-old/` with the crate and is never published. `tests/oracle.rs` proves it read-only | Publish 1.3.1 with the hooks (no value to crates.io users). Or port without hooks: every `math.rs` row would then be reachable only through public pdf/cdf calls, losing per-function parity |
| D2 | ✅ **Correctly rounded primitives.** pramana needs exp, ln, ln_1p, exp_m1, pow, sin, cos, tan, atan and log2. kimiya's `src/mathx.cyr` has exp, expm1, ln, log10, pow, sin, cos, sincos, atan2 and acos, with `cr_verify.py` and `mpfr_ref.py`. **No sibling has a public log1p, tan or log2.** ushma 2.0.0 has a triple-double log1p kernel inside its ln (`ushma/src/mathx.cyr:819`), which is a starting point | Vendor kimiya's two-phase primitives (L1). By the two crates' own benchmarks they are faster than the impetus-derived crmath the ganita issue measures (§1.3): exp 39 against 110 ns, pow about 135 against 757 ns, atan2 111 ns against 1.41 µs. Compute atan(x) as the correctly rounded atan2(x, 1), which is the same value. Build correctly rounded log1p, tan and log2 with kimiya's method, verified on ≥ 1M MPFR arguments each. Join the open ganita correctly-rounded-tier issue as a consumer, adding the three new functions | ganita's private fdlibm `_gn_log1p` / `_gn_expm1` (≤ 0.87 ulp) and `ganita_f64_tan` (≤ 1 ulp): not correctly rounded, so not "better than glibc" |
| D3 | ✅ **How accurate the special functions are** (§3). Correctly rounded primitives alone leave Rust's special-function errors in place: up to 1.2×10⁶ ulp in `ln_gamma`, 1,590 ulp in `normal_cdf` tails, and 583 ulp in `beta_inc` | **Decided 2026-11-15: option C.** M2 part a ports `math.rs` in Rust's operation order over the correctly rounded primitives (0 ulp against `*.cr.tsv`), which proves the transcription. M2 part b replaces the five weak spots in §3.4 with the T1 algorithms. **erf and erfc are correctly rounded**: a port of CORE-MATH's binary64 erf and erfc without FMA (§3.4). The other replacements are each gated against reference rows (`*.ref.tsv`) at the bounds **fixed on 2026-11-15** (§3.4): `normal_cdf` ≤ 1 ulp, `ln_gamma` ≤ 2, `gamma_inc` ≤ 4, `beta_inc` ≤ 8, each over a stated domain. ADR 0009 accepts all of them as "more accurate than Rust". Downstream rows are remapped once, before M3 uses them. **Fallback (decided 2026-11-15):** if the erf/erfc port does not verify within its time slot, ship erf/erfc at ≤ 1 ulp (option B's algorithm) and keep the port as a 2.x item. **Not needed:** the port verified inside its slot (§5 step 9e, 2026-11-15). | A, T0 only (every distribution and hypothesis row would be remapped twice for later fixes). B, the same without correctly rounded erf/erfc (≤ 1 ulp). D, T1 without part a (a T1 defect could not be told apart from a transcription defect) |
| D4 | ✅ **Numerical upgrades outside the special functions:** compensated summation in `sum` / `mean`, and polynomial regression on a centered, scaled Vandermonde matrix with Householder QR | Not in 2.0: in those chains Rust's operation order is the definition (as pravash). Measure them against NIST StRD in M4/M5 (class R, §3.6) and file the failures as 2.1 candidates, with numbers | Change them in 2.0: that alters every descriptive and regression row, a scope kimiya's mid-port overflow audit showed is costly (1.7 days, about 30 ADR rows) |
| D5 | ✅ **JSON.** Rust serializes 38 types and validates 20 on deserialize, with 32 round-trip tests; CLAUDE.md requires serde on every type. tanmatra dropped serde and kimiya marked it n/a. pravash wrote hand codecs over bayan's value tree (1,947 lines). The `#derive` has **no validation hook**, no `Vec<Vec<f64>>`, `u64` or data-variant support, and four open defects | Carry JSON for all 38 types with hand codecs (pravash's `src/json.cyr` as template). Every document serde_json writes from 1.3.0 decodes to the same bits, and every Cyrius document round-trips. Every decoder goes through the validating constructor, as Rust's `try_from` does. **Amended 2026-11-15, after P7 (§5 step 9c):** pramana writes serde_json's text byte for byte, with the serde form of impetus's `fmtf.cyr`, vendored once with D12's Debug form on the same digit core, so JSON text compares class A (P7: 596 of 596 documents). A stand-in pre-pass covers bayan's integer and string gaps (ledger L11); with it, 1,460 of 1,495 serde_json documents get serde_json's verdict, and every value read has its bits. The other 35, which serde_json accepts only because it skips a member unchecked, are ADR 007 row 16 | Text identity not required, with bayan's Grisu2 digits (D5 as first decided): P7 found the text different from serde_json's on 30 of 596 documents, and on 9,714 of 280,562 doubles. `#derive` (it cannot validate). Or dropping JSON (it breaks a CLAUDE.md principle) |
| D6 | ✅ **Errors.** Rust's five `PramanaError` variants carry message strings | `PMN_ERR_*` codes, append-only, with no detail strings (as every sibling), plus `PMN_ERR_OUT_OF_MEMORY` and `PMN_ERR_DESERIALIZE`. `pmn_err_name(code)` gives the text. A 1.3.0 error document decodes to its code, and the message is dropped (ADR 0007) | A thread-local "last error detail" string: no sibling precedent |
| D7 | ✅ **Traits and callbacks.** `Distribution` has 13 impls, with `sample(&self, rng: &mut impl Rng)`. `Rng` is implemented by users. Closures are taken by `monte_carlo_integrate`, `metropolis_hastings`, `gibbs_sampling` (`Fn(&[f64], &mut dyn Rng)`) and `ks_one_sample` | A checked `trait PmnRng { fn next_f64(self): f64 }`, with `PmnSimpleRng` as its impl. **Amended 2026-11-15, after P2:** every API that takes Rust's `&mut impl Rng` is generic over `R: PmnRng` and takes `rng: *R`, so a draw is a static call, as Rust's monomorphized one is. That includes the trait method `fn sample<R: PmnRng>(self, rng: *R): f64`, which Cyrius 6.7.18 accepts. `dyn PmnRng` is kept only where Rust has `&mut dyn Rng`, for Gibbs' conditionals, which get the address of a stack `PmnRngRef` (P3). `Distribution` as a checked trait on each struct, statically dispatched. Callbacks as a function pointer plus a context pointer (kimiya): nothing allocates, and any arity works through `callptr`. **Measured (P2, P3; §5 step 9b):** the callback form stands; a dyn costs about 3 ns on a 12 ns draw | `dyn PmnRng` at every API boundary (D7 as first decided): about 3 ns more per draw (P2), and the gamma, Poisson and beta samplers draw several times per sample. Capturing closures: they allocate on the heap and take at most 5 parameters (3 on Windows). A Gibbs conditional with an explicit context needs 4 (corrected by P3: one that captures its context needs 3) |
| D8 | ✅ **`ai`.** `StatQuery::query` is async (tokio) and uses the Rust hoosh 1.3 client with five tools (`describe`, `correlate`, `regress`, `t_test`, `forecast`). The Cyrius hoosh has no client library | Port it onto `sandhi_http_post` (synchronous) and bayan, against hoosh's OpenAI-compatible tool-calling API. It lives in `src/ai.cyr`, outside the core `[lib]`, or in a `[lib.ai]` profile (probe P8 decides), so core consumers never compile sandhi or TLS. Tests run on a recorded-response transport. File a client-library request in hoosh (L9). **Decided by P8 (2026-11-15; §5 step 9c):** a `[lib.ai]` profile, with sandhi kept out of `[deps] stdlib`. The `ai` test includes `lib/sandhi.cyr` itself. Consumers get both bundles with compile-verified sidecars, and pramana's own build stays at 0.49 s, not 1.30 s | Drop `ai`: there is no language reason, so the §0.1 rule forbids it |
| D9 | ✅ **`soorat-compat`.** Five data shapes plus `HistogramData::from_data`; soorat is still Rust | Port it into the core bundle, with JSON. Dead-code elimination removes it for consumers who don't call it, and a Rust soorat can read the JSON. No feature flag | Leave it out until soorat ports |
| D10 | ✅ **u128 results.** `factorial`, `permutations` and `combinations` return `Result<u128>` | An out-pointer `PmnU128 { lo, hi }` with pramana's own overflow-checked multiply and divide by u64 (about 60 lines; class A) | bayan's `u128` handles: they pull bayan's big-integer layer into a 4-function module |
| D11 | ✅ **Memory convention.** About 20 functions return a `Vec`, and nine types own `Vec`s | **Caller buffers** `(out, cap)` plus a count, wherever the length follows from the inputs: `residuals*`, `moving_average`, `exponential_smoothing`, `difference`, `integrate`, `arima_forecast`, `evaluate_grid`, `simulate`, `viterbi`, `steady_state` and the MVN sample. **Owned structs** (`KernelDensity`, `MultivariateNormal`, `MarkovChain`, `HiddenMarkovModel`, the models, `PcaResult`, `McmcResult`) are freelist-allocated, with `_drop`. **Constructors** write into caller storage (`PmnNormal_new(out, mean, sd)`), which avoids the open small-struct-in-`Result` miscompile. **Scratch** (median and percentile copies, sort buffers, HMM tables) comes from a work-table pool (kimiya ADR 0007). Probe P4 confirms the heap stays flat | Returning `alloc`'d pointers: the bump heap never frees, so a `pdf` in a loop leaks |
| D12 | ✅ **`Debug`.** Rust derives it on every public type. pravash's maintainer chose to port it, but that was found only at M8, which cost a part | Port it, per milestone, by reusing pravash's `src/fmtf.cyr` (Rust's shortest float text) and its `debug(sb, alternate)` builders. With D5's amendment, `src/fmtf.cyr` is one vendored file: impetus's digit core with its serde_json form, plus pravash's Debug form | Don't carry it (JSON covers inspection) |

---

## 1. Porting models, references and the language

- **pravash 2.0.1** (`/home/macro/Repos/pravash`), the tooling template. Copy and adapt:
  - `scripts/golden/` (the generator crate, `validate.py`, the glibc header);
  - `tests/parity.tcyr`, `tests/parity_core.cyr` and `tests/ulp.cyr`;
  - `scripts/surface_parity.py`, which also checks derives, trait methods and the reverse direction;
  - `scripts/rust_test_ledger.py`, `scripts/bench_record.py`, `scripts/doc_examples.py`, `scripts/upstream_ledger.py`;
  - `src/json.cyr` (hand codecs) and `src/fmtf.cyr` / `src/debug.cyr` (D12);
  - the CLAUDE.md translation of the Rust principles;
  - the `oracle` feature pattern on the Rust side (impetus's `src/oracle.rs`).
- **kimiya 2.0.1**, the accuracy model. Use:
  - ADRs 0005 and 0006, the correctly rounded primitives in `src/mathx.cyr` (with expm1), and the table and
    verification tooling (`mathx_tables.py`, `cr_verify.py`, `mpfr_ref.py`);
  - ADR 0003's "transcribe the missing hisab routine", and its existing `_km_least_squares_poly` (pramana calls the
    same hisab 1.4.0 routine);
  - ADR 0007, the work-table pool;
  - the batch loop of `handoff/`: an agent implements while the main session writes an independent Python model of
    the Rust formulas, and the two are compared bit for bit. "All but one [difference] were model mistakes."
- **tanmatra 2.0.2:**
  - the parity classes and the port-parity rule K_F = 2·S·calls + 2, where S is measured by nudging each primitive by ±1 ulp;
  - the Rust-test ledger;
  - `reference-validation.md`: each bound is written down before the first comparison, and the reference's own
    uncertainty must be at most a tenth of the bound. This is the model for §3.6.
- **Language and toolchain** (`/home/macro/Repos/cyrius`): `docs/guides/cyrius-guide.md`, `docs/stdlib-reference.md`,
  `docs/guides/faq.md`, `docs/development/issues/`, and `cyrius help manifest`.
- **ganita 1.2.15** (`/home/macro/Repos/ganita`): its roadmap, ADRs 0001 and 0004, and its open issues (§1.3).
- **ushma 2.0.0 and impetus 2.0.0**, two more finished ports. They were not reviewed in depth. ushma has correctly
  rounded tanh and expm1, and a triple-double log1p kernel (D2); impetus has the original crmath and the `oracle` pattern.
- **CORE-MATH** (MIT): correctly rounded binary64 erf and erfc, the source for D3 = C. It has no lgamma.

### 1.1 What the sibling ports measured

| Crate | Rust → Cyrius lines | Duration (commit span ⚠) | Golden rows | Better than Rust | Bench: median Cyrius/Rust time (range) |
|---|---|---|---|---|---|
| tanmatra 2.0.2 | 11,419 Rust lines → 23,293 (3,647 of them generated data) | 2026-11-08 19:01 → 2.0.0 at 22:33; 2.0.2 the next day | 191,583, 0 failing, 1 accepted | **none**: its policy, before ADR 0006, was bit parity with glibc. It also validated against NIST ASD and CODATA (61,948 assertions) | 5.8× (0.82–43×) |
| kimiya 2.0.1 | 6,824 code lines → 17,955 | 2026-11-08 → 2.0.0 on 11-13 (4.6 days) | 342,485 ok, 46,634 accepted, **810 improved** (glibc misrounds) | correctly rounded and target-independent; ill-conditioned functions gain most (Nernst rows up to 4.3×10¹⁸ ulp off on aarch64 are now 0) | 7.7× (1.5–69×) |
| pravash 2.0.1 | 6,882 code lines → ~20.8k | 2026-11-11 18:26 → 2.0.0 on 11-13 10:18 (~40 h, 42 commits) | 67,276 ok, **131 improved**, 597 accepted | correctly rounded; a 1-ulp exp error grew to 7 ulp through Lamb–Oseen's 1 − e^(…) | 4.38× (0.76–36.7×) |

⚠ Durations are commit timestamps only; work done before the first commit isn't visible.

pramana is the smallest of the four, at 4,454 code lines, and the most numerically delicate. Expect pravash-scale
effort plus M2 part b (D3).

Lessons that shape this plan:
1. Fix the accuracy target and build its MPFR tooling in M0. kimiya threw away three generations of primitives
   before settling (§3).
2. Probe the linear algebra before writing code, and budget transcription as the default (kimiya ADR 0003, §8.2).
3. Plan JSON and `Debug` from the start (pravash found `Debug`, 6 missing clones and 3 wrongly public types at M8).
4. Prefix every name: there is one global namespace, and a duplicate definition is a warning, with the last one
   winning (§8.4).
5. Design the work-table pool, null-buffer checks and aliasing checks in M1. kimiya's 2.0.1 review found all three
   missing.
6. Remove the Rust CI job before the 2.0.0 tag. pravash's `release.yml` has `needs: [ci]`, so a Rust job would gate
   the Cyrius release.
7. Pin the recording toolchain. pravash's generator `--check` failed on rustc 1.100, because rustc's `Debug` escape
   table moved to Unicode 18 (916 ranges for 1.99.1's 910). No float row changed, but every run was then pinned to
   `cargo +1.99.1`. pramana records on 1.100.0 (its MSRV), pinned exactly in `rust-old/rust-toolchain.toml`, not `stable`.

### 1.2 Language review: Cyrius 6.7.18 against pramana's Rust surface

Read from the 6.7.18 guide, the stdlib reference, the `lib/` sources and the open issues on 2026-11-15. Every verdict is
"port".

| Rust in pramana | Cyrius 6.7.18 | Verdict |
|---|---|---|
| f64 arithmetic | An f64 is its bit pattern in an i64 word. Operators are typed by the **left** operand: `0 - x` and `2 * x` are integer operations. Results of builtins are untyped, so bind them to a `: f64` local and compare with `f64_lt` and its siblings. Literals are correctly rounded but have **no exponent form**, and a hex literal in a `const` is an integer (guide *Types*, *Number Literals*) | port with `mathx` helpers, constants generated (M0 step 6) |
| std f64 methods | §8.1. There is no public `is_nan`, `is_finite`, `copysign`, `EPSILON`, `INFINITY`, `NAN` or `MIN_POSITIVE`, and the `F64_*` constants are untyped `var`s. `f64_min` / `f64_max` ignore a NaN only as the first operand (open cyrius issue 2026-11-11) | Rust-semantics helpers in `src/mathx.cyr` (as pravash's L2) |
| f64 bit manipulation (fdlibm erf high/low words) | free: an f64 already is its bits | port as written |
| u64 xorshift (`<<`, `>>`, `^`) | `>>` is a **logical** shift and `>>>` the arithmetic one (guide:1080); `<<` wraps. There is no unsigned compare (tanmatra) | port; probe P1 checks 10⁶ draws |
| u128 (combinatorics) | not native; bayan has `u128` handles | D10 |
| structs, `impl`, methods | packed structs with typed fields, `impl` blocks | port |
| `Result<T, PramanaError>`, `?` | value-form `Result` with a **one-word payload** and `?`. A ≤ 8-byte struct payload is miscompiled (open issue 2026-11-08) | codes plus out-pointers (D6, D11) |
| `Kernel` (unit enum), `#[non_exhaustive]` | plain enums, append-only by convention; unit variants serialize by name | port |
| `Distribution` (13 impls), `Rng` (user impls) | checked `trait` / `impl X for T` with static dispatch; `dyn Tr` is a 16-byte `{data, vtable}`; generics are monomorphized with checked bounds (ADR-007). A trait may declare a generic method (`fn sample<R: PmnRng>(self, rng: *R): f64`), and stays usable as `dyn` for its other methods (checked 2026-11-15) | D7, probe P2 |
| `impl Fn(f64) -> f64`, `Fn(&[f64]) -> f64`, `Fn(&[f64], &mut dyn Rng) -> f64` | capturing closures (heap, ≤ 5 parameters, 3 on Windows), or `&f` with `callptr` / `fncallN` and a context word. P3: a closure's captured f64 is an untyped word in arithmetic (`k * x` multiplies the bits), and `cyrius lint` refuses a capturing closure unless the file includes `lib/alloc.cyr` | function pointer plus context (D7), probe P3 |
| `Vec<f64>`, `Vec<Vec<f64>>` | `vec` holds 8-byte words, **aborts** on an out-of-bounds `vec_get` and never frees an outgrown buffer; ganita matrices are flat row-major | flat arrays plus counts; caller buffers (D11) |
| `sort_unstable_by`, `select_nth_unstable_by`, `sort_by(total_cmp)` (median, percentile, mode, KS, PCA) | `vec_sort_by` (introsort, unstable) and `vec_select_nth` (aborts if k is out of range) work on a `Vec` only; there is no stable sort | an f64-array introsort and quickselect stand-in (L3); probe P6 settles ±0 and tie order |
| `HashMap<String, Vec<f64>>` / `<String, f64>` (`ai` only) | `map_new_str`. Values are words, so f64 bits fit. The seed is random per process, and Rust's `RandomState` order is random too | port; iterate in sorted key order for determinism (ADR 0007) |
| serde on 38 types, `try_from` validation on 20 | `#derive(Serialize/Deserialize)` through bayan: no validation hook, no `Vec<Vec<f64>>`, `u64` or data variants; four open defects. bayan writes Grisu2 digits and parses correctly rounded; non-finite values are written as `null`. P7: the value tree refuses an integer at or above 2^63, reads `-0` as +0, and accepts a raw control byte in a string, all where serde_json differs; a text pre-pass covers the three. Grisu2 differs from serde_json's text on 9,714 of 280,562 doubles, impetus's `fmtf.cyr` on none | hand codecs (D5), probe P7 |
| `thiserror` messages | integer codes, in the stdlib style | D6 |
| `async fn query` (tokio) plus the Rust hoosh client | `async`/`await` over epoll exists; `sandhi_http_post` is synchronous; hoosh 2.x is a gateway with no client library | synchronous HTTP (D8) |
| `#[must_use]`, `#[inline]` | `#must_use`, `#pure`; `#inline` declines helpers with more than 2 parameters or a pointer parameter | translate |
| cargo features `ai`, `soorat-compat`, `logging` | `[features]` switch optional dependencies only; `[lib.X]` profiles; `[build] dce = true`. P8: `[deps] stdlib` is prepended to every compile, so listing sandhi there costs each build and test file about 0.8 s; a profile's sidecar is verified from its bundle alone. Dead code is removed on x86_64 ELF, but only NOPed on PE and aarch64 | one bundle plus dead-code elimination; `ai` per D8 |
| criterion (34 benches), `#[test]` (319) | `cyrius bench` (`.bcyr`), `cyrius test` (`.tcyr`), `cyrius fuzz` (`.fcyr`). Asserts are integer-only, so use pravash's `tests/ulp.cyr` | twins for all |
| `pub use hisab` | — | not carried: a Rust re-export |

### 1.3 ganita, reviewed in its own repo (2026-11-15)

`/home/macro/Repos/ganita` is the code the 6.7.18 stdlib vendors:
- **Release:** 1.2.15, pinned to cyrius 6.7.5. Its last commit is 2026-10-08, so nothing is unreleased.
- **Contents:** `src/linalg.cyr` (2,839 lines), `src/math_advanced.cyr` (1,419), `src/matrix.cyr`, `src/math_f32.cyr`.

What it adds to §1.2:

| Finding | Where | Effect on this plan |
|---|---|---|
| No erf, erfc, gamma, lgamma, beta or incomplete functions. They are in neither ganita's scope nor its "Out of scope" list | `src/math_advanced.cyr`, `docs/development/roadmap.md` | `src/special.cyr` is pramana's own. Asking ganita to take it over is a scope request, not a gap report (L8) |
| The correctly-rounded-tier issue is open with **four consumers**: impetus, kimiya, pravash, ushma. Its speed table measures the impetus-derived crmath: exp 110 ns, pow 757 ns, atan2 1.41 µs | `docs/development/issues/2026-11-09-correctly-rounded-exp-sin-cos-hypot.md` | pramana joins as a consumer, asking for log1p, tan and log2 too (L1). It vendors kimiya's faster two-phase versions (D2). **Joined at step 10 as the sixth, after soorat** |
| **No fused multiply-add** anywhere in ganita (nor a scalar one in the stdlib) | `grep fma src/` | the CORE-MATH erf/erfc port must be FMA-free (§3.4). Done in 9e with an exact emulation, at 6.5–20× CORE-MATH's cost in C; a scalar fma is requested (§10, L12) |
| `ganita_mat_cholesky_solve` allocates n words per call. **No issue is filed for it**, though its siblings `mat_lu_solve` and `mat_inv` have open issues | `docs/development/issues/2026-11-11-mat-{lu-solve,inv}-allocates-per-call.md` | pramana files the third issue of that family (L4) |
| `ganita_mat_eigen_sym` stops at an absolute 1e-12, an open issue | `2026-10-31-eigen-sym-absolute-stop.md` | PCA transcribes hisab 1.4.0's Jacobi (L6) |
| Householder QR / least squares break near DBL_MAX (open) | `2026-10-31-qr-householder-overflow-near-dbl-max.md` | not used: polynomial regression transcribes 1.4.0's MGS (L5) |
| `pow(−1, huge)` returns +∞ (open) | `2026-11-08-pow-minus-one-huge-even-exponent.md` | not used: pramana's pow is kimiya's correctly rounded one |
| **55 legacy aliases** (`f64_pow`, `mat_*`, …) are exported into the stdlib by `_compat.cyr`, and ganita's M4 **deletes** them | roadmap G-2, M4 | pramana calls only `ganita_*` names (in practice just `ganita_f64_cbrt` and possibly the Cholesky pair, per P5) |
| Private helpers (`_f64_is_nan_pat`, `_f64_is_inf`, `_gn_log1p`, `_gn_expm1`, …) sit in the global namespace | `src/math_advanced.cyr:55, 61, 108, 203` | never redefine them. The `_pmn_` prefix (§8.4) keeps clear |
| Failure vocabulary: a pointer or 0, 1 or 0, and negative codes. Functions that judge a matrix refuse NaN/∞ before writing anything | ADRs 0001 and 0004 | map each failure to its `PMN_ERR_*` code, keeping Rust's order of checks (pramana validates first anyway) |

---

## 2. What is different about pramana

1. **The special functions are the product.** `math.rs` (428 code lines) feeds the pdf/cdf of eight distributions,
   every p-value, and both quantile functions. §3.2 measured it at between 0.8 ulp (erf) and 4.3×10¹³ ulp (`ln_gamma`
   beside its roots). So M2 is the critical milestone, and it is the only place this port goes beyond "Rust's formula
   on correctly rounded primitives" (D3).
2. **Iterations with tolerances are everywhere.** These include:
   - the series and continued fractions (stop at `CONVERGENCE_EPS` = 1e-15, a budget of 200 + 20·√shape, Lentz's 1e-300);
   - the Wilson–Hilferty switch at a > 1e9;
   - `steady_state` (power iteration, switching to the lazy chain after 8 stalls);
   - Baum–Welch, logistic regression's Newton steps, the Jacobi eigen solver, `t_quantile`'s bisection and `z_quantile`'s Halley step.

   A 1-ulp primitive difference can move a stopping decision, so each one is a branch point (§6), and the Python
   transcription records term counts.
3. **Random streams.** `SimpleRng` (xorshift64) is integer code, so it is class A. The samplers turn uniforms into
   variates through ln, sqrt, cos, tan, pow and exp: Box–Muller, Marsaglia–Tsang, PTRS for λ > 30, Knuth's Beta
   splitting for n > 64, and inverse CDFs. Their accept/reject tests are branch points. A correctly rounded primitive
   flips one only when the two sides are within an ulp, and the corrected oracle replays the same decisions with
   MPFR's primitives. So streams are class C per draw and class A on draw counts.
4. **Caller-supplied functions** in Monte Carlo integration, Metropolis–Hastings, Gibbs and the one-sample KS test (D7).
5. **JSON on 38 types with validated decoding.** This is more JSON than any sibling except pravash (D5).
6. **Real network I/O in `ai`.** pravash's `ai` held configs only (D8).
7. **No stateful solver with a step loop.** The stateful objects are `MarkovChain`, `SimpleRng` and the fitted models;
   there are no particle or grid states. That makes the parity harness simpler than pravash's: no state loaders or step rows.
8. **Linear algebra through four hisab 1.4.0 calls,** whose Cyrius counterparts differ (§8.2).

---

## 3. Accuracy: what Cyrius can do better than Rust

### 3.1 Method

`scripts/port-baseline/run.sh` reproduces everything in this section. It takes about 12 minutes on one core.
- **Inputs:** seeded inputs over each function's domain.
- **Rust side:** pramana 1.3.0's `src/math.rs` (with the test-only `erf` enabled) and Rust std's f64 methods, built
  with rustc 1.100.0 against glibc 2.44.
- **Reference side:** MPFR 4.2.2 at 1,024 bits, except for the integer-shape incomplete beta, which uses exact
  rationals (`I_x(a,b) = Σ_{j=a}^{a+b−1} C(a+b−1, j) xʲ(1−x)^{a+b−1−j}` with a dyadic x, so 1 − x is exact).
- **Reported per function:**
  - the error in ulps of the correctly rounded value, max and mean;
  - "not CR", the share of results that differ from the correctly rounded value.
- **Excluded:** zero and subnormal results.

This is the seed of M0's reference tooling.

### 3.2 Baseline: Rust 1.3.0 against MPFR (measured 2026-11-15)

| Function | Domain | n | Max ulp | Mean ulp | Not CR | Worst input |
|---|---|---|---|---|---|---|
| `exp` (glibc) | [−700, 700] | 100,000 | 0.505 | 0.251 | 0.088% | 239.014… |
| `ln` (glibc) | [1e-300, 1e300] | 100,000 | 0.498 | 0.003 | 0% | — |
| `ln_1p` (glibc) | [−0.999, 1e3] | 100,000 | 0.748 | 0.212 | 0.70% | 0.2833… |
| `exp_m1` (glibc) | [−40, 700] | 100,000 | 0.738 | 0.214 | 1.19% | 0.9204… |
| `sin` / `cos` (glibc) | [−10, 10] | 100,000 each | 0.515 / 0.511 | 0.25 | 0.15% / 0.13% | — |
| `tan` (glibc) | [−1.5, 1.5] | 100,000 | 0.533 | 0.249 | 0.235% | 1.4939… |
| `powf` (glibc) | x ∈ [1e-3, 1e3], y ∈ [−50, 50] | 100,000 | 0.505 | 0.25 | 0.075% | — |
| `erf` (fdlibm port) | [−6, 6] and ±[1e-300, 1] | 100,000 | 0.822 | 0.254 | 3.1% | −4.32e-5 |
| `erfc` (fdlibm port) | [−6, 27] | 98,623 | **2.49** | 0.412 | 30.6% | 18.01 |
| `normal_cdf` | z ∈ [−5, 5] | 50,000 | **29.1** | 2.2 | 42.4% | −4.915 |
| `normal_cdf` | z ∈ [−38, 8] | 49,475 | **1,590** | 175 | 80.5% | −36.03 |
| `ln_gamma` (Lanczos g=7, n=9) | [0.5, 10] | 100,000 | **1.2×10⁶** | 65.2 | 93.2% | 2.0000060 |
| `ln_gamma` | \|x − 1\| or \|x − 2\| ∈ [1e-12, 0.1] | 25,000 | **4.3×10¹³** | 3.8×10¹¹ | 99.98% | 1.99999999999896 |
| `ln_gamma` | (−20, 0), non-integer | 25,000 | **6.9×10⁴** | 17.6 | 84.9% | −4.039 |
| `ln_gamma` | [10, 1e15] | 25,000 | 4.85 | 0.578 | 47.2% | 75.21 |
| `ln_gamma` | (1e-300, 0.5) | 25,000 | 8.21 | 0.028 | 1.7% | 0.4956 |
| `gamma_inc` P(a, x) | a ∈ [0.01, 1000], x near a | 4,000 | **58.1** | 4.87 | 79.8% | a = 602, x = 556.7 |
| `gamma_inc` Q(a, x) | same | 4,000 | **246** | 12.1 | 76.2% | a = 0.0103, x = 0.0194 |
| `beta_inc` I_x(a, b) | integer a, b ∈ [1, 60] | 4,000 | **550** | 14.1 | 57.1% | (36, 15, x = 2.8e-4) |
| `beta_inc` 1 − I_x | same | 4,000 | **583** | 12.8 | 56.7% | (22, 50, x = 0.99916) |

P9 (§5 step 9d-2) re-measured the incomplete gamma rows on all 20,000 pairs, against a 320-bit reference: Rust reaches
**97** ulp on P and **485** on Q (means 4.82 and 12.4). Outside these sets it is far worse: 2.9×10⁹ ulp on P in the
deep lower tail, and 2.8×10⁴ on Q for small shapes (`port-parity.md` §6.5).

### 3.3 What the numbers say

- **glibc's primitives are already good:** ≤ 0.75 ulp, misrounding 0.075–1.2% of results. Correct rounding fixes the
  last bit on those rows, plus cross-target identity. That is the whole gain the siblings reported: 131 rows in
  pravash, 810 in kimiya.
- **pramana's own functions are where the error is.** It comes from four sources:
  1. **Lanczos `ln_gamma` has an absolute error of about 1e-15**, so its *relative* error explodes where lnΓ → 0
     (x → 1, 2) and beside the negative-axis zeros. Inside the pdfs only the absolute error matters, and it still
     matters there. For example, the Gamma(5) pdf takes exp(−lnΓ(5)); a mean error of 65 ulp of lnΓ(5) = 3.18 is about
     2.9e-14 absolute, which is about 130 ulp of the pdf. (This is arithmetic from the measured mean, not a measurement.)
  2. **Argument rounding before an ill-conditioned function.** `normal_cdf(z) = ½·erfc(−z/√2)` rounds z/√2 first.
     erfc's condition number at x is about 2x², so at z = −37 (x ≈ 26) the 0.5-ulp argument error becomes up to about
     1,400 ulp. That matches the measured 1,590.
  3. **Large exponents in the prefactor.** The incomplete functions multiply by exp(a ln x − x − lnΓ(a)) or by
     xᵃ yᵇ / (a B(a, b)). An absolute error ε in an exponent of magnitude M gives a relative error of about M·ε in
     the result. This is the 550–583 ulp at deep beta tails; pramana's Loader/`bd0` prefix already removes it for
     large shapes.
  4. **Stopping and complements.** The series and continued fractions stop at a 1e-15 relative term (about 4.5 ulp),
     and small-a Q is formed as 1 − P (the 246-ulp row).
- **None of these is a language property.** Cyrius gains nothing on them automatically. It gains because the port is
  the natural point to replace these four algorithms, and the MPFR tooling to prove the replacements comes with the
  method anyway.

### 3.4 Expected accuracy, Cyrius over Rust

There are three tiers:
- **T0** is the 2.0 parity build: Rust's operation order on correctly rounded primitives.
- **T1** is M2 part b (D3 = C). It includes **correctly rounded erf and erfc**, from CORE-MATH.
- **T2** is not attempted:
  - correctly rounded lgamma has no implementation to port (CORE-MATH covers every C23 binary64 function except
    lgamma and compound);
  - correctly rounded incomplete gamma and beta are research-grade.

**The CORE-MATH erf/erfc port.**
- **Source:** CORE-MATH's binary64 erf and erfc (MIT license, compatible with GPL-3.0-only; keep the notice).
- **No FMA:** the code assumes fused multiply-add, which Cyrius lacks (no scalar FMA in the stdlib or ganita, §1.3).
  - Where an fma only builds a double-double product, an exact Dekker product (Veltkamp split) replaces it.
  - Where its single rounding matters, use an exact software fma. P9 used Dekker's product and Boldo and Melquiond's
    round-to-odd sum (kimiya's `_km_rn3`) in place of musl's integer `fma.c`.
  - Probe P9 records which call sites are which: 2 Dekker products, 57 exact fmas, and one fma on erf's tiny path
    whose product can be subnormal, rounded once on the subnormal grid instead (`port-parity.md` §6.7).
- **Verification as kimiya's primitives:**
  - CORE-MATH's own worst cases;
  - ≥ 1M seeded MPFR arguments per function, plus about 100k forced through its accurate path;
  - 0 mismatches on all three targets.
- **Time slot and fallback:** the port has a fixed slot inside P9. If it does not verify in time, erf and erfc ship at
  **≤ 1 ulp** (decided 2026-11-15): erfc's tail exp(−x²−0.5625)·exp(…) becomes one correctly rounded exp of a
  double-double argument. The CORE-MATH port then moves to 2.x, and ADR 0009 records the bound actually shipped.
- **Done in P9 (§5 step 9e, 2026-11-15):** the port gives CORE-MATH's own bits on every worst case and on every
  seeded argument, and the same bits on three targets. MPFR finds all 2,768,518 non-NaN rows correctly rounded, so
  the fallback is not needed.

**The T1 bounds are fixed** (the maintainer, 2026-11-15). They are commitments, not outcomes of P9. A row that misses
its bound is a defect in the new algorithm, and the algorithm is reworked. A bound is never loosened. Only an ADR 0009
amendment by the maintainer can narrow a domain.

| Function | Bound | Committed domain | Outside the domain |
|---|---|---|---|
| erf, erfc | correctly rounded (0.5 ulp); ≤ 1 ulp under the fallback | every finite x | — |
| `normal_cdf`, `normal_sf` | ≤ 1 ulp | every finite z | — |
| `ln_gamma` | ≤ 2 ulp | finite x > 0, up to where lnΓ overflows (≈ 2.55×10³⁰⁵), including beside the roots at 1 and 2 | x < 0. It occurs only inside the reflection formula: every caller in pramana passes a positive argument. The bound is ≤ 2 ulp of max(\|lnΓ(x)\|, 1), a mixed bound, because without extra precision the relative error is unbounded at the negative-axis zeros |
| `gamma_inc`, P and Q | ≤ 4 ulp | a ∈ [1e-3, 1e5], x > 0 | a ∈ (1e5, 1e9]: no row less accurate than Rust 1.3.0 on the same input. a > 1e9: Rust's Wilson–Hilferty approximation is kept as it is. It is not ulp-accurate, and replacing it (Temme's uniform expansion) is a 2.x item |
| `beta_inc`, I and 1 − I | ≤ 8 ulp | a, b ∈ [1e-3, 1e5], x ∈ [0, 1] | larger shapes: no row less accurate than Rust 1.3.0 on the same input |

Rules for every row:
- **Errors are in ulps of the correctly rounded value.** Zero and subnormal results are measured in units of the
  subnormal spacing, 2⁻¹⁰⁷⁴, against the same bound.
- **Verification covers:**
  - M2's grids across every branch point;
  - §3.2's sets;
  - seeded MPFR or exact-rational arguments: ≥ 1M each for erf, erfc, `normal_cdf` and `ln_gamma`, and ≥ 50k each
    for `gamma_inc` and `beta_inc`, since MPFR's `gamma_inc` is slow at 1,024 bits. These are weighted toward the
    hard regions: x ≈ a, deep tails, small shapes, and the series/continued-fraction switch.
- **Targets:** every row is checked on all three.
- **"No less accurate than Rust"** is checked row by row, against Rust 1.3.0's own error on the same reference value.

| Function | Rust 1.3.0 (measured) | Cyrius T0 (parity) | Cyrius T1 (fixed bound; means are expectations) | How T1 gets there |
|---|---|---|---|---|
| exp, ln, pow, sin, cos, atan | ≤ 0.51 ulp; 0–0.15% misrounded; host-glibc dependent | **0.5 ulp, 0% misrounded, identical on 3 targets** | same | kimiya's two-phase primitives (D2) |
| ln_1p, exp_m1, tan, log2 | ≤ 0.75 ulp; up to 1.19% misrounded | **0.5 ulp, 0%** | same | kimiya's expm1; new log1p, tan, log2 (D2) |
| erf | 0.82 max, 3.1% not CR | ≈ Rust: \|x\| < 0.84375 is a pure polynomial (same bits); above, only rows where glibc's exp misrounds change | **0.5 ulp, correctly rounded** (D3 = C) | the FMA-free CORE-MATH port above. **P9 port (§5 step 9e): correctly rounded on all 1,281,250 rows, CORE-MATH's worst cases included** |
| erfc | **2.49** max, 30.6% not CR | ≈ Rust | **0.5 ulp, correctly rounded** (D3 = C) | as erf. **P9 port: correctly rounded on all 1,487,268 rows** |
| normal_cdf, normal_sf | 29 ulp in [−5, 5]; **1,590** in the tails | ≈ Rust | **≤ 1 ulp** | carry z/√2 as a double-double, then take the correctly rounded erfc of the high part plus a first-order correction for the low part: erfc(h + l) ≈ erfc(h) − l·(2/√π)e^(−h²). The ×½ is exact. **P9 (§5 step 9d-4) refined this:** the correction goes onto CORE-MATH's double-double erfc(h) before it rounds, and the ×½ into that one rounding (it is not exact for subnormal results). The sketch's two roundings reach exactly 1.0 ulp. **P9 prototype: ≤ 0.5 ulp, correctly rounded on all but 4 of 1,246,809 rows (proven ≤ 0.508); 8–20× T0's cost** |
| ln_gamma, x > 0 | 4.85–8.2 ulp at large and small x; 65 mean / 1.2×10⁶ max on [0.5, 10]; **4.3×10¹³** beside 1 and 2 | ≈ Rust (every term is Rust's) | **≤ 2 ulp** on x > 0, including beside 1 and 2 | rational/series forms of lnΓ(1 + ε) and lnΓ(2 + ε) around the roots (fdlibm's `e_lgamma_r` layout), with double-double logarithms and products and one rounding. **P9 prototype (§5 step 9d-1): ≤ 1.31 ulp, mean 0.26, on 450,000 rows** (Rust: 4.3×10¹³ beside the roots) |
| ln_gamma, x < 0 | 6.9×10⁴ max | ≈ Rust | ≤ 2 ulp of max(\|lnΓ\|, 1) (mixed; see the bounds above) | reflection with sin(πx) reduced exactly (x − round(x) first), subtracted in double-double. **P9 prototype: ≤ 1.40 ulp of max(\|lnΓ\|, 1)** on 123,655 rows (Rust: 1,583) |
| gamma_inc P, Q | 4.9 / 12.1 mean; **58 / 246** max (97 / 485 on all of §3.2's pairs; 2.9×10⁹ in the deep lower tail) | ≈ Rust | **≤ 4 ulp** max, ~1 mean | a double-double prefactor exponent and exp of it; the series and continued fraction in double-double to 2⁻⁶²; small-a Q directly, not 1 − P. **P9 prototype (§5 step 9d-2): ≤ 1.0 ulp, mean ≤ 0.25, on 360,000 rows; 10–17× T0's cost.** Plain-double loops miss (to 79 ulp) |
| beta_inc I, 1 − I | 13–14 mean; **550 / 583** max | ≈ Rust | **≤ 8 ulp** max, ~1–2 mean | TOMS 708-style prefactor in double-double; a tightened continued fraction. **P9 prototype (§5 step 9d-3): ≤ 0.51 ulp, mean ≤ 0.25, on 214,744 rows** (a double-double prefactor and continued fraction, a direct power-series complement for small shapes); 10–17× T0's cost |
| Gamma, Beta, χ², t, F pdfs | inherit lnΓ's absolute error (~1e-14 relative, from §3.3) | ≈ Rust | ~1–3 ulp | from the lnΓ fix |
| Student-t, F, χ², Binomial, Poisson CDFs; every p-value | inherit `beta_inc` / `gamma_inc` (tens to hundreds of ulp in tails) | ≈ Rust | ≤ ~8 ulp | from the incomplete-function fixes |
| `z_quantile`, `t_quantile` | not measured (Acklam + Halley; bisection on the t CDF) | ≈ Rust | bounded by the CDF's accuracy | measured at M4 against MPFR inversion |
| Samplers (every distribution) | each draw carries its formula's primitive errors | **0-ulp primitives**; streams equal Rust's except a rare last-bit change | same | — |
| sum, mean, polynomial regression | naive summation; MGS on the raw Vandermonde | = Rust (D4) | 2.1 candidates (D4) | Neumaier sums; a centered, scaled Householder fit |

Beyond the ulp counts, Cyrius gives two properties Rust does not:
- **One answer on every machine.** Rust 1.3.0's bits depend on the host's glibc version and architecture. The Cyrius
  results are identical on x86_64 ELF, x86_64 PE and aarch64.
- **Proven, not sampled, primitives.** kimiya verified its primitives on ≥ 1M MPFR arguments each, with 0 mismatches
  on three targets.

### 3.5 Golden rows for the accuracy work

| File | Expectation | Produced by | Compared at |
|---|---|---|---|
| `tests/golden/<m>.tsv` | Rust 1.3.0's bits | the generator over `rust-old/` | its class (§6) |
| `<m>.cr.tsv` | Rust's operation order on correctly rounded primitives, for the rows where glibc misrounds | `validate.py`'s Python transcription with MPFR primitives (kimiya's `mpfr_ref.py`) | 0 ulp (T0) |
| `<m>.ref.tsv` (new) | the true function value | MPFR (`erf`, `erfc`, `lgamma`, `gamma_inc`); exact rationals for integer-shape `beta_inc`; a 256-bit MPFR continued fraction for general `beta_inc`, t and F | the ADR 0009 bound per function (T1); **0 ulp** for erf and erfc (D3 = C) |
| downstream rows after T1 | Rust's formula with the special functions taken from `.ref` | `validate.py` | K by the port-parity rule, S measured |

### 3.6 External reference validation

Following tanmatra's rule, every bound is written down before the first comparison, and the reference's own
uncertainty must be at most a tenth of the bound.
- **NIST StRD:** certified values to 15 digits.
  - Univariate (mean, standard deviation, lag-1 autocorrelation): PiDigits, Lottery, Lew, Mavro, Michelso, NumAcc1–4.
  - One-way ANOVA: SiRstv, SmLs01–09, AtmWtAg.
  - Linear and polynomial regression: Norris, Pontius, NoInt1/2, Longley, Filip, Wampler1–5.

  These are fetched at M0 (public domain). Filip (degree 10) is expected to fail 1.3.0's raw-Vandermonde fit;
  that is a D4 finding with numbers, not a 2.0 blocker.
- **MPFR / exact** values for every special function (§3.5).
- **Statistical tests on samplers:** KS against the exact CDF at fixed seeds, and moment checks. These are class R
  rows, as in the 1.3.0 test suite.

### 3.7 What accuracy costs

| Correctly rounded primitive | Cost, kimiya | Cost, pravash vs glibc |
|---|---|---|
| exp | 39 ns, 3.5× x87 | 23× glibc |
| ln | 48 ns | 12× |
| pow | 133–138 ns, faster than ganita's | 62× |
| sin / cos | 73 / 80 ns, 2× fdlibm | 18× |
| exp, ln, sin, cos, pow when they leave the fast path | 0.4–4 µs; sin/cos at fl(k·π/2) 18 µs | — |

pramana's distributions call one to four primitives per pdf/cdf/draw, so expect §9's transcendental-bound benches to
sit at the upper end of the sibling ratios. The T1 double-double work is measured in P9; it is applied only inside
the five functions of §3.4. The FMA-free erf/erfc port pays for each exact product with a Veltkamp split
(about 17 flops in place of one fma), or for a software fma. P9 measured it (§5 step 9e): 508–675 ns a call on erf
and erfc's main ranges and 2.3 µs on erfc's asymptotic range, 18–31× glibc. In C the emulation alone takes
CORE-MATH from 6–38 ns to 82–426 ns, so a scalar fma (§10, L12) would remove most of it.

---

## 4. Scope

### 4.1 In scope for 2.0.0: everything

| Milestone | Rust module | Code lines | `pub fn` | Tests | Cyrius module(s) | Library use |
|---|---|---|---|---|---|---|
| M1 | `error.rs` | 15 | — | — | `src/error.cyr` | — |
| M1 | *(public and private constants)* | — | — | — | `src/constants.cyr` (generated) | — |
| M1 | *(std f64 methods)* | — | — | — | `src/mathx.cyr`, `src/crmath.cyr` plus `src/crmath_data.cyr` (D2) | ganita `cbrt` (exact) |
| M1 | `rng.rs` | 38 | 1 | 5 | `src/rng.cyr` | — |
| M1 | `combinatorics.rs` | 58 | 4 | 7 | `src/combinatorics.cyr` (with `PmnU128`, D10) | — |
| M1 | `bridge.rs` | 141 | 11 | 19 | `src/bridge.cyr` | — |
| M1 | *(JSON core)* | — | — | — | `src/json.cyr` (bayan value tree) | bayan |
| M2 | `math.rs` | 428 | 14 `pub(crate)` | 18 | `src/special.cyr` (a: T0, b: T1) | — |
| M3 | `distribution.rs` | 1,037 | 20 + 65 trait methods | 75 | `src/dist.cyr` (trait, validation), `src/dist_cont.cyr`, `src/dist_gamma.cyr`, `src/dist_disc.cyr`, `src/dist_mvn.cyr` | `src/linalg.cyr` (Cholesky) |
| M4 | `descriptive.rs` | 424 | 20 | 34 | `src/sort.cyr` (L3), `src/descriptive.cyr`, `src/kde.cyr`, `src/pca.cyr` | `src/linalg.cyr` (Jacobi) |
| M4 | `hypothesis.rs` | 537 | 9 | 40 | `src/hypothesis.cyr` | — |
| M5 | `regression.rs` | 336 | 9 | 23 | `src/regression.cyr` | `src/linalg.cyr` (Cholesky, MGS least squares) |
| M5 | `timeseries.rs` | 268 | 7 | 19 | `src/timeseries.cyr` | `src/linalg.cyr` (Cholesky) |
| M5 | `bayesian.rs` | 119 | 4 | 7 | `src/bayesian.cyr` | — |
| M6 | `markov.rs` | 454 | 11 | 26 | `src/markov.cyr` | — |
| M6 | `monte_carlo.rs` | 194 | 4 | 15 | `src/monte_carlo.cyr` | — |
| M7 | `integration/soorat.rs` | 70 | 1 | 9 | `src/soorat.cyr` (D9) | — |
| M7 | `ai.rs` | 311 | 1 | 4 | `src/ai.cyr`, outside `[lib]` or `[lib.ai]` (D8) | sandhi, bayan |
| M8 | `tests/integration.rs`, doc tests | — | — | 15 + 3 | the Rust-test ledger | — |

Notes:
- Each milestone also ports its own types' JSON codecs (D5) and `Debug` (D12).
- `src/linalg.cyr` holds the hisab 1.4.0 transcriptions (§8.2). It lands with its first user, M3's `MultivariateNormal`.
- The surface-parity check covers every `pub fn`, every trait method and every public field.

### 4.2 Not carried

| Rust surface | Why |
|---|---|
| the `logging` feature | a no-op since 1.3.0, so there is nothing to port. sakshi spans can come after 2.0 if a consumer asks |
| `pub use hisab` | a Rust re-export; Cyrius consumers declare their own dependencies |
| `thiserror` message text | codes (D6) |
| `Send`/`Sync`, `#[inline]` as Rust applies it, `impl Into<String>` | Rust-only mechanics |

### 4.3 API changes in the Cyrius version (seed for ADR 0007)

This is the seed list, finalized at M0 step 2. Each row says how the harness still compares the function.

| # | Rust 1.3.0 | Cyrius 2.0.0 | Golden comparison |
|---|---|---|---|
| 1 | `Result<T, PramanaError(String)>` | value-form `Result` plus a `PMN_ERR_*` code (D6) | class A on the variant |
| 2 | constructors return `Result<Self>` | `PmnX_new(out, …)` fills caller storage and returns a code (D11) | A on the code, then each field by its class |
| 3 | `Vec` returns | caller buffer plus count, or a freelist-owned result with `_drop` (D11) | per element |
| 4 | `u128` | `PmnU128 { lo, hi }` out-pointer (D10) | A |
| 5 | `impl Fn` closures | function pointer plus context (D7) | same values |
| 6 | `&mut impl Rng`; `&mut dyn Rng` | `rng: *R` generic over `R: PmnRng`; Gibbs' conditionals get a `PmnRngRef` (D7, amended) | same streams |
| 7 | serde_json text | the same text, through `fmtf.cyr`'s serde_json form; every 1.3.0 document decodes to the same bits (D5, amended after P7) | A on the text and on the decoded bits |
| 8 | `PramanaError` JSON with a message | the code only; the message is dropped on decode | A on the variant |
| 9 | `async StatQuery::query`; HashMap column order | synchronous; columns in sorted name order (D8) | A on the tool results; the prompt text is n/a |
| 10 | cargo features | one bundle with dead-code elimination; `ai` per D8 | — |
| 11 | bits depend on the host glibc | bits identical on every target | measured on 3 targets |
| 12 | glibc's last-ulp misrounds | correctly rounded primitives (ADR 0009) | `*.cr.tsv` at 0 ulp |
| 13 | `math.rs`'s erf/erfc, `normal_cdf`, `ln_gamma`, `gamma_inc`, `beta_inc` | the T1 algorithms (D3, ADR 0009) | `*.ref.tsv` at the ADR's bound; downstream remapped by name |
| 14 | `median`, `percentile`, `quartiles` and `iqr` take the k-th element from `select_nth_unstable_by`, whose order decides which zero a mix of −0 and +0 yields (P6: 9,393 of 180,000 zero-heavy results) | the k-th element, and its successor, of a **stable** sort of the input (ties, −0 and +0 among them, kept in input order). Decided 2026-11-15 | A on the value. Where Rust's sign of zero differs, the row maps to the stable rule's result (`descriptive.map.tsv`, from `validate.py`'s stable model) and passes as accepted (ADR 7) |
| 15 | derived `Debug` | `x.debug(sb, alternate)` (D12) | A on the text |
| 16 | serde_json reads a document whose unread member holds a duplicate key, nesting past 128 levels or a lone surrogate, because it skips that member unchecked | refused (`DESERIALIZE`): bayan checks the whole document, until its opt-in for unread members lands (P7: 35 of 1,495). Decided 2026-11-15 | A on the verdict; those rows map to the refusal and pass as accepted (ADR 7) |

---

## 5. M0: the prep, then initialize with `cyrius port`

Shared mechanics are as pravash's §4; the pramana specifics follow.

0. ✅ **Freeze and hook the Rust crate. Done 2026-11-15** and committed as `94a7153`.
   1. ✅ **The P(-1) gates, re-run on rustc 1.100.0 with the hooks in.** All green:
      - fmt;
      - clippy `-D warnings` with all features, the defaults, and each of `soorat-compat`, `logging`, `ai` and `oracle` alone;
      - rustdoc `-D warnings`, `cargo audit`, `cargo deny`.

      Tests: defaults 306, `soorat-compat` 315, `logging` 306, `ai` 310, `oracle` 317, `--all-features` 330
      (1.3.0's 319 plus the 11 hook tests). Every count without `oracle` equals 1.3.0's.
   2. ✅ **The `oracle` feature** (D1): non-default, `#[doc(hidden)] pub mod oracle` (`src/oracle.rs`), read-only.
      - **Layout:** the hooks live in child modules (`src/{math,distribution,hypothesis,markov,descriptive}/oracle.rs`),
        which can read their parent's private items. So each audited source gains only a three-line `mod oracle;`
        declaration. `math.rs` also builds its test-only `erf` (and its `EFX8` constant) under `oracle`.
      - **What it exposes:**
        - the 14 `pub(crate)` functions of `math.rs`, with both tails;
        - `standard_normal`, `sample_poisson_ptrs` and `sample_gamma_standard` (the gamma sampler is the Beta-splitting
          building block);
        - `student_t_cdf`, `student_t_two_tailed`, `f_cdf_sf`;
        - `z_quantile`, `t_quantile` (bisection on the t CDF), `kolmogorov_sf`, `ks_pvalue`, `chi_squared_pvalue`, and
          the Welch statistics as a `(diff, se, df)` tuple;
        - the HMM's scaled forward pass `(alpha, scales)` and its log-likelihood;
        - `Kernel::evaluate`;
        - `MultivariateNormal`'s factorization `(cholesky_l, log_det)`. This is the **only** private state:
          `MarkovChain`, `HiddenMarkovModel` and `KernelDensity` already have public fields.
      - **Guards:** the two samplers' hooks return NaN, drawing nothing, for parameters the crate never passes
        (λ < 10 or non-finite; α ≤ 0 or non-finite), where the samplers would loop forever.
      - **Draw counts** come from a counting `Rng` wrapper in the generator (`Rng` is a public trait), so nothing is
        written inside a sampler.
      - **`tests/oracle.rs`, 11 tests:**
        - every hook gives, bit for bit, what the public API computes through the same private code: the normal,
          gamma, χ², beta, t and F CDFs; the Gamma pdf through `ln_gamma`; Welch's t, `ci_two_means`, `ci_proportion`,
          the KS and χ² p-values; the HMM log-likelihood; KDE with one point;
        - each sampler hook draws the same values and the same number of uniforms as the public sampler (10⁴ normal
          draws, PTRS at λ = 31…10⁸, gamma at α = 0.3…50);
        - the quantiles match pinned 1.3.0 bits;
        - a Baum–Welch fit and 2,000 MVN draws are identical with and without the hooks in use.
      - **Mutation-checked six ways**, each caught: an extra uniform in the gamma hook; one ulp in `z_quantile`,
        `t_quantile`, `ln_gamma` and `log_det`; the HMM hook skipping its input checks. The `z_quantile` and
        `t_quantile` mutations first got through, because a confidence interval's rounding absorbed them; that is
        why the quantiles are now pinned to bits.
   3. ✅ **The toolchain.** `rust-toolchain.toml` is pinned to `1.100.0` (it was `stable`); it moves to `rust-old/` at
      step 1. The CI coverage job now adds `llvm-tools-preview` to the pinned toolchain, and the MSRV comment no longer
      says `stable`. `Cargo.lock` is already tracked; it moves to `rust-old/Cargo.lock`. Every TSV header will record
      **glibc 2.44** and **rustc 1.100.0**, and `--check` will refuse a corpus from either another glibc or another rustc.
   4. ✅ **Committed as `94a7153` ("m0 0 done").** This commit is the freeze point: step 1's scaffold moves the crate into
      `rust-old/`, and from then on `src/` there never changes.
1. ✅ **Scaffold. Done 2026-11-15** on the freeze commit `94a7153`.
   - **Order:** `cargo clean` first, which removed 7.3 GiB of `target/` that would otherwise have moved, then
     `cyrius port --dry-run`, then `cyrius port`.
   - **The dry run under-reported, as the clobber issue says**
     (`cyrius/docs/development/issues/2026-11-08-cyrius-port-clobbers-project-docs-and-omits-bench.md`, still open on
     6.7.18). It named no overwritten file.
   - **What the scaffold did:**
     - **Moved to `rust-old/`:** `Cargo.toml`, `Cargo.lock`, `src/` (with the `oracle` modules), `tests/`, `benches/`,
       `examples/`, `Makefile`, `deny.toml`, `codecov.yml`, `rust-toolchain.toml`, `bench-history.csv`, `benchmarks.md`.
       It also wrote `rust-old/LINES_OF_SOURCE.txt`: 10,424 lines.
     - **Overwrote with templates:** CLAUDE.md, `docs/development/roadmap.md`, `.github/workflows/ci.yml` and `release.yml`.
     - **Created:** `src/main.cyr`, `cyrius.cyml`, `cyrius.lock` and `lib/` (26 stdlib files vendored), `.gitattributes`,
       `tests/pramana.{tcyr,bcyr,fcyr}`, `docs/development/state.md`, `docs/adr/`, `docs/architecture/README.md`,
       `docs/examples/`, `docs/guides/getting-started.md`.
     - **Appended to `.gitignore`:** `/rust-old/target/`, `/rust-old/**/__pycache__/` and `/build/`. Its message
       said `/rust-old/`, but `rust-old/` stays tracked, as wanted. `Cargo.lock` was never ignored, so
       `rust-old/Cargo.lock` stays tracked.
   - **The frozen crate still passes in `rust-old/`** on its pinned 1.100.0: 330 tests with `--all-features` (the 11
     oracle tests among them), and clippy `-D warnings` is clean.
   - **The skeleton:** `cyrius build` and `cyrius test` (2 assertions) pass. `cyrius bench` fails to compile, because
     the manifest has `[test] stdlib = ["assert"]`. The manifest also says `version = "0.1.0"`. Both are the issue's
     items 2 and 3, and step 2 repairs them.
   - Committed as `d8e761d` ("m0 1 done"), with the scaffold's overwritten files, which step 2 then repaired.
2. ✅ **Repair. Done 2026-11-15** and committed as `40a8269` (on top of `d8e761d`, the committed scaffold). The originals came from `94a7153`.
   - **CLAUDE.md:** merged. It keeps the identity, the process and the DO NOT list (including "NEVER use `gh`"), and
     translates the Rust principles against §1.2:
     - `#[non_exhaustive]` → append-only enums;
     - `#[must_use]` → `#must_use` / `#pure`;
     - "zero unwrap/panic" → no `+?`, `-?`, `*?`, `result_unwrap`, `assert_fatal`, unchecked `vec_get` or `vec_select_nth` in `src/`;
     - "every type Serialize + Deserialize" → every public type has JSON codecs with validated decode and a round-trip test (D5).

     It adds the §0.1 deferral rule, ADR 009's fixed bounds, the D11 memory rules and the `Pmn`/`PMN_`/`_pmn_` names.
     It replaces the MSRV with the Cyrius pin, and says to run the Rust oracle from inside `rust-old/` (rustup reads
     `rust-toolchain.toml` from the current directory).
   - **Roadmap:** restored from `94a7153`, keeping the 2.0.0 section; it gained a "Cyrius threat model" item.
     **`state.md`:** written for 1.3.0 → 2.0.0-dev.
   - **Workflows:**
     - The Rust ones are in `rust-old/.github/workflows/`, where `release.yml` can no longer publish.
     - The scaffold's Cyrius CI gained `test --bounds`, `deps --locked` and `audit`.
     - A `rust-oracle` job runs inside `rust-old/`, installs the pinned toolchain explicitly, and runs
       `cargo test --locked` with all features and with `oracle` alone. Its comment says to **remove it before the
       2.0.0 tag**.
   - **Manifest repairs** (the clobber issue's items 2–4):
     - `version = "${file:VERSION}"`;
     - `bench` in `[test] stdlib`;
     - a real description and the repository;
     - doc comments on the six scaffold stubs.
   - **Moved to `rust-old/`:**
     - `scripts/bench-history.sh` and `scripts/version-bump.sh` → `rust-old/scripts/`;
     - `docs/architecture/overview.md` → `rust-old/docs/architecture/`;
     - `docs/guides/testing.md` → `rust-old/docs/guides/`;
     - `docs/development/threat-model.md` and `docs/development/dependency-watch.md`, both Rust-specific →
       `rust-old/docs/development/`.

     The scaffold had already moved `bench-history.csv`, `benchmarks.md`, `deny.toml` and `codecov.yml`.
     `scripts/port-baseline/` stays. It now reads `rust-old/src/math.rs` and builds with the recording toolchain.
     Its `erf` filter also matches the new `oracle` cfg, which had broken its first re-run.
   - **ADRs,** continuing pramana's numbering in `docs/decisions/`, plus a `README.md` index and `template.md` in its
     format. The scaffold's `docs/adr/` is gone. The new records:
     - 006, the port of Rust 1.3.0 with the oracle, including how ADRs 001–005 carry;
     - 007, the API divergences (§4.3);
     - 008, local stand-ins and the upstream ledger;
     - 009, correct rounding and the fixed special-function bounds (D2, D3).

     005's status now says it is Rust-only.
   - **Other docs:**
     - CONTRIBUTING and SECURITY are rewritten for the Cyrius line; SECURITY's attack surface now covers JSON
       decoding, the heap and `ai`.
     - The README has a port banner, and its build section points at `rust-old/`. Its full rewrite is M8's.
   - **Gates, all green:** `cyrius build`, `test` (warning-free), `test --bounds`, `bench`, `fuzz`, `lint`,
     `fmt --check`, `vet`, `deps --locked`, `audit`.
3. ✅ **`cyrius.cyml`. Done 2026-11-15** and committed as `0e147d3`. It is written as planned below.
   - **Keys:** confirmed against `cyrius help manifest`. `entry`, `output`, `description` and `repository` are kept,
     and every stdlib leaf has a comment saying why it is there. There is no hisab (§8.2), and no `[lib]` until M1,
     because `modules = []` is invalid.
   - **`args` dropped:** the scaffold had listed it, but a library reads no command-line arguments.
     - `cyrius deps` flagged `lib/args.cyr` and `lib/args_win.cyr` as outside the footprint. They were removed, and
       `cyrius deps --lock` re-locked.
     - `args_agnos.cyr` and `args_macos.cyr` stay: `io.cyr` and the syscall peers reference them.
   - **The lock:** 43 files, and `cyrius deps --locked` confirms `lib/` matches it exactly.
   - **Dead-code elimination:** the skeleton builds to 407,872 bytes, with 806,256 bytes of dead code removed. That
     matches pravash's empty consumer at 408 KB.
   - **Gates, all green and warning-free:** `build`, `test`, `test --bounds`, `bench`, `fuzz`, `lint`, `fmt --check`,
     `vet`, `deps --locked`, `audit`.
   ```toml
   [package]
   name = "pramana"
   version = "${file:VERSION}"
   license = "GPL-3.0-only"
   language = "cyrius"
   cyrius = "6.7.18"

   [build]
   dce = true

   [deps]
   stdlib = ["syscalls", "string", "alloc", "str", "fmt", "io", "vec", "math", "ganita",
             "overflow", "tagged", "result", "freelist", "fnptr", "atomic", "bayan"]
             # sandhi joins only for the ai profile (D8)

   [test]
   stdlib = ["assert", "bench"]
   ```
   Module order: error → constants → crmath_data → crmath → mathx → json → rng → combinatorics → bridge → special →
   linalg → sort → dist → dist_cont → dist_gamma → dist_disc → dist_mvn → descriptive → kde → pca → hypothesis →
   regression → timeseries → bayesian → markov → monte_carlo → soorat (→ ai, per D8).
4. ✅ **Toolchain and math check. Done 2026-11-15** and committed as `53078cb` ([`port-parity.md`](port-parity.md) §1).
   - **The probe:** `scripts/probes/p0_semantics.cyr` against its Rust twin `p0_semantics.rs`, built in release mode
     on 1.100.0 against glibc 2.44, with `p0_compare.py` joining the two.
   - **Coverage:** 125 cases in 155 rows, warning-free:
     - 136 rows give the same bits, and 4 are identical NaNs;
     - the 15 that differ are all variants the port won't use, or L1 transcendentals.
   - **Exact as they are:**
     - floor, ceil, trunc, sqrt and abs (including −0);
     - sin and cos; atan, log2 and ln (builtin and polyfill); cbrt; the exp builtin;
     - `powi` at all six exponents (2, 3, 4, 9, 25, 49) in LLVM's square-and-multiply order;
     - `SimpleRng`'s xorshift64 through 1,000 steps for four seeds (`>>` is logical);
     - the `total_cmp` keys and the fdlibm word operations;
     - the typed constants;
     - **every exponent literal written positionally**, up to 326 characters (`MIN_POSITIVE`), each correctly
       rounded by the lexer. Lines over 120 characters fail `cyrius lint`, though, so step 6 writes those constants
       as typed globals holding their bits;
     - IEEE comparisons.
   - **Needs a helper (ledger L2):**
     - `f64_min` / `f64_max` return NaN for a NaN second operand;
     - **`f64_fract` is floor-based** (`f64_fract(−2.75) = 0.25`, Rust gives −0.75), so the port uses `x − f64_trunc(x)`;
     - **`f64_from` reads a u64 ≥ 2⁶³ as negative** (Binomial's `n`), so the port uses `_pmn_u64_to_f64` (sticky halving);
     - `f64_to` doesn't saturate like `as usize`.

     Each helper was probed: same bits on every case.
   - **A rule:** on rustc 1.100, an f64 sum starts at **−0.0** (`sum([]) = −0.0`), so every port sum starts there.
   - **Transcendentals:** 5 of 47 rows are 1 ulp apart. MPFR shows glibc misrounds three of them (exp at 239.014,
     expm1 at 0.9204, tan at 1.4939) and the stdlib/ganita two (the exp polyfill at 1, ganita's pow at (2, 0.5)).
     Neither side is correctly rounded, which confirms D2: everything goes through the correctly rounded functions.
5. ✅ **Golden-value generator. Done 2026-11-15** and committed as `30eb41b` (`scripts/golden/`, from pravash's `scripts/oracle/`;
   [`port-parity.md`](port-parity.md) §2).
   - **The crate:**
     - It depends on `rust-old/` by path, with `oracle` and `soorat-compat` on and `ai` off.
     - Its `Cargo.lock` was seeded from `rust-old/Cargo.lock` (hisab 1.4.0), and its own `rust-toolchain.toml` pins
       1.100.0.
     - A `build.rs` records the rustc.
   - **The output:** TSV format 1 with f64 values as bit patterns.
     - Every header records `pramana 1.3.0 (rust-old/), hisab 1.4.0, rustc 1.100.0; libm: glibc 2.44 x86_64 fma`.
     - `--check` refuses any other provenance, naming both.
     - Two runs give the same bytes.
   - **`tests/golden/math.tsv`: 35,597 rows in 31 functions.** It covers every std f64 method pramana calls (§8.1),
     plus the operations the step 4 probe flagged: `fract`, `u64 as f64`, `as usize`, `total_cmp`, and the fdlibm
     word operations.
     - The grids follow pramana's uses: Box–Muller's 2π·u, the Cauchy tan(π(u − ½)), Silverman's n^(−0.2), Hick's
       log2 of counts, the KS series' y⁹, y²⁵, y⁴⁹.
     - `nm -D` confirms where each result comes from. glibc supplies exactly the ten transcendentals exp, log, log1p,
       expm1, pow, sin, cos, tan, atan and log2. `cbrt` is compiler_builtins', and the rest is inline code.
   - **`validate.py`** (standard library plus ctypes) checks all 35,597 rows without the oracle:
     - the format and `modules.txt`;
     - every A/B row recomputed in Python, in Rust's order (LLVM's `powi`, Rust's `fract`, the saturating casts);
     - every glibc row through ctypes;
     - every `cbrt` row proven correctly rounded by MPFR.

     `mpfr_ref.py` is kimiya's binding core, which passes 8 self-checks.
   - **`tests/golden/math.cr.tsv`: 76 rows** where glibc misrounds, each by 1 ulp, with MPFR's correctly rounded
     value. These are the M1 primitives' 0-ulp expectations:

     | Function | Rows glibc misrounds |
     |---|---|
     | `exp_m1` | **52 of 1,010 (5.1%)** |
     | `tan` | 7 of 2,010 |
     | `ln_1p` | 6 of 1,013 |
     | `powf` | 6 of 4,400 |
     | `sin` | 4 of 2,510 |
     | `log2` | 1 of 2,510 |
     | exp, ln, cos, atan | none on these grids |
   - **Tamper checks:** a one-bit change to an `exp` row fails both `--check` and `validate.py`; an edited `.cr.tsv`
     row fails `validate.py`; a header naming glibc 2.45 is refused by `--check` with a `PROVENANCE` line.
   - **Local gates, not CI:** the corpus holds glibc 2.44's bits, so a CI runner's glibc would be refused by design.
   - **Each module's rows arrive with its milestone, as in pravash:**
     - `special.tsv`, with its Python transcription and its MPFR reference rows `special.ref.tsv`, at M2;
     - the samplers' and quantiles' rows at M3 and M4;
     - each type's serde_json documents (the JSON table) with that type;
     - the constants at step 6.
6. ✅ **Constant generator. Done 2026-11-15** and committed as `bb0f938` ([`port-parity.md`](port-parity.md) §3).
   - **What it writes:** `src/constants.cyr`, with all **112 constants**: every `const` item in `rust-old/src`'s
     library code, and the 9 Rust std float constants that code names (`f64::MAX` only appears in tests). They are:
     - the 55 fdlibm erf/erfc coefficients;
     - Lanczos's 9 coefficients and `G`;
     - Acklam's 21 coefficients, `P_LOW`, and `P_HIGH = 1.0 − P_LOW`;
     - the KS constants;
     - `LN_SQRT_2PI`, the convergence settings, the sampler thresholds, the tolerances and the Markov limits
       (`MAX_PREALLOCATED_STEPS = 1 << 16`);
     - `ai`'s two public bounds.
   - **Values from the source:** pramana's constants are private, with no hook, so the generator reads them from the
     source text (`scripts/golden/src/consts.rs`). Rust's own `str::parse::<f64>` rounds each literal exactly as rustc
     does, and the two expressions are evaluated in IEEE f64. A plain count proves the list complete.
   - **Proven against the oracle before anything is written** (`src/p_constants.rs`). Transcriptions in Rust's
     operation order, compiled by the same rustc, are compared bit for bit:
     - the fdlibm table against `oracle::erf` and `oracle::erfc` (5,061 cases each, every branch);
     - Lanczos, `G` and `LN_SQRT_2PI` against `oracle::ln_gamma` (4,009);
     - Acklam, `P_LOW` and `P_HIGH` against `oracle::z_quantile` (3,514);
     - the KS constants against `oracle::kolmogorov_sf` (3,007);
     - `POISSON_PTRS_THRESHOLD` and `BINOMIAL_DIRECT_MAX_N` by the draws either side of each threshold.
   - **The mutation hook** (`GOLDEN_NUDGE`, one ulp): 11 of 15 nudges are caught.
     - `QQ5` and `P_LOW` change no output at one ulp on any proof input: QQ5's term is about 10⁻²² relative, and
       `z_quantile`'s branches meet at `P_LOW` after the Halley step.
     - `CONVERGENCE_EPS` and `MVN_SYMMETRY_TOL` are extraction-only by design. Their modules' rows test them at M2
       and M3.
   - **The output form:**
     - a `const` with Rust's positional `Display` digits;
     - 1e-300 (`LENTZ_TINY`), `MIN_POSITIVE`, ±∞ and NaN as typed `f64` globals holding their bits;
     - names `_PMN_<PATH>` (private) or `PMN_<PATH>` (`ai`'s two public bounds).

     The file is fmt- and lint-clean, with no line over 120 characters.
   - **Independent checks:**
     - `tests/golden/constants.tsv` pins each value;
     - `validate.py` re-reads all 112 with its own Python extractor, and they match;
     - `probe_tables.py` compiles the file warning-free, and all 112 read back exact.
   - **Tamper checks:** one digit changed in `src/constants.cyr` fails `probe_tables.py` and `--check`; an edited
     `constants.tsv` value fails `validate.py`.
7. ✅ **Parity harness. Done 2026-11-15** and committed as `d7941bc` ([`port-parity.md`](port-parity.md) §4).
   - **The files:**
     - `tests/ulp.cyr` is pravash's, unchanged.
     - `tests/parity_core.cyr` is pravash's final version (2.0.1), which already handles correctly rounded rows. It is
       adapted to pramana's ADRs:
       - a correctly rounded row passes as *accepted (ADR 9)*;
       - pravash's `.d3.tsv` table becomes pramana's mapped rows, `<module>.map.tsv`, accepted as *ADR 7*;
       - two print calls whose lengths didn't match their literals in pravash's copy are fixed, and every literal's
         length is now audited by script.
     - `tests/parity.tcyr` holds pramana's hooks and dispatchers.
     - `tests/constants_rows.cyr` is the generated lookup from each path to its value.
     - `validate.py` writes `math.cr.tsv` in the siblings' four-column format, which the harness reads.
   - **First run:** 2 modules, 0 failing, warning-free and the same under `--bounds`. Of the rows:
     - **3,963 ok:** all 112 constants, and sqrt, abs, floor and the fdlibm word operations;
     - **24,669 measured:** the transcendentals on their stdlib and ganita counterparts, the step 8 baseline;
     - **7,077 pending:** the helpers M1 brings.

     The 76 correctly rounded rows load and match their golden rows.
   - **Mutation-checked four ways**, each failing its module:
     - one bit of a golden `sqrt` row;
     - `LN_SQRT_2PI` one ulp off in `src/constants.cyr`;
     - `floor` dispatched to `f64_ceil` (613 rows fail);
     - a correction row matching no golden row.
   - **In every run:** `cyrius test` and `cyrius audit` now run the harness, about 1 s for the suite. The comparison
     reads only committed files, so CI runs it too.
   - **Comparators that arrive with their modules, as in pravash:**
     - the `*.ref.tsv` comparator (M2 part b);
     - the stream comparator (M3, the samplers);
     - the JSON comparator (M1, the first codecs);
     - the mapped-rows reporting (ADR 007's first remap).
8. ✅ **Baselines. Done 2026-11-15** and committed as `aec9861` ([`port-parity.md`](port-parity.md) §5).
   - **Method:** the harness prints a 64-bit FNV-1a digest per function, folded from every output word in row order
     (pravash's). It runs three ways: `cyrius test` (x86_64 ELF), `--win` (x86_64 PE, under wine) and `--aarch64`
     (under qemu). All three are warning-free.
   - **Identical on all three targets for every dispatched function:** the 11 transcendental counterparts, all class
     A/B math rows, and the constants.
   - **The native builtins differ on aarch64, as designed:** `f64_exp`, `f64_ln`, `f64_atan` and `f64_log2` are x87
     on x86_64. On aarch64 each one's digest equals the dispatched polyfill's x86_64 digest, which independently
     confirms the polyfills are the cross-target forms.
   - **The bounds** (`_par_c_bound`), set before each function's first gated comparison:
     - **K = 2** for the interim counterparts of exp, ln, ln_1p, exp_m1, sin, cos, tan, atan and log2 (the siblings'
       rule for primitives; measured max 1 on all three targets);
     - **K = 0** for cbrt, since both sides are correctly rounded (measured max 0);
     - **powf stays measured:** ganita's open `pow(−1, huge)` defect gives +∞ where Rust gives 1, and M1 replaces
       ganita's pow.

     At M1 the correctly rounded functions arrive, and every bound becomes 0 (ADR 009).
   - **Gated result, identical on all three:** 0 failing; 24,162 ok, 4,400 measured (powf), 7,077 pending, and
     70 corrected rows accepted (ADR 9). Until M1, "accepted (ADR 9)" means within K = 2 of the correctly rounded
     value.
   - **Mutation-checked at the edges:**
     - exp's bound set to 0 fails its 264 one-ulp rows;
     - exp 3 ulps off fails 3,129 rows under K = 2;
     - exp 2 ulps off fails only the 246 rows it pushes to 3 ulps or into NaN/∞;
     - cbrt 1 ulp off fails 809 of 810 rows under K = 0.
9. ⏳ **Probes**, each a small program answered with numbers before M1 writes code (`scripts/probes/`, with Rust twins
   in `scripts/probes/rust/`; results in `port-parity.md` §6). Run in five parts:
   - **9a ✅ (2026-11-15): P1, P5, P6.**
     - **P1:** all 3,736 cases are bit-identical to the oracle. These cover 10⁶ RNG draws from each of five seeds,
       and the `PmnU128` prototype under `factorial`, `permutations` and `combinations` across the u64 range,
       including Rust's intermediate-overflow errors.
     - **P5:** transcribe all four hisab routines. ganita and hisab 3.5 differ from 1.4.0 (Cholesky up to 40 ulp,
       solve 18, eigenvalues 14, least squares 13,619), and every call leaks.
     - **P6:** Rust's unstable selection leaks the sign of a zero into `median` and `percentile` (9,393 of 180,000
       zero-heavy results). **Decided 2026-11-15: accept the divergence** (ADR 007 row 14). The port selects from a
       stable sort, and the rows where Rust's zero differs are mapped. rustc 1.100's `select_nth_unstable_by`
       (about 300 lines of Rust's core library) was not chosen.
   - **9b ✅ (2026-11-15): P2, P3, P4** (`port-parity.md` §6.2).
     - **P2, dispatch:** `callptr` to a known `&f` costs what a direct call costs. A bounded generic is a static call.
       A `dyn` costs about 3 ns more per call (a quarter of a 12 ns draw, 7 ns with an exp body). **D7 amended
       2026-11-15:** the RNG is generic over `R: PmnRng`, as Rust's `impl Rng`, with `dyn` only for Gibbs'
       conditionals, as Rust's `&mut dyn Rng`.
     - **P3, callbacks:** every form (`callptr(&fn, ctx, …)`, a closure with or without captures) is within 1–3 ns of a
       direct call and gives the same bits. Calls allocate nothing; a capturing closure's environment is 16 bytes,
       once.
       - **D7's callback form stands.** Its rejected column is corrected: a Gibbs conditional that captures its
         context needs 3 parameters, which fits Windows. A 4-parameter capturing closure is a compile error on PE.
       - **Gibbs' RNG:** a generic sampler forms the `dyn` at a call into a non-generic helper. The helper keeps it in
         a stack record and passes the record's address. Nothing allocates.
       - **Found:** a captured f64 is an untyped word in arithmetic (`k * x` gives 0 for 3.0 × 0.25, with a warning),
         and `cyrius lint` refuses a capturing closure without an explicit `include "lib/alloc.cyr"`. Both are filed
         at step 10; the rules are in §8.3.
     - **P4, the flat heap:** D11's conventions prototyped and soaked. Every soak leaves the heap flat: 10⁶ pdf/cdf
       calls with constructors into caller storage, 10⁵ MVN pdfs, 10⁴ and 10³ MVN new/drop cycles, 10⁶ MH samples,
       10³ Baum–Welch iterations, 10 medians of 10⁶, 10⁶ caller-buffer calls, and pool overflow.
       - **The pool** changes kimiya's ADR 0007 in one way: slots grow on the freelist and free what they outgrew, and
         an overflow claim is a freelist block (§8.5).
       - **Findings for later:** an owned object over 4 KB pays the freelist's open large-block issue (36 µs per MVN
         new/drop at d = 32). The stable median by merge sort takes 222 ms for 10⁶; M4 can make it O(n) with a stable
         three-way quickselect.
   - **9c ✅ (2026-11-15): P7, P8** (`port-parity.md` §6.3; committed as `155dce0`).
     - **P7, JSON:** serde_json 1.0.154 wrote 1,495 documents for nine types that carry every shape pramana
       serializes, with serde_json's verdict and value for each. pramana decodes each one through bayan's tree, serde's
       struct rules and the constructor's checks.
       - **bayan's tree alone** agrees on 1,325. It differs on integers at or above 2^63 (108; 13 of the 35 saved
         `SimpleRng` states are refused), on raw control bytes (14) and on `-0` (13).
       - **A stand-in pre-pass** fixes all 135 (§8.3, ledger L11). The 35 left are documents serde_json accepts only
         because it skips a member unchecked: a duplicate key, nesting past 128 or a lone surrogate inside it.
       - **Error codes:** every agreeing refusal maps a constructor's refusal to its code and the rest to
         `DESERIALIZE`.
       - **Text:** impetus's `fmtf.cyr` writes serde_json's text on all 596 documents both read; bayan's Grisu2 on 566.
         Over 280,562 doubles, `fmtf.cyr` differs nowhere and Grisu2 on 9,714.
       - **Decided 2026-11-15:**
         - D5 amended: pramana writes serde_json's text byte for byte, with `fmtf.cyr` vendored once for this and
           D12's Debug form (ADR 007 row 7);
         - ADR 007 row 16 accepts the 35 until bayan's opt-in for unread members lands.
       - **serde_json's default parse** misreads a float in 68 of the 596 documents, whichever printer wrote them. The
         golden JSON rows are generated with `float_roundtrip` on, as ushma's and impetus's are.
     - **P8, the bundle:** three layouts of a core stand-in, an `ai` stand-in on sandhi, and pramana's tests.
       - **Dead-code elimination:** a one-function consumer is the size of an empty one (411,928 bytes on x86_64),
         every core function adds 65,536, and a core consumer never compiles sandhi.
       - **`ai`** adds 839 KB and about 1 s of build. A profile bundle and an outside file give the same binary.
       - **sandhi in `[deps] stdlib`** costs every pramana compile about 0.8 s (build 1.30 s against 0.49 s).
       - **Decided for D8:** the `[lib.ai]` profile, with sandhi out of `[deps]`. The `ai` test includes
         `lib/sandhi.cyr` itself, and `distlib --check`, `deps --locked` and `audit` pass.
       - **PE and aarch64:** dead code is NOPed, not compacted (cyrius's roadmap: "the DCE compaction arc").
   - **9d: P9's prototypes**, one part per function. `normal_cdf` comes last, after 9e, because its T1 calls the
     correctly rounded erfc that 9e ports.
     - **9d-1 ✅ (2026-11-15): `ln_gamma`** (`port-parity.md` §6.4; the probe is `998b64b`).
       - **The prototype:** fdlibm's `e_lgamma_r` layout on correctly rounded ln, with double-double logarithms
         (kimiya's ln core), a double-double product on [3, 8) and for x ≥ 8, and an exact sin(πx) reduction and a
         double-double subtraction for x < 0.
       - **Measured against MPFR** on 1,675,000 rows (§3.2's sets plus six seeded sets weighted to the roots,
         [2, 8), huge and subnormal x, the poles and the negative-axis zeros):
         - **x > 0:** at most 1.31 ulp (mean 0.26), against the fixed ≤ 2. Rust is off by up to 4.3×10¹³;
         - **x < 0:** at most 1.40 ulp of max(|lnΓ|, 1), against ≤ 2.
       - **The same bits** on x86_64, PE and aarch64.
       - **Cost:** 72–192 ns for x > 0, against T0's 168; 570 ns for x < 0, against 400.
       - **Found:** Rust 1.3.0 returns +∞ for 0 < x < about 3.2e-301. Its reflection gives up when |sin(πx)| < 1e-300.
     - **9d-2 ✅ (2026-11-15): `gamma_inc`** (`port-parity.md` §6.5).
       - **The prototype:**
         - a double-double prefactor exponent: lnΓ from a ζ-series table below a = 10, and Rust's Stirling kernel
           with an exact u − ln(1 + u) from there;
         - the series and the continued fraction in double-double;
         - the split at x = a;
         - a direct small-a Q.
       - **The reference** is a 320-bit series and continued fraction in `check.c`, because `mpfr_gamma_inc` takes up
         to 0.8 s a call near a = 1e5. It is checked against `mpfr_gamma_inc` to 6×10⁻⁷⁵ ulp.
       - **Measured on 360,400 rows** (§3.2's pairs and five seeded sets over a ∈ [10⁻³, 10⁵]): T1 is at most
         1.0 ulp (mean ≤ 0.25), against ≤ 4. Rust reaches 97 / 485 ulp on §3.2's own pairs, and 2.9×10⁹ on P in the
         deep lower tail.
       - **The same bits** on x86_64, PE and aarch64.
       - **Cost:** 2.5–64 µs a call, against T0's 0.23–3.8 µs, which is 10–17×.
       - **The cost experiment:** T1 with Rust's plain-double loops (T1b) misses, at up to 79 ulp near x = a for large
         a and 58 on Q for small a just past x = 1.1. M2 part b should speed those two regions with Temme's uniform
         expansion (after 9e's erfc) and a wider direct small-a form, not with less precision.
     - **9d-3 ✅ (2026-11-15): `beta_inc`** (`port-parity.md` §6.6).
       - **The prototype:**
         - the smaller of x and y taken as exact;
         - a double-double prefactor, with lnΓ a double-double at every argument, so ln B's cancellation costs nothing;
         - the continued fraction in double-double;
         - TOMS 708's power series for a complement below 2⁻⁶.
       - **The reference** is a 320-bit continued fraction in `check.c`, checked against exact rationals on §3.2's
         integer shapes to 4×10⁻⁷⁸ ulp.
       - **Measured on 214,744 rows** over a, b ∈ [10⁻³, 10⁵]: T1 is within 0.51 ulp (mean ≤ 0.25), against ≤ 8. Rust
         reaches 550 / 583 on §3.2's rows and 1.7×10⁵ near the mean with one large shape.
       - **Three T1 defects** were found and reworked on the way: a zeroed prefactor near the subnormal range, a
         complement by the far side's slow continued fraction, and Stirling's series in plain double.
       - **The same bits** on three targets.
       - **Cost:** 7.5–71 µs, against T0's 0.75–4.2 µs, which is 10–17×.
     - **9d-4 ✅ (2026-11-15): `normal_cdf`** (`port-parity.md` §6.8).
       - **The prototype:** −z/√2 as a double-double h + l. The first-order correction for l is applied to
         CORE-MATH's double-double erfc(h) before it rounds: from the fast path where its proven bound is under 2⁻⁶⁰
         relative, else from the accurate paths. The ½ is folded into one rounding, exact on the subnormal grid too.
         Proven within 0.5 + 2⁻⁷ ulp.
       - **Measured against MPFR** on 2,506,918 rows: §3.2's sets and nine seeded sets (tails, the deep tail's
         subnormal results, the decimal grid z = k/1000, `cr_erfc`'s branch points):
         - **T1:** within 0.5 ulp on all 1,246,809 of its rows, and correctly rounded on all but 4. The bound is ≤ 1;
         - **Rust:** 1,590 ulp on §3.2's rows and 1,688 in the deep tail;
         - **the correctly rounded erfc on Rust's rounded argument:** 1,688 ulp. The argument's rounding is the error;
         - **§3.4's sketch,** which rounds twice: exactly 1.0 unit on subnormal results, and 25% misrounded.
       - **The same bits** on x86_64, PE and aarch64.
       - **Cost:** 1.3 µs in (−5, 5), 3.8 µs where CORE-MATH's accurate path is taken, and 2.3–4.6 µs in the tail,
         against T0's 0.15–0.24 µs (8–20×). A 2⁻⁵⁶ fast bound (0.5 + 2⁻³ ulp) takes 30% off the core; M2 part b
         chooses.
   - **9e ✅ (2026-11-15): the CORE-MATH erf/erfc port, without FMA** (`port-parity.md` §6.7).
     - **The port:** CORE-MATH's binary64 `erf.c` and `erfc.c`, function for function, with every table generated
       from the sources (`p9_coremath_tables.py`: 17 tables, 3,929 words). Of the 59 fma sites, 2 are Dekker products
       and 57 exact fmas (Dekker's product, then a round-to-odd three-term sum). The last fma of erf's tiny path can
       have a subnormal product, so it is rounded once on the subnormal grid instead.
     - **Stage 1, in C:** the emulation matches hardware fma on 10⁸ triples, and CORE-MATH built with it matches
       CORE-MATH with hardware FMA on all 369,190 worst cases (both signs) and 4×10⁷ arguments.
     - **Stage 2, the Cyrius port:** CORE-MATH's own bits on 2,568,534 rows (the worst cases, and 1,049,344 seeded
       erf and 1,150,000 seeded erfc arguments). MPFR at 1,024 bits finds every row correctly rounded: 2,768,518,
       with 100,000 through each accurate path alone.
     - **The same bits** on x86_64, PE and aarch64.
     - **Cost:** 508–675 ns on the main ranges and 2.3 µs on erfc's asymptotic range, against glibc's 22–76 ns. Most
       of it is the emulated fma, so a scalar fma is requested (§10, L12).
     - **Inside the slot,** so D3 = C stands without the fallback.

   The probes as planned:
   1. **Integers.** 10⁶ xorshift64 draws and their f64 mapping against Rust (class A); `PmnU128` against Rust's u128
      on `factorial(0..=34)`, `permutations` and `combinations` (D10).
   2. **Dispatch.** Cost per `pdf` call and per draw for a checked trait with generics, for `dyn`, and for a function
      pointer (D7).
   3. **Callbacks.** A function pointer plus context against a closure, for Monte Carlo integration, MH and Gibbs:
      cost, allocation, and the 3-parameter Windows limit.
   4. **Memory.** The heap high-water mark must flatten under: 10⁶ pdf/cdf calls per distribution, 10⁵ MVN pdfs,
      10⁶ MH samples, 10³ Baum–Welch iterations, and medians of 10⁶ in a loop (D11 and the pool).
   5. **Linear algebra against hisab 1.4.0.** `ganita_mat_cholesky` / `ganita_mat_cholesky_solve`, hisab 3.5's
      `eigen_symmetric` and ganita's least squares, against 1.4.0's `cholesky`, `cholesky_solve`, `least_squares_poly`
      and `eigen_symmetric`. Use seeded SPD, Vandermonde and covariance matrices; record bits and allocations. Each
      routine is then marked "use" or "transcribe" (§8.2).
   6. **Sort and select** (done in 9a; decided as ADR 007 row 14). The f64 quickselect and introsort stand-in against Rust's `select_nth_unstable_by` and
      `sort_unstable_by`, on tie-heavy and ±0-heavy inputs (median, percentile, KS, mode, PCA's eigenvalue order).
   7. **JSON.** Hand codecs for the 38 types, fed 1.3.0's documents: bits, round trips, how often Grisu2's digits
      differ, validation refusals, non-finite values (`null` on both sides), the error form.
   8. **Bundle.** Dead-code elimination for a one-function consumer; the core with and without `ai` (sandhi, TLS);
      a `[lib.ai]` profile against an outside-bundle file (D8).
   9. **T1 feasibility** (D3 = C).
      - Prototype the dd-argument `normal_cdf`, root-centred `ln_gamma`, and the dd prefactors of `gamma_inc` and
        `beta_inc`. Measure each against MPFR with `scripts/port-baseline/`, plus its cost. Check each against its
        **fixed** bound (§3.4). A prototype that misses shows where the algorithm needs more precision before M2 part b
        starts; it is never a reason to move the bound. ADR 0009 records the bounds as fixed.
      - Port CORE-MATH's erf and erfc without FMA, in a fixed time slot. Classify every fma site: a Dekker product, or
        an exact software fma. Verify as §3.4 says; if it misses the slot, fall back to ≤ 1 ulp (§3.4).
10. **✅ (2026-11-15) File the upstream requests and seed the ledger** (§10).
    - **Filed:** eight new issues:
      - cyrius: the captured f64, lint without the manifest, a scalar fma (each with a repro);
      - ganita: the Cholesky family (with a repro), the special functions' scope;
      - hisab: polynomial least squares, `eigen_symmetric`'s tolerance and scratch;
      - hoosh: a client bundle.

      A fifth repro, for the fract and u64 conversions, goes with pramana's section of the joined f64 issue.
    - **Joined:** thirteen open issues, each with pramana's probe numbers:
      - cyrius: the f64 surface, stable sort, `vec`'s aborts, the derive's shapes, `Debug` and the shortest printer,
        the freelist's large blocks;
      - ganita: the correctly rounded tier, as its sixth consumer, with per-function misround counts from the golden
        corpus;
      - bayan: the four JSON tree issues, the shortest printer, the nesting cap.
    - **Checked as filed:** each repro was run from its repo's root and shows its gap on cyrius 6.7.18. Each 1.4.0
      behaviour quoted was read from hisab 1.4.0's source. A finding on the way: the stdlib's vector `f64v_fmadd` rounds
      twice on x86_64 (`mulpd` + `addpd`) and once on aarch64 (`fmla`), so it is not the same function on the two
      targets (the fma issue records it).
    - **The ledger:** §10's table in pravash's shape, read by `scripts/upstream_ledger.py` (pravash's, which also
      checks the requests filed with no stand-in) and run in CI. Its first run caught an unmarked stand-in:
      `src/constants.cyr`'s typed std constants (L2). The generator now writes the `UPSTREAM(L2)` note. 12 rows, 1 note,
      0 failures.
    - **The gate:** build, `test` (warning-free) and `test --bounds`, `bench`, `fmt --check` and `lint`, `vet`,
      `deps --locked`, `audit` and the ledger pass. What remains is the maintainer's commit.

**Gate:**
- `cyrius build` and `cyrius bench` pass;
- `cyrius test` runs the harness with no warnings;
- `lint` and `fmt --check` are clean;
- `scripts/upstream_ledger.py --check` passes;
- committed: the hooks (in `rust-old/`), the generator, `validate.py`, the harness, ADRs 006–009, the probe results,
  the ledger.

---

## 6. Parity policy

tanmatra's classes apply, with pravash's R class and a stream class:

| Class | Tolerance | pramana examples |
|---|---|---|
| **A — exact** | bit-identical, equal integers | error codes and their order of checks; validation verdicts; combinatorics; `mode`; `viterbi` paths; Markov `step`/`simulate` states; `naive_bayes_classify`; logistic class labels; `reject` flags away from α; xorshift words; draw counts; decoded JSON; constants |
| **B — arithmetic** | ≤ 2 ulp, expected 0 | `mean`, `variance`, moments, `sum`; linear-regression closed forms; `bridge`; moving average and smoothing; `difference`/`integrate`; KS statistics; correlation; `powi` sites |
| **C — transcendental** | 0 ulp against `*.cr.tsv` (T0); K by the port-parity rule downstream of T1 | every pdf/cdf/sf; samplers per draw; KDE kernels; `bayes_theorem`; HMM forward/scaling; MH acceptance; bridge `log2`/`ln` fits |
| **D — iterative or library-backed** | relative ≤ 1e-12, or a per-function bound set before its first comparison | `steady_state`, Baum–Welch, logistic regression's Newton steps, PCA's Jacobi, `t_quantile`; any ganita path probe P5 keeps |
| **S — stream** | per draw, the class of the path that produced it; counts class A | sampler sequences, MCMC chains, Gibbs sweeps, `monte_carlo_*` |
| **R — reference** | the stated bound | `*.ref.tsv` (T1); NIST StRD; the statistical tests of 1.3.0's suite (moments, KS at fixed seeds); the 15 integration tests |

Rules:
- **Never widen a class to make a row pass.** A failure is one of three things: a port defect; a library defect,
  which is filed upstream; or a 1.3.0 defect, which is fixed as 1.3.x and regenerated. The only exceptions are
  ADR 0007 / 0009 rows, which the harness still reports by name.
- **Branch points are tested on both sides:**
  - `CONVERGENCE_EPS` and the iteration budgets;
  - the series/continued-fraction switches (gamma: x < a + 1; beta: x < (a + 1)/(a + b + 2));
  - Wilson–Hilferty at a > 1e9;
  - `stirling_error` at a ≥ 10;
  - the Student-t pdf form at df ≥ 20, and the large-shape `ln_beta` and beta-prefix forms;
  - PTRS at λ > 30, direct Binomial at n ≤ 64, and every accept/reject test;
  - α boundaries in `reject`;
  - `z_quantile`'s region splits;
  - the KS series split at λ = 1.18;
  - `MVN_SYMMETRY_TOL`;
  - `steady_state`'s tolerance and stall limit;
  - the Newton/IRLS stopping test and the Jacobi tolerance;
  - the HMM zero-probability errors.
- **Accuracy scope:**
  - correctly rounded primitives everywhere (T0);
  - the T1 functions only in `src/special.cyr` (D3);
  - in every other chain, Rust's operation order is the definition (D4).

---

## 7. Milestones

Every milestone, and every part of a large one, ends with pravash's gates:
- `cyrius build`, then `test` and `test --bounds` with **no warnings** (the f64 type diagnostics appear only there);
- `bench`, recorded against Rust;
- `vet`, `lint`, `fmt --check`;
- `distlib --all`, `distlib --check`, `deps --locked`;
- surface parity;
- the soak test (probe 4) for anything that allocates;
- the milestone's JSON and `Debug` rows;
- the ledger check, filing new gaps and converting any stand-in whose library has shipped;
- the three targets;
- CHANGELOG and `state.md`.

| Milestone | Modules | Key work | Golden grid (minimum) |
|---|---|---|---|
| **M1 Foundation** | error, constants, crmath, mathx, json, rng, combinatorics, bridge | `PMN_ERR_*`. The correctly rounded primitives: vendored, plus new log1p, tan and log2, each verified on ≥ 1M MPFR arguments on 3 targets (D2). The Rust-semantics helpers. `PmnSimpleRng` and the `PmnRng` trait. `PmnU128`. The 11 bridge functions. The work-table pool. JSON core and the `SimpleRng` codec (a zero state maps to 1) | every primitive's `*.cr.tsv` at 0 ulp; 10⁶ RNG words; `factorial(0..=35)` and the overflow edges; bridge inputs over their ranges, including NaN, ±∞ and empty slices |
| **M2 Special functions** (a: T0, b: T1) | special | a: `math.rs` in Rust's order, covering erf/erfc (fdlibm), `normal_cdf` / `normal_sf`, `ln_gamma`, `stirling_error`, `bd0`, `ln_gamma_kernel`, `ln_beta`, `beta_inc`, `gamma_inc`, with every branch. b: the five T1 replacements under ADR 0009's bounds, erf and erfc correctly rounded (D3 = C) | a: dense grids across every branch point, both tails, shapes from 1e-3 to 1e12; `*.cr.tsv` at 0 ulp. b: `*.ref.tsv` at the bounds; §3.2's sets re-measured and recorded in `port-parity.md` |
| **M3 Distributions** (a–d) | dist, dist_cont, dist_gamma, dist_disc, dist_mvn | a: Normal, Uniform, Exponential, Cauchy, Weibull, Bernoulli. b: Gamma, Beta, χ², Student-t, F (Marsaglia–Tsang). c: Poisson (Loader pdf, PTRS), Binomial (saddle-point pdf, `beta_inc` CDF, Beta splitting). d: `MultivariateNormal` (Cholesky, `log_det`, symmetry tolerance; decode recomputes the factor). Validated JSON on all 14 | pdf/cdf/mean/variance over each parameter range and its edges; 10⁴-draw streams per sampler at 3 seeds; every constructor refusal; JSON with invalid parameters refused |
| **M4 Descriptive and hypothesis** (a, b) | sort, descriptive, kde, pca, hypothesis | a: the stats, O(n) median/percentile, `mode`'s first-occurrence ties, KDE (4 kernels, Silverman), correlation matrix, PCA. b: t-tests, Welch, χ² (exact p), ANOVA, KS with ties, the CIs, z/t quantiles | NaN/empty/constant inputs; tie- and ±0-heavy sorts (P6); p-values across df, including large df; NIST StRD univariate and ANOVA (class R) |
| **M5 Regression, time series, Bayesian** | regression, timeseries, bayesian | centered linear, polynomial (transcribed MGS), logistic (Newton with Cholesky, l2), predictions and residuals; moving average, smoothing, ACF, `difference`/`integrate`, ARIMA fit (Yule–Walker on Cholesky) and forecast; Bayes and naive Bayes | NIST StRD Norris, Pontius, NoInt, Longley, Wampler, Filip (class R; failures are D4 findings); logistic on separable and non-separable data; ARIMA d = 0..3 |
| **M6 Stochastic** | markov, monte_carlo | `MarkovChain` (bounds-checked step, simulate capped, steady state with the stall switch), HMM (scaled forward, Viterbi, scaled Baum–Welch with O(T·N) emission counts); Monte Carlo integration (Welford), π, MH (NaN treated as −∞, escape from a zero-density start), Gibbs | chains: periodic, absorbing, reducible; HMM underflow lengths (10⁴ observations); MCMC streams at fixed seeds; all refusals |
| **M7 Integrations** | soorat, ai | the soorat data shapes and `HistogramData::from_data` (non-finite skipped, the ulp floor); `ai` per D8, with the five tools, the untrusted-argument bounds, and a recorded-response transport | histogram edges at 1e12; every tool on fixture responses; unknown tool and column errors; bounds refusals |
| **M8 Release 2.0.0** (a–e) | all | a: the Rust-test ledger, accounting for all 319 tests. b: the benchmark record, 34 rows in both columns. c: the full surface diff, README, `docs/guides/migration-2.0.md`, CHANGELOG *Breaking*, examples compiled and run. d: the fuzz harness for the 38 JSON readers. e: the release gate on 3 targets, the Rust CI job removed, VERSION 2.0.0 | the full corpus green on x86_64 ELF, PE and aarch64 |

**M1's parts** (set at its start, 2026-11-15), in the manifest's module order:
- **a:** `src/error.cyr` ✅;
- **b1:** `src/crmath_data.cyr` and `src/crmath.cyr`, kimiya's correctly rounded exp, expm1, ln, pow, sin, cos and
  atan2 vendored by `scripts/vendor_crmath.py`, verified by `scripts/crverify/` on ≥ 1M MPFR rows each ✅;
- **b2:** a correctly rounded log1p, new, in `src/crmath_ext.cyr`, on kimiya's ln cores ✅;
- **b3:** a correctly rounded tan, new, on kimiya's sin and cos ✅;
- **b4:** a correctly rounded log2, new, on kimiya's ln with the exponent kept exact ✅. Part b is complete: every
  class-C math row of the golden corpus is at 0 ulp;
- **c:** `src/mathx.cyr`, the Rust-semantics helpers;
- **d:** the JSON core, `src/fmtf.cyr` and the `SimpleRng` codec;
- **e:** the RNG, combinatorics with `PmnU128`, and the bridges;
- **f:** the work-table pool, then M1's gate.

**Order:** M0 → M1 → M2 → … → M8.
- The special functions come first because almost everything else calls them, and T1 must land before any downstream
  row is generated against it (D3).
- Distributions next, because hypothesis tests, MCMC and the bridges sample from or evaluate them.
- `linalg` lands with its first user (M3d).
- Markov and Monte Carlo come last among the libraries: they depend only on the RNG and the distributions, and their
  stream rows are the costliest to generate.

---

## 8. Rust → Cyrius mapping (pramana specifics)

### 8.1 Math functions

These are every std f64 method in `src/` outside tests, as of 1.3.0.

| Rust (calls) | Cyrius | Parity |
|---|---|---|
| `sqrt` (25), `abs` (38), `floor` (4) | builtins | exact on every target |
| `max` (21), `min` (9), `clamp` (10) | `_pmn_max` / `_pmn_min` / `_pmn_clamp` with Rust's NaN-ignoring semantics (`f64_min` / `f64_max` return NaN for a NaN second operand, port-parity §1) | exact |
| `fract` (2: the Poisson and Binomial pdfs' integer test) | `x − f64_trunc(x)`. **Never `f64_fract`**, which is floor-based (`f64_fract(−2.75) = 0.25`) | exact (probed) |
| `u64 as f64` (Binomial's `n`, `trials`, `successes`) | `_pmn_u64_to_f64`: `f64_from` reads a word ≥ 2⁶³ as negative, so halve with a sticky bit, convert and double | exact (probed at 2⁶³, 2⁶³ + 2¹¹ + 1, u64::MAX) |
| `f64 as usize` (3: `percentile`'s rank, the iteration budget, the histogram bin) | `_pmn_sat_usize`: NaN and negatives → 0, ≥ 2⁶⁴ → u64::MAX | exact (probed) |
| `iter().sum::<f64>()` | a loop that **starts at −0.0**, as rustc 1.100's `Sum for f64` does (`sum([]) = −0.0`) | A on the sign of zero (probed) |
| `is_finite` (39), `is_nan` (23) | `x - x == 0`, `x != x` on typed locals | exact |
| `exp` (32), `ln` (53), `powf` (7), `sin` (2), `cos` (3) | kimiya's correctly rounded primitives (D2, L1) | C, 0 ulp against `*.cr.tsv` |
| the T1 special functions' logarithms (M2 part b) | kimiya's double-double ln core (`_km_ln_core`: ln x as hi + lo, relative error near 2⁻⁸⁰), kept callable inside pramana beside the correctly rounded ln (P9, §5 step 9d-1) | the T1 bounds (ADR 009) |
| `ln_1p` (11), `exp_m1` (2), `tan` (1), `atan` (1), `log2` (1) | correctly rounded: expm1 from kimiya; log1p, tan and log2 new; atan as atan2(x, 1) | C, 0 ulp |
| `cbrt` (1, Wilson–Hilferty) | `ganita_f64_cbrt`, exact like Rust's compiler_builtins | C, expected 0 (probe 4) |
| `powi` (17: exponents 2, 3, 4, 9, 25, 49) | explicit products in LLVM's square-and-multiply order: the result accumulates the set bits low to high, e.g. x²⁵ = (x · x⁸) · x¹⁶ | exact (probed) |
| `total_cmp` (3), `partial_cmp` (2) | integer compare on the sign-folded bits; `f64_lt` on NaN-free data | A |
| fdlibm `high_word` / `low_word` | shifts and masks on the word | A |
| `f64::{EPSILON, MIN_POSITIVE, INFINITY, NEG_INFINITY, NAN}`, `consts::{PI, E, TAU}` | typed globals generated with their bits (M0 step 6) | A |

**Typing traps** (guide *Types*):
- Bind every builtin and ganita result to a `: f64` local.
- Write `-x`, not `0 - x`.
- There is no exponent notation, so write `1e-15` out positionally or generate it.
- A hex pattern in a `const` is an integer.
- The `F64_*` stdlib constants are untyped.
- f64 type warnings appear only under `test` and `build`.

### 8.2 hisab 1.4.0 calls → Cyrius

| Rust (hisab 1.4.0) | Cyrius 6.7.18 / hisab 3.5.0 | Plan |
|---|---|---|
| `num::cholesky` (`Vec<Vec<f64>>`; MVN, ARIMA, logistic) | `ganita_mat_cholesky(m, out_l)`: lower triangle; refuses a pivot ≤ 0, NaN or ∞. hisab has no public dense Cholesky | use it if P5 shows 1.4.0's bits; otherwise transcribe (L4) |
| `num::cholesky_solve` (MVN pdf, ARIMA, logistic) | `ganita_mat_cholesky_solve`: **allocates n words per call**, and an MVN `pdf` in a loop would leak | an allocation-free transcription (L4), until ganita offers an `_into` form |
| `num::least_squares_poly` (MGS QR on the raw Vandermonde) | none: ganita's `mat_least_squares` is Householder and allocates | transcribe 1.4.0's (L5); start from kimiya's `_km_least_squares_poly` |
| `num::eigen_symmetric(a, 1e-12, 1000)` (Jacobi; PCA) | hisab 3.5 `eigen_symmetric`: relative Jacobi, sorted by \|λ\|, no tol/max_iter. ganita `mat_eigen_sym`: absolute stop at 1e-12, with an open accuracy issue | transcribe 1.4.0's Jacobi (L6); PCA rows class D |

This leaves no hisab call in the library. hisab would appear only at test scope, if P5 needs it for comparisons,
which keeps every consumer's compile free of it.

### 8.3 Types and idioms

| Rust | Cyrius |
|---|---|
| distribution structs (2–3 f64 fields) | `PmnNormal` and the like, filled in caller storage by `PmnNormal_new(out, …)` (D11) |
| `impl Distribution for X` | `impl PmnDistribution for PmnX`; methods keep their names (`d.pdf(x)`) |
| `&[f64]` | pointer plus count |
| `Vec<f64>` returns | `(out, cap)` plus a count returned, or an owned result with `_drop` (D11) |
| `Vec<Vec<f64>>` (covariance, transition and emission matrices) | flat row-major f64 arrays with dimensions |
| `Option<f64>` (`select_pair`) | an out-pointer plus 1/0 |
| `Result<T, PramanaError>` | value-form `Result` with a `PMN_ERR_*` code; never through `callptr` |
| `impl Fn…` | function pointer plus context word (D7); a parameter declared `f: *fn(f64): f64` takes `&fn` or a closure (P3) |
| a closure capturing an f64 (tests, examples) | copy the capture into a typed local before arithmetic: `var kf: f64 = k;` (P3) |
| `rng: &mut impl Rng` | `rng: *R`, with `<R: PmnRng>` on the fn or method: `fn sample<R: PmnRng>(self, rng: *R): f64` in `PmnDistribution` (D7, amended after P2) |
| `&mut dyn Rng` (Gibbs' conditionals) | the address of a stack `PmnRngRef { o: dyn PmnRng }`, formed by a non-generic helper (P3) |
| serde | `src/json.cyr` hand codecs; decode calls the validating constructor (D5). Before bayan parses, a pre-pass (P7's `p7_lift`, ledger L11) refuses a raw control byte in a string and rewrites integers: a negative one becomes a float, so `-0` stays −0.0; one in 2^63 … 2^64 − 1 becomes the negative integer with its bits; a larger one becomes a float. Then serde's struct rules: skip an unknown member, refuse a known one twice or missing, read the sequence form. Writing gives serde_json's compact text byte for byte: f64s through `src/fmtf.cyr`'s serde_json form, strings through `str_builder_add_json_str` (D5, ADR 007 row 7) |
| a u64 or usize field | a JSON integer only (a float, even `1.0` or `-0`, refuses); the pre-pass's negative integers are its values at or above 2^63 (P7) |
| an f64 field | any JSON number, refusing one that parses to ±∞ (serde_json's "number out of range"); `null` refuses, so a document holding NaN can't be read back, in Rust or here (P7) |
| `HashMap<String, Vec<f64>>` (`ai`) | `map_new_str`, with keys sorted before iteration |

### 8.4 Names

- **Public structs** are `Pmn<Name>`, with `impl` blocks so that methods keep their Rust names. Constructors are
  `PmnNormal_new` and the like.
- **Free functions** are `pmn_<name>`: `pmn_mean`, `pmn_t_test_one_sample`.
- **Globals and enum members** carry `PMN_`, and private helpers `_pmn_`.
- **Checked 2026-11-15:**
  - `Pmn`, `PMN_` and `_pmn_` occur nowhere in the 6.7.18 stdlib, `hisab/dist/hisab.cyr` or any sibling `dist/*.cyr`.
  - `_pm_` and `PM_` are taken (sankhya's and nein's bundles), and so are `_prm_` and `PR_`.
  - Re-check at M1, including the names a derive would generate.

### 8.5 Memory

Probe 4 settles this, under D11:
- **Constructors** fill caller storage.
- **Owned objects** are freelist-allocated with `_drop`. Their buffers are sized at construction, never grown.
- **Scratch** comes from the work-table pool: kimiya ADR 0007's eight grow-only slots, claimed by compare-and-swap. The
  parity harness fails if a slot stays claimed. Unlike kimiya's, a slot grows on the freelist and frees the buffer it
  outgrew, and a claim past the eighth slot is a freelist block freed on release (kimiya takes both from the bump
  heap). P4 measured both paths flat.
- **P4's soaks** (port-parity §6.2) left the bump heap, the freelist and the mapped size unchanged under every
  convention above. Two costs remain: an owned buffer over 4 KB maps and unmaps on each new/drop (the freelist's open
  large-block issue), and the pool holds its buffers for the process's lifetime.
- **JSON** decoding allocates the decoded object, which the caller drops. The value tree goes into an arena allocator
  that is reset for each document (P7).
- **Never `alloc_reset`**: it would wipe the consumer's heap.
- **Every buffer parameter** is checked for null and for aliasing (the gaps kimiya's 2.0.1 review found).

---

## 9. Performance expectations

The sibling medians are 4.4× (pravash), 5.8× (tanmatra) and 7.7× (kimiya) Rust's time. The gap has four sources:
- every call is a call, with `Result` pair returns;
- `#inline` declines most helpers;
- nothing vectorizes;
- the correctly rounded primitives cost 39–138 ns against glibc's ~5–20 ns.

The trivial bodies are the worst case (kimiya: 39–69×).

The predicted bands for pramana's 34 benches are hypotheses to measure, not targets:

| Group | Benches | Expected Cyrius/Rust | Why |
|---|---|---|---|
| Transcendental per call | `normal_pdf_1000`, `normal_cdf_1000`, `student_t_pdf_1000`, `chi_squared_cdf_1000`, `poisson_cdf_lambda100_200pts`, `binomial_cdf_n200_20pts`, `mvn_pdf_3d_1000` | 6–15× | each call is 1–4 correctly rounded primitives plus a series or continued fraction; T1 adds double-double work in the incomplete functions |
| Samplers | `gamma`, `beta`, `f`, `cauchy`, `weibull` (pow at ~135 ns), `poisson_sample_lambda100`, `binomial_sample_n10000_100`, `mvn_sample_3d_1000` | 5–15× | Box–Muller is ln, sqrt and cos per pair; Marsaglia–Tsang is ln and exp per draw; Weibull is pow |
| Array loops | `stats_10000`, `moving_average_10000_w50`, `median_10001`, `mode_10000`, `linear_1000`, `ks_two_sample_1000`, `chi_squared_test_20`, `t_test_one_sample_1000` | 2–6× | Rust vectorizes the sums; the selection and sorts are comparable |
| exp-bound loops | `kde_gaussian_200pts_1000eval` (2×10⁵ exps), `mh_1d_10000`, `hmm_forward_1000`, `hmm_baum_welch_200_x5`, `naive_bayes_4class_10feat` | 5–10× | the primitive cost dominates |
| Small and integer | `combinations_30_15` (software u128), `steady_state_10`, `markov/step_1000`, `pi_100000`, `logistic_1d_43pts`, `polynomial_degree3_100pts` | 2–30× | call overhead, and u128 emulated in software |

**Expected median: 5–9×.** The levers that kept the bits in the siblings: walk pointers directly; `#inline` range
checks; hoist primitive calls out of loops; use caller buffers. A slow row is filed with its numbers, not gated.

---

## 10. Upstream requests and the conversion ledger

Each request is filed in the owning repo's `docs/development/issues/`, with a self-proving repro where one fits and
the probe numbers. Where an issue is already open, pramana adds itself as a consumer instead, with its own numbers.
Each stand-in carries `UPSTREAM(Ln)`, and `scripts/upstream_ledger.py --check` runs in CI.

**The ledger.** Seeded at M0 step 10 (2026-11-15), when every request below was filed or joined with pramana's numbers
(eight new issues and thirteen joined, with five repros). `scripts/upstream_ledger.py` reads this table, so keep its
shape:
- the status is `open`, `conditional` (no request until its milestone measures a need) or `closed`;
- each stand-in file is a backticked `src/…` path;
- each request is a backticked `<repo>:<path>` (from `/home/macro/Repos`).

| # | Status | pramana stand-in | Gap | Upstream request | Converts to | Parity rows to re-run |
|---|---|---|---|---|---|---|
| L1 | open | `src/crmath.cyr`: kimiya's correctly rounded two-phase exp, ln, pow, sin, cos, atan2 and expm1, vendored (D2); new log1p, tan and log2 to the same contract; kimiya's double-double ln core (§8.1) | ganita has no correctly rounded tier. On pramana's golden rows its counterparts misround up to 8% (exp 264 of 3,274; tan 82 of 2,010); log1p and expm1 are private; log2 has no guarantee | `ganita:docs/development/issues/2026-11-09-correctly-rounded-exp-sin-cos-hypot.md` (pramana joined as the sixth consumer, after soorat, adding log1p, tan and log2) | ganita's correctly rounded tier | every class C row, at 0 ulp against `*.cr.tsv` |
| L2 | open | `src/mathx.cyr`: `_pmn_min` / `_pmn_max` with Rust's NaN rule, fract as x − trunc(x), `_pmn_u64_to_f64`, `_pmn_sat_usize`, `is_nan` / `is_finite`; the typed std constants are generated into `src/constants.cyr` | `f64_min` / `f64_max` keep a NaN second operand (69 rows); `f64_fract` is floor-based; `f64_from` reads a u64 as signed; no saturating casts (port-parity §1) | `cyrius:docs/development/issues/2026-11-11-f64-min-max-drop-nan-only-as-first-operand.md` (pramana's section adds a trunc-based fract and an unsigned conversion, with the repro `2026-11-15-f64-fract-floor-and-f64-from-signed.cyr`) | the stdlib's `f64_*` | the math sweep's class A rows: fract, min, max, clamp, `as usize`, `u64 as f64` |
| L3 | open | `src/sort.cyr`: a stable merge sort and the order statistics' selection on raw f64 arrays, NaN refused first (ADR 007 row 14) | no stable sort; `vec_select_nth`, `vec_get` and `vec_set` abort; both work on `vec`, not on a caller's `(ptr, n)` | `cyrius:docs/development/issues/2026-11-11-no-stable-sort.md`; `cyrius:docs/development/issues/2026-11-11-vec-keeps-outgrown-buffer-and-aborts.md` (pramana's sections added to both) | a stable sort over raw words in the stdlib | the order statistics' rows (class A on the stable rule) |
| L4 | open | `src/linalg.cyr`: hisab 1.4.0's `cholesky` and `cholesky_solve`, transcribed, with caller scratch | `ganita_mat_cholesky_solve` leaves 8n bytes per call; ganita's Cholesky sums in another order (P5: up to 40 ulp from 1.4.0) | `ganita:docs/development/issues/2026-11-15-mat-cholesky-solve-allocates-and-sums-in-another-order.md` (with a repro) | ganita's `_into` solve, and its factor if it offers the term-by-term order | the MVN, ARIMA and logistic-regression rows |
| L5 | open | `src/linalg.cyr`: hisab 1.4.0's `least_squares_poly` (modified Gram–Schmidt), transcribed | hisab 3.5 has no polynomial least squares; ganita's Householder fit is up to 13,619 ulp from 1.4.0 and leaves 1.3–2.3 KB per call | `hisab:docs/development/issues/2026-11-15-no-polynomial-least-squares.md` | hisab's polynomial fit | the polynomial-regression rows |
| L6 | open | `src/linalg.cyr`: hisab 1.4.0's cyclic Jacobi `eigen_symmetric(tol, max_iter)`, transcribed | hisab 3.5's `eigen_symmetric` takes no tolerance or budget, leaves ≈ 16n² bytes per call, and differs from 1.4.0 by up to 14 ulp | `hisab:docs/development/issues/2026-11-15-eigen-symmetric-no-tolerance-budget-or-caller-scratch.md` | hisab's `eigen_symmetric` with a tolerance and caller scratch | the PCA rows (class D) |
| L7 | open | `src/json.cyr`: hand codecs for the 38 types over bayan's tree | `#derive` has no validation hook (20 types validate on deserialize), no `Vec<Vec<f64>>` (15 fields) and no u64 | `cyrius:docs/development/issues/2026-11-11-derive-json-no-vector-array-or-tagged-variant-shapes.md` (pramana's section adds the hook, matrices and u64) | `#derive(Serialize)` / `#derive(Deserialize)` | the JSON rows (class A) |
| L8 | open | `src/special.cyr`: the T1 special functions (CORE-MATH's erf and erfc, `normal_cdf`, `ln_gamma`, `gamma_inc`, `beta_inc`) | in neither ganita's scope nor its out-of-scope list (§1.3) | `ganita:docs/development/issues/2026-11-15-special-functions-scope-request.md` (a scope request) | ganita's special functions, if it takes them | every `*.ref.tsv` row |
| L9 | open | `src/ai.cyr`: a synchronous tool-calling client over sandhi and bayan, in the `[lib.ai]` profile (D8) | hoosh 2.x is a binary by decision, with no client library; agnosai and selah each carry their own | `hoosh:docs/development/issues/2026-11-15-pramana-no-client-bundle-for-tool-calling.md` | a hoosh client bundle | the `ai` tests' recorded responses |
| L10 | open | `src/fmtf.cyr`: impetus's digit core with its serde_json form and pravash's Debug form, and pravash's `debug(sb, alternate)` builders (D5, D12) | no `#derive(Debug)` and no shortest printer in the stdlib; bayan's Grisu2 differs from serde_json's text on 9,714 of 280,562 doubles | `cyrius:docs/development/issues/2026-11-13-derive-debug-and-shortest-float-digits.md`; `bayan:docs/development/issues/2026-11-11-no-serde-compatible-shortest-f64-printer.md` (pramana's numbers added to both) | the stdlib printer, `bayan_f64_to_json_serde`, `#derive(Debug)` | the JSON text rows and the `Debug` rows (class A) |
| L11 | open | `src/json.cyr`'s pre-pass (P7) | the tree refuses an integer at or above 2⁶³, reads `-0` as +0 and accepts a raw control byte in a string: 135 of 1,495 serde_json documents read differently | `bayan:docs/development/issues/2026-11-09-json-integer-above-i64-max-refuses-the-document.md`; `bayan:docs/development/issues/2026-11-11-json-integer-negative-zero-loses-its-sign.md`; `bayan:docs/development/issues/2026-11-11-json-strings-accept-raw-control-bytes-and-invalid-utf8.md` (pramana's numbers added to each) | bayan's u64 node, signed zero and strict strings | the JSON decode rows (class A) |
| L12 | open | `src/crmath.cyr`'s exact fma: Dekker's product, a round-to-odd sum, and the tiny-path rounding (§5 step 9e) | no scalar fma (§1.3). The vector `f64v_fmadd` rounds twice on x86_64 and once on aarch64. The emulation costs 6.5–20× CORE-MATH's own in C | `cyrius:docs/development/issues/2026-11-15-no-scalar-fma.md` (with a repro) | `f64_fma`: hardware where the target has it, exact software elsewhere, the same bits everywhere | the erf, erfc and `normal_cdf` rows; their bits must not change |

**Also filed at step 10, with no stand-in** (a rule in §8.3 covers each meanwhile):
- **cyrius:** a closure's captured f64 is an untyped word in arithmetic, though it binds `T = f64` as a generic
  argument (P3), `cyrius:docs/development/issues/2026-11-15-closure-captured-f64-is-untyped-in-arithmetic.md`, with a
  repro.
- **cyrius:** `cyrius lint` compiles a file without the manifest's stdlib, so it refuses a capturing closure that
  builds (P3), `cyrius:docs/development/issues/2026-11-15-lint-refuses-a-capturing-closure-the-build-accepts.md`, with
  a repro.
- **cyrius:** pramana's numbers added to the open freelist large-block issue: 36 µs per MVN new and drop at d = 32,
  and an mmap per MH result (P4), `cyrius:docs/development/issues/2026-11-11-freelist-maps-every-block-over-4k.md`.
- **bayan:** pramana's numbers added to impetus's open request for an opt-in that leaves unread members unchecked,
  with the two other cases P7 found there, nesting past 128 and a lone surrogate. 35 of 1,495 serde_json documents
  wait on it (ADR 007 row 16):
  `bayan:docs/development/issues/2026-11-09-json-duplicate-key-has-no-opt-in-for-ignored-members.md` and
  `bayan:docs/development/issues/2026-11-15-json-nesting-cap-of-128-refuses-what-serde-skips.md`.

---

## 11. Risks

1. **T1 scope creep.** kimiya's mid-port overflow audit cost 1.7 days and about 30 ADR rows, and is still unfinished.
   M2 part b is bounded to the five functions of §3.4, at the bounds and domains fixed there. Everything else goes to
   D4 or 2.1. Because the bounds are fixed, a miss costs time, not accuracy: more double-double evaluation where a
   prototype falls short. P9 exists to find those places before M2 part b starts.
   The CORE-MATH erf/erfc port (D3 = C) was the largest single item. It verified inside its fixed slot in P9
   (§5 step 9e), so the ≤ 1 ulp fallback is not needed.
2. **The recording toolchain moving.** pravash's `--check` broke when stable moved to 1.100, through a generated
   `Debug` escape table (Unicode 18), not float math. Pin `rust-old/rust-toolchain.toml` to `1.100.0` exactly, record
   rustc in every header, and gate step 5 on two identical builds.
3. **Open miscompiles.** `s.f = g(t.e)` stores the argument, and a small struct in a `Result` is misread. The
   out-pointer conventions avoid both, and the parity rows catch the rest (pravash's merge rows found the first one).
4. **bayan derive defects.** The hand codecs (D5) never use the derive's scanner.
5. **Cross-target runs are local** (wine, qemu). Run all three every milestone.
6. **The Rust CI job gating the release.** Remove it at M8e, before the tag.
7. **`ai` cannot be golden-tested against a live model.** Use a recorded-response transport, compare tool dispatch
   class A, and treat the prompt text as n/a.

---

## 12. After 2.0.0

- The D4 candidates, with the M4/M5 StRD numbers: compensated sums, a centered/scaled Householder polynomial fit,
  and T1 for `z_quantile` / `t_quantile` if M4 measured a gap.
- Convert stand-ins as their libraries ship (§10), re-running their parity rows.
- **The 1.3.x deprecation notice on crates.io.** bodh, jivanu and sangha stay on `pramana = "1"` until they port.
- **The zugot recipe.** `marketplace/pramana.cyml` names the Rust 1.3.0 release. kimiya's recipe still names Rust
  1.1.1, so the siblings set no precedent; switching to the Cyrius 2.0.0 build is the maintainer's call.
- sakshi spans, if a consumer asks (§4.2).

## 13. Tracking

- `docs/development/state.md`: version, milestone, counts, upstream issues, next step.
- `docs/development/port-parity.md`: one section per milestone part, with its result line and per-target histograms.
- `docs/development/rust-test-ledger.md` (generated, `--check` in CI).
- `docs/benchmarks-rust-v-cyrius.md`: per milestone, both columns.
- This plan's milestone table, with ✅ / ⏳ per step as each one lands, as pravash's plan.
