"""Correctly rounded binary64 results from the system MPFR library (port plan §3.5, ADR 009).

Each function returns the IEEE-754 binary64 round-to-nearest-even value of the exact mathematical result: MPFR at
53 bits of precision with binary64's exponent range (emin = -1073, emax = 1024) and `mpfr_subnormalize`, so subnormal
results, overflow to infinity and exact ties round exactly as IEEE round-to-nearest does. A decimal reference at fixed
precision can't do that: it misjudges exact ties.

The binding core is kimiya's `scripts/oracle/mpfr_ref.py` (its ADR 0006), unchanged; kimiya's unbounded-exponent and
geometry helpers are left out. M2 adds the special functions (erf, erfc, lgamma, the incomplete gamma) beside it.

No Python package is needed: MPFR (libmpfr.so.6, MPFR 4.x) is called through ctypes. Run
`python3 -I -B scripts/golden/mpfr_ref.py` for the self-checks.
"""

import ctypes
import ctypes.util
import math
import struct

_lib = ctypes.util.find_library("mpfr") or "libmpfr.so.6"
_m = ctypes.CDLL(_lib)

# mpfr_t on LP64: _mpfr_prec (long), _mpfr_sign (int, padded), _mpfr_exp (long), _mpfr_d (limb *).
_MPFR_T = ctypes.c_byte * 32
_RNDN = 0
_P = ctypes.c_void_p

_m.mpfr_get_version.restype = ctypes.c_char_p
_m.mpfr_set_emin.argtypes = [ctypes.c_long]
_m.mpfr_set_emax.argtypes = [ctypes.c_long]
_m.mpfr_init2.argtypes = [_P, ctypes.c_long]
_m.mpfr_set_d.argtypes = [_P, ctypes.c_double, ctypes.c_int]
_m.mpfr_get_d.argtypes = [_P, ctypes.c_int]
_m.mpfr_get_d.restype = ctypes.c_double
_m.mpfr_subnormalize.argtypes = [_P, ctypes.c_int, ctypes.c_int]

_UNARY = ("exp", "expm1", "exp2", "exp10", "log", "log2", "log10", "log1p",
          "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "cbrt", "sqrt",
          "erf", "erfc")   # erf and erfc: M2 part b1's reference rows
_BINARY = ("pow", "atan2", "hypot")
for _name in _UNARY:
    getattr(_m, "mpfr_" + _name).argtypes = [_P, _P, ctypes.c_int]
for _name in _BINARY:
    getattr(_m, "mpfr_" + _name).argtypes = [_P, _P, _P, ctypes.c_int]

_m.mpfr_set_emin(-1073)
_m.mpfr_set_emax(1024)
_a = _MPFR_T()
_b = _MPFR_T()
_r = _MPFR_T()
for _v in (_a, _b, _r):
    _m.mpfr_init2(ctypes.byref(_v), 53)

VERSION = _m.mpfr_get_version().decode()


def _finish(t):
    t = _m.mpfr_subnormalize(ctypes.byref(_r), t, _RNDN)
    return _m.mpfr_get_d(ctypes.byref(_r), _RNDN)


def cr(name, x):
    """The correctly rounded binary64 value of the unary MPFR function `name` at x (a float)."""
    if name not in _UNARY:
        raise ValueError(name)
    _m.mpfr_set_d(ctypes.byref(_a), x, _RNDN)
    t = getattr(_m, "mpfr_" + name)(ctypes.byref(_r), ctypes.byref(_a), _RNDN)
    return _finish(t)


def cr2(name, x, y):
    """The correctly rounded binary64 value of the binary MPFR function `name` (pow, atan2, hypot)."""
    if name not in _BINARY:
        raise ValueError(name)
    _m.mpfr_set_d(ctypes.byref(_a), x, _RNDN)
    _m.mpfr_set_d(ctypes.byref(_b), y, _RNDN)
    t = getattr(_m, "mpfr_" + name)(ctypes.byref(_r), ctypes.byref(_a), ctypes.byref(_b), _RNDN)
    return _finish(t)


