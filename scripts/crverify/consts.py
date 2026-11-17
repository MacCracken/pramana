"""Checks pramana's own constants against their definitions (M1 part b4, M2 part b2, M3 part e; port plan D2).

    python3 -I -B scripts/crverify/consts.py .            # exit 1 when a literal differs from its definition

Each constant is a triple-double written as three bit patterns, H = RN(c), M = RN(c − H), L = RN(c − H − M), or a
double-double, H and L = RN(c − H), or one double, RN(c), from Python's decimal at 90 digits. The files hold them as
typed globals named <name>_H, <name>_M, <name>_L, or <name> for one double, so no digit is typed by hand without this
check. Stdlib only.
"""

import math
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


def double(c: Decimal) -> list:
    h = float(c)
    return [bits(h), bits(float(c - Decimal(h)))]


def single(c: Decimal) -> list:
    return [bits(float(c))]


CONSTANTS = {  # name: (file, parts, words)
    # 1/ln 2, for log2 = E + ln(m)·(1/ln 2)
    "_PMN_CR_INVLN2": ("src/crmath_ext.cyr", "HML", triple(Decimal(1) / Decimal(2).ln())),
    # 1/√2, for Φ's argument −z/√2 as a double-double (M2 part b2)
    "_PMN_CM_RS2": ("src/crerf.cyr", "HL", double(Decimal(1) / Decimal(2).sqrt())),
    # ln 2π as the port computes it: the correctly rounded ln of the double 2π, 2·RN(π), as Rust's `(2.0 * PI).ln()`
    # gives it, not RN(ln 2π) of the real 2π (MultivariateNormal's log density, M3 part e)
    "_PMN_MVN_LN_2PI": ("src/dist_mvn.cyr", "", single((2 * Decimal(math.pi)).ln())),
}


def main() -> int:
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    bad = 0
    for name, (path, parts, want) in CONSTANTS.items():
        src = open(f"{root}/{path}", encoding="utf-8").read()
        names = [f"{name}_{part}" for part in parts] if parts else [name]
        got = [re.search(rf"var {n}: f64 = 0x([0-9A-F]{{16}});", src) for n in names]
        got = [g.group(1) if g else None for g in got]
        if got != want:
            print(f"{name}: {path} has {got}, the definition gives {want}")
            bad += 1
        else:
            print(f"{name}: ok ({' '.join(want)})")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
