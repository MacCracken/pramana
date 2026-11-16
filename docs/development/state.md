# pramana — Current State

> Refreshed every release and at every port milestone. `CLAUDE.md` is
> preferences/process/procedures (durable); this file is **state** (volatile).

## Version

**1.3.0** (`VERSION`), on its way to **2.0.0**, the Cyrius library. `cyrius.cyml` reads the version through
`version = "${file:VERSION}"`, and the release workflow checks both against the tag.

The last Rust release is **1.3.0**, tagged at `284eb18` (2026-11-14); `release.yml` publishes on a tag. The oracle
is commit `94a7153`: the audited 1.3.0 plus the hidden `oracle` hooks, with the toolchain pinned to 1.100.0. It is
frozen in `rust-old/` and never published (port plan D1).

## Port progress

Plan of record: [`pramana-port-plan.md`](pramana-port-plan.md). Every decision in its §0.2 was taken on 2026-11-15.

| Milestone | Status |
|---|---|
| M0 Initialize | ✅ **every step done; the gate passes** (`aa5df35`). **Step 0** (Rust, `94a7153`): the P(-1) gates are green; the hidden `oracle` feature exposes the private special functions, samplers, quantiles, HMM forward pass, KDE kernels and the `MultivariateNormal` factor; `tests/oracle.rs` has 11 tests, mutation-checked six ways; the toolchain is pinned to 1.100.0. **Step 1** (`d8e761d`): the `cyrius port` scaffold, with the Rust crate in `rust-old/`, where it still passes (330 tests with `--all-features`). **Step 2** (`40a8269`): repairs (CLAUDE.md, the roadmap, CI, the manifest's version and `bench`, the scaffold stubs documented, Rust-only docs and scripts moved, ADRs 006–009, CONTRIBUTING, SECURITY, a README banner). **Step 3** (`0e147d3`): the full `cyrius.cyml` (16 stdlib leaves, no hisab, `dce = true`; 43 files locked); every gate green and warning-free. **Step 4**: the semantics probe ([`port-parity.md`](port-parity.md) §1): 125 cases, 140 rows exact; helpers needed for min/max NaN, `fract` (floor-based `f64_fract`), `u64 → f64` (signed `f64_from`) and saturating `as usize`; sums start at −0.0; 5 transcendental rows 1 ulp apart, 3 of them glibc's misrounds (`53078cb`). **Step 5**: the golden generator (`scripts/golden/`): `tests/golden/math.tsv`, 35,597 rows in 31 functions, every one re-checked by `validate.py` without the oracle; `math.cr.tsv`, 76 rows where glibc misrounds (`exp_m1` 5.1%); deterministic and tamper-checked (`30eb41b`). **Step 6**: `src/constants.cyr`, 112 constants extracted from `rust-old/src` and proven by recomputing `erf`, `erfc`, `ln_gamma`, `z_quantile` and `kolmogorov_sf` bit for bit; two independent extractors agree, and every value compiles back exact (`bb0f938`). **Step 7**: the parity harness (pravash's, adapted): 2 modules, 0 failing; 3,963 rows ok, 24,669 measured, 7,077 pending; mutation-checked four ways; part of `cyrius test` (`d7941bc`). **Step 8**: every dispatched function's digest identical on x86_64 ELF, PE (wine) and aarch64 (qemu); bounds K = 2 for the interim primitives, 0 for cbrt, powf measured; gated: 0 failing, 24,162 ok, 70 accepted (ADR 9), on all three (`aec9861`). **Step 9** (probes, in parts): 9a done: P1 integers identical (3,736 cases, `PmnU128` prototype included); P5 transcribe all four hisab routines (ganita up to 40 / 18 / 14 / 13,619 ulp off, and leaking); P6 Rust's selection leaks the sign of a zero (9,393 of 180,000): decided, accept it (ADR 007 row 14, stable selection, rows mapped) (`96ae9d5`, `aa3dc1e`). 9b done: P2 `dyn` costs about 3 ns a call over a static generic (a quarter of a 12 ns draw), so D7 was amended: the RNG is generic, as Rust's `impl Rng`, with `dyn` only for Gibbs' conditionals; P3 every callback form within 1–3 ns of a direct call, no per-call allocation, D7's form stands; a captured f64 is untyped in arithmetic and lint refuses capturing closures (both for step 10); P4 every D11 soak flat (bump heap, freelist, mapped size, pool slots), with the pool growing on the freelist (`4a051bf`). 9c done: P7 serde_json's 1,495 documents through bayan's tree and a stand-in pre-pass agree on 1,460, every value read bit-exact (bayan alone: 1,325; integers ≥ 2^63, `-0`, raw control bytes), impetus's `fmtf.cyr` writes serde_json's text on all 596 (Grisu2 566), so D5 was amended to text identity, and ADR 007 row 16 accepts the 35 that serde_json reads by skipping a member unchecked; P8 one-function consumer = empty one (411,928 B), `ai` +839 KB, sandhi in `[deps]` costs each compile 0.8 s, so D8 is `[lib.ai]` with sandhi out of `[deps]` (`155dce0`). 9d-1 done: P9's `ln_gamma` prototype (fdlibm's layout, double-double logs and products) is within 1.31 ulp for x > 0 and 1.40 ulp of max(|lnΓ|, 1) for x < 0 on 1,675,000 MPFR rows (bounds ≤ 2; Rust 4.3×10¹³ beside the roots), identical on three targets, 72–192 ns against T0's 168; Rust 1.3.0 returns +∞ for 0 < x < 3.2e-301 (`998b64b`, `d424b90`). 9d-2 done: P9's `gamma_inc` prototype (double-double prefactor and loops, the split at x = a, a direct small-a Q) is within 1.0 ulp (mean ≤ 0.25) on 360,400 rows against a 320-bit MPFR reference (bound ≤ 4; Rust 97 / 485 on §3.2's pairs and 2.9×10⁹ in the deep lower tail), identical on three targets, at 10–17× T0's cost; plain-double loops miss (to 79 ulp) (`2f572be`). 9d-3 done: P9's `beta_inc` prototype (the smaller of x and y exact, a double-double prefactor and continued fraction, TOMS 708's power series for small complements) is within 0.51 ulp (mean ≤ 0.25) on 214,744 rows against a 320-bit reference checked against exact rationals (bound ≤ 8; Rust 550 / 583 on §3.2's rows, 1.7×10⁵ near the mean), identical on three targets, at 10–17× T0's cost (`121e182`). 9e done: CORE-MATH's erf and erfc ported without FMA (2 Dekker products, 57 exact fmas by a round-to-odd sum, one tiny-path fma rounded on the subnormal grid; 17 tables generated from the sources) give CORE-MATH's own bits on 2,568,534 rows, all its worst cases included, and are correctly rounded on all 2,768,518 MPFR rows, identical on three targets; 508–675 ns a call (2.3 µs in erfc's asymptotic range), 18–31× glibc, most of it the emulated fma (a scalar fma is requested, L12); D3 = C holds without the fallback (`ea7f1a0`). 9d-4 done: P9's `normal_cdf` prototype (−z/√2 as a double-double, the first-order correction applied to CORE-MATH's unrounded double-double erfc, the ½ folded into one rounding; proven ≤ 0.508 ulp) is within 0.5 ulp on all 1,246,809 of its MPFR rows and correctly rounded on all but 4 (bound ≤ 1; Rust 1,590 on §3.2's rows, 1,688 in the deep tail; the correctly rounded erfc on Rust's rounded argument also 1,688; the plan's two-rounding sketch reaches 1.0 ulp), identical on three targets, at 8–20× T0's cost (1.3–4.6 µs) (`26f04f2`). Step 9 is done. **Step 10** done: eight upstream issues filed and thirteen joined with pramana's numbers (five repros), across cyrius, ganita, hisab, bayan and hoosh; the ledger (port plan §10, 12 rows) is read by `scripts/upstream_ledger.py --check`, now in CI, whose first run caught `src/constants.cyr`'s unmarked typed constants (the generator now writes their `UPSTREAM(L2)` note); the M0 gate passes |
| M1 Foundation | ⏳ **in progress** (port plan §7 lists its parts; parts a and b1 are `1f718c6`). **Part a**: `src/error.cyr`, the `PMN_ERR_*` codes (the five Rust variants, then `OUT_OF_MEMORY` and `DESERIALIZE`), `pmn_err_name` and `pmn_err_message`; `[lib]` and `dist/pramana.cyr` start. **Part b1**: kimiya 2.0.1's correctly rounded exp, expm1, ln, pow, sin, cos and atan2 vendored by `scripts/vendor_crmath.py` (58 functions, 59 tables, `_pmn_km_` names); every golden row of the seven at 0 ulp (all 76 corrected rows accepted, nothing measured), identical on three targets; correctly rounded on 7.7M MPFR rows (`scripts/crverify/`), overflow and NaN results included; 8–14× glibc (46–136 ns). **Part b2**: a correctly rounded log1p of pramana's own (`src/crmath_ext.cyr`: kimiya's ln core with r = x below 2^-10, and ln(s) + log1p(t/s) on the exact split 1 + x = s + t above), at 0 ulp on all 1,013 golden rows; correctly rounded on 1.3M MPFR rows, 200,000 of them through the accurate path alone, which found and fixed a Veltkamp overflow past 2^996; 51–96 ns, 9–14× glibc (`f08f233`). **Part b3**: a correctly rounded tan of pramana's own (`src/crmath_ext.cyr`: kimiya's sin and cos of one reduction, divided in double-double after an exact renormalisation, which the golden rows showed was needed; triple-double in the accurate path), at 0 ulp on all 2,010 golden rows; correctly rounded on 1.3M MPFR rows, 200,000 through the accurate path alone; 188–190 ns, 17.4× glibc (`3257918`). **Part b4**: a correctly rounded log2 of pramana's own (log2 x = E + ln(m)/ln 2 on kimiya's ln, E exact, so the powers of two are exact), at 0 ulp on all 2,510 golden rows; correctly rounded on 1.3M MPFR rows, every power of two and 200,000 accurate-path rows included; 104–109 ns, 20.6× glibc. **Part b is complete** (`0c3f469`): every class-C math row is gated at 0 ulp, identical on three targets, and the ten primitives are correctly rounded on 11.6M MPFR rows. **Part c**: `src/mathx.cyr`, the Rust f64 surface (NaN-ignoring min / max / clamp, trunc-based fract, unsigned `u64 as f64`, saturating `as usize`, is_nan / is_finite / total_cmp, square-and-multiply powi) and the transcendentals' names; the 7,077 pending rows all pass, so the whole corpus is gated (35,633 ok, 76 accepted, 0 failed, measured or pending) on three targets; `tests/mathx.tcyr` adds 30 checks (`d13fb61`). **Part d1**: `src/fmtf.cyr`, impetus 2.0.0's serde_json f64 printer vendored by `scripts/vendor_fmtf.py`, and a golden `fmt` module (25,200 doubles of serde_json's own text, rebuilt independently by `validate.py`): all match byte for byte on three targets, and a tie-rule mutation fails 37 rows; 2.8–13.9 µs a value, 135–650× serde_json (a finding: the printer L10 asks for) (`d34db7c`). **Part d2**: `src/pool.cyr`, the work-table pool (eight CAS slots growing on the freelist, overflow claims as freelist blocks) and a bump allocator over a claim for bayan's `_a` calls, moved forward from part f for the JSON reader; `tests/pool.tcyr` (21 checks) and `tests/pool_threads.tcyr` (four racing threads, 15.8M words read back, all eight slots and the overflow taken), whose first run found a growth race (a slot won in the second scan doubled even when it already fit, to 64 GB) that P4's prototype had lost kimiya's check for; the release is a CAS (18–19 ns a claim, from 35–36 with a fence); the parity harness fails a module that leaves a slot claimed (`c302838`). **Part d3a**: `src/json.cyr`, the JSON read core for values: probe P7's pre-pass (ledger L11), bayan's tree in a pool arena that starts at 24× the text and doubles on the bump allocator's counted refusals (trees reach 88×), a document-size guard for `vec_push_a`'s capacity abort (ledger L13, filed), and readers for f64, u64/usize, bool, String (UTF-8 checked when read), Vec<f64>, Vec<u64>, Vec<String> and Vec<Vec<f64>>; a golden `json` module (3,886 documents, serde_json with `float_roundtrip`, 120 exact float midpoints among them) re-read by `validate.py`'s own serde model, all matching on three targets; `tests/json.tcyr` (77 checks); pool slots now capped at 1 MiB per claim; decodes 7–22× serde_json (a finding) (`c61f76b`). **Part d3b**: serde's struct rules (`_pmn_json_struct`: unknown members skipped, a field twice or missing refused, every key's UTF-8 checked, the sequence form exact) and the two enum forms (`_pmn_json_unit_variant`, `_pmn_json_newtype_variant`), against serde_json on two derive test structs and 1.3.0's `Kernel` and `PramanaError` (839 more golden rows, 4,725 in all); `validate.py` gains serde's ignore path and a model of bayan's three whole-document refusals, which writes `json.map.tsv`: 20 rows ADR 007 row 16 maps to the refusal, all passing as accepted on three targets; `tests/json.tcyr` 104 checks; a Normal document decodes in 1.65 µs (21× serde_json) (`18c4e70`). **Part d4**: the JSON write core (serde_json's compact text for every shape: fmtf's f64s, unsigned decimals, serde_json's string escapes, vectors, row-major matrices, struct members, both enum forms), each writer returning `OUT_OF_MEMORY` rather than aborting, by reserving room before the stdlib writers that have no non-aborting form (ledger L14, filed); the error codec, `pmn_err_to_json` (an empty message; NONE and the Cyrius-only codes refused, ADR 007 row 8 as amended by the maintainer) and `pmn_err_from_json` (the message read, then dropped); 1,484 more golden rows (1,382 serde_json writes and 102 code-only reads, 6,209 in all), re-checked by `validate.py`'s own writer model, all matching on three targets; `tests/json.tcyr` 137 checks with round trips of 6,300 seeded values. Part e (the RNG with its `SimpleRng` codec, combinatorics, the bridges) next |
| M2–M8 | not started |

## Toolchain

- **Cyrius pin**: 6.7.18 (`cyrius.cyml` `[package].cyrius`)
- **ganita**: 1.2.15, bundled in the 6.7.18 stdlib
- **hisab**: none. The four hisab 1.4.0 routines pramana used are transcribed in M3–M5 (port plan §8.2)
- **Rust oracle**: rustc 1.100.0 (`rust-old/rust-toolchain.toml`), hisab 1.4.0 (`rust-old/Cargo.lock`), glibc 2.44

## Source

- **Rust oracle**: 4,454 library code lines in `rust-old/src/` (14 modules plus `integration/soorat.rs`, 102 `pub fn`s
  plus 65 trait methods), and the `oracle` hooks. Frozen; never edited.
- **Cyrius**: `[lib]` is, in order:
  - `src/error.cyr`;
  - the generated `src/constants.cyr` (112 constants);
  - the vendored `src/crmath_data.cyr` and `src/crmath.cyr` (kimiya 2.0.1's correctly rounded primitives: 58
    functions, 59 tables), and `src/crmath_ext.cyr` (pramana's correctly rounded log1p, tan and log2);
  - `src/mathx.cyr` (the Rust f64 surface);
  - the vendored `src/fmtf.cyr` (impetus 2.0.0's serde_json f64 printer, 15 items);
  - `src/pool.cyr` (the work-table pool and the bump allocator);
  - `src/json.cyr` (the JSON read core: the pre-pass, the document arena and the value readers).

  They are bundled as `dist/pramana.cyr`. `src/main.cyr` is the scaffold's build entry.

## Tests

- **Rust oracle**: run from inside `rust-old/` with `cargo test --all-features`: 301 unit, 15 integration, 11
  oracle and 3 doc tests (330).
- **Golden corpus** (`tests/golden/`): `constants.tsv` 112 rows, `fmt.tsv` 25,200 rows (serde_json's f64 text),
  `json.tsv` 6,209 rows (serde_json's reading and writing of each value type, two test structs and 1.3.0's two
  enums, and the error codec's reading),
  `json.map.tsv` 20 rows (ADR 007 row 16), `math.tsv` 35,597 rows and `math.cr.tsv` 76 rows. They are checked locally by
  `(cd scripts/golden && cargo run --release --locked -- --check)` and `python3 -I -B scripts/golden/validate.py .`.
- **Cyrius**: `tests/parity.tcyr` replays the whole corpus, 4 modules (67,022 rows ok, 76 accepted under ADR 9 and
  20 under ADR 7,
  nothing failed, measured or pending), and fails a module that leaves a pool slot claimed; `tests/error.tcyr` and
  `tests/mathx.tcyr` (30 checks each); `tests/pool.tcyr` (26) and `tests/pool_threads.tcyr` (6, four racing
  threads); `tests/json.tcyr` (137); `tests/crmath.bcyr`, `tests/mathx.bcyr`, `tests/fmtf.bcyr`, `tests/pool.bcyr` and
  `tests/json.bcyr` (with the Rust twin
  `scripts/probes/rust/src/bin/m1_primitives.rs`); the scaffold's `tests/pramana.{tcyr,bcyr,fcyr}` stubs.
- **Correct rounding**: `bash scripts/crverify/run.sh` checks 11.6M seeded rows of the primitives against MPFR on
  three targets; `scripts/crverify/consts.py` checks `src/crmath_ext.cyr`'s constants (in CI too).

## Accuracy baseline

Rust 1.3.0 against MPFR 4.2.2: [port plan §3.2](pramana-port-plan.md), reproduced by
`scripts/port-baseline/run.sh`.

## Upstream requests and stand-ins

The conversion ledger is the port plan's §10: 14 rows (L1–L14), all open, each naming its upstream request, and four
requests filed with no stand-in. `python3 -I -B scripts/upstream_ledger.py --check` gates it (CI too): 11 `UPSTREAM`
notes in `src/` so far, 0 failures. L13 (M1 part d3a) is the JSON reader's guard against `vec_push_a`'s capacity
abort, added to the open cyrius vec issue; L14 (part d4) the JSON writers' reservations before the stdlib writers
that abort on a failed grow, added to the open cyrius str_builder issue. The rest of the stand-ins arrive with M1–M7.

## Dependencies

Direct (declared in `cyrius.cyml`; 51 files locked in `cyrius.lock`):

- stdlib: syscalls, string, alloc, str, fmt, io, vec, math, ganita, overflow, tagged, result, freelist, fnptr, atomic,
  bayan
- test stdlib (`[test] stdlib`, test / bench / fuzz builds only): assert, bench, thread (for `tests/pool_threads.tcyr`)
- no hisab (port plan §8.2); sandhi joins only for the `ai` code, outside the core bundle (D8)
- `[build] dce = true`: the skeleton builds to 407,872 bytes

## Consumers

- **Cyrius:** none declares a pramana dependency yet. `ai` will talk to hoosh 2.12.3 (Cyrius) over its HTTP API.
- **Rust (`pramana = "1"`, optional):** bodh, jivanu and sangha, all at 1.0.0. They stay on crates.io 1.x until they
  port.

## Next

M1 part e (port plan §7): the RNG (`PmnSimpleRng`, the `PmnRng` trait) with its `SimpleRng` codec (a zero state maps
to 1), combinatorics with `PmnU128`, and the 11 bridge functions. Then part f, M1's gate.
