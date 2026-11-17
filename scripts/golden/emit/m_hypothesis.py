"""`hypothesis.tsv` (M4): 1.3.0's hypothesis.rs. Part b1, the six tests: the t-tests, the χ² test, the one-way ANOVA
and the two KS tests, on seeded, constant, tie- and ±0-heavy, overflowing, NaN- and ∞-bearing samples; every refusal
in Rust's order; significance levels at and beside (0, 1)'s ends. Class C: the p-values come from the T1 incomplete
beta and gamma (the t, F and χ² tails), the T1 Φ and the correctly rounded exp (the Kolmogorov series). A slice is
`v:n` and its values; the one-sample KS test's CDF is `e:Cdf.<Name>` (validate.py's KS_CDFS, the harness's twins); a
TestResult is its name (`s:`), statistic, p-value, df, level and verdict (`i:` 0 / 1)."""
from pathlib import Path

from .grids import INF, MAXF, NAN, TINY, Rng
from .models import V
from .tsv import Module, e, f, from_model, parsed, v

ROOT = Path(__file__).resolve().parents[3]
_PRIMS = None


def prims():
    global _PRIMS
    if _PRIMS is None:
        _PRIMS = V.dist_prims(ROOT)[0]
    return _PRIMS


def sl(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def row(m, fn, ins):
    s, sc = V.take_slices(parsed(ins))
    m.row(fn, "C", ins, from_model(V.HYP[fn](prims(), s, sc)))


ALPHAS = [0.05, 0.01, 0.5, 0.0, 1.0, -0.0, NAN, INF, TINY, 0.9999999999999999, -0.05]


def samples(g):
    out = [[1.0, 2.0, 3.0, 4.0, 5.0], [5.1, 4.9, 5.0, 5.2, 4.8, 5.1, 4.9, 5.0, 5.3, 4.7], [2.0, 2.0, 2.0], [1.0],
           [], [1.0, NAN, 3.0], [1.0, INF, 3.0], [MAXF, -MAXF, 0.0], [1e308, 1e308, 1e308, -1e308], [TINY, 0.0, -TINY],
           [-0.0, 0.0, 0.0, -0.0, 1.0], [1e16, 1e16 + 2.0, 1e16 + 4.0]]
    for n in (2, 3, 5, 8, 13, 21, 34, 50):
        for k in range(3):
            loc = g.uniform(-5.0, 5.0)
            sc = g.log_uniform(-3.0, 3.0)
            out.append([loc + sc * g.uniform(-1.0, 1.0) for _ in range(n)])
    for n in (4, 9, 30):
        out.append([float(g.int(0, 3)) for _ in range(n)])
    return out


def emit():
    m = Module("hypothesis")
    g = Rng(0x243F6A8885A308D3)
    ss = samples(g)
    # one-sample t-test: each sample against μ₀ grids, then the level grid on one
    for xs in ss:
        for mu in (0.0, 3.0, 5.0, -1.5, 1e300):
            row(m, "hypothesis::t_test_one_sample", sl(xs) + [f(mu), f(0.05)])
    for mu in (INF, -INF, NAN):
        row(m, "hypothesis::t_test_one_sample", sl(ss[0]) + [f(mu), f(0.05)])
    for a in ALPHAS:
        row(m, "hypothesis::t_test_one_sample", sl(ss[0]) + [f(3.0), f(a)])
        row(m, "hypothesis::t_test_one_sample", sl([1.0]) + [f(3.0), f(a)])
    # two-sample Welch: every pair of a subset, both orders
    pick = ss[:12] + ss[12::4]
    for a in pick:
        for b in pick:
            row(m, "hypothesis::t_test_two_sample", sl(a) + sl(b) + [f(0.05)])
    for a in ALPHAS:
        row(m, "hypothesis::t_test_two_sample", sl(ss[0]) + sl(ss[1]) + [f(a)])
    # variances near 1e-170: v² underflows, the Welch denominator is 0 and the df falls back to 1
    row(m, "hypothesis::t_test_two_sample", sl([0.0, 1e-85, 2e-85]) + sl([0.0, 3e-85, 1e-85]) + [f(0.05)])
    row(m, "hypothesis::t_test_two_sample", sl([0.0, 1e-85, 2e-85]) + sl([5.0, 5.0, 5.0]) + [f(0.05)])
    # χ²
    cases = [([10.0, 20.0, 30.0], [20.0, 20.0, 20.0]), ([20.0, 20.0, 20.0], [20.0, 20.0, 20.0]), ([5.0], [5.0]),
             ([7.0], [3.0]), ([], []), ([1.0, 2.0], [1.0]), ([1.0, 2.0], [0.0, 3.0]), ([1.0, 2.0], [-1.0, 3.0]),
             ([-1.0, 2.0], [1.0, 3.0]), ([NAN, 2.0], [1.0, 3.0]), ([1.0, 2.0], [INF, 3.0]), ([0.0, 0.0], [1.0, 1.0]),
             ([-0.0, 5.0], [2.5, 2.5]), ([1e300, 0.0], [TINY, 1.0]), ([MAXF, 0.0], [1.0, 1.0])]
    for _ in range(60):
        k = g.int(2, 12)
        exp_ = [g.uniform(1.0, 50.0) for _ in range(k)]
        obs = [float(g.int(0, 60)) for _ in range(k)]
        cases.append((obs, exp_))
    for obs, exp_ in cases:
        row(m, "hypothesis::chi_squared_test", sl(obs) + sl(exp_) + [f(0.05)])
    for a in ALPHAS:
        row(m, "hypothesis::chi_squared_test", sl([10.0, 20.0, 30.0]) + sl([20.0, 20.0, 20.0]) + [f(a)])
    # ANOVA
    groups = [[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]], [[1.0, 2.0, 3.0]], [], [[1.0], [2.0]],
              [[1.0, 2.0], [3.0], [4.0]], [[1.0, 2.0], []], [[2.0, 2.0], [2.0, 2.0]], [[1.0, 1.0], [2.0, 2.0]],
              [[1.0, NAN], [2.0, 3.0]], [[1.0, INF], [2.0, 3.0]], [[MAXF, -MAXF], [1.0, 2.0]], [[1e300, 1e300], [-1e300]],
              [[-0.0, 0.0], [0.0, 1.0]], [[5.0, 5.1, 4.9], [5.0, 5.2, 4.8], [5.1, 4.9, 5.0]]]
    for _ in range(40):
        k = g.int(2, 6)
        grp = []
        for _ in range(k):
            loc = g.uniform(-3.0, 3.0)
            grp.append([loc + g.uniform(-1.0, 1.0) for _ in range(g.int(1, 15))])
        groups.append(grp)
    for grp in groups:
        ins = []
        for x in grp:
            ins += sl(x)
        row(m, "hypothesis::one_way_anova", ins + [f(0.05)])
    for a in ALPHAS:
        row(m, "hypothesis::one_way_anova", sl([1.0, 2.0, 3.0]) + sl([4.0, 5.0, 6.0]) + [f(a)])
    # KS two-sample
    ks = [([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]), ([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), ([1.0, 1.0, 2.0], [1.0, 2.0, 2.0]),
          ([], [1.0]), ([1.0], []), ([1.0, NAN], [1.0]), ([1.0], [INF]), ([-0.0, 0.0, 1.0], [0.0, -0.0, 2.0]),
          ([0.5], [0.5]), ([MAXF, -MAXF], [0.0])]
    for _ in range(80):
        na, nb = g.int(1, 60), g.int(1, 60)
        sh = g.uniform(-0.5, 0.5)
        a = [g.uniform(0.0, 1.0) for _ in range(na)]
        b = [g.uniform(0.0, 1.0) + sh for _ in range(nb)]
        if g.int(0, 3) == 0:
            a = [float(int(x * 5.0)) for x in a]
            b = [float(int(x * 5.0)) for x in b]
        ks.append((a, b))
    for a, b in ks:
        row(m, "hypothesis::ks_two_sample", sl(a) + sl(b) + [f(0.05)])
    for a in ALPHAS:
        row(m, "hypothesis::ks_two_sample", sl([1.0, 2.0, 3.0]) + sl([4.0, 5.0, 6.0]) + [f(a)])
    # KS one-sample: each CDF on seeded samples and the edges
    one = [[0.1, 0.5, 0.9], [0.5], [], [NAN], [INF], [-0.0, 0.0], [0.25, 0.25, 0.75], [-40.0, 40.0], [MAXF]]
    for _ in range(40):
        n = g.int(1, 80)
        one.append([g.uniform(-3.0, 3.0) for _ in range(n)])
    for _ in range(20):
        n = g.int(1, 80)
        one.append([g.uniform(0.0, 1.0) for _ in range(n)])
    for cdf in ["StdNormal", "Uniform01", "Exponential1", "Overshoot", "NanAtHalf"]:
        for xs in one:
            row(m, "hypothesis::ks_one_sample", sl(xs) + [e("Cdf", cdf), f(0.05)])
    for a in ALPHAS:
        row(m, "hypothesis::ks_one_sample", sl([0.1, 0.5, 0.9]) + [e("Cdf", "Uniform01"), f(a)])
    b2_rows(m, Rng(0x082EFA98EC4E6C89), ss)
    return m


def b2_row(m, fn, ins):
    s, sc = V.take_slices(parsed(ins))
    m.row(fn, "C", ins, from_model(V.HYP_B2[fn](prims(), s, sc)))


LEVELS = [0.95, 0.99, 0.5, 0.9, 0.999, 0.0, 1.0, NAN, -0.5, TINY, 0.9999999999999999]


def b2_rows(m, g, ss):
    """Part b2: the two quantiles (1.3.0's private hooks) and the three confidence intervals."""
    ps = [0.5, 0.975, 0.995, 0.025, 0.9, 0.1, 0.02425, 0.97575, 0.024249999999999997, 0.9757500000000001, 1e-10,
          1.0 - 1e-10, 1e-300, TINY, 0.0, 1.0, -0.0, NAN, 0.5000000000000001, 0.4999999999999999, 0.5 + 1e-15,
          0.9999999999999999]
    for _ in range(40):
        ps.append(g.unit())
    for p in ps:
        b2_row(m, "hypothesis::z_quantile", [f(p)])
    dfs = [1.0, 2.0, 4.0, 9.0, 29.0, 100.0, 1e6, 0.5, NAN]
    tps = [0.975, 0.995, 0.5, 0.9, 0.025, 1e-6, 0.999999, 0.0, 1.0, NAN, 0.5 + 1e-15]
    for df in dfs:
        for p in tps:
            b2_row(m, "hypothesis::t_quantile", [f(p), f(df)])
    for xs in ss[:14] + ss[14::5]:
        for cl in (0.95, 0.99):
            b2_row(m, "hypothesis::ci_mean", sl(xs) + [f(cl)])
    for cl in LEVELS:
        b2_row(m, "hypothesis::ci_mean", sl(ss[0]) + [f(cl)])
    pick = ss[:6] + ss[12::6]
    for a in pick:
        for b in pick:
            b2_row(m, "hypothesis::ci_two_means", sl(a) + sl(b) + [f(0.95)])
    for cl in LEVELS:
        b2_row(m, "hypothesis::ci_two_means", sl(ss[0]) + sl(ss[1]) + [f(cl)])
    U64 = (1 << 64) - 1
    pairs = [(50, 100), (0, 100), (100, 100), (1, 1), (0, 1), (5, 0), (101, 100), (U64, U64), (1, U64), (1 << 63, U64),
             (U64, 1 << 63), (7, 13)]
    for _ in range(30):
        n = g.int(1, 10000)
        pairs.append((g.int(0, n), n))
    from .tsv import i as itok
    for k, n in pairs:   # u64 words: one at or above 2^63 is a negative `i:` token
        b2_row(m, "hypothesis::ci_proportion", [itok(V.to_i64(k)), itok(V.to_i64(n)), f(0.95)])
    for cl in LEVELS:
        b2_row(m, "hypothesis::ci_proportion", [itok(50), itok(100), f(cl)])
