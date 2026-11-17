"""`dist.tsv`'s MultivariateNormal rows (M3 part d2): the constructor (L and log det Σ, or the error), log_pdf and pdf,
and the draws. Inputs: d, the mean as `v:d`, the covariance's rows and columns as `i:` tokens and its entries as
`v:rows·cols` row-major; then the point as `v:n`, or the seed and the count. Class C."""
import math

from .grids import INF, MAXF, MIN_POSITIVE, NAN
from .models import V
from .tsv import bits, f, from_bits, i, v
from .m_dist import SEEDS, row, prims


class Case:
    def __init__(self, mean, rows, cols, cov):
        self.mean, self.rows, self.cols, self.cov = mean, rows, cols, cov

    @staticmethod
    def square(mean, cov):
        return Case(mean, len(mean), len(mean), cov)

    @staticmethod
    def of(mean, m):
        return Case.square(list(mean), [x for r in m for x in r])

    def toks(self):
        return ([i(len(self.mean)), v(len(self.mean))] + [f(x) for x in self.mean]
                + [i(self.rows), i(self.cols), v(len(self.cov))] + [f(x) for x in self.cov])


def valid(c):
    out = V.DIST["MultivariateNormal::new"](prims(), *[t[1] if t[0] in "fi" else "v:%d" % t[1] for t in c.toks()])
    return not (isinstance(out[0], str) and out[0].startswith("E:"))


