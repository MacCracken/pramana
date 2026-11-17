"""`regression.tsv` (M5 part a): 1.3.0's regression.rs. The linear and polynomial fits (class A: arithmetic and the
correctly rounded √, hisab 1.4.0's modified Gram–Schmidt), their predictions and residuals, on exact lines and
polynomials, noisy seeded data, offset x (timestamps), constant and near-constant inputs, overflow, and every refusal;
the logistic fit (class C: the sigmoid's exp) on separable and non-separable data, several features, the l2 and
iteration grids, and its predictions. A slice is `v:n` and its values; the logistic features are one slice per row,
then the labels."""
from pathlib import Path

from .grids import INF, MAXF, NAN, TINY, Rng
from .models import V
from .tsv import Module, f, from_model, i, parsed, v

ROOT = Path(__file__).resolve().parents[3]
_PRIMS = None


def prims():
    global _PRIMS
    if _PRIMS is None:
        _PRIMS = V.dist_prims(ROOT)[0]
    return _PRIMS


def sl(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def row(m, fn, cls, ins):
    s, sc = V.take_slices(parsed(ins))
    m.row(fn, cls, ins, from_model(V.REG[fn](prims(), s, sc)))


def linear_sets(g):
    x5 = [1.0, 2.0, 3.0, 4.0, 5.0]
    sets = [(x5, [3.0, 5.0, 7.0, 9.0, 11.0]), (x5, [2.0, 4.1, 5.9, 8.2, 9.8]), ([1.0, 2.0], [3.0, 5.0]),
            ([1.0], [2.0]), ([], []), (x5, [1.0, 2.0]), ([2.0, 2.0, 2.0], [1.0, 2.0, 3.0]), (x5, [7.0] * 5),
            ([1.0, NAN, 3.0], [1.0, 2.0, 3.0]), ([1.0, 2.0, 3.0], [1.0, INF, 3.0]), ([MAXF, -MAXF, 0.0], [1.0, 2.0, 3.0]),
            ([1e300, 2e300, 3e300], [1.0, 2.0, 3.0]), ([TINY, 2 * TINY, 3 * TINY], [1.0, 2.0, 3.0]),
            ([1.6e9 + k for k in range(10)], [2.0 * k + 0.5 for k in range(10)]), ([-0.0, 0.0, 1.0], [0.0, 1.0, 2.0]),
            ([1.0, 1.0 + 2.2e-16, 1.0 + 4.4e-16], [1.0, 2.0, 3.0])]
    for _ in range(60):
        n = g.int(2, 50)
        a, b = g.uniform(-5.0, 5.0), g.uniform(-10.0, 10.0)
        xs = [g.uniform(-100.0, 100.0) for _ in range(n)]
        sets.append((xs, [a * x + b + g.uniform(-1.0, 1.0) for x in xs]))
    return sets


def poly_sets(g):
    xs = [float(k) for k in range(10)]
    sets = [([0.0, 1.0, 2.0, 3.0, 4.0], [1.0, 2.0, 5.0, 10.0, 17.0], 2), (xs, [x ** 3 - 2 * x + 1 for x in xs], 3),
            (xs, [x ** 3 - 2 * x + 1 for x in xs], 5), ([1.0, 2.0], [1.0, 2.0], 0), ([1.0, 2.0], [1.0, 2.0], 2),
            ([1.0, 2.0, 3.0], [1.0, 2.0], 1), ([1.0, NAN, 3.0], [1.0, 2.0, 3.0], 1), ([1.0, 2.0, 3.0], [1.0, INF, 3.0], 1),
            ([1.0, 1.0, 1.0, 1.0], [1.0, 2.0, 3.0, 4.0], 2), ([5.0] * 6, [1.0] * 6, 1), (xs, [3.0] * 10, 2),
            ([1e-7 * k for k in range(8)], [float(k) for k in range(8)], 4), ([1e6 + k for k in range(12)], xs + [10.0, 11.0], 3),
            (xs, xs, 9), ([-0.0, 0.0, 1.0, 2.0], [0.0, 1.0, 4.0, 9.0], 2)]
    for _ in range(50):
        n = g.int(4, 40)
        d = g.int(1, min(n - 1, 6))
        cs = [g.uniform(-3.0, 3.0) for _ in range(d + 1)]
        x = [g.uniform(-4.0, 4.0) for _ in range(n)]
        sets.append((x, [V.rg_eval(cs, t) + g.uniform(-0.1, 0.1) for t in x], d))
    return sets


def emit():
    m = Module("regression")
    g = Rng(0x452821E638D01377)
    for x, y in linear_sets(g):
        row(m, "regression::linear_regression", "A", sl(x) + sl(y))
    for slope, icpt in ((2.0, 1.0), (-0.5, 3.25), (0.0, 0.0), (1e300, 1e300), (NAN, 1.0)):
        for x in (0.0, -0.0, 1.0, 2.5, -7.0, 1e300, INF, NAN, TINY):
            row(m, "regression::predict", "A", [f(slope), f(icpt), f(0.9), f(x)])
        row(m, "regression::residuals", "A", [f(slope), f(icpt), f(0.9)] + sl([1.0, 2.0, 3.0]) + sl([3.0, 5.0, 7.5]))
    row(m, "regression::residuals", "A", [f(2.0), f(1.0), f(1.0)] + sl([1.0, 2.0]) + sl([3.0]))
    row(m, "regression::residuals", "A", [f(2.0), f(1.0), f(1.0)] + sl([]) + sl([]))
    for x, y, d in poly_sets(g):
        row(m, "regression::polynomial_regression", "A", sl(x) + sl(y) + [i(d)])
    # a column norm between hisab's 1e-12 dependence threshold and 1e-6: x = 1e-4·k at degree 2 (about 3.5e-8)
    for s in (1e-4, 3e-5, 1e-3):
        xs = [s * k for k in range(8)]
        row(m, "regression::polynomial_regression", "A", sl(xs) + sl([float(k * k) for k in range(8)]) + [i(2)])
    for cs in ([1.0, 2.0, 3.0], [], [5.0], [1e300, 1e300], [-0.0], [1.0, -1.0, 1.0, -1.0, 1.0]):
        for x in (0.0, -0.0, 1.0, -2.0, 0.5, 1e100, NAN, INF):
            row(m, "regression::predict_poly", "A", sl(cs) + [f(x)])
        row(m, "regression::residuals_poly", "A", sl(cs) + sl([1.0, 2.0, 3.0]) + sl([6.0, 17.0, 34.0]))
    row(m, "regression::residuals_poly", "A", sl([1.0]) + sl([1.0, 2.0]) + sl([1.0]))
    # logistic: 1.3.0's bench body, separable and overlapping data, several features, the refusals
    one = [[(k - 21) * 0.25] for k in range(43)]
    lab = [1.0 if (k * 7) % 43 > 21 else 0.0 for k in range(43)]
    cases = [(one, lab, 1.0, 50), ([[float(x)] for x in range(-5, 6)], [0.0] * 5 + [1.0] * 6, 1.0, 50),
             ([[float(x)] for x in range(-5, 6)], [0.0] * 5 + [1.0] * 6, 0.0, 100),
             ([[1.0], [2.0]], [0.0, 1.0], 0.0, 25), ([[1.0]], [1.0], 1.0, 10), ([[1.0], [2.0]], [0.0], 1.0, 10),
             ([[1.0], [2.0]], [0.0, 1.0], 1.0, 0), ([[1.0], [2.0]], [0.0, 1.0], -1.0, 10),
             ([[1.0], [2.0]], [0.0, 1.0], NAN, 10), ([[1.0], [2.0, 3.0]], [0.0, 1.0], 1.0, 10),
             ([[1.0], [NAN]], [0.0, 1.0], 1.0, 10), ([[1.0], [2.0]], [0.0, 0.5], 1.0, 10), ([[], []], [0.0, 1.0], 1.0, 10),
             ([[1.0, 2.0], [2.0, 1.0], [3.0, 3.0], [0.0, 0.5]], [0.0, 1.0, 1.0, 0.0], 0.1, 30),
             ([[1e3], [-1e3], [2e3], [-2e3]], [1.0, 0.0, 1.0, 0.0], 0.0, 50), ([[0.0]] * 6, [0.0, 1.0] * 3, 0.0, 20)]
    for _ in range(25):
        n = g.int(4, 60)
        p = g.int(1, 4)
        w = [g.uniform(-2.0, 2.0) for _ in range(p)]
        rows = [[g.uniform(-3.0, 3.0) for _ in range(p)] for _ in range(n)]
        labels = [1.0 if sum(a * b for a, b in zip(w, r)) + g.uniform(-1.0, 1.0) > 0.0 else 0.0 for r in rows]
        cases.append((rows, labels, [0.0, 0.1, 1.0, 10.0][g.int(0, 3)], [1, 3, 50][g.int(0, 2)]))
    g2 = Rng(0x3F84D5B5B5470917)          # more fits: the Newton steps' last |Δ| across the 1e-8 stop
    for _ in range(60):
        n = g2.int(6, 40)
        w = g2.uniform(-1.5, 1.5)
        rows = [[g2.uniform(-2.0, 2.0)] for _ in range(n)]
        labels = [1.0 if w * r[0] + g2.uniform(-1.5, 1.5) > 0.0 else 0.0 for r in rows]
        if all(lb == labels[0] for lb in labels):
            labels[0] = 1.0 - labels[0]
        cases.append((rows, labels, [0.0, 0.01, 0.5][g2.int(0, 2)], 100))
    for rows, labels, l2, it in cases:
        ins = []
        for r in rows:
            ins += sl(r)
        row(m, "regression::logistic_regression", "C", ins + sl(labels) + [f(l2), i(it)])
    for cs in ([0.5, 1.2], [0.0], [], [1.0, -1.0, 2.0], [1e300, 1.0], [-800.0], [800.0]):
        for feats in ([], [1.0], [2.0, 3.0], [-1.0, 0.5, 9.0], [NAN], [1e308, 1e308]):
            row(m, "regression::predict_logistic_proba", "C", sl(cs) + sl(feats))
            for th in (0.5, 0.0, 1.0, NAN):
                row(m, "regression::predict_logistic_class", "C", sl(cs) + sl(feats) + [f(th)])
    return m
