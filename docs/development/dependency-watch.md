# Dependency Watch

Tracking direct dependencies, their current pinned ranges, and upgrade notes.

Last reviewed: 2026-03-28

## Runtime Dependencies

| Crate     | Version | Features          | Notes                                       |
|-----------|---------|-------------------|---------------------------------------------|
| hisab     | 1.x     | `num`             | Core linalg, Cholesky, eigen. Required.     |
| hoosh     | 1.x     | (default-features off) | AI/LLM client. Optional behind `ai` flag. |
| serde     | 1.x     | `derive`          | Serialization for all public types.         |
| thiserror | 2.x     | --                | Derive macro for `PramanaError`.            |
| tracing   | 0.1     | --                | Structured logging spans and events.        |
| tokio     | 1.x     | `rt`              | Async runtime. Optional behind `ai` flag.   |
| serde_json| 1.x     | --                | JSON handling. Optional behind `ai` flag.   |

## Optional (Feature-Gated)

The `ai` feature activates: `hoosh`, `tokio`, `serde_json`.

The `logging` feature activates: `tracing-subscriber` (0.3, `env-filter` + `fmt`).

## Dev Dependencies

| Crate     | Version | Notes                              |
|-----------|---------|------------------------------------|
| criterion | 0.5     | Benchmarking harness (`html_reports`). |
| serde_json| 1.x     | Serde roundtrip tests.             |

## Upgrade Policy

- **Patch/minor**: upgrade freely after CI passes.
- **Major**: review changelog, check for breaking API changes, update code, bump
  pramana's version if public API is affected.
- Run `cargo audit` and `cargo deny check` before every release.
