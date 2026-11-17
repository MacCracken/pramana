"""`special.tsv`: 1.3.0's special functions, src/math.rs (M2; ADR 009), named `math::<fn>`: erf and erfc on fdlibm's
branch points, Φ and 1 − Φ, lnΓ and its relatives, the incomplete gamma and beta, on validate.py's transcriptions
through glibc (1.3.0's bits). Class C: validate.py writes special.cr.tsv (a primitive's misround) and special.ref.tsv
(the T1 functions' true values, and Rust's formulas fed them), which the port is held to (ADR 009's bounds)."""
import math

from .grids import NAN, INF, Rng, pow2, specials
from .models import V
from .tsv import Module, bits, f, from_bits

THRESHOLDS = [0x3C700000, 0x3E300000, 0x3FD00000, 0x3FEB0000, 0x3FF40000, 0x4006DB6D, 0x40180000, 0x403C0000]


def sort_dedup(xs):
    xs = sorted(xs, key=V.total_key)
    out = []
    for x in xs:
        if not out or bits(out[-1]) != bits(x):
            out.append(x)
    return out


def around(out, x, k):
    b = bits(x)
    for d in range(k + 1):
        out.append(from_bits(b + d))
        if b & 0x7FFFFFFFFFFFFFFF >= d:
            out.append(from_bits(b - d))


def intervals():
    tail = 20.0 / 7.0
    return [(pow2(-28), 0.25), (0.25, 0.84375), (0.84375, 1.25), (1.25, tail), (tail, 6.0), (6.0, 28.0), (28.0, 40.0)]


def erf_grid():
    out = specials()
    for t in THRESHOLDS:
        at = t << 32
        for k in range(5):
            for b0 in (at + k, at - 1 - k):
                x = from_bits(b0)
                out += [x, -x]
    g = Rng(0x51EC1A1FE7F0A1B3)
    for t in THRESHOLDS:
        for hw in (t, t - 1):
            for _ in range(24):
                x = from_bits((hw << 32) | (g.word() & 0xFFFFFFFF))
                out += [x, -x]
    for lo, hi in intervals():
        for _ in range(200):
            x = g.uniform(lo, hi)
            out += [x, -x]
    for _ in range(200):
        out.append(g.signed_log_uniform(-17.0, -8.0))
        out.append(g.signed_log_uniform(-320.0, -17.0))
        out.append(g.signed_log_uniform(-300.0, 300.0))
    out += [float(k) / 32.0 for k in range(-960, 961)]
    out.append(pow2(-1074))
    return sort_dedup(out)


def normal_grid(xs):
    out = [x * math.sqrt(2.0) for x in xs]
    out += [float(k) / 16.0 for k in range(-640, 641)]
    g = Rng(0x0C3DF00D5EED2B1A)
    out += [g.uniform(-40.0, 40.0) for _ in range(400)]
    return sort_dedup(out)


def ln_gamma_grid():
    out = specials()
    around(out, 0.5, 8)
    cut = 1e-300 / math.pi
    around(out, cut, 40)
    around(out, -cut, 40)
    for n in range(31):
        x = -float(n)
        around(out, x, 4)
        around(out, x - 0.5, 2)
    around(out, 1.0, 8)
    around(out, 2.0, 8)
    out += [float(k) / 16.0 for k in range(-320, 481)]
    out += [float(n) for n in range(1, 201)]
    g = Rng(0x6A09E667F3BCC908)
    for _ in range(400):
        out.append(g.uniform(-10.0, 0.5))
        out.append(g.uniform(0.5, 10.0))
        out.append(g.uniform(10.0, 200.0))
    for _ in range(300):
        out.append(g.log_uniform(-310.0, 306.0))
        out.append(-g.log_uniform(-310.0, 15.0))
    return sort_dedup(out)


