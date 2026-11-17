"""`dist.tsv`: 1.3.0's distributions (M3), named by Rust's path (`Normal::pdf`), and the oracle's private samplers and
tails: each constructor on its parameter grid, the pdf and cdf across and beyond the support at fixed, seeded and
random parameters, the mean and variance, 10^4-draw streams from three seeds (and 256 of each other case), and
MultivariateNormal's rows (m_mvn). On validate.py's DIST models through glibc, which are 1.3.0's bits; validate.py
writes dist.cr.tsv and dist.ref.tsv, which hold the port to the correctly rounded primitives and T1."""
import math
from pathlib import Path

from .grids import INF, MAXF, MIN_POSITIVE, NAN, Rng, pow2, specials
from .models import V
from .tsv import Module, bits, f, from_bits, from_model, i, parsed

SEEDS = [42, 0x9E3779B97F4A7C15, 12345]
U64 = (1 << 64) - 1
EPS = 2.220446049250313e-16
L = None


def prims():
    global L
    if L is None:
        L = V.dist_prims(Path(__file__).resolve().parents[3])[0]
    return L


def row(m, fn, cls, ins):
    m.row(fn, cls, ins, from_model(V.DIST[fn](prims(), *parsed(ins))))


def fl(xs):
    return [f(x) for x in xs]


def param_grid():
    return specials() + [0.25, 1e-300, -1e-300, 1e300, -1e300, -3.5, 1e308, -1e308, 0.5, 2.0]


def around(x):
    up = from_bits(1) if x == 0.0 else from_bits(bits(x) + 1)
    down = -from_bits(1) if x == 0.0 else from_bits(bits(x) - 1)
    return [up, x, down] if x < 0.0 else [down, x, up]


def floor(x):
    return V.r_floor(x)


def streams(m, fn, cls, cases, long=lambda p: False):
    for seed in SEEDS:
        for k, ins in enumerate(cases):
            count = 10000 if k == 0 or long(ins) else 256
            row(m, fn, cls, ins + [i(seed), i(count)])


def new_rows(m, name, grid2=None, grid1=None):
    if grid2 is not None:
        for a in grid2:
            for b0 in grid2:
                row(m, f"{name}::new", "A", [f(a), f(b0)])
    else:
        for a in grid1:
            row(m, f"{name}::new", "A", [f(a)])


def normal(m, g):
    grid = specials() + [0.25, 1e-300, -1e-300, 1e300, -1e300, -3.5]
    new_rows(m, "Normal", grid2=grid)
    fixed = [(0.0, 1.0), (1.5, 2.3), (-3.5, 0.25), (1e6, 1e-3), (-0.0, 1.0), (0.0, 1e-300), (0.0, 1e300),
             (-1e-300, 7.0), (MAXF, 1.0)]
    seeded = []
    for _ in range(40):
        mu = g.signed_log_uniform(-6.0, 6.0)
        seeded.append((mu, g.log_uniform(-4.0, 4.0)))

    def point(mu, sd, x):
        row(m, "Normal::pdf", "C", fl([mu, sd, x]))
        row(m, "Normal::cdf", "C", fl([mu, sd, x]))

    for k in range(-640, 641):
        point(0.0, 1.0, float(k) / 16.0)
    for mu, sd in fixed[1:]:
        for k in range(-160, 161):
            point(mu, sd, mu + sd * (float(k) / 4.0))
        for x in specials():
            point(mu, sd, x)
    for mu, sd in seeded:
        for k in range(-16, 17):
            point(mu, sd, mu + sd * (2.5 * float(k)))
    for _ in range(1000):
        mu = g.signed_log_uniform(-3.0, 3.0)
        sd = g.log_uniform(-3.0, 3.0)
        point(mu, sd, mu + sd * g.uniform(-12.0, 12.0))
    for mu, sd in fixed + seeded:
        row(m, "Normal::mean", "A", fl([mu, sd]))
        row(m, "Normal::variance", "A", fl([mu, sd]))
    streams(m, "Normal::sample", "C", [fl(p) for p in fixed])
    for seed in (1, 42, U64):
        row(m, "standard_normal", "C", [i(seed), i(1024)])