# Φ and 1 − Φ (M2 part b2's reference rows): ½·erfc(∓z/√2) at 1,024 bits over MPFR's widest exponent range (erfc
# reaches 2^-1140 before the halving), then rounded once to binary64 by mpfr_get_d, subnormals included. Computing
# erfc at 53 bits on a rounded −z/√2 is exactly the error T1 removes, so this never rounds the argument.
_m.mpfr_get_emin_min.restype = ctypes.c_long
_m.mpfr_get_emax_max.restype = ctypes.c_long
_m.mpfr_sqrt_ui.argtypes = [_P, ctypes.c_ulong, ctypes.c_int]
_m.mpfr_div.argtypes = [_P, _P, _P, ctypes.c_int]
_m.mpfr_neg.argtypes = [_P, _P, ctypes.c_int]
_m.mpfr_div_2ui.argtypes = [_P, _P, ctypes.c_ulong, ctypes.c_int]
_hz = _MPFR_T()
_hs = _MPFR_T()
_ht = _MPFR_T()
for _v in (_hz, _hs, _ht):
    _m.mpfr_init2(ctypes.byref(_v), 1024)


def _phi(z, upper):
    _m.mpfr_set_emin(_m.mpfr_get_emin_min())
    _m.mpfr_set_emax(_m.mpfr_get_emax_max())
    try:
        _m.mpfr_set_d(ctypes.byref(_hz), z, _RNDN)
        _m.mpfr_sqrt_ui(ctypes.byref(_hs), 2, _RNDN)
        _m.mpfr_div(ctypes.byref(_ht), ctypes.byref(_hz), ctypes.byref(_hs), _RNDN)
        if not upper:
            _m.mpfr_neg(ctypes.byref(_ht), ctypes.byref(_ht), _RNDN)
        _m.mpfr_erfc(ctypes.byref(_ht), ctypes.byref(_ht), _RNDN)
        _m.mpfr_div_2ui(ctypes.byref(_ht), ctypes.byref(_ht), 1, _RNDN)
        return _m.mpfr_get_d(ctypes.byref(_ht), _RNDN)
    finally:
        _m.mpfr_set_emin(-1073)
        _m.mpfr_set_emax(1024)


def ncdf(z):
    """Φ(z) = ½·erfc(−z/√2), correctly rounded to binary64 (z a float)."""
    return _phi(z, False)


def nsf(z):
    """1 − Φ(z) = ½·erfc(z/√2), correctly rounded to binary64 (z a float)."""
    return _phi(z, True)


# ln|Γ(x)| (M2 part b3's reference rows): mpfr_lgamma at 53 bits, binary64's exponent range, subnormalized, the sign
# of Γ discarded. +∞ at 0, at the negative integers and at ±∞.
_m.mpfr_lgamma.argtypes = [_P, ctypes.POINTER(ctypes.c_int), _P, ctypes.c_int]


def lgamma(x):
    """ln|Γ(x)|, correctly rounded to binary64 (x a float)."""
    sign = ctypes.c_int(0)
    _m.mpfr_set_d(ctypes.byref(_a), x, _RNDN)
    t = _m.mpfr_lgamma(ctypes.byref(_r), ctypes.byref(sign), ctypes.byref(_a), _RNDN)
    return _finish(t)


# ln x as a double-double (M2 part b3: validate.py's model of the T1 lnΓ): h = RN(ln x), l = RN(ln x − h), from ln x
# at 1,024 bits over MPFR's widest exponent range. kimiya's ln core, which the port uses, is within about 2^-80 of ln
# x, so the model's single final rounding matches the port's except within about that of a tie.
_m.mpfr_log.argtypes = [_P, _P, ctypes.c_int]
_m.mpfr_sub_d.argtypes = [_P, _P, ctypes.c_double, ctypes.c_int]


def ln_dd(x):
    """(RN(ln x), RN(ln x − RN(ln x))) for a positive finite x."""
    _m.mpfr_set_emin(_m.mpfr_get_emin_min())
    _m.mpfr_set_emax(_m.mpfr_get_emax_max())
    try:
        _m.mpfr_set_d(ctypes.byref(_hz), x, _RNDN)
        _m.mpfr_log(ctypes.byref(_ht), ctypes.byref(_hz), _RNDN)
        h = _m.mpfr_get_d(ctypes.byref(_ht), _RNDN)
        _m.mpfr_sub_d(ctypes.byref(_ht), ctypes.byref(_ht), h, _RNDN)
        return h, _m.mpfr_get_d(ctypes.byref(_ht), _RNDN)
    finally:
        _m.mpfr_set_emin(-1073)
        _m.mpfr_set_emax(1024)


