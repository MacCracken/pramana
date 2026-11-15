# 8. Local stand-ins for library gaps, converted through a ledger

Date: 2026-11-15
Status: Accepted

## Context

The port needs things the Cyrius libraries don't yet provide in the form pramana needs (plan §1.3, §8.2):
- correctly rounded transcendentals, among them log1p, tan and log2, which no library has;
- the Rust f64 surface (`is_nan`, `copysign`, typed constants, NaN-ignoring min/max);
- sort and select on raw f64 arrays without aborts;
- an allocation-free Cholesky solve;
- polynomial least squares;
- a symmetric eigensolver with Rust 1.4.0's tolerance and order;
- a `#derive` with a validation hook;
- a hoosh client library.

Waiting for each library would stall the port. kimiya (ADR 0003) and pravash (ADR 0003) faced the same gaps and
shipped local code with a tracked path back to the library.

## Decision

**Upstream requests never block the port.** For each gap:

1. **File a request** in the owning repo's `docs/development/issues/`, with a self-proving repro and pramana's
   numbers. Where an issue is already open, add pramana as a consumer instead.
2. **Ship a local stand-in.** It carries an `UPSTREAM(Ln)` comment naming its ledger row, its request, and the
   library call that will replace it.
3. **Keep the conversion ledger** in plan §10, with one row per stand-in. `scripts/upstream_ledger.py --check`
   runs in CI: every stand-in is marked, and every note has a row.
4. **Convert when the library ships,** before or after 2.0. Re-run the stand-in's parity rows, and record the
   conversion in the CHANGELOG under its ledger row.

**Transcribe by default.** A Rust hisab 1.4.0 routine is transcribed rather than replaced by a different Cyrius
algorithm, unless probe P5 shows the library gives 1.4.0's bits without allocating per call (kimiya's experience:
nearly everything ended up transcribed). The transcriptions live in `src/linalg.cyr`, so the library has no hisab
dependency.

## Consequences

- **Positive.** No milestone waits on another project. Each library learns about its gap with real numbers.
- **Negative.** pramana temporarily owns code that belongs elsewhere: the correctly rounded functions, the
  linear-algebra transcriptions, the JSON codecs.
- **Neutral.** The ledger is maintained work at every milestone gate.

## Alternatives considered

- **Wait for upstream:** rejected. It serializes the port behind five projects' release schedules.
- **Use the library's different algorithm as is:** rejected where it changes the bits against the oracle without
  an accuracy reason. That would turn every downstream row into class D.
