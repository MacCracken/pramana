# Architecture Decision Records

Decisions about pramana: what we chose, the context, and the consequences we accept. Use these when a future reader
would reasonably ask *"why did we do it this way?"*

## Conventions

- **Filename**: `NNN-kebab-case-title.md`, zero-padded to three digits (pramana's numbering since 001). Never renumber.
- **One decision per ADR.** If a decision supersedes a prior one, add a new ADR and set the old one's status to
  `Superseded by NNN`.
- **Status lifecycle**: `Proposed` → `Accepted` → (optionally) `Superseded` or `Deprecated`.
- Use [`template.md`](template.md) as the starting point.

## ADR vs. architecture note vs. guide

| Kind | Lives in | Answers |
|---|---|---|
| ADR | `docs/decisions/` | *Why did we choose X over Y?* |
| Architecture note | `docs/architecture/` | *What non-obvious constraint is true about the code?* |
| Guide | `docs/guides/` | *How do I do X?* |

## Index

| # | Title | Status |
|---|---|---|
| [001](001-flat-module-layout.md) | Flat module layout | Accepted; carried into the port |
| [002](002-handrolled-special-functions.md) | Hand-rolled special functions | Accepted; extended by 009 |
| [003](003-feature-gated-ai.md) | Feature-gated AI integration | Accepted; carried in intent (port plan D8) |
| [004](004-irls-for-logistic-regression.md) | IRLS for logistic regression | Accepted; carried into the port |
| [005](005-public-crate-deps.md) | Public crate dependencies | Rust 1.x only |
| [006](006-cyrius-port-with-rust-oracle.md) | Port to Cyrius as 2.0.0, with Rust 1.3.0 as the oracle | Accepted; amended by 010 (Python, not Rust's run, is the proof) |
| [007](007-port-api-divergences.md) | API divergences in the Cyrius 2.0 port | Accepted (rows appended as milestones land) |
| [008](008-local-stand-ins-for-library-gaps.md) | Local stand-ins for library gaps, converted through a ledger | Accepted |
| [009](009-correct-rounding-and-fixed-accuracy-bounds.md) | Correct rounding, and fixed accuracy bounds for the special functions | Accepted; amended 2026-11-16 (downstream rows) |
| [010](010-python-proof-rust-tooling-removed.md) | Python as the parity proof; the Rust tooling removed | Accepted |
