"""`combinatorics.tsv`: 1.3.0's counting functions (M1 part e2): factorial, permutations and combinations on probe P1's
grid and 200 seeded pairs (a u128 as its low then high word, each an `i:`), and stirling_approx on 0 … 200, every
power of two and seeded words (class C: glibc's pow, corrected in combinatorics.cr.tsv by validate.py)."""
from .grids import Rng
from .models import V
from .tsv import Module, err, f, i

U64 = (1 << 64) - 1


def outcome(fn, n, r=0):
    kind, val = V.counting(fn, n, r)
    if kind == "err":
        return [err(val)]
    return [i(val & U64), i(val >> 64)]


def r_grid(n):
    vs = [0, 1, 2, 3, 5, 10, 20, 33, 34, 35, 64, 65, 66, 67]
    vs += [max(n - 2, 0), max(n - 1, 0), n, min(n + 1, U64)]
    return sorted(set(vs))


def emit():
    m = Module("combinatorics")
    ns = list(range(41)) + [130, 131, 132, 133, 1 << 32, 1 << 63, U64]
    for n in ns:
        m.row("combinatorics::factorial", "A", [i(n)], outcome("combinatorics::factorial", n))
    pairs = [(n, r) for n in ns for r in r_grid(n)]
    g = Rng(0xA54FF53A5F1D36F1)
    for _ in range(200):
        n = g.int(0, 140)
        r = g.int(0, 141)
        pairs.append((n, r))
    for fn in ("combinatorics::permutations", "combinatorics::combinations"):
        for n, r in pairs:
            m.row(fn, "A", [i(n), i(r)], outcome(fn, n, r))
    sn = list(range(201)) + [1 << k for k in range(64)] + [U64] + [g.word() >> (k % 64) for k in range(60)]
    for n in sorted(set(sn)):
        m.row("combinatorics::stirling_approx", "C", [i(n)], [f(V.stirling(n, V._libm.pow))])
    return m
