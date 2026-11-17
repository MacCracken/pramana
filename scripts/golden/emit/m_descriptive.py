"""`descriptive.tsv` (M4): 1.3.0's descriptive.rs. Part a1, the fourteen statistics on empty, single, special-valued,
constant, overflowing, NaN- and ∞-bearing slices; probe P6's zero-heavy slices (elements from −0, +0, −1, 1 and 2,
lengths 1–24); seeded slices to 64 elements, tie-heavy and sorted ones, and lengths 1,000 and 1,001 that drive the
selection's partitions and its fallback; the percentile on a p grid with its range edges and their neighbours. All class
A: arithmetic and the correctly rounded √. A slice is `v:n` and its values; a tuple its elements."""
from pathlib import Path

from .grids import INF, MAXF, MIN_POSITIVE, NAN, TINY, Rng
from .models import V
from .tsv import Module, bits, e, f, from_bits, from_model, none, parsed, v

ROOT = Path(__file__).resolve().parents[3]
_PRIMS = None


def prims():
    global _PRIMS
    if _PRIMS is None:
        _PRIMS = V.dist_prims(ROOT)[0]
    return _PRIMS


def slice_(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def row(m, table, fn, cls, ins):
    sl, sc = V.take_slices(parsed(ins))
    m.row(fn, cls, ins, from_model(table[fn](prims(), sl, sc)))


ONE_ARG = ["sum", "mean", "median", "mode", "variance", "std_dev", "skewness", "kurtosis", "quartiles", "iqr", "min",
           "max", "range"]
ORDER = ["median", "quartiles", "iqr", "min", "max", "range", "mode"]
P_GRID = [0.0, -0.0, 10.0, 25.0, 33.3, 50.0, 75.0, 90.0, 100.0, 1e-300, TINY, 37.5, 62.5, 99.99999999999999,
          from_bits(bits(100.0) + 1), from_bits(bits(0.0) | (1 << 63) | 1), -1.0, 150.0, NAN, INF, -INF]
P_SHORT = [0.0, 10.0, 25.0, 33.3, 50.0, 75.0, 90.0, 100.0]


def all_stats(m, xs, ps=P_SHORT, only=None):
    for name in (only or ONE_ARG):
        row(m, V.DESC, "descriptive::" + name, "A", slice_(xs))
    for p in ps:
        row(m, V.DESC, "descriptive::percentile", "A", slice_(xs) + [f(p)])


def seeded(g, n, kind):
    if kind == 0:
        return [g.uniform(-10.0, 10.0) for _ in range(n)]
    if kind == 1:
        return [g.signed_log_uniform(-300.0, 300.0) for _ in range(n)]
    if kind == 2:   # heavy ties: small integers
        return [float(g.int(0, 4)) - 2.0 for _ in range(n)]
    if kind == 3:   # ties on a few bit patterns, ±0 among them
        pool = [0.0, -0.0, 1.5, -1.5, 3.0]
        return [pool[g.int(0, 4)] for _ in range(n)]
    return [g.log_uniform(-5.0, 5.0) for _ in range(n)]


def emit():
    m = Module("descriptive")
    g = Rng(0x9E3779B97F4A7C15)
    sp = [0.0, -0.0, 1.0, -1.0, 0.5, -2.5, 1e-300, 1e300, MIN_POSITIVE, TINY, MAXF, -MAXF, NAN, INF, -INF]
    # empty and single slices
    all_stats(m, [], ps=P_GRID)
    for x in sp:
        all_stats(m, [x], ps=[50.0, 0.0, 100.0, NAN])
    # pairs and triples of specials
    for a in sp:
        for b in sp:
            all_stats(m, [a, b], ps=[25.0, 50.0])
    for k in range(200):
        xs = [sp[g.int(0, len(sp) - 1)] for _ in range(3)]
        all_stats(m, xs, ps=[50.0])
    # constant slices (zero variance), overflow and cancellation
    for c in (0.0, -0.0, 1.0, 3.25, -7.0, MAXF, TINY):
        for n in (2, 3, 7, 16):
            all_stats(m, [c] * n)
    for xs in ([MAXF, MAXF], [MAXF, -MAXF], [1e308, 1e308, -1e308], [1e200, 1.0, -1e200], [1e16, 1.0, -1e16],
               [1.0, 1e-16, 1e-16, 1e-16], [INF, 1.0], [INF, -INF], [NAN, 1.0, 2.0], [1.0, NAN], [-INF, -INF, 0.0]):
        all_stats(m, xs, ps=P_GRID)
    # probe P6's zero-heavy slices: the stable rule's zero sign
    pool = [-0.0, 0.0, -1.0, 1.0, 2.0]
    for n in range(1, 25):
        for _ in range(30):
            xs = [pool[g.int(0, 4)] for _ in range(n)]
            all_stats(m, xs, only=ORDER)
    # seeded slices
    for n in list(range(2, 41)) + [48, 63, 64]:
        for kind in range(5):
            all_stats(m, seeded(g, n, kind))
    # sorted, reversed, organ-pipe, all-equal and tie-heavy long slices: the selection's partitions and fallback
    for n in (1000, 1001):
        base = seeded(g, n, 0)
        shapes = [base, sorted(base), sorted(base, reverse=True),
                  sorted(base[: n // 2]) + sorted(base[n // 2:], reverse=True),
                  [2.0] * n, seeded(g, n, 2), seeded(g, n, 3), [float(j % 7) for j in range(n)]]
        for xs in shapes:
            all_stats(m, xs, ps=P_GRID)
    kde_rows(m, Rng(0xA54FF53A5F1D36F1))
    corr_rows(m, Rng(0x510E527FADE682D1))
    pca_rows(m, Rng(0x9B05688C2B3E6C1F))
    return m


def pca_row(m, vs):
    ins = []
    for x in vs:
        ins += slice_(x)
    row(m, V.DESC, "descriptive::pca", "C", ins)


def pca_rows(m, g):
    """`pca(variables)` (part a3): class C, its Jacobi's angle from atan, cos and sin."""
    x = [1.0, 2.0, 3.0, 4.0, 5.0]
    pca_row(m, [])
    pca_row(m, [x])
    pca_row(m, [[1.0], [2.0]])
    pca_row(m, [x, [1.0, 2.0]])
    pca_row(m, [x, [1.0, NAN, 3.0, 4.0, 5.0]])
    pca_row(m, [x, [1.0, INF, 3.0, 4.0, 5.0]])
    pca_row(m, [x, [2.1, 3.9, 6.1, 7.9, 10.1]])
    xs = [float(j) for j in range(100)]
    pca_row(m, [xs, [2.0 * v + 0.01 * V._libm.sin(v) for v in xs]])
    pca_row(m, [[1.0, 3.0, 2.0, 5.0, 4.0], [5.0, 1.0, 4.0, 2.0, 3.0]])
    pca_row(m, [x, [5.0, 3.0, 1.0, 4.0, 2.0]])
    pca_row(m, [x, [3.0] * 5])                                    # a constant variable: a zero eigenvalue
    pca_row(m, [[2.0] * 4, [3.0] * 4])                            # all constant: ratios 0
    pca_row(m, [x, x])                                            # duplicated: rank 1
    pca_row(m, [x, [-v for v in x]])
    pca_row(m, [x, x, x])
    pca_row(m, [[1e300, -1e300, 0.0], [1.0, 2.0, 3.0]])
    pca_row(m, [[1e-300, 2e-300, 4e-300], [1.0, 2.0, 3.0]])
    pca_row(m, [[-0.0, 0.0], [0.0, -0.0]])
    for _ in range(60):
        p = g.int(2, 6)
        n = g.int(2, 40)
        base = [g.uniform(-10.0, 10.0) for _ in range(n)]
        vs = []
        for j in range(p):
            r = g.int(0, 2)
            if r == 0:
                vs.append([g.uniform(-10.0, 10.0) for _ in range(n)])
            elif r == 1:
                a = g.uniform(-3.0, 3.0)
                vs.append([a * b + g.uniform(-0.5, 0.5) for b in base])
            else:
                vs.append([float(g.int(0, 3)) for _ in range(n)])
        pca_row(m, vs)


KERNELS = ["Gaussian", "Epanechnikov", "Uniform", "Triangular"]


def kde_row(m, data, pts, k, h):
    """`KernelDensity::new(data, h, k)` (h None: `silverman(data, k)`), then `evaluate_grid(pts)`. Class C: the
    Gaussian kernel's exp and Silverman's pow."""
    hb = [none()] if h is None else [f(h)]
    row(m, V.DESC, "KernelDensity::evaluate_grid", "C", slice_(data) + slice_(pts) + [e("Kernel", k)] + hb)


def points(data, h, extra):
    lo, hi = min(data), max(data)
    span = (hi - lo) + 10.0 * h if h is not None and h < 1e300 else (hi - lo) + 10.0
    pts = [lo - 5.0 * (h or 1.0) + span * j / 24.0 for j in range(25)]
    return [x for x in pts if x == x] + extra


def kde_rows(m, g):
    sp_pts = [0.0, -0.0, NAN, INF, -INF, MAXF, -MAXF, TINY]
    sets = [[0.0, 1.0, 2.0, 3.0, 4.0], [5.0], [0.0, 1.0], [1.0, 2.0, 3.0],
            [(j - 100.0) * 0.05 for j in range(200)], [j * 0.1 for j in range(100)],
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 10.0], [1e300, -1e300], [MAXF, -MAXF], [TINY, -TINY, 0.0],
            [1e16, 1e16 + 2.0, 1e16 + 4.0]]
    for _ in range(12):
        sets.append([g.uniform(-10.0, 10.0) for _ in range(g.int(2, 40))])
    for _ in range(6):
        sets.append([g.signed_log_uniform(-20.0, 20.0) for _ in range(g.int(2, 12))])
    hs = [1.0, 0.5, 0.05, 3.0, 1e-300, TINY, 1e300, MAXF]
    for data in sets:
        for k in KERNELS:
            for h in hs:
                kde_row(m, data, points(data, h, sp_pts), k, h)
            kde_row(m, data, points(data, 1.0, sp_pts), k, None)
    # the constructors' refusals
    for data in ([], [NAN], [INF], [1.0, -INF], [1.0, NAN, 2.0]):
        for k in KERNELS:
            kde_row(m, data, [0.0], k, 1.0)
            kde_row(m, data, [0.0], k, None)
    for h in (0.0, -0.0, -1.0, NAN, INF, -INF, -TINY):
        for k in KERNELS:
            kde_row(m, [1.0, 2.0], [0.0], k, h)
    for data in ([1.0], [2.0, 2.0, 2.0], [-0.0, 0.0]):
        kde_row(m, data, [0.0], "Gaussian", None)


def corr_row(m, vs):
    ins = []
    for x in vs:
        ins += slice_(x)
    row(m, V.DESC, "descriptive::correlation_matrix", "A", ins)


def corr_rows(m, g):
    x = [1.0, 2.0, 3.0, 4.0, 5.0]
    corr_row(m, [])
    corr_row(m, [x])
    corr_row(m, [[1.0], [2.0]])
    corr_row(m, [[], []])
    corr_row(m, [x, [1.0, 2.0]])
    corr_row(m, [[1.0], [2.0, 3.0]])
    corr_row(m, [x, [2.0 * v for v in x]])
    corr_row(m, [x, list(reversed(x))])
    corr_row(m, [[1.0, 2.0, 3.0, 4.0], [4.0, 3.0, 2.0, 5.0], [1.0, 5.0, 2.0, 4.0]])
    corr_row(m, [x, [3.0] * 5, x])
    corr_row(m, [[0.0, 0.0], [0.0, 0.0]])
    corr_row(m, [[1.0, NAN, 3.0], [1.0, 2.0, 3.0]])
    corr_row(m, [[1.0, INF, 3.0], [1.0, 2.0, 3.0]])
    corr_row(m, [[MAXF, -MAXF], [1.0, 2.0]])
    corr_row(m, [[1e300, 2e300, 3e300], [1e-300, 2e-300, 4e-300]])
    corr_row(m, [[TINY, 2 * TINY, 0.0], [1.0, 2.0, 3.0]])
    corr_row(m, [[-0.0, 0.0], [0.0, -0.0]])
    for _ in range(80):
        p = g.int(2, 6)
        n = g.int(2, 30)
        base = [g.uniform(-10.0, 10.0) for _ in range(n)]
        vs = []
        for j in range(p):
            r = g.int(0, 3)
            if r == 0:
                vs.append([g.uniform(-10.0, 10.0) for _ in range(n)])
            elif r == 1:
                a = g.uniform(-3.0, 3.0)
                vs.append([a * b + g.uniform(-0.1, 0.1) for b in base])
            elif r == 2:
                vs.append([g.signed_log_uniform(-100.0, 100.0) for _ in range(n)])
            else:
                vs.append([float(g.int(0, 3)) for _ in range(n)])
        corr_row(m, vs)
