"""`timeseries.tsv` (M5 part b): 1.3.0's timeseries.rs. The moving average, exponential smoothing, autocorrelation,
differencing and integration on seeded, constant, trending, overflowing and non-finite series, their window, α, lag
and order grids with every refusal; the ARIMA(p, d, 0) fit and its forecasts on AR(1)–AR(3) series with drift,
random walks, constant and short series, d = 0 … 3. All class A: arithmetic, the correctly rounded √ and hisab's
Cholesky pair. A slice is `v:n` and its values; a model is its coefficients (a slice), d, the intercept and the
residual variance."""
from pathlib import Path

from .grids import INF, MAXF, NAN, TINY, Rng
from .models import V
from .tsv import Module, f, from_model, i, parsed, v

ROOT = Path(__file__).resolve().parents[3]


def sl(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def row(m, fn, ins):
    s, sc = V.take_slices(parsed(ins))
    m.row(fn, "A", ins, from_model(V.TS[fn](None, s, sc)))


def series(g):
    out = [[1.0, 2.0, 3.0, 4.0, 5.0], [], [7.0], [2.0, 2.0, 2.0, 2.0], [1.0, -1.0] * 6, [1.0, NAN, 3.0, 4.0],
           [1.0, INF, 3.0], [MAXF, MAXF, -MAXF, 0.0], [1e300, 2e300, 3e300], [TINY, 0.0, TINY, -TINY],
           [-0.0, 0.0, -0.0, 0.0], [1e16 + k for k in range(6)]]
    for n in (5, 12, 30, 60):
        for _ in range(4):
            out.append([g.uniform(-10.0, 10.0) for _ in range(n)])
    for n in (20, 50):
        w, s = 0.0, []
        for _ in range(n):
            w += g.uniform(-1.0, 1.0)
            s.append(w)
        out.append(s)
    return out


def ar_series(g, phis, n, drift):
    z = [0.0] * len(phis)
    for _ in range(n):
        val = drift + sum(p * z[-1 - k] for k, p in enumerate(phis)) + g.uniform(-1.0, 1.0)
        z.append(val)
    return z[len(phis):]


def emit():
    m = Module("timeseries")
    g = Rng(0xBE5466CF34E90C6C)
    ss = series(g)
    for x in ss:
        for w in (0, 1, 2, 3, 5, len(x), len(x) + 1):
            row(m, "timeseries::moving_average", sl(x) + [i(w)])
        for a in (0.3, 1.0, 0.5, 1e-300, 0.0, -0.0, 1.0000000000000002, NAN, INF, 0.9999999999999999):
            row(m, "timeseries::exponential_smoothing", sl(x) + [f(a)])
        for lag in (0, 1, 2, 3, len(x) - 1, len(x)):
            if lag >= 0:
                row(m, "timeseries::autocorrelation", sl(x) + [i(lag)])
        for d in (0, 1, 2, 3, len(x)):
            row(m, "timeseries::difference", sl(x) + [i(d)])
    for x in ss[:12] + ss[12::3]:
        for d in (0, 1, 2, 3):
            dx = V.ts_diff(x, d) if d > 0 else x
            if isinstance(dx, str):
                dx = x
            for pre in (x[:d], x[:max(d - 1, 0)], x[:d + 2], []):
                row(m, "timeseries::integrate", sl(dx) + sl(pre) + [i(d)])
    # ARIMA: fits and their forecasts
    cases = [([1.0, 2.0, 3.0, 4.0, 5.0], 1, 0), ([1.0, 2.0, 3.0, 4.0, 5.0], 1, 1), ([2.0] * 10, 1, 0),
             ([1.0, 2.0], 1, 0), ([1.0, 2.0, 3.0], 2, 1), ([1.0, NAN, 3.0, 4.0, 5.0], 1, 0), ([1.0, 2.0, 3.0], 0, 0),
             ([1.0, 2.0, 3.0], 1, 3), ([float(k * k) for k in range(12)], 1, 2), ([1e300, -1e300] * 5, 1, 0),
             ([1.0, -1.0] * 8, 2, 0), ([0.0, 1e-16, 0.0, 1e-16, 0.0, 1e-16], 1, 0),
             ([0.0, 1e-12, 0.0, 1e-12, 0.0, 1e-12, 0.0, 1e-12], 1, 0),     # c(0) ≈ 2.5e-25: above the 1e-30 floor
             ([0.0, 1e-12, 3e-12, 2e-12, 0.0, 4e-12, 1e-12, 2e-12], 1, 0),
             ([0.0, 2e-15, 0.0, 2e-15, 0.0, 2e-15], 1, 0)]                 # c(0) ≈ 1e-30: the floor's edge
    for phis in ([0.6], [0.5, -0.3], [0.4, 0.2, -0.1], [-0.8], [0.95]):
        for d in (0, 1, 2):
            for drift in (0.0, 0.5):
                x = ar_series(g, phis, 80, drift)
                if d > 0:
                    acc = 0.0
                    y = []
                    for val in x:
                        acc += val
                        y.append(acc)
                    x = y
                cases.append((x, len(phis), d))
    for x, p, d in cases:
        row(m, "timeseries::arima_fit", sl(x) + [i(p), i(d)])
        r = V.ts_arima_fit(x, p, d)
        if not isinstance(r, str):
            phi, dd, icpt, rv = r
            for steps in (1, 5, 0):
                row(m, "timeseries::arima_forecast", sl(phi) + [i(dd), f(icpt), f(rv)] + sl(x) + [i(steps)])
            row(m, "timeseries::arima_forecast", sl(phi) + [i(dd), f(icpt), f(rv)] + sl(x[:dd + len(phi) - 1]) + [i(3)])
    row(m, "timeseries::arima_forecast", sl([]) + [i(0), f(2.5), f(1.0)] + sl([1.0]) + [i(3)])
    row(m, "timeseries::arima_forecast", sl([0.5]) + [i(-1), f(0.0), f(1.0)] + sl([1.0, 2.0, 3.0]) + [i(2)])
    return m