def uniform(m, g):
    new_rows(m, "Uniform", grid2=param_grid())
    fixed = [(0.0, 1.0), (-1.0, 5.0), (0.0, 10.0), (-3.5, 0.25), (-0.0, 1.0), (1e-300, 2e-300), (-1e300, 1e300),
             (0.0, MAXF), (1e6, 1e6 + 1.0), (1e308, MAXF), (from_bits(1), from_bits(2))]
    seeded = []
    for _ in range(40):
        lo = g.signed_log_uniform(-6.0, 6.0)
        seeded.append((lo, lo + g.log_uniform(-4.0, 4.0)))

    def point(lo, hi, x):
        row(m, "Uniform::pdf", "A", fl([lo, hi, x]))
        row(m, "Uniform::cdf", "A", fl([lo, hi, x]))

    for lo, hi in fixed + seeded:
        w = hi - lo
        for k in range(-8, 25):
            point(lo, hi, lo + w * (float(k) / 16.0))
        for x in around(lo) + around(hi):
            point(lo, hi, x)
    for lo, hi in fixed:
        for x in specials():
            point(lo, hi, x)
    for _ in range(1000):
        lo = g.signed_log_uniform(-3.0, 3.0)
        hi = lo + g.log_uniform(-3.0, 3.0)
        point(lo, hi, lo + (hi - lo) * g.uniform(-0.5, 1.5))
    for lo, hi in fixed + seeded:
        row(m, "Uniform::mean", "A", fl([lo, hi]))
        row(m, "Uniform::variance", "A", fl([lo, hi]))
    streams(m, "Uniform::sample", "A", [fl(p) for p in fixed])


def exponential(m, g):
    new_rows(m, "Exponential", grid1=param_grid())
    fixed = [1.0, 2.0, 0.5, 2.5, 1e-3, 1e3, 1e-300, 1e300, MIN_POSITIVE, MAXF]
    seeded = [g.log_uniform(-6.0, 6.0) for _ in range(40)]

    def point(la, x):
        row(m, "Exponential::pdf", "C", fl([la, x]))
        row(m, "Exponential::cdf", "C", fl([la, x]))

    for la in fixed + seeded:
        for k in range(-8, 321):
            point(la, (float(k) / 8.0) / la)
        for x in [1e-20, 1e-300, from_bits(1), 1e-10 / la, 1e-16 / la]:
            point(la, x)
    for la in fixed:
        for x in specials():
            point(la, x)
    for _ in range(1000):
        la = g.log_uniform(-3.0, 3.0)
        point(la, g.log_uniform(-12.0, 2.0) / la)
    for la in fixed + seeded:
        row(m, "Exponential::mean", "A", fl([la]))
        row(m, "Exponential::variance", "A", fl([la]))
    streams(m, "Exponential::sample", "C", [fl([la]) for la in fixed])


def bernoulli(m, g):
    new_rows(m, "Bernoulli", grid1=param_grid() + [1.0 - EPS, EPS, 0.7, 1.0 + EPS, -EPS])
    fixed = [0.5, 0.0, 1.0, 0.3, 0.7, 1e-300, 1.0 - EPS / 2.0, from_bits(1), 0.1]
    seeded = [g.unit() for _ in range(20)]
    xs = specials()
    for k in range(5):
        d = float(k) * EPS / 2.0
        xs += [d, -d, 1.0 + d, 1.0 - d]
    xs += [0.5, 2.0, -1.0, 0.999999, 1e-300, 3.0, 1.5]
    for p in fixed + seeded:
        for x in xs:
            row(m, "Bernoulli::pdf", "A", fl([p, x]))
            row(m, "Bernoulli::cdf", "A", fl([p, x]))
        row(m, "Bernoulli::mean", "A", fl([p]))
        row(m, "Bernoulli::variance", "A", fl([p]))
    streams(m, "Bernoulli::sample", "A", [fl([p]) for p in fixed])


