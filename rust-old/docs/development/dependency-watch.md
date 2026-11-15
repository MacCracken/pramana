# Dependency Watch

Tracking direct dependencies, their current pinned ranges, and upgrade notes.

Last reviewed: 2026-11-14 (Rust 1.100, edition 2024 -- still the latest edition)

## Runtime Dependencies

| Crate     | Version  | Features          | Notes                                       |
|-----------|----------|-------------------|---------------------------------------------|
| hisab     | 1.4      | `num`             | Cholesky, eigen, least squares. Required. 1.4 pulls `glam` even with only `num`. |
| serde     | 1.0.229  | `derive`          | Serialization for all public types.         |
| thiserror | 2.0.21   | --                | Derive macro for `PramanaError`.            |

## Optional (Feature-Gated)

| Crate      | Version | Feature | Notes                                   |
|------------|---------|---------|-----------------------------------------|
| hoosh      | 1.3     | `ai`    | AI/LLM client (default-features off). API unchanged 1.1 -> 1.3 for our use. |
| tokio      | 1.54    | `ai`    | Async runtime (`rt`).                   |
| serde_json | 1.0.154 | `ai`    | Tool-call arguments.                    |

`soorat-compat` and `logging` enable no dependencies. `logging` is a reserved
no-op: `tracing` / `tracing-subscriber` were removed on 2026-11-14 because
nothing in the crate used them.

## Dev Dependencies

| Crate     | Version  | Notes                                   |
|-----------|----------|-----------------------------------------|
| criterion | 0.8      | Benchmarks (`html_reports`). 0.8 deprecates `criterion::black_box`; benches use `std::hint::black_box`. |
| serde_json| 1.0.154  | Serde roundtrip tests.                  |

## Upgrade Policy

- **Patch/minor**: upgrade freely after CI passes.
- **Major**: review changelog, check for breaking API changes, update code, bump
  pramana's version if public API is affected.
- Run `cargo audit` and `cargo deny check` before every release.
- Keep `deny.toml`'s license allow-list to licenses actually in the tree
  (`cargo deny check licenses` warns on unused entries).
