"""Vendors kimiya's correctly rounded f64 primitives into src/crmath_data.cyr and src/crmath.cyr (ledger L1, L12).

Port plan D2 and §8.1: every transcendental pramana computes goes through a correctly rounded function (ADR 009), and
ganita has no correctly rounded tier (ledger L1). kimiya 2.0.1 has exp, expm1, ln, pow, sin, cos and atan2, two-phase
(Ziv) on MPFR-generated tables, from + − × ÷, `f64_sqrt` and integer bit operations only. Probe P9 also uses its
double-double ln core and exp core, and its exact two-sum, Dekker product and round-to-odd sum (the exact fma of
ledger L12), which M2's special functions need beside the rounded functions.

After soorat's scripts/vendor_crmath.py: this copies the dependency closure of ROOTS (functions with their doc
comments, and the tables of src/mathx_data.cyr they read) in kimiya's source order, from kimiya's git tag VERSION (not
its working tree), renaming `_km_` → `_pmn_km_` and `_KM_` → `_PMN_KM_` so nothing collides with a consumer that
includes kimiya too. Usage, from the repo root:

    python3 -I -B scripts/vendor_crmath.py .            # write src/crmath_data.cyr and src/crmath.cyr from ../kimiya
    python3 -I -B scripts/vendor_crmath.py . --check    # exit 1 when either differs from what ../kimiya gives

Without ../kimiya (CI checks out pramana alone), `--check` reports "unverified" and exits 0. Stdlib only.
"""

import os
import re
import subprocess
import sys

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else ".")
CHECK = "--check" in sys.argv
KIMIYA = os.path.join(os.path.dirname(ROOT), "kimiya")
VERSION = "2.0.1"
ROOTS = [
    # the primitives (Rust's f64 methods; atan is atan2(x, 1), the same value)
    "_km_cr_exp", "_km_cr_expm1", "_km_cr_ln", "_km_cr_pow", "_km_cr_sin", "_km_cr_cos", "_km_cr_atan2",
    # the double-double cores and exact arithmetic the special functions build on (probe P9; ledger L12)
    "_km_ln_core", "_km_exp_core", "_km_cr_pow2", "_km_two_sum", "_km_fast_two_sum", "_km_two_prod", "_km_rn3",
]
OUT_DATA = os.path.join(ROOT, "src", "crmath_data.cyr")
OUT_FNS = os.path.join(ROOT, "src", "crmath.cyr")


def items(text, kind):
    """{name: (start, end)} of the top-level `fn`, `var` or `const` items of text, each starting at its doc
    comment."""
    out = {}
    lines = text.split("\n")
    offs = [0]
    for ln in lines:
        offs.append(offs[-1] + len(ln) + 1)
    pat = re.compile({"fn": r"^fn (\w+)\(", "var": r"^var (\w+)", "const": r"^const (\w+)"}[kind])
    for i, ln in enumerate(lines):
        m = pat.match(ln)
        if not m:
            continue
        j = i
        while j > 0 and lines[j - 1].startswith("#"):
            j -= 1
        k = i
        if kind == "fn":
            while lines[k] != "}":
                k += 1
        elif "{" in ln and "}" not in ln:
            while not lines[k].startswith("}"):
                k += 1
        out[m.group(1)] = (offs[j], offs[k + 1])
    return out


def git_show(path):
    return subprocess.run(["git", "-C", KIMIYA, "show", f"{VERSION}:{path}"], capture_output=True, text=True,
                          check=True).stdout


def rename(text):
    """kimiya's names to pramana's. The prefix is 4 characters longer, so a comment line it pushes past 120 columns is
    wrapped at its last space before them (lint's limit); nothing else changes."""
    text = re.sub(r"\b_km_", "_pmn_km_", text)
    text = re.sub(r"\b_KM_", "_PMN_KM_", text)
    out = []
    for line in text.split("\n"):
        stripped = line.lstrip()
        if len(line) <= 120 or not stripped.startswith("#"):
            out.append(line)
            continue
        lead = line[: len(line) - len(stripped)] + "# "
        cut = line.rfind(" ", 0, 121)
        out.append(line[:cut])
        out.append(lead + line[cut + 1:])
    return "\n".join(out)