def cauchy(m, g):
    new_rows(m, "Cauchy", grid2=param_grid())
    fixed = [(0.0, 1.0), (2.0, 3.0), (-3.5, 0.25), (1e6, 1e-3), (-0.0, 1.0), (0.0, 1e-300), (0.0, 1e300), (MAXF, 1.0),
             (-MAXF, MAXF), (1e-300, 7.0), (0.0, from_bits(1))]
    seeded = []
    for _ in range(40):
        x0 = g.signed_log_uniform(-6.0, 6.0)
        seeded.append((x0, g.log_uniform(-4.0, 4.0)))

    def point(x0, ga, x):
        row(m, "Cauchy::pdf", "A", fl([x0, ga, x]))
        row(m, "Cauchy::cdf", "C", fl([x0, ga, x]))

    far = [1e10, 1e20, 1e154, 1.4e154, 1e300]
    for x0, ga in fixed:
        for k in range(-160, 161):
            point(x0, ga, x0 + ga * (float(k) / 4.0))
        for z in far:
            point(x0, ga, x0 + ga * z)
            point(x0, ga, x0 - ga * z)
        for x in specials():
            point(x0, ga, x)
    for x0, ga in seeded:
        for k in range(-16, 17):
            point(x0, ga, x0 + ga * (float(k) * 2.5))
    for _ in range(1000):
        x0 = g.signed_log_uniform(-3.0, 3.0)
        ga = g.log_uniform(-3.0, 3.0)
        point(x0, ga, x0 + ga * g.signed_log_uniform(-3.0, 6.0))
    for x0, ga in fixed + seeded:
        row(m, "Cauchy::mean", "A", fl([x0, ga]))
        row(m, "Cauchy::variance", "A", fl([x0, ga]))
    streams(m, "Cauchy::sample", "C", [fl(p) for p in fixed])


def weibull(m, g):
    new_rows(m, "Weibull", grid2=param_grid())
    fixed = [(1.0, 1.0), (2.0, 1.0), (0.5, 1.0), (1.0, 2.0), (2.0, 3.0), (1.5, 0.25), (5.0, 1e3), (0.1, 1.0),
             (10.0, 1.0), (100.0, 1.0), (2.0, 1e-300), (3.0, 1e300), (1e-3, 1.0), (MAXF, 1.0), (from_bits(1), 1.0)]
    seeded = []
    for _ in range(40):
        k = g.log_uniform(-1.5, 1.5)
        seeded.append((k, g.log_uniform(-3.0, 3.0)))

    def point(k, la, x):
        row(m, "Weibull::pdf", "C", fl([k, la, x]))
        row(m, "Weibull::cdf", "C", fl([k, la, x]))

    for k, la in fixed + seeded:
        for j in range(-8, 81):
            point(k, la, la * (float(j) / 16.0))
        for j in range(-40, 13):
            point(k, la, la * pow2(j))
        for x in around(la):
            point(k, la, x)
        point(k, la, la * 1e-300)
    for k, la in fixed:
        for x in specials():
            point(k, la, x)
    for _ in range(1000):
        k = g.log_uniform(-1.0, 1.3)
        la = g.log_uniform(-3.0, 3.0)
        point(k, la, la * g.log_uniform(-6.0, 1.5))
    shapes = [1e-3, 0.01, 0.1, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 3.6, 10.0, 100.0, 1e3, 1e6, 1e10, 1e16, 1e300, 0.003,
              MIN_POSITIVE, from_bits(1), MAXF] + [g.log_uniform(-3.0, 6.0) for _ in range(200)]
    moments = fixed + seeded
    for k in shapes:
        moments += [(k, la) for la in (1.0, 2.5, 1e-300, 1e300)]
    for k, la in moments:
        row(m, "Weibull::mean", "C", fl([k, la]))
        row(m, "Weibull::variance", "C", fl([k, la]))
    streams(m, "Weibull::sample", "C", [fl(p) for p in fixed])


