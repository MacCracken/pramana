"""`rng.tsv`: 1.3.0's SimpleRng (M1 part e1): the first 256 draws from 40 seeds, and 10^6-draw FNV-1a digests from six."""
from .grids import Rng
from .models import V
from .tsv import Module, f, i, v

DRAWS = 256
LONG = 1_000_000


def emit():
    m = Module("rng")
    g = Rng(0x3C6EF372FE94F82B)
    seeds = [0, 1, 2, 42, 12345, 1 << 63, (1 << 64) - 1, 0x9E3779B97F4A7C15, 88172645463325252]
    seeds += [g.word() >> (k % 8) for k in range(31)]
    for seed in seeds:
        out = [v(DRAWS)] + [f(V.from_bits(b)) for b in V.xorshift_draws(seed or 1, DRAWS)]
        m.row("SimpleRng::next_f64", "A", [i(seed), i(DRAWS)], out)
    for seed in [0, 42, 1 << 63, (1 << 64) - 1, 0x9E3779B97F4A7C15, 88172645463325252]:
        h = 0xCBF29CE484222325
        for b in V.xorshift_draws(seed or 1, LONG):
            h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        m.row("SimpleRng::digest", "A", [i(seed), i(LONG)], [i(h)])
    return m