def main():
    if not os.path.isdir(os.path.join(KIMIYA, ".git")):
        if CHECK:
            print("vendor_crmath: unverified (../kimiya is not checked out)")
            return 0
        print("vendor_crmath: ../kimiya not found", file=sys.stderr)
        return 1
    try:
        src = git_show("src/mathx.cyr")
        data = git_show("src/mathx_data.cyr")
        commit = subprocess.run(["git", "-C", KIMIYA, "rev-parse", "--short", VERSION], capture_output=True,
                                text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"vendor_crmath: can't read kimiya's tag {VERSION}: {e}", file=sys.stderr)
        return 1
    fns = items(src, "fn")
    dvars = items(data, "var")
    consts = items(src, "const")  # mathx.cyr's own compile-time constants (`_KM_INF`), copied with the functions
    missing = [r for r in ROOTS if r not in fns]
    if missing:
        print(f"vendor_crmath: kimiya {VERSION} has no {', '.join(missing)}", file=sys.stderr)
        return 1
    seen, need, cneed, stack = set(), set(), set(), list(ROOTS)
    while stack:
        name = stack.pop()
        if name in seen:
            continue
        seen.add(name)
        a, b = fns[name]
        body = src[src.index("fn " + name + "(", a):b]
        for ref in re.findall(r"\b(_[kK][mM]_\w+)\b", body):
            if ref in fns and ref not in seen:
                stack.append(ref)
            elif ref in dvars:
                need.add(ref)
            elif ref in consts:
                cneed.add(ref)
    order_fns = sorted(seen, key=lambda n: fns[n][0])
    order_vars = sorted(need, key=lambda n: dvars[n][0])
    order_consts = sorted(cneed, key=lambda n: consts[n][0])
    origin = f"kimiya {VERSION} (tag {VERSION}, {commit})"
    data_head = [
        "# crmath_data.cyr — the tables of pramana's correctly rounded primitives, vendored from",
        f"# {origin} src/mathx_data.cyr, where they are generated from MPFR 4.2.2.",
        "# GENERATED by scripts/vendor_crmath.py: do not edit. Regenerate: python3 -I -B scripts/vendor_crmath.py .",
        "# (--check verifies it against ../kimiya)",
        "#",
        f"# The {len(order_vars)} tables src/crmath.cyr reads, renamed `_KM_` → `_PMN_KM_`. kimiya is GPL-3.0-only, "
        "as pramana is.",
        "# UPSTREAM(L1): ganita:docs/development/issues/2026-11-09-correctly-rounded-exp-sin-cos-hypot.md",
        "#   replace with ganita's correctly rounded tier once it has one; parity: every class C row at 0 ulp",
        "",
        "",
    ]
    fns_head = [
        f"# crmath.cyr — pramana's correctly rounded f64 primitives, vendored from {origin}",
        "# src/mathx.cyr. GENERATED by scripts/vendor_crmath.py: do not edit. Regenerate:",
        "# python3 -I -B scripts/vendor_crmath.py .   (--check verifies it against ../kimiya)",
        "#",
        "# Port plan D2 and §8.1 (ADR 009): `_pmn_km_cr_exp`, `_pmn_km_cr_expm1`, `_pmn_km_cr_ln`, `_pmn_km_cr_pow`,",
        "# `_pmn_km_cr_sin`, `_pmn_km_cr_cos` and `_pmn_km_cr_atan2` are kimiya's `_km_cr_*` renamed",
        "# (`_km_` → `_pmn_km_`, `_KM_` → `_PMN_KM_`). Each is two-phase (Ziv): a fast path with a proven error bound",
        "# and a rounding test, then a triple-double accurate path. Also vendored: the double-double ln and exp cores,",
        "# and the exact two-sum, Dekker product and round-to-odd sum (probe P9; the exact fma of ledger L12).",
        f"# {len(order_fns)} functions in all, in kimiya's source order; the ADR and section numbers in the comments "
        "below are",
        "# kimiya's. Its tables are src/crmath_data.cyr. kimiya is GPL-3.0-only, as pramana is.",
        "# UPSTREAM(L1): ganita:docs/development/issues/2026-11-09-correctly-rounded-exp-sin-cos-hypot.md",
        "#   replace with ganita's correctly rounded tier once it has one; parity: every class C row at 0 ulp",
        "# UPSTREAM(L12): cyrius:docs/development/issues/2026-11-15-no-scalar-fma.md",
        "#   `_pmn_km_two_prod` with `_pmn_km_rn3` is the exact fma; replace with f64_fma; the bits must not change",
        "",
        "",
    ]
    # Only the vendored text is renamed: the headers name kimiya's originals.
    data_text = "\n".join(data_head) + rename("".join(data[dvars[n][0]:dvars[n][1]] + "\n" for n in order_vars))
    fns_text = "\n".join(fns_head) + rename("".join(src[consts[n][0]:consts[n][1]] + "\n" for n in order_consts)
                                             + "".join(src[fns[n][0]:fns[n][1]] + "\n" for n in order_fns))
    data_text = data_text.rstrip("\n") + "\n"
    fns_text = fns_text.rstrip("\n") + "\n"
    if CHECK:
        stale = []
        for path, text in ((OUT_DATA, data_text), (OUT_FNS, fns_text)):
            try:
                with open(path, encoding="utf-8") as f:
                    if f.read() != text:
                        stale.append(os.path.relpath(path, ROOT))
            except FileNotFoundError:
                stale.append(os.path.relpath(path, ROOT))
        if stale:
            print(f"vendor_crmath: {', '.join(stale)} stale (regenerate: python3 -I -B scripts/vendor_crmath.py .)")
            return 1
        print(f"vendor_crmath: ok ({origin}: {len(order_fns)} functions, {len(order_vars)} tables)")
        return 0
    for path, text in ((OUT_DATA, data_text), (OUT_FNS, fns_text)):
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        print(f"wrote {os.path.relpath(path, ROOT)}: {text.count(chr(10))} lines")
    print(f"{len(order_fns)} functions, {len(order_vars)} tables from {origin}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
