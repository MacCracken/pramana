# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

- **Cyrius port plan**: `docs/development/pramana-port-plan.md`, the plan of record for porting 1.3.0 to Cyrius 6.7.18 as 2.0.0, drawn from the tanmatra, kimiya and pravash ports and a review of ganita. Every decision was taken on 2026-11-15, among them D3: correctly rounded erf/erfc, and fixed accuracy bounds for `normal_cdf` (≤ 1 ulp), `ln_gamma` (≤ 2), `gamma_inc` (≤ 4) and `beta_inc` (≤ 8)
- **Accuracy baseline tool**: `scripts/port-baseline/` measures 1.3.0's special functions and the glibc primitives against MPFR 4.2.2 (and exact rationals for the integer-shape incomplete beta). Max error in ulps:
  - glibc primitives: 0.75 or less;
  - `erf` 0.82, `erfc` 2.49;
  - `normal_cdf` tails 1,590;
  - `ln_gamma` 1.2×10⁶ on [0.5, 10], and 4.3×10¹³ beside its roots at 1 and 2;
  - `gamma_inc` 246;
  - `beta_inc` 583.

  The tool is not part of the crate.
- **`oracle` feature** (hidden, read-only, never published): hooks that expose the private special functions, samplers, quantiles, HMM forward pass, KDE kernels and `MultivariateNormal` factorization to the port's golden-value generator. They live in child modules, so each audited source gains only a `mod oracle;` declaration. `tests/oracle.rs` (11 tests) proves each hook is the public code path, bit for bit, and draws the same uniforms; it is mutation-checked six ways. Without the feature, nothing changes: every per-feature test count equals 1.3.0's

- **The semantics probe** (port plan M0 step 4; `scripts/probes/`, results in `docs/development/port-parity.md` §1).
  - **What it compares:** every f64 and u64 operation 1.3.0 uses, bit for bit against a release-built Rust twin on
    rustc 1.100.0 and glibc 2.44: 125 cases in 155 rows.
  - **Exact as they are:** 136 rows, plus 4 identical NaNs. Among them are `powi` in LLVM's order, the xorshift RNG,
    the `total_cmp` keys and every positional exponent literal.
  - **Needs a helper:**
    - `f64_min` / `f64_max` with a NaN second operand;
    - `fract`, because the stdlib's `f64_fract` is floor-based;
    - `u64 → f64` at or above 2⁶³, because `f64_from` is signed;
    - saturating `as usize`.
  - **A rule:** f64 sums start at −0.0, as Rust's do.
  - **Transcendentals:** 5 of 47 rows differ by 1 ulp. MPFR shows glibc misrounds three of them, and the stdlib and
    ganita two.

- **The golden-value generator** (port plan M0 step 5; `scripts/golden/`, adapted from pravash's).
  - **The corpus:** `tests/golden/math.tsv`, 35,597 rows in 31 functions. Every std f64 operation 1.3.0 calls, plus
    the casts and word operations the probe flagged, recorded on rustc 1.100.0 and glibc 2.44.
  - **The independent re-check:** `validate.py` re-checks every row without the oracle. It uses Python for the exact
    rows, glibc through ctypes for the transcendentals, and MPFR to prove every `cbrt` row correctly rounded.
  - **The correctly rounded rows:** `tests/golden/math.cr.tsv`, the 76 rows where glibc misrounds, each by 1 ulp.
    `exp_m1` misrounds on 52 of 1,010 (5.1%).
  - **Guarantees:** the output is deterministic; a run under another glibc or rustc is refused; tampering is caught.

- **`src/constants.cyr`** (port plan M0 step 6), generated from the oracle's 112 constants: the fdlibm, Lanczos,
  Acklam and KS tables, the thresholds and tolerances, and the Rust std float constants.
  - **Proven bit for bit** by recomputing `erf`, `erfc`, `ln_gamma`, `z_quantile` and `kolmogorov_sf` with the
    extracted values (3,000–5,000 cases each), and the sampler thresholds by their draws.
  - **Cross-checked** by a second, Python extractor and by compiling every value back
    (`scripts/golden/probe_tables.py`).
  - **Mutation-checked:** 11 of 15 one-ulp nudges are caught. Two are invisible in every output, and two are
    extraction-only by design.

- **The parity harness** (port plan M0 step 7): `tests/parity.tcyr` replays the golden corpus against the port.
  - **The parts:** it is built on pravash's `parity_core.cyr` and `ulp.cyr`, adapted to ADR 009's correctly rounded
    rows and ADR 007's mapped rows, with two print-length bugs fixed. A generated lookup (`tests/constants_rows.cyr`)
    serves the constants rows.
  - **First run:** 2 modules, 0 failing. 3,963 rows ok (every constant, sqrt, abs, floor, the fdlibm word
    operations), 24,669 transcendental rows measured, and 7,077 pending for M1's helpers. Warning-free, and the same
    under `--bounds`.
  - **Mutation-checked** four ways. It runs in `cyrius test` and `cyrius audit`.

- **Cross-target baselines** (port plan M0 step 8). The parity harness prints a digest per function.
  - **Identical on all three targets:** every dispatched function gives identical bits on x86_64 ELF, x86_64 PE
    (wine) and aarch64 (qemu). The native x87 builtins differ on aarch64, where they equal the polyfills, which
    confirms the polyfill choice.
  - **The bounds:** class-C bounds are set at K = 2 for the interim primitives and 0 for `cbrt`; `powf` stays
    measured, because of ganita's open `pow(−1, huge)` defect.
  - **Gated result:** 0 failing, 24,162 ok, on all three targets.
  - **Mutation-checked** at the bounds' edges.

- **Probes, part 9a** (port plan M0 step 9; `scripts/probes/`, Rust twins in `scripts/probes/rust/`).
  - **P1, integers:** 3,736 cases are bit-identical to the oracle: 10⁶ RNG draws from each of five seeds, and a
    two-word u128 prototype under `factorial`, `permutations` and `combinations` across the u64 range.
  - **P5, linear algebra:** ganita's Cholesky, solve, eigensolver and least squares, and hisab 3.5's eigensolver,
    differ from hisab 1.4.0 by up to 40, 18, 14 and 13,619 ulp, and each call leaks. All four will be transcribed.
  - **P6, sort and select:** Rust's unstable selection leaks the sign of a zero into `median` and `percentile` on
    9,393 of 180,000 zero-heavy inputs. Decided: the port selects from a stable sort, and the rows where Rust's zero
    differs are mapped (ADR 007 row 14).

- **Probes, part 9b** (port plan M0 step 9; `scripts/probes/p{2,3,4}_*.bcyr`, run by `cyrius bench`; results in
  `docs/development/port-parity.md` §6.2). Pinned, three interleaved runs, x86_64.
  - **P2, dispatch:** a direct call, a static generic and `callptr(&f, …)` cost about the same. A `dyn` call costs
    about 3 ns more: 15 against 12 ns per RNG draw, and 64 against 57 ns per pdf with exp. So D7 was amended: the RNG
    is generic over `R: PmnRng`, as Rust's `impl Rng`, with `dyn` only for Gibbs' conditionals (ADR 007 row 6).
  - **P3, callbacks:** a function pointer plus context and closures (with or without captures) are within 1–3 ns of a
    direct call. Every form gives the same bits, and calls allocate nothing. Found:
    - a captured f64 is an untyped word in arithmetic, so `k * x` computes 0 for 3.0 × 0.25 (with a warning);
    - `cyrius lint` refuses a capturing closure unless the file includes `lib/alloc.cyr`;
    - a 4-parameter capturing closure is a compile error on Windows. D7's function pointer plus context needs no
      capture.
  - **P4, the flat heap:** D11's conventions prototyped, among them a work-table pool that grows on the freelist and
    frees what it outgrew. Every soak leaves the bump heap, the freelist and the mapped size unchanged: 10⁶ pdf/cdf
    calls, 10⁵ MVN pdfs, 10⁶ MH samples, 10³ Baum–Welch iterations, 10 medians of 10⁶, and pool overflow.

- **Probes, part 9c** (port plan M0 step 9; `scripts/probes/p7_json.cyr` with `p7_compare.py` and the Rust twin
  `rust/src/bin/p7_json.rs`, and `p8_bundle.sh` with `p8_bundle/`; results in `docs/development/port-parity.md`
  §6.3).
  - **P7, JSON:** serde_json 1.0.154 wrote 1,495 documents for nine types covering every serialized shape.
    - **bayan's value tree alone** reads 1,325 as serde_json does. It differs on integers at or above 2^63 (13 of 35
      saved RNG states are refused), on `-0` and on raw control bytes in strings.
    - **A stand-in pre-pass** brings agreement to 1,460, with every value read bit-exact. The 35 left are documents
      serde_json accepts only because it skips a member unchecked.
    - **Text:** impetus's `fmtf.cyr` writes serde_json's text on all 596 documents both sides read. bayan's Grisu2
      matches on 566, and differs on 9,714 of 280,562 doubles.
    - **Decided:** D5 is amended to write serde_json's text through `fmtf.cyr` (ADR 007 row 7), and ADR 007 row 16
      accepts the 35.
  - **P8, the bundle:** a one-function consumer is the size of an empty one (411,928 bytes on x86_64), and a core
    consumer never compiles sandhi. `ai` adds 839 KB. Listing sandhi in `[deps] stdlib` would make every pramana
    compile 0.8 s slower (build 1.30 s against 0.49 s). So D8 is a `[lib.ai]` profile with sandhi kept out of
    `[deps]`, and the `ai` test includes it itself. On PE and aarch64, dead code is NOPed, not compacted.

- **Probes, part 9d-1: P9's `ln_gamma` prototype** (port plan M0 step 9; `scripts/probes/p9_lgamma*.{cyr,bcyr,sh}`,
  with fdlibm's coefficients generated by `p9_fdlibm_consts.py`; results in `docs/development/port-parity.md` §6.4).
  - **The design:** fdlibm's `e_lgamma_r` layout on correctly rounded ln, with double-double logarithms and products,
    an exact sin(πx) reduction for x < 0, and one rounding.
  - **Accuracy, against MPFR 4.2.2 at 1,024 bits** on 1,675,000 rows:
    - **x > 0:** at most 1.31 ulp, mean 0.26, against ADR 009's fixed ≤ 2. Rust 1.3.0's Lanczos form reaches
      1.2×10⁶ ulp on (0.5, 10) and 4.3×10¹³ beside 1 and 2;
    - **x < 0:** at most 1.40 ulp of max(|lnΓ|, 1), against ≤ 2. Rust reaches 1,583.
  - **Targets:** the same bits on x86_64, PE and aarch64.
  - **Cost:** 72–192 ns a call for x > 0, against 168 ns for Rust's form on correctly rounded primitives.
  - **Found:** Rust 1.3.0's `ln_gamma` returns +∞ for 0 < x < about 3.2e-301, where lnΓ ≈ 690.
  - `scripts/port-baseline/check.c` measures a name ending `_mixed` in ulps of max(|value|, 1).

