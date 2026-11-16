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
