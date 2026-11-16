# scripts/golden: what the port derives from the Rust oracle

`golden` drives the frozen Rust pramana 1.3.0 in [`rust-old/`](../../rust-old/), with its hidden `oracle` hooks (port
plan §5 step 0), and writes two kinds of output:
- **The golden corpus, `tests/golden/<module>.tsv`** (port plan §5 step 5). It has one row per call, with every float
  as its IEEE-754 bit pattern. The Cyrius parity harness (`tests/parity.tcyr`, from M0 step 7) replays every row
  against the port and checks it within its tolerance class.
- **The generated Cyrius sources**: `src/constants.cyr` from step 6, and data modules as milestones need them. No
  value the port reads is typed by hand.

`validate.py` re-checks the corpus without the oracle. It also writes the correctly rounded rows,
`tests/golden/<module>.cr.tsv`, from MPFR (`mpfr_ref.py`).

```sh
cd scripts/golden
cargo run --release --locked                  # write
cargo run --release --locked -- --check       # verify; exit 1 on any difference
cd ../..
python3 -I -B scripts/golden/validate.py .              # independent re-check, and the .cr.tsv files
python3 -I -B scripts/golden/validate.py . --write-cr   # rewrite the .cr.tsv and .map.tsv files
python3 -I -B scripts/golden/mpfr_ref.py                # MPFR binding self-checks
python3 -I -B scripts/golden/probe_tables.py .          # compile src/constants.cyr; every value against its row
(cd scripts/golden && GOLDEN_NUDGE='math::fdlibm::RB3' cargo run --release --locked)   # mutation hook: never writes
```

Run cargo **from this directory**: rustup reads `rust-toolchain.toml` (the recording toolchain, 1.100.0) from the
current directory, not from `--manifest-path`.

These are **local gates, not CI**. The corpus holds glibc 2.44's bits, and a runner with another glibc is refused by
design.

The format, the comparison rules and the generator's structure are pravash's (`/home/macro/Repos/pravash`, at its
tag 2.0.0, `scripts/oracle/`), which are impetus's, kimiya's and tanmatra's: every port's harness reads the same
files. This README records what is specific to pramana.

## Rules

- **`rust-old/` is never built in place or edited.** This crate depends on it by path, with `oracle` and
  `soorat-compat` on. `ai` is off: its rows use a recorded transport. `Cargo.lock` was seeded from
  `rust-old/Cargo.lock`, so every shared dependency is the version the oracle was tested with (hisab 1.4.0).
- **The output is deterministic.** Two runs give byte-identical files. Grids use only + − × ÷, `sqrt` (correctly
  rounded by IEEE 754, so the same everywhere; from M2 part a3), `floor`, decimal parsing, bit construction and seeded
  xorshift. Never a transcendental std method, a timestamp or a hash-map order.
- **The provenance is pinned.** Every header records the oracle, hisab, the rustc that built the generator
  (`build.rs`), and the libm platform (`glibc.rs`: `glibc 2.44 x86_64 fma`). `--check` refuses a corpus recorded
  under any other provenance, naming both, because the rows hold that compiler's and that glibc's bits.
- **Where the transcendentals come from** (`nm -D` on the generator, rustc 1.100.0):

  | Source | Functions | Rows |
  |---|---|---|
  | glibc's libm (dynamic) | `exp`, `log` (`ln`), `log1p` (`ln_1p`), `expm1` (`exp_m1`), `pow` (`powf`), `sin`, `cos`, `tan`, `atan`, `log2` | glibc's bits; `validate.py` recomputes them through ctypes |
  | compiler_builtins (linked into the binary) | `cbrt` | platform-free; `validate.py` proves every one correctly rounded |
  | inline code | `sqrt`, `abs`, `floor`, `fract`, `min`, `max`, `clamp`, `is_nan`, `is_finite`, `total_cmp`, `powi` with a literal exponent, the casts, the fdlibm word operations | exact; `validate.py` recomputes them in Python |

