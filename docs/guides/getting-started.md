# Getting started with pramana

## Use it

Include the bundle and call the library. [`../examples/`](../examples/) has runnable programs, and
[`migration-2.0.md`](migration-2.0.md) the conventions (names, error codes, caller buffers, drops, callbacks):

```sh
cyrius build docs/examples/quick_start.cyr build/quick_start && ./build/quick_start
```

The AI client is the `[lib.ai]` bundle, `dist/pramana-ai.cyr`. Its consumer includes `lib/sandhi.cyr` first, as
[`../examples/ai_query.cyr`](../examples/ai_query.cyr) does.

## Build and test it

```sh
cyrius deps                                              # the pinned stdlib
python3 -I -B scripts/golden/gen.py .                    # the golden corpus (about 3 minutes, needs libmpfr6)
python3 -I -B scripts/golden/validate.py . --write-cr    # every row checked; the overlays written
cyrius build src/main.cyr build/pramana
cyrius test                                              # the unit tests and tests/parity.tcyr's replay
cyrius bench
cyrius distlib --all && cyrius distlib --check           # the bundles
```

## Layout

- `src/`: the library, one module per 1.3.0 module, in `cyrius.cyml`'s `[lib]` order. `tests/lib.cyr` mirrors that
  order for the tests.
- `tests/`: `*.tcyr` unit tests (each carries its 1.3.0 tests over, `docs/development/rust-test-ledger.md`),
  `parity.tcyr` (the golden replay) and `*.bcyr` benches. `tests/golden/` is generated and never committed.
- `scripts/golden/`: Python's models of 1.3.0 (`validate.py`) and the corpus emitters (`emit/m_*.py`).
- `scripts/crverify/`: the correct-rounding checks against MPFR.
- `rust-old/`: 1.3.0's source, the parity reference. It is read as text and never built (ADR 010).

## Change something

1. Read the port plan's rules (`docs/development/pramana-port-plan.md`) and CLAUDE.md's process.
2. Change one thing, with its golden rows (the Python model first) and its unit tests.
3. Run the cleanliness check (CLAUDE.md): fmt, lint, `cyrius test` warning-free, the bundles, the ledgers.
4. Add a CHANGELOG entry. Record benchmarks against `docs/benchmarks-rust-v-cyrius.md`.

See [`../decisions/template.md`](../decisions/template.md) when a design choice deserves an ADR.