def gamma_points(a, b0):
    mean = a * b0
    sd = math.sqrt(a) * b0
    xs = [mean * (float(j) / 16.0) for j in range(-4, 65)]
    xs += [b0 * pow2(j) for j in range(-40, 13)]
    xs += [mean + sd * (float(j) / 2.0) for j in range(-24, 25)]
    xs += around(mean)
    xs += [b0 * 1e-300, mean * 1e3, mean * 1e10]
    return xs


def gamma(m, g):
    new_rows(m, "Gamma", grid2=param_grid())
    fixed = [(1.0, 1.0), (2.0, 1.0), (0.5, 1.0), (2.0, 3.0), (5.0, 0.5), (0.1, 1.0), (10.0, 2.0), (100.0, 1.0),
             (1e3, 1.0), (1e-3, 1.0), (1e5, 1e-3), (1.0, 1e-300), (3.0, 1e300), (2e9, 1.0), (from_bits(1), 1.0),
             (MAXF, 1.0)]
    seeded = []
    for _ in range(40):
        a = g.log_uniform(-2.0, 3.0)
        seeded.append((a, g.log_uniform(-3.0, 3.0)))

    def point(a, b0, x):
        row(m, "Gamma::pdf", "C", fl([a, b0, x]))
        row(m, "Gamma::cdf", "C", fl([a, b0, x]))

    for a, b0 in fixed:
        for x in gamma_points(a, b0):
            point(a, b0, x)
        for x in specials():
            point(a, b0, x)
    for a, b0 in seeded:
        for j in range(-8, 9):
            point(a, b0, a * b0 + math.sqrt(a) * b0 * float(j))
    for _ in range(1000):
        a = g.log_uniform(-2.0, 3.0)
        b0 = g.log_uniform(-3.0, 3.0)
        point(a, b0, b0 * a * g.log_uniform(-3.0, 0.7))
    for a, b0 in fixed + seeded:
        row(m, "Gamma::mean", "A", fl([a, b0]))
        row(m, "Gamma::variance", "A", fl([a, b0]))
    streams(m, "Gamma::sample", "C", [fl(p) for p in fixed], lambda p: p == [f(0.5), f(1.0)])


def chi_squared(m, g):
    new_rows(m, "ChiSquared", grid1=param_grid())
    fixed = [2.0, 1.0, 3.0, 4.0, 5.0, 10.0, 30.0, 100.0, 1e3, 1e5, 0.5, 0.1, 1e-3, 4e9, 1e-300, from_bits(1)]
    seeded = [g.log_uniform(-2.0, 4.0) for _ in range(40)]

    def point(df, x):
        row(m, "ChiSquared::pdf", "C", fl([df, x]))
        row(m, "ChiSquared::cdf", "C", fl([df, x]))

    for df in fixed:
        for x in gamma_points(df / 2.0, 2.0):
            point(df, x)
        for x in specials():
            point(df, x)
    for df in seeded:
        for j in range(-8, 9):
            point(df, df + math.sqrt(2.0 * df) * float(j))
    for _ in range(1000):
        df = g.log_uniform(-2.0, 4.0)
        point(df, df * g.log_uniform(-3.0, 0.7))
    for df in fixed + seeded:
        row(m, "ChiSquared::mean", "A", fl([df]))
        row(m, "ChiSquared::variance", "A", fl([df]))
    streams(m, "ChiSquared::sample", "C", [fl([df]) for df in fixed])