- **`fmt.tsv`** (M1 part d1, `src/m_fmt.rs`): serde_json 1.0.154's text for 25,200 doubles: every power of two and
  of ten with its neighbours, the integers to 1,000, short decimals, a log-uniform sweep across the form switches,
  and seeded bit patterns. pramana's JSON writes this text byte for byte (port plan D5). serde_json is a dependency
  here, pinned to `rust-old/Cargo.lock`'s version. `validate.py` rebuilds every row from Python's own shortest
  digits (`repr`, ties to even as serde_json's zmij), and 62 of the inputs are exact ties.
- **`json.tsv`** (M1 parts d3a, d3b and d4, `src/m_json.rs`), 6,209 rows. The reading rows are
  `serde_json::from_slice::<T>`, with `float_roundtrip` on, on 4,725 documents. T is one of:
  - each value type 1.3.0's serialized types are made of (`f64`, `u64`, `usize`, `bool`, `String`, `Vec<f64>`,
    `Vec<u64>`, `Vec<String>`, `Vec<Vec<f64>>`);
  - two test structs, `Pair` and `Shapes` (serde derive, with `serde` pinned to `rust-old/Cargo.lock`'s version), for
    derive's struct rules;
  - 1.3.0's two enums, `Kernel` (unit variants) and `PramanaError` (newtype variants).

  The input is the document's bytes (`s:`, which need not be UTF-8), and the output the value's tokens or
  `E:Deserialize`, serde_json's refusal. `validate.py` re-reads every document with its own model of serde_json's
  reading, written from serde_json's source.

  Part d4 adds:
  - **the error codec's reading** (`json::PramanaError::code`, 102 rows): the variant alone, the message dropped
    (ADR 007 row 8);
  - **the writing rows** (`json::write::<T>`, 1,382 rows): `serde_json::to_vec` of seeded values of every shape, the
    value's tokens in and serde_json's text out. `PramanaError`'s variants are written with an empty message, as
    pramana writes a code (ADR 007 row 8, amended). `validate.py` re-writes each with its own model of serde_json's
    writing: the escape table, the f64 text model it checks `fmt.tsv` with, and the struct and enum forms.
- **`debug.tsv`** (M1 part f, `src/m_debug.rs`): Rust's derived `Debug`, `{:?}` and `{:#?}` (D12), of 1.3.0's types
  as each milestone ports them: in M1, `SimpleRng` from 25 seeds and each `PramanaError` variant with an empty
  message. The second input is 0 for `{:?}` and 1 for `{:#?}`. `validate.py` models the derive's struct and tuple
  text.
- **`bridge.tsv`** (M1 part e3, `src/m_bridge.rs`): the eleven bridges on special-value grids, the 1e-15 floors and
  their neighbours, empty, single, mismatched and NaN-bearing slices, the fits' skipped records, and seeds. Class B,
  except the likelihood and the two fits (class C). A slice of pairs is `v:n` then each pair's `i:` and `f:` tokens;
  a tuple is its elements in order. `validate.py` models all eleven in Rust's operation order, through glibc and
  MPFR.
- **`combinatorics.tsv`** (M1 part e2, `src/m_combinatorics.rs`): `factorial`, `permutations` and `combinations` on
  probe P1's grid (n from 0 to 40, the u128 edge 130–133, 2³², 2⁶³, u64::MAX, each against an r grid, and 200 seeded
  pairs), and `stirling_approx` (class C) on 315 n. A u128 result is two `i:` tokens, its low word then its high
  word. `validate.py` recomputes the counts with exact integers in 1.3.0's checked loops, Stirling through glibc's
  pow, and its correctly rounded rows through MPFR's.
- **`rng.tsv`** (M1 part e1, `src/m_rng.rs`): 1.3.0's `SimpleRng`: the first 256 draws from 40 seeds, and 10⁶-draw
  FNV-1a digests from six. `validate.py` re-draws each with its own xorshift. `json.tsv` gains SimpleRng's codec
  rows in the same part.
