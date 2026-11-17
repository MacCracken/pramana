"""`debug.tsv`: Rust's derived Debug, `{:?}` and `{:#?}` (D12; M1 part f, M3), of SimpleRng, PramanaError's variants
with an empty message, the distributions at their parameter grids, MultivariateNormal's codec cases (nested lists),
and `debug::f64`, an f64 field's form on fmt.tsv's 25,200 doubles. The last input is 0 for `{:?}` and 1 for `{:#?}`.
validate.py's debug_text models the derive. Class A: the text."""
from .grids import MAXF, MIN_POSITIVE, Rng
from .models import V
from .mvn_cases import mvn_cases, mvn_toks
from .tsv import Module, e, f, from_bits, i, parsed, s, v
from . import m_fmt

U64 = (1 << 64) - 1
EPS = 2.220446049250313e-16


def emit():
    m = Module("debug")

    def both(fn, ins):
        for alt in (0, 1):
            toks = ins + [i(alt)]
            m.row(fn, "A", toks, [s(V.debug_text(fn, parsed(toks)))])

    g = Rng(0x1F83D9ABFB41BD6B)
    seeds = [0, 1, 42, 1 << 63, U64] + [g.word() >> (k % 64) for k in range(20)]
    for seed in seeds:
        both("debug::SimpleRng", [i(seed)])
    for name in ["InvalidParameter", "InvalidSample", "ConvergenceFailure", "DimensionMismatch", "ComputationError"]:
        both("debug::PramanaError", [e("PramanaError", name)])
    normals = [(0.0, 1.0), (-0.0, 1.0), (1.5, 2.3), (-3.5, 0.25), (1e-300, 1e300), (MAXF, MIN_POSITIVE), (1e16, 1e-4),
               (9.999999999999999e15, 9.999999999999999e-5), (from_bits(1), 5e-324), (2.9802322387695313e-8, 1.0)]
    for _ in range(14):
        mu = g.signed_log_uniform(-20.0, 20.0)
        normals.append((mu, g.log_uniform(-20.0, 20.0)))
    for mu, sd in normals:
        both("debug::Normal", [f(mu), f(sd)])
    uniforms = [(0.0, 1.0), (-0.0, 1.0), (-1.0, 5.0), (-MAXF, 0.0), (1e-300, 1e300), (1e16, 1e17)]
    for _ in range(10):
        lo = g.signed_log_uniform(-20.0, 20.0)
        uniforms.append((lo, lo + g.log_uniform(-20.0, 20.0)))
    for lo, hi in [u for u in uniforms if V.d_uniform_new(*u)]:
        both("debug::Uniform", [f(lo), f(hi)])
    lambdas = [1.0, 2.5, 1e-4, 9.999999999999999e-5, MAXF, from_bits(1)] + [g.log_uniform(-20.0, 20.0) for _ in range(10)]
    for la in lambdas:
        both("debug::Exponential", [f(la)])
    ps = [0.0, -0.0, 1.0, 0.7, 1.0 - EPS / 2.0, from_bits(1)] + [g.unit() for _ in range(10)]
    for p in ps:
        both("debug::Bernoulli", [f(p)])

    def two(fixed):
        out = list(fixed)
        for _ in range(10):
            a = g.log_uniform(-20.0, 20.0)
            out.append((a, g.log_uniform(-20.0, 20.0)))
        return out

    cauchys = [(0.0, 1.0), (-0.0, 1.0), (2.0, 3.0), (MAXF, from_bits(1)), (1e-300, 1e300), (1e16, 1e-4)]
    for _ in range(10):
        x0 = g.signed_log_uniform(-20.0, 20.0)
        cauchys.append((x0, g.log_uniform(-20.0, 20.0)))
    for x0, ga in cauchys:
        both("debug::Cauchy", [f(x0), f(ga)])
    for k, la in two([(1.0, 1.0), (2.0, 3.0), (0.5, 1e-4), (MAXF, from_bits(1)), (1e-300, 1e300), (1e16, 0.1)]):
        both("debug::Weibull", [f(k), f(la)])
    for a, b in two([(1.0, 1.0), (2.0, 3.0), (0.5, 1e-4), (MAXF, from_bits(1)), (1e-300, 1e300), (1e16, 0.1)]):
        both("debug::Gamma", [f(a), f(b)])
    dfs = [1.0, 3.0, 1e-4, 9.999999999999999e-5, MAXF, from_bits(1)] + [g.log_uniform(-20.0, 20.0) for _ in range(10)]
    for df in dfs:
        both("debug::ChiSquared", [f(df)])
    for a, b in two([(2.0, 5.0), (1.0, 1.0), (0.5, 1e-4), (MAXF, from_bits(1)), (1e-300, 1e300), (1e16, 0.1)]):
        both("debug::Beta", [f(a), f(b)])
        both("debug::FDistribution", [f(a), f(b)])
    ts = [1.0, 3.0, 1e-4, 9.999999999999999e-5, MAXF, from_bits(1)] + [g.log_uniform(-20.0, 20.0) for _ in range(10)]
    for df in ts:
        both("debug::StudentT", [f(df)])
    rates = [3.5, 30.0, 1e-4, 9.999999999999999e-5, MAXF, from_bits(1)] + [g.log_uniform(-20.0, 20.0) for _ in range(10)]
    for la in rates:
        both("debug::Poisson", [f(la)])
    bins = [(10, 0.3), (1, 0.0), (U64, 1.0), (1 << 63, 1e-300), (64, 0.5), (5, 1e-4)]
    for k in range(10):
        n = (g.word() >> (k % 64)) | 1
        bins.append((n, g.unit()))
    for n, p in bins:
        both("debug::Binomial", [i(n), f(p)])
    for c in mvn_cases(g):
        both("debug::MultivariateNormal", mvn_toks(c))
    for x in m_fmt.inputs():
        m.row("debug::f64", "A", [f(x)], [s(V.rust_debug_f64(x))])
    # M4 part a2: Kernel and KernelDensity
    for k in ["Gaussian", "Epanechnikov", "Uniform", "Triangular"]:
        both("debug::Kernel", [e("Kernel", k)])
    g2 = Rng(0x5BE0CD19137E2179)
    kdes = [([1.0, 2.0, 3.0], 0.5, "Gaussian"), ([0.0], 1.0, "Epanechnikov"), ([-0.0, 1e16], 9.999999999999999e-5,
            "Uniform"), ([MAXF, from_bits(1)], 1e-300, "Triangular")]
    for k in range(12):
        kdes.append(([g2.signed_log_uniform(-20.0, 20.0) for _ in range(g2.int(1, 6))], g2.log_uniform(-10.0, 10.0),
                     ["Gaussian", "Epanechnikov", "Uniform", "Triangular"][k % 4]))
    for data, h, k in kdes:
        both("debug::KernelDensity", [v(len(data))] + [f(x) for x in data] + [f(h), e("Kernel", k)])
    # M4 part a3: PcaResult, its lists of any shape
    from .m_json import pca_toks
    g3 = Rng(0x1F83D9AB5BE0CD19)
    pcas = [([[1.0, 0.0], [0.0, 1.0]], [2.0, 1.0], [0.667, 0.333]), ([], [], []), ([[1.0], [2.0, 3.0], []], [5e-324],
            [MAXF, -0.0])]
    for k in range(8):
        p = g3.int(1, 4)
        pcas.append(([[g3.signed_log_uniform(-20.0, 20.0) for _ in range(p)] for _ in range(p)],
                     [g3.log_uniform(-10.0, 10.0) for _ in range(p)], [g3.unit() for _ in range(p)]))
    for c in pcas:
        both("debug::PcaResult", pca_toks(*c))
    return m
