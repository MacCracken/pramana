# Pramana — Claude Code Instructions

> **Core rule**: this file is **preferences, process, and procedures** —
> durable rules that change rarely. Volatile state (current version,
> module line counts, port progress, test counts, consumers) lives in
> [`docs/development/state.md`](docs/development/state.md).
> Do not inline state here.

## Project Identity

**Pramana** (Sanskrit: proof/measure/evidence) — statistics and probability library for AGNOS

- **Type**: Flat library (Cyrius), being ported from the Rust crate 1.3.0. The port ships as 2.0.0
- **License**: GPL-3.0-only
- **Language**: Cyrius. The toolchain is pinned in `cyrius.cyml` `[package].cyrius`
- **Version**: `VERSION` at the project root is the source of truth. Don't inline the number here
- **Rust oracle**: pramana 1.3.0 plus the hidden `oracle` hooks, frozen at `rust-old/`. Its `Cargo.lock` and
  its `rust-toolchain.toml` (pinned to 1.100.0) are both tracked
- **Plan of record**: [`docs/development/pramana-port-plan.md`](docs/development/pramana-port-plan.md), with these ADRs:
  - [006](docs/decisions/006-cyrius-port-with-rust-oracle.md): the method;
  - [007](docs/decisions/007-port-api-divergences.md): the 2.0 API changes;
  - [008](docs/decisions/008-local-stand-ins-for-library-gaps.md): stand-ins for library gaps;
  - [009](docs/decisions/009-correct-rounding-and-fixed-accuracy-bounds.md): the accuracy target.
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md) · [First-Party Documentation](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)

## Goal

pramana owns statistics and probability for AGNOS: probability distributions and their special functions,
descriptive statistics, hypothesis tests, regression, Bayesian inference, Markov chains and hidden Markov
models, Monte Carlo and MCMC, and time series.

Accuracy is part of the contract:
- the elementary functions are correctly rounded;
- erf and erfc are correctly rounded;
- the other special functions meet the fixed ulp bounds of ADR 009;
- the same inputs give the same bits on every target.

## Consumers

AGNOS components that need statistical analysis, probability distributions, hypothesis testing, Monte Carlo
simulation or Markov chain modeling. The current list is in `state.md`. It includes the Rust crates that stay on
crates.io 1.x until they port.

## Quick Start

```sh
cyrius deps                                    # resolve dependencies (vendors lib/)
cyrius build src/main.cyr build/pramana        # compile
cyrius test                                    # run tests/*.tcyr
cyrius bench                                   # run tests/*.bcyr
(cd rust-old && cargo test --locked --all-features)              # the Rust oracle, read-only, on its pinned 1.100.0
(cd rust-old && cargo test --locked --features oracle --test oracle)   # the oracle hooks alone
```

Run the Rust oracle **from inside `rust-old/`**. rustup reads `rust-toolchain.toml` from the current directory, not
from the manifest's, so `--manifest-path rust-old/Cargo.toml` run from the root would use the wrong toolchain.

## Development Process

### P(-1): Scaffold Hardening (before any new features)

0. Read the roadmap, the port plan, the CHANGELOG and the open issues — know what was intended before auditing what was built
1. Test + benchmark sweep of the existing code
2. Cleanliness check (below)
3. Get baseline benchmarks
4. Internal deep review — gaps, optimizations, security, errors, docs
5. External research — domain completeness, missing capabilities, best practices, world-class accuracy
6. Cleanliness check — must be clean after the review
7. Additional tests/benchmarks from the findings
8. Post-review benchmarks — prove the wins
9. Repeat if heavy

### Cleanliness check

```sh
cyrius fmt --check <files>
cyrius lint <files>
cyrius vet src/main.cyr
cyrius build src/main.cyr build/pramana
cyrius test                 # must be warning-free, not just passing
cyrius test --bounds
cyrius bench
cyrius distlib --all && cyrius distlib --check   # once [lib] exists (M1)
cyrius deps --locked
cyrius audit
python3 -I -B scripts/upstream_ledger.py --check # once the ledger exists (M0 step 10)
```

`cyrius test` must be **warning-free**. The float type diagnostics come only from the compiles that `test`
and `build` drive, never from `cyrius lint`. Each one is a bug:
- *integer arithmetic with an f64 right operand*
- *f64 arithmetic with a non-f64 right operand*
- *an integer stored into an f64 slot*
- *comparison mixes f64 and integer*

### Work Loop (continuous)

1. Work phase — one module at a time, in the order the port plan gives
2. Cleanliness check
3. Test + benchmark additions: golden parity rows, stream rows, reference (MPFR, NIST StRD) rows, benches
4. Run benchmarks, recorded against the Rust numbers (`docs/benchmarks-rust-v-cyrius.md`)
5. Internal review — performance, memory (a flat heap), correctness, and a surface-parity diff against
   `rust-old/src/`
