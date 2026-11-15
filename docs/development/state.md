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
| M0 Initialize | ⏳ **in progress.** **Step 0** (Rust, `94a7153`): the P(-1) gates are green; the hidden `oracle` feature exposes the private special functions, samplers, quantiles, HMM forward pass, KDE kernels and the `MultivariateNormal` factor; `tests/oracle.rs` has 11 tests, mutation-checked six ways; the toolchain is pinned to 1.100.0. **Step 1** (`d8e761d`): the `cyrius port` scaffold, with the Rust crate in `rust-old/`, where it still passes (330 tests with `--all-features`). **Step 2** (`40a8269`): repairs (CLAUDE.md, the roadmap, CI, the manifest's version and `bench`, the scaffold stubs documented, Rust-only docs and scripts moved, ADRs 006–009, CONTRIBUTING, SECURITY, a README banner). **Step 3** (`0e147d3`): the full `cyrius.cyml` (16 stdlib leaves, no hisab, `dce = true`; 43 files locked); every gate green and warning-free. **Step 4**: the semantics probe ([`port-parity.md`](port-parity.md) §1): 125 cases, 140 rows exact; helpers needed for min/max NaN, `fract` (floor-based `f64_fract`), `u64 → f64` (signed `f64_from`) and saturating `as usize`; sums start at −0.0; 5 transcendental rows 1 ulp apart, 3 of them glibc's misrounds (`53078cb`). **Step 5**: the golden generator (`scripts/golden/`): `tests/golden/math.tsv`, 35,597 rows in 31 functions, every one re-checked by `validate.py` without the oracle; `math.cr.tsv`, 76 rows where glibc misrounds (`exp_m1` 5.1%); deterministic and tamper-checked. **Steps 6–10**: pending |
| M1–M8 | not started |

## Toolchain

- **Cyrius pin**: 6.7.18 (`cyrius.cyml` `[package].cyrius`)
- **ganita**: 1.2.15, bundled in the 6.7.18 stdlib
- **hisab**: none. The four hisab 1.4.0 routines pramana used are transcribed in M3–M5 (port plan §8.2)
- **Rust oracle**: rustc 1.100.0 (`rust-old/rust-toolchain.toml`), hisab 1.4.0 (`rust-old/Cargo.lock`), glibc 2.44

## Source

- **Rust oracle**: 4,454 library code lines in `rust-old/src/` (14 modules plus `integration/soorat.rs`, 102 `pub fn`s
  plus 65 trait methods), and the `oracle` hooks. Frozen; never edited.
- **Cyrius**: the scaffold's `src/main.cyr` stub. No library modules yet (`[lib]` arrives with M1).

## Tests

- **Rust oracle**: run from inside `rust-old/` with `cargo test --all-features`: 301 unit, 15 integration, 11
  oracle and 3 doc tests (330).
- **Golden corpus** (`tests/golden/`): `math.tsv` 35,597 rows and `math.cr.tsv` 76 rows. They are checked locally by
  `(cd scripts/golden && cargo run --release --locked -- --check)` and `python3 -I -B scripts/golden/validate.py .`.
- **Cyrius**: the scaffold's `tests/pramana.{tcyr,bcyr,fcyr}` stubs. The parity harness arrives at M0 step 7.

## Accuracy baseline

Rust 1.3.0 against MPFR 4.2.2: [port plan §3.2](pramana-port-plan.md), reproduced by
`scripts/port-baseline/run.sh`.

## Upstream requests and stand-ins

The conversion ledger is in the port plan, §10. Requests are filed at M0 step 10; none are filed yet.

## Dependencies

Direct (declared in `cyrius.cyml`; 43 files locked in `cyrius.lock`):

- stdlib: syscalls, string, alloc, str, fmt, io, vec, math, ganita, overflow, tagged, result, freelist, fnptr, atomic,
  bayan
- test stdlib (`[test] stdlib`, test / bench / fuzz builds only): assert, bench
- no hisab (port plan §8.2); sandhi joins only for the `ai` code, outside the core bundle (D8)
- `[build] dce = true`: the skeleton builds to 407,872 bytes

## Consumers

- **Cyrius:** none declares a pramana dependency yet. `ai` will talk to hoosh 2.12.3 (Cyrius) over its HTTP API.
- **Rust (`pramana = "1"`, optional):** bodh, jivanu and sangha, all at 1.0.0. They stay on crates.io 1.x until they
  port.

## Next

M0 steps 6–10 (port plan §5): the constants, the parity harness, the baselines, the probes (P1–P9, among them the CORE-MATH
erf/erfc port in its time slot), and filing the upstream requests.