- **Probes, part 9d-2: P9's `gamma_inc` prototype** (port plan M0 step 9; `scripts/probes/p9_gamma*`, and the ζ(k)
  table generated by `p9_zeta_table.c`; results in `docs/development/port-parity.md` §6.5).
  - **The design:**
    - a double-double prefactor exponent: lnΓ from a ζ series below a = 10, and an exact u − ln(1 + u) above;
    - the series and the continued fraction in double-double;
    - the split at x = a, with a direct small-shape Q instead of 1 − P.
  - **Accuracy:** checked against a 320-bit MPFR series and continued fraction (`check.c`'s `gref`, which agrees with
    `mpfr_gamma_inc` to 6×10⁻⁷⁵ ulp), on 360,400 rows over a ∈ [10⁻³, 10⁵]. T1 is within 1.0 ulp, mean ≤ 0.25,
    against ADR 009's fixed ≤ 4.
  - **Rust 1.3.0 is worse than §3.2 recorded:**
    - on its own 20,000 pairs it reaches 97 ulp on P and 485 on Q;
    - in the deep lower tail it reaches 2.9×10⁹ on P, from u = (x − a)/a ≈ −1;
    - for small shapes it reaches 2.8×10⁴ on Q.
  - **Targets:** the same bits on x86_64, PE and aarch64.
  - **Cost:** 2.5–64 µs a call, 10–17× Rust's form on correctly rounded primitives. The same prototype with Rust's
    plain-double loops misses the bound (up to 79 ulp), so the cost belongs to the loops. M2 part b measures cheaper
    algorithms for the two slow regions.

- **Probes, part 9d-3: P9's `beta_inc` prototype** (port plan M0 step 9; `scripts/probes/p9_beta*`, with
  `p9_beta_exact.py`; results in `docs/development/port-parity.md` §6.6).
  - **The design:**
    - the smaller of x and y taken as exact;
    - a double-double prefactor, with lnΓ a double-double at every argument;
    - the continued fraction in double-double;
    - TOMS 708's power series for a complement below 2⁻⁶.
  - **Accuracy:** checked against a 320-bit continued fraction (`check.c`'s `bref`, which agrees with exact rationals
    to 4×10⁻⁷⁸ ulp), on 214,744 rows over a, b ∈ [10⁻³, 10⁵]. T1 is within 0.51 ulp, mean ≤ 0.25, against ADR 009's
    fixed ≤ 8.
  - **Rust 1.3.0** reaches 550 / 583 ulp on §3.2's integer shapes, and 1.7×10⁵ near the mean with one large shape.
  - **Defects reworked in T1 on the way:**
    - a prefactor zeroed near the subnormal range;
    - a complement by the far side's slow continued fraction (3.6×10¹⁰ ulp);
    - Stirling's correction in plain double on a + b's high part (5.3 ulp).
  - **Targets:** the same bits on x86_64, PE and aarch64.
  - **Cost:** 7.5–71 µs a call, 10–17× Rust's form on correctly rounded primitives.

- **Probes, part 9e: correctly rounded erf and erfc, CORE-MATH's without FMA** (port plan M0 step 9;
  `scripts/probes/p9_erf*`, `p9_coremath_tables.py` and the C helpers in `p9_erf_c/`; results in
  `docs/development/port-parity.md` §6.7).
  - **The port:** CORE-MATH's binary64 `erf.c` and `erfc.c` (MIT; fetched, never committed), function for function,
    with the 17 tables (3,929 words) generated from the sources and their notice kept.
  - **No FMA:** of the 59 fma sites, 2 are Dekker products and 57 run as an exact fma (Dekker's product, then
    Boldo and Melquiond's round-to-odd three-term sum). The one product that can be subnormal, at the end of erf's
    tiny path, is rounded once on the subnormal grid instead.
  - **Stage 1, in C:** the emulation matches hardware fma on 10⁸ triples, and CORE-MATH built with it matches its own
    hardware-FMA build on all 369,190 worst cases (both signs) and 4×10⁷ arguments.
  - **Accuracy:** the Cyrius port gives CORE-MATH's own bits on 2,568,534 rows, and MPFR 4.2.2 at 1,024 bits finds
    every one of the 2,768,518 non-NaN rows correctly rounded, 100,000 through each accurate path alone included.
    Rust 1.3.0 is at 0.82 ulp on erf and 2.49 on erfc (§3.2).
  - **Targets:** the same bits on x86_64, PE and aarch64.
  - **Cost:** 508–675 ns a call on the main ranges and 2.3 µs on erfc's asymptotic range, 18–31× glibc. Most of it
    is the emulated fma (6.5–20× CORE-MATH's cost in C), so a scalar fma is requested from cyrius (ledger L12).
  - **D3 = C holds:** the port verified inside its slot, so the ≤ 1 ulp fallback is not needed.

- **Probes, part 9d-4: P9's `normal_cdf` prototype** (port plan M0 step 9; `scripts/probes/p9_ncdf*`; results in
  `docs/development/port-parity.md` §6.8).
  - **The design:** −z/√2 as a double-double h + l. The first-order correction for l is applied to CORE-MATH's
    double-double erfc(h) before it rounds, and the ½ is folded into that one rounding, exact on the subnormal grid
    too. The double-double comes from CORE-MATH's fast path where its proven bound is under 2⁻⁶⁰ relative, and from
    its accurate paths otherwise. That makes the error provably at most 0.5 + 2⁻⁷ ulp.
  - **Accuracy:** checked against MPFR 4.2.2 at 1,024 bits on 2,506,918 rows, including §3.2's sets, the deep tail's
    subnormal results, the decimal grid z = k/1000 and `cr_erfc`'s branch points. T1 is within 0.5 ulp on all
    1,246,809 of its rows, correctly rounded on all but 4, against ADR 009's fixed ≤ 1. `normal_sf(z)` is
    `normal_cdf(−z)` bit for bit, so it is covered too.
  - **What the steps buy:**
    - Rust 1.3.0 reaches 1,590 ulp on §3.2's rows and 1,688 in the deep tail;
    - the correctly rounded erfc on Rust's rounded argument reaches the same 1,688: the argument's rounding is the
      error;
    - the plan's sketch, which rounds twice, reaches exactly 1.0 ulp and misrounds up to 25% of a set. The plan's
      §3.4 now gives T1's form.
  - **Targets:** the same bits on x86_64, PE and aarch64.
  - **Cost:** 1.3–4.6 µs a call against Rust's form's 0.15–0.24 µs on correctly rounded exp (8–20×). Most of the
    core's share is CORE-MATH's accurate path for z in (−4.07, −2.96). A looser fast bound of 2⁻⁵⁶ keeps the
    error under 0.625 ulp and takes 30% off the core; M2 part b chooses.
  - **`check.c`** counts the zero and subnormal results of probe P9's erf, erfc and `normal_cdf` rows in units of
    2⁻¹⁰⁷⁴ (ADR 009). `scripts/port-baseline/merge.py` merges its reports from interleaved chunks, so a slow MPFR check
    runs on ten cores.

- **Upstream requests and the conversion ledger** (port plan M0 step 10, §10; ADR 008).
  - **Filed in the owning repos' `docs/development/issues/`**, with pramana's probe numbers:
    - **cyrius:** a captured f64 is an untyped word in closure arithmetic; `cyrius lint` refuses a capturing closure
      the build accepts; no scalar fma (L12; the vector `f64v_fmadd` rounds twice on x86_64 and once on aarch64);
    - **ganita:** `cholesky_solve` allocates 8n bytes per call, and the factor sums in another order than hisab 1.4.0
      (L4); the special functions' scope (L8);
    - **hisab:** no polynomial least squares (L5); `eigen_symmetric` has no tolerance, budget or caller scratch (L6);
    - **hoosh:** a client bundle for tool calling, which agnosai and selah each carry their own of (L9).
  - **Joined, as a consumer with its own numbers:**
    - **cyrius:** the f64 surface, adding a trunc-based fract and an unsigned u64 → f64 (L2); stable sort and `vec`'s
      aborts (L3); the derive's shapes, adding a validation hook, matrices and u64 (L7); `Debug` and the shortest
      printer (L10); the freelist's large blocks;
    - **ganita:** the correctly rounded tier, as its sixth consumer, adding log1p, tan and log2, with each counterpart's
      misrounds on the golden corpus (exp 264 of 3,274, tan 82 of 2,010; L1);
    - **bayan:** the u64, `-0` and raw-control-byte issues (L11), the serde_json printer (L10), and the unread-member
      opt-in and nesting cap (ADR 007 row 16).
  - **Repros:** five (four in cyrius, one in ganita), each run from its repo's root on cyrius 6.7.18 and showing its
    gap.
  - **The ledger:** §10's table in pravash's shape (status, stand-in, gap, request, conversion, rows to re-run), read
    by `scripts/upstream_ledger.py --check` (pravash's, extended to check the requests filed with no stand-in), now a
    CI step. Its first run caught `src/constants.cyr`'s typed std constants unmarked; the generator now writes their
    `UPSTREAM(L2)` note, and the rest of its output is unchanged. 12 rows, 1 note, 0 failures.
  - **The M0 gate passes:** build, `test` (warning-free), `test --bounds`, `bench`, `fmt --check`, `lint`, `vet`,
    `deps --locked`, `audit` and the ledger.

- **M1 part a: the error codes** (`src/error.cyr`; port plan D6, ADR 007 row 1). `PMN_ERR_*`: the five Rust
  variants in their order, then the Cyrius-only `OUT_OF_MEMORY` and `DESERIALIZE`, append-only. `pmn_err_name` and
  `pmn_err_message` give the variant's name and thiserror's message up to its field. `tests/error.tcyr` pins them (30
  checks). `[lib]` starts, so `dist/pramana.cyr` is written, and `distlib --check` joins CI.
- **M1 part b1: the correctly rounded primitives** (`src/crmath_data.cyr`, `src/crmath.cyr`; port plan D2, ADR 009;
  ledger L1, L12; results in `docs/development/port-parity.md` §7.2).
  - **Vendored** from kimiya 2.0.1's tag by `scripts/vendor_crmath.py` (after soorat's): exp, expm1, ln, pow, sin, cos
    and atan2, with the double-double ln and exp cores and the exact two-sum, Dekker product and round-to-odd sum.
    That is 58 functions and 59 tables, renamed `_pmn_km_`. `--check` runs in CI.
  - **The golden rows:** exp, ln, exp_m1, powf, sin, cos and atan (atan2(x, 1)) are at 0 ulp on all 18,326 of their
    rows, with their bounds at 0. The 76 MPFR-corrected rows are all accepted, and ganita's `pow(-1, DBL_MAX)` = +∞
    outlier is gone. The same digests on x86_64, PE and aarch64.
  - **Against MPFR:** correctly rounded on all 7.7M seeded rows (`scripts/crverify/`, about 1.1M per function),
    overflow, subnormal and NaN results included. `check.c` gained `atan`, `log2` and the `cr:` rows, which count
    those cases instead of skipping them.
  - **Cost:** 46–136 ns a call, 8.0–14.1× glibc's (`tests/crmath.bcyr` against the Rust twin
    `scripts/probes/rust/src/bin/m1_primitives.rs`; `docs/benchmarks-rust-v-cyrius.md`, new).
