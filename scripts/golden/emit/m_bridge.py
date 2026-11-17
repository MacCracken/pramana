"""`bridge.tsv`: 1.3.0's eleven bridge functions (M1 part e3) on grids of specials (with the functions' 1e-15 floor and
its neighbours), empty, single, constant, mismatched and NaN-bearing slices, the fits' degenerate records, and seeded
values. Class B, but the likelihood and the two fits (class C: glibc's exp, ln and log2, corrected in bridge.cr.tsv).
A slice is `v:n` and its values; a slice of pairs is `v:n` then each pair's `i:` and `f:`; a tuple its elements."""
from .grids import INF, MAXF, MIN_POSITIVE, NAN, TINY, Rng
from .models import V
from .tsv import Module, bits, f, from_bits, i, parsed, v

U64 = (1 << 64) - 1
FLOOR = 1e-15


def specials():
    return [0.0, -0.0, 1.0, -1.0, 0.5, 2.5, -3.25, 1.96, 1e-300, 1e300, MIN_POSITIVE, TINY, MAXF, FLOOR,
            from_bits(bits(FLOOR) - 1), from_bits(bits(FLOOR) + 1), NAN, INF, -INF]


def slice_(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def pairs(xs):
    out = [v(len(xs))]
    for w, x in xs:
        out += [i(w), f(x)]
    return out


def seeded(g, k):
    out = []
    for j in range(k):
        r = j % 5
        if r == 0:
            out.append(g.uniform(-10.0, 10.0))
        elif r == 1:
            out.append(g.log_uniform(-12.0, 12.0))
        elif r == 2:
            out.append(-g.log_uniform(-12.0, 12.0))
        elif r == 3:
            out.append(g.uniform(0.0, 1.0))
        else:
            out.append(g.log_uniform(-300.0, 300.0))
    return out


def row(m, fn, cls, ins):
    outs = V.bridge_model(fn, parsed(ins), V._libm.exp, V._libm.log, V._libm.log2)
    m.row(fn, cls, ins, [f(x) for x in outs])


def emit():
    m = Module("bridge")
    g = Rng(0x510E527FADE682D1)
    sp = specials()
    grid = [(a, b) for a in sp for b in sp]
    s1 = seeded(g, 300)
    s2 = seeded(g, 300)
    grid += list(zip(s1, s2))
    for a, b in grid:
        row(m, "bridge::ensemble_spread_to_confidence", "B", [f(a), f(b)])
        row(m, "bridge::observation_error_to_likelihood", "C", [f(a), f(b)])
        row(m, "bridge::uncertainty_to_gaussian", "B", [f(a), f(b)])
        row(m, "bridge::sensor_noise_to_std_dev", "B", [f(a), f(b)])
        row(m, "bridge::z_rates_to_d_prime", "B", [f(a), f(b)])
    counts = [0, 1, 2, 3, 10, 1000, (1 << 53) + 1, U64]
    few = [0.0, -0.0, 1.0, 2.5, 1e-300, 1e300, NAN, INF, 1e-15]
    for k in counts:
        for s in few:
            for t in few:
                row(m, "bridge::item_variances_to_alpha", "B", [i(k), f(s), f(t)])
    for t in [FLOOR, from_bits(bits(FLOOR) - 1), from_bits(bits(FLOOR) + 1)]:
        for k in (2, 7):
            row(m, "bridge::item_variances_to_alpha", "B", [i(k), f(t * 0.5), f(t)])
    for _ in range(200):
        k = g.int(0, 40)
        s = g.log_uniform(-3.0, 3.0)
        t = g.log_uniform(-3.0, 3.0)
        row(m, "bridge::item_variances_to_alpha", "B", [i(k), f(s), f(t)])
    reps = []
    for na in counts:
        for nb in (0, 1, 5, U64):
            for sa, sb in ((1.0, 2.0), (0.0, 0.0), (1e-16, 0.0), (1e-15, 0.0), (NAN, 1.0), (INF, 1.0)):
                reps.append((3.5, sa, na, 1.25, sb, nb))
    for k in range(60):
        reps.append((1.0, from_bits(bits(FLOOR) - 30 + k), 1, 0.5, 0.0, 1))
    for _ in range(300):
        na = g.int(1, 50)
        nb = g.int(1, 50)
        ma = g.uniform(-5.0, 5.0)
        sa = g.log_uniform(-3.0, 1.0)
        mb = g.uniform(-5.0, 5.0)
        sb = g.log_uniform(-3.0, 1.0)
        reps.append((ma, sa, na, mb, sb, nb))
    for ma, sa, na, mb, sb, nb in reps:
        row(m, "bridge::replicates_to_t_statistic", "B", [f(ma), f(sa), i(na), f(mb), f(sb), i(nb)])
    slices = [[], [2.5], [NAN], [INF], [-0.0], [1e300, -1e300], [3.0] * 7]
    slices += [[x, 1.0, 2.0] for x in sp]
    for k in range(200):
        n = g.int(2, 20)
        if k % 3 == 0:
            vs = [1e8 + g.uniform(-1.0, 1.0) for _ in range(n)]
        else:
            vs = seeded(g, n)
        if k % 25 == 0:
            vs[0] = NAN
        slices.append(vs)
    for vs in slices:
        row(m, "bridge::yields_to_mean_variance", "B", slice_(vs))
    twos = [([], []), ([1.0], [2.0]), ([1.0, 2.0], [3.0]), ([], [1.0])]
    for k in range(200):
        n = g.int(2, 15)
        a = seeded(g, n)
        bb = [2.0 * x + g.uniform(-1.0, 1.0) for x in a]
        if k % 20 == 0:
            bb[n - 1] = NAN
        twos.append((a, bb))
    for a, bb in twos:
        row(m, "bridge::trait_scores_to_correlation", "B", slice_(a) + slice_(bb))
    trials = [[], [(1, 10.0)], [(3, 5.0), (3, 4.0), (3, 6.0)], [(n, 60.0 / float(n)) for n in range(1, 13)],
              [(0, 1.0), (1, 0.0), (2, -1.0), (3, NAN), (4, INF), (5, 2.0), (6, 1.5)],
              [(1, 2.0), ((1 << 32) - 1, 1e-300), (2, 1e300)]]
    for _ in range(200):
        n_ = g.int(0, 15)
        vs = []
        for k in range(n_):
            n = g.int(0, 1000)
            if k % 9 == 0:
                t = -1.0
            elif k % 9 == 1:
                t = 0.0
            else:
                t = 40.0 / (1.0 + 0.01 * float(n)) + g.uniform(0.0, 3.0)
            vs.append((n, t))
        trials.append(vs)
    for vs in trials:
        row(m, "bridge::learning_trials_to_power_law", "C", pairs(vs))
    hick = [[], [(2, 0.3)], [(4, 0.5), (4, 0.6)], [(1 << k, 0.2 + 0.15 * float(k)) for k in range(10)],
            [(0, 0.1), (1, NAN), (2, INF), (3, -0.5), (8, 0.6), (16, 0.7)],
            [(1, 0.2), ((1 << 53) + 1, 0.9), (U64, 1.1)]]
    for _ in range(200):
        n_ = g.int(0, 15)
        vs = []
        for k in range(n_):
            e = g.int(0, 12)
            n = 0 if k % 7 == 0 else 1 << e
            t = NAN if k % 11 == 0 else 0.18 + 0.12 * float(e) + g.uniform(0.0, 0.05)
            vs.append((n, t))
        hick.append(vs)
    for vs in hick:
        row(m, "bridge::response_times_to_hick_params", "C", pairs(vs))
    return m