def points(g, c):
    d = len(c.mean)
    mu = c.mean
    scale = [math.sqrt(c.cov[k * d + k]) for k in range(d)]
    ps = [list(mu)]
    seeded = 12 if d <= 4 else (4 if d <= 8 else 1)
    for _ in range(seeded):
        ps.append([mu[k] + scale[k] * g.uniform(-4.0, 4.0) for k in range(d)])
    if d <= 8:
        for k in range(min(d, 3)):
            for t in (-3.0, 1.0, 40.0):
                x = list(mu)
                x[k] += t * scale[k]
                ps.append(x)
    far = list(mu)
    far[0] += 1e200
    ps.append(far)
    if d > 8:
        return ps
    for special in (NAN, INF, -INF):
        x = list(mu)
        x[d // 2] = special
        ps.append(x)
    ps.append(list(mu[:d - 1]))
    ps.append(list(mu) + [0.0])
    return ps


def spd(g, d, shift):
    b = [g.uniform(-1.0, 1.0) for _ in range(d * d)]
    a = [0.0] * (d * d)
    for r in range(d):
        for c in range(d):
            s = 0.0
            for k in range(d):
                s += b[r * d + k] * b[c * d + k]
            a[r * d + c] = s + (shift if r == c else 0.0)
    return a


def identity(d, s):
    return [s if k // d == k % d else 0.0 for k in range(d * d)]


def pair(s, a, b):
    return Case.square([0.0, 0.0], [s, a, b, s])


def emit(m, g):
    cases = [Case.square([], [])]
    for x in [4.0, 1.0, 2.0, 0.25, 0.0, -0.0, -1.0, NAN, INF, -INF, from_bits(1), 1e-300, 1e300, MAXF]:
        cases.append(Case.square([0.0], [x]))
    for mu in [-0.0, 1.5, -3.0, 1e300, MAXF, -MAXF, from_bits(1), NAN, INF, -INF]:
        cases.append(Case.square([mu], [4.0]))
    for rows, cols in [(0, 0), (1, 1), (1, 2), (2, 1), (3, 3), (2, 3), (3, 2), (4, 1)]:
        cases.append(Case([0.0, 0.0], rows, cols, [float(k % 3) for k in range(rows * cols)]))
    cases.append(Case([0.0], 1, 2, [1.0, 0.0]))
    cases.append(Case([0.0], 0, 0, []))
    cases.append(Case([NAN, 0.0], 1, 1, [1.0]))
    cases.append(Case([0.0, 0.0], 3, 3, [NAN] * 9))
    for k in range(4):
        for x in (NAN, INF, -INF):
            cov = [4.0, 1.0, 1.0, 9.0]
            cov[k] = x
            cases.append(Case.square([1.0, 2.0], cov))
    lim = 0.5 * (1.0 + 1e-10)
    for k in range(-3, 4):
        b0 = from_bits(bits(lim) + k)
        cases += [pair(1.0, 0.5, b0), pair(1.0, b0, 0.5)]
    floor = 1e-10 * MIN_POSITIVE
    for k in range(-2, 3):
        b0 = from_bits(bits(floor) + k)
        cases += [pair(1.0, 0.0, b0), pair(1.0, -b0, 0.0)]
    cases += [pair(1.0, 0.0, -0.0), pair(1.0, 0.0, from_bits(1)), pair(1.0, 0.5, 0.0), pair(1e-6, 1e-7, 2e-7),
              pair(1e12, 0.5e12, 0.5e12 + 1e-3), pair(4.0, 1.0, 1.0 + 1e-12), pair(1.0, MAXF, -MAXF),
              pair(4.0, 1.7117351981710087, 1.7117351979998352), pair(4.0, 1.7117351979998352, 1.7117351981710087)]
    for cov in ([1.0, 1.0, 1.0, 1.0], [1.0, 2.0, 2.0, 1.0], [-4.0, 0.0, 0.0, 3.0], [0.0, 0.0, 0.0, 1.0]):
        cases.append(Case.square([0.0, 0.0], cov))
    tiny = from_bits(1)
    cases.append(Case.square([0.0] * 3, [tiny, 0.0, 1e200, 0.0, 1.0, 0.0, 1e200, 0.0, 1.0]))
    cases += [Case.of([2.0], [[4.0]]), Case.of([0.0, 0.0], [[1.0, 0.0], [0.0, 1.0]]),
              Case.of([1.0, -2.0], [[1.0, 0.3], [0.3, 2.0]]), Case.of([1.0, -1.0], [[2.0, 0.5], [0.5, 3.0]]),
              Case.of([1.0, 2.0], [[4.0, 1.0], [1.0, 9.0]]), Case.of([1.0, 2.0], [[2.0, 0.5], [0.5, 1.0]]),
              Case.of([0.0, 0.0, 0.0], [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])]
    for d in [1, 2, 3, 4, 5, 6, 8, 12, 16, 24, 32]:
        mean = [g.signed_log_uniform(-3.0, 3.0) for _ in range(d)]
        cases.append(Case.square(list(mean), spd(g, d, float(d))))
        cases.append(Case.square(mean, spd(g, d, 1e-3)))
    for d, s in [(3, 1e300), (3, 1e-300), (3, tiny), (3, MAXF), (5, 1e-300), (2, MIN_POSITIVE)]:
        mean = [g.signed_log_uniform(-3.0, 3.0) for _ in range(d)]
        cases.append(Case.square(mean, identity(d, s)))
    scaled = spd(g, 4, 4.0)
    for s in (1e300, 1e-300):
        cases.append(Case.square([0.0, -0.0, 1.0, -1.0], [x * s / 8.0 for x in scaled]))
    for d in range(3, 15):
        cases.append(Case.square([0.0] * d, [1.0 / float(k // d + k % d + 1) for k in range(d * d)]))
    for d in (3, 7, 16):
        lehmer = [float(min(k // d, k % d) + 1) / float(max(k // d, k % d) + 1) for k in range(d * d)]
        cases.append(Case.square([float(k) - 1.0 for k in range(d)], lehmer))
    ok = []
    for c in cases:
        row(m, "MultivariateNormal::new", "C", c.toks())
        if valid(c):
            ok.append(c)
    for c in ok:
        d = len(c.mean)
        for j, x in enumerate(points(g, c)):
            ins = c.toks() + [v(len(x))] + [f(y) for y in x]
            row(m, "MultivariateNormal::log_pdf", "C", ins)
            if d <= 8 or j == 0:
                row(m, "MultivariateNormal::pdf", "C", ins)
    draw_cases = [Case.of([1.0, -2.0], [[1.0, 0.3], [0.3, 2.0]]), Case.of([2.0], [[4.0]]),
                  Case.of([1.0, 2.0], [[4.0, 1.0], [1.0, 9.0]]), Case.square([0.5, -0.0, 3.0], spd(g, 3, 3.0)),
                  Case.square([-1.0, 0.0, 1.0, 2.0, -0.0], spd(g, 5, 1e-3)),
                  Case.square([float(k) * 0.5 for k in range(8)], spd(g, 8, 8.0)),
                  Case.square([0.0] * 3, identity(3, 1e300)), Case.square([MAXF, -0.0], identity(2, 1e-300))]
    for seed in SEEDS:
        for k, c in enumerate(draw_cases):
            count = 10000 if k == 0 else 256
            row(m, "MultivariateNormal::sample", "C", c.toks() + [i(seed), i(count)])
