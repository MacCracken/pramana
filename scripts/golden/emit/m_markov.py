"""`markov.tsv` (M6 part a): 1.3.0's markov.rs. `MarkovChain::new` on seeded stochastic matrices and every refusal
(empty, not square, a negative, non-finite or off-sum row at and past the 1e-6 bound, a state out of range);
`step` and `simulate` from `SimpleRng::new(seed)` on those chains, seeds whose draw passes a row's rounded sum (the
fallback to the last reachable state); `steady_state` on regular, periodic (the lazy switch), absorbing, reducible,
already-stationary and slow chains (ConvergenceFailure). `HiddenMarkovModel::new` and its refusals in Rust's order;
the forward log-likelihood, Viterbi and Baum–Welch on seeded models with sequences up to 2,000 symbols,
zero-probability sequences, ties, unvisited states, out-of-range and empty observations. A matrix is a flat slice
and its rows and columns; observations are `i:` words. Class C: the log-likelihood and Viterbi (ln); the rest is
arithmetic."""
from pathlib import Path

from .grids import INF, NAN, Rng
from .models import V
from .tsv import Module, f, from_bits, from_model, i, parsed, v

ROOT = Path(__file__).resolve().parents[3]
_PRIMS = None
U64 = (1 << 64) - 1


# Three near-cyclic chains (a 1e-3 perturbation of a 5- or 7-cycle) whose residuals stall now and then: the stall
# count resets on every shrinking step, and a count that never reset would switch to the lazy chain. Found by a
# search over random perturbations, held here as their bits.
STALL_RESET = [
    [
        ["0x1.509062361e36dp-11", "0x1.ff82fe7a30b67p-1", "0x1.4d88987e8fd33p-30", "0x1.5df772f4529abp-16", "0x1.310b9f7ca4bcep-12"],
        ["0x1.664bf8ef97c99p-12", "0x1.7dcf7fd2978ebp-14", "0x1.ff82096c5c5a7p-1", "0x1.1e137be295425p-12", "0x1.0be1485659b5dp-12"],
        ["0x1.41b5a782e833ap-16", "0x1.68fb184bb51adp-13", "0x1.57bf5853b110ep-12", "0x1.ff8b9aa2310ebp-1", "0x1.82d2af85d06c6p-12"],
        ["0x1.4f023bf783938p-13", "0x1.62535e5322e00p-12", "0x1.0a9b1edb3590ap-13", "0x1.850716eca958ep-12", "0x1.ff7d7adbaadaep-1"],
        ["0x1.ff7dd2354aeeep-1", "0x1.85e549e1a03e8p-16", "0x1.f46d60817b9bfp-15", "0x1.ab8c9ba4b51ffp-14", "0x1.a7cf970889245p-11"],
    ],
    [
        ["0x1.0bb23651cdfefp-12", "0x1.ff9b3bf019ad0p-1", "0x1.6913ca5d6f6b7p-17", "0x1.c4c4854c36355p-13", "0x1.8cdbf31efb0d6p-18", "0x1.a4ff773afa491p-23", "0x1.265b582c5fef1p-12"],
        ["0x1.79c3b69ae9ba2p-14", "0x1.f5d7c86cd0f11p-21", "0x1.ffae7fb88f16dp-1", "0x1.349dc68db1f4dp-23", "0x1.15c580f9f3a5ap-12", "0x1.24eb8aceadd9ap-15", "0x1.e419b7df7af89p-13"],
        ["0x1.9e9fb44f5003cp-19", "0x1.f87476fa12cd0p-13", "0x1.2dab68d76bcc7p-11", "0x1.ff7e89dbfbd1cp-1", "0x1.0bd12e943f33ep-19", "0x1.8e098e7f56d01p-14", "0x1.2d23363fd091dp-14"],
        ["0x1.0beccc7770e9cp-13", "0x1.2611e58260f9bp-17", "0x1.fbedb96a02154p-14", "0x1.7df575cc0fa63p-12", "0x1.ff7ced91e6c10p-1", "0x1.626f427ca708ep-21", "0x1.8bca5f9a5e076p-12"],
        ["0x1.8a1788ad8dcf6p-16", "0x1.2cefbd69be00bp-15", "0x1.7651ef3804de3p-16", "0x1.a62fe19768774p-12", "0x1.45c1771f0950ap-12", "0x1.ff7e92948c719p-1", "0x1.93aae774e1f66p-13"],
        ["0x1.8d1d768fe0e6bp-20", "0x1.69489381ec86bp-14", "0x1.6ccfc693d6cd1p-12", "0x1.d8ff2416f20a1p-14", "0x1.06ca478d60657p-13", "0x1.4e6b57045e654p-14", "0x1.ff9d422690f34p-1"],
        ["0x1.ff7cf1642295ap-1", "0x1.e96c138afe35bp-24", "0x1.960272153044ap-13", "0x1.7347bdfb4ce11p-14", "0x1.9b153c066016ap-14", "0x1.14f1353efd938p-11", "0x1.7f6d98846ed4cp-14"],
    ],
    [
        ["0x1.c2de5060ac297p-12", "0x1.ff80e462c703ap-1", "0x1.2cc32ec57f242p-19", "0x1.afe62db9bd561p-14", "0x1.2466f7979d737p-15", "0x1.20187a3fcc731p-22", "0x1.a2d6a289b8ac8p-12"],
        ["0x1.a9ebd2ac28d1fp-14", "0x1.78eecf7bcaa8fp-15", "0x1.ffc16170279c1p-1", "0x1.6204ea1f96582p-13", "0x1.017dc41fb99bfp-20", "0x1.44a37fda2f0fap-15", "0x1.01869ab2d5fbcp-13"],
        ["0x1.ec6329479ac33p-23", "0x1.50b23a077d6b4p-15", "0x1.3530f0cb45c8dp-12", "0x1.ffa6155c172b1p-1", "0x1.732c60b1d6edcp-17", "0x1.643080912d49bp-12", "0x1.9dcfa33879d73p-26"],
        ["0x1.c655a420d8f70p-12", "0x1.3d4ba43e06c85p-12", "0x1.c0ca493b8135cp-21", "0x1.5b5113acbd20fp-14", "0x1.ff7d92246c1f1p-1", "0x1.d27a6a8987ef2p-14", "0x1.0de93e37e2555p-14"],
        ["0x1.fa2b662503442p-13", "0x1.8e470241cd0d6p-15", "0x1.e300e6255fd68p-14", "0x1.16585bb65756bp-15", "0x1.9e26e46cbfc97p-14", "0x1.ffb90507024ebp-1", "0x1.790d9ddf0e039p-18"],
        ["0x1.847c7d50706b3p-13", "0x1.a561baf2cf71cp-14", "0x1.d07f43eeff002p-13", "0x1.25180726cd621p-13", "0x1.0db4680118007p-15", "0x1.d65372050acdep-19", "0x1.ffa6874f25801p-1"],
        ["0x1.ff7ceda73c1aap-1", "0x1.62d3be4ee60dep-18", "0x1.b4fbda0b61069p-12", "0x1.3a1f6c1c8e674p-12", "0x1.9541525968875p-20", "0x1.205f42644bad1p-12", "0x1.f7ad475ad6f41p-20"],
    ],
]


