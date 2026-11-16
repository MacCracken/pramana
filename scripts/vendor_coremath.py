"""Vendors CORE-MATH's binary64 erf and erfc tables into src/crerf_data.cyr, and checks src/crerf.cyr's constants
(M2 part b1; port plan D3 = C, §3.4; ADR 009; ledger L8, L12).

pramana's erf and erfc are CORE-MATH's (Paul Zimmermann, MIT), ported without FMA by probe P9 (§5 step 9e) and moved
into src/ at M2 part b1. The sources are fetched, never committed, into build/coremath/:

    CORE-MATH's src/binary64/erf/erf.c, src/binary64/erf/erf.wc, src/binary64/erfc/erfc.c, src/binary64/erfc/erfc.wc
    (https://gitlab.inria.fr/core-math/core-math), the files whose sha256 sums SOURCES pins

Usage, from the repo root:

    python3 -I -B scripts/vendor_coremath.py .            # write src/crerf_data.cyr from build/coremath/
    python3 -I -B scripts/vendor_coremath.py . --check    # exit 1 when it differs, or a constant in src/crerf.cyr
                                                          # doesn't match the C literal its comment cites

What it writes and checks:
- **The tables:** every `static const double` table and `#define` constant the port reads, as words holding each
  literal's bit pattern, parsed by float.fromhex, so no digit is typed by hand. A table is named by the function it
  sits in, since both files reuse names. The tables erfc.c copies from erf.c are checked identical and written once.
- **The inline constants:** src/crerf.cyr writes a few thresholds inline as bit patterns. Each such 16-digit hex word
  carries a comment citing the C literal it encodes (`0x3C90000000000000   # 0x1p-54`). `--check` requires that
  comment on every one, the bits equal to float.fromhex of the literal, and the literal present in the sources. The
  constants that are pramana's own (the double-double 1/√2 that Φ uses, M2 part b2) are marked `checked by
  consts.py` and checked there, against their definitions.

Without build/coremath/ (CI checks out pramana alone), `--check` reports "unverified" and exits 0. Adapted from probe
P9's scripts/probes/p9_coremath_tables.py. Stdlib only.
"""

import hashlib
import os
import re
import struct
import sys

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else ".")
CHECK = "--check" in sys.argv
SRC = os.path.join(ROOT, "build", "coremath")
SOURCES = {
    "erf.c": "a984176d6b175699c0da69827776f6dab98fad9cf17fd8c8af53558f2c85b39f",
    "erfc.c": "53b96001983317d267b3d9867995a74fe21fffe44931d53cc51d0fd688fe9964",
}
OUT = os.path.join(ROOT, "src", "crerf_data.cyr")
PORT = os.path.join(ROOT, "src", "crerf.cyr")
HEX = re.compile(r"-?0x[0-9a-fA-F]+(?:\.[0-9a-fA-F]*)?p[-+]?\d+")


def bits(lit: str) -> int:
    return struct.unpack("<Q", struct.pack("<d", float.fromhex(lit)))[0]


def tables(text: str) -> dict:
    """{(enclosing function, or "" at file scope; name): [literal, …]} for every `static const double name[…] = {…};`.
    The scope comes from the brace depth of the text with every table body blanked out (comments too, since they
    can hold braces)."""
    table = re.compile(r"static const double (\w+)\s*(\[[^\]]*\])+\s*=\s*\{(.*?)\};", re.S)
    found = list(table.finditer(text))
    masked = list(re.sub(r"/\*.*?\*/|//[^\n]*", lambda m: " " * len(m.group(0)), text, flags=re.S))
    for m in found:
        for i in range(m.start(3), m.end(3)):
            masked[i] = " "
    masked = "".join(masked)
    head = re.compile(r"(\w+)\s*\([^;{)]*\)\s*$")
    depth, func, scope = 0, "", {}
    starts = {m.start(): m for m in found}
    for i, ch in enumerate(masked):
        if i in starts:
            scope[i] = func if depth > 0 else ""
        if ch == "{":
            if depth == 0:
                h = head.search(masked[max(0, i - 200) : i])
                func = h.group(1) if h else ""
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                func = ""
    out = {}
    for m in found:
        body = re.sub(r"/\*.*?\*/|//[^\n]*", "", m.group(3), flags=re.S)
        dims = re.findall(r"\[(\d*)\]", m.group(0)[: m.group(0).index("=")])
        rows = re.findall(r"\{([^{}]*)\}", body)
        if rows and len(dims) == 2 and dims[1]:
            # A two-dimensional table: C zero-pads a short row to the declared width, and so does the port.
            width = int(dims[1])
            lits = []
            for row in rows:
                cells = HEX.findall(row)
                if len(cells) > width:
                    sys.exit(f"{m.group(1)}: a row of {len(cells)} entries in width {width}")
                lits += cells + ["0x0p+0"] * (width - len(cells))
        else:
            lits = HEX.findall(body)
        if lits:
            out[(scope[m.start()], m.group(1))] = lits
    return out


