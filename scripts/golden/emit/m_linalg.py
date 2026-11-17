"""`linalg.tsv`: hisab 1.4.0's `cholesky` and `cholesky_solve` (M3 part d1, ledger L4), recomputed term for term by
validate.py's lin_cholesky and lin_cholesky_solve (class A: arithmetic and the correctly rounded √). The factor on the
empty matrix, 1×1 and 2×2 edges, seeded SPD matrices to n = 64, Hilbert's to 14, Lehmer's and scaled tridiagonals;
the solve against unit, seeded, zero and NaN-bearing right sides, hand factors, and n = 0. A matrix is `v:n²` and its
entries row-major; a refusal is `E:InvalidParameter`."""
from .grids import INF, MAXF, NAN, TINY, Rng
from .models import V
from .tsv import Module, err, f, i, v


def mat(rows):
    out = [v(len(rows) * len(rows))]
    for r in rows:
        out += [f(x) for x in r]
    return out


def vec(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def flat(rows):
    return [x for r in rows for x in r]


def factor_row(m, a):
    n = len(a)
    lf = V.lin_cholesky(n, flat(a))
    ins = [i(n)] + mat(a)
    if lf is None:
        m.row("linalg::cholesky", "A", ins, [err("InvalidParameter")])
        return None
    rows = [lf[k * n:(k + 1) * n] for k in range(n)]
    m.row("linalg::cholesky", "A", ins, mat(rows))
    return rows


def solve_row(m, l, b):
    n = len(l)
    x = V.lin_cholesky_solve(n, flat(l), b)
    m.row("linalg::cholesky_solve", "A", [i(n)] + mat(l) + vec(b), vec(x))


def spd(g, n, shift):
    bm = [[g.uniform(-1.0, 1.0) for _ in range(n)] for _ in range(n)]
    a = [[0.0] * n for _ in range(n)]
    for r in range(n):
        for c in range(n):
            s = 0.0
            for k in range(n):
                s += bm[r][k] * bm[c][k]
            a[r][c] = s + (shift if r == c else 0.0)
    return a


def hilbert(n):
    return [[1.0 / float(r + c + 1) for c in range(n)] for r in range(n)]


def lehmer(n):
    return [[float(min(r, c) + 1) / float(max(r, c) + 1) for c in range(n)] for r in range(n)]


def tridiagonal(n):
    return [[2.0 if r == c else (-1.0 if abs(r - c) == 1 else 0.0) for c in range(n)] for r in range(n)]


def scaled(a, s):
    return [[x * s for x in r] for r in a]


def rights(g, n):
    bs = [[1.0 if j == k else 0.0 for j in range(n)] for k in range(min(n, 8))]
    bs += [[g.uniform(-10.0, 10.0) for _ in range(n)] for _ in range(3)]
    bs.append([0.0] * n)
    nb = [float(j % 7) - 3.0 for j in range(n)]
    nb[n // 2] = NAN
    bs.append(nb)
    return bs


def emit():
    m = Module("linalg")
    g = Rng(0x9B05688C2B3E6C1F)
    factors = []
    factor_row(m, [])
    for x in [4.0, 1.0, 2.0, 0.0, -0.0, -1.0, NAN, INF, -INF, TINY, MAXF]:
        l = factor_row(m, [[x]])
        if l is not None:
            factors.append(l)
    twos = [[[1.0, 0.0], [0.0, 1.0]], [[4.0, 2.0], [2.0, 3.0]], [[1.0, 1.0], [1.0, 1.0]], [[1.0, 2.0], [2.0, 1.0]],
            [[4.0, 99.0], [2.0, 3.0]], [[4.0, NAN], [NAN, 3.0]], [[4.0, 0.0], [INF, 3.0]], [[1e-300, 0.0], [0.0, 1e300]],
            [[-4.0, 0.0], [0.0, 3.0]]]
    for a in twos:
        l = factor_row(m, a)
        if l is not None:
            factors.append(l)
    bigs = []
    for n in [3, 4, 5, 6, 8, 10, 12, 16, 24, 32, 64]:
        bigs.append(spd(g, n, float(n)))
        bigs.append(spd(g, n, 1e-3))
    bigs += [hilbert(n) for n in range(3, 15)]
    for n in (3, 7, 16):
        bigs += [lehmer(n), tridiagonal(n), scaled(tridiagonal(n), 1e300), scaled(tridiagonal(n), 1e-300),
                 scaled(tridiagonal(n), -1.0)]
    for a in bigs:
        l = factor_row(m, a)
        if l is not None:
            factors.append(l)
    for l in factors:
        for b in rights(g, len(l)):
            solve_row(m, l, b)
    zero_diag = [[2.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 3.0]]
    junk = [[2.0, 7.0, -9.0], [1.0, 3.0, NAN], [0.5, 1.0, 3.0]]
    for l in (zero_diag, junk):
        for b in rights(g, 3):
            solve_row(m, l, b)
    solve_row(m, [], [])
    eigen_rows(m, Rng(0x6A09E667BB67AE85))
    return m


def eigen_row(m, a, tol=1e-12, it=1000):
    """hisab's `eigen_symmetric(a, tol, max_iter)` (M4 part a3, ledger L6): class C, its angle from atan, cos and sin."""
    from .tsv import from_model, parsed
    n = len(a)
    ins = [i(n)] + mat(a) + [f(tol), i(it)]
    sl, sc = V.take_slices(parsed(ins))
    m.row("linalg::eigen_symmetric", "C", ins, from_model(V.LINALG_C["linalg::eigen_symmetric"](_glibc(), sl, sc)))


_G = None


def _glibc():
    global _G
    if _G is None:
        from pathlib import Path
        _G = V.dist_prims(Path(__file__).resolve().parents[3])[0]
    return _G


def eigen_rows(m, g):
    eigen_row(m, [])
    eigen_row(m, [[3.5]])
    eigen_row(m, [[NAN]])
    eigen_row(m, [[1.0, 0.0], [0.0, 1.0]])
    eigen_row(m, [[2.0, 1.0], [1.0, 2.0]])                          # |w_pp − w_qq| < 1e-12: θ = π/4
    eigen_row(m, [[2.0, 1.0], [1.0, 2.0 + 1e-13]])
    eigen_row(m, [[2.0, 1.0], [1.0, 2.0 + 2e-12]])
    from .tsv import bits as _bits, from_bits as _fb
    for d in (1e-12, _fb(_bits(1e-12) - 1), _fb(_bits(1e-12) + 1)):   # |w_pp − w_qq| at, below and above 1e-12
        for it in (1, 1000):                                           # one sweep shows the angle; then converged
            eigen_row(m, [[d, 1.0], [1.0, 0.0]], it=it)
    eigen_row(m, [[4.0, 1.0], [1.0, 9.0]])
    eigen_row(m, [[1.0, 2.0], [2.0, 1.0]])                          # indefinite: λ = 3, −1
    eigen_row(m, [[0.0, 0.0], [0.0, 0.0]])
    eigen_row(m, [[1.0, 1e-13], [1e-13, 2.0]])                      # below tol²? no: 1e-13 > 1e-24
    eigen_row(m, [[1.0, 1e-25], [1e-25, 2.0]])                      # below tol²: no rotation
    eigen_row(m, [[-3.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 1.0]])   # equal |λ|: the stable order
    eigen_row(m, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    eigen_row(m, [[2.0, -1.0, 0.0], [-1.0, 2.0, -1.0], [0.0, -1.0, 2.0]])
    eigen_row(m, [[1e300, 1e299], [1e299, 1e300]])
    eigen_row(m, [[1e-300, 1e-301], [1e-301, 2e-300]])
    eigen_row(m, [[TINY, TINY], [TINY, TINY]])
    eigen_row(m, [[1.0, NAN], [NAN, 1.0]])
    eigen_row(m, [[1.0, 2.0], [3.0, 4.0]])                          # not symmetric: every entry read
    eigen_row(m, [[4.0, 1.0], [1.0, 9.0]], tol=0.0, it=50)          # tol 0: every sweep rotates
    for it in (0, 1, 2, 3):
        eigen_row(m, [[4.0, 1.0, 0.5], [1.0, 9.0, 2.0], [0.5, 2.0, 1.0]], it=it)
    eigen_row(m, [[4.0, 1.0], [1.0, 9.0]], tol=1e-3)
    eigen_row(m, [[4.0, 1.0], [1.0, 9.0]], tol=INF)
    for n in (2, 3, 4, 5, 6, 8, 10):
        for k in range(8):
            a = spd(g, n, float(n))
            eigen_row(m, a)
        for k in range(4):
            b = [[g.uniform(-1.0, 1.0) for _ in range(n)] for _ in range(n)]
            sym = [[b[r][c] + b[c][r] for c in range(n)] for r in range(n)]
            eigen_row(m, sym)
    for n in (2, 3, 4):
        eigen_row(m, hilbert(n))
        eigen_row(m, lehmer(n))

