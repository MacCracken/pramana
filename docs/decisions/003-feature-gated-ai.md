# 3. Feature-Gated AI Integration

Date: 2026-03-28
Status: Accepted

## Context

Pramana optionally integrates with hoosh (the AGNOS AI client) for tasks such as
LLM-assisted anomaly explanation and natural-language statistical queries. Hoosh
transitively pulls in `reqwest`, `tokio`, `serde_json`, and many other crates.

Most consumers of pramana need pure statistical functions and should not pay the
compile-time or binary-size cost of an HTTP stack.

## Decision

Gate the entire `ai` module behind a Cargo feature flag:

```toml
[features]
ai = ["dep:hoosh", "dep:tokio", "dep:serde_json"]
```

The `ai` feature is **not** in `default`. Users who want AI capabilities opt in
with `pramana = { version = "1", features = ["ai"] }`.

The `ai.rs` source file is conditionally compiled:

```rust
#[cfg(feature = "ai")]
pub mod ai;
```

## Consequences

**Positive**

- Default dependency tree stays small: hisab, serde, thiserror, tracing.
- Compile times for the common case are unaffected by network crate churn.
- Clear opt-in signal -- consumers know they are adding async + network deps.

**Negative**

- Two build configurations to test (`default` and `--all-features`).
- AI-related types cannot be used in signatures of non-AI modules without
  `#[cfg]` guards, which limits cross-module ergonomics.