6. Cleanliness check — must be clean after the review
7. Deeper tests/benchmarks from the review
8. Benchmarks again — prove the wins
9. If the review was heavy, return to step 5
10. Documentation — CHANGELOG, `state.md`, roadmap, the port plan's tracking, the conversion ledger
11. Version check — `VERSION` and `cyrius.cyml` (`${file:VERSION}`) in sync, and the zugot recipe at release
12. Return to step 1

### Task Sizing

- **Low/Medium effort**: Batch freely — multiple items per work loop cycle
- **Large effort**: Small bites only — break into sub-tasks, verify each before moving to the next. Never batch large items together
- **If unsure**: Treat it as large. Smaller bites are always safer than overcommitting

### Refactoring

- Refactor when the code tells you to — duplication, unclear boundaries, performance bottlenecks
- Never refactor speculatively. Wait for the third instance before extracting an abstraction
- Refactoring is part of the work loop, not a separate phase
- Every refactor must pass the same cleanliness + benchmark gates as new code

## Key Principles

- **Cross-check against `rust-old/`.** The port's correctness bar is "matches what Rust did", within the
  tolerance class the port plan (§6) assigns. Diverge only with an ADR in `docs/decisions/`.
- **Accuracy beats glibc.** Parity is functional. Use correctly rounded primitives, never ones that copy
  glibc's last-ulp errors (port plan §0.1, ADR 009).
- **The special-function bounds are fixed.** The bounds in ADR 009 (port plan §3.4) are commitments:
  - erf and erfc are correctly rounded;
  - `normal_cdf` ≤ 1 ulp, `ln_gamma` ≤ 2, `gamma_inc` ≤ 4 and `beta_inc` ≤ 8, each over its stated domain.

  A row that misses means the algorithm is reworked. Never loosen a bound; only the maintainer can narrow a domain,
  by amending the ADR.
- **No deferral without a language check.** A Rust feature leaves the scope only for a reason written down
  against the port plan's language review (§1.2): a missing capability named with its guide or stdlib section, a
  measured cost, or no oracle. Re-check before inheriting another port's deferral.
- **Upstream requests never block.** A gap in ganita, hisab, bayan or the stdlib is filed in that project with
  pramana's numbers, and pramana ships a local stand-in meanwhile. Each stand-in carries an `UPSTREAM(Ln)` note and
  a row in the conversion ledger (port plan §10). It is converted when the library ships, with its parity rows
  re-run (ADR 008).
- **Correctness over cleverness.** If the Cyrius behavior diverges silently from Rust, the bugs win.
- Test after every change, not after the feature is "done". ONE change at a time.
- **Never skip benchmarks.** Numbers don't lie. Speed is measured, not gated: a slow row is a finding to file, not
  a reason to change the port. The dev box is shared: compare with pinned, interleaved runs.
- **Tests + benchmarks are the way.** Minimum 80%+ coverage target.
- **Own the stack.** If an AGNOS library wraps an external capability, depend on the AGNOS library.
- **No magic.** Every operation is measurable, auditable, traceable.
- **Deterministic.**
  - Never iterate a `hashmap` (its seed is random per process): sort the keys first.
  - Every sort key is total: inputs are checked NaN-free before they are sorted.
  - The same inputs give the same bits on every target.
- **A flat heap** (port plan D11, §8.5).
  - Constructors fill caller storage.
  - Results whose length follows from the inputs go into caller buffers `(out, cap)`.
  - Owned objects are freelist-allocated with a `_drop`.
  - Scratch comes from the work-table pool.
  - Never `alloc_reset`: it wipes the consumer's heap.
  - Check every buffer parameter for null and for aliasing.
- **Enums.** Public enums have explicit, append-only numeric values that are never renumbered (the Cyrius form of
  `#[non_exhaustive]`). Property functions `match` with an `_ =>` arm.
- **`#must_use` on all pure functions**, plus `#pure` where it applies; `#inline` on hot paths.
- **No aborts in library code.** `src/` never uses:
  - `+?`, `-?` or `*?` (they exit the process);
  - `assert_fatal`, `panic` or `result_unwrap`;
  - an unchecked `vec_get` or `vec_select_nth` (both abort out of range).

  Errors return a value-form `Result` with a `PMN_ERR_*` code (ADR 007).
- **Every public type has JSON.** This is the Cyrius form of "Serialize + Deserialize on every type" (port plan D5):
  - hand codecs in `src/json.cyr`, over bayan's value tree;
  - every decoder goes through the validating constructor, as Rust's `try_from` did;
  - a document that serde_json wrote from 1.3.0 decodes to the same bits;
  - every type has a round-trip test.
