//! `math.tsv` — what Rust **std** returns for every f64 operation pramana 1.3.0's library code uses: the
//! reference for the step 8 baselines and, through `validate.py`, for the correctly rounded rows `math.cr.tsv`
//! (port plan §5 step 5, §8.1).
//!
//! The calls, counted in `rust-old/src` outside test code (2026-11-15): ln 53, is_finite 39, abs 38, exp 32,
//! sqrt 25, is_nan 23, max 21, powi 17 (exponents 2, 3, 4, 9, 25, 49, always literals), ln_1p 11, clamp 10, min 9,
//! powf 7, floor 4, cos 3, sin 2, exp_m1 2, fract 2, tan 1, atan 1, log2 1, cbrt 1. Then the operations the
//! semantics probe (port-parity §1) found need helpers or rules: `u64 as f64`, `as usize`, the fdlibm high word
//! and low mask, and `total_cmp`.
//!
//! Where each result comes from (rustc 1.100.0, x86_64-unknown-linux-gnu; `nm -D` on the binary): see the
//! README's table. In short, the transcendentals are glibc's (these rows are glibc's bits: the header records the
//! glibc, and `--check` refuses another), `cbrt` is compiler_builtins' (platform-free), and everything else is
//! inline code.
//!
//! Every input passes through `black_box`, so each row is a run-time call, never a fold. The exponent of `powi` is
//! *not* black-boxed: pramana always writes it as a literal, so the rows record the expansion its code runs.
//!
//! Classes: **A** exact (sqrt, abs, floor, fract, min, max, clamp, the predicates, the casts and the word
//! operations); **B** arithmetic (the powi expansions); **C** transcendental (glibc's and compiler_builtins').
//! Adapted from pravash's `m_math.rs`.

use std::f64::consts::PI;
use std::hint::black_box;

use crate::grids::{Rng, pow2, specials};
use crate::tsv::{Class, Module, f, i};

fn unary(m: &mut Module, name: &str, class: Class, fun: fn(f64) -> f64, xs: &[f64]) {
    for &x in xs {
        let y = fun(black_box(x));
        m.row(name, class, &[f(x)], &[f(y)]);
    }
}

fn binary(m: &mut Module, name: &str, class: Class, fun: fn(f64, f64) -> f64, xy: &[(f64, f64)]) {
    for &(x, y) in xy {
        let r = fun(black_box(x), black_box(y));
        m.row(name, class, &[f(x), f(y)], &[f(r)]);
    }
}

/// Every ordered pair of `xs`.
fn grid(xs: &[f64]) -> Vec<(f64, f64)> {
    xs.iter()
        .flat_map(|&x| xs.iter().map(move |&y| (x, y)))
        .collect()
}

/// Specials, ±2^k at every 16th binade, and a wide signed log-uniform sweep.
fn wide(seed: u64, n: usize) -> Vec<f64> {
    let mut v = specials();
    for k in (-1074..=1023).step_by(16) {
        let x = pow2(k);
        v.push(x);
        v.push(-x);
    }
    let mut rng = Rng::new(seed);
    v.extend((0..n).map(|_| rng.signed_log_uniform(-300.0, 300.0)));
    v
}

fn sweep(seed: u64, n: usize, lo: f64, hi: f64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.uniform(lo, hi)).collect()
}

fn log_sweep(seed: u64, n: usize, lo_exp10: f64, hi_exp10: f64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.log_uniform(lo_exp10, hi_exp10)).collect()
}

fn signed_log_sweep(seed: u64, n: usize, lo_exp10: f64, hi_exp10: f64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| rng.signed_log_uniform(lo_exp10, hi_exp10))
        .collect()
}