# e^(h + l)·2^-e as a double-double (M2 part b4: validate.py's model of the T1 incomplete gamma): (RN(v), RN(v − RN(v)))
# from v at 1,024 bits over MPFR's widest exponent range. kimiya's exp core, which the port uses with the same e, is
# within 2^-70 of v, so the model's single final rounding matches the port's except within about that of a tie.
_m.mpfr_add_d.argtypes = [_P, _P, ctypes.c_double, ctypes.c_int]
_m.mpfr_mul_2si.argtypes = [_P, _P, ctypes.c_long, ctypes.c_int]


def exp_dd(h, l, e):
    """(RN(v), RN(v − RN(v))) for v = e^(h + l)·2^-e (floats h, l; an integer e)."""
    _m.mpfr_set_emin(_m.mpfr_get_emin_min())
    _m.mpfr_set_emax(_m.mpfr_get_emax_max())
    try:
        _m.mpfr_set_d(ctypes.byref(_hz), h, _RNDN)
        _m.mpfr_add_d(ctypes.byref(_hz), ctypes.byref(_hz), l, _RNDN)
        _m.mpfr_exp(ctypes.byref(_ht), ctypes.byref(_hz), _RNDN)
        _m.mpfr_mul_2si(ctypes.byref(_ht), ctypes.byref(_ht), -e, _RNDN)
        hi = _m.mpfr_get_d(ctypes.byref(_ht), _RNDN)
        _m.mpfr_sub_d(ctypes.byref(_ht), ctypes.byref(_ht), hi, _RNDN)
        return hi, _m.mpfr_get_d(ctypes.byref(_ht), _RNDN)
    finally:
        _m.mpfr_set_emin(-1073)
        _m.mpfr_set_emax(1024)


def inv_ln2(n):
    """RN(n/ln 2) for an integer n (kimiya's exp core takes K = RN(x·512/ln 2))."""
    _m.mpfr_set_d(ctypes.byref(_hz), 2.0, _RNDN)
    _m.mpfr_log(ctypes.byref(_ht), ctypes.byref(_hz), _RNDN)
    _m.mpfr_ui_div(ctypes.byref(_ht), n, ctypes.byref(_ht), _RNDN)
    return _m.mpfr_get_d(ctypes.byref(_ht), _RNDN)


# Euler's γ and ζ(k), k = 2 … 64, as double-doubles (RN(v), RN(v − RN(v))) from 320 bits, with ζ(k) − 1 taken before
# the split: the T1 incomplete gamma's lnΓ series (M2 part b4). src/zeta_data.cyr holds the same values, which
# scripts/gen_zeta.py writes the same way.
_m.mpfr_const_euler.argtypes = [_P, ctypes.c_int]
_m.mpfr_zeta_ui.argtypes = [_P, ctypes.c_ulong, ctypes.c_int]


def zeta_table():
    """(γ, ζ, ζ − 1): γ as (hi, lo), and ζ(k) and ζ(k) − 1 as lists of (hi, lo) indexed by k, k = 0 and 1 unused."""
    v, r = _MPFR_T(), _MPFR_T()
    for x in (v, r):
        _m.mpfr_init2(ctypes.byref(x), 320)

    def split():
        hi = _m.mpfr_get_d(ctypes.byref(v), _RNDN)
        _m.mpfr_sub_d(ctypes.byref(r), ctypes.byref(v), hi, _RNDN)
        return hi, _m.mpfr_get_d(ctypes.byref(r), _RNDN)

    _m.mpfr_const_euler(ctypes.byref(v), _RNDN)
    euler = split()
    zeta, zm1 = [(0.0, 0.0)] * 2, [(0.0, 0.0)] * 2
    for k in range(2, 65):
        _m.mpfr_zeta_ui(ctypes.byref(v), k, _RNDN)
        zeta.append(split())
        _m.mpfr_sub_ui(ctypes.byref(v), ctypes.byref(v), 1, _RNDN)
        zm1.append(split())
    for x in (v, r):
        _m.mpfr_clear(ctypes.byref(x))
    return euler, zeta, zm1