def defines(text: str) -> dict:
    """The #define hex constants and the scalar `static const double NAME = hex;` ones (CH, CL)."""
    out = {m.group(1): m.group(2) for m in re.finditer(r"#define (\w+) (-?0x[0-9a-fA-F.]+p[-+]?\d+)", text)}
    out.update({m.group(1): m.group(2)
                for m in re.finditer(r"static const double (\w+) = (-?0x[0-9a-fA-F.]+p[-+]?\d+);", text)})
    return out


def render(erf: str, erfc: str) -> str:
    ta, tb = tables(erf), tables(erfc)
    for shared in ("C", "C2"):
        if ta[("", shared)] != tb[("", shared)]:
            sys.exit(f"table {shared} differs between erf.c and erfc.c")
    if ta[("cr_erf_fast", "c0")] != tb[("cr_erf_fast", "c0")]:
        sys.exit("cr_erf_fast's c0 differs")
    if ta[("cr_erf_accurate_tiny", "p")] != tb[("cr_erf_accurate_tiny", "p")]:
        sys.exit("cr_erf_accurate_tiny's p differs")
    plan = [
        ("_PMN_CM_C", ta[("", "C")]),
        ("_PMN_CM_C2", ta[("", "C2")]),
        ("_PMN_CM_FAST_C0", ta[("cr_erf_fast", "c0")]),
        ("_PMN_CM_TINY_EXC", ta[("cr_erf_accurate_tiny", "exceptions")]),
        ("_PMN_CM_TINY_P", ta[("cr_erf_accurate_tiny", "p")]),
        ("_PMN_CM_ACC_EXC", ta[("cr_erf_accurate", "exceptions")]),
        ("_PMN_CM_T1", tb[("", "T1")]),
        ("_PMN_CM_T2", tb[("", "T2")]),
        ("_PMN_CM_Q1", tb[("", "Q_1")]),
        ("_PMN_CM_T", tb[("", "T")]),
        ("_PMN_CM_E2", tb[("", "E2")]),
        ("_PMN_CM_TACC", tb[("", "Tacc")]),
        ("_PMN_CM_ASFAST_THR", tb[("erfc_asympt_fast", "threshold")]),
        ("_PMN_CM_ASACC_EXC", tb[("erfc_asympt_accurate", "exceptions")]),
        ("_PMN_CM_ASACC_THR", tb[("erfc_asympt_accurate", "threshold")]),
    ]
    # cr_erfc_accurate has two `exceptions` tables, for x < 0 and for x > 0, in that order.
    acc = list(re.finditer(r"static const double exceptions\[\]\[3\] = \{(.*?)\};",
                           erfc[erfc.index("cr_erfc_accurate (double x)"):], re.S))
    if len(acc) != 2:
        sys.exit("cr_erfc_accurate's two exception tables not found")
    for name, m in (("_PMN_CM_ERFC_NEG_EXC", acc[0]), ("_PMN_CM_ERFC_POS_EXC", acc[1])):
        plan.append((name, HEX.findall(re.sub(r"/\*.*?\*/|//[^\n]*", "", m.group(1), flags=re.S))))
    out = [
        "# crerf_data.cyr — CORE-MATH's binary64 erf and erfc tables and constants (M2 part b1; port plan §3.4, ADR 009).",
        "#",
        "# GENERATED by scripts/vendor_coremath.py from CORE-MATH's src/binary64/erf/erf.c and erfc/erfc.c, the files",
        f"# whose sha256 sums begin {SOURCES['erf.c'][:16]} and {SOURCES['erfc.c'][:16]}. Do not edit. Each word is "
        "the bit",
        "# pattern of the source's hex literal, read by float.fromhex. The sources' notice, which the MIT licence asks "
        "to",
        "# keep:",
        "#",
    ]
    notice = erf[: erf.index("*/") + 2]
    out += [("# " + row).rstrip() for row in notice.splitlines()]
    out.append("")
    for name, lits in plan:
        out.append(f"# {len(lits)} words")
        out.append(f"var {name}: i64[{len(lits)}] = {{")
        for i in range(0, len(lits), 4):
            row = ", ".join(f"0x{bits(x):016X}" for x in lits[i : i + 4])
            out.append(f"    {row}{',' if i + 4 < len(lits) else ''}")
        out.append("};")
        out.append("")
    out.append("# The #define and scalar constants, as typed globals holding their bits.")
    for name, lit in sorted({**defines(erf), **defines(erfc)}.items()):
        if name in ("MASK", "SIGN_MASK", "X0"):   # X0 only decides errno, which the port does not carry
            continue
        out.append(f"var _PMN_CM_D_{name}: f64 = 0x{bits(lit):016X};   # {lit}")
    return "\n".join(out) + "\n"


