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
          "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "cbrt", "sqrt")
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