# P(a, x) and Q(a, x) (M2 part b4's reference rows): scripts/crverify/check.c's `gref`, through ctypes. pre =
# x^a e^-x / Γ(a) at prec = 320 bits. For x <= a or x < 1, P = pre·Σ_{n>=0} x^n/(a(a+1)…(a+n)), positive and falling
# terms stopped under 2^-(prec + 10) of the sum (tested every 16th term, as the ctypes calls are the cost; the few more
# only add accuracy), and Q = 1 − P; otherwise Q = pre·(Legendre's continued fraction by modified Lentz, stopped when
# a step changes it by under 2^-310), and P = 1 − Q. The fraction converges too slowly below x = 1 for a small shape.
# The complement is taken from the exact product (2·prec bits). It is at least about 0.3 where x <= a and about 0.2a
# where x < 1, so below a = 2^-10 prec grows by log2(1/a) + 64. Each of P and Q is then rounded once to binary64 by
# mpfr_get_d, subnormals included. mpfr_gamma_inc itself takes up to 0.8 s a call near a = 1e5 (port-parity §6.5).
for _name in ("mpfr_add_ui", "mpfr_sub_ui", "mpfr_mul_ui"):
    getattr(_m, _name).argtypes = [_P, _P, ctypes.c_ulong, ctypes.c_int]
for _name in ("mpfr_mul", "mpfr_add", "mpfr_sub"):
    getattr(_m, _name).argtypes = [_P, _P, _P, ctypes.c_int]
_m.mpfr_ui_div.argtypes = [_P, ctypes.c_ulong, _P, ctypes.c_int]
_m.mpfr_ui_sub.argtypes = [_P, ctypes.c_ulong, _P, ctypes.c_int]
_m.mpfr_mul_2si.argtypes = [_P, _P, ctypes.c_long, ctypes.c_int]
_m.mpfr_set_ui_2exp.argtypes = [_P, ctypes.c_ulong, ctypes.c_long, ctypes.c_int]
_m.mpfr_set.argtypes = [_P, _P, ctypes.c_int]
_m.mpfr_abs.argtypes = [_P, _P, ctypes.c_int]
_m.mpfr_exp.argtypes = [_P, _P, ctypes.c_int]
_m.mpfr_cmp.argtypes = [_P, _P]
_m.mpfr_cmp_d.argtypes = [_P, ctypes.c_double]
_m.mpfr_clear.argtypes = [_P]


