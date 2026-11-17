"""`bayesian.tsv` (M5 part c): 1.3.0's bayesian.rs. `BayesianUpdate::new` (its fields and posterior, or the refusal)
on a grid of probabilities with the coherence bound at, below and above 1 + 1e-12; `bayes_theorem` on the same grid
and the specials; `naive_bayes_classify` on seeded problems, ties, zero and negative priors and likelihoods, and every
refusal. Class C: naive Bayes' ln; the update and the theorem are arithmetic."""
from pathlib import Path

from .grids import INF, NAN, TINY, Rng
from .models import V
from .tsv import Module, bits, f, from_bits, from_model, parsed, v

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
    m.row(fn, cls, ins, from_model(V.BAYES[fn](prims(), s, sc)))


def emit():
    m = Module("bayesian")
    g = Rng(0x7B54A41DC25A59B5)
    ps = [0.0, -0.0, 0.01, 0.3, 0.5, 0.9, 1.0, 1.0000000000000002, -1e-300, NAN, INF, TINY]
    for p in ps:
        for l in ps:
            for e in (0.1, 0.5, 1.0, 0.0, 0.2, NAN):
                row(m, "BayesianUpdate::new", "A", [f(p), f(l), f(e)])
                row(m, "bayesian::bayes_theorem", "A", [f(p), f(l), f(e)])
    # coherence: l·p against e·(1 + 1e-12), at, below and above the bound
    lim = 0.2 * (1.0 + 1e-12)
    for e2 in (lim, from_bits(bits(lim) - 1), from_bits(bits(lim) + 1)):
        row(m, "BayesianUpdate::new", "A", [f(0.5), f(0.4 * e2 / 0.2), f(0.2)])
    row(m, "BayesianUpdate::new", "A", [f(0.5), f(0.4), f(0.2)])
    row(m, "BayesianUpdate::new", "A", [f(0.5), f(0.9), f(0.2)])
    for _ in range(100):
        p, l = g.unit(), g.unit()
        row(m, "BayesianUpdate::new", "A", [f(p), f(l), f(min(1.0, p * l + g.unit() * 0.5))])
        row(m, "bayesian::bayes_theorem", "A", [f(g.signed_log_uniform(-10, 10)), f(g.signed_log_uniform(-10, 10)),
                                                 f(g.signed_log_uniform(-10, 10))])
    # naive Bayes
    cases = [([1.0, 0.5], [0.5, 0.5], [[0.9, 0.8], [0.1, 0.2]]), ([1.0], [], []), ([1.0], [0.5, 0.5], [[0.5]]),
             ([1.0, 2.0], [0.5, 0.5], [[0.5], [0.5, 0.5]]), ([1.0], [0.5, 0.5], [[0.5], [0.5]]),
             ([1.0], [0.0, 0.0], [[0.5], [0.9]]), ([1.0], [-0.5, 0.5], [[0.9], [0.1]]), ([1.0], [0.5, 0.5], [[0.0], [-1.0]]),
             ([1.0], [NAN, 0.5], [[0.9], [0.1]]), ([1.0], [0.5, 0.5], [[NAN], [0.1]]), ([], [0.3, 0.7], [[], []]),
             ([0.5] * 10, [0.25] * 4, [[0.1 * (c + 1)] * 10 for c in range(4)])]
    for _ in range(60):
        nc, nf = g.int(1, 6), g.int(0, 8)
        priors = [g.unit() for _ in range(nc)]
        rows = [[g.unit() if g.int(0, 9) else 0.0 for _ in range(nf)] for _ in range(nc)]
        cases.append(([g.uniform(-1.0, 1.0) for _ in range(nf)], priors, rows))
    for feats, priors, rows in cases:
        ins = sl(feats) + sl(priors)
        for r in rows:
            ins += sl(r)
        row(m, "bayesian::naive_bayes_classify", "C", ins)
    return m
