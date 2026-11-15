"""p6_model.py — P6: compares Rust's median / percentile with selection from a stable sort (port plan §5 step 9).

    python3 -I -B scripts/probes/p6_model.py build/p6_rust.tsv

The model is rust-old/src/descriptive.rs's arithmetic, with the k-th element and its successor taken from a stable
sort of the input (Python's `sorted`: −0 and +0 compare equal and keep their input order). A mismatch can only be the
sign of a zero, because equal non-zero floats have identical bits. Reports how many results differ, and for which
functions.
"""

import math
import struct
import sys


def f(h):
    return struct.unpack("<d", struct.pack("<Q", int(h, 16)))[0]


def bits(x):
    return struct.unpack("<Q", struct.pack("<d", x))[0]


def select_pair(data, k):
    s = sorted(data)  # stable; −0 == +0
    return s[k], (s[k + 1] if k + 1 < len(s) else None)


def median(data):
    n = len(data)
    if n % 2 == 0:
        lo, hi = select_pair(data, n // 2 - 1)
        return (lo + (lo if hi is None else hi)) / 2.0
    return select_pair(data, n // 2)[0]


def percentile(data, p):
    n = len(data)
    if n == 1:
        return data[0]
    rank = (p / 100.0) * (n - 1)
    lower = min(int(math.floor(rank)), n - 1)
    frac = rank - lower
    lo, hi = select_pair(data, lower)
    if frac == 0.0:
        return lo
    hi = lo if hi is None else hi
    return lo * (1.0 - frac) + hi * frac


def main():
    total = differ = 0
    by = {}
    examples = []
    for line in open(sys.argv[1], encoding="utf-8"):
        case, fn, p, ins, out = line.rstrip("\n").split("\t")
        data = [f(h) for h in ins.split(" ")]
        want = median(data) if fn == "median" else percentile(data, float(p))
        got = int(out, 16)
        total += 1
        key = fn if fn == "median" else f"percentile({p})"
        t = by.setdefault(key, [0, 0])
        t[0] += 1
        if got != bits(want):
            differ += 1
            t[1] += 1
            assert want == 0.0 and f(out) == 0.0, (case, fn, p, want, f(out))  # only a zero's sign can differ
            if len(examples) < 3:
                examples.append(f"{fn}({p}) on {[repr(x) for x in data]}: Rust {f(out)!r}, stable {want!r}")
    for k in by:
        print(f"  {k:18s} {by[k][1]:5d} of {by[k][0]:5d} differ (sign of zero)")
    print(f"P6: {differ} of {total} results differ from stable selection, every one only in the sign of a zero")
    for e in examples:
        print("  e.g.", e)


if __name__ == "__main__":
    main()