def gamma_pq(a, x):
    """(P(a, x), Q(a, x)), each correctly rounded to binary64, for finite 0 < a and 0 < x (floats)."""
    prec = 320
    if a < 2.0 ** -10:
        prec += 64 + math.ceil(-math.log2(a))
    regs = [_MPFR_T() for _ in range(15)]
    for r in regs[:14]:
        _m.mpfr_init2(ctypes.byref(r), prec)
    _m.mpfr_init2(ctypes.byref(regs[14]), 2 * prec)
    A, X, pre, t, s, term, h, b, c, d, an, dl, tiny, lim, w = (ctypes.byref(r) for r in regs)
    _m.mpfr_set_emin(_m.mpfr_get_emin_min())
    _m.mpfr_set_emax(_m.mpfr_get_emax_max())
    try:
        sign = ctypes.c_int(0)
        _m.mpfr_set_d(A, a, _RNDN)
        _m.mpfr_set_d(X, x, _RNDN)
        _m.mpfr_log(t, X, _RNDN)
        _m.mpfr_mul(pre, t, A, _RNDN)
        _m.mpfr_sub(pre, pre, X, _RNDN)
        _m.mpfr_lgamma(t, ctypes.byref(sign), A, _RNDN)
        _m.mpfr_sub(pre, pre, t, _RNDN)
        _m.mpfr_exp(pre, pre, _RNDN)
        if x <= a or x < 1.0:
            _m.mpfr_ui_div(term, 1, A, _RNDN)
            _m.mpfr_set(s, term, _RNDN)
            n = 1
            while True:
                _m.mpfr_add_ui(t, A, n, _RNDN)
                _m.mpfr_div(t, X, t, _RNDN)
                _m.mpfr_mul(term, term, t, _RNDN)
                _m.mpfr_add(s, s, term, _RNDN)
                if n % 16 == 0:
                    _m.mpfr_mul_2si(lim, s, -(prec + 10), _RNDN)
                    if _m.mpfr_cmp(term, lim) < 0:
                        break
                n += 1
            _m.mpfr_mul(w, s, pre, _RNDN)
            p = _m.mpfr_get_d(w, _RNDN)
            _m.mpfr_ui_sub(w, 1, w, _RNDN)
            return p, _m.mpfr_get_d(w, _RNDN)
        _m.mpfr_set_ui_2exp(tiny, 1, -20000, _RNDN)
        _m.mpfr_add_ui(b, X, 1, _RNDN)
        _m.mpfr_sub(b, b, A, _RNDN)
        _m.mpfr_set_ui_2exp(c, 1, 20000, _RNDN)
        _m.mpfr_ui_div(d, 1, b, _RNDN)
        _m.mpfr_set(h, d, _RNDN)
        i = 1
        while True:
            _m.mpfr_ui_sub(an, i, A, _RNDN)
            _m.mpfr_mul_ui(an, an, i, _RNDN)
            _m.mpfr_neg(an, an, _RNDN)
            _m.mpfr_add_ui(b, b, 2, _RNDN)
            _m.mpfr_mul(d, an, d, _RNDN)
            _m.mpfr_add(d, d, b, _RNDN)
            _m.mpfr_abs(t, d, _RNDN)
            if _m.mpfr_cmp(t, tiny) < 0:
                _m.mpfr_set(d, tiny, _RNDN)
            _m.mpfr_div(t, an, c, _RNDN)
            _m.mpfr_add(c, b, t, _RNDN)
            _m.mpfr_abs(t, c, _RNDN)
            if _m.mpfr_cmp(t, tiny) < 0:
                _m.mpfr_set(c, tiny, _RNDN)
            _m.mpfr_ui_div(d, 1, d, _RNDN)
            _m.mpfr_mul(dl, d, c, _RNDN)
            _m.mpfr_mul(h, h, dl, _RNDN)
            _m.mpfr_sub_ui(t, dl, 1, _RNDN)
            _m.mpfr_abs(t, t, _RNDN)
            if _m.mpfr_cmp_d(t, 2.0 ** -310) < 0:
                break
            i += 1
        _m.mpfr_mul(w, h, pre, _RNDN)
        q = _m.mpfr_get_d(w, _RNDN)
        _m.mpfr_ui_sub(w, 1, w, _RNDN)
        return _m.mpfr_get_d(w, _RNDN), q
    finally:
        _m.mpfr_set_emin(-1073)
        _m.mpfr_set_emax(1024)
        for r in regs:
            _m.mpfr_clear(ctypes.byref(r))


# I_x(a, b) and 1 − I_x(a, b) (M2 part b5's reference rows): scripts/crverify/check.c's `bref`, through ctypes.
# MPFR has no incomplete beta. The smaller of x and y is exact and the other is 1 − it, both held exactly (1,200 bits).
# On the side x < (a + 1)/(a + b + 2), V = x^a y^b/(a B(a, b)) times Lentz's continued fraction (Rust's even and odd
# steps) at prec bits, stopped when a step changes it by under 2^-(prec - 10); else the same for I_y(b, a). The other
# value is 1 − V, exact at 2·prec + 64 bits. P9 ran at 320 bits, which keeps ~270 bits of the complement on all its
# rows but none for a tiny shape, whose complement can be 2^-1000: when it is below 2^-(prec - 330), prec grows by its
# binary exponent plus 64 and V is recomputed. A complement still below 2^-1200 at 1,600 bits rounds to 0. Each value
# is then rounded once to binary64 by mpfr_get_d, subnormals included.
_m.mpfr_get_exp.argtypes = [_P]
_m.mpfr_get_exp.restype = ctypes.c_long
_m.mpfr_zero_p.argtypes = [_P]
_m.mpfr_set_ui.argtypes = [_P, ctypes.c_ulong, ctypes.c_int]
_m.mpfr_init2.argtypes = [_P, ctypes.c_long]


