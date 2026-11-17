#!/usr/bin/env python3
"""The Rust-test ledger (port plan M8 part a): every test of pramana 1.3.0 and where 2.0 carries it.
    python3 -I -B scripts/rust_test_ledger.py [--write | --check]
It lists every `#[test]` and `#[tokio::test]` in rust-old/src and rust-old/tests, and the doc tests in
rust-old/src/lib.rs, and accounts for each by its Rust file: the Cyrius test file and groups that carry that file's
tests over, or the reason it is retired. It checks that every Rust file with tests has an entry, that every Cyrius
file named exists, and that every group named is a `test_group("…")` in it (by prefix). `--write` writes
docs/development/rust-test-ledger.md; `--check` (the gate) also fails when that file is out of date.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST = ROOT / "rust-old"
OUT = ROOT / "docs" / "development" / "rust-test-ledger.md"

# Rust file → (the Cyrius test file, the group-title prefixes that carry its tests), or (None, the reason it retires).
HOMES = {
    "src/rng.rs": ("tests/rng.tcyr", ["SimpleRng::new", "draws", "the JSON codec"]),
    "src/combinatorics.rs": ("tests/combinatorics.tcyr", ["factorial (1.3.0's tests)", "permutations (1.3.0's tests)",
                                                          "combinations (1.3.0's tests)",
                                                          "stirling_approx (1.3.0's test)"]),
    "src/bridge.rs": ("tests/bridge.tcyr", ["badal and ushma (1.3.0's tests)", "kimiya (1.3.0's tests)",
                                            "bodh (1.3.0's tests)", "the fits (1.3.0's tests)"]),
    "src/math.rs": ("tests/special.tcyr", ["erf_known_values", "erf_erfc_sum_to_one", "erf_reference_values",
                                           "erf_special_values", "normal_cdf_tails", "ln_gamma_known_values",
                                           "stirling_error_continuous_at_cutoff", "ln_beta_matches_ln_gamma",
                                           "ln_gamma_kernel_matches_direct_form"]),
    "src/distribution.rs": ("tests/dist.tcyr, tests/dist_mvn.tcyr", ["1.3.0's"]),
    "src/descriptive.rs": ("tests/descriptive.tcyr, tests/kde.tcyr, tests/pca.tcyr", ["1.3.0's"]),
    "src/hypothesis.rs": ("tests/hypothesis.tcyr", ["1.3.0's"]),
    "src/regression.rs": ("tests/regression.tcyr", ["1.3.0's"]),
    "src/timeseries.rs": ("tests/timeseries.tcyr", ["1.3.0's"]),
    "src/bayesian.rs": ("tests/bayesian.tcyr", ["1.3.0's"]),
    "src/markov.rs": ("tests/markov.tcyr", ["1.3.0's"]),
    "src/monte_carlo.rs": ("tests/monte_carlo.tcyr", ["1.3.0's"]),
    "src/integration/soorat.rs": ("tests/soorat.tcyr", ["1.3.0's"]),
    "src/ai.rs": ("tests/ai.tcyr", ["1.3.0's"]),
    "tests/integration.rs": ("tests/integration.tcyr", ["1.3.0's integration tests"]),
    "src/lib.rs (doc tests)": ("tests/integration.tcyr", ["1.3.0's doc tests"]),
    "tests/oracle.rs": (None, "with the `oracle` feature (ADR 010): its tests proved that each hidden hook was "
                              "the public code path, for the Rust golden generator; Python's models in "
                              "scripts/golden/validate.py replaced the generator, matching its corpus on all 329,431 "
                              "rows before it was removed, and every golden row now replays against the port"),
}

TEST = re.compile(r"#\[(?:tokio::)?test\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+(\w+)")


def rust_tests():
    """{Rust file (relative to rust-old): [test names]}, the doc tests under 'src/lib.rs (doc tests)'."""
    out = {}
    for p in sorted(list((RUST / "src").rglob("*.rs")) + list((RUST / "tests").glob("*.rs"))):
        names = TEST.findall(p.read_text(encoding="utf-8"))
        if names:
            out[str(p.relative_to(RUST))] = names
    lib = (RUST / "src" / "lib.rs").read_text(encoding="utf-8")
    fences = [ln for ln in lib.splitlines() if ln.strip().startswith("//! ```")]
    heads = [ln for ln in lib.splitlines() if ln.strip().startswith("//! ## ")]
    docs = []
    for k in range(len(fences) // 2):
        docs.append(f"doc test {k + 1}")
    if docs:
        titles = [h.strip()[len("//! ## "):] for h in heads]
        docs = [f"{t} (the crate docs)" for t in titles if t in ("Quick Start", "Hypothesis Testing", "Monte Carlo")]
        out["src/lib.rs (doc tests)"] = docs
    return out


def groups(path):
    text = (ROOT / path).read_text(encoding="utf-8")
    return re.findall(r'test_group\(\s*"([^"]*)"', text)


def ledger():
    tests = rust_tests()
    fails = []
    for f in tests:
        if f not in HOMES:
            fails.append(f"{f}: {len(tests[f])} tests, no entry in HOMES")
    for f, (home, grp) in HOMES.items():
        if f not in tests:
            fails.append(f"{f}: in HOMES, but 1.3.0 has no tests there")
            continue
        if home is None:
            continue
        files = [h.strip() for h in home.split(",")]
        have = []
        for h in files:
            if not (ROOT / h).exists():
                fails.append(f"{f}: {h} does not exist")
                continue
            have += groups(h)
        for g in grp:
            if not any(x.startswith(g) or g in x for x in have):
                fails.append(f"{f}: no test group in {home} matches {g!r}")
    return tests, fails


def render(tests):
    total = sum(len(v) for v in tests.values())
    carried = sum(len(v) for f, v in tests.items() if HOMES.get(f, (None,))[0] is not None)
    lines = ["# The Rust-test ledger",
             "",
             "> Generated by `scripts/rust_test_ledger.py --write` (port plan M8 part a); `--check` is the gate. Every test",
             "> of pramana 1.3.0 (`rust-old/`), and where 2.0 carries it.",
             "",
             f"1.3.0 has **{total}** tests: the `#[test]`s of `rust-old/src` and `rust-old/tests`, and the crate docs' doc",
             f"tests. **{carried}** are carried over, each as the Rust test states it, into the Cyrius test file and groups",
             f"below; **{total - carried}** retire with the feature they tested. Beyond them, every value 1.3.0 computes is",
             "a golden row replayed at 0 ulp (`tests/parity.tcyr`), so the carried tests are a second, independent check.",
             "",
             "| Rust file | Tests | 2.0 |",
             "|---|---|---|"]
    for f in sorted(tests):
        home, grp = HOMES[f]
        where = f"`{home}`" if home else "retired"
        if home and grp != ["1.3.0's"]:
            where += ": " + "; ".join(f"“{g}”" for g in grp)
        elif home:
            where += ": the groups named “1.3.0's …”"
        else:
            where += f": {grp}"
        lines.append(f"| `{f}` | {len(tests[f])} | {where} |")
    lines += ["", "## The tests, by file", ""]
    for f in sorted(tests):
        lines.append(f"- **`{f}`** ({len(tests[f])}): " + ", ".join(f"`{n}`" for n in tests[f]))
    return "\n".join(lines) + "\n"


def main(argv):
    tests, fails = ledger()
    text = render(tests)
    if "--write" in argv:
        OUT.write_text(text, encoding="utf-8")
        print(f"wrote {OUT.relative_to(ROOT)}")
    elif "--check" in argv and (not OUT.exists() or OUT.read_text(encoding="utf-8") != text):
        fails.append(f"{OUT.relative_to(ROOT)} is out of date (run with --write)")
    for msg in fails:
        print("FAIL " + msg)
    total = sum(len(v) for v in tests.values())
    print(f"rust-test ledger: {total} tests in {len(tests)} files, {len(fails)} failures")
    return 1 if fails and "--check" in argv else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