def gamma_standard(m):
    for seed in (1, 42, U64):
        for a in [1e-3, 0.1, 0.5, 0.999999, 1.0, 1.5, 2.5, 10.0, 1e3, 1e9, 1e300]:
            row(m, "sample_gamma_standard", "C", [f(a), i(seed), i(1024)])


def beta(m, g):
    new_rows(m, "Beta", grid2=param_grid())
    fixed = [(2.0, 5.0), (1.0, 1.0), (0.5, 0.5), (2.0, 2.0), (5.0, 1.0), (0.1, 0.1), (30.0, 0.7), (400.0, 350.0),
             (1e3, 1e3), (1e-3, 1.0), (1.0, 1e5), (1e5, 1e5), (2e9, 3.0), (from_bits(1), 1.0), (MAXF, 1.0)]
    seeded = []
    for _ in range(40):
        a = g.log_uniform(-2.0, 3.0)
        seeded.append((a, g.log_uniform(-2.0, 3.0)))

    def point(a, b0, x):
        row(m, "Beta::pdf", "C", fl([a, b0, x]))
        row(m, "Beta::cdf", "C", fl([a, b0, x]))

    for a, b0 in fixed:
        mean = a / (a + b0)
        sd = math.sqrt(a * b0 / ((a + b0) * (a + b0) * (a + b0 + 1.0)))
        xs = [float(j) / 64.0 for j in range(-4, 69)]
        for j in range(1, 61):
            xs += [pow2(-j), 1.0 - pow2(-j)]
        xs += [mean + sd * (float(j) / 2.0) for j in range(-16, 17)]
        xs += around(0.0) + around(1.0) + specials()
        for x in xs:
            point(a, b0, x)
    for a, b0 in seeded:
        for j in range(17):
            point(a, b0, float(j) / 16.0)
    for _ in range(1000):
        a = g.log_uniform(-2.0, 3.0)
        b0 = g.log_uniform(-2.0, 3.0)
        point(a, b0, g.unit())
    for a, b0 in fixed + seeded:
        row(m, "Beta::mean", "A", fl([a, b0]))
        row(m, "Beta::variance", "A", fl([a, b0]))
    streams(m, "Beta::sample", "C", [fl(p) for p in fixed])


def t_points():
    ts = [float(k) / 4.0 for k in range(-80, 81)]
    for z in [1e3, 1e10, 1e20, 1e155, 1e300]:
        ts += [z, -z]
    return ts + specials()


def student_t(m, g):
    new_rows(m, "StudentT", grid1=param_grid())
    fixed = [3.0, 1.0, 2.0, 3.5, 5.0, 10.0, 19.999999999999996, 20.0, 30.0, 100.0, 1e3, 1e6, 0.5, 0.1, 1e-3, 2e9, 3e9,
             from_bits(1)]
    seeded = [g.log_uniform(-1.0, 4.0) for _ in range(40)]

    def point(df, x):
        row(m, "StudentT::pdf", "C", fl([df, x]))
        row(m, "StudentT::cdf", "C", fl([df, x]))

    for df in fixed:
        for x in t_points():
            point(df, x)
    for df in seeded:
        for k in range(-16, 17):
            point(df, float(k) / 2.0)
    for _ in range(1000):
        df = g.log_uniform(-1.0, 4.0)
        point(df, g.signed_log_uniform(-3.0, 3.0))
    for df in fixed + seeded:
        row(m, "StudentT::mean", "A", fl([df]))
        row(m, "StudentT::variance", "A", fl([df]))
    streams(m, "StudentT::sample", "C", [fl([df]) for df in fixed])