def _beta_cf(V, A, B, X, Y, prec):
    """V = x^a y^b/(a B(a, b)) times the continued fraction, at prec bits (A, B, X, Y: exact mpfr refs)."""
    regs = [_MPFR_T() for _ in range(11)]
    for r in regs:
        _m.mpfr_init2(ctypes.byref(r), prec)
    t, u, pre, c, d, h, num, dl, tiny, stop, w = (ctypes.byref(r) for r in regs)
    try:
        sign = ctypes.c_int(0)
        _m.mpfr_set_ui_2exp(tiny, 1, -20000, _RNDN)
        _m.mpfr_set_ui_2exp(stop, 1, -(prec - 10), _RNDN)
        _m.mpfr_log(t, X, _RNDN)
        _m.mpfr_mul(pre, t, A, _RNDN)
        _m.mpfr_log(t, Y, _RNDN)
        _m.mpfr_mul(t, t, B, _RNDN)
        _m.mpfr_add(pre, pre, t, _RNDN)
        _m.mpfr_lgamma(t, ctypes.byref(sign), A, _RNDN)
        _m.mpfr_sub(pre, pre, t, _RNDN)
        _m.mpfr_lgamma(t, ctypes.byref(sign), B, _RNDN)
        _m.mpfr_sub(pre, pre, t, _RNDN)
        _m.mpfr_add(u, A, B, _RNDN)
        _m.mpfr_lgamma(t, ctypes.byref(sign), u, _RNDN)
        _m.mpfr_add(pre, pre, t, _RNDN)
        _m.mpfr_log(t, A, _RNDN)
        _m.mpfr_sub(pre, pre, t, _RNDN)
        _m.mpfr_exp(pre, pre, _RNDN)
        _m.mpfr_set_ui(c, 1, _RNDN)
        _m.mpfr_ui_sub(t, 1, B, _RNDN)
        _m.mpfr_mul(t, t, X, _RNDN)
        _m.mpfr_add_ui(u, A, 1, _RNDN)
        _m.mpfr_mul(u, u, Y, _RNDN)
        _m.mpfr_add(d, t, u, _RNDN)
        _m.mpfr_add_ui(u, A, 1, _RNDN)
        _m.mpfr_div(d, d, u, _RNDN)
        _m.mpfr_ui_div(d, 1, d, _RNDN)
        _m.mpfr_set(h, d, _RNDN)
        m = 1
        while True:
            for odd in (0, 1):
                if not odd:  # m(b − m)x / ((a + 2m − 1)(a + 2m))
                    _m.mpfr_sub_ui(num, B, m, _RNDN)
                    _m.mpfr_mul_ui(num, num, m, _RNDN)
                    _m.mpfr_mul(num, num, X, _RNDN)
                    _m.mpfr_add_ui(t, A, 2 * m - 1, _RNDN)
                    _m.mpfr_add_ui(u, A, 2 * m, _RNDN)
                else:        # −(a + m)(a + b + m)x / ((a + 2m)(a + 2m + 1))
                    _m.mpfr_add_ui(num, A, m, _RNDN)
                    _m.mpfr_add(t, A, B, _RNDN)
                    _m.mpfr_add_ui(t, t, m, _RNDN)
                    _m.mpfr_mul(num, num, t, _RNDN)
                    _m.mpfr_mul(num, num, X, _RNDN)
                    _m.mpfr_neg(num, num, _RNDN)
                    _m.mpfr_add_ui(t, A, 2 * m, _RNDN)
                    _m.mpfr_add_ui(u, A, 2 * m + 1, _RNDN)
                _m.mpfr_mul(t, t, u, _RNDN)
                _m.mpfr_div(num, num, t, _RNDN)
                _m.mpfr_mul(d, num, d, _RNDN)
                _m.mpfr_add_ui(d, d, 1, _RNDN)
                _m.mpfr_abs(t, d, _RNDN)
                if _m.mpfr_cmp(t, tiny) < 0:
                    _m.mpfr_set(d, tiny, _RNDN)
                _m.mpfr_div(t, num, c, _RNDN)
                _m.mpfr_add_ui(c, t, 1, _RNDN)
                _m.mpfr_abs(t, c, _RNDN)
                if _m.mpfr_cmp(t, tiny) < 0:
                    _m.mpfr_set(c, tiny, _RNDN)
                _m.mpfr_ui_div(d, 1, d, _RNDN)
                _m.mpfr_mul(dl, d, c, _RNDN)
                _m.mpfr_mul(h, h, dl, _RNDN)
            _m.mpfr_sub_ui(t, dl, 1, _RNDN)
            _m.mpfr_abs(t, t, _RNDN)
            if _m.mpfr_cmp(t, stop) < 0:
                break
            m += 1
        _m.mpfr_mul(V, pre, h, _RNDN)
    finally:
        for r in regs:
            _m.mpfr_clear(ctypes.byref(r))


