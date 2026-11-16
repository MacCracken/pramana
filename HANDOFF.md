# Handoff: pramana's Cyrius port, M2 part b4 done → part b5 next

Written 2026-11-16 at the end of a session that ran M2 parts a2–b4. Read this, then `CLAUDE.md`,
`docs/development/state.md` and the M2 sections of `docs/development/port-parity.md` (§8.1–§8.8). Delete this file
once you've taken over.

## 1. Where things stand

- **The last commit is `7628c0c` ("m2 b3 done").** Part b4 (the incomplete gamma within 4 ulp) is complete and
  verified, but **not committed**. The maintainer commits between parts; never commit or push yourself (CLAUDE.md).
- **b4's working-tree changes** (all intended):
  - new: `src/dd.cyr`, `src/gammainc.cyr`, `src/zeta_data.cyr`, `scripts/gen_zeta.py`, `scripts/crverify/rows_gamma.cyr`;
  - modified: `src/special.cyr`, `src/lgamma.cyr`, `cyrius.cyml`, `tests/lib.cyr`, `dist/pramana.cyr`,
    `tests/parity.tcyr`, `tests/parity_core.cyr`, `tests/special.tcyr`, `tests/golden/special.{cr,ref}.tsv`,
    `scripts/golden/{validate.py,mpfr_ref.py,README.md}`, `scripts/port-baseline/check.c`,
    `scripts/crverify/run.sh`, `.github/workflows/ci.yml`, and the docs (CHANGELOG, plan, state, roadmap,
    port-parity §8.8, the benchmarks record).
- **Gates at hand-off:** every one is green on x86_64 ELF, PE (wine) and aarch64 (qemu), and every `math::` result line
  is identical on the three.
  - Parity replay: 9 modules, 115,965 rows ok and 17,508 accepted (ADR 9: 17,478, of which 17,230 are reference rows;
    ADR 7: 30); nothing failed, measured or pending.
  - `tests/special.tcyr`: 105 checks. `cyrius test` is warning-free.
  - `validate.py`, golden `--check`, the ledger (14 rows, 17 `UPSTREAM` notes), surface parity, and every vendor and
    constant check, `gen_zeta.py --check` included.
- **`crverify` shas** (`scripts/crverify/run.sh <set>`): `lgamma` b9b77d894d78127f, `gamma` 0714f60026574372.

## 2. How the maintainer works

- They drive with short commands: "Start b5", then "Start c". One part per command, and they commit in between.
- Each part ends with `mcp__ccd_turn__wrap_up`, a recap plus the ask "Start part <next>?" and one button.
- Ask (AskUserQuestion) only for a decision that is genuinely theirs. In b3 that was how to gate rows downstream of a
  T1 function, which became ADR 009's amendment. Otherwise decide, and say what you decided.
- They expect progress notes during long runs, and asked in b4 that background jobs never finish or stall unseen. Run
  long jobs with `run_in_background`, and put a `Monitor` on them that reports progress and every terminal state.
- Write docs in the house style: short sentences, bullets for lists, numbers with their reference. Every accuracy claim
  carries measured ulps and its reference, and every performance claim carries benchmark numbers.

## 3. The per-part procedure (what "done" meant for b1–b4)

1. **Read** the plan's part line (§7 "M2's parts"), P9's probe record (port-parity §6.x) and its prototype under
   `scripts/probes/`, and ADR 009's bound and domain for the function.
2. **Port** the prototype into `src/`, unchanged but for names (`_pmn_<prefix>_`), on `src/dd.cyr`. Add new files to
   `[lib]` in `cyrius.cyml` **and** to `tests/lib.cyr`, in the same order. Constants are generated, never typed:
   inline hex words need a cited `# 0x1pN` and a `--check` that proves them (see `scripts/gen_zeta.py`).
3. **Compare with P9 byte for byte.** A `crverify` row file reproduces P9's seeded sets from the same seed and order,
   and its rows must equal P9's own output (`build/p9/*_rows.txt`). That is the proof of the transcription.
4. **Go past P9's sets.** Every b-part so far found bugs P9's sets never reached:
   - b3: lnΓ overflowed into a NaN;
   - b4: two Dekker-split overflows, and a series stop that cost 7 ulp at a = 10⁹.

   Always add sets out to the extremes (subnormal arguments, the largest shapes, x near 0, 1 and f64::MAX), and
   compare the port with a NaN-aware reference.