def f_distribution(m, g):
    new_rows(m, "FDistribution", grid2=param_grid())
    fixed = [(4.0, 9.0), (1.0, 1.0), (2.0, 2.0), (5.0, 2.0), (1.0, 30.0), (30.0, 1.0), (10.0, 20.0), (100.0, 100.0),
             (1e3, 1e4), (0.5, 0.5), (0.1, 3.0), (1e5, 1e5), (2e9, 5.0), (3.0, 4.5), (3.0, 4.0), (from_bits(1), 1.0)]
    seeded = []
    for _ in range(40):
        d1 = g.log_uniform(-1.0, 3.0)
        seeded.append((d1, g.log_uniform(-1.0, 3.0)))

    def point(d1, d2, x):
        row(m, "FDistribution::pdf", "C", fl([d1, d2, x]))
        row(m, "FDistribution::cdf", "C", fl([d1, d2, x]))

    for d1, d2 in fixed:
        xs = [float(j) / 16.0 for j in range(-4, 97)] + [pow2(j) for j in range(-40, 21)] + [1e10, 1e300] + specials()
        for x in xs:
            point(d1, d2, x)
    for d1, d2 in seeded:
        for j in range(17):
            point(d1, d2, float(j) / 4.0)
    for _ in range(1000):
        d1 = g.log_uniform(-1.0, 3.0)
        d2 = g.log_uniform(-1.0, 3.0)
        point(d1, d2, g.log_uniform(-3.0, 2.0))
    for d1, d2 in fixed + seeded:
        row(m, "FDistribution::mean", "A", fl([d1, d2]))
        row(m, "FDistribution::variance", "A", fl([d1, d2]))
    streams(m, "FDistribution::sample", "C", [fl(p) for p in fixed])


def tails(m, g):
    for df in [0.5, 1.0, 3.5, 30.0, 1e6, 2e9]:
        for t in t_points():
            row(m, "student_t_two_tailed", "C", fl([t, df]))
            row(m, "student_t_cdf", "C", fl([t, df]))
    for df in specials():
        for t in [0.0, 1.5, -2.0, NAN]:
            row(m, "student_t_two_tailed", "C", fl([t, df]))
    for _ in range(500):
        df = g.log_uniform(-1.0, 5.0)
        row(m, "student_t_two_tailed", "C", fl([g.signed_log_uniform(-3.0, 3.0), df]))
    for d1, d2 in [(4.0, 9.0), (1.0, 1.0), (0.5, 30.0), (100.0, 3.0), (1e4, 1e4)]:
        xs = [float(j) / 16.0 for j in range(-4, 97)] + [pow2(j) for j in range(-40, 21)] + specials()
        for x in xs:
            row(m, "f_cdf_sf", "C", fl([x, d1, d2]))
    for _ in range(500):
        d1 = g.log_uniform(-1.0, 4.0)
        d2 = g.log_uniform(-1.0, 4.0)
        row(m, "f_cdf_sf", "C", fl([g.log_uniform(-4.0, 3.0), d1, d2]))


def poisson(m, g):
    new_rows(m, "Poisson", grid1=param_grid())
    fixed = [2.5, 1.0, 0.5, 3.5, 10.0, 29.999999999999996, 30.0, 31.0, 100.0, 1e3, 1e6, 1e12, 1e-3, 1e-300,
             from_bits(1), MAXF]
    seeded = [g.log_uniform(-2.0, 4.0) for _ in range(40)]

    def point(la, x):
        row(m, "Poisson::pdf", "C", fl([la, x]))
        row(m, "Poisson::cdf", "C", fl([la, x]))

    for la in fixed:
        sd = math.sqrt(la)
        xs = [float(k) for k in range(61)] + [floor(la + sd * (float(j) / 2.0)) for j in range(-24, 25)]
        xs += [0.5, 2.5, 3.0000000000000004, 2.9999999999999996, -0.5, -1.0, -0.0]
        xs += [la, floor(la), la + 0.5, 1e15, 9007199254740992.0, 1e300] + specials()
        for x in xs:
            point(la, x)
    for la in seeded:
        sd = math.sqrt(la)
        for j in range(-12, 13):
            point(la, floor(la + sd * (float(j) / 2.0)))
    for k in range(1000):
        la = g.log_uniform(-2.0, 5.0)
        x = floor(la * g.log_uniform(-2.0, 0.5))
        point(la, x + 0.25 if k % 10 == 0 else x)
    for la in fixed + seeded:
        row(m, "Poisson::mean", "A", fl([la]))
        row(m, "Poisson::variance", "A", fl([la]))
    streams(m, "Poisson::sample", "C", [fl([la]) for la in fixed], lambda p: p == [f(100.0)])


