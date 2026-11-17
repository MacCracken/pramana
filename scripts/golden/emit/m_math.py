"""`math.tsv`: what Rust std returns for every f64 operation 1.3.0's library code uses (port plan §5 step 5, §8.1), from
validate.py's models: the exact operations (class A), the powi expansions (B), and glibc's transcendentals, which
are 1.3.0's bits (C; validate.py writes the correctly rounded rows the port is held to, math.cr.tsv). Rust's max and
min return the second operand when the two are equal (±0 against ∓0), as the port's `_pmn_max` does."""
import math

from .grids import MIN_POSITIVE, NAN, INF, MAXF, Rng, pow2, specials
from .models import V
from .tsv import Module, bits, f, from_bits, i

PI = math.pi


def model(fn, ins):
    if fn in ("f64::max", "f64::min"):
        return V.r_maxmin(fn, ins[0], ins[1], ins[1])
    return V.expected_math(fn, ins, None)


def unary(m, name, cls, xs):
    for x in xs:
        m.row(name, cls, [f(x)], [f(model(name, [x]))])


def binary(m, name, cls, xy):
    for x, y in xy:
        m.row(name, cls, [f(x), f(y)], [f(model(name, [x, y]))])


def grid(xs):
    return [(x, y) for x in xs for y in xs]


def wide(seed, n):
    out = specials()
    for k in range(-1074, 1024, 16):
        x = pow2(k)
        out.extend([x, -x])
    r = Rng(seed)
    out.extend(r.signed_log_uniform(-300.0, 300.0) for _ in range(n))
    return out


def sweep(seed, n, lo, hi):
    r = Rng(seed)
    return [r.uniform(lo, hi) for _ in range(n)]


def log_sweep(seed, n, lo, hi):
    r = Rng(seed)
    return [r.log_uniform(lo, hi) for _ in range(n)]


def signed_log_sweep(seed, n, lo, hi):
    r = Rng(seed)
    return [r.signed_log_uniform(lo, hi) for _ in range(n)]


def around_integers(lo, hi):
    out = []
    for k in range(lo, hi + 1):
        x = float(k)
        out.extend([x, x + 0.5, from_bits(bits(x) + 1), from_bits(bits(x) - 1)])
    return out