- **M1 part b2: a correctly rounded log1p** (`src/crmath_ext.cyr`, `_pmn_cr_log1p`; port plan D2; ledger L1; results
  in `docs/development/port-parity.md` §7.3). No sibling has one, so it is new, on kimiya's ln:
  - **|x| < 2⁻¹⁰:** ln's own core on its c = 1 interval, with r = x exactly;
  - **otherwise:** ln s plus log1p(τ) on the exact split 1 + x = s + t, with τ = t/s;
  - **the accurate path:** both in triple-double, rounded once;
  - **accuracy:** `ln_1p` is at 0 ulp on all 1,013 golden rows, including the 6 where glibc misrounds. It is correctly
    rounded on 1,300,000 MPFR rows on three targets, 200,000 of them through the accurate path alone;
  - **fixed on the way:** the accurate path's Dekker product overflowed past 2⁹⁹⁶ (NaN, with its sign differing
    between x86 and ARM). It now scales s and t by 2⁻⁵¹² first;
  - **cost:** 51–96 ns a call, 9.4–13.9× glibc.
- **M1 part b3: a correctly rounded tan** (`src/crmath_ext.cyr`, `_pmn_cr_tan`; port plan D2; ledger L1; results in
  `docs/development/port-parity.md` §7.4). New, on kimiya's sin and cos:
  - **the fast path:** one reduction, the fast evaluations of sin|x| and cos|x|, each renormalised exactly, and a
    double-double quotient with a bound composed from theirs;
  - **the accurate path:** both in triple-double, a triple-double division, rounded once;
  - **accuracy:** `tan` is at 0 ulp on all 2,010 golden rows, including the 7 where glibc misrounds. It is correctly
    rounded on 1,300,000 MPFR rows on three targets, 200,000 of them through the accurate path alone, including
    ±4 ulps of k·π/2 and Payne–Hanek arguments;
  - **fixed on the way:** the first version divided kimiya's unnormalised fast pair as if normalised. Its bound was
    then wrong, and the golden rows caught 1,803 misrounded rows before MPFR ran;
  - **cost:** 188–190 ns a call, 17.4× glibc, about one sin plus one cos.
- **M1 part b4: a correctly rounded log2** (`src/crmath_ext.cyr`, `_pmn_cr_log2`; port plan D2; ledger L1; results in
  `docs/development/port-parity.md` §7.5). New, on kimiya's ln, with the exponent kept exact (log2 x = E + ln(m)/ln 2):
  - **exact cases:** the powers of two give E exactly, and they are the only ones;
  - **the constant:** 1/ln 2 as three bit patterns, checked against 90-digit decimal arithmetic by
    `scripts/crverify/consts.py`, in CI;
  - **accuracy:** `log2` is at 0 ulp on all 2,510 golden rows. It is correctly rounded on 1,318,874 MPFR rows on three
    targets: every power of two and its neighbours, and 200,000 through the accurate path alone;
  - **cost:** 104–109 ns a call, 20.6× glibc.
- **The primitives are complete** (M1 part b): every transcendental Rust 1.3.0 calls is correctly rounded, and every
  class-C math row of the golden corpus is gated at 0 ulp (28,556 rows ok, nothing measured), identically on x86_64,
  PE and aarch64. The ten functions are correctly rounded on 11.6M MPFR rows, at 8–21× glibc's cost.
- **M1 part c: the Rust f64 surface** (`src/mathx.cyr`; port plan §8.1; ledger L2; results in
  `docs/development/port-parity.md` §7.6). Every std f64 operation Rust 1.3.0 calls, with Rust's semantics:
  - **NaN-ignoring** min, max and clamp;
  - **fract** as x − trunc(x);
  - **conversions:** an unsigned `u64 as f64` and a saturating `as usize`;
  - **predicates and order:** is_nan, is_finite and total_cmp;
  - **powi** by square-and-multiply in LLVM's order;
  - **the transcendentals' names**, over the correctly rounded functions.

  The 7,077 golden rows that waited on them all pass (the powi rows at 0 ulp), so the whole corpus is gated: 35,633
  rows ok and 76 accepted, with nothing failed, measured or pending, on x86_64, PE and aarch64. `tests/mathx.tcyr`
  adds 30 checks; `tests/mathx.bcyr` and its Rust twin record the costs.
- **M1 part d1: serde_json's f64 text** (`src/fmtf.cyr`; port plan D5, ADR 007 row 7; ledger L10; results in
  `docs/development/port-parity.md` §7.7):
  - **the printer:** impetus 2.0.0's serde_json form, vendored from its tag by `scripts/vendor_fmtf.py` (15 items;
    `--check` in CI). It gives the shortest round-trip digits, ties to even, in serde_json's form;
  - **a new golden module, `fmt.tsv`:** serde_json's own text for 25,200 doubles, from the generator, which now
    depends on serde_json 1.0.154 as `rust-old/` does. `validate.py` rebuilds every row from Python's `repr`;
  - **the result:** all 25,200 rows match byte for byte on x86_64, PE and aarch64, and the replay is three modules,
    60,833 rows ok. Switching the printer to Rust's round-up tie rule fails 37 rows, so the 62 exact ties in the
    inputs are exercised;
  - **cost:** 2.8–13.9 µs a value, 135–650× serde_json's (`tests/fmtf.bcyr` and its Rust twin). The exact bignum
    search is slow, and no hot path writes JSON; the printer that L10 asks for upstream would replace it.
- **M1 part d2: the work-table pool** (`src/pool.cyr`; port plan D11 and §8.5, kimiya ADR 0007; results in
  `docs/development/port-parity.md` §7.8). It was moved forward from part f, because the JSON reader's value tree
  needs it:
  - **the pool:** eight slots claimed and released by compare-and-swap. A slot grows on the freelist and frees what it
    outgrew; a claim past the eighth slot is a freelist block of its own, freed on release;
  - **a bump allocator over a claim**, a stdlib Allocator for bayan's `_a` calls, kept in the caller's storage;
  - **the harness:** the parity replay now fails a module that leaves a slot claimed (mutation-checked);
  - **tests:** `tests/pool.tcyr` (21 checks) and `tests/pool_threads.tcyr`, in which four threads race twelve claims
    for eight slots and read back 15.8M tagged words, on x86_64, PE and aarch64. `thread` joins the test-only stdlib
    leaves;
  - **fixed before it shipped:** the threaded test's first run found a growth race. A slot won in the claim's second
    scan doubled even when it already fitted, so slots reached 64 GB and 1,323 claims failed. kimiya's pool has the
    check; P4's prototype had lost it. No slot now exceeds twice the largest claim;
  - **cost:** a claim and its release take 18–19 ns, where Rust allocates and drops a 256-byte `Vec` in 8.5–8.7 ns
    (`tests/pool.bcyr` and its Rust twin).

- **M1 part d3a: the JSON read core for values** (`src/json.cyr`; port plan D5 and §8.3; ledger L7, L11 and L13;
  results in `docs/development/port-parity.md` §7.9):
  - **the pre-pass**, probe P7's stand-in for bayan's three gaps (integers at or above 2⁶³, `-0`, raw control bytes in
    strings);
  - **the tree** in a bump allocator over a pool claim. The claim starts at 24 bytes per byte of text and doubles while
    the allocator refuses, since a tree can reach 88 times its text. `_pmn_json_decode` closes the document on every
    path;
  - **the readers** for f64, u64 and usize, bool, String (UTF-8 checked when read, as serde_json does), Vec<f64>,
    Vec<u64>, Vec<String> and Vec<Vec<f64>> (with its shape, ragged or not);
  - **a size guard** (ledger L13): `vec_push_a` ends the process one element past `VEC_CAP_MAX`, and bayan pushes
    every array element through it, so a document that could hold such an array (2·`VEC_CAP_MAX` bytes, 512 MiB by
    default) is refused unread as `OUT_OF_MEMORY`. pramana's numbers were added to the open cyrius vec issue;
  - **a golden `json` module:** serde_json's reading of 3,886 documents with `float_roundtrip`. They cover every
    number and string form, 120 exact float midpoints with their neighbours, and seeded floats, strings, vectors and
    matrices. `validate.py` re-reads every one with its own model of serde_json, written from serde_json's source. All
    3,886 match on x86_64, PE and aarch64. Seven mutation checks each fail their rows, or show the arena's doubling
    carries every document from a 1× start;
  - **tests:** `tests/json.tcyr` has 77 checks;
  - **the pool:** a claim over 1 MiB now takes no slot, so no slot holds more than 2 MiB. The bump allocator counts
    its refusals, so the reader can tell an exhausted arena from a parse error;
  - **cost:** 7–22× serde_json per document (`tests/json.bcyr` and its Rust twin, on byte-identical documents). For
    example, a Vec of 100 f64s takes 66–67 µs against 3.9–4.0 µs. The pre-pass's rewrite cut each decode by about
    10%. The rest is recorded as a finding.

