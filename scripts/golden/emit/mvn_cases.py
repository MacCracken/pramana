"""MultivariateNormal's codec and Debug cases (M3 part d2), shared by m_json and m_debug: 1.3.0's test cases, signed
zeros, wide scales and seeded SPD matrices for d from 1 to 5, each as (mean, covariance rows)."""
from .grids import MAXF
from .tsv import f, v


def mvn_cases(g):
    cases = [
        ([2.0], [[4.0]]),
        ([1.0, 2.0], [[4.0, 1.0], [1.0, 9.0]]),
        ([1.0, 2.0], [[2.0, 0.5], [0.5, 1.0]]),
        ([0.0, -0.0], [[1.0, -0.0], [-0.0, 1.0]]),
        ([0.0] * 3, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        ([MAXF, -5e-324], [[1e300, 0.0], [0.0, 1e-300]]),
        ([0.0, 0.0], [[4.0, 1.0], [1.0000000000001, 9.0]]),
    ]
    for d in range(1, 6):
        b = [g.uniform(-1.0, 1.0) for _ in range(d * d)]
        a = []
        for r in range(d):
            row = []
            for c in range(d):
                s = -0.0
                for k in range(d):
                    s += b[r * d + k] * b[c * d + k]
                row.append(s + (float(d) if r == c else 0.0))
            a.append(row)
        mean = [g.signed_log_uniform(-3.0, 3.0) for _ in range(d)]
        cases.append((mean, a))
    return cases


def mvn_toks(case):
    mean, rows = case
    out = [v(len(mean))] + [f(x) for x in mean] + [v(len(rows))]
    for r in rows:
        out += [v(len(r))] + [f(x) for x in r]
    return out