/// The doubles either side of every integer in `lo..=hi`, the halves, and the integers.
fn around_integers(lo: i32, hi: i32) -> Vec<f64> {
    let mut v = Vec::new();
    for k in lo..=hi {
        let x = f64::from(k);
        v.push(x);
        v.push(x + 0.5);
        v.push(f64::from_bits(x.to_bits().wrapping_add(1)));
        v.push(f64::from_bits(x.to_bits().wrapping_sub(1)));
    }
    v
}

fn powi2(x: f64) -> f64 {
    x.powi(2)
}
fn powi3(x: f64) -> f64 {
    x.powi(3)
}
fn powi4(x: f64) -> f64 {
    x.powi(4)
}
fn powi9(x: f64) -> f64 {
    x.powi(9)
}
fn powi25(x: f64) -> f64 {
    x.powi(25)
}
fn powi49(x: f64) -> f64 {
    x.powi(49)
}

/// `x as usize` as the 64-bit word (a value ≥ 2^63 reads negative as an `i:` token).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn as_usize(x: f64) -> i64 {
    (black_box(x) as usize) as i64
}

/// fdlibm's high word, `(x.to_bits() >> 32) as u32`, as `rust-old/src/math.rs` computes it.
#[allow(clippy::cast_possible_truncation)]
fn high_word(x: f64) -> i64 {
    i64::from((black_box(x).to_bits() >> 32) as u32)
}

/// fdlibm's low-word mask, `f64::from_bits(x.to_bits() & 0xffff_ffff_0000_0000)`.
fn low_masked(x: f64) -> f64 {
    f64::from_bits(black_box(x).to_bits() & 0xffff_ffff_0000_0000)
}

/// `u64 as f64` (Binomial's `n`), rounded to nearest.
#[allow(clippy::cast_precision_loss)]
fn u64_as_f64(n: u64) -> f64 {
    black_box(n) as f64
}