- **M1 part d3b: serde's struct rules and the two enum forms** (`src/json.cyr`; port plan D5 and §8.3; ADR 007 row
  16; results in `docs/development/port-parity.md` §7.10):
  - **`_pmn_json_struct`:** serde derive's rules. An unknown member is skipped unread; a field named twice or never
    refuses; every key must be UTF-8, as serde_json parses each one; the sequence form takes exactly the field count;
  - **`_pmn_json_unit_variant` and `_pmn_json_newtype_variant`:** the externally tagged forms of 1.3.0's two enums.
    A unit variant is `"Name"` or `{"Name": null}`, and a newtype is `{"Name": value}`;
  - **839 more golden rows** (4,725 in all): serde_json's reading of two derive test structs (`Normal`'s fields, and
    one field of every kind 1.3.0's types hold) and of 1.3.0's `Kernel` and `PramanaError`. They cover every field
    missing, named twice and set to each form of its kind, unknown members of every kind, the sequence form at every
    length, both enum forms, and seeded values;
  - **the mapped rows** (`tests/golden/json.map.tsv`, ADR 007 row 16): 20 documents serde_json reads only by
    skipping a member unchecked, which bayan refuses whole: a duplicate key, nesting past 128 levels, or a lone
    surrogate. `validate.py` finds them with its own model of bayan, beside its serde model, which now also follows
    serde's ignore path. All 4,725 rows pass on x86_64, PE and aarch64, 20 of them accepted under ADR 7;
  - **checks:** seven mutation checks each fail their rows; `tests/json.tcyr` grows to 104 checks;
  - **cost:** a `Normal` document decodes in 1.65–1.68 µs against serde_json's 79–81 ns (21×), and a `Kernel` in
    518–523 ns against 35 ns (15×).

- **M1 part d4: the JSON write core and the error codec** (`src/json.cyr`; port plan D5 and §8.3; ADR 007 rows 7
  and 8; ledger L14; results in `docs/development/port-parity.md` §7.11):
  - **the writers** append serde_json's compact text to the caller's builder: fmtf's f64s (`null` for NaN and ±∞),
    unsigned decimals, strings with serde_json's escape table, vectors, row-major matrices, struct members in field
    order, and both enum forms. Each returns `OUT_OF_MEMORY` when the builder can't grow, never aborting;
  - **a guard** (ledger L14): five stdlib builder writers end the process on a failed grow, and `putc` and
    `add_json_str` have no form that returns −1. pramana reserves the room first, 32 bytes per f64 and the exact
    escaped length per string, so they never grow. pramana's numbers were added to the open cyrius str_builder
    issue;
  - **the error codec:** `pmn_err_to_json(code, sb)` writes 1.3.0's variant with an empty message
    (`{"InvalidParameter":""}`) and refuses `NONE`, `OUT_OF_MEMORY` and `DESERIALIZE`, which no 1.3.0 reader takes.
    `pmn_err_from_json(buf, len)` returns `Ok(code)`, reading the message and dropping it. ADR 007 row 8 is amended
    with the maintainer's choice;
  - **1,484 more golden rows** (6,209 in all): 1,382 `serde_json::to_vec` writes of every shape and 102 code-only
    error reads. `validate.py` re-writes each with its own model of serde_json's writing. All match on x86_64, PE and
    aarch64;
  - **checks:** five mutation checks each fail their rows. `tests/json.tcyr` grows to 137 checks, with round trips of
    6,300 seeded values;
  - **cost:** writing 100 f64s takes 291 µs against serde_json's 1.9 µs, fmtf's exact printer (ledger L10). A `Normal`
    takes 1.2 µs (19×), a string 0.7 µs (22×), an error code 0.22 µs (13×).

- **M1 part e1: the RNG and its codec** (`src/rng.cyr`; port plan D7 as amended, ADR 007 row 6; results in
  `docs/development/port-parity.md` §7.12):
  - **the trait and the generator:** `PmnRng`, Rust's `Rng`, which a consumer implements to bring their own
    generator, and `PmnSimpleRng`, 1.3.0's xorshift64. `PmnSimpleRng_new(out, seed)` maps a seed of 0 to 1;
  - **its JSON codec:** `g.to_json(sb)` and `PmnSimpleRng_from_json(out, buf, len)`. A saved state of 0 reads as 1,
    as 1.3.0's repr does;
  - **a golden `rng` module:** 40 seeds × 256 draws, and 10⁶-draw digests from six seeds, re-drawn independently by
    `validate.py`. There are also 190 serde_json codec rows, 10 of them ADR 007 row 16 mappings. All match on x86_64,
    PE and aarch64, drawn through a bounded generic as pramana's APIs draw;
  - **tests:** `tests/rng.tcyr` has 26 checks, among them a consumer's own generator through the trait, and a
    stream saved and resumed bit for bit;
  - **cost:** 7 ns a draw direct and 10 ns through the generic, against 1.7 ns for Rust's inlined draw
    (`tests/rng.bcyr` and its twin, the same stream).

- **M1 part e2: combinatorics with `PmnU128`** (`src/combinatorics.cyr`; port plan D10, ADR 007 row 4; results in
  `docs/development/port-parity.md` §7.13):
  - **the counting functions:** `pmn_factorial`, `pmn_permutations` and `pmn_combinations` return 1.3.0's
    `Result<u128>` as a code, with the value in a caller's `PmnU128 { lo, hi }`, written only on success. Their checked
    arithmetic is 1.3.0's step for step, every u64 compared unsigned, and C(130, 65) refuses on its intermediate
    overflow as Rust does;
  - **`pmn_stirling_approx`**, through the correctly rounded powf;
  - **the arithmetic** is probe P1's 128 × 64 multiply with overflow detection and its 128 / 64 division, which gains
    a 32-bit-limb fast path for divisors below 2³¹;
  - **a golden `combinatorics` module:** 2,405 rows on P1's grid and seeded pairs, with Stirling class C.
    `validate.py` re-checks them with exact integers and glibc's pow. `combinatorics.cr.tsv` has 0 rows, because
    glibc's pow is correctly rounded on all 315 inputs. All match on x86_64, PE and aarch64;
  - **tests:** `tests/combinatorics.tcyr` has 40 checks, among them 20,000 seeded fast-path divisions checked by
    multiplying back.

- **M1 part e3: the bridges** (`src/bridge.cyr`; results in `docs/development/port-parity.md` §7.14):
  - **the functions:** 1.3.0's eleven conversions for badal, kimiya, ushma and bodh, in Rust's operation order. Sums
    start at −0.0 as Rust's do, counts convert unsigned, and the fits keep their points in a pool claim;
  - **two conventions** (port plan §8.3): a returned tuple fills the caller's `out` array, with a code for the
    caller's own errors, and a slice of pairs is 16-byte records;
  - **a golden `bridge` module:** 5,553 rows of special-value grids, the 1e-15 floors and their neighbours, empty and
    NaN-bearing slices, the fits' skipped records, and seeds. 1,073 are class C. `validate.py` models all eleven
    through glibc and MPFR, and `bridge.cr.tsv` has 0 rows. Every output matches at 0 ulp on x86_64, PE and aarch64;
  - **tests:** `tests/bridge.tcyr` has 44 checks, all 19 of 1.3.0's tests among them, and the −0.0 sums, which class B's tolerance can't see;
  - **cost:** 4–8.5× 1.3.0 (`tests/bridge.bcyr` and its twin, the same outputs).

- **M1 part f: the gate, and M1 complete** (results in `docs/development/port-parity.md` §7.15):
  - **Debug (D12):** `src/debug.cyr`'s builders for the derive's struct and tuple shapes, `g.debug(sb, alternate)` for
    `PmnSimpleRng`, and `pmn_err_debug(code, sb, alternate)` (the variant with an empty message, refusing the codes
    1.3.0 has no variant for, as the JSON writer does). A golden `debug` module records Rust's own `{:?}` and `{:#?}`,
    60 rows, all matching;
  - **surface parity:** `scripts/surface_parity.py --check`, in CI, maps every public item of a ported 1.3.0 module
    to the Cyrius symbols `src/` defines. That is 30 items in error, rng, combinatorics and bridge, all ported;
  - **the soak:** `tests/soak.tcyr` runs 10⁴ rounds of every allocating M1 path. The bump heap, the freelist's arena
    and the mapped size stay unchanged, and nothing is left claimed;
  - **the audit:** `src/` checked against the library rules. The four functions whose result a caller must read are
    now `#must_use`, and the three test sites that ignored one now assert it;
  - **M1's result:** eight golden modules, 75,266 rows ok and 106 accepted, on x86_64, PE and aarch64, warning-free.

- **M2 part a1: erf, erfc, Φ and 1 − Φ, T0** (`src/special.cyr`; port plan D3 = C, ADR 009; results in
  `docs/development/port-parity.md` §8.1):
  - **the functions:** 1.3.0's fdlibm `erf` and `erfc`, and `normal_cdf` and `normal_sf` on erfc, in Rust's
    operation order on the correctly rounded exp, with the generated fdlibm coefficients. They are crate-private in
    1.3.0, so they are `_pmn_` here and outside the surface check;
  - **a golden `special` module:** 28,290 rows through the `oracle` hooks, named `math::<fn>`: every high word fdlibm
    branches on, with its neighbours and 24 seeded low words at and below it, both signs; seeded sweeps of each
    branch; tiny and wide sweeps; and k/32 and k/16 grids. `validate.py` transcribes the four functions and re-derives
    every row through glibc's exp. `special.cr.tsv` holds the 10 rows that move by 1 ulp when exp is correctly
    rounded (erfc near 10⁻⁶⁴ and 10⁻¹³⁸, and Φ and 1 − Φ there). Every row matches at 0 ulp on x86_64, PE and aarch64;
  - **mutation checks:** all eleven non-equivalent mutants fail rows. Each operation-order change fails 838–4,176,
    and each moved branch point 2–48. The first grid missed a moved RA/RB boundary, so seeded low words were added
    at every branch point;
  - **tests:** `tests/special.tcyr` has 25 checks, all six of 1.3.0's erf and Φ tests among them, and what a class-C
    replay can't see: erf(−0) = −0, a NaN's payload kept, and erf's odd symmetry bit for bit;
  - **cost:** 7–7.5× 1.3.0 in the tails, which call the correctly rounded exp twice; 10.6–13× in the small
    rationals, which is the compiler's f64 code (`tests/special.bcyr` and its twin `m2_special.rs`, the same sums).
    The Horner steps are written out, because `#inline` declines a three-argument step: that took 18–38 ns off
    every row. pramana's numbers were added to cyrius's open `#inline` issue.

