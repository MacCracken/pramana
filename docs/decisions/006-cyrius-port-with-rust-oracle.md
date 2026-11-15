# 6. Port to Cyrius as 2.0.0, with Rust 1.3.0 as the oracle

Date: 2026-11-15
Status: Accepted

## Context

AGNOS crates are moving from Rust to Cyrius. tanmatra, kimiya, pravash, impetus and ushma were ported the same way:
the last Rust release is frozen as an executable reference (the "oracle"), and the Cyrius code is checked against its
recorded outputs. pramana 1.3.0 had just been through a full P(-1) audit, with every fix under a regression test.
That makes it a sound reference.

pramana is 4,454 library code lines in 14 modules. Its own special functions feed nearly every public result, so
the port needs an oracle that reaches private functions, not only the public API.

## Decision

pramana is ported to Cyrius and ships as **2.0.0**. Rust **1.3.0**, plus the hidden read-only `oracle` hooks, is the
oracle. The plan of record is `docs/development/pramana-port-plan.md`.

- **Frozen.** The crate lives in `rust-old/` (commit `94a7153`) and is never edited. A wrong value it records is
  fixed as Rust 1.3.x by the maintainer, and the corpus is regenerated.
- **Two exceptions to the library-crate rule:**
  - `rust-old/Cargo.lock` is tracked, because the bits depend on hisab 1.4.0;
  - `rust-old/rust-toolchain.toml` is pinned to 1.100.0, because pravash's recorded corpus broke when stable moved.
- **The hooks are tooling, never published.** They live in child modules (`src/<module>/oracle.rs`), so each audited
  source gained only a `mod oracle;` line. `tests/oracle.rs` proves them bit-identical to the public paths and
  mutation-checks them.
- **No Rust 2.0.** API changes land in the Cyrius API only (ADR 007).
- **No deferral without a language check.** A Rust feature leaves the 2.0 scope only for a reason written against
  the plan's language review (§1.2): a missing capability, a measured cost, or nothing to test against. The review
  found none, so every module and feature is carried.
- **Parity classes** follow tanmatra and pravash (plan §6): A exact, B arithmetic, C transcendental, D iterative or
  library-backed, S streams, R reference. A class is never widened to make a row pass.
- **Releases:** the Rust CI job that guards the oracle is removed before the 2.0.0 tag, because `release.yml` gates
  on the whole CI workflow.

**The earlier decisions under the port:**

| ADR | Status in 2.0 |
|---|---|
| 001 flat module layout | carried: one flat `[lib]` bundle, `src/<module>.cyr` |
| 002 hand-rolled special functions | carried, and extended by ADR 009 |
| 003 feature-gated `ai` | carried in intent: `ai` stays out of the core bundle (plan D8) |
| 004 IRLS for logistic regression | carried: the Cholesky solve becomes a transcription (ADR 008) |
| 005 public crate dependencies | Rust-only; the Cyrius `[deps]` replace it |

## Consequences

- **Positive.** Every Cyrius result has a recorded Rust expectation or a named, ADR-accepted divergence. The audit's
  fixes carry over by construction.
- **Negative.** `rust-old/` must keep building on its pinned toolchain until 2.0.0. The hooks are extra code in a
  frozen crate.
- **Neutral.** Rust users stay on crates.io 1.x (bodh, jivanu and sangha today). A 1.3.x deprecation notice follows
  2.0.0.

## Alternatives considered

- **Port the pre-audit code**, or fix while porting: rejected, as tanmatra's ADR 0006 explains. A reference with
  known defects, or one that moves, can't settle a disagreement.
- **Publish 1.3.1 with the hooks:** no value to crates.io users (plan D1).
- **Port without hooks:** the private special functions would then be reachable only through public pdf/cdf calls,
  losing per-function parity.
