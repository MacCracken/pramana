# 5. Public Crate Dependencies

Date: 2026-03-28
Status: Accepted

## Context

Pramana depends on two other AGNOS crates:

- **hisab** -- linear algebra, matrix operations, constants.
- **hoosh** -- AI/LLM client (optional).

Both are published on crates.io. During early development they were referenced
via local `path` dependencies pointing into the AGNOS monorepo. This made builds
fast but required every consumer to clone the entire workspace.

## Decision

Use published crates.io versions for all AGNOS dependencies:

```toml
hisab = { version = "1", default-features = false, features = ["num"] }
hoosh = { version = "1", default-features = false, optional = true }
```

No `path` or `git` overrides in the committed `Cargo.toml`. Developers who need
a local override during cross-crate work can use `[patch.crates-io]` in a
workspace-level `Cargo.toml` or a `.cargo/config.toml` override -- neither of
which is committed.

## Consequences

**Positive**

- Reproducible builds -- `cargo build` works for anyone with internet access.
- Version pinning via SemVer means breaking changes are caught at upgrade time.
- CI does not need the monorepo checked out.

**Negative**

- Cross-crate development requires a local `[patch]` workflow.
- Publishing order matters: hisab must be published before a pramana release
  that bumps its hisab dependency.