def prims():
    global _PRIMS
    if _PRIMS is None:
        _PRIMS = V.dist_prims(ROOT)[0]
    return _PRIMS


def sl(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def isl(xs):
    return [v(len(xs))] + [i(x) for x in xs]


def row(m, fn, cls, ins):
    s, sc = V.take_slices(parsed(ins))
    m.row(fn, cls, ins, from_model(V.MARKOV[fn](prims(), s, sc)))


def stochastic(g, n, zeros=0.0):
    """A row of n weights normalized by their sum, each zero with probability `zeros`."""
    while True:
        w = [0.0 if g.unit() < zeros else g.unit() + 0.01 for _ in range(n)]
        t = sum(w)
        if t > 0.0:
            return [x / t for x in w]


def flat(rows):
    return [x for r in rows for x in r]


def chain_ins(rows, state):
    return sl(flat(rows)) + [i(len(rows)), i(len(rows[0]) if rows else 0), i(state)]


def seed_for_state(x):
    """The seed whose SimpleRng's first xorshift step gives the state x (the step inverted)."""
    mask = (1 << 64) - 1
    y = x
    for k in (17, 34, 51):          # x ^= x << 17
        y ^= (x << k) & mask
    x = y
    y = x
    for k in range(7, 64, 7):       # x ^= x >> 7
        y ^= x >> k
    x = y
    y = x
    for k in (13, 26, 39, 52):      # x ^= x << 13
        y ^= (x << k) & mask
    return y


def chains(g):
    out = [[[1.0]], [[0.7, 0.3], [0.4, 0.6]], [[0.0, 1.0], [1.0, 0.0]], [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
           [[1.0, 0.0], [0.5, 0.5]], [[0.5, 0.5, 0.0, 0.0], [0.5, 0.5, 0.0, 0.0], [0.0, 0.0, 0.3, 0.7], [0.0, 0.0, 0.6, 0.4]],
           [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], [[0.1] * 10 for _ in range(10)],
           [[-0.0, 1.0], [1.0, -0.0]], [[0.5, 0.25, 0.25], [0.25, 0.5, 0.25], [0.25, 0.25, 0.5]],
           [[0.0, 0.5, 0.0, 0.5], [0.5, 0.0, 0.5, 0.0], [0.0, 0.5, 0.0, 0.5], [0.5, 0.0, 0.5, 0.0]],
           [[1.0 - 1e-9, 1e-9], [2e-9, 1.0 - 2e-9]], [[1.0 - 1e-4, 1e-4], [3e-4, 1.0 - 3e-4]],
           [[0.5, 0.4999991, 0.0], [0.3, 0.3, 0.3999991], [0.0, 0.9999991, 0.0]]]
    for n in (2, 3, 4, 5, 6, 8, 10, 16):
        for z in (0.0, 0.3):
            out.append([stochastic(g, n, z) for _ in range(n)])
    return out


def hmms(g):
    out = [([[0.7, 0.3], [0.4, 0.6]], [[0.5, 0.4, 0.1], [0.1, 0.3, 0.6]], [0.6, 0.4]),
           ([[1.0]], [[1.0]], [1.0]), ([[0.5, 0.5], [0.5, 0.5]], [[0.5, 0.5], [0.5, 0.5]], [0.5, 0.5]),
           ([[0.9, 0.1, 0.0], [0.0, 0.9, 0.1], [0.0, 0.0, 1.0]], [[1.0, 0.0], [0.5, 0.5], [0.0, 1.0]], [1.0, 0.0, 0.0]),
           ([[0.0, 1.0], [1.0, 0.0]], [[1.0, 0.0], [0.0, 1.0]], [1.0, 0.0]),
           ([[0.6, 0.4, 0.0], [0.3, 0.7, 0.0], [0.5, 0.5, 0.0]], [[0.2, 0.8], [0.9, 0.1], [0.5, 0.5]], [0.5, 0.5, 0.0]),
           ([[1.0 / 3.0] * 3 for _ in range(3)], [[0.25] * 4 for _ in range(3)], [1.0 / 3.0] * 3)]
    for n, m in ((1, 3), (2, 2), (2, 5), (3, 3), (3, 6), (4, 4), (5, 2), (6, 8)):
        for z in (0.0, 0.25):
            out.append(([stochastic(g, n, z) for _ in range(n)], [stochastic(g, m, z) for _ in range(n)],
                        stochastic(g, n)))
    return out


def hmm_ins(h, obs, extra=(), dims=None):
    a, b, ip = h
    n = len(a)
    tr, tc, er, ec = dims if dims is not None else (n, len(a[0]) if a else 0, len(b), len(b[0]) if b else 0)
    return sl(flat(a)) + sl(flat(b)) + sl(ip) + isl(obs) + [i(tr), i(tc), i(er), i(ec)] + [i(x) for x in extra]


def sample_obs(g, h, t):
    """A sequence the model emits, drawn with the generator's own stream."""
    a, b, ip = h

    def pick(row):
        r, cum = g.unit(), 0.0
        for j, p in enumerate(row):
            cum += p
            if r < cum:
                return j
        return max(j for j, p in enumerate(row) if p > 0.0)

    s, obs = pick(ip), []
    for _ in range(t):
        obs.append(pick(b[s]))
        s = pick(a[s])
    return obs


def emit():
    m = Module("markov")
    g = Rng(0x452821E638D01377)
    cs = chains(g)
    # MarkovChain::new: the chains, then the refusals
    for c in cs:
        for st in (0, len(c) - 1, len(c), U64, 1 << 63):
            row(m, "MarkovChain::new", "A", chain_ins(c, st))
    one = 1.0 + 1e-6
    for x in (one, from_bits(V.bits(one) + 1), from_bits(V.bits(one) + 2), from_bits(V.bits(one) - 1), 1.0 - 1e-6,
              from_bits(V.bits(1.0 - 1e-6) - 1), from_bits(V.bits(1.0 - 1e-6) + 1), 1.0 - 1e-6 - 2e-16, 0.0, -0.0, -1.0,
              NAN, INF, -INF, 1.0000000000000002):
        row(m, "MarkovChain::new", "A", chain_ins([[x]], 0))
        row(m, "MarkovChain::new", "A", chain_ins([[0.5, 0.5], [x - 0.5, 0.5]], 1))
    for bad in ([[0.5, 0.5, 0.0], [0.5, 0.5, 0.0]], [[0.5, 0.5]], [[1.0], [1.0]], [[1.3, -0.3], [0.4, 0.6]],
                [[0.5, 0.3], [0.4, 0.6]], [[0.5, NAN], [0.4, 0.6]], [[INF, 0.0], [0.4, 0.6]], [[0.6, 0.4], [0.5, 0.6]]):
        row(m, "MarkovChain::new", "A", chain_ins(bad, 0))
    row(m, "MarkovChain::new", "A", sl([]) + [i(0), i(0), i(0)])
    row(m, "MarkovChain::new", "A", sl([]) + [i(2), i(0), i(0)])
    # step and simulate
    seeds = [0, 1, 42, U64, 1 << 63] + [g.word() for _ in range(3)]
    for c in cs:
        for seed in seeds[:5] if len(c) > 4 else seeds:
            for k in (0, 1, 7, 100):
                row(m, "MarkovChain::step", "A", chain_ins(c, 0) + [i(seed), i(k)])
            row(m, "MarkovChain::simulate", "A", chain_ins(c, len(c) - 1) + [i(seed), i(25)])
    walk = [stochastic(g, 6, 0.2) for _ in range(6)]
    for seed in (7, 99):
        row(m, "MarkovChain::simulate", "A", chain_ins(walk, 2) + [i(seed), i(2000)])
    # draws exactly on a cumulative sum (0, 1/4, 1/2, 3/4): r < Σ is strict
    exact = [[[0.5, 0.5], [0.25, 0.75]], [[0.25, 0.5, 0.25], [0.0, 0.75, 0.25], [0.5, 0.0, 0.5]], [[0.0, 1.0], [1.0, 0.0]],
             [[0.0, 0.0, 1.0], [0.0, 0.25, 0.75], [0.75, 0.25, 0.0]]]
    for x in (1, 0x7FF, 0x4000000000000000, 0x8000000000000000, 0xC000000000000000, 0x80000000000007FF):
        seed = seed_for_state(x)
        assert next(V.d_uniforms(seed)) == (x >> 11) / 9007199254740992.0
        for c in exact:
            for st in range(len(c)):
                row(m, "MarkovChain::step", "A", chain_ins(c, st) + [i(seed), i(1)])
    # draws past a row's sum (0.9999991, inside the 1e-6 bound): the last state with P > 0
    edge = [[0.5, 0.4999991, 0.0], [0.3, 0.3, 0.3999991], [0.0, 0.9999991, 0.0]]
    for x in (0xFFFFFFFFFFFFF800, 0xFFFFF0F0F0F0F0F0, 0xFFFFF10000000000, 0xFFFFF0E000000000):
        seed = seed_for_state(x)
        assert next(V.d_uniforms(seed)) == (x >> 11) / 9007199254740992.0
        for st in (0, 1, 2):
            row(m, "MarkovChain::step", "A", chain_ins(edge, st) + [i(seed), i(1)])
            row(m, "MarkovChain::simulate", "A", chain_ins(edge, st) + [i(seed), i(3)])
    # steady_state
    for c in cs:
        row(m, "MarkovChain::steady_state", "A", chain_ins(c, 0))
    for n in (2, 3, 5, 7, 12, 20):
        for _ in range(4):
            row(m, "MarkovChain::steady_state", "A", chain_ins([stochastic(g, n, 0.4) for _ in range(n)], 0))
    # periodic chains whose uniform start isn't stationary: the residual stalls, then the lazy chain converges
    periodic = [[[0.0, 0.5, 0.5], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                [[0.0, 1.0, 0.0, 0.0], [0.3, 0.0, 0.7, 0.0], [0.0, 0.4, 0.0, 0.6], [0.0, 0.0, 1.0, 0.0]],
                [[0.0, 0.3, 0.7, 0.0], [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 0.0]]]
    for k, l in ((1, 2), (2, 3), (3, 3), (2, 5), (4, 6)):
        top = [[0.0] * k + stochastic(g, l) for _ in range(k)]
        periodic.append(top + [stochastic(g, k) + [0.0] * l for _ in range(l)])
    for c in periodic:
        row(m, "MarkovChain::steady_state", "A", chain_ins(c, 0))
        row(m, "MarkovChain::simulate", "A", chain_ins(c, 0) + [i(5), i(12)])
    # second eigenvalue ±0.995: the residual shrinks by 0.995 a step, under the 0.999 stall bound, converging in
    # about 4,600 steps (between 10³ and 10⁴)
    for a, b in ((0.002, 0.003), (0.0015, 0.0035), (0.001, 0.004)):
        row(m, "MarkovChain::steady_state", "A", chain_ins([[1.0 - a, a], [b, 1.0 - b]], 0))
        row(m, "MarkovChain::steady_state", "A", chain_ins([[a, 1.0 - a], [1.0 - b, b]], 0))
    for c in STALL_RESET:
        row(m, "MarkovChain::steady_state", "A", chain_ins([[float.fromhex(x) for x in r] for r in c], 0))
    # HiddenMarkovModel::new and its refusals
    hs = hmms(g)
    for h in hs:
        row(m, "HiddenMarkovModel::new", "A", hmm_ins(h, []))
    base = hs[0]
    for x in (NAN, INF, -0.1, 1.0 + 2e-6, 1.0 + 1e-6, -0.0):
        row(m, "HiddenMarkovModel::new", "A", hmm_ins((base[0], base[1], [x, 0.4]), []))
        row(m, "HiddenMarkovModel::new", "A", hmm_ins(([[0.7, 0.3], [x - 0.6, 0.6]], base[1], base[2]), []))
        row(m, "HiddenMarkovModel::new", "A", hmm_ins((base[0], [[0.5, 0.4, 0.1], [x - 0.9, 0.3, 0.6]], base[2]), []))
    a0, b0, p0 = base
    for ins in (sl(flat(a0)) + sl(flat(b0)) + sl([]) + isl([]) + [i(2), i(2), i(2), i(3)],
                hmm_ins(([[1.0, 0.0, 0.0]] * 2, b0, p0), [], dims=(2, 3, 2, 3)),
                hmm_ins(([[1.0]], b0, p0), [], dims=(1, 1, 2, 3)),
                hmm_ins((a0, [[1.0, 0.0, 0.0]], p0), [], dims=(2, 2, 1, 3)),
                hmm_ins((a0, [], p0), [], dims=(2, 2, 2, 0)), hmm_ins((a0, [], p0), [], dims=(2, 2, 0, 0)),
                hmm_ins(([[0.5, 0.3], [0.4, 0.6]], [[0.5, 0.6, 0.1], [0.1, 0.3, 0.6]], [0.6, 0.6]), []),
                hmm_ins(([[0.5, 0.3], [0.4, 0.6]], [[0.5, 0.6, 0.1], [0.1, 0.3, 0.6]], p0), []),
                hmm_ins((a0, [[0.5, 0.6, 0.1], [0.1, 0.3, 0.6]], [0.6, 0.6]), [], dims=(3, 2, 2, 3))):
        row(m, "HiddenMarkovModel::new", "A", ins)
    # the algorithms
    for h in hs:
        nm = len(h[1][0])
        seqs = [[0], [nm - 1], sample_obs(g, h, 2), sample_obs(g, h, 10), sample_obs(g, h, 60), sample_obs(g, h, 300),
                [], [nm], [0, nm, 0], [0, U64], [k % nm for k in range(40)], [(k * 7) % nm for k in range(17)]]
        for obs in seqs:
            row(m, "HiddenMarkovModel::forward_log_likelihood", "C", hmm_ins(h, obs))
            row(m, "HiddenMarkovModel::viterbi", "C", hmm_ins(h, obs))
        for obs in seqs:
            for it in ((1, 3, 10) if len(obs) < 100 else (1, 3)):
                row(m, "HiddenMarkovModel::baum_welch", "A", hmm_ins(h, obs, (it,)))
        row(m, "HiddenMarkovModel::baum_welch", "A", hmm_ins(h, seqs[2], (0,)))
        row(m, "HiddenMarkovModel::baum_welch", "A", hmm_ins(h, [], (0,)))
        row(m, "HiddenMarkovModel::baum_welch", "A", hmm_ins(h, seqs[4], (40,)))
    long = hs[len(hs) - 4]
    for t in (1000, 2000):
        obs = sample_obs(g, long, t)
        row(m, "HiddenMarkovModel::forward_log_likelihood", "C", hmm_ins(long, obs))
        row(m, "HiddenMarkovModel::viterbi", "C", hmm_ins(long, obs))
        row(m, "HiddenMarkovModel::baum_welch", "A", hmm_ins(long, obs, (2,)))
    # zero-probability sequences: symbol 0 from the third state of the left-to-right model, then ComputationError
    lr = hs[3]
    for obs in ([1, 1, 0], [0, 1, 1, 1, 0], [1, 0, 1], [0, 0, 1, 1, 1, 1]):
        row(m, "HiddenMarkovModel::forward_log_likelihood", "C", hmm_ins(lr, obs))
        row(m, "HiddenMarkovModel::viterbi", "C", hmm_ins(lr, obs))
        row(m, "HiddenMarkovModel::baum_welch", "A", hmm_ins(lr, obs, (5,)))
    return m