#[allow(clippy::too_many_lines)] // one grid per function, in the module comment's order
pub fn emit() -> Module {
    let mut m = Module::new("math");

    // ── exact (class A) ──
    let mut xs = wide(0x5157, 300);
    xs.extend(log_sweep(0x5158, 300, -40.0, 20.0));
    xs.extend((1..=100).map(|k| f64::from(k * k)));
    xs.extend(sweep(0x5159, 300, 0.0, 1e4));
    unary(&mut m, "f64::sqrt", Class::A, f64::sqrt, &xs);
    unary(&mut m, "f64::abs", Class::A, f64::abs, &wide(0xab5, 100));

    // floor: percentile ranks, the Poisson / Binomial CDF's floor(x), the histogram bins.
    let mut halves = around_integers(-20, 20);
    halves.extend([
        0.499_999_999_999_999_94,
        -0.499_999_999_999_999_94,
        4_503_599_627_370_495.5,
        -4_503_599_627_370_495.5,
        4_503_599_627_370_497.0,
    ]);
    halves.extend(sweep(0x7a11, 300, -1e4, 1e4));
    halves.extend(wide(0x7a12, 100));
    unary(&mut m, "f64::floor", Class::A, f64::floor, &halves);

    // fract: the Poisson and Binomial pdfs' integer test (`x.fract() != 0.0`). Rust's is x − trunc(x).
    let mut fr = halves.clone();
    fr.extend(around_integers(-3, 3).iter().map(|&x| -x));
    unary(&mut m, "f64::fract", Class::A, f64::fract, &fr);

    // min / max: every special pair (NaN on either side, ±0 both ways, ±∞), equal pairs, a value against 0.0 and
    // f64::MIN_POSITIVE both ways (pramana's samplers write `u.max(f64::MIN_POSITIVE)`), and random pairs.
    let mut mm = grid(&specials());
    let mut rng = Rng::new(0x3a3a);
    for _ in 0..60 {
        let x = rng.signed_log_uniform(-30.0, 30.0);
        mm.extend([
            (x, x),
            (x, 0.0),
            (0.0, x),
            (x, f64::NAN),
            (f64::NAN, x),
            (x, f64::MIN_POSITIVE),
        ]);
    }
    let mut rng = Rng::new(0x3a3b);
    for _ in 0..300 {
        let x = rng.signed_log_uniform(-300.0, 300.0);
        mm.push((x, rng.signed_log_uniform(-300.0, 300.0)));
    }
    binary(&mut m, "f64::max", Class::A, f64::max, &mm);
    binary(&mut m, "f64::min", Class::A, f64::min, &mm);

    // clamp: valid bounds only (min ≤ max, neither NaN; Rust panics otherwise), probed at and around each bound,
    // ±0, NaN and a random value. pramana's own bounds first (probabilities, the Binomial splitting's p).
    let mut rng = Rng::new(0xc1a2);
    let mut bounds = vec![(0.0, 1.0), (-1.0, 1.0), (0.0, 0.0), (-0.0, 0.0), (0.0, f64::INFINITY)];
    for _ in 0..60 {
        let a = rng.signed_log_uniform(-10.0, 10.0);
        let c = rng.signed_log_uniform(-10.0, 10.0);
        bounds.push(if a <= c { (a, c) } else { (c, a) });
    }
    for &(lo, hi) in &bounds {
        let probes = [
            lo,
            hi,
            0.0,
            -0.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            rng.signed_log_uniform(-10.0, 10.0),
        ];
        for &x in &probes {
            let r = black_box(x).clamp(black_box(lo), black_box(hi));
            m.row("f64::clamp", Class::A, &[f(x), f(lo), f(hi)], &[f(r)]);
        }
    }

    // is_finite / is_nan
    let mut preds = specials();
    preds.extend([
        f64::from_bits(0xfff8_0000_0000_0000), // -NaN
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling-pattern NaN
        -f64::MAX,
        -f64::from_bits(1),
    ]);
    preds.extend(signed_log_sweep(0xf1f1, 20, -300.0, 300.0));
    for &x in &preds {
        m.row("f64::is_finite", Class::A, &[f(x)], &[i(i64::from(black_box(x).is_finite()))]);
        m.row("f64::is_nan", Class::A, &[f(x)], &[i(i64::from(black_box(x).is_nan()))]);
    }

    // total_cmp (mode's sort): every pair of specials and of the probe's key cases, as -1 / 0 / 1.
    let mut keys = specials();
    keys.extend([f64::from_bits(0xfff8_0000_0000_0000), -f64::MAX, -f64::from_bits(1)]);
    for (x, y) in grid(&keys) {
        let o = black_box(x).total_cmp(&black_box(y)) as i8;
        m.row("f64::total_cmp", Class::A, &[f(x), f(y)], &[i(i64::from(o))]);
    }

    // `as usize`: percentile's rank, the iteration budget, the histogram bin. Truncation, saturation at both
    // ends, NaN → 0.
    let mut casts = specials();
    casts.extend(around_integers(-5, 5));
    casts.extend(sweep(0xca51, 200, -1e3, 1e6));
    casts.extend([
        9_223_372_036_854_775_807.0,  // rounds to 2^63
        9_223_372_036_854_777_856.0,  // 2^63 + 2048
        18_446_744_073_709_549_568.0, // the largest double below 2^64
        18_446_744_073_709_551_616.0, // 2^64
        1e6,                          // MAX_ITERATIONS
    ]);
    casts.extend(wide(0xca52, 100));
    for &x in &casts {
        m.row("f64::as_usize", Class::A, &[f(x)], &[i(as_usize(x))]);
    }

    // `u64 as f64`: Binomial's n, trials and successes. Every power of two and its neighbours, the 2^53 rounding
    // boundary, the words ≥ 2^63 that a signed conversion gets wrong, and random words across the range.
    let mut words: Vec<u64> = vec![0, 1, u64::MAX, u64::MAX - 1];
    for k in 0..64_u32 {
        let p = 1_u64 << k;
        words.extend([p, p.wrapping_sub(1), p.wrapping_add(1)]);
        if k >= 54 {
            // The rounding boundary above 2^k (an ulp there is 2^(k−52)): the half-ulp tie (rounds to the even
            // 2^k), just above and just below it, and a tie on an odd mantissa (rounds up).
            let half = 1_u64 << (k - 53);
            words.extend([p | half, p | half | 1, p | (half >> 1), p | (half << 1) | half]);
        }
    }
    let mut rng = Rng::new(0x64f6);
    for _ in 0..300 {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let hi = (rng.unit() * 4_294_967_296.0) as u64;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let lo = (rng.unit() * 4_294_967_296.0) as u64;
        let shift = rng.int(0, 63);
        #[allow(clippy::cast_sign_loss)]
        words.push(((hi << 32) | lo) >> shift as u64);
    }
    for &n in &words {
        #[allow(clippy::cast_possible_wrap)]
        m.row("u64::as_f64", Class::A, &[i(n as i64)], &[f(u64_as_f64(n))]);
    }

    // The fdlibm erf port's word operations (rust-old/src/math.rs).
    let mut ws = wide(0x4f4f, 200);
    ws.extend(sweep(0x4f50, 200, -6.0, 6.0));
    ws.extend([0.84375, -0.84375, 1.25, 6.0, 28.0, f64::from_bits(0x3e30_0000_0000_0000)]);
    for &x in &ws {
        m.row("f64::high_word", Class::A, &[f(x)], &[i(high_word(x))]);
    }
    unary(&mut m, "f64::low_masked", Class::A, low_masked, &ws);

    // ── arithmetic (class B): powi with the literal exponents pramana uses ──
    let mut bases = sweep(0x9013, 300, -2.0, 2.0);
    bases.extend(log_sweep(0x9014, 200, -10.0, 10.0));
    bases.extend(specials());
    unary(&mut m, "f64::powi2", Class::B, powi2, &bases);
    unary(&mut m, "f64::powi3", Class::B, powi3, &bases);
    unary(&mut m, "f64::powi4", Class::B, powi4, &bases);
    // 9, 25, 49: the KS series' y^9, y^25, y^49 with y = exp(−π²/(8λ²)) ∈ (0, 1), and x^4, x^9 in its tail.
    let mut ys = sweep(0x9015, 400, 0.0, 1.0);
    ys.extend(specials());
    unary(&mut m, "f64::powi9", Class::B, powi9, &ys);
    unary(&mut m, "f64::powi25", Class::B, powi25, &ys);
    unary(&mut m, "f64::powi49", Class::B, powi49, &ys);

    // ── transcendental (class C) ──
    // exp: the pdfs' exp(ln pdf), the samplers, the special-function prefactors.
    let mut ex = wide(0xe0e0, 200);
    ex.extend(sweep(0xe0e1, 1500, -745.0, 710.0));
    ex.extend(sweep(0xe0e2, 1000, -50.0, 5.0));
    ex.extend(signed_log_sweep(0xe0e3, 300, -20.0, 0.0));
    unary(&mut m, "f64::exp", Class::C, f64::exp, &ex);

    // ln: the log-pdfs, ln u in the samplers, the special functions.
    let mut ln = specials();
    ln.extend([-1.0, -0.0]);
    ln.extend(log_sweep(0x1e01, 1500, -308.0, 308.0));
    ln.extend(sweep(0x1e02, 800, 0.0, 1.0));
    let mut rng = Rng::new(0x1e03);
    for _ in 0..400 {
        let d = rng.log_uniform(-16.0, -1.0);
        ln.extend([1.0 + d, 1.0 - d]);
    }
    unary(&mut m, "f64::ln", Class::C, f64::ln, &ln);

    // ln_1p: the t pdf's ln(1 + x²/ν), the beta prefactors, bd0.
    let mut l1 = specials();
    l1.extend([-1.0, -0.5, -0.999_999_999_999_999_9]);
    l1.extend(log_sweep(0x11a1, 600, -20.0, 3.0));
    l1.extend(log_sweep(0x11a2, 400, -20.0, 0.0).iter().map(|&x| -x.min(0.999_999_999)));
    unary(&mut m, "f64::ln_1p", Class::C, f64::ln_1p, &l1);

    // exp_m1: the Exponential and Weibull CDFs (−exp_m1(−x)).
    let mut em = specials();
    em.extend(signed_log_sweep(0xe1a1, 600, -20.0, 2.0));
    em.extend(sweep(0xe1a2, 400, -40.0, 710.0));
    unary(&mut m, "f64::exp_m1", Class::C, f64::exp_m1, &em);

    // powf: Silverman's n^(−0.2), the gamma sampler's boost u^(1/α), Weibull's z^k and z^(k−1), random pairs and
    // every special pair.
    let mut pw: Vec<(f64, f64)> = (1..=2000).map(|n| (f64::from(n), -0.2)).collect();
    let mut rng = Rng::new(0x9a01);
    for _ in 0..500 {
        let u = rng.unit().max(f64::MIN_POSITIVE);
        pw.push((u, 1.0 / rng.log_uniform(-2.0, 0.0)));
    }
    for _ in 0..500 {
        let z = rng.log_uniform(-3.0, 2.0);
        let k = rng.uniform(0.2, 10.0);
        pw.extend([(z, k), (z, k - 1.0)]);
    }
    for _ in 0..800 {
        let b = rng.log_uniform(-3.0, 3.0);
        pw.push((b, rng.uniform(-50.0, 50.0)));
    }
    pw.extend(grid(&specials()));
    binary(&mut m, "f64::powf", Class::C, f64::powf, &pw);

    // sin, cos: Box–Muller's cos(2π·u), the reflection's sin(π·x), and sweeps.
    let mut tr = specials();
    tr.extend(sweep(0x5c01, 1000, -10.0, 10.0));
    let mut rng = Rng::new(0x5c02);
    tr.extend((0..1000).map(|_| 2.0 * PI * rng.unit()));
    tr.extend((0..300).map(|_| PI * rng.uniform(-20.0, 0.5)));
    tr.extend(log_sweep(0x5c03, 200, 0.0, 6.0));
    unary(&mut m, "f64::sin", Class::C, f64::sin, &tr);
    unary(&mut m, "f64::cos", Class::C, f64::cos, &tr);

    // tan: the Cauchy sampler's tan(π(u − 0.5)).
    let mut tn = specials();
    tn.extend(sweep(0x7a01, 1000, -1.57, 1.57));
    let mut rng = Rng::new(0x7a02);
    tn.extend((0..1000).map(|_| PI * (rng.unit() - 0.5)));
    unary(&mut m, "f64::tan", Class::C, f64::tan, &tn);

    // atan: the Cauchy CDF's atan((x − x0)/γ).
    let mut at = specials();
    at.extend(signed_log_sweep(0xa7a1, 1000, -10.0, 10.0));
    at.extend(sweep(0xa7a2, 500, -50.0, 50.0));
    unary(&mut m, "f64::atan", Class::C, f64::atan, &at);

    // log2: Hick's log2(choices), choices a count.
    let mut l2: Vec<f64> = (1..=2000).map(f64::from).collect();
    l2.extend(specials());
    l2.extend(log_sweep(0x1092, 500, -300.0, 300.0));
    unary(&mut m, "f64::log2", Class::C, f64::log2, &l2);

    // cbrt: the Wilson–Hilferty branch's cbrt(x/a).
    let mut cb = specials();
    cb.extend(signed_log_sweep(0xcb01, 500, -300.0, 300.0));
    cb.extend(sweep(0xcb02, 300, 0.5, 2.0));
    unary(&mut m, "f64::cbrt", Class::C, f64::cbrt, &cb);

    m
}