- **M2 part a2: lnΓ and its relatives, T0** (`src/special.cyr`; port plan D3 = C, ADR 009; results in
  `docs/development/port-parity.md` §8.2):
  - **the functions:** 1.3.0's Lanczos `ln_gamma` (g = 7, n = 9, the reflection below ½), `stirling_error`, Loader's
    `bd0`, `ln_gamma_kernel` and `ln_beta` (R's lbeta scheme), in Rust's operation order on the correctly rounded ln,
    ln_1p and sin. 1.3.0's own edges are kept for T1 to change: lnΓ is +∞ for |x| ≲ 3.2·10⁻³⁰¹, finite at the
    negative integers, and NaN at +∞; ln B reads a NaN shape through Rust's NaN-ignoring min and max;
  - **golden rows:** 16,780 more in `special` (45,070 in all): lnΓ's switch at ½ and its 1e-300 cutoff, the negative
    integers and half-integers, the switches at 10, bd0 and the kernel on fixed ratios over seeded scales from 10⁻³ to
    10¹², and ln B on a pairwise shape grid and seeds. `validate.py` transcribes all five. `special.cr.tsv` gains 92
    rows: glibc's log1p misrounds on them (ln on one), and cancellation carries that to 13 ulp in bd0 and 101 in the
    kernel. Every row matches at 0 ulp on x86_64, PE and aarch64;
  - **mutation checks:** all fifteen mutants fail rows (2–5,853) or crash the replay. The first grid let a reordered
    ln B correction through, so 1,000 both-large pairs were added;
  - **tests:** `tests/special.tcyr` has 41 checks: 1.3.0's four tests for these functions, the poles and cutoff,
    bd0's identities (1.3.0 never tested bd0), and ln B's NaN reading and symmetry bit for bit;
  - **cost:** 5–8× 1.3.0 where ln and sin lead, 11–12× where the correctly rounded log1p does (`bd0`, the
    large-shape kernel, the both-large ln B); all 21 bench rows give the same sums as the twin.

- **M2 part a3: the regularized incomplete gamma, T0** (`src/special.cyr`; port plan D3 = C, ADR 009; results in
  `docs/development/port-parity.md` §8.3):
  - **the functions:** 1.3.0's `gamma_inc` (P and Q as two values: the series below x = a + 1, Lentz's continued
    fraction above, Wilson–Hilferty past a = 10⁹), its `iteration_budget` (min(200 + 20√a, 10⁶), truncated as Rust's
    `as usize`), and the two regularized wrappers, in Rust's operation order on the correctly rounded primitives.
    Probe P9's T0 transcription had no Wilson–Hilferty branch, so the port follows Rust's source;
  - **golden rows:** 6,771 more in `special` (51,841 in all): the switches at x = a + 1 and a = 10⁹, the mode
    x = a ± j√a, fixed ratios over seeded shapes from 10⁻³ to 10¹², the χ² shapes, the prefactor's underflow, and
    shapes down to 10⁻³⁰⁰, where 1.3.0's series overshoots 1 before its clamp. `validate.py` transcribes the loops.
    `special.cr.tsv` gains 36 rows (log1p 26, exp 10), up to 1,417 ulp in the deep tails, where exp turns the
    kernel's absolute rounding into relative error. Every row matches at 0 ulp on x86_64, PE and aarch64;
  - **mutation checks:** eleven mutants fail rows (14–1,114). The clamp mutant passed until the tiny-shape rows were
    added. The three that pass are code no reachable input runs: the budget's base (the loops take at most 8.16√a
    iterations, measured) and the continued fraction's two 1e-300 guards;
  - **tests:** `tests/special.tcyr` has 62 checks: 1.3.0's four incomplete-gamma tests, the budget's values, the
    edges, the wrappers bit for bit, and P(1, x) = 1 − e^-x. Its 1e-13 bound was widened from my first 1e-14, after
    MPFR showed 1.3.0's own algorithm is 1.4·10⁻¹⁴ off at x = 300 on correctly rounded primitives;
  - **cost:** 4.5–5.4× 1.3.0; all 24 bench rows give the same sums as the twin. The harness pads names to 44
    columns, for the wrappers' 41-character names.