def stirling_grid():
    out = specials()
    around(out, 10.0, 8)
    out += [float(n) for n in range(1, 101)]
    g = Rng(0xBB67AE8584CAA73B)
    for _ in range(300):
        out.append(g.uniform(0.5, 10.0))
        out.append(g.uniform(10.0, 1000.0))
        out.append(g.log_uniform(-3.0, 300.0))
    for _ in range(100):
        out.append(g.log_uniform(-310.0, -3.0))
        out.append(g.uniform(-10.0, 0.0))
    return sort_dedup(out)


def cross(xs, ys):
    return [(x, y) for x in xs for y in ys]


def bd0_pairs():
    out = cross(specials(), [1.0, 2.5]) + cross([0.0, 1.0, 2.5], specials())
    g = Rng(0x3C6EF372FE94F82B)
    ratios = [0.0, 0.001, 0.1, 0.5, 0.9, 0.99, 1.0, 1.01, 1.1, 2.0, 10.0, 1000.0]
    for _ in range(300):
        mu = g.log_uniform(-3.0, 12.0)
        out += [(mu * r, mu) for r in ratios]
    for _ in range(100):
        mu = g.log_uniform(-3.0, 12.0)
        b0 = bits(mu)
        for d in range(1, 5):
            out += [(from_bits(b0 + d), mu), (from_bits(b0 - d), mu)]
    for _ in range(300):
        k = g.int(0, 100)
        out.append((float(k), g.uniform(0.1, 100.0)))
    for _ in range(300):
        x = g.log_uniform(-3.0, 12.0)
        out.append((x, g.log_uniform(-3.0, 12.0)))
    return out


def kernel_pairs():
    out = cross(specials(), [1.0, 15.0]) + cross([1.0, 15.0], specials())
    tens = []
    around(tens, 10.0, 6)
    out += cross(tens, [1.0, 10.0, 15.0])
    g = Rng(0xA54FF53A5F1D36F1)
    ratios = [0.001, 0.1, 0.5, 0.9, 1.0, 1.1, 2.0, 10.0, 1000.0]
    for _ in range(300):
        a = g.log_uniform(-3.0, 12.0)
        out += [(a, a * r) for r in ratios]
    for _ in range(100):
        a = g.log_uniform(-3.0, 12.0)
        out += [(a, a * (1.0 + float(k) / 64.0)) for k in range(-8, 9)]
    for _ in range(300):
        a = g.log_uniform(-3.0, 12.0)
        out.append((a, g.log_uniform(-3.0, 12.0)))
    return out


def ln_beta_pairs():
    shapes = [0.001, 0.01, 0.1, 0.5, 1.0, 1.5, 2.0, 3.0, 5.0, 9.5, 10.5, 20.0, 100.0, 1e3, 1e6, 1e9, 1e12]
    around(shapes, 10.0, 1)
    shapes = sort_dedup(shapes)
    out = cross(shapes, shapes) + cross(specials(), [1.0, 15.0]) + cross([1.0, 15.0], specials())
    out += [(0.0, -0.0), (-0.0, 0.0), (-0.5, 2.0), (-2.5, 12.0), (12.0, -2.5), (-15.5, 30.0)]
    g = Rng(0x510E527FADE682D1)
    for _ in range(600):
        a = g.log_uniform(-3.0, 12.0)
        out.append((a, g.log_uniform(-3.0, 12.0)))
    for _ in range(300):
        a = g.uniform(0.5, 20.0)
        out.append((a, g.uniform(0.5, 20.0)))
    for _ in range(1000):
        a = g.uniform(10.0, 60.0)
        out.append((a, g.uniform(10.0, 60.0)))
    return out


