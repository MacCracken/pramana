"""`fmt.tsv`: serde_json 1.0.154's text for an f64 (port plan D5, ADR 007 row 7), on the specials, every power of two
and of ten with its neighbours, the integers to 1,000, short decimals, a log-uniform sweep and seeded bit patterns.
Class A: the text, byte for byte."""
from .grids import Rng, pow2, specials
from .models import V
from .tsv import Module, bits, f, from_bits, s


def inputs():
    out = specials()

    def near(x):
        b = bits(x)
        out.extend([from_bits(b - 1), x, from_bits(b + 1)])

    for k in range(-1074, 1024):
        x = pow2(k)
        if k == -1074:
            out.extend([x, from_bits(2)])
        else:
            near(x)
    for k in range(-323, 309):
        near(float(f"1e{k}"))
    out.extend(float(k) for k in range(1001))
    out.extend(float(f"{k}e-3") for k in range(1, 5001))
    rng = Rng(0x9E3779B97F4A7C15)
    out.extend(rng.log_uniform(-8.0, 18.0) for _ in range(5000))
    pats = []
    while len(pats) < 5000:
        lo = rng.int(0, 0xFFFFFFFF)
        hi = rng.int(0, 0xFFFFFFFF)
        x = from_bits((hi << 32) | lo)
        if x == x and x not in (float("inf"), float("-inf")):
            pats.append(x)
    out.extend(-x for x in pats[:1000])
    out.extend(pats)
    return out


def emit():
    m = Module("fmt")
    for x in inputs():
        m.row("f64::to_json", "A", [f(x)], [s(V.json_f64_text(x))])
    return m
