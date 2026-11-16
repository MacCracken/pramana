# Benchmarks — Rust 1.3.0 vs the Cyrius port

Each row times one benchmark body twice:
- **Rust:** the same work in Rust, either `rust-old/`'s criterion benches or a twin in `scripts/probes/rust/`;
- **Cyrius:** a `tests/*.bcyr` file, run by `cyrius bench`.

Port milestones append rows as their modules land; M8 part b records Rust's 34 criterion benches in both columns.
Numbers are host-specific, so compare within a column set, never across hosts.

Speed is measured, not gated (CLAUDE.md; port plan §0). A slow row is a finding to file, with its numbers, not a
reason to change the port. The dev box is shared, so every row comes from pinned (`taskset -c 5`), interleaved runs.

## M1 part b1: the correctly rounded primitives (2026-11-15)

**Bodies.** `tests/crmath.bcyr` and its twin `scripts/probes/rust/src/bin/m1_primitives.rs`: the same xorshift fills
4,096 arguments per row, and each row calls the function on them 250 times (1,024,000 calls). Rust's f64 methods are
glibc 2.44's libm (rustc 1.100.0). Cyrius's are kimiya 2.0.1's correctly rounded functions, vendored
(`src/crmath.cyr`). Three rounds, Cyrius and Rust interleaved, pinned to core 5, at a load average of 2.3–3.5.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `exp`, x ∈ [−20, 20) | 5.7–5.8 ns | 46 ns | 8.0× |
| `ln`, x ∈ [0.001, 1000) | 5.3–5.4 ns | 54–55 ns | 10.2× |
| `exp_m1`, x ∈ [−1, 1) | 5.7 ns | 50–51 ns | 8.9× |
| `powf`, x ∈ [0.5, 2), y ∈ [−10, 10) | 12.1–12.2 ns | 134–136 ns | 11.1× |
| `sin`, x ∈ [−10, 10) | 9.2–9.3 ns | 91 ns | 9.8× |
| `cos`, x ∈ [−10, 10) | 9.4 ns | 87–89 ns | 9.4× |
| `atan`, x ∈ [−10, 10) | 8.4–8.5 ns | 119–120 ns | 14.1× |

**Findings.**
- glibc is 8–14× faster. It misrounds 76 of the golden rows (`math.cr.tsv`), and the stdlib's own forms up to 8% of
  them (port parity §5). The Cyrius functions are correctly rounded on every one of 7.7M MPFR-checked arguments and
  give the same bits on x86_64, PE and aarch64 (port parity §7.2).
- The ratios sit inside the siblings' (12–62× against glibc for pravash, port plan §3.7). atan is the slowest row:
  it runs as atan2(x, 1), so a one-argument atan is the candidate to measure if a caller ever needs it faster
  (Rust 1.3.0 calls it once).
- Run: `taskset -c 5 cyrius bench tests/crmath.bcyr` and, from `scripts/probes/rust/`,
  `taskset -c 5 cargo run --release --locked --bin m1_primitives`.

## M1 part b2: log1p (2026-11-15)

The same bodies, two rows added. Three rounds, interleaved, pinned to core 5, at a load average of 2.0–6.0.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `ln_1p`, x ∈ [−0.5, 1) | 6.8–6.9 ns | 95–96 ns | 13.9× |
| `ln_1p`, x ∈ [−1/1024, 1/1024) | 5.5–5.7 ns | 51–53 ns | 9.4× |

**Finding.** The general path costs about 40 ns more than `ln`: a separate fast-path call with a two-value return,
then a TwoSum and a division before ln's core. Inlining the fast path, as kimiya inlines ln's core (about 14 ns there),
is the first thing to measure if `ln_1p` ever matters. Rust 1.3.0 calls it at 13 sites, in the special functions'
kernels and four densities.

## M1 part b3: tan (2026-11-15)

The same bodies, one row added. Three rounds, interleaved, pinned to core 5, at a load average of 2.1–3.2.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `tan`, x ∈ [−10, 10) | 10.8–10.9 ns | 188–190 ns | 17.4× |

**Finding.** `tan` costs about one `sin` plus one `cos` (95–96 and 88 ns in the same rounds). It evaluates both on
one reduction, then divides in double-double. kimiya's joint sin-and-cos evaluation shares the table lookups and is
the candidate to measure if `tan` ever matters; Rust 1.3.0 calls it once, in the Cauchy sampler.

## M1 part b4: log2 (2026-11-15)

The same bodies, one row added. Three rounds, interleaved, pinned to core 5, at a load average of 1.7–2.7.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `log2`, x ∈ [0.001, 1000) | 5.1–5.3 ns | 104–109 ns | 20.6× |

**Finding.** `log2` costs about 50 ns over `ln` (56 ns in the same rounds): separate split and fast-path calls with
multi-value returns, ln's core called rather than inlined, and the Dekker product by 1/ln 2. `ln_1p` shows the same
pattern; inlining both is the candidate to measure together. Rust 1.3.0 calls `log2` once, in the Hick's-law fit.

## M1 part c: the Rust f64 surface (2026-11-15)