def gamma_pairs():
    mids = [0.5, 1.0, 30.0]
    out = cross(specials(), mids) + cross(mids, specials())
    for a in [0.001, 0.01, 0.1, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 5.0, 9.5, 10.0, 10.5, 20.0, 50.0, 100.0, 1e3, 1e4, 1e5,
              1e6]:
        xs = []
        around(xs, a + 1.0, 3)
        out += [(a, x) for x in xs]
    wh = 1e9
    was = []
    around(was, wh, 2)
    was += [wh * (1.0 - 1e-12), wh * (1.0 + 1e-12)]
    for a in was:
        for j in [-6.0, -3.0, -1.0, 0.0, 1.0, 3.0, 6.0]:
            out.append((a, wh + j * math.sqrt(wh)))
    g = Rng(0x9B05688C2B3E6C1F)
    for _ in range(50):
        a = g.log_uniform(9.0, 12.0)
        out += [(a, a + float(j) * math.sqrt(a)) for j in range(-6, 7)]
    ratios = [0.001, 0.1, 0.5, 0.9, 0.99, 1.0, 1.01, 1.1, 2.0, 10.0, 100.0]
    for _ in range(300):
        a = g.log_uniform(-3.0, 9.0)
        out += [(a, a * r) for r in ratios]
    for _ in range(60):
        a = g.log_uniform(0.0, 9.0)
        out += [(a, a + float(j) * math.sqrt(a)) for j in range(-4, 5)]
    for k in range(1, 121):
        a = float(k) / 2.0
        out += [(a, a * r) for r in [0.25, 0.5, 1.0, 2.0, 4.0]]
    out += cross([0.5, 1.0, 5.0, 50.0], [700.0, 750.0, 800.0, 1000.0, 1e4])
    out += cross([0.001, 0.5, 1.0, 100.0, 1000.0], [1e-300, 1e-310, from_bits(1)])
    out += cross([1e-300, 1e-100, 1e-30, 1e-20, 1e-16, 1e-12, 1e-8],
                 [1e-10, 0.001, 0.1, 0.5, 0.9, 0.99, 0.999999, 1.0, 1.5, 3.0])
    for _ in range(500):
        a = g.log_uniform(-3.0, 6.0)
        out.append((a, g.log_uniform(-3.0, 6.0)))
    return out


def beta_rows():
    out = []
    for s in specials():
        out += [[2.0, 3.0, s, 1.0 - s], [s, 3.0, 0.3, 0.7], [2.0, s, 0.3, 0.7], [2.0, 3.0, 0.3, s], [s, s, 0.5, 0.5]]
    shapes = [0.001, 0.01, 0.1, 0.5, 1.0, 1.5, 2.0, 5.0, 9.5, 10.5, 20.0, 100.0, 1e3, 1e5]
    around(shapes, 10.0, 1)
    shapes = sort_dedup(shapes)
    xs = [1e-10, 1e-4, 0.01, 0.1, 0.3, 0.5, 0.7, 0.9, 0.99, 0.9999, 1.0 - 1e-10]
    for a in shapes:
        for b0 in shapes:
            out += [[a, b0, x, 1.0 - x] for x in xs]
    g = Rng(0xCBBB9D5DC1059ED8)
    for _ in range(100):
        a = g.log_uniform(-3.0, 5.0)
        b0 = g.log_uniform(-3.0, 5.0)
        split = (a + 1.0) / (a + b0 + 2.0)
        near = []
        around(near, split, 2)
        out += [[a, b0, x, 1.0 - x] for x in near]
    for _ in range(100):
        a = g.log_uniform(1.0, 6.0)
        b0 = g.log_uniform(1.0, 6.0)
        s = a + b0
        mean = a / s
        sd = math.sqrt(a * b0 / (s * s * (s + 1.0)))
        for j in range(-4, 5):
            x = mean + float(j) * sd
            if 0.0 < x < 1.0:
                out.append([a, b0, x, 1.0 - x])
    for nu in [1.0, 2.0, 5.0, 10.0, 30.0, 100.0, 1e3, 1e6]:
        for t in [0.01, 0.5, 1.0, 2.0, 5.0, 20.0, 1e3]:
            t2 = t * t
            out.append([nu / 2.0, 0.5, nu / (nu + t2), t2 / (nu + t2)])
    for d in [10.0, 1e4, 1e6, 1e8]:
        for x in [0.1, 1.0, 3.0, 30.0]:
            out.append([1.0, d / 2.0, 2.0 * x / (2.0 * x + d), d / (2.0 * x + d)])
    out += [[2.0, 3.0, 0.3, 0.5], [2.0, 3.0, 0.3, 0.8], [20.0, 30.0, 0.4, 0.5], [20.0, 30.0, 0.6, 0.3]]
    for a in [1e-300, 1e-20, 1e-8]:
        for b0 in [1e-300, 1e-8, 1.0, 10.0]:
            out += [[a, b0, x, 1.0 - x] for x in [0.001, 0.3, 0.5, 0.7, 0.999]]
    out += [[1e3, 1e3, 0.01, 0.99], [1e5, 1e5, 0.3, 0.7], [1e5, 1e5, 0.7, 0.3], [5.0, 1e6, 0.5, 0.5]]
    for _ in range(1000):
        a = g.log_uniform(-3.0, 5.0)
        b0 = g.log_uniform(-3.0, 5.0)
        x = g.unit()
        out.append([a, b0, x, 1.0 - x])
    return out


