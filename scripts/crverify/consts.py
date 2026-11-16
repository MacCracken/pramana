"""Checks the constants of src/crmath_ext.cyr against their definitions (M1 part b4; port plan D2).

    python3 -I -B scripts/crverify/consts.py .            # exit 1 when a literal differs from its definition

Each constant is a triple-double written as three bit patterns: H = RN(c), M = RN(c − H), L = RN(c − H − M), from
Python's decimal at 90 digits. The file holds them as typed globals, so no digit is typed by hand without this check.
Stdlib only.
"""

import re
import struct
import sys
from decimal import Decimal, getcontext

getcontext().prec = 90


def bits(v: float) -> str:
    return "%016X" % struct.unpack("<Q", struct.pack("<d", v))[0]


def triple(c: Decimal) -> list:
    h = float(c)
    m = float(c - Decimal(h))
    low = float(c - Decimal(h) - Decimal(m))
    return [bits(h), bits(m), bits(low)]


CONSTANTS = {
    # 1/ln 2, for log2 = E + ln(m)·(1/ln 2)
    "_PMN_CR_INVLN2": triple(Decimal(1) / Decimal(2).ln()),
}


def main() -> int:
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    src = open(f"{root}/src/crmath_ext.cyr", encoding="utf-8").read()
    bad = 0
    for name, want in CONSTANTS.items():
        got = [re.search(rf"var {name}_{part}: f64 = 0x([0-9A-F]{{16}});", src) for part in "HML"]
        got = [g.group(1) if g else None for g in got]
        if got != want:
            print(f"{name}: src/crmath_ext.cyr has {got}, the definition gives {want}")
            bad += 1
        else:
            print(f"{name}: ok ({' '.join(want)})")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