- **Strings.** Write into a `str_builder` instead of building temporaries; borrow by pointer and copy only into
  owned storage (the Cyrius form of "`write!` over `format!`").
- **Float hygiene.**
  - Bind every call result to a `: f64` local before using it in an operator.
  - Never put hex in a `const`: it is an integer.
  - Never write `(0 - x)` on a float: use `-x`.
  - There is no exponent notation in literals: the lexer has none. Generated files write decimals positionally.
  - The stdlib `F64_*` constants are untyped. Use the generated typed globals.
  - Use the canonical `ganita_f64_*` names, never the legacy aliases, which ganita M4 deletes.
  - Every transcendental goes through the correctly rounded functions in `src/crmath.cyr` (port plan §8.1).
- **Constants are generated, never typed by hand.** `scripts/golden/` writes them from the oracle. Regenerate them;
  never edit them.
- **Names.** Every public global carries the `PMN_` prefix (constants, enum members, tables). Private helpers use
  `_pmn_`. Public structs are `Pmn<Name>`, and free functions are `pmn_<name>`. A bundle shares one namespace with
  its consumers, and the last definition of a name silently wins.
- **Function names follow the Rust API**, except where ADR 007 changes them.
- **Optional dependencies** go in `[features]`, so consumers pull only what they need; `[build] dce` drops unused code.
- Build with `cyrius build`, not raw `cat file | cc5` — the manifest auto-resolves deps.
- Source files only need project includes — stdlib auto-resolves from `cyrius.cyml`.
- `var buf[N]` = N **bytes**, not N entries.

## DO NOT

- **Do not commit or push** — the user handles all git operations (commit, push, tag)
- **NEVER use `gh` CLI** — use `curl` to the GitHub API only
- **Do not modify `rust-old/`.** It is the parity oracle. Tools that drive it (the golden-value and table generators)
  live in `scripts/golden/` and depend on it by path. A wrong oracle value is fixed as Rust 1.3.x, by the
  maintainer, and the corpus regenerated
- Do not modify `lib/` files (vendored stdlib / dep symlinks)
- Do not hardcode toolchain versions in CI YAML — `cyrius = "X.Y.Z"` in `cyrius.cyml` is the source of truth
- Do not widen a parity tolerance class to make a row pass, and do not loosen an ADR 009 bound. Class-C bounds are
  set by the rule in `docs/development/port-parity.md` before a function's first comparison
- Do not wait on an upstream release: write the stand-in and file the request
- Do not add unnecessary dependencies — keep it lean
- Do not `panic` or abort in library code
- Do not break backward compatibility without a major version bump
- Do not skip benchmarks before claiming performance improvements
- Do not skip tests before claiming changes work
- Do not commit `build/` or `rust-old/target/`. `rust-old/Cargo.lock` and `rust-old/rust-toolchain.toml` **are**
  committed: the frozen oracle needs its exact dependency versions and toolchain (ADR 006)

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, LICENSE

docs/:
  development/pramana-port-plan.md — the port plan of record (includes the conversion ledger, §10)
  development/state.md             — live state
  development/roadmap.md           — planned work only; done work goes in the CHANGELOG
  development/port-parity.md       — measured parity baselines and probe results (from M0 steps 8–9)
  benchmarks-rust-v-cyrius.md      — Rust vs Cyrius benchmark record (from M1)
  decisions/NNN-title.md           — architecture decision records (template.md, README.md index)
  architecture/                    — non-obvious constraints
  guides/                          — task-oriented how-tos
  examples/                        — runnable examples
rust-old/docs/                     — the Rust 1.3.0 overview, testing guide, threat model and dependency watch
scripts/port-baseline/             — the accuracy baseline of Rust 1.3.0 against MPFR (port plan §3)
```

ADR format: `# NNN. Title`, then Date, Status, Context, Decision, Consequences, Alternatives considered (see
`docs/decisions/template.md`).

## CHANGELOG Format

Follow [Keep a Changelog](https://keepachangelog.com/): `## [Unreleased]`, then per version
`### Added / Changed / Fixed / Removed / Security / Performance`.

- Every change that alters behavior gets a CHANGELOG entry
- Performance claims MUST include benchmark numbers (Rust vs Cyrius where relevant)
- Accuracy claims MUST include the measured ulp errors and the reference they were measured against
- Breaking changes get a **Breaking** section with a migration guide
- Group by module when multiple changes land in one release
- Link the ADR when a change was driven by an architectural decision
- A stand-in's conversion to its library gets an entry naming the ledger row