def beta_ic(a, b, x, y):
    """(I_x(a, b), 1 − I_x(a, b)), each correctly rounded to binary64, for finite a, b > 0 and x, y > 0 (floats); the
    smaller of x and y is taken as exact and the other as 1 − it."""
    exact = [_MPFR_T() for _ in range(6)]
    for r in exact:
        _m.mpfr_init2(ctypes.byref(r), 1200)
    A, B, X, Y, S, R = (ctypes.byref(r) for r in exact)
    _m.mpfr_set_emin(_m.mpfr_get_emin_min())
    _m.mpfr_set_emax(_m.mpfr_get_emax_max())
    try:
        _m.mpfr_set_d(A, a, _RNDN)
        _m.mpfr_set_d(B, b, _RNDN)
        if x <= y:
            _m.mpfr_set_d(X, x, _RNDN)
            _m.mpfr_ui_sub(Y, 1, X, _RNDN)
        else:
            _m.mpfr_set_d(Y, y, _RNDN)
            _m.mpfr_ui_sub(X, 1, Y, _RNDN)
        _m.mpfr_add_ui(S, A, 1, _RNDN)
        _m.mpfr_add(R, A, B, _RNDN)
        _m.mpfr_add_ui(R, R, 2, _RNDN)
        _m.mpfr_div(S, S, R, _RNDN)
        direct = _m.mpfr_cmp(X, S) < 0
        prec = 320
        while True:
            v, w = _MPFR_T(), _MPFR_T()
            _m.mpfr_init2(ctypes.byref(v), prec)
            _m.mpfr_init2(ctypes.byref(w), 2 * prec + 64)
            V, W = ctypes.byref(v), ctypes.byref(w)
            if direct:
                _beta_cf(V, A, B, X, Y, prec)
            else:
                _beta_cf(V, B, A, Y, X, prec)
            _m.mpfr_ui_sub(W, 1, V, _RNDN)
            ex = -2 * prec if _m.mpfr_zero_p(W) else _m.mpfr_get_exp(W)
            if ex >= -(prec - 330) or (prec >= 1600 and ex < -1200):
                break
            _m.mpfr_clear(V)
            _m.mpfr_clear(W)
            np_ = prec - ex + 64
            prec = np_ if np_ > prec else 2 * prec
        _m.mpfr_set_emin(-1073)
        _m.mpfr_set_emax(1024)
        res = []
        for z in (V, W):
            _m.mpfr_set(ctypes.byref(_r), z, _RNDN)
            t = _m.mpfr_check_range(ctypes.byref(_r), 0, _RNDN)
            res.append(_finish(t))
        _m.mpfr_clear(V)
        _m.mpfr_clear(W)
        return (res[0], res[1]) if direct else (res[1], res[0])
    finally:
        _m.mpfr_set_emin(-1073)
        _m.mpfr_set_emax(1024)
        for r in exact:
            _m.mpfr_clear(ctypes.byref(r))


def _self_check():
    """Known correctly rounded values, among them glibc's misrounds that port-parity §1 found."""
    import struct

    def bits(x):
        return struct.unpack("<Q", struct.pack("<d", x))[0]

    checks = [
        (cr("exp", 1.0), 0x4005BF0A8B145769),                    # e
        (cr("exp", 239.01439753131297), 0x557C57A25EE55A56),     # glibc misrounds to ...57
        (cr("expm1", 0.92043886641489558), 0x3FF82A90ABAD523F),  # glibc misrounds to ...3e
        (cr("tan", 1.4938616554658477), 0x4029F1DC8E0C52C1),     # glibc misrounds to ...c2
        (cr2("pow", 2.0, 0.5), 0x3FF6A09E667F3BCD),              # √2
        (cr("exp", -745.2), 0x0000000000000000),                 # underflows to +0
        (cr("exp", -744.0), 0x0000000000000002),                 # 2^(−1073.36) ≈ 1.55·2^−1074: two subnormal units
        (cr("log", 0.0), 0xFFF0000000000000),                    # −∞
    ]
    bad = [(hex(bits(got)), hex(want)) for got, want in checks if bits(got) != want]
    for got, want in bad:
        print(f"FAILED: got {got}, want {want}")
    print(f"MPFR {VERSION}: {len(checks) - len(bad)} of {len(checks)} self-checks pass")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(_self_check())