- **`special.tsv`** (M2, `src/m_special.rs`): 1.3.0's `src/math.rs` through the `oracle` hooks, its rows named by
  Rust's path, `math::<fn>`. Part a1 records `erf` and `erfc` at every high word fdlibm branches on, with the five
  doubles either side and 24 seeded low words in the high word at and below it, both signs, then seeded sweeps of
  each branch, tiny and wide sweeps and k/32 on [−30, 30]; and `normal_cdf` and `normal_sf` on that grid times √2,
  k/16 on [−40, 40] and seeds. Part a2 records `ln_gamma` (its switch at ½, its 1e-300 cutoff near 0, the negative
  integers and half-integers, the roots, sweeps to ±10³⁰⁶), `stirling_error` (its switch at 10), `bd0` and
  `ln_gamma_kernel` on x = m·r and x = a·r for fixed ratios over seeded scales, and `ln_beta` on a pairwise shape grid
  from 10⁻³ to 10¹², seeds, and NaN, ±0 and negative shapes. Part a3 records `gamma_inc` (P and Q, two outputs) at
  its switches x = a + 1 and a = 10⁹, near the mode x = a ± j√a, on fixed ratios x = a·r over seeded shapes, for the
  χ² shapes k/2, for shapes from 10⁻³⁰⁰ to 10⁻⁸ (where the series' clamp binds), and where the prefactor underflows;
  and the two regularized wrappers on seeds. Part a4 records `beta_inc` (I and 1 − I from a, b, x and y) on a pairwise
  shape grid against eleven x, at its side switch, at the mean ± j standard deviations for two large shapes, in its
  callers' forms with x and y each computed exactly (Student's t, F), with a y that is not 1 − x, and for shapes from
  10⁻³⁰⁰; and `regularized_incomplete_beta` on seeds and out-of-range x. All class C. `validate.py` transcribes each function and re-derives
  every row through glibc's exp, ln, log1p and sin (and the correctly rounded cbrt, as Rust's compiler_builtins
  gives); `special.cr.tsv` holds the rows that move when they are correctly rounded.
- **`json.map.tsv`** (ADR 007 row 16), written by `validate.py --write-cr`. These are the rows serde_json reads only
  by skipping a member unchecked, which bayan refuses whole: a duplicate key, nesting past 128 levels, or a lone
  surrogate. `validate.py` finds them with its own model of bayan's three whole-document refusals. They map to
  `E:Deserialize` and pass as accepted (ADR 7).
- **The constants come from the source, and the tables are proven** (`src/consts.rs`, `src/p_constants.rs`).
  - **Why from the source:** pramana's constants are private, so no API exposes them. The generator reads every
    `const` item in the library code, function-local tables included, through Rust's own literal parser, and proves
    the list complete against a plain count.
  - **The proofs:** before writing anything, it recomputes the oracle's functions with the extracted values and
    compares bit for bit with the oracle's hooks:
    - the fdlibm table against `erf` and `erfc`;
    - Lanczos, `G` and `LN_SQRT_2PI` against `ln_gamma`;
    - Acklam, `P_LOW` and `P_HIGH` against `z_quantile`;
    - the KS constants against `kolmogorov_sf`;
    - the two sampler thresholds by their draws.

    A failed proof writes nothing.
  - **The mutation hook:** `GOLDEN_NUDGE=<path>` moves one extracted f64 by an ulp, reports which proof catches it,
    and never writes.
  - **Two more checks:** `validate.py` re-reads every constant with its own Python extractor, and `probe_tables.py`
    compiles `src/constants.cyr` and checks each value's bits.
- **The corpus is regenerated only from the oracle.** A wrong oracle value is fixed in Rust as 1.3.x, then
  regenerated. It is never patched in a `.tsv`.
- **Modules arrive with their milestone.** Each one starts by adding its golden emitter (`src/m_<module>.rs`, with
  the grid from the port plan §7) and registering it in `src/main.rs`. Its `validate.py` rules come in the same part.
  The special-function rows (`special.tsv`) arrive with M2 part a, and their reference rows (`special.ref.tsv`) with
  part b.