- **M2 part a4: the regularized incomplete beta, T0; T0 complete** (`src/special.cyr`; port plan D3 = C, ADR 009;
  results in `docs/development/port-parity.md` §8.4):
  - **the functions:** 1.3.0's `beta_inc(a, b, x, y)` (I and 1 − I, with the caller's exact y), its Lentz `beta_cf`
    (TOMS 708's brcomp prefactor for two shapes ≥ 10, else logs on the precise side; even and odd steps), and
    `regularized_incomplete_beta`, in Rust's operation order on the correctly rounded primitives;
  - **golden rows:** 6,260 more in `special` (58,101 in all): a pairwise shape grid against eleven x, the side
    switch, the mean ± j standard deviations for two large shapes, the t and F callers' exact x and y, a y that is
    not 1 − x, and shapes from 10⁻³⁰⁰. `validate.py` transcribes both functions. `special.cr.tsv` gains 88 rows
    (log1p 82, exp 6, up to 751 ulp). Every row matches at 0 ulp on x86_64, PE and aarch64;
  - **a finding:** for an infinite or f64::MAX shape the prefactor is NaN, not 0, so 1.3.0 runs its whole
    10⁶-iteration budget before returning NaN (6 golden rows). The port does the same; T1 can refuse such arguments
    first;
  - **mutation checks:** eleven of eighteen mutants fail rows (8–2,508). The seven that pass are the budget and the
    five 1e-300 guards: counted on every row whose result depends on the loop, it uses at most 14.4% of its budget
    and no |d| or |c| falls below 10⁻⁷;
  - **tests:** `tests/special.tcyr` has 75 checks: 1.3.0's four incomplete-beta tests, the edges, the infinite
    shape's NaN, I_x(a, b) = 1 − I_y(b, a) bit for bit, and the wrapper;
  - **cost:** 3.8–5.2× 1.3.0. The small-shape bench row's sum differs from the twin's: the Python model re-creates
    its 4,096 arguments and gives Rust's sum through glibc and the port's through correctly rounded primitives (144
    arguments differ, glibc's log1p misrounding at the mean);
  - **T0 is complete:** the fourteen functions of 1.3.0's math.rs its `oracle` hooks expose, on 58,101 golden rows,
    226 of them corrected under ADR 009.

- **M2 part b1: erf and erfc correctly rounded (T1)** (`src/crerf.cyr`, `src/crerf_data.cyr`; port plan D3 = C, ADR
  009; ledger L8, L12; results in `docs/development/port-parity.md` §8.5):
  - **the functions:** erf and erfc are CORE-MATH's binary64 algorithms, ported without FMA by probe P9 and moved
    into `src/` unchanged but for names. 1.3.0's fdlibm transcription is retired. Φ, 1 − Φ and gamma_inc's
    Wilson–Hilferty branch keep Rust's formulas, now on the correctly rounded erfc;
  - **the tables:** generated by a new `scripts/vendor_coremath.py` from CORE-MATH's sources (fetched into
    `build/coremath/`, sha256-pinned, never committed). Its `--check`, in CI, also verifies every hex word typed
    inline in the port against the C literal it cites; its first run found three without a citation;
  - **correct rounding, proven:** `scripts/crverify/run.sh erf` (a new set) runs P9's 2,768,534 arguments, CORE-MATH's
    184,595 worst cases in both signs among them. Every row is correctly rounded against MPFR at 1,024 bits, the
    three targets' outputs are identical, and 0 rows differ from CORE-MATH's own C build;
  - **reference rows:** `validate.py` models every special row three ways and writes `special.ref.tsv`: 3,546 rows a
    T1 function moves (erf 226 and erfc 848 to MPFR's values, up to 2 ulp from 1.3.0; Φ 1,110, 1 − Φ 1,133 and
    gamma_inc 229 through Rust's formulas on them). The parity harness loads it as a third table, compared under the
    function's ADR 009 bound (0 for erf and erfc). Every row passes on x86_64, PE and aarch64;
  - **mutation checks:** a two-rounding fma fails 1,386 rows and an erfc that skips its accurate path 343; a tampered
    and a stale reference row each fail. An erf that skips its accurate path passes the replay (no golden erf row
    needs it) and fails 32,658 of `crverify`'s worst-case rows;
  - **tests:** `tests/special.tcyr` has 79 checks, with erf and erfc at CORE-MATH's hard cases, erfc's subnormal
    exception and its underflow edge;
  - **cost:** 542 ns to 2.4 µs a call, 28–157× Rust's fdlibm: correct rounding with the fma emulated exactly (a
    scalar fma, ledger L12, would remove most of it). The bench rows now also print an FNV-1a digest of their results'
    bits, which a sum of 10⁶ values can't stand in for. All 26 rows' digests are explained by `validate.py`'s models
    (Rust's through glibc, the port's through the reference), and the earlier parts' "sums agree" are corrected in
    the benchmark record.

- **M2 part b2: Φ and 1 − Φ within 1 ulp (T1)** (`src/crerf.cyr`, `src/special.cyr`; port plan D3 = C, ADR 009;
  results in `docs/development/port-parity.md` §8.6):
  - **the functions:** `normal_cdf` is probe P9's T1 form. −z/√2 is carried as a double-double, the low part's
    first-order correction is applied to CORE-MATH's unrounded double-double erfc, and the ½ is folded into one
    rounding, so the result is proven within 0.5 + 2⁻⁷ ulp. `normal_sf(z)` is `normal_cdf(−z)` bit for bit, as in
    1.3.0. The fast-path bound stays P9's verified 2⁻⁶⁰; 2⁻⁵⁶ would be about 30% faster in the core and is
    recorded as the option;
  - **constants:** the double-double 1/√2 is checked by `scripts/crverify/consts.py`, extended to double-doubles and
    `src/crerf.cyr`;
  - **verified:** `scripts/crverify/run.sh ncdf` (a new set) runs P9's nine seeded sets, 1,146,809 arguments. Its
    output equals P9's byte for byte and is identical on three targets. Against MPFR's ½·erfc(−z/√2) at 1,024 bits
    every row is within 0.5 ulp, and all but 4 are correctly rounded;
  - **reference rows:** `mpfr_ref.py` gains the true Φ and 1 − Φ, rounded once from 1,024 bits. `special.ref.tsv`
    grows to 7,158 rows (Φ 2,781 and 1 − Φ 2,793 where 1.3.0's value isn't the correctly rounded one, and 510
    Wilson–Hilferty `gamma_inc` rows). Every Φ and 1 − Φ golden row is correctly rounded, against ADR 009's 1-ulp
    bound, on x86_64, PE and aarch64;
  - **mutation checks:** dropping the correction fails 4,799 rows (to 759 ulp), dropping 1/√2's low word 5,266, and
    1 − Φ by subtraction 2,891. A second rounding for the ½ stays within the bound (30 rows at 1 ulp, shown in the
    histogram), and so does a looser fast bound (5 of `crverify`'s rows misrounded);
  - **tests:** `tests/special.tcyr` has 87 checks: Φ at points in the core, the tail and the subnormal tail, a point
    the fast path can't prove, Φ(±0), the 1 − Φ symmetry over 400 points, and NaNs;
  - **cost:** 1.37–1.40 µs in the core and 2.44–2.53 µs in the tails, 70–106× Rust's Φ and 8–30% over b1's Φ on
    the correctly rounded erfc; the bench's digests show the port's Φ equal to MPFR's on all 12,288 arguments.

- **M2 part b3: lnΓ within 2 ulp (T1)** (`src/lgamma.cyr`, `src/lgamma_data.cyr`, `src/special.cyr`; port plan D3 =
  C, ADR 009, amended; ledger L8; results in `docs/development/port-parity.md` §8.7):
  - **the function:** `ln_gamma` is probe P9's T1 form: fdlibm's `e_lgamma_r` layout with kimiya's double-double ln
    core for every logarithm, double-double products, an exact reflection for x < 0, and one final rounding. 1.3.0's
    Lanczos form is retired. Stirling's remainder, the kernel, ln B, `gamma_inc` and `beta_inc` keep Rust's formulas
    on it;
  - **changed edges, as part a2 recorded T1 would:** −ln x below 2⁻⁷⁰, where 1.3.0 returned +∞ below about
    3.2·10⁻³⁰¹; +∞ at the negative integers, where 1.3.0 gave finite values; +∞ at +∞, where 1.3.0 gave NaN;
  - **a fix to P9's form:** past lnΓ's overflow (about 2.556·10³⁰⁵) its double-double x·(ln x − 1) made ∞ − ∞ and
    returned NaN; two golden rows found it, past the end of P9's sets. T1 now returns +∞ there;
  - **the coefficients:** generated by a new `scripts/vendor_lgamma.py` from the libm crate 0.2.16's `lgamma_r.rs`,
    sha256-pinned, each checked against the bits in its comment. Its `--check`, in CI, also proves every word the
    port types inline: powers of two its comments cite, and thresholds as they occur in libm;
  - **verified:** `scripts/crverify/run.sh lgamma` (a new set) runs 1,000,000 arguments, P9's six sets 2.5 times
    over, with the large set carried to f64::MAX. Against MPFR's lgamma at 1,024 bits: max 1.27 ulp for x > 0 and
    1.26 ulp of max(|lnΓ|, 1) for x < 0, against ADR 009's 2. The three targets' outputs are identical;
  - **gating downstream rows (ADR 009 amended):** fed T1's lnΓ, 1.3.0's formulas land up to 1,024 ulp from the same
    formulas fed the true lnΓ. A row downstream of a T1 function is now gated, at 0 ulp, on Rust's formula fed the
    port's T1 value. `validate.py` models T1's lnΓ in the port's operation order with exact double-double steps, and
    every downstream row matches it at 0 ulp. Each T1 function's own rows stay gated against MPFR at its bound;
  - **reference rows:** `mpfr_ref.py` gains MPFR's lgamma. `special.ref.tsv` grows to 14,299 rows: lnΓ 2,441 and
    5,210 downstream (Stirling 328, the kernel 909, ln B 505, `gamma_inc` 1,515, `beta_inc` 1,571, the wrappers 382).
    The harness gives `ln_gamma` ADR 009's 2 ulp and, for x < 0, the mixed bound by a new `_par_c_mixed` hook. Every
    lnΓ row is within its bound (3,250 at 0 ulp, 100 at 1, 13 at 2, and 14 on the mixed bound), every downstream row
    is at 0, on x86_64, PE and aarch64;
  - **mutation checks:** six mutants each fail rows, from 2 (the overflow guard dropped) to 5,213 (−ln x's low part
    dropped). A plain-double product on [3, 8) keeps every lnΓ row within 2 ulp, and fails 31 downstream rows;
  - **tests:** `tests/special.tcyr` has 92 checks. The three that held 1.3.0's edges now hold T1's. New checks cover
    the roots, ½ and 10 within 2 ulp, −2.5 on the mixed bound, and the overflow from both sides;
  - **cost:** 153–179 ns for x > 0 and 555–562 ns reflected, 5.6–11× Rust's Lanczos form and 12–39% over T0's
    (P9: 72–192 and 570 ns). Downstream, Stirling's remainder and the small-shape kernel cost 26–28% more, ln B with a
    small shape 3–7%, the incomplete functions within the noise. The bench's 26 digests are all explained by the
    models, the lnΓ and downstream rows by the T1 model.

- **M2 part b4: the incomplete gamma within 4 ulp (T1)** (`src/gammainc.cyr`, `src/dd.cyr`, `src/zeta_data.cyr`;
  port plan D3 = C, ADR 009; ledger L8; results in `docs/development/port-parity.md` §8.8):
  - **the function:** `gamma_inc` up to a = 10⁹ is probe P9's T1 form: Rust's series and continued fraction with a
    double-double prefactor (lnΓ below a = 10 from a ζ series), double-double loops, the split at x = a, and Q taken
    directly for small shapes. Rust's Wilson–Hilferty form past 10⁹ stays, on T1's Φ. 1.3.0's double-precision loops
    are retired. The two regularized wrappers keep Rust's projections;
  - **three fixes to P9's form,** found past its sets:
    - the series now stops on a bound of its tail, not on its last term, which left 7 ulp at a = 10⁹;
    - below a = 2⁻⁵¹² the series' Σ/a is scaled by 2⁶⁰⁰, as Dekker's split overflowed into a NaN P at a = 10⁻³⁰⁰;
    - past x = 2⁹⁹⁰ the result is (1, 0), as the kernel overflowed the same way at x = f64::MAX, a golden row;
  - **shared double-double arithmetic:** `src/dd.cyr`, which `src/lgamma.cyr` now uses too, with its bits unchanged;
  - **the ζ(k) table:** generated by a new `scripts/gen_zeta.py` from MPFR, equal to P9's. Its `--check`, in CI, also
    proves the hex words typed inline in `src/dd.cyr` and `src/gammainc.cyr`;
  - **verified:** `scripts/crverify/run.sh gamma` (a new set) checks 148,000 rows against a 320-bit reference: P9's
    five sets, plus shapes from 10⁵ to 10⁹ and down to the least subnormal. The maximum is 1.0 ulp (2 units of 2⁻¹⁰⁷⁴
    for subnormal Q), against ADR 009's 4, and the three targets' outputs are identical. The reference now sums the
    series below x = 1 too, where the continued fraction hung for tiny shapes, and widens its precision below
    a = 2⁻¹⁰. `check.c` now counts a NaN gamma result as a failure, rather than skipping it;
  - **reference rows:** `mpfr_ref.py` gains `gamma_pq`, the true P and Q. `validate.py` computes them in a process
    pool, cached in `build/golden/`, and gains `t1_gamma_inc`, the port step for step, which equals the port on all
    3,799 golden rows with a ≤ 10⁵. `special.ref.tsv` grows to 17,230 rows: `gamma_inc` 3,669 at the truth (and part b2's
    510 Wilson–Hilferty rows), and the wrappers 534 on T1's values;
  - **the harness:** `gamma_inc` is gated at 4 ulp for a ∈ [10⁻³, 10⁵]. Outside that domain a new `_par_c_vs_rust` hook
    applies ADR 009's "no row less accurate than Rust" row by row, with 1.3.0's own error as each output's bound. On
    every golden row the port is within 1 ulp and never worse than 1.3.0, which reaches 872,342 ulp;
  - **mutation checks:** eight of eleven mutants fail rows (2 to 1,481). The exact downstream gate alone catches the
    dropped low words of ζ(k) and γ. Two pass within the bound, and P9's last-term stop is caught by `crverify` and a
    unit test;
  - **tests:** `tests/special.tcyr` has 105 checks. P(1, x) and Q(1, x) are now held within 4 ulp of the correctly
    rounded 1 − e⁻ˣ and e⁻ˣ, and 13 new points are checked against MPFR;
  - **cost:** 7.24–7.50 µs a call for small shapes and 5.82–5.97 µs for large ones, 48–61× Rust and 10–13× T0 (P9:
    10–17×). Every series and continued-fraction step is double-double, which P9 showed the bound needs. The bench's
    26 digests are all explained, the T1 rows by the T1 model.
- **M2 part b5: the incomplete beta within 8 ulp (T1)** (`src/betainc.cyr`; port plan D3 = C, ADR 009; ledger L8;
  results in `docs/development/port-parity.md` §8.9):
  - **the function:** `beta_inc` up to max(a, b) = 10⁹ is probe P9's T1 form: Rust's continued fraction on the side
    x < (a + 1)/(a + b + 2), the smaller of x and y taken as exact, a double-double prefactor and fraction, and TOMS
    708's power series for a complement below 2⁻⁶. It runs on `src/dd.cyr`, gammainc's lnΓ below 20 and the ζ table.
    Past 10⁹ Rust's form stays, as for `gamma_inc` (the maintainer's choice): near the switch the fraction costs 5 ms a
    call at 10⁹ and 130 ms at 10¹⁵. Rust's form is off by 10³–10⁶ ulp there near the mean, a finding for 2.1. An
    infinite shape returns Rust's (NaN, NaN) before the loop, not after its 10⁶-step budget. The wrapper keeps Rust's
    projection;
  - **five fixes to P9's form,** found past its sets:
    - the power series' ln(a·B(a, b)) is lnΓ(1 + a) − [lnΓ(a + b) − lnΓ(b)], with the difference in a form whose
      every term is proportional to a (Stirling's from b = 20, the ζ series term by term below). P9's sum of lnΓ lost
      the a/(a + b) of two tiny shapes entirely (1 − I = 6.7·10⁻³¹⁹ for 2.4·10⁻³³), and left 118 ulp at b = 6.1·10⁸;
    - a subnormal numerator is scaled by 2⁶⁰⁰ in the double-double quotient a/b, which kept 27 bits of it;
    - the prefactor takes TOMS 708's brcomp form for two shapes of 20 or more, and the Stirling difference for ln B
      with one: P9's sum cancelled from a + b down to O(1), leaving 2–3 ulp near 10⁹;
    - the side is chosen on the x the computation uses (1 − y when y is the smaller), not the caller's x: on 1.3.0's
      rows that pass a y that isn't 1 − x, P9's form returned (0, 1) for (1, 0);
  - **verified:** `scripts/crverify/run.sh beta` (a new set) checks 140,098 rows against `bref`. They are P9's five sets,
    reproduced byte for byte before the fixes, and sets out to shapes of 10⁹ (the switch included) and down to the least
    subnormal, with x and y at 2⁻¹⁰⁷⁴, 2⁻¹⁰²², 2⁻⁵³ and the switch. The maximum is 1.0 ulp (one unit of 2⁻¹⁰⁷⁴ on two
    subnormal results), against ADR 009's 8; the three targets' outputs are identical. `bref` holds x and y exactly,
    grows its precision for a tiny complement (P9's fixed 320 bits keep none of a 2⁻¹⁰⁰⁰ one), and now agrees with the
    exact rationals to 2.3×10⁻⁹⁷ ulp. `check.c` now counts a NaN `bref` result as a failure, rather than skipping it;
  - **reference rows:** `mpfr_ref.py` gains `beta_ic`, the true I and 1 − I. `validate.py` pools and caches them (5,714
    inputs, 50 s) and gains `t1_beta_inc`, the port step for step, which equals it on all 6,219 golden inputs in T1's
    range. `special.ref.tsv` grows to 20,621 rows: `beta_inc` 4,834 at the truth and the wrapper 243 on T1's value.
    `special.cr.tsv` falls to 91;
  - **the harness:** `beta_inc` is gated at 8 ulp for a, b ∈ [10⁻³, 10⁵], and outside that domain, up to 10⁹, by
    `_par_c_vs_rust`. Every golden row is within 1 ulp and none is less accurate than 1.3.0, which reaches 59,059 ulp on
    the domain and 151,644 past it, and returns NaN for tiny shapes;
  - **mutation checks:** 17 of 21 mutants fail the replay, a unit test or both. Several of the fixes only show past the
    golden rows, so each is pinned by a unit test from the `crverify` row where its mutant is worst (2×10¹⁴ ulp for
    lnΓ(1 + a) as lnΓ(a) + ln a, 50,109 for the brcomp prefactor dropped). Two survivors cost 1 ulp on 1 and 4
    `crverify` rows, and two are equivalent;
  - **tests:** `tests/special.tcyr` has 126 checks: §3.2's worst rows, P9's defects and every b5 fix against MPFR, the
    switch on a y that isn't 1 − x, and Rust's form past 10⁹ bit for bit;
  - **cost:** 17.78–18.46 µs a call on small shapes and 33.41–36.20 µs on large ones, both at the mean, 72× and 56× Rust
    and 14–15× T0 (P9: 10–17×). Every continued-fraction step is double-double, which P9 showed the bound needs. The
    bench's 26 digests are all explained, the T1 rows by the T1 model.
- **M2 part c: M2's gate** (port plan §7; results in `docs/development/port-parity.md` §8.10). **M2 is complete:**
  - **§3.2's sets, re-measured on the port:** `scripts/port-baseline/run.sh` now measures both sides on identical rows.
    A new `scripts/port-baseline/port.cyr` reads the Rust baseline's outputs and writes them back with the port's
    results, and the Rust side reproduces the plan's §3.2 table exactly. Against MPFR, or the exact rationals for the
    integer-shape incomplete beta:

    | Function | Rust 1.3.0, max | The port, max | ADR 009's bound |
    |---|---|---|---|
    | the primitives, `erf`, `erfc` | 2.49 ulp (`erfc`) | 0.5 ulp, 0% not CR | correctly rounded |
    | `normal_cdf`, z ∈ [−38, 8] | 1,590 ulp | 0.5 ulp, 0% not CR | 1 ulp |
    | `ln_gamma` on [0.5, 10]; beside 1 and 2 | 1.2×10⁶; 4.3×10¹³ ulp | 1.25; 0.98 ulp | 2 ulp |
    | `ln_gamma` for x < 0, in ulps of max(\|lnΓ\|, 1) | 1,583 | 0.93 | 2 |
    | `gamma_inc` P and Q, all 20,000 pairs | 97 and 485 ulp | 0.60 and 0.64 ulp | 4 ulp |
    | `beta_inc` I and 1 − I, integer shapes | 550 and 583 ulp | 0.50 and 0.50 ulp | 8 ulp |

  - **the rows T1 moves:** all of `special`'s 58,101 are gated, with nothing pending: 37,389 at 1.3.0's own bits, 91
    corrected for a primitive, 17,592 at a T1 function's true value and 3,029 downstream of one. Only the distributions
    and hypothesis tests call math.rs besides, and their rows will be recorded against T1 from the start;
  - **the audit of M2's sources:** no aborts, no allocation, every transcendental correctly rounded, every inline hex
    word proven by a `--check`, every name prefixed. `#must_use` now marks 58 pure helpers of the T1 files that lacked
    it; the `crverify` generators compile to byte-identical binaries before and after;
  - **the soak:** `tests/soak.tcyr` gains 10⁴ rounds of all fourteen of math.rs's functions across their branches.
    The bump heap, the freelist, the mapped size and the pool are unchanged on all three targets (13 checks).

### Performance

- **The u128 division's fast path:** `combinations(60, 30)` falls from 28.5 µs to 1.14 µs (pinned, the same run),
  and is 8.4× Rust's 131 ns. The other counting rows are 4–24× Rust (`tests/combinatorics.bcyr` and its twin).
- **The pool's release** is a compare-and-swap rather than kimiya's full fence and plain store: a claim and its release
  fall from 35–36 ns to 18–19 ns (`tests/pool.bcyr`, three pinned, interleaved rounds each way). The stdlib's CAS
  already orders as acquire-release on x86_64 and aarch64.

### Changed

- **The Cyrius port scaffold** (port plan M0 steps 1–2).
  - **`cyrius port`** moved the Rust 1.3.0 crate into `rust-old/` (330 tests still pass there on its pinned
    toolchain) and created the Cyrius skeleton: `cyrius.cyml`, `src/main.cyr`, `tests/pramana.{tcyr,bcyr,fcyr}`.
  - **The repairs:**
    - CLAUDE.md merged with the Cyrius rules; the roadmap restored; `state.md` written;
    - the Rust workflows moved to `rust-old/.github/`, and the Cyrius CI extended with a frozen-oracle job;
    - the manifest reads `VERSION` and includes `bench`;
    - the Rust-only docs and scripts moved to `rust-old/`;
    - ADRs 006–009 (`docs/decisions/`);
    - CONTRIBUTING and SECURITY rewritten, and a README port banner added.

  `cyrius build`, `test`, `test --bounds`, `bench`, `fuzz`, `lint`, `fmt --check`, `vet`, `deps --locked` and
  `audit` all pass.
- **`cyrius.cyml`** (port plan M0 step 3):
  - **stdlib leaves:** math, ganita, overflow, tagged, result, freelist, fnptr, atomic and bayan, on top of the core
    leaves; `args` is dropped, since a library reads no command-line arguments;
  - **no hisab** (its four routines will be transcribed);
  - **dead-code elimination on:** the skeleton builds to 407,872 bytes, with 806,256 bytes eliminated;
  - **the lock:** 43 files, and 51 since M1 part d2 added `thread` to the test-only leaves.
- **Toolchain pinned**: `rust-toolchain.toml` moves from `stable` to `1.100.0`, the toolchain that records the port's golden values. The CI coverage job adds `llvm-tools-preview` to the pinned toolchain

## [1.3.0] - 2026-11-14

Toolchain/dependency update and P(-1) audit/hardening sweep.

### Changed

- **Toolchain**: MSRV 1.89 → **1.100** (edition 2024 remains the latest edition); CI MSRV job updated
- **Dependencies**: hisab 1.3 → 1.4, hoosh 1.1 → 1.3, tokio 1.50 → 1.54, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, criterion 0.5 → 0.8 (benches use `std::hint::black_box`); lockfile 318 → 274 crates
- **Removed unused `tracing` / `tracing-subscriber`** — nothing in the crate logged. The `logging` feature is kept as a reserved no-op so `features = ["logging"]` keeps building
- `deny.toml`: dropped license allowances not present in the tree (AGPL-3.0-only, OpenSSL, Unicode-DFS-2016, Zlib)
- **Special functions** (`math.rs`, see ADR 002 amendment): `erf`/`erfc` are a port of fdlibm `s_erf.c` (< 1 ulp; was A&S 7.1.26 with 1.5e-7 absolute and ~40% tail relative error); incomplete gamma/beta return both tails computed directly, with `O(√shape)` iteration budgets (was a fixed 200), cancellation-free prefactors, and a stable `ln_beta`
- `Normal::cdf`, all t/χ²/F p-values, `z_quantile` (now Halley-refined, full precision) and `t_quantile` inherit the accurate special functions; `Exponential`/`Weibull` CDFs use `exp_m1`
- **Poisson**: CDF via `Q(⌊x⌋+1, λ)` and PMF via a Stirling-form kernel — O(√λ)/O(1) instead of O(x²); sampling for `λ > 30` uses Hörmann's exact PTRS sampler instead of a normal approximation (different random stream for `λ > 30`)
- **Binomial**: CDF via the incomplete beta function and PMF via Loader's saddle-point form — O(1) instead of O(n·k); sampling for `n > 64` uses exact Beta order-statistic splitting (O(log n); different random stream for `n > 64`)
- `median`/`percentile`/`quartiles`/`iqr` use O(n) selection with `total_cmp`; `mode` is O(n log n) (was O(n²)) with the same first-occurrence tie-break
- `MarkovChain::steady_state` iterates the lazy chain `(I + P)/2` and tests the true residual `‖πP − π‖₁`, so periodic chains converge
- HMM forward/backward and Baum-Welch use Rabiner per-step scaling (the docs already claimed this); Baum-Welch accumulates emission counts in O(T·N)
- `linear_regression` uses centered two-pass sums; `chi_squared_test` uses the exact χ² upper tail (was the Wilson–Hilferty approximation)
- `MultivariateNormal`'s symmetry check is relative (`1e-10 · max|Σᵢⱼ|`) instead of absolute
- `HistogramData::from_data` skips non-finite values (and does not count them in `total`); bins are always strictly increasing, even for constant data
- `MarkovChain::step` rounding fallback picks the last *reachable* state instead of the last state

### Added

- `#![forbid(unsafe_code)]`, `#![warn(missing_docs)]`
- Validated deserialization (`#[serde(try_from)]`, wire format unchanged) for every distribution, `KernelDensity`, `MarkovChain`, `HiddenMarkovModel` and `BayesianUpdate`; `MultivariateNormal` recomputes its Cholesky factor on deserialize; `SimpleRng` maps a deserialized zero state to 1
- `ai::MAX_FORECAST_STEPS` (1000) and `ai::MAX_AR_ORDER` (32): model-supplied tool arguments are bounded; `QueryResult` is now `Serialize + Deserialize`
- 11 benchmarks: `normal_cdf_1000`, `poisson_cdf_lambda100_200pts`, `binomial_cdf_n200_20pts`, `binomial_sample_n10000_100`, `median_10001`, `mode_10000`, `ks_two_sample_1000`, `chi_squared_test_20`, `hmm_forward_1000`, `hmm_baum_welch_200_x5`, `steady_state_10` (34 total)
- 83 tests (236 → 319): one regression test per fixed defect, plus accuracy tests against closed forms, glibc `erf`/`erfc`, 50-digit decimal references and brute-force HMM likelihoods

### Fixed

- **Panics reachable from the public API** (all now errors or handled):
  - `median`/`percentile`/`quartiles`/`iqr` panicked on NaN input ("comparison does not implement a total order", Rust ≥ 1.81 sort); also `pca` eigenvalue sort
  - Deserializing a `MultivariateNormal` with a tampered `dim`/`cholesky_l`, or a `MarkovChain` with an out-of-range `state`, made `sample`/`step` panic; `MarkovChain::step` with an out-of-range public `state`; HMM methods with inconsistent public matrices; `predict_logistic_proba` on an empty model
  - `permutations(u64::MAX, 0)` and `polynomial_regression(.., usize::MAX)` overflowed; `metropolis_hastings`/`gibbs_sampling` with `n_samples + burn_in` overflow or huge `n_samples`; `arima_forecast` with huge `steps` (capacity overflow)
- **Hangs**: `Poisson::cdf`/`pdf` and `Binomial::cdf`/`pdf` were O(x²)/O(n·k) (effectively infinite for `x = 1e18` or `∞`); `Binomial::sample` was O(n) (`n = u64::MAX`); `mode` was O(n²)
- **Wrong results**:
  - `ChiSquared(1e6).cdf(1e6)` returned 0.11 (true 0.50): incomplete-gamma series truncated at 200 iterations; same for large-df chi-squared/Poisson/Gamma CDFs
  - HMM `forward_log_likelihood` returned `-inf` for sequences longer than a few hundred steps; Baum-Welch then returned `initial = [0, 0, …]`
  - `ks_two_sample` mishandled ties (identical samples gave D = 1/3); KS p-values for small λ came from a non-converged series
  - `integrate` (and `arima_forecast` with `d ≥ 2`) seeded every level with raw prefix values: Δ²-integration of `[1, 4, 9, 16, 25]` gave `[1, 5, 11, 19, 29]`
  - `Binomial::pdf` returned NaN for `p = 0` or `p = 1`
  - `linear_regression` reported "zero variance in x" for offset data such as `x = 1e9 + [0, 1, 2, 3]` (one-pass cancellation)
  - One-way ANOVA with zero within-group variance but different means gave a NaN p-value, so `reject` was silently `false`; tiny upper-tail F p-values rounded to 0
  - `learning_trials_to_power_law` / `response_times_to_hick_params` divided by the full slice length including skipped points (`T = 10·N⁻¹` fitted as `a = 3.51, b = 0.094`)
  - `monte_carlo_integrate` std error could be NaN (negative `E[f²] − E[f]²`); now Welford
  - Metropolis–Hastings started at a zero-density point computed `−∞ − −∞ = NaN` and never moved
  - `MarkovChain::steady_state` failed to converge for periodic chains
- **Validation gaps**: NaN/∞ parameters were accepted by every distribution constructor (`x <= 0.0` is false for NaN), `KernelDensity`, MVN, Markov/HMM rows, `logistic_regression` (`l2_reg`), `exponential_smoothing` (`alpha`), MCMC `proposal_std`, and Monte Carlo bounds; NaN `alpha`/`confidence_level` was accepted by every hypothesis test and CI (NaN p-value ⇒ `reject = false`); hypothesis tests, regressions, PCA and ARIMA now reject non-finite data; `chi_squared_test` rejects negative observed counts; `BayesianUpdate::new` rejects incoherent inputs (`likelihood · prior > evidence`, posterior > 1); `ks_one_sample` reports a CDF returning NaN; `Uniform::new` rejects widths that overflow
- `ai` `describe` tool advertised skewness/kurtosis it never computed
- Docs: `autocorrelation` (lag 0 returns 1), stale `MultivariateNormal::log_pdf` comment, threat model (claimed scaled HMM, tracing warnings, and Baum-Welch early stopping that did not exist), roadmap (bridges/soorat shipped in 1.1/1.2)

### Performance

Measured with criterion 0.8 on the same machine, interleaving original code → this change → original code
(`--warm-up-time 1 --measurement-time 3`); "before" is the faster of the two original-code runs. Benchmarks
whose code did not change moved +5% ± 3% between runs, so differences of that size are noise.

| Benchmark | Before | After | Change |
|-----------|-------:|------:|-------:|
| `distribution/poisson_cdf_lambda100_200pts` | 5.52 ms | 23.0 µs | **−99.6%** |
| `distribution/binomial_cdf_n200_20pts` | 3.13 ms | 3.80 µs | **−99.9%** |
| `distribution/binomial_sample_n10000_100` | 1.88 ms | 44.8 µs | **−97.6%** |
| `descriptive/mode_10000` | 13.8 ms | 174 µs | **−98.7%** |
| `descriptive/median_10001` | 108 µs | 18.4 µs | **−83%** |
| `markov/hmm_baum_welch_200_x5` | 267 µs | 151 µs | **−44%** |
| `markov/steady_state_10` | 979 ns | 597 ns | **−39%** |
| `distribution/mvn_sample_3d_1000` | 80.7 µs | 68.9 µs | **−15%** |
| `regression/linear_1000` | 4.32 µs | 3.83 µs | **−11%** |
| `distribution/chi_squared_cdf_1000` | 51.6 µs | 57.7 µs | +12% |
| `markov/hmm_forward_1000` | 35.9 µs | 44.5 µs | +24% |
| `distribution/poisson_sample_lambda100` | 17.0 µs | 29.7 µs | +74% |
| `hypothesis/chi_squared_test_20` | 88.6 ns | 156 ns | +77% |
| `distribution/normal_cdf_1000` | 11.7 µs | 24.4 µs | +109% |

The regressions buy correctness: `normal_cdf` now uses the fdlibm `erfc` (< 1 ulp instead of ~1e-7
absolute / ~40% tail error, two `exp` calls instead of one); `chi_squared_test` computes the exact χ² tail
instead of the Wilson–Hilferty approximation; λ > 30 Poisson draws are exact PTRS samples instead of
rounded Gaussians; the HMM forward pass scales every step (the old pass returned `-inf` beyond a few hundred
observations); the incomplete gamma function converges further (large-df CDFs were wrong). All other
benchmarks (t-test, KS, Student-t pdf, logistic, samplers, KDE, MCMC, …) are within noise (−1% to +8%,
logistic +10–22% across runs with ±8% spread). The O(x²)/O(n·k)/O(n²) → O(1)/O(log n)/O(n log n) changes
grow with input size: e.g. `Poisson::cdf(1e18)` and `Binomial::new(u64::MAX, p).sample()` now return
immediately instead of never.

## [1.2.0]

### Added
- **bridge/bodh** — psychology cross-crate bridges: `item_variances_to_alpha` (psychometric reliability), `z_rates_to_d_prime` (signal detection), `trait_scores_to_correlation` (construct relationships), `learning_trials_to_power_law` (learning curve fitting), `response_times_to_hick_params` (Hick's law coefficient extraction)

## [1.1.0]

### Added
- **bridge** — cross-crate primitive-value bridges for badal (ensemble spread to confidence, observation error to likelihood), kimiya (yields to mean/variance, replicates to t-statistic), ushma (uncertainty to Gaussian, sensor noise to std dev)
- **integration/soorat** — feature-gated `soorat-compat` module with visualization data structures: `DistributionCurve` (PDF/CDF plots), `McmcTrace` (sample chains), `RegressionFit` (predicted+confidence bands), `HistogramData` (binned from raw data), `CorrelationMatrix` (NxN heatmap)

### Updated
- hoosh 1.0.0 -> 1.1.0, majra 0.21.3 -> 1.0.2, iri-string 0.7.11 -> 0.7.12, zerocopy 0.8.47 -> 0.8.48

## [1.0.0] - 2026-03-28

### Added

- **AI**: Natural language statistical queries via hoosh (`ai` feature flag) — describe, correlate, regress, t-test, and forecast tools with LLM-driven dispatch
- **Distributions**: Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull, Multivariate Normal (Cholesky-based sampling, log-PDF)
- **Regression**: Polynomial regression (QR-based least squares, Horner evaluation, R²), logistic regression (IRLS, L2 regularization, probability/class prediction)
- **Hypothesis testing**: One-way ANOVA (F-test, full SS decomposition), Kolmogorov-Smirnov test (one-sample and two-sample), confidence intervals (mean, two-means, proportion)
- **Quantile functions**: t-distribution (bisection) and standard normal (Acklam approximation)
- **Time series**: ARIMA (differencing/integration, Yule-Walker AR fitting, forecasting)
- **Descriptive**: Kernel density estimation (Gaussian, Epanechnikov, Uniform, Triangular; Silverman bandwidth), correlation matrix (Pearson), Principal Component Analysis (eigendecomposition)
- **Monte Carlo**: Metropolis-Hastings MCMC (Gaussian random walk, burn-in) and Gibbs sampling
- **Markov**: Hidden Markov Models (Forward algorithm, Viterbi decoding, Baum-Welch EM)
- **Math**: Shared crate-internal module with `ln_gamma`, `ln_beta`, regularized incomplete beta/gamma functions, erf/erfc
- Doc-tests for crate-level examples
- Benchmarks for all modules (23 benchmarks)
- 212 tests (194 unit + 15 integration + 3 doc-tests)

### Changed

- Hypothesis test functions (`t_test_one_sample`, `t_test_two_sample`, `chi_squared_test`) now accept configurable `alpha` significance level
- `#[non_exhaustive]` on all public structs with validated constructors
- Poisson sampling uses normal approximation for lambda > 30
- Gamma sampling uses iterative Ahrens-Dieter boost (no recursion)
- Dependencies use public crates.io versions (hisab, hoosh)

### Fixed

- Dead variable in naive Bayes classifier loop
- Missing serde roundtrip tests for Exponential, Poisson, Binomial, Bernoulli
- Potential stack overflow in Gamma sampling for small alpha
- Potential infinite loop in Poisson sampling for large lambda

## [0.1.0] - 2026-03-26

### Added

- Initial release
- Probability distributions: Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli
- Descriptive statistics: mean, median, mode, variance, std_dev, skewness, kurtosis, percentiles
- Hypothesis testing: one-sample t-test, two-sample t-test, chi-squared test
- Linear regression with R-squared
- Bayesian inference: Bayes theorem, naive Bayes classifier
- Combinatorics: factorial, permutations, combinations, Stirling approximation
- Monte Carlo: integration, pi estimation, deterministic SimpleRng
- Markov chains: transition matrix, steady-state, simulation
- Time series: moving average, exponential smoothing, autocorrelation
- Serde support for all public types
- Criterion benchmarks
