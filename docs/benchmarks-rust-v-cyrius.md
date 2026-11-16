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
