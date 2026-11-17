"""`monte_carlo.tsv` (M6 part b): 1.3.0's monte_carlo.rs. `monte_carlo_integrate` on nine integrands (the harness's
`_par_mc_f`: polynomials, sin, exp(−x²), 1/x through a pole, a large offset, √x past its domain, a constant) over
finite, tiny, huge and refused intervals; `monte_carlo_pi`; `metropolis_hastings` on ten log-densities (Gaussian,
the box with −∞ or NaN outside, a zero-density start, a mixture, Rosenbrock's, always −∞, flat) in one to three
dimensions over the σ grid; `gibbs_sampling` on five conditionals (the harness's `_par_mc_cond`) in one to four
dimensions; every refusal (run lengths that overflow among them). Each from `SimpleRng::new(seed)`. A chain's samples
are one flat slice, then its acceptance rate. Class C where a row reaches ln, cos, sin or exp (Box–Muller among them);
the rest is arithmetic."""
from pathlib import Path

from .grids import INF, MAXF, NAN, Rng, TINY
from .models import V
from .tsv import Module, f, from_bits, from_model, i, parsed, v

ROOT = Path(__file__).resolve().parents[3]
_PRIMS = None
U64 = (1 << 64) - 1


def prims():
    global _PRIMS
    if _PRIMS is None:
        _PRIMS = V.dist_prims(ROOT)[0]
    return _PRIMS


def sl(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def isl(xs):
    return [v(len(xs))] + [i(x) for x in xs]


def row(m, fn, cls, ins):
    s, sc = V.take_slices(parsed(ins))
    m.row(fn, cls, ins, from_model(V.MC[fn](prims(), s, sc)))


def emit():
    m = Module("monte_carlo")
    g = Rng(0xC0AC29B7C97C50DD)
    seeds = [0, 1, 42, U64, g.word(), g.word()]
    # monte_carlo_integrate
    spans = [(0.0, 1.0), (-1.0, 1.0), (0.0, 3.141592653589793), (-3.0, 3.0), (1.0, 2.0), (-1e300, 1e300),
             (1.0, 1.0000000000000002), (-1.0, 0.5), (1e15, 1e15 + 2.0)]
    bad = [(-MAXF, MAXF), (NAN, 1.0), (0.0, INF), (-INF, 0.0), (1.0, 1.0), (2.0, 1.0), (-0.0, 0.0), (0.0, NAN)]
    for fid in range(9):
        cls = "C" if fid in (3, 4) else "A"
        for a, b in spans:
            for n in (1, 2, 10, 1000):
                for seed in seeds[:3]:
                    row(m, "monte_carlo::monte_carlo_integrate", cls, [i(fid), f(a), f(b), i(n), i(seed)])
        for seed in seeds:
            row(m, "monte_carlo::monte_carlo_integrate", cls, [i(fid), f(0.0), f(1.0), i(5000 if cls == "A" else 2000),
                                                              i(seed)])
        for a, b in bad:
            row(m, "monte_carlo::monte_carlo_integrate", cls, [i(fid), f(a), f(b), i(10), i(1)])
        row(m, "monte_carlo::monte_carlo_integrate", cls, [i(fid), f(0.0), f(1.0), i(0), i(1)])
    # monte_carlo_pi
    for n in (0, 1, 2, 3, 10, 100, 1000, 100000):
        for seed in seeds:
            row(m, "monte_carlo::monte_carlo_pi", "A", [i(n), i(seed)])
    # metropolis_hastings: target, initial point, σ, n, burn-in
    starts = {1: [[0.0], [5.0], [-0.0]], 2: [[0.0, 0.0], [3.0, -3.0]], 3: [[0.1, -0.2, 0.3]]}
    for tid in range(10):
        for dim in (1, 2, 3):
            if tid == 6 and dim != 2:
                continue
            for init in starts[dim]:
                for sd in (0.5, 2.5):
                    for n, burn in ((1, 0), (10, 0), (200, 50)):
                        row(m, "monte_carlo::metropolis_hastings", "C",
                            sl(init) + [i(tid), f(sd), i(n), i(burn), i(seeds[(tid + dim) % 6])])
    for tid, init in ((0, [0.0]), (4, [0.0]), (5, [0.0]), (6, [-1.0, 1.0]), (3, [0.0, 0.0])):
        for sd in (1e-3, 1.0, 100.0, TINY):
            row(m, "monte_carlo::metropolis_hastings", "C", sl(init) + [i(tid), f(sd), i(300), i(10), i(7)])
        row(m, "monte_carlo::metropolis_hastings", "C", sl(init) + [i(tid), f(1.0), i(2000), i(100), i(99)])
    for init, sd, n, burn in (([], 1.0, 10, 0), ([0.0], 0.0, 10, 0), ([0.0], -1.0, 10, 0), ([0.0], NAN, 10, 0),
                              ([0.0], INF, 10, 0), ([0.0], -0.0, 10, 0), ([0.0], 1.0, 0, 0), ([0.0], 1.0, U64, 0),
                              ([0.0], 1.0, U64, 1), ([0.0], 1.0, 1, U64), ([], 0.0, 0, 0), ([0.0], NAN, 0, 0)):
        row(m, "monte_carlo::metropolis_hastings", "C", sl(init) + [i(0), f(sd), i(n), i(burn), i(1)])
    # gibbs_sampling: conditionals (ids, parameters), initial point, n, burn-in
    gcases = [([1], [0.0], [0.0]), ([1, 1], [0.0, 0.0], [0.0, 0.0]), ([0, 0], [0.8, 0.8], [1.0, -1.0]),
              ([0, 0, 0], [0.5, -0.3, 0.9], [0.0, 0.0, 0.0]), ([2, 3], [0.0, 0.0], [0.5, 0.5]),
              ([4, 4], [1.0, -2.0], [0.0, 10.0]), ([3, 2, 4, 1], [0.0, 0.0, 3.0, 0.0], [0.0, 1.0, 2.0, 3.0]),
              ([2], [0.0], [-0.0])]
    for cids, pars, init in gcases:
        cls = "C" if any(c in (0, 1) for c in cids) else "A"
        for n, burn in ((1, 0), (10, 0), (300, 20)):
            for seed in seeds[:3]:
                row(m, "monte_carlo::gibbs_sampling", cls, isl(cids) + sl(pars) + sl(init) + [i(n), i(burn), i(seed)])
    for cids, pars, init, n, burn in (([], [], [], 10, 0), ([1], [0.0], [], 10, 0), ([1], [0.0], [0.0, 0.0], 10, 0),
                                      ([1, 1], [0.0, 0.0], [0.0], 10, 0), ([1], [0.0], [0.0], 0, 0),
                                      ([1], [0.0], [0.0], 1, U64), ([1], [0.0], [0.0], U64, 1), ([], [], [0.0], 0, 0)):
        row(m, "monte_carlo::gibbs_sampling", "C", isl(cids) + sl(pars) + sl(init) + [i(n), i(burn), i(1)])
    return m