**Bodies.** `tests/mathx.bcyr` and the twin's `mathx` rows (`m1_primitives mathx`): the same seed, regions and loop.
Three rounds, interleaved, pinned to core 5, at a load average of 1.5–2.3.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `max`, x and y ∈ [−10, 10) | 1.4–1.6 ns | 13 ns | 8.7× |
| `clamp`, x ∈ [−10, 10) to [−1, 2] | 0.8–1.1 ns | 13–16 ns | 16× |
| `fract`, x ∈ [−10⁶, 10⁶) | 3.3–3.4 ns | 8–12 ns | 2.9× |
| `powi(25)`, x ∈ [0.9, 1.1) | 0.9–1.1 ns | 20–24 ns | 22× |
| `u64 as f64`, any word | 0.8 ns | 11–13 ns | 15× |
| `as usize`, x ∈ [−10³, 10²⁰) | 1.2–1.3 ns | 12–16 ns | 11× |
| `total_cmp`, x and y ∈ [−10, 10) | 2.0 ns | 11–13 ns | 6× |

**Findings.** Rust inlines each into a few instructions; the Cyrius rows pay a call plus the loop's per-iteration
`which` tests (probe P2's caveat), so these ratios measure the call more than the work. `powi(25)` is the one with
work in it: a generic loop of seven squarings and multiplies, where LLVM unrolls a literal exponent. Explicit
products for the six exponents Rust uses (2, 3, 4, 9, 25, 49), or an `#inline` that takes, are the candidates if a
caller's bench shows them.

## M1 part d1: serde_json's f64 text (2026-11-15)

**Bodies.** `tests/fmtf.bcyr` writes each value into one reused builder; the twin's `fmt` rows call
`serde_json::to_writer` into one reused buffer. Three rounds, interleaved, pinned to core 5, at a load average of
1.1–1.5.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `to_json`, x ∈ [0, 10⁶) | 19.2–22.4 ns | 2.82–2.85 µs | about 135× |
| `to_json`, any finite bits | 20.2–23.2 ns | 13.7–13.9 µs | about 650× |

**Finding.** impetus's printer is exact by a bignum search (Burger & Dybvig), whose cost grows with the decimal
exponent; serde_json's zmij is table-driven. pravash measured the same digit core at 54× for `{:?}` of eight f64s.
No hot path writes JSON. The fix is upstream: the shortest printer that ledger L10 asks of the stdlib and bayan.

## M1 part d2: the work-table pool (2026-11-15)

**Bodies.** `tests/pool.bcyr` claims, writes a word and releases, 10⁶ times per row (10⁵ for the nested row). Rust has
no pool: where pramana claims scratch, a Rust function allocates, so the twin's `pool` row allocates a 256-byte `Vec`,
pushes a word and drops it. Three rounds, interleaved, pinned to core 5, at a load average of 1.1–1.5.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| a 256-byte claim and its release (a slot hit) | 8.5–8.7 ns | 18–19 ns | 2.2× |
| a claim, a bump allocator, 4 allocations, the release | — | 66 ns | — |
| nine nested 256-byte claims, the ninth an overflow block | — | 262–267 ns | — |

**A win inside the part.** The first release was kimiya's, a full fence then a plain store: 35–36 ns a claim, half of
it the fence (`mfence` alone measured 18 ns). The stdlib's compare-and-swap already orders as acquire-release on both
arches, so the release is now a CAS 1 → 0: 18–19 ns, with the same three rounds before and after.

## M1 part d3a: the JSON read core (2026-11-15)

**Bodies.** `tests/json.bcyr` decodes each document 10⁴ times through `_pmn_json_decode`: the pre-pass, a pool claim,
bayan's parse, the reader, and the release. The twin's `json` rows call `serde_json::from_slice` with
`float_roundtrip`, built with `--features roundtrip`. Both sides build the documents from the same xorshift (Cyrius
through `fmtf.cyr`, Rust through serde_json) and print each one's length and FNV-1a hash, and all five are identical.
Three rounds, interleaved, pinned to core 5, at a load average of 2.3–3.1.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| Vec<f64>, 100 values in [−10⁶, 10⁶), 1,880 bytes | 3.91–3.98 µs | 66.4–67.1 µs | 17× |
| Vec<Vec<f64>>, 10 × 10 in [0, 1), 1,954 bytes | 5.01 µs | 74.4–74.6 µs | 15× |
| Vec<u64>, 100 words, 2,047 bytes | 3.21–3.25 µs | 69.5–71.8 µs | 22× |
| String, 130 bytes with escapes | 391–398 ns | 2.83–2.85 µs | 7.2× |
| f64, `1.5` | 21.5–23.2 ns | 390–392 ns | 18× |

**Where it goes**, on the 100-word document: the pre-pass about a third, bayan's parse most of the rest, the readers
2.2 µs, and a pool claim 20 ns.

**A win inside the part.** The pre-pass was rewritten from 27 to 23 µs on that document, which cut each decode by
about 10% (Vec<f64> 74–76 → 66–67 µs):
- one bound check instead of one a byte;
- inline digit tests;
- a decimal subtraction in place of a division loop for integers at or above 2⁶³.

**Finding.** About 11 ns a byte remain in the pre-pass, from code generation that keeps every local in memory. No hot
path decodes JSON, and the pre-pass is a stand-in (ledger L11) that goes when bayan reads these numbers itself.

## M1 part d3b: struct rules and enums (2026-11-15)

**Bodies.** `tests/json.bcyr`'s two new rows decode as M3's codecs will:
- **`Normal`:** `_pmn_json_struct`, both f64s, then `Normal::new`'s checks. The twin decodes 1.3.0's real `Normal`,
  whose `try_from` makes the same checks.
- **`Kernel`:** `_pmn_json_unit_variant` against `serde_json::from_slice::<Kernel>`.

The documents are byte-identical. Three rounds, interleaved, pinned to core 5, at a load average of 2.5–3.8.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| Normal, its two fields and checks, 27 bytes | 79.1–80.9 ns | 1.65–1.68 µs | 21× |
| Kernel, a unit variant, 14 bytes | 34.5–35.4 ns | 518–523 ns | 15× |

The rounds also re-ran part d3a's rows, within 3% of their record (Vec<f64> 64.9–65.1 µs, the string 2.75 µs). Small
documents pay mostly the fixed costs of the pre-pass, the claims and bayan's containers. This is part d3a's finding.

## M1 part d4: the write core and the error codec (2026-11-15)

**Bodies.** `tests/json.bcyr`'s write rows write one value 10⁴ times into one cleared builder; the twin's call
`serde_json::to_writer` into one cleared `Vec`. Both sides print each text's length and FNV-1a hash, and all four
match. Three rounds, interleaved, pinned to core 5. The first attempt ran at a load average of 9–12 (another
project's processes) and was discarded; these ran once it had fallen below 2.5 (1.5–1.7).

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| write a Vec<f64>, 100 values in [−10⁶, 10⁶) | 1.90 µs | 290.7–292.0 µs | 153× |
| write a Normal | 63.7–65.3 ns | 1.21–1.23 µs | 19× |
| write a String, 5 escapes in 24 bytes | 30.9–33.0 ns | 688–711 ns | 22× |
| write an error code | 16.7–17.4 ns | 215–218 ns | 13× |

The same rounds re-ran the read rows within 6% of their record (Vec<f64> 62.0–62.9 µs, Normal 1.59–1.60 µs).

**Finding.** Writing f64s costs what part d1 measured for `fmtf.cyr`'s exact printer, about 2.9 µs a value here,
which the printer ledger L10 asks for would replace. The other rows are call overhead around the stdlib writers.

## M1 part e1: the RNG (2026-11-15)

**Bodies.** `tests/rng.bcyr` draws 10⁶ times from `PmnSimpleRng` seeded 42, first by a direct method call, then
through a bounded generic `<R: PmnRng>`. The twin's `rng` rows draw the same stream from 1.3.0's `SimpleRng`. Both
print the 10⁶th draw, `0x3fe327c85d8ff932`. Three rounds, interleaved, pinned to core 5, at a load average of 1.7.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `next_f64`, a direct call | 1.71–1.84 ns | 7 ns | 4× |
| `next_f64`, through a bounded generic | 1.70–1.72 ns | 10 ns | 6× |

Rust inlines the draw in both forms. Cyrius calls it, and the generic adds a call (P2's 2–3 ns). Sampling-heavy code
draws in loops, so this is the cost to watch in M3's samplers and M6's MCMC.

## M1 part e2: combinatorics (2026-11-15)

**Bodies.** `tests/combinatorics.bcyr` calls each function 10⁵ times on fixed arguments; the twin's `comb` rows call
1.3.0's. Both print the results' words, which agree. Three rounds, interleaved, pinned to core 5. The box had been
loaded (9–14) by another project's processes; these rounds waited for the 1-minute load to fall to 2.0.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `factorial(34)` | 65.3–74.2 ns | 930–935 ns | 14× |
| `permutations(30, 15)` | 15.7–15.8 ns | 372–376 ns | 24× |
| `combinations(60, 30)` | 131.2–131.3 ns | 1.10 µs | 8.4× |
| `combinations(130, 65)`, an overflow | 474.9–481.5 ns | 1.87–1.89 µs | 3.9× |
| `stirling_approx(100)` | 13.9–14.2 ns | 145–147 ns | 10× |

**A win inside the part.** The u128 division was probe P1's restoring division, 128 steps a call. `combinations`
divides at every step, so C(60, 30) took 28.5 µs (215× Rust). A 32-bit-limb fast path for divisors below 2³¹, which
covers every divisor `combinations` makes in practice, took it to 1.14 µs. Old and new were measured in the same
pinned run while the box was loaded, and gave the same words. The remaining gap is the multiply: eight 32-bit partial
products against Rust's two machine multiplies.

## M1 part e3: the bridges (2026-11-15)

**Bodies.** `tests/bridge.bcyr` against the twin's `bridge` rows, on the same inputs:
- 4,096 (residual, σ) pairs from mathx.bcyr's xorshift, cycled 250 times;
- 100 yields and 100 score pairs from the same stream, 10⁵ calls;
- 20 records for each fit, 10⁵ calls.

Both print the outputs' bits, and all five agree. Three rounds, interleaved, pinned to core 5, at a load average of
1.3.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `observation_error_to_likelihood` | 7.0–7.1 ns | 60 ns | 8.5× |
| `yields_to_mean_variance`, 100 yields | 134.4–135.4 ns | 575–578 ns | 4.3× |
| `trait_scores_to_correlation`, 100 pairs | 190.9–192.2 ns | 1.23 µs | 6.4× |
| `learning_trials_to_power_law`, 20 trials | 362.7–365.2 ns | 2.76–2.82 µs | 7.6× |
| `response_times_to_hick_params`, 20 records | 209.8–218.3 ns | 838–844 ns | 4.0× |

The likelihood pays the correctly rounded exp (46 ns, part b1), and the power law 40 correctly rounded logarithms.
The slice rows are within the sibling ports' median band (4.4–7.7×, port plan §9).

## M2 part a1: erf, erfc, Φ and 1 − Φ, T0 (2026-11-15)

**Bodies.** `tests/special.bcyr` against its twin, `scripts/probes/rust/src/bin/m2_special.rs`, which calls 1.3.0's
functions through the `oracle` hooks, on glibc's exp. Both use the same xorshift, regions and loop: 4,096 arguments a
row, cycled 250 times. Each prints its rows' sums as bits, and all ten agree (no argument here is one where glibc's
exp misrounds). Three rounds, interleaved, pinned to core 5, at a load average of 2.6–2.9 on 16 cores.

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| the loop alone, x → x | 1.0–1.2 ns | 4–8 ns | — |
| `erf`, \|x\| < 0.84375 | 3.6–3.7 ns | 47–48 ns | 13× |
| `erf`, 0.84375 ≤ x < 1.25 | 5.2–5.3 ns | 55–56 ns | 10.6× |
| `erf`, 1.25 ≤ x < 6 | 21.3–21.4 ns | 159–161 ns | 7.5× |
| `erfc`, 0.25 ≤ x < 0.84375 | 4.2 ns | 50 ns | 11.9× |
| `erfc`, 1.25 ≤ x < 2.857 | 21.0 ns | 157 ns | 7.5× |
| `erfc`, 2.857 ≤ x < 27 | 21.2–21.3 ns | 154–155 ns | 7.3× |
| `normal_cdf`, z in [−10, 10) | 19.4–19.5 ns | 145–146 ns | 7.5× |
| `normal_cdf`, z in [−38, −1.2) | 23.1–23.2 ns | 161–162 ns | 7.0× |
| `normal_sf`, z in [0, 38) | 22.6–22.8 ns | 158–159 ns | 7.0× |

**Where the time goes:**
- **The tails** (|x| ≥ 1.25, and Φ beyond |z| ≈ 1.77) call exp twice. The correctly rounded exp is 46–49 ns (part
  b1), so about 95 ns of each row's 155 is exp, against glibc's few. They sit at 7–7.5×, beside the bridges' exp row
  (8.5×).
- **The small rationals** have no exp. They are 10 or 12 Horner steps and a division: about 40 ns of f64 code against
  Rust's 2.5, once the loop is taken off. That is the compiler's f64 code, a finding outside the port.
- **Horner steps are written out.** As a three-argument helper, `c + s · acc`, each step is a call: `#inline` declines
  functions of more than two parameters. Written out, the small rational fell from 65–66 to 47–48 ns, the mid branch
  from 85–87 to 55–56, and the tails from 184–196 to 151–157. The bits are the same (all 28,290 golden rows at 0 ulp
  in both forms). The numbers were added to cyrius's open `#inline` issue
  (`2026-11-11-inline-refuses-small-numeric-helpers.md`).

## M2 part a2: lnΓ and its relatives, T0 (2026-11-15)

**Bodies.** `tests/special.bcyr` and its twin `m2_special.rs`, as in part a1. The row helper now draws a pair per
argument, so a1's rows run on new arguments. Both sides print every row's sum as bits, and all 21 agree. Three rounds,
interleaved, pinned to core 5, at a load average of 2.3–2.4 on 16 cores. a1's rows re-measured within 3 ns of part
a1's record (12.3× and 10.6× for erf's two rationals, 7.0–7.6× for the tails).

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `ln_gamma`, x in [0.5, 10) | 23.1–23.3 ns | 136–137 ns | 5.9× |
| `ln_gamma`, x in [−10, 0.5), reflected | 50.3 ns | 401–403 ns | 8.0× |
| `ln_gamma`, x in [10, 1000) | 27.2–27.5 ns | 137–138 ns | 5.0× |
| `stirling_error`, a in [0.5, 10) | 28.7 ns | 198–200 ns | 6.9× |
| `stirling_error`, a in [10, 1000), the series | 3.9–4.1 ns | 34–35 ns | 8.5× |
| `bd0`, x in [0, 100), m in [1, 100) | 9.5–9.7 ns | 106 ns | 11.2× |
| `ln_gamma_kernel`, a in [1, 10), x in [0.1, 20) | 29.7–30.3 ns | 198–200 ns | 6.7× |
| `ln_gamma_kernel`, a in [10, 1000), x in [5, 1500) | 17.3–17.5 ns | 204–207 ns | 11.8× |
| `ln_beta`, a and b in [0.5, 10) | 70.5–71.7 ns | 493–497 ns | 7.0× |
| `ln_beta`, a in [0.5, 10), b in [10, 1000) | 43.1–43.4 ns | 398–405 ns | 9.2× |
| `ln_beta`, a and b in [10, 1000) | 28.8–29.4 ns | 340–342 ns | 11.8× |

**Where the time goes.** Each row is its primitives:
- **lnΓ** on [½, ∞) is two correctly rounded lns (about 46 ns each, part b1) and eight divisions, at 5–6× Rust.
  Reflected, it adds a sin and two more lns, at 8×.
- **`bd0`, the large-shape kernel and the both-large ln B** are the slowest rows, 11–12×. Each is led by the
  correctly rounded log1p (51–96 ns, part b2), against glibc's few ns. The kernel adds an ln and the series, and
  ln B three series, two lns and a log1p.
- **The Stirling series** is pure arithmetic: eight divisions and six multiply-adds, 34 ns against Rust's 4, the
  compiler's f64 code as in part a1's rationals.

None of these functions is a public API in 1.3.0: they are the inner terms of the distributions and tests that M3
and M4 port. T1 (part b) replaces `ln_gamma` with P9's fdlibm-layout prototype, 72–192 ns (§5 step 9d-1).

## M2 part a3: the regularized incomplete gamma, T0 (2026-11-15)

**Bodies.** `tests/special.bcyr` and its twin `m2_special.rs`, as in parts a1 and a2. The gamma rows sum P − Q, so
both outputs reach the sum, and all 24 rows' sums agree. Three rounds, interleaved, pinned to core 5, at a load
average of 3.4–3.5 on 16 cores. The earlier rows ran 1–6 ns slower than their own records under this load; their
ratios held (5–13×).

| Row | Rust | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `gamma_inc`, a in [0.5, 10), x in [0.1, 20) | 125.1–126.9 ns | 564–565 ns | 4.5× |
| `gamma_inc`, a in [10, 1000), x in [5, 1500) | 126.2–126.8 ns | 610–615 ns | 4.8× |
| `gamma_inc`, Wilson–Hilferty, a and x in [10⁹, 1.0001·10⁹) | 58.5–59.0 ns | 315 ns | 5.4× |

**Where the time goes.** The series and the continued fraction are a division or two per iteration, and divisions
cost about the same in both compilers' code, so these rows sit closer to Rust than the special functions'
polynomials do. The prefactor adds the kernel (part a2) and a correctly rounded exp. The Wilson–Hilferty row is a
cbrt, a sqrt and part a1's Φ and 1 − Φ, which make four correctly rounded exps.

**A first form of the Wilson–Hilferty row was dropped.** It drew a and x independently over [10⁹, 10¹²), so z was
almost always far out, P − Q was ±1, and both sums came to exactly 0. The narrowed range keeps |z| ≲ 3.

## M2 part a4: the regularized incomplete beta, T0 (2026-11-15)

**Bodies.** `tests/special.bcyr` and its twin `m2_special.rs`, as before. The beta rows draw two shapes and put x at
the mean a/(a + b), with y = b/(a + b), where the continued fraction runs longest, and sum I − (1 − I). Three rounds,
interleaved, pinned to core 5, at a load average of 2.1–2.3 on 16 cores. The gamma rows re-measured 5–11 ns faster
than part a3's record, at the lower load, with the same ratios.

| Row | Rust | Cyrius | Cyrius / Rust | Sums |
|---|---|---|---|---|
| `beta_inc`, a and b in [0.5, 10), x at the mean | 245.4–248.2 ns | 1.29–1.30 µs | 5.2× | differ, as glibc misrounds |
| `beta_inc`, a and b in [10, 1000), x at the mean | 614.4–622.7 ns | 2.30–2.33 µs | 3.8× | agree |

**The differing sum is glibc's, not the port's.** 25 of the bench's 26 rows agree bit for bit with the twin; the
small-shape beta row doesn't (Cyrius `…8f21`, Rust `…8f8f`). The Python model settles it:
- it re-creates the row's 4,096 arguments from the bench's xorshift;
- through glibc's primitives it gives Rust's sum exactly, and through correctly rounded ones the port's;
- 144 of the 4,096 arguments (3.5%) differ between the two, because at the mean both of the prefactor's logs come
  from log1p, and glibc's log1p misrounds there.

**My tabulating script first missed both beta rows.** The bench prints a row over 1 µs in µs, and the script read
only ns, so it reported no differing sums. It now reads both. The earlier parts' rows were all under 1 µs, and their
row counts matched.

**Where the time goes.** The prefactor (part a2's ln B or the brcomp form, and an exp) and the loop's two divisions
per half-step. The large-shape row is the closest to Rust of any special-function row: its time is mostly the
loop's arithmetic, which costs about the same in both compilers' code.

## M2 part b1: erf and erfc correctly rounded, T1 (2026-11-16)

**A stronger cross-check, and a correction.** Until now each bench row printed its sum of 1,024,000 results, and I
recorded agreeing sums as agreement. A sum that size absorbs a 1-ulp change in any one result, so it could not have
shown one. This part proved that: CORE-MATH's erfc differs from 1.3.0's on 1,648 of the mid-range row's 4,096
arguments, and the sums still agree.
- **The digest:** each row now also prints an FNV-1a digest of its 4,096 results' bit patterns, on both sides,
  computed in a separate untimed pass. The timed loop is unchanged.
- **The check:** a script re-creates every row's arguments from the bench's xorshift and computes both digests
  through `validate.py`'s models. All 26 are explained: Rust's equals the glibc model's, and the port's equals the
  reference model's (correctly rounded primitives, and MPFR's erf and erfc).
- **What the sums hid:** the values differ between the two sides on 0 to 1,648 of each row's 4,096 arguments.
  - **The erf, erfc, Φ and Wilson–Hilferty rows:** 13 to 1,648, from T1.
  - **Part a2–a4's other rows:** up to 275, all of it glibc's misrounding: bd0 268, the large-shape kernel 275, ln B
    with both shapes large 145, beta at the mean 144, and 10 or fewer on the rest.

  Those parts' "sums agree" should read "sums agree, which checks nothing finer than the sum's own rounding".

**Timings.** Only the rows b1 changes were re-timed, from a copy of the bench with the other rows left out. Three
rounds, interleaved, pinned to core 5. Each round started only once the load average fell below 3, after an earlier
attempt ran at 15–24 (another project's fuzzer) and doubled Rust's own rows; these ran at 2.8–3.1. The
Wilson–Hilferty row's arguments differ from the full bench's (the copy skips the rows that draw before it), from the
same range.

| Row | Rust 1.3.0 (fdlibm) | Cyrius T1 (CORE-MATH) | Cyrius / Rust | Cyrius T0 (part a1) |
|---|---|---|---|---|
| the loop alone | 1.2–1.3 ns | 8 ns | — | 7–8 ns |
| `erf`, \|x\| < 0.84375 | 3.9–4.0 ns | 623–630 ns | 157× | 47–48 ns |
| `erf`, 0.84375 ≤ x < 1.25 | 5.3–5.4 ns | 630–634 ns | 119× | 55–56 ns |
| `erf`, 1.25 ≤ x < 6 | 21.8–22.0 ns | 616–625 ns | 28× | 159–161 ns |
| `erfc`, 0.25 ≤ x < 0.84375 | 4.3–4.4 ns | 542–550 ns | 127× | 50 ns |
| `erfc`, 1.25 ≤ x < 2.857 | 21.7–21.8 ns | 797–800 ns | 37× | 157 ns |
| `erfc`, 2.857 ≤ x < 27 | 22.0–22.1 ns | 2.42–2.43 µs | 110× | 154–155 ns |
| `normal_cdf`, z in [−10, 10) | 20.1 ns | 1.06–1.07 µs | 53× | 145–146 ns |
| `normal_cdf`, z in [−38, −1.2) | 23.7–24.0 ns | 2.27–2.28 µs | 96× | 161–162 ns |
| `normal_sf`, z in [0, 38) | 23.1–23.3 ns | 2.24–2.28 µs | 98× | 158–159 ns |
| `gamma_inc`, Wilson–Hilferty | 58.8–59.3 ns | 1.23 µs | 21× | 315 ns |

**Where the time goes.** This is correct rounding without a hardware fma, at the cost ADR 009 accepted for it:
- **CORE-MATH's fast paths** are double-double polynomials. Every fma in them is emulated exactly, by a Dekker
  product and a round-to-odd sum: about 17 flops for one instruction.
- **P9 measured the same costs** (508–675 ns on the main ranges, 2.3 µs on erfc's asymptotic range; §3.7), and found
  that in C the emulation alone takes CORE-MATH from 6–38 ns to 82–426 ns.
- **A scalar fma would remove most of it.** That is the cyrius request ledger row L12 tracks.

Against the T0 port of fdlibm, T1 costs 4–16×; against Rust's fdlibm, 28–157×. Φ and 1 − Φ, and gamma_inc's
Wilson–Hilferty branch, inherit it through erfc until part b2 replaces Φ.

## M2 part b2: Φ and 1 − Φ within 1 ulp, T1 (2026-11-16)

**Bodies.** As part b1: the rows b2 changes, from the copy of the bench, three rounds interleaved and pinned to core
5, each started under a load average of 3 (2.6–2.9 throughout). erf and erfc, which b2 doesn't change, re-measured
2–4% under their b1 record. The full bench's digests are all explained by `validate.py`'s models. The port's Φ rows
equal MPFR's correctly rounded Φ on all 12,288 of their arguments, and 1.3.0's differ from it on 1,942, 4,033 and
3,924 of each row's 4,096.

| Row | Rust 1.3.0 | Cyrius T1 (b2) | Cyrius / Rust | Cyrius at b1 (Rust's formula on the CR erfc) |
|---|---|---|---|---|
| `normal_cdf`, z in [−10, 10) | 19.4–20.0 ns | 1.37–1.40 µs | 70× | 1.06–1.07 µs |
| `normal_cdf`, z in [−38, −1.2) | 23.0–23.6 ns | 2.44–2.53 µs | 106× | 2.27–2.28 µs |
| `normal_sf`, z in [0, 38) | 22.3–23.0 ns | 2.44–2.45 µs | 106× | 2.24–2.28 µs |
| `gamma_inc`, Wilson–Hilferty | 58.2–58.3 ns | 1.42–1.45 µs | 25× | 1.23 µs |

**Where the time goes.** On top of the correctly rounded erfc, T1 adds:
- the argument's double-double;
- an exp for the correction;
- in (−4.1, −2.5), CORE-MATH's accurate path, where its fast bound misses 2⁻⁶⁰.

That is 30% in the core and 8% in the tails. P9 measured 1.28–1.33 µs in (−5, 5) and 2.29–2.36 µs in (−36, −5). A
2⁻⁵⁶ fast bound would take about 30% off the core (P9, §6.8) at a proven 0.5 + 2⁻³ ulp; the port keeps the verified
2⁻⁶⁰.

## M2 part b3: lnΓ within 2 ulp, T1 (2026-11-16)

**Bodies.** The whole of `tests/special.bcyr` and its twin, since lnΓ reaches most rows. Three rounds, interleaved and
pinned to core 5, each started under a load average of 3 (1.7–1.9 throughout).
- **The digests:** all 26 are explained by `validate.py`'s models. The port's lnΓ rows equal the T1 model's on all
  12,288 arguments, and so do the downstream rows. 1.3.0's lnΓ differs from T1's on 3,851, 3,814 and 2,172 of each
  row's 4,096.
- **The rows b3 leaves alone:** erf, erfc, Φ and the Wilson–Hilferty row re-measured 0.5–4% under their b1 and b2
  records, at the lower load.
- **One noisy round:** the first round put two rows 10–17% above the other two (ln B with both shapes large at 374 ns,
  the small-shape `gamma_inc` at 642 ns). The ranges below include it.

| Row | Rust 1.3.0 | Cyrius T1 (b3) | Cyrius / Rust | Cyrius T0 (parts a2–a4) |
|---|---|---|---|---|
| `ln_gamma`, x in [0.5, 10) | 22.3–23.1 ns | 177–179 ns | 7.9× | 136–137 ns |
| `ln_gamma`, x in [−10, 0.5), reflected | 49.6–52.5 ns | 555–562 ns | 11.0× | 401–403 ns |
| `ln_gamma`, x in [10, 1000) | 27.2–28.5 ns | 153–156 ns | 5.6× | 137–138 ns |
| `stirling_error`, a in [0.5, 10) | 28.4–29.4 ns | 250–253 ns | 8.7× | 198–200 ns |
| `ln_gamma_kernel`, a in [1, 10), x in [0.1, 20) | 29.5–30.5 ns | 253–261 ns | 8.5× | 198–200 ns |
| `ln_beta`, a and b in [0.5, 10) | 69.5–72.0 ns | 524–532 ns | 7.5× | 493–497 ns |
| `ln_beta`, a in [0.5, 10), b in [10, 1000) | 41.7–43.8 ns | 411–415 ns | 9.6× | 398–405 ns |
| `gamma_inc`, a in [0.5, 10), x in [0.1, 20) | 118.0–123.3 ns | 549–642 ns | 4.9× | 564–565 ns |
| `beta_inc`, a and b in [0.5, 10), x at the mean | 239.2–248.6 ns | 1.30–1.33 µs | 5.4× | 1.29–1.30 µs |

The rows that never call lnΓ are unchanged: Stirling's series, bd0, the large-shape kernel, ln B with both shapes
large, and the large-shape incomplete functions. They measured 38, 101–103, 204–208, 335–374, 585–614 ns and
2.27–2.34 µs.

**Where the time goes.** T1's lnΓ costs 12–30% more than T0's for x > 0 and 39% more reflected:
- every logarithm is kimiya's double-double ln core, not a rounded ln;
- the products and the large-x form's x·(ln x − 1) are double-double;
- the reflection adds an exactly reduced sin(πr) and the double-double ln of x·sin(πr).

P9 measured 72–192 ns for x > 0 (by branch) and 570 ns for x < 0. Downstream, the cost is the extra lnΓ calls:
- 26–28% on Stirling's remainder and the small-shape kernel, which call it once or twice around a few flops;
- 3–7% on ln B with a small shape;
- within the noise on the incomplete functions, whose loops dominate.

## M2 part b4: the incomplete gamma within 4 ulp, T1 (2026-11-16)

**Bodies.** The whole of `tests/special.bcyr` and its twin. Three rounds, interleaved and pinned to core 5, each started
under a load average of 3 (1.6–1.9 throughout).
- **The digests:** all 26 are explained by `validate.py`'s models. The port's two T1 `gamma_inc` rows equal the T1
  model (`t1_gamma_inc`) on all 8,192 of their arguments. 1.3.0's P − Q differs from T1's on 1,651 and 239 of each
  row's 4,096.
- **The rows b4 leaves alone,** the Wilson–Hilferty row included, re-measured within 5% of their b3 record.

| Row | Rust 1.3.0 | Cyrius T1 (b4) | Cyrius / Rust | Cyrius T0 (part a3) |
|---|---|---|---|---|
| `gamma_inc`, a in [0.5, 10), x in [0.1, 20) | 116.8–122.4 ns | 7.24–7.50 µs | 61× | 549–573 ns |
| `gamma_inc`, a in [10, 1000), x in [5, 1500) | 119.5–124.3 ns | 5.82–5.97 µs | 48× | 585–614 ns |

**Where the time goes.** T1 costs 10–13× T0, the 10–17× P9 measured (§6.5):
- **every series and continued-fraction step is double-double:** a double-double division and product, and a
  TwoSum, where Rust takes a division and a multiply-add;
- **the prefactor** is a double-double exponent through kimiya's exp core, with lnΓ below a = 10 from a ζ series of
  up to 64 terms;
- **the tail-aware stop** adds about 9% more series terms near x = a, and almost none elsewhere.

P9 found that plain-double loops miss the bound, by up to 79 ulp, so the cost has to come down through better
algorithms rather than less precision:
- Temme's uniform expansion near x = a for large a;
- the direct small-shape form carried further than x = 1.1.

Both are findings to measure, not changes for this part.

## M2 part b5: the incomplete beta within 8 ulp, T1 (2026-11-16)

**Bodies.** The whole of `tests/special.bcyr` and its twin. Three rounds, interleaved and pinned to core 5, each started
under a load average of 3 (2.79, 2.69 and 2.93 at the starts; other projects' jobs raised it during some rounds).
- **The digests:** all 26 are explained by `validate.py`'s models. The port's two T1 `beta_inc` rows equal the T1
  model (`t1_beta_inc`) on all 8,192 of their arguments. 1.3.0's I − (1 − I) differs from T1's on 4,051 and 4,027 of
  each row's 4,096: at the mean, where these rows sit, 1.3.0 is least accurate.
- **The rows b5 leaves alone** re-measured within 5% at their fastest round (the gamma rows 7.60 and 6.13 µs against 7.24 and 5.82), with single-round spikes of up to 70% on a few rows where the load rose mid-round of their b4 record.

| Row | Rust 1.3.0 | Cyrius T1 (b5) | Cyrius / Rust | Cyrius T0 (part a4) |
|---|---|---|---|---|
| `beta_inc`, a and b in [0.5, 10), x at the mean | 242.7–261.3 ns | 17.78–18.46 µs | 72× | 1.29–1.30 µs |
| `beta_inc`, a and b in [10, 1000), x at the mean | 605.0–654.1 ns | 33.41–36.20 µs | 56× | 2.30–2.33 µs |

**Where the time goes.** T1 costs 14–15× T0, within the 10–17× P9 measured (§6.6):
- **every continued-fraction step is double-double:** per half-step two double-double divisions, a reciprocal and four
  products, where Rust takes two divisions and a few multiply-adds; at the mean the fraction runs longest;
- **the prefactor** is a double-double exponent through kimiya's exp core: for two shapes of 20 or more the brcomp form
  with three double-double Stirling corrections, each of which divides out its seven coefficients afresh;
- **the fixes add little on these rows:** the power series and the Stirling difference run only for small shapes
  against large ones, which these rows don't draw.

As for `gamma_inc`, P9 showed that plain-double loops miss the bound, so the cost has to come down through algorithms:
- TOMS 708's `bgrat` for one large shape and `basym` for two, which are O(1) where the fraction takes O(√max(a, b))
  steps (P9's note);
- the Stirling coefficients as generated double-double constants, rather than seven divisions per call.

Both are findings to measure, not changes for this part.

## M3 part a1: Normal, T1 on its cdf (2026-11-16)

**Bodies.** `tests/dist.bcyr` and its twin `scripts/probes/rust/src/bin/m3_dist.rs`, built with `--features roundtrip`
(serde_json's correctly rounded parse, as the golden corpus reads JSON; its default parse misses by an ulp on some
means, which the JSON row's digest showed). The loop is `tests/special.bcyr`'s: 4,096 seeded arguments or pairs per
row, cycled 250 times; a draw row takes its uniforms from one generator seeded 42. Three rounds, interleaved and pinned
to core 5, each started under a load average of 3 (2.21, 2.60, 2.60).
- **The digests:** all six are explained by `validate.py`'s models (`build/handoff/bench_digests_dist.py`): 1.3.0's
  through glibc, the port's through the reference model. The pdf differs from 1.3.0's on 2 of 4,096 values and the
  draws on 1 (glibc's exp, ln or cos misrounding); the cdf on 1,877 and 4,023 (the T1 Φ); the JSON and Debug rows not
  at all.

| Row | Rust 1.3.0 | Cyrius | Cyrius / Rust |
|---|---|---|---|
| `Normal::pdf`, N(0, 1), x in [−10, 10) | 23.9–26.6 ns | 86.0–90.0 ns | 3.4× |
| `Normal::cdf`, N(0, 1), x in [−10, 10) | 26.6–27.6 ns | 1.37–1.41 µs | 51× |
| `Normal::cdf`, N(0, 1), x in [−38, −1.2) | 30.0–31.0 ns | 2.51–2.58 µs | 84× |
| `Normal::sample`, μ in [−10, 10), σ in [0.1, 10) | 28.8–29.3 ns | 227–235 ns | 8.0× |
| Normal to JSON and back | 188.6–192.5 ns | 8.72–9.67 µs | 48× |
| Normal's Debug | 270.5–271.1 ns | 6.32–6.40 µs | 23.5× |

**Where the time goes:**
- **the pdf** is one correctly rounded exp, kimiya's, about 3.4× glibc's, as M1 part b1 measured;
- **the cdf** is the T1 Φ, CORE-MATH's erfc with its FMA emulated (M2 part b2: 1.4–2.5 µs, 70–106× Rust's Φ); the
  constructor and the division cost nothing beside it;
- **a draw** is two correctly rounded primitives, ln and cos, and two uniforms;
- **JSON and Debug** print through impetus's exact shortest-digit printer on a bignum, the cost M1 part d1 measured
  (ledger L10 asks the stdlib for a faster one); the JSON row also parses through bayan's tree.

These are findings to measure against, not changes for this part: the cdf's cost is T1's, which ADR 009 accepts.
