# Contributing to Pramana

Thank you for your interest in contributing to Pramana.

pramana 2.0 is a Cyrius library, being ported from the Rust crate 1.3.0. The plan of record is
[`docs/development/pramana-port-plan.md`](docs/development/pramana-port-plan.md), and
[`docs/development/state.md`](docs/development/state.md) says where the port stands. The frozen Rust crate in
`rust-old/` is the parity oracle: never edit it.

## Development Setup

```sh
git clone https://github.com/MacCracken/pramana.git
cd pramana

# The Cyrius toolchain version is pinned in cyrius.cyml ([package].cyrius)
cyrius --version

# Resolve dependencies (vendors lib/), build, test
cyrius deps
cyrius build src/main.cyr build/pramana
cyrius test

# The Rust oracle, read-only. Run it from inside rust-old/, so rustup applies its pinned toolchain
(cd rust-old && cargo test --locked --all-features)
```

## Pull Request Process

1. **Fork and branch** — create a feature branch from `main`.
2. **Keep commits focused** — one logical change per commit, and one module at a time during the port.
3. **Write tests.**
   - Ported code needs golden parity rows against 1.3.0, plus reference tests: MPFR or exact values for the
     special functions, NIST StRD for statistics.
   - A bug fix needs a regression test.
   - The golden corpus is generated: never patch a row by hand.
4. **Run the cleanliness check** before pushing (CLAUDE.md lists it): `cyrius fmt --check`, `cyrius lint`,
   `cyrius vet`, `cyrius build`, `cyrius test` (warning-free), `cyrius test --bounds`, `cyrius bench`,
   `cyrius deps --locked`, `cyrius audit`.
5. **Open a PR** against `main` with a clear description of the change.
6. **Address review feedback** — maintainers may request changes before merging.

## Code Style

- `cyrius fmt` formatting and a clean `cyrius lint` (enforced by CI).
- `cyrius test` and `cyrius build` print no float type warnings. Each warning is a bug.
- Public functions have doc comments.
- `#must_use` on pure functions, `#inline` on hot paths, append-only enum values.
- No aborts in `src/`: no `+?`, `-?`, `*?`, `assert_fatal`, `panic`, `result_unwrap`, or unchecked `vec_get` /
  `vec_select_nth`. Errors are value-form `Result`s with `PMN_ERR_*` codes.
- Names: `Pmn<Name>` structs, `pmn_<name>` free functions, `PMN_` globals, `_pmn_` private helpers.
- Every transcendental goes through the correctly rounded functions (ADR 009).
- A stand-in for a missing library routine carries an `UPSTREAM(Ln)` note and a ledger row (ADR 008).

## Testing Requirements

- Every ported public function is compared against 1.3.0's golden values, within the parity class the port plan
  (§6) assigns. Never widen a class to make a row pass.
- The special functions meet ADR 009's fixed bounds, measured against MPFR. Never loosen a bound.
- Every Rust test is ported, folded into golden rows, or recorded with a reason (the Rust-test ledger).
- Every public type has a JSON round-trip test, and its decoder refuses invalid parameters.
- Benchmarks go in `tests/*.bcyr`, beside their Rust twins, and are run before and after performance-sensitive changes.

```sh
cyrius test            # unit, parity and reference tests
cyrius test --bounds   # the same, bounds-checked
cyrius bench           # benchmarks
cyrius fuzz            # fuzz targets
```

## Commit Messages

Use clear, imperative-mood commit messages, one logical change per commit:

```
port the incomplete gamma function (M2 part a)

fix NaN handling in Weibull parameter validation
```

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0-only.