def emit():
    m = Module("math")
    xs = wide(0x5157, 300) + log_sweep(0x5158, 300, -40.0, 20.0) + [float(k * k) for k in range(1, 101)]
    xs += sweep(0x5159, 300, 0.0, 1e4)
    unary(m, "f64::sqrt", "A", xs)
    unary(m, "f64::abs", "A", wide(0xAB5, 100))
    halves = around_integers(-20, 20) + [0.49999999999999994, -0.49999999999999994, 4503599627370495.5,
                                         -4503599627370495.5, 4503599627370497.0]
    halves += sweep(0x7A11, 300, -1e4, 1e4) + wide(0x7A12, 100)
    unary(m, "f64::floor", "A", halves)
    fr = halves + [-x for x in around_integers(-3, 3)]
    unary(m, "f64::fract", "A", fr)
    mm = grid(specials())
    r = Rng(0x3A3A)
    for _ in range(60):
        x = r.signed_log_uniform(-30.0, 30.0)
        mm += [(x, x), (x, 0.0), (0.0, x), (x, NAN), (NAN, x), (x, MIN_POSITIVE)]
    r = Rng(0x3A3B)
    for _ in range(300):
        x = r.signed_log_uniform(-300.0, 300.0)
        mm.append((x, r.signed_log_uniform(-300.0, 300.0)))
    binary(m, "f64::max", "A", mm)
    binary(m, "f64::min", "A", mm)
    r = Rng(0xC1A2)
    bounds = [(0.0, 1.0), (-1.0, 1.0), (0.0, 0.0), (-0.0, 0.0), (0.0, INF)]
    for _ in range(60):
        a = r.signed_log_uniform(-10.0, 10.0)
        c = r.signed_log_uniform(-10.0, 10.0)
        bounds.append((a, c) if a <= c else (c, a))
    for lo, hi in bounds:
        probes = [lo, hi, 0.0, -0.0, NAN, INF, -INF, r.signed_log_uniform(-10.0, 10.0)]
        for x in probes:
            m.row("f64::clamp", "A", [f(x), f(lo), f(hi)], [f(V.r_clamp(x, lo, hi))])
    preds = specials() + [from_bits(0xFFF8000000000000), from_bits(0x7FF0000000000001), -MAXF, -from_bits(1)]
    preds += signed_log_sweep(0xF1F1, 20, -300.0, 300.0)
    for x in preds:
        m.row("f64::is_finite", "A", [f(x)], [i(model("f64::is_finite", [x]))])
        m.row("f64::is_nan", "A", [f(x)], [i(model("f64::is_nan", [x]))])
    keys = specials() + [from_bits(0xFFF8000000000000), -MAXF, -from_bits(1)]
    for x, y in grid(keys):
        m.row("f64::total_cmp", "A", [f(x), f(y)], [i(model("f64::total_cmp", [x, y]))])
    casts = specials() + around_integers(-5, 5) + sweep(0xCA51, 200, -1e3, 1e6)
    casts += [9223372036854775807.0, 9223372036854777856.0, 18446744073709549568.0, 18446744073709551616.0, 1e6]
    casts += wide(0xCA52, 100)
    for x in casts:
        m.row("f64::as_usize", "A", [f(x)], [i(model("f64::as_usize", [x]))])
    U = (1 << 64) - 1
    words = [0, 1, U, U - 1]
    for k in range(64):
        p = 1 << k
        words += [p, (p - 1) & U, (p + 1) & U]
        if k >= 54:
            half = 1 << (k - 53)
            words += [p | half, p | half | 1, p | (half >> 1), p | (half << 1) | half]
    r = Rng(0x64F6)
    for _ in range(300):
        hi = int(r.unit() * 4294967296.0)
        lo = int(r.unit() * 4294967296.0)
        shift = r.int(0, 63)
        words.append(((hi << 32) | lo) >> shift)
    for n in words:
        m.row("u64::as_f64", "A", [i(n)], [f(float(n))])
    ws = wide(0x4F4F, 200) + sweep(0x4F50, 200, -6.0, 6.0)
    ws += [0.84375, -0.84375, 1.25, 6.0, 28.0, from_bits(0x3E30000000000000)]
    for x in ws:
        m.row("f64::high_word", "A", [f(x)], [i(bits(x) >> 32)])
    unary(m, "f64::low_masked", "A", ws)
    bases = sweep(0x9013, 300, -2.0, 2.0) + log_sweep(0x9014, 200, -10.0, 10.0) + specials()
    for p in (2, 3, 4):
        unary(m, f"f64::powi{p}", "B", bases)
    ys = sweep(0x9015, 400, 0.0, 1.0) + specials()
    for p in (9, 25, 49):
        unary(m, f"f64::powi{p}", "B", ys)
    ex = wide(0xE0E0, 200) + sweep(0xE0E1, 1500, -745.0, 710.0) + sweep(0xE0E2, 1000, -50.0, 5.0)
    ex += signed_log_sweep(0xE0E3, 300, -20.0, 0.0)
    unary(m, "f64::exp", "C", ex)
    ln = specials() + [-1.0, -0.0] + log_sweep(0x1E01, 1500, -308.0, 308.0) + sweep(0x1E02, 800, 0.0, 1.0)
    r = Rng(0x1E03)
    for _ in range(400):
        d = r.log_uniform(-16.0, -1.0)
        ln += [1.0 + d, 1.0 - d]
    unary(m, "f64::ln", "C", ln)
    l1 = specials() + [-1.0, -0.5, -0.9999999999999999] + log_sweep(0x11A1, 600, -20.0, 3.0)
    l1 += [-min(x, 0.999999999) for x in log_sweep(0x11A2, 400, -20.0, 0.0)]
    unary(m, "f64::ln_1p", "C", l1)
    em = specials() + signed_log_sweep(0xE1A1, 600, -20.0, 2.0) + sweep(0xE1A2, 400, -40.0, 710.0)
    unary(m, "f64::exp_m1", "C", em)
    pw = [(float(n), -0.2) for n in range(1, 2001)]
    r = Rng(0x9A01)
    for _ in range(500):
        u = max(r.unit(), MIN_POSITIVE)
        pw.append((u, 1.0 / r.log_uniform(-2.0, 0.0)))
    for _ in range(500):
        z = r.log_uniform(-3.0, 2.0)
        k = r.uniform(0.2, 10.0)
        pw += [(z, k), (z, k - 1.0)]
    for _ in range(800):
        b0 = r.log_uniform(-3.0, 3.0)
        pw.append((b0, r.uniform(-50.0, 50.0)))
    pw += grid(specials())
    binary(m, "f64::powf", "C", pw)
    tr = specials() + sweep(0x5C01, 1000, -10.0, 10.0)
    r = Rng(0x5C02)
    tr += [2.0 * PI * r.unit() for _ in range(1000)]
    tr += [PI * r.uniform(-20.0, 0.5) for _ in range(300)]
    tr += log_sweep(0x5C03, 200, 0.0, 6.0)
    unary(m, "f64::sin", "C", tr)
    unary(m, "f64::cos", "C", tr)
    tn = specials() + sweep(0x7A01, 1000, -1.57, 1.57)
    r = Rng(0x7A02)
    tn += [PI * (r.unit() - 0.5) for _ in range(1000)]
    unary(m, "f64::tan", "C", tn)
    at = specials() + signed_log_sweep(0xA7A1, 1000, -10.0, 10.0) + sweep(0xA7A2, 500, -50.0, 50.0)
    unary(m, "f64::atan", "C", at)
    l2 = [float(n) for n in range(1, 2001)] + specials() + log_sweep(0x1092, 500, -300.0, 300.0)
    unary(m, "f64::log2", "C", l2)
    cb = specials() + signed_log_sweep(0xCB01, 500, -300.0, 300.0) + sweep(0xCB02, 300, 0.5, 2.0)
    unary(m, "f64::cbrt", "C", cb)
    return m
