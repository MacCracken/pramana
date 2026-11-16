"""p9_coremath_tables.py — generate probe P9's erf/erfc tables from CORE-MATH's sources (port plan §5 step 9e).

    python3 -I -B scripts/probes/p9_coremath_tables.py build/p9/coremath/erf.c build/p9/coremath/erfc.c \
        > scripts/probes/p9_erf_data.cyr

Reads CORE-MATH's binary64 erf.c and erfc.c (MIT; fetched into build/p9/coremath/, never committed) and writes every
`static const double` table and every hex constant the port needs as Cyrius words holding bit patterns: each literal
parsed by float.fromhex, so no digit is typed by hand. A table is named by the function it sits in, since both files
reuse names (`exceptions`, `p`, `threshold`). The tables erfc.c copies from erf.c are checked identical and written
once. The sources' sha256 sums go into the header, so the tables can be traced to the exact files.
"""

import hashlib
import re
import struct
import sys

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


def emit(name: str, lits: list) -> None:
    print(f"var {name}: i64[{len(lits)}] = {{")
    for i in range(0, len(lits), 4):
        row = ", ".join(f"0x{bits(x):016X}" for x in lits[i : i + 4])
        print(f"    {row}{',' if i + 4 < len(lits) else ''}")
    print("};")


def main() -> int:
    erf_path, erfc_path = sys.argv[1], sys.argv[2]
    erf, erfc = open(erf_path, encoding="utf-8").read(), open(erfc_path, encoding="utf-8").read()
    ta, tb = tables(erf), tables(erfc)
    for shared in ("C", "C2"):
        if ta[("", shared)] != tb[("", shared)]:
            sys.exit(f"table {shared} differs between erf.c and erfc.c")
    if ta[("cr_erf_fast", "c0")] != tb[("cr_erf_fast", "c0")]:
        sys.exit("cr_erf_fast's c0 differs")
    if ta[("cr_erf_accurate_tiny", "p")] != tb[("cr_erf_accurate_tiny", "p")]:
        sys.exit("cr_erf_accurate_tiny's p differs")
    plan = [
        ("_P9E_C", ta[("", "C")]),
        ("_P9E_C2", ta[("", "C2")]),
        ("_P9E_FAST_C0", ta[("cr_erf_fast", "c0")]),
        ("_P9E_TINY_EXC", ta[("cr_erf_accurate_tiny", "exceptions")]),
        ("_P9E_TINY_P", ta[("cr_erf_accurate_tiny", "p")]),
        ("_P9E_ACC_EXC", ta[("cr_erf_accurate", "exceptions")]),
        ("_P9E_T1", tb[("", "T1")]),
        ("_P9E_T2", tb[("", "T2")]),
        ("_P9E_Q1", tb[("", "Q_1")]),
        ("_P9E_T", tb[("", "T")]),
        ("_P9E_E2", tb[("", "E2")]),
        ("_P9E_TACC", tb[("", "Tacc")]),
        ("_P9E_ASFAST_THR", tb[("erfc_asympt_fast", "threshold")]),
        ("_P9E_ASACC_EXC", tb[("erfc_asympt_accurate", "exceptions")]),
        ("_P9E_ASACC_THR", tb[("erfc_asympt_accurate", "threshold")]),
    ]
    # cr_erfc_accurate has two `exceptions` tables, for x < 0 and for x > 0, in that order.
    acc = [m for m in re.finditer(r"static const double exceptions\[\]\[3\] = \{(.*?)\};",
                                  erfc[erfc.index("cr_erfc_accurate (double x)"):], re.S)]
    if len(acc) != 2:
        sys.exit("cr_erfc_accurate's two exception tables not found")
    for name, m in (("_P9E_ERFC_NEG_EXC", acc[0]), ("_P9E_ERFC_POS_EXC", acc[1])):
        plan.append((name, HEX.findall(re.sub(r"/\*.*?\*/|//[^\n]*", "", m.group(1), flags=re.S))))
    sha = lambda s: hashlib.sha256(s.encode("utf-8")).hexdigest()[:16]
    print("# p9_erf_data.cyr — CORE-MATH's binary64 erf and erfc tables and constants, for probe P9 "
          "(port plan §5 step 9e).")
    print("#")
    print("# GENERATED by scripts/probes/p9_coremath_tables.py from CORE-MATH's src/binary64/erf/erf.c (sha256")
    print(f"# {sha(erf)}…) and erfc/erfc.c ({sha(erfc)}…), fetched 2026-11-15. Do not edit. Each word is the bit")
    print("# pattern of the source's hex literal, read by float.fromhex. The sources' notice, which the MIT "
          "licence asks")
    print("# to keep:")
    print("#")
    notice = erf[: erf.index("*/") + 2]
    for row in notice.splitlines():
        print(("# " + row).rstrip())
    print()
    for name, lits in plan:
        print(f"# {len(lits)} words")
        emit(name, lits)
        print()
    print("# The #define and scalar constants, as typed globals holding their bits.")
    for name, lit in sorted({**defines(erf), **defines(erfc)}.items()):
        if name in ("MASK", "SIGN_MASK", "X0"):   # X0 only decides errno, which the port does not carry
            continue
        print(f"var _P9E_D_{name}: f64 = 0x{bits(lit):016X};   # {lit}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
