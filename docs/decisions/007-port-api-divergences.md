# 7. API divergences in the Cyrius 2.0 port

Date: 2026-11-15
Status: Accepted (seed list; rows are appended as milestones land)

## Context

Cyrius has no `String`-carrying error enums, no `u128` and no serde. A `Result` payload is one word, a small struct in a `Result` is miscompiled (an open issue), and the
heap is a bump allocator unless a library frees explicitly. Some of the Rust API therefore can't be carried
shape-for-shape (plan §1.2). Each change must still be compared by the parity harness, so nothing leaves parity
silently.

## Decision

The Cyrius API differs from Rust 1.3.0 only in the rows below, each with how the harness compares it. The plan's
§4.3 is the working copy; this ADR is the record. New rows are appended, never renumbered.

| # | Rust 1.3.0 | Cyrius 2.0.0 | Compared as |
|---|---|---|---|
| 1 | `Result<T, PramanaError(String)>` | value-form `Result` with a `PMN_ERR_*` code; no detail strings; `pmn_err_name(code)` for text (D6) | class A on the variant |
| 2 | constructors return `Result<Self>` | `PmnX_new(out, …)` fills caller storage and returns a code (D11) | A on the code, then each field by its class |
| 3 | `Vec` returns | a caller buffer `(out, cap)` plus a count where the length follows from the inputs; otherwise a freelist-owned result with `_drop` (D11) | per element |
| 4 | `u128` from `factorial`, `permutations`, `combinations` | an out-pointer `PmnU128 { lo, hi }` (D10) | A |
| 5 | `impl Fn` closures | a function pointer plus a context pointer (D7) | the same values |
| 6 | `&mut impl Rng` | `dyn PmnRng`, implemented by `PmnSimpleRng` (D7) | the same streams |
| 7 | serde_json text | bayan's Grisu2 text; every 1.3.0 document decodes to the same bits (D5) | A on the decoded bits |
| 8 | `PramanaError` JSON with a message | the code only; the message is dropped on decode | A on the variant |
| 9 | `async StatQuery::query`; `HashMap` column order | synchronous HTTP to hoosh; columns in sorted name order (D8) | A on the tool results; the prompt text is n/a |
| 10 | cargo features `ai`, `soorat-compat`, `logging` | one bundle with dead-code elimination; `ai` outside the core bundle; `logging` (a no-op) not carried (D8, D9) | — |
| 11 | bits depend on the host glibc | bits identical on every target | measured on x86_64 ELF, PE and aarch64 |
| 12 | glibc's last-ulp misrounds | correctly rounded primitives (ADR 009) | `*.cr.tsv` at 0 ulp |
| 13 | `math.rs`'s erf/erfc, `normal_cdf`, `ln_gamma`, `gamma_inc`, `beta_inc` | the more accurate algorithms of ADR 009 | `*.ref.tsv` at the ADR's bounds; downstream rows remapped by name |
| 14 | `median`, `percentile`, `quartiles` and `iqr` take the k-th element from `select_nth_unstable_by`, whose order decides which zero a mix of −0 and +0 yields (P6: 9,393 of 180,000 zero-heavy results) | the k-th element, and its successor, of a **stable** sort of the input (ties, −0 and +0 among them, kept in input order). Decided 2026-11-15 | A on the value. Where Rust's sign of zero differs, the row maps to the stable rule's result (`descriptive.map.tsv`, from `validate.py`'s stable model) and passes as accepted (ADR 7) |
| 15 | derived `Debug` | `x.debug(sb, alternate)` writers, with pravash's shortest-float printer (D12) | A on the text |

## Consequences

- **Positive.** Each change is named, and the harness reports it by name, so a reader of a failing row knows
  whether it is a divergence or a defect.
- **Negative.** Consumers migrating from Rust have to adapt to every row; the migration guide (M8) covers each.
- **Neutral.** The list grows as milestones find more. Every addition needs its comparison column.

## Alternatives considered

- **Shape-for-shape ports**, e.g. errors carrying strings or constructors returning heap pointers: rejected. They
  leak on the bump heap, or hit the open small-struct-in-`Result` miscompile.
- **A Rust 2.0 with the same API changes:** rejected (ADR 006). The Rust line ends at 1.3.x.