def poisson_ptrs(m):
    for seed in (1, 42, U64):
        for la in [10.0, 15.0, 30.0, 31.0, 100.0, 1e4, 1e8, 1e12, 1e15]:
            row(m, "sample_poisson_ptrs", "C", [f(la), i(seed), i(1024)])


def binomial(m, g):
    ns = [0, 1, 2, 10, 64, 65, 1000, 1 << 53, (1 << 53) + 1, 1 << 63, U64]
    ps = param_grid() + [1.0 - EPS / 2.0, 1.0 + EPS, 0.3]
    for n in ns:
        for p in ps:
            row(m, "Binomial::new", "A", [i(n), f(p)])
    fixed = [(10, 0.3), (1, 0.5), (2, 0.5), (20, 0.4), (64, 0.5), (65, 0.5), (100, 0.01), (1000, 0.5), (1000000, 0.3),
             (1000000000000, 0.5), ((1 << 53) + 1, 0.5), (U64, 0.5), (10, 0.0), (10, 1.0), (50, 1e-300),
             (50, 1.0 - EPS / 2.0), (30, from_bits(1))]
    seeded = []
    for _ in range(40):
        n = int(g.log_uniform(0.0, 4.0)) + 1
        seeded.append((n, g.unit()))

    def point(n, p, x):
        row(m, "Binomial::pdf", "C", [i(n), f(p), f(x)])
        row(m, "Binomial::cdf", "C", [i(n), f(p), f(x)])

    for n, p in fixed:
        nf = float(n)
        mean = nf * p
        sd = math.sqrt(nf * p * (1.0 - p))
        xs = [float(k) for k in range(41) if float(k) <= nf]
        xs += [nf - float(k) for k in range(11) if nf - float(k) >= 0.0]
        xs += [floor(mean + sd * (float(j) / 2.0)) for j in range(-24, 25)]
        xs += [nf, nf + 1.0, 0.5, 2.5, 3.0000000000000004, -1.0, -0.0, mean, 1e300] + specials()
        for x in xs:
            point(n, p, x)
    for n, p in seeded:
        nf = float(n)
        sd = math.sqrt(nf * p * (1.0 - p))
        for j in range(-12, 13):
            point(n, p, floor(nf * p + sd * (float(j) / 2.0)))
    for k in range(1000):
        n = int(g.log_uniform(0.0, 5.0)) + 1
        p = g.unit()
        x = floor(float(n) * g.unit())
        point(n, p, x + 0.5 if k % 10 == 0 else x)
    for n, p in fixed + seeded:
        row(m, "Binomial::mean", "A", [i(n), f(p)])
        row(m, "Binomial::variance", "A", [i(n), f(p)])
    for seed in SEEDS:
        for k, (n, p) in enumerate(fixed):
            count = 10000 if k == 0 or (n, p) == (1000, 0.5) else 256
            row(m, "Binomial::sample", "C", [i(n), f(p), i(seed), i(count)])


def emit():
    from . import m_mvn
    m = Module("dist")
    g = Rng(0x510E527FADE682D1)
    normal(m, g)
    uniform(m, g)
    exponential(m, g)
    bernoulli(m, g)
    cauchy(m, g)
    weibull(m, g)
    gamma(m, g)
    chi_squared(m, g)
    gamma_standard(m)
    beta(m, g)
    student_t(m, g)
    f_distribution(m, g)
    tails(m, g)
    poisson(m, g)
    poisson_ptrs(m)
    binomial(m, g)
    m_mvn.emit(m, g)
    return m