def wrapper_pairs(seed):
    g = Rng(seed)
    out = []
    for _ in range(400):
        a = g.log_uniform(-2.0, 4.0)
        out.append((a, a * g.uniform(0.1, 3.0)))
    return out


def setup():
    from pathlib import Path
    root = Path(__file__).resolve().parents[3]
    consts = V.python_constants(root)
    F = {k[len("math::fdlibm::"):]: val for k, val in consts.items() if k.startswith("math::fdlibm::")}
    K = {"F": F, "LG": [consts[f"math::ln_gamma::COEFFICIENTS[{j}]"] for j in range(9)], "G": consts["math::ln_gamma::G"],
         "LS": consts["math::LN_SQRT_2PI"], "EPS": consts["math::CONVERGENCE_EPS"], "TINY": consts["math::LENTZ_TINY"],
         "MAXIT": consts["math::MAX_ITERATIONS"], "WH": consts["math::GAMMA_WILSON_HILFERTY_SHAPE"]}
    return K, V.Prims("glibc", F)


def emit():
    K, L = setup()
    m = Module("special")

    def row(fn, args):
        val = V.SPECIAL[fn][1](K, L, *args)
        outs = list(val) if isinstance(val, tuple) else [val]
        m.row(fn, "C", [f(a) for a in args], [f(o) for o in outs])

    xs = erf_grid()
    for fn in ("math::erf", "math::erfc"):
        for x in xs:
            row(fn, [x])
    zs = normal_grid(xs)
    for fn in ("math::normal_cdf", "math::normal_sf"):
        for z in zs:
            row(fn, [z])
    for x in ln_gamma_grid():
        row("math::ln_gamma", [x])
    for a in stirling_grid():
        row("math::stirling_error", [a])
    for x, mu in bd0_pairs():
        row("math::bd0", [x, mu])
    for a, x in kernel_pairs():
        row("math::ln_gamma_kernel", [a, x])
    for a, b0 in ln_beta_pairs():
        row("math::ln_beta", [a, b0])
    for a, x in gamma_pairs():
        row("math::gamma_inc", [a, x])
    for a, x in wrapper_pairs(0x1F83D9ABFB41BD6B):
        row("math::regularized_lower_incomplete_gamma", [a, x])
    for a, x in wrapper_pairs(0x5BE0CD19137E2179):
        row("math::regularized_upper_incomplete_gamma", [a, x])
    for r in beta_rows():
        row("math::beta_inc", r)
    g = Rng(0x629A292A367CD507)
    ib = [[x, 2.0, 2.0] for x in [-1.0, -0.0, 0.0, 1.0, 2.0, NAN, INF]]
    for _ in range(400):
        x = g.unit()
        a = g.log_uniform(-2.0, 3.0)
        ib.append([x, a, g.log_uniform(-2.0, 3.0)])
    for r in ib:
        row("math::regularized_incomplete_beta", r)
    return m
