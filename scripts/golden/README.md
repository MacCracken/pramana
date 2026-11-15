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
python3 -I -B scripts/golden/validate.py . --write-cr   # rewrite the .cr.tsv files
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
- **The output is deterministic.** Two runs give byte-identical files. Grids use only + − × ÷, `floor`, decimal
  parsing, bit construction and seeded xorshift. Never a transcendental std method, a timestamp or a hash-map order.
- **The provenance is pinned.** Every header records the oracle, hisab, the rustc that built the generator
  (`build.rs`), and the libm platform (`glibc.rs`: `glibc 2.44 x86_64 fma`). `--check` refuses a corpus recorded
  under any other provenance, naming both, because the rows hold that compiler's and that glibc's bits.
- **Where the transcendentals come from** (`nm -D` on the generator, rustc 1.100.0):

  | Source | Functions | Rows |
  |---|---|---|
  | glibc's libm (dynamic) | `exp`, `log` (`ln`), `log1p` (`ln_1p`), `expm1` (`exp_m1`), `pow` (`powf`), `sin`, `cos`, `tan`, `atan`, `log2` | glibc's bits; `validate.py` recomputes them through ctypes |
  | compiler_builtins (linked into the binary) | `cbrt` | platform-free; `validate.py` proves every one correctly rounded |
  | inline code | `sqrt`, `abs`, `floor`, `fract`, `min`, `max`, `clamp`, `is_nan`, `is_finite`, `total_cmp`, `powi` with a literal exponent, the casts, the fdlibm word operations | exact; `validate.py` recomputes them in Python |

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
  The special-function rows (`special.tsv`) and their MPFR reference rows (`special.ref.tsv`) arrive with M2.

| Module | Milestone | Golden rows | Golden emitter | Generated Cyrius / correctly rounded rows |
|---|---|---|---|---|
| `constants` | M0 | 112 | `src/m_constants.rs` (from `src/consts.rs`, proven by `src/p_constants.rs`) | `src/constants.cyr` (`src/t_constants.rs`) |
| `math` | M0 | 35,597 | `src/m_math.rs` | `math.cr.tsv`, 76 rows |

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

  A `.cr.tsv` file has the same five lines, naming MPFR and the provenance of the rows it corrects.
- **Tokens** (space-separated; `-` when a list is empty):
  - `f:<16 hex>`: an f64 as its bit pattern, lowercase;
  - `i:<decimal>`: an integer, or a boolean as 0 / 1. A 64-bit word ≥ 2⁶³ (a u64, or `as usize` of a huge value) reads
    negative;
  - `e:<Enum>.<Variant>`: an enum variant;
  - `s:<percent-encoded UTF-8>`: a string (every byte outside printable ASCII, and `%`, as `%XX`);
  - `n:`: `None`;
  - `E:<Variant>`: `Err(PramanaError::<Variant>)`, with the payload not compared;
  - `v:<len>`: a sequence, whose element tokens follow.
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
- it computes the correctly rounded value of every class-C row with MPFR, and writes (`--write-cr`) or checks the
  rows where glibc's differs.

**Tamper checks (2026-11-15):**
- a one-bit change to an `exp` row fails it and `--check`;
- an edited `.cr.tsv` row fails it;
- a header naming glibc 2.45 is refused by `--check` with a `PROVENANCE` line.