5. **The reference rows and the model.**
   - `scripts/golden/mpfr_ref.py` gains the function's true value.
   - `scripts/golden/validate.py` gains a model of the port, step for step, used for the rows downstream of a T1
     function. The rule is ADR 009 as amended at b3: downstream rows are Rust's formula on the port's T1 value, at 0
     ulp; a T1 function's own rows are its true value, at its bound.
   - Run `python3 -I -B scripts/golden/validate.py . --write-cr`, then without the flag to check.
6. **The harness** (`tests/parity.tcyr`):
   - `_par_c_bound` gives the function's ADR 009 bound;
   - `_par_c_mixed` applies the mixed bound (lnΓ, x < 0);
   - `_par_c_vs_rust` applies ADR 009's rule outside the committed domain, "no row less accurate than Rust", row by
     row (b4; it needs `_par_has_alt` from `tests/parity_core.cyr`).
7. **Unit tests** in `tests/special.tcyr`, against MPFR values given as bit patterns. Rewrite any check that pinned
   1.3.0's behaviour T1 changes, and add one for each bug found.
8. **Mutation checks:** copy and adapt `build/handoff/mut_b4.py`. It mutates the source, runs the replay, restores it,
   and prints the failed rows per function. Record every mutant, including those that pass and why.
9. **`crverify`:** a new set in `scripts/crverify/run.sh`, run on three targets, ≥ 50k rows for the incomplete
   functions (ADR 009).
10. **Gates:** run `build/handoff/gates.sh` (ELF: build, test, bounds, fmt, lint, vet, distlib, deps, audit, ledger,
    surface, vendor and constant checks, golden `--check`, `validate.py`), and in parallel `cyrius test --win` and
    `cyrius test --aarch64`. Then compare the `^  math::` lines of the three logs (md5).
11. **The bench:**
    - one unpinned `cyrius bench tests/special.bcyr` into `<dir>/bench_cy1.txt`, plus the Rust twin
      (`(cd scripts/probes/rust && ./target/release/m2_special)`) into `<dir>/bench_rs1.txt`;
    - then `python3 -I -B build/handoff/bench_digests.py <dir>`, which must print "all digests explained";
    - then three rounds, interleaved and pinned to core 5 (`taskset -c 5`), each started only when the load average
      is below 3. The dev box is shared and other projects' jobs spike it.
12. **The docs:**
    - port-parity §8.N;
    - the benchmarks record section;
    - a CHANGELOG entry under `[Unreleased] / Added`, as "M2 part bN: …";
    - the plan: the status header (around line 36), the §3.5 accuracy row, the §7 part line ✅, and ledger row L8
      for any new `src/` file;
    - state.md: the M2 row, Source, Tests, the crverify list, the UPSTREAM count and Next;
    - the roadmap line;
    - the golden README's paragraph and table row;
    - CI, for any new `--check` script;
    - memory (`~/.claude/projects/-home-macro-Repos-pramana/memory/`).

## 4. Next: part b5, `beta_inc` within 8 ulp (P9 step 9d-3)

- **ADR 009:** I and 1 − I within 8 ulp for a, b ∈ [10⁻³, 10⁵] and x ∈ [0, 1]. For larger shapes, no row less
  accurate than Rust. Extend `_par_c_vs_rust` to `math::beta_inc` with the same per-output rule.
- **The prototype:** `scripts/probes/p9_beta_impl.cyr`, with the T1 functions from `p9_digamma` and `p9_stirling_dd`
  through `p9_t1_beta_inc`, plus `p9_beta.cyr` (the seeded sets), `p9_beta.sh` and `p9_beta_exact.py` (exact
  rationals). The record is port-parity §6.6: within 0.51 ulp on 214,744 rows, 10–17× T0.
- **Reuse:**
  - `src/dd.cyr`;
  - `_pmn_gi_lngamma_small_dd` in `src/gammainc.cyr`, whose comment already says beta calls it below 20;
  - the ζ table in `src/zeta_data.cyr`.

  P9's beta prototype has its own Stirling double-double for arguments of 20 and up; port that too.
- **The current T0 code:** `_pmn_beta_inc`, `_pmn_beta_cf` and `_pmn_regularized_incomplete_beta` in
  `src/special.cyr` (part a4). Retire the T0 continued fraction as b3 and b4 retired theirs. `_pmn_iteration_budget`
  may then lose its last caller; check before you delete it.
