"""p9_beta_exact.py — exact I_x(a, b) for probe P9's reference check (port plan §5 step 9d-3).

    python3 -I -B scripts/probes/p9_beta_exact.py build/p9/beta_rust.txt > build/p9/beta_exact_rows.txt

Reads the accuracy baseline's integer-shape rows (`a b k p q`, x = k/2^20, so 1 − x is exact) and writes one
`bref_vs_exact a b x y <decimal>` row each: the exact value I_x(a, b) = Σ_{j=a}^{a+b−1} C(a+b−1, j) x^j (1−x)^(a+b−1−j)
as a rational (scripts/port-baseline/beta_check.py's sum), printed to 120 significant digits. check.c then reports
how far its 320-bit continued fraction is from each.
"""

import struct
import sys
from decimal import Decimal, getcontext
from fractions import Fraction
from math import comb


def hexbits(v: float) -> str:
    return f"{struct.unpack('<Q', struct.pack('<d', v))[0]:016x}"


def main() -> int:
    getcontext().prec = 120
    for line in open(sys.argv[1], encoding="utf-8"):
        a, b, k, _, _ = line.split()
        a, b, k = int(a), int(b), int(k)
        n, d = a + b - 1, 1 << 20
        num = sum(comb(n, j) * k**j * (d - k) ** (n - j) for j in range(a, n + 1))
        exact = Fraction(num, d**n)
        if exact == 0:
            continue
        value = Decimal(exact.numerator) / Decimal(exact.denominator)
        x = k / d
        print(f"bref_vs_exact {hexbits(float(a))} {hexbits(float(b))} {hexbits(x)} {hexbits(1.0 - x)} {value:.119e}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