- **`special.ref.tsv`** (M2 part b, ADR 009's T1), written by `validate.py --write-cr`: for each `special` row a T1
  function moves, its true value (MPFR), or downstream Rust's formula on the T1 functions' values as the port computes
  them, and correctly rounded primitives. `validate.py` models every row three ways (glibc, correctly rounded primitives, and the
  reference) and writes a moved row to `.cr.tsv` when only a primitive's rounding moves it, else here. The harness
  compares a reference row under the function's `_par_c_bound`, ADR 009's bound for a T1 function (0 for erf and
  erfc, from part b1; 1 for Φ and 1 − Φ, from part b2, whose true values are `mpfr_ref.py`'s `ncdf` and `nsf`:
  ½·erfc(∓z/√2) at 1,024 bits, rounded once; 2 for lnΓ, from part b3, mixed for x < 0 through `_par_c_mixed`).
  A row downstream of a T1 function is Rust's formula on the T1 function's own value (ADR 009 as amended
  2026-11-16), so `validate.py` models the T1 functions that aren't correctly rounded as the port computes them:
  - `t1_ln_gamma`, P9's lnΓ on MPFR's double-double logarithms (`mpfr_ref.ln_dd`) and libm 0.2.16's coefficients;
  - `t1_gamma_inc` (part b4), src/gammainc.cyr step for step, on kimiya's Dekker product, MPFR's double-double
    logarithms and exponentials (`mpfr_ref.ln_dd`, `mpfr_ref.exp_dd`, with the exp core's exponent taken as the port
    takes it) and MPFR's γ and ζ(k) (`mpfr_ref.zeta_table`). It feeds the two regularized wrappers;
  - `t1_beta_inc` (part b5), src/betainc.cyr step for step on the same toolkit, with Rust's form (`sp_beta_inc`) past
    max(a, b) = 1e9. It equals the port on all 6,219 golden inputs in T1's range, and feeds
    `regularized_incomplete_beta`.

  The incomplete gamma's own rows (part b4) take their true P and Q from `mpfr_ref.gamma_pq`: check.c's 320-bit
  series or continued fraction through ctypes, wider below a = 2^-10. A row at a = 1e9 takes about 10 s, so
  `validate.py` runs them in a process pool and caches them in `build/golden/`, under a hash of the reference's source
  and the MPFR version. The first run takes a few minutes; later ones read the cache. The harness's bound for them is
  `_par_c_gamma_bound`: 4 ulp for a in [1e-3, 1e5], 0 for the edges and the Wilson–Hilferty rows past 1e9.

  The incomplete beta's own rows (part b5) take their true I and 1 − I from `mpfr_ref.beta_ic`: check.c's continued
  fraction through ctypes, with x and y held exactly (the smaller as given, the other as 1 − it) and the precision grown
  for a tiny complement. `validate.py` pools and caches them as for `gamma_inc` (5,714 inputs, about 50 s). The
  harness's bound is `_par_c_beta_bound`: 8 ulp for a and b in [1e-3, 1e5], 0 for the edges, an infinite shape and the
  rows past 1e9, where Rust's form stays. In between, as for `gamma_inc` outside its domain, `_par_c_vs_rust` holds each
  output to 1.3.0's own error.

| Module | Milestone | Golden rows | Golden emitter | Generated Cyrius / correctly rounded rows |
|---|---|---|---|---|
| `constants` | M0 | 112 | `src/m_constants.rs` (from `src/consts.rs`, proven by `src/p_constants.rs`) | `src/constants.cyr` and the harness's lookup `tests/constants_rows.cyr` (`src/t_constants.rs`) |
| `math` | M0 | 35,597 | `src/m_math.rs` | `math.cr.tsv`, 76 rows |
| `fmt` | M1 d1 | 25,200 | `src/m_fmt.rs` | — |
| `json` | M1 d3a–d4, e1 | 6,399 | `src/m_json.rs` | `json.map.tsv`, 30 rows (ADR 007 row 16) |
| `rng` | M1 e1 | 46 | `src/m_rng.rs` | — |
| `bridge` | M1 e3 | 5,553 | `src/m_bridge.rs` | `bridge.cr.tsv`, 0 rows (glibc's exp, ln and log2 are correctly rounded on every input) |
| `debug` | M1 f | 60 | `src/m_debug.rs` | — |
| `combinatorics` | M1 e2 | 2,405 | `src/m_combinatorics.rs` | `combinatorics.cr.tsv`, 0 rows (glibc's pow is correctly rounded on every Stirling input) |
| `special` | M2 a1–a4, b1–b5 | 58,101 | `src/m_special.rs` | `special.ref.tsv`, 20,621 rows (b1: erf and erfc at their true values, 1,074; b2: Φ and 1 − Φ at theirs, 5,574; b3: lnΓ at its, 2,441; b4: the incomplete gamma's P and Q at theirs, 3,669; b5: the incomplete beta's I and 1 − I at theirs, 4,834; and the rows downstream of them on T1's values, 3,029, among them gamma_inc's 510 Wilson–Hilferty rows and the three wrappers' 777); `special.cr.tsv`, 91 rows (glibc's log1p, exp or ln misrounded in bd0, the kernel and ln B, 87; two `beta_inc` rows whose true value is Rust's form on correctly rounded primitives; one Wilson–Hilferty `gamma_inc` row; and part a1's last erfc row, where fdlibm on a correctly rounded exp already gives the true erfc) |

## Format 1

Five header lines, then one row per call:

```text
fn <TAB> class <TAB> inputs <TAB> outputs <TAB> note
```

- **Header:**
  1. `# pramana golden corpus, format 1: <module>`
  2. `# oracle: <provenance>. Generated by scripts/golden; do not edit.`
  3. the regenerate command
  4. the column legend
  5. `# rows: <n>`

  A `.cr.tsv` file (the siblings' correction format, which `tests/parity_core.cyr` reads) also has five header lines,
  naming the provenance of the rows it corrects and MPFR. Its rows have four columns: `fn <TAB> inputs <TAB> outputs
  <TAB> note`. The fn and inputs are the golden row's, byte for byte, and are the key.
- **Tokens** (space-separated; `-` when a list is empty):
  - `f:<16 hex>`: an f64 as its bit pattern, lowercase;
  - `i:<decimal>`: an integer, or a boolean as 0 / 1. A 64-bit word ≥ 2⁶³ (a u64, or `as usize` of a huge value) reads
    negative;
  - `e:<Enum>.<Variant>`: an enum variant;
  - `s:<percent-encoded UTF-8>`: a string (every byte outside printable ASCII, and `%`, as `%XX`);
  - `n:`: `None`;
  - `E:<Variant>`: `Err(PramanaError::<Variant>)`, with the payload not compared. `E:Deserialize` is serde_json's own
    refusal, which pramana reports as `PMN_ERR_DESERIALIZE` (port plan D6);
  - `v:<len>`: a sequence, whose element tokens follow;
  - a u128 is two `i:` tokens, its low word then its high word.
- **Classes** (port plan §6):
  - **A** exact (NaN matches any NaN);
  - **B** ≤ 2 ulp, expected 0;
  - **C** transcendental: 0 ulp against the `.cr.tsv` row where one exists, otherwise against this row;
  - **D** relative ≤ 1e-12, or a bound set before the first comparison;
  - **S** streams and **R** references arrive with their modules.
- **The note** is a human rendering of the same call, for review only; the harness ignores it. A `.cr.tsv` note
  also says how far off glibc was.

## validate.py

Standard library plus ctypes. It never runs the Rust oracle:
- it parses every file (header, row count, five fields, classes, tokens), and checks that `modules.txt` lists
  exactly the generator's files;
- it recomputes every class-A/B `math` row in Python, in Rust's operation order. That includes LLVM's
  square-and-multiply `powi`, Rust's `fract`, and the saturating `as usize`;
- it recomputes every glibc class-C row by calling the same glibc function through ctypes;
- it proves every `cbrt` row correctly rounded with MPFR;
- it rebuilds every `fmt` row from Python's shortest digits, and reads every `json` document with a model of
  serde_json 1.0.154's reading (its strict grammar, `-0` as a float, Rust's `as f64`, surrogate pairing, UTF-8 by
  Python's strict decoder, every float by Python's correctly rounded `float`);
- it computes the correctly rounded value of every class-C row with MPFR, and writes (`--write-cr`) or checks the
  rows where glibc's differs.

**Tamper checks (2026-11-15):**
- a one-bit change to an `exp` row fails it and `--check`;
- an edited `.cr.tsv` row fails it;
- a header naming glibc 2.45 is refused by `--check` with a `PROVENANCE` line.