- **Golden rows:** 5,853 `math::beta_inc` and 407 `math::regularized_incomplete_beta`. The latter is downstream:
  `beta_inc(a, b, x, 1 − x).0`, gated at 0 against the model.
- **The reference:**
  - `check.c` has `bref`: a 320-bit continued fraction, the smaller of x and y exact, run on the side
    x < (a + 1)/(a + b + 2), with the other value as 1 − it. It is validated against exact rationals.
  - Port it to `mpfr_ref.py` as b4 ported `gref` to `gamma_pq`, and pool and cache it in `validate.py` like
    `gamma_reference`.
  - Check its convergence regions before trusting it. b4's `gref` hung for tiny shapes at small x, because Legendre's
    fraction there never converges in practice. P9's §6.6 notes that the far side's fraction can't converge when x is
    tiny, which is why P9 added TOMS 708's power series for small complements.
  - `check.c`'s `bref` branch (around line 140) still **skips** a NaN result (`isnan(r)` → `skipped++`). Make it count
    as a failure, as b4 did for `gref`.
- **Expect bugs past P9's sets.** P9's sets cover a, b ∈ [10⁻³, 10⁵]. Test:
  - tiny and subnormal shapes, shapes up to 10⁹ and more;
  - x at 0, 1, the least subnormal and 1 − 2⁻⁵³;
  - the switch boundary, and y given exactly.

  Watch for:
  - anything double-double that can pass 2⁹⁹⁶ (Dekker's split overflows into NaN);
  - series and continued-fraction stops that test the last term rather than a bound on the tail.
- **After b5:** part c, M2's gate. Re-measure §3.2's sets on the T1 functions and record them in port-parity, and remap
  the rows T1 moves once, before M3 records any row against them (plan §7).

## 5. Tools and where they live

- **`build/handoff/`** (gitignored, on disk; copied from the old session's scratchpad):
  - `gates.sh`, which writes its logs to `build/handoff/logs/`;
  - `bench_digests.py <dir>`: re-creates every `tests/special.bcyr` row's arguments and explains both sides' FNV-1a
    digests through `validate.py`'s glibc and reference models. Add rows for b5 if the bench changes;
  - `mut_b4.py` and `mut_b3.py`, the mutation runners;
  - `gdump.cyr`, `model_vs_port.py` and `gamma_domain.py`: b4's per-row checks (the port's values on every golden row,
    the model against the port, and the port's and Rust's errors against the truth by domain). Adapt them for beta.

  If `build/` is ever wiped, each is short: the procedure in §3 is enough to rewrite it.
- **`validate.py`'s reference cache:** `build/golden/gamma_ref.<hash>.tsv`, keyed by `gamma_pq`'s source and the MPFR
  version, and appended row by row. The first run takes minutes; later runs take about 50 s.
- **The Rust twin for the bench:** `scripts/probes/rust` (`cargo build --release` there if `target/` is gone).
- **The Rust oracle:** run inside `rust-old/`, never with `--manifest-path`. Never modify `rust-old/`.

## 6. Pitfalls this session hit

- **`pkill -f "<pattern>"` kills its own shell** when the pattern appears in the command line, which it always does.
  Kill by `pgrep`'d pid with a bracketed regex (`pgrep -f "build/crverify/chec[k]"`).
- **`cyrius fmt <file>` rewrites in place and prints nothing.** Use `cyrius fmt --check` to test.
- **zsh does not word-split `$var`.** Do loops over argument pairs in Python.
- **The editor's diagnostics compile a file alone,** without `tests/lib.cyr` or the test stdlib. Its "undefined
  function" and float-type warnings are noise; `cyrius test` and `build` are the truth.
- **Cyrius:**
  - forward calls with typed returns work warning-free;
  - `var x: f64 = load64(&TABLE + k * 8)` reads a table word as f64;
  - `#inline` declines functions with more than 2 parameters;
  - `_pmn_km_cr_pow2(e)` is valid only for −1022 ≤ e ≤ 1023.
- **x86's default NaN is `0xfff8…`.** Count NaNs by their bits, not by matching "7ff8".
- **Python 3.14 defaults to forkserver on Linux,** so pools use `multiprocessing.get_context("fork")`. Through ctypes,
  each MPFR call costs about 2 µs.
- **Other projects' jobs share the machine:** another `validate.py` (`scripts/oracle/`) and fuzzers. Gate the bench on
  load < 3, and never kill processes that aren't this project's.