def inline_failures(port: str, sources: str) -> list:
    """Every 16-digit hex word in src/crerf.cyr's code is either an integer word the sources write verbatim
    (`0x7ff0000000000000`), or carries a `# <C hex literal>` comment on its line that encodes it (or its absolute
    value, for an integer compare on a sign-cleared word) and that occurs in the sources."""
    fails = []
    lits_in_src = {bits(m) for m in HEX.findall(sources)} | {bits(m.lstrip("-")) for m in HEX.findall(sources)}
    words_in_src = {int(w, 16) for w in re.findall(r"0x([0-9a-fA-F]{16})(?![0-9a-fA-F.p])", sources)}
    for n, line in enumerate(port.splitlines(), 1):
        code, _, comment = line.partition("#")
        if "checked by consts.py" in comment:
            continue                    # pramana's own constants (1/√2 for Φ): scripts/crverify/consts.py checks them
        for word in re.findall(r"0x([0-9A-Fa-f]{16})\b", code):
            w = int(word, 16)
            if w in words_in_src:
                continue
            cited = HEX.findall(comment)
            if not cited:
                fails.append(f"src/crerf.cyr:{n}: 0x{word} has no `# <C literal>` comment")
                continue
            if not any(bits(c) == w or bits(c.lstrip("-")) == w for c in cited):
                fails.append(f"src/crerf.cyr:{n}: 0x{word} is not the bits of {cited[0]}")
            elif all(re.fullmatch(r"0x1p[-+]?\d+", c) for c in cited):
                # A power of two the port uses itself (the tiny path's scalings): its bits are its whole meaning.
                continue
            elif not any(bits(c) in lits_in_src or bits(c.lstrip("-")) in lits_in_src for c in cited):
                fails.append(f"src/crerf.cyr:{n}: {cited[0]} does not occur in the CORE-MATH sources")
    return fails


def main() -> int:
    texts = {}
    for name, digest in SOURCES.items():
        path = os.path.join(SRC, name)
        if not os.path.exists(path):
            if CHECK:
                print(f"vendor_coremath: unverified (no {os.path.relpath(path, ROOT)}; fetch CORE-MATH's sources)")
                return 0
            sys.exit(f"missing {path}: fetch CORE-MATH's src/binary64/erf{{,c}}/ files into build/coremath/")
        raw = open(path, "rb").read()
        if hashlib.sha256(raw).hexdigest() != digest:
            sys.exit(f"{path}: sha256 {hashlib.sha256(raw).hexdigest()[:16]}…, expected {digest[:16]}…")
        texts[name] = raw.decode("utf-8")
    text = render(texts["erf.c"], texts["erfc.c"])
    fails = []
    if CHECK:
        if not os.path.exists(OUT) or open(OUT, encoding="utf-8").read() != text:
            fails.append("src/crerf_data.cyr differs from what the sources give")
    else:
        open(OUT, "w", encoding="utf-8").write(text)
        print(f"wrote src/crerf_data.cyr ({text.count(chr(10))} lines)")
    if os.path.exists(PORT):
        fails += inline_failures(open(PORT, encoding="utf-8").read(), texts["erf.c"] + texts["erfc.c"])
    for f in fails:
        print("FAILED:", f)
    if CHECK:
        print("vendor_coremath: " + ("ok (CORE-MATH erf.c, erfc.c; every inline constant cited)" if not fails
                                     else f"{len(fails)} failures"))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
