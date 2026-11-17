"""Shared input grids (pravash's, unchanged in behaviour): only exact or correctly rounded operations, so every input is
the same on any machine. Log-uniform values are a binary exponent times a mantissa, without a transcendental."""
import math

from .tsv import from_bits

MASK = 0xFFFFFFFFFFFFFFFF
NAN = float("nan")
INF = float("inf")
MAXF = 1.7976931348623157e308
MIN_POSITIVE = 2.2250738585072014e-308
TINY = 5e-324
LOG2_10 = 3.321928094887362


def specials():
    return [0.0, -0.0, 1.0, -1.0, MIN_POSITIVE, TINY, MAXF, INF, -INF, NAN]


def pow2(k):
    assert -1074 <= k <= 1023, k
    if k >= -1022:
        return from_bits((k + 1023) << 52)
    return from_bits(1 << (k + 1074))


class Rng:
    """xorshift64* with fixed seeds."""

    def __init__(self, seed):
        self.x = max(seed & MASK, 1)

    def next_u64(self):
        x = self.x
        x ^= x >> 12
        x ^= (x << 25) & MASK
        x ^= x >> 27
        self.x = x
        return (x * 0x2545F4914F6CDD1D) & MASK

    def word(self):
        return self.next_u64()

    def unit(self):
        return float(self.next_u64() >> 11) * (1.0 / 9007199254740992.0)

    def uniform(self, lo, hi):
        return lo + (hi - lo) * self.unit()

    def int(self, lo, hi):
        span = (hi - lo + 1) & MASK
        return lo + self.next_u64() % span

    def log_uniform(self, lo_exp10, hi_exp10):
        lo = math.floor(lo_exp10 * LOG2_10)
        hi = math.floor(hi_exp10 * LOG2_10) - 1
        k = self.int(max(lo, -1074), min(hi, 1023))
        mant = 1.0 + self.unit()
        return mant * pow2(k)

    def signed_log_uniform(self, lo_exp10, hi_exp10):
        x = self.log_uniform(lo_exp10, hi_exp10)
        return -x if self.next_u64() & 1 else x
