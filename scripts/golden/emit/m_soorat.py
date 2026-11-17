"""`soorat.tsv` (M7 part a): 1.3.0's integration/soorat.rs. `HistogramData::from_data` on seeded, constant (at 1 and
at 1e12, where the two-ulp floor sets the width), mixed-sign, signed-zero, subnormal, overflowing (±f64::MAX: an
infinite width) and non-finite data over the bin grid, n_bins = 0 among them; the derived PartialEq of each shape on
equal pairs and on pairs one field apart (a NaN unequal to itself, −0 equal to +0, a length, a byte of a name). A
value is its codec tokens. Class A: arithmetic."""
from .grids import INF, MAXF, NAN, TINY, Rng
from .models import V
from .tsv import Module, f, from_model, i, parsed, s, v

U64 = (1 << 64) - 1


def fl(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def ul(xs):
    return [v(len(xs))] + [i(x) for x in xs]


def sv(xs):
    return [v(len(xs))] + [s(x) for x in xs]


def row(m, fn, ins):
    sl, sc = V.take_slices(parsed(ins))
    m.row(fn, "A", ins, from_model(V.SOORAT[fn](None, sl, sc)))


def curve_toks(c):
    x, p, q, name = c
    return fl(x) + fl(p) + fl(q) + [s(name)]


def trace_toks(t):
    a, b, name = t
    return fl(a) + fl(b) + [s(name)]


def fit_toks(ft):
    vecs, r2 = ft
    return [t for vec in vecs for t in fl(vec)] + [f(r2)]


def hist_toks(h):
    e, c, tot = h
    return fl(e) + ul(c) + [i(tot)]


def corr_toks(c):
    vals, size, labels = c
    return fl(vals) + [i(size)] + sv(labels)


def variants(g, base, kind):
    """Pairs of base against itself, and against copies one field apart."""
    out = [(base, base)]
    if kind == "DistributionCurve":
        x, p, q, name = base
        out += [(base, (x[:-1], p, q, name)), (base, (x, p + [0.5], q, name)), (base, (x, p, q, name + "x")),
                (base, ([NAN] + x[1:], p, q, name)), (([NAN] + x[1:], p, q, name), ([NAN] + x[1:], p, q, name)),
                (base, ([-0.0 if k == 0 else t for k, t in enumerate(x)], p, q, name)),
                (base, (x, p, [q[0] + 1e-16] + q[1:], name)), (base, (x, p, q, ""))]
    elif kind == "McmcTrace":
        a, b, name = base
        out += [(base, (a, b[:-1], name)), (base, ([a[0] + 1.0] + a[1:], b, name)), (base, (a, b, "μ")),
                (base, (a + [NAN], b, name)), ((a + [NAN], b, name), (a + [NAN], b, name))]
    elif kind == "RegressionFit":
        vecs, r2 = base
        out += [(base, (vecs, r2 + 1e-12)), (base, (vecs, NAN)), ((vecs, NAN), (vecs, NAN)),
                (base, ([vecs[0], vecs[1], vecs[2][:-1], vecs[3], vecs[4]], r2)),
                (base, ([vecs[0], vecs[1], vecs[2], vecs[3], [-0.0] + vecs[4][1:]], r2))]
    elif kind == "HistogramData":
        e, c, tot = base
        out += [(base, (e, c, tot + 1)), (base, (e, c[:-1] + [c[-1] + 1], tot)), (base, (e[:-1], c, tot)),
                (base, (e, c, U64)), (base, ([NAN] + e[1:], c, tot))]
    else:
        vals, size, labels = base
        out += [(base, (vals, size + 1, labels)), (base, (vals, size, labels[:-1])),
                (base, (vals, size, labels[:-1] + [labels[-1] + "\x00"])), (base, ([NAN] + vals[1:], size, labels)),
                (base, ([-0.0] + vals[1:], size, labels))]
    return out


TOKS = {"DistributionCurve": curve_toks, "McmcTrace": trace_toks, "RegressionFit": fit_toks,
        "HistogramData": hist_toks, "CorrelationMatrix": corr_toks}


def bases(g):
    return {
        "DistributionCurve": [([0.0, 1.0, 2.0], [0.0, 0.5, 0.0], [0.0, 0.5, 1.0], "Normal(0,1)"),
                              ([g.signed_log_uniform(-5, 5) for _ in range(5)], [g.unit() for _ in range(4)],
                               [g.unit() for _ in range(6)], "a\"b\\c\n")],
        "McmcTrace": [([1.0, 1.1, 0.9, 1.05], [-10.0, -9.5, -9.8, -9.3], "mu"),
                      ([g.signed_log_uniform(-5, 5) for _ in range(7)], [-g.log_uniform(-2, 4) for _ in range(7)], "σ")],
        "RegressionFit": [([[1.0, 2.0, 3.0], [2.1, 3.9, 6.1], [2.0, 4.0, 6.0], [2.5, 4.5, 6.5], [1.5, 3.5, 5.5]], 0.998),
                          ([[g.unit() for _ in range(k + 1)] for k in range(5)], g.unit())],
        "HistogramData": [([1.0, 2.0, 3.0], [2, 1], 3), ([0.5 * k for k in range(9)], [g.int(0, 99) for _ in range(8)],
                                                         777)],
        "CorrelationMatrix": [([1.0, 0.8, 0.8, 1.0], 2, ["x", "y"]),
                              ([g.unit() for _ in range(9)], 3, ["alpha", "β", ""])],
    }


def emit():
    m = Module("soorat")
    g = Rng(0x6A09E667BB67AE85)
    datas = [[], [1.0, 2.0, 3.0, 4.0, 5.0, 2.5, 3.5], [5.0, 5.0, 5.0], [1e12, 1e12, 1e12], [NAN, 1.0, 2.0, INF, 3.0],
             [NAN, NAN], [INF, -INF], [-0.0, 0.0, -0.0], [0.0, -0.0], [TINY, -TINY, 0.0], [MAXF, -MAXF, 0.0],
             [MAXF, MAXF], [-1e300, 1e300], [1.0], [1.0, 1.0000000000000002], [-3.0, 7.0, 7.0, -3.0],
             [1e16, 1e16 + 2.0, 1e16 + 4.0], [0.1 * k for k in range(100)]]
    for n in (10, 50, 200):
        for _ in range(4):
            datas.append([g.signed_log_uniform(-3, 3) for _ in range(n)])
        datas.append([g.uniform(-1.0, 1.0) if g.int(0, 9) else NAN for _ in range(n)])
    for data in datas:
        for nb in (0, 1, 2, 3, 4, 7, 10, 33, 100):
            row(m, "HistogramData::from_data", fl(data) + [i(nb)])
    for kind, bs in bases(g).items():
        for base in bs:
            for a, b in variants(g, base, kind):
                row(m, f"{kind}::eq", TOKS[kind](a) + TOKS[kind](b))
    return m
