//! `dist.tsv` — 1.3.0's distributions, `src/distribution.rs` (M3; port plan §7). Rows are named by Rust's path:
//! `Normal::pdf`, and `standard_normal` for the oracle's Box–Muller hook. Each part adds its distributions.
//!
//! Part a1, `Normal` and Box–Muller:
//! - `Normal::new`: the constructor on every pair of a parameter grid (the specials, 0.25, ±1e-300, ±1e300, −3.5):
//!   the two fields it holds, or the error. Class A.
//! - `Normal::pdf`, `Normal::cdf`: N(0, 1) at x = k/16 on [−40, 40]; nine fixed (μ, σ) at x = μ + σ·k/4 on
//!   z ∈ [−40, 40] and at the specials; 40 seeded (μ, σ) at z = −40, −37.5, …, 40; and 1,000 seeded (μ, σ, x).
//!   Class C: the cdf is downstream of the T1 Φ (ADR 009), the pdf takes glibc's exp.
//! - `Normal::mean`, `Normal::variance`: every (μ, σ) above. Class A.
//! - `Normal::sample`: 10^4 draws of N(0, 1) from three seeds (the plan's 10^4-draw streams), and 256 draws of each
//!   other fixed (μ, σ) from the same seeds. Class C: Box–Muller takes glibc's ln and cos.
//! - `standard_normal`: the oracle's hook, 1,024 draws from three seeds. Class C.
//!
//! Part a2, `Uniform`, `Exponential` and `Bernoulli`, the same kinds of rows:
//! - `X::new` on the parameter grid (with ±1e308, so Uniform's width can overflow): the fields, or the error;
//! - `X::pdf` and `X::cdf` at fixed parameters across and beyond the support, at its edges and their neighbours, at
//!   the specials, and Bernoulli's at the f64::EPSILON windows around 0 and 1; at 40 seeded parameters; at 1,000
//!   seeded points;
//! - `X::mean` and `X::variance` at every parameter above;
//!   Class A, but Exponential's pdf, cdf and sample, class C (glibc's exp, expm1 and ln);
//! - `X::sample`: 10^4 draws of the standard case (U(0, 1), Exp(1), Bernoulli(0.5)) from three seeds, and 256 of
//!   each other fixed case.
//!
//! Part a3, `Cauchy` and `Weibull`, the same kinds of rows, and for Weibull's mean and variance, which take the T1
//! lnΓ, a wider grid of shapes. Class C but Cauchy's constructor, pdf, mean and variance and Weibull's constructor
//! (class A): Cauchy's cdf and draws take glibc's atan and tan, Weibull's take glibc's pow, exp, expm1 and ln, and its
//! moments 1.3.0's Lanczos lnΓ.
//!
//! Part b1, `Gamma` and `ChiSquared`, the same kinds of rows, the pdf and cdf also around the mode (the mean ± up to
//! 12 standard deviations), and `sample_gamma_standard`, the oracle's Marsaglia–Tsang hook, 1,024 draws at eleven
//! shapes from three seeds. Class C but the constructors, means and variances: the pdf takes glibc's ln and exp and
//! 1.3.0's Lanczos lnΓ, the cdf 1.3.0's incomplete gamma, and the draws glibc's ln, cos and pow.
//!
//! Part b2, `Beta`, `StudentT` and `FDistribution`, the same kinds of rows, and the oracle's Student-t and F tails
//! (`student_t_two_tailed`, `student_t_cdf`, `f_cdf_sf`, which the hypothesis tests share) on their own grids. Class C
//! but the constructors, means and variances: the pdfs take glibc's ln, ln_1p and exp and 1.3.0's lnΓ or ln B, the
//! cdfs 1.3.0's incomplete beta, and the draws Marsaglia–Tsang's.
//!
//! Part c1, `Poisson`: the pmf and cdf at the integers, off them and around the mean, the draws either side of the
//! sampler's switch at λ = 30 (Knuth's multiplication, then PTRS), and `sample_poisson_ptrs`, the oracle's PTRS
//! hook, 1,024 draws at nine rates from three seeds. Class C but the constructor, mean and variance: the pmf takes
//! glibc's ln and exp and 1.3.0's lnΓ, the cdf 1.3.0's incomplete gamma, and the draws glibc's exp and ln.
//!
//! Part c2, `Binomial`: n is a u64 (an `i:` token holding its 64-bit word, so u64::MAX is `i:-1`). The pmf and cdf at
//! the integers, off them, around the mean and at both ends, for n from 1 to u64::MAX, and the draws either side of
//! the direct simulation's 64 trials (Beta splitting above). Class C but the constructor, mean and variance: the pmf
//! takes glibc's ln, ln_1p and exp and 1.3.0's lnΓ (Stirling's remainder below 10), the cdf 1.3.0's incomplete beta,
//! and the draws Marsaglia–Tsang's.
//!
//! Part d2, `MultivariateNormal`: its rows come from `m_mvn.rs`, which says what they are.
//!
//! Inputs: the parameters, then x (or the seed and the count). Outputs: the value, or the draws as a sequence.

use std::hint::black_box;

use crate::grids::{Rng as Seeds, specials};
use crate::tsv::{Class, Module, Tok, err, f, i};
use pramana::distribution::{
    Bernoulli, Beta, Binomial, Cauchy, ChiSquared, Distribution, Exponential, FDistribution, Gamma, Normal, Poisson,
    StudentT, Uniform, Weibull,
};
use pramana::{SimpleRng, oracle};

const SEEDS: [u64; 3] = [42, 0x9e37_79b9_7f4a_7c15, 12345];

/// The fixed (μ, σ) pairs: the standard normal, ordinary, small and large scales, a signed zero, f64::MAX's mean.
fn fixed_normals() -> Vec<(f64, f64)> {
    vec![
        (0.0, 1.0),
        (1.5, 2.3),
        (-3.5, 0.25),
        (1e6, 1e-3),
        (-0.0, 1.0),
        (0.0, 1e-300),
        (0.0, 1e300),
        (-1e-300, 7.0),
        (f64::MAX, 1.0),
    ]
}

/// A row's draws: the count, then each.
fn draws(xs: &[f64]) -> Vec<Tok> {
    let mut t = vec![Tok::V(xs.len())];
    t.extend(xs.iter().map(|&x| f(x)));
    t
}

#[allow(clippy::cast_possible_wrap)]
fn normal(m: &mut Module, g: &mut Seeds) {
    let mut grid = specials();
    grid.extend([0.25, 1e-300, -1e-300, 1e300, -1e300, -3.5]);
    for &mu in &grid {
        for &sd in &grid {
            match Normal::new(black_box(mu), black_box(sd)) {
                Ok(n) => m.row("Normal::new", Class::A, &[f(mu), f(sd)], &[f(n.mean), f(n.std_dev)]),
                Err(e) => m.row("Normal::new", Class::A, &[f(mu), f(sd)], &[err(&e)]),
            }
        }
    }
    let mut params = fixed_normals();
    let seeded: Vec<(f64, f64)> =
        (0..40).map(|_| (g.signed_log_uniform(-6.0, 6.0), g.log_uniform(-4.0, 4.0))).collect();
    params.extend(&seeded);
    let point = |m: &mut Module, mu: f64, sd: f64, x: f64| {
        let n = Normal::new(mu, sd).expect("valid parameters");
        m.row("Normal::pdf", Class::C, &[f(mu), f(sd), f(x)], &[f(n.pdf(black_box(x)))]);
        m.row("Normal::cdf", Class::C, &[f(mu), f(sd), f(x)], &[f(n.cdf(black_box(x)))]);
    };
    for k in -640..=640 {
        point(m, 0.0, 1.0, f64::from(k) / 16.0);
    }
    for &(mu, sd) in &fixed_normals()[1..] {
        for k in -160..=160 {
            point(m, mu, sd, mu + sd * (f64::from(k) / 4.0));
        }
        for &x in &specials() {
            point(m, mu, sd, x);
        }
    }
    for &(mu, sd) in &seeded {
        for k in -16..=16 {
            point(m, mu, sd, mu + sd * (2.5 * f64::from(k)));
        }
    }
    for _ in 0..1000 {
        let mu = g.signed_log_uniform(-3.0, 3.0);
        let sd = g.log_uniform(-3.0, 3.0);
        let x = mu + sd * g.uniform(-12.0, 12.0);
        point(m, mu, sd, x);
    }
    for &(mu, sd) in &params {
        let n = Normal::new(mu, sd).expect("valid parameters");
        m.row("Normal::mean", Class::A, &[f(mu), f(sd)], &[f(n.mean())]);
        m.row("Normal::variance", Class::A, &[f(mu), f(sd)], &[f(n.variance())]);
    }
    for &seed in &SEEDS {
        for (k, &(mu, sd)) in fixed_normals().iter().enumerate() {
            let count = if k == 0 { 10_000 } else { 256 };
            let n = Normal::new(mu, sd).expect("valid parameters");
            let mut r = SimpleRng::new(seed);
            let xs: Vec<f64> = (0..count).map(|_| n.sample(&mut r)).collect();
            m.row("Normal::sample", Class::C, &[f(mu), f(sd), i(seed as i64), i(count as i64)], &draws(&xs));
        }
    }
    for seed in [1u64, 42, u64::MAX] {
        let mut r = SimpleRng::new(seed);
        let xs: Vec<f64> = (0..1024).map(|_| oracle::standard_normal(&mut r)).collect();
        m.row("standard_normal", Class::C, &[i(seed as i64), i(1024)], &draws(&xs));
    }
}

/// The parameter grid the constructors get: the specials, then values either side of every check.
fn param_grid() -> Vec<f64> {
    let mut grid = specials();
    grid.extend([0.25, 1e-300, -1e-300, 1e300, -1e300, -3.5, 1e308, -1e308, 0.5, 2.0]);
    grid
}

/// x and its two neighbours (the bit patterns either side), for a finite x.
fn around(x: f64) -> [f64; 3] {
    let up = if x == 0.0 { f64::from_bits(1) } else { f64::from_bits(x.to_bits() + 1) };
    let down = if x == 0.0 { -f64::from_bits(1) } else { f64::from_bits(x.to_bits() - 1) };
    if x < 0.0 { [up, x, down] } else { [down, x, up] }
}

/// The streams: 10^4 draws of the first case and 256 of the others, from each seed.
#[allow(clippy::cast_possible_wrap)]
fn streams<D: Distribution>(m: &mut Module, func: &str, class: Class, cases: &[(Vec<f64>, D)]) {
    for &seed in &SEEDS {
        for (k, (ins, d)) in cases.iter().enumerate() {
            let count = if k == 0 { 10_000 } else { 256 };
            let mut r = SimpleRng::new(seed);
            let xs: Vec<f64> = (0..count).map(|_| d.sample(&mut r)).collect();
            let mut toks: Vec<Tok> = ins.iter().map(|&v| f(v)).collect();
            toks.push(i(seed as i64));
            toks.push(i(count as i64));
            m.row(func, class, &toks, &draws(&xs));
        }
    }
}

fn uniform(m: &mut Module, g: &mut Seeds) {
    for &lo in &param_grid() {
        for &hi in &param_grid() {
            match Uniform::new(black_box(lo), black_box(hi)) {
                Ok(u) => m.row("Uniform::new", Class::A, &[f(lo), f(hi)], &[f(u.min), f(u.max)]),
                Err(e) => m.row("Uniform::new", Class::A, &[f(lo), f(hi)], &[err(&e)]),
            }
        }
    }
    let fixed: Vec<(f64, f64)> = vec![
        (0.0, 1.0),
        (-1.0, 5.0),
        (0.0, 10.0),
        (-3.5, 0.25),
        (-0.0, 1.0),
        (1e-300, 2e-300),
        (-1e300, 1e300),
        (0.0, f64::MAX),
        (1e6, 1e6 + 1.0),
        (1e308, f64::MAX),
        (f64::from_bits(1), f64::from_bits(2)),
    ];
    let seeded: Vec<(f64, f64)> = (0..40)
        .map(|_| {
            let lo = g.signed_log_uniform(-6.0, 6.0);
            (lo, lo + g.log_uniform(-4.0, 4.0))
        })
        .collect();
    let point = |m: &mut Module, lo: f64, hi: f64, x: f64| {
        let u = Uniform::new(lo, hi).expect("valid parameters");
        m.row("Uniform::pdf", Class::A, &[f(lo), f(hi), f(x)], &[f(u.pdf(black_box(x)))]);
        m.row("Uniform::cdf", Class::A, &[f(lo), f(hi), f(x)], &[f(u.cdf(black_box(x)))]);
    };
    for &(lo, hi) in fixed.iter().chain(&seeded) {
        let w = hi - lo;
        for k in -8..=24 {
            point(m, lo, hi, lo + w * (f64::from(k) / 16.0));
        }
        for x in around(lo).into_iter().chain(around(hi)) {
            point(m, lo, hi, x);
        }
    }
    for &(lo, hi) in &fixed {
        for &x in &specials() {
            point(m, lo, hi, x);
        }
    }
    for _ in 0..1000 {
        let lo = g.signed_log_uniform(-3.0, 3.0);
        let hi = lo + g.log_uniform(-3.0, 3.0);
        let x = lo + (hi - lo) * g.uniform(-0.5, 1.5);
        point(m, lo, hi, x);
    }
    for &(lo, hi) in fixed.iter().chain(&seeded) {
        let u = Uniform::new(lo, hi).expect("valid parameters");
        m.row("Uniform::mean", Class::A, &[f(lo), f(hi)], &[f(u.mean())]);
        m.row("Uniform::variance", Class::A, &[f(lo), f(hi)], &[f(u.variance())]);
    }
    let cases: Vec<(Vec<f64>, Uniform)> =
        fixed.iter().map(|&(lo, hi)| (vec![lo, hi], Uniform::new(lo, hi).expect("valid"))).collect();
    streams(m, "Uniform::sample", Class::A, &cases);
}

fn exponential(m: &mut Module, g: &mut Seeds) {
    for &l in &param_grid() {
        match Exponential::new(black_box(l)) {
            Ok(e) => m.row("Exponential::new", Class::A, &[f(l)], &[f(e.lambda)]),
            Err(e) => m.row("Exponential::new", Class::A, &[f(l)], &[err(&e)]),
        }
    }
    let fixed: Vec<f64> = vec![1.0, 2.0, 0.5, 2.5, 1e-3, 1e3, 1e-300, 1e300, f64::MIN_POSITIVE, f64::MAX];
    let seeded: Vec<f64> = (0..40).map(|_| g.log_uniform(-6.0, 6.0)).collect();
    let point = |m: &mut Module, l: f64, x: f64| {
        let e = Exponential::new(l).expect("valid parameters");
        m.row("Exponential::pdf", Class::C, &[f(l), f(x)], &[f(e.pdf(black_box(x)))]);
        m.row("Exponential::cdf", Class::C, &[f(l), f(x)], &[f(e.cdf(black_box(x)))]);
    };
    for &l in fixed.iter().chain(&seeded) {
        for k in -8..=320 {
            point(m, l, (f64::from(k) / 8.0) / l);
        }
        for x in [1e-20, 1e-300, f64::from_bits(1), 1e-10 / l, 1e-16 / l] {
            point(m, l, x);
        }
    }
    for &l in &fixed {
        for &x in &specials() {
            point(m, l, x);
        }
    }
    for _ in 0..1000 {
        let l = g.log_uniform(-3.0, 3.0);
        let x = g.log_uniform(-12.0, 2.0) / l;
        point(m, l, x);
    }
    for &l in fixed.iter().chain(&seeded) {
        let e = Exponential::new(l).expect("valid parameters");
        m.row("Exponential::mean", Class::A, &[f(l)], &[f(e.mean())]);
        m.row("Exponential::variance", Class::A, &[f(l)], &[f(e.variance())]);
    }
    let cases: Vec<(Vec<f64>, Exponential)> =
        fixed.iter().map(|&l| (vec![l], Exponential::new(l).expect("valid"))).collect();
    streams(m, "Exponential::sample", Class::C, &cases);
}

fn bernoulli(m: &mut Module, g: &mut Seeds) {
    let eps = f64::EPSILON;
    let mut grid = param_grid();
    grid.extend([1.0 - eps, eps, 0.7, 1.0 + eps, -eps]);
    for &p in &grid {
        match Bernoulli::new(black_box(p)) {
            Ok(b) => m.row("Bernoulli::new", Class::A, &[f(p)], &[f(b.p)]),
            Err(e) => m.row("Bernoulli::new", Class::A, &[f(p)], &[err(&e)]),
        }
    }
    let fixed: Vec<f64> = vec![0.5, 0.0, 1.0, 0.3, 0.7, 1e-300, 1.0 - eps / 2.0, f64::from_bits(1), 0.1];
    let seeded: Vec<f64> = (0..20).map(|_| g.unit()).collect();
    let mut xs: Vec<f64> = specials();
    for k in 0..=4 {
        let d = f64::from(k) * eps / 2.0;
        xs.extend([d, -d, 1.0 + d, 1.0 - d]);
    }
    xs.extend([0.5, 2.0, -1.0, 0.999_999, 1e-300, 3.0, 1.5]);
    for &p in fixed.iter().chain(&seeded) {
        let b = Bernoulli::new(p).expect("valid parameters");
        for &x in &xs {
            m.row("Bernoulli::pdf", Class::A, &[f(p), f(x)], &[f(b.pdf(black_box(x)))]);
            m.row("Bernoulli::cdf", Class::A, &[f(p), f(x)], &[f(b.cdf(black_box(x)))]);
        }
        m.row("Bernoulli::mean", Class::A, &[f(p)], &[f(b.mean())]);
        m.row("Bernoulli::variance", Class::A, &[f(p)], &[f(b.variance())]);
    }
    let cases: Vec<(Vec<f64>, Bernoulli)> =
        fixed.iter().map(|&p| (vec![p], Bernoulli::new(p).expect("valid"))).collect();
    streams(m, "Bernoulli::sample", Class::A, &cases);
}

fn cauchy(m: &mut Module, g: &mut Seeds) {
    for &x0 in &param_grid() {
        for &ga in &param_grid() {
            match Cauchy::new(black_box(x0), black_box(ga)) {
                Ok(c) => m.row("Cauchy::new", Class::A, &[f(x0), f(ga)], &[f(c.x0), f(c.gamma)]),
                Err(e) => m.row("Cauchy::new", Class::A, &[f(x0), f(ga)], &[err(&e)]),
            }
        }
    }
    let fixed: Vec<(f64, f64)> = vec![
        (0.0, 1.0),
        (2.0, 3.0),
        (-3.5, 0.25),
        (1e6, 1e-3),
        (-0.0, 1.0),
        (0.0, 1e-300),
        (0.0, 1e300),
        (f64::MAX, 1.0),
        (-f64::MAX, f64::MAX),
        (1e-300, 7.0),
        (0.0, f64::from_bits(1)),
    ];
    let seeded: Vec<(f64, f64)> =
        (0..40).map(|_| (g.signed_log_uniform(-6.0, 6.0), g.log_uniform(-4.0, 4.0))).collect();
    let point = |m: &mut Module, x0: f64, ga: f64, x: f64| {
        let c = Cauchy::new(x0, ga).expect("valid parameters");
        m.row("Cauchy::pdf", Class::A, &[f(x0), f(ga), f(x)], &[f(c.pdf(black_box(x)))]);
        m.row("Cauchy::cdf", Class::C, &[f(x0), f(ga), f(x)], &[f(c.cdf(black_box(x)))]);
    };
    let far = [1e10, 1e20, 1e154, 1.4e154, 1e300];
    for &(x0, ga) in &fixed {
        for k in -160..=160 {
            point(m, x0, ga, x0 + ga * (f64::from(k) / 4.0));
        }
        for z in far {
            point(m, x0, ga, x0 + ga * z);
            point(m, x0, ga, x0 - ga * z);
        }
        for &x in &specials() {
            point(m, x0, ga, x);
        }
    }
    for &(x0, ga) in &seeded {
        for k in -16..=16 {
            point(m, x0, ga, x0 + ga * (f64::from(k) * 2.5));
        }
    }
    for _ in 0..1000 {
        let x0 = g.signed_log_uniform(-3.0, 3.0);
        let ga = g.log_uniform(-3.0, 3.0);
        let x = x0 + ga * g.signed_log_uniform(-3.0, 6.0);
        point(m, x0, ga, x);
    }
    for &(x0, ga) in fixed.iter().chain(&seeded) {
        let c = Cauchy::new(x0, ga).expect("valid parameters");
        m.row("Cauchy::mean", Class::A, &[f(x0), f(ga)], &[f(c.mean())]);
        m.row("Cauchy::variance", Class::A, &[f(x0), f(ga)], &[f(c.variance())]);
    }
    let cases: Vec<(Vec<f64>, Cauchy)> =
        fixed.iter().map(|&(x0, ga)| (vec![x0, ga], Cauchy::new(x0, ga).expect("valid"))).collect();
    streams(m, "Cauchy::sample", Class::C, &cases);
}

fn weibull(m: &mut Module, g: &mut Seeds) {
    for &k in &param_grid() {
        for &l in &param_grid() {
            match Weibull::new(black_box(k), black_box(l)) {
                Ok(w) => m.row("Weibull::new", Class::A, &[f(k), f(l)], &[f(w.k), f(w.lambda)]),
                Err(e) => m.row("Weibull::new", Class::A, &[f(k), f(l)], &[err(&e)]),
            }
        }
    }
    let fixed: Vec<(f64, f64)> = vec![
        (1.0, 1.0),
        (2.0, 1.0),
        (0.5, 1.0),
        (1.0, 2.0),
        (2.0, 3.0),
        (1.5, 0.25),
        (5.0, 1e3),
        (0.1, 1.0),
        (10.0, 1.0),
        (100.0, 1.0),
        (2.0, 1e-300),
        (3.0, 1e300),
        (1e-3, 1.0),
        (f64::MAX, 1.0),
        (f64::from_bits(1), 1.0),
    ];
    let seeded: Vec<(f64, f64)> = (0..40).map(|_| (g.log_uniform(-1.5, 1.5), g.log_uniform(-3.0, 3.0))).collect();
    let point = |m: &mut Module, k: f64, l: f64, x: f64| {
        let w = Weibull::new(k, l).expect("valid parameters");
        m.row("Weibull::pdf", Class::C, &[f(k), f(l), f(x)], &[f(w.pdf(black_box(x)))]);
        m.row("Weibull::cdf", Class::C, &[f(k), f(l), f(x)], &[f(w.cdf(black_box(x)))]);
    };
    for &(k, l) in fixed.iter().chain(&seeded) {
        for j in -8..=80 {
            point(m, k, l, l * (f64::from(j) / 16.0));
        }
        for j in -40..=12 {
            point(m, k, l, l * f64::from(j).exp2());
        }
        for x in around(l) {
            point(m, k, l, x);
        }
        point(m, k, l, l * 1e-300);
    }
    for &(k, l) in &fixed {
        for &x in &specials() {
            point(m, k, l, x);
        }
    }
    for _ in 0..1000 {
        let k = g.log_uniform(-1.0, 1.3);
        let l = g.log_uniform(-3.0, 3.0);
        let x = l * g.log_uniform(-6.0, 1.5);
        point(m, k, l, x);
    }
    // The moments take lnΓ(1 + 1/k) and lnΓ(1 + 2/k): every parameter above, and a wider grid of shapes.
    let mut shapes: Vec<f64> = vec![
        1e-3, 0.01, 0.1, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 3.6, 10.0, 100.0, 1e3, 1e6, 1e10, 1e16, 1e300, 0.003,
        f64::MIN_POSITIVE, f64::from_bits(1), f64::MAX,
    ];
    shapes.extend((0..200).map(|_| g.log_uniform(-3.0, 6.0)));
    let mut moments: Vec<(f64, f64)> = fixed.iter().chain(&seeded).copied().collect();
    for &k in &shapes {
        for l in [1.0, 2.5, 1e-300, 1e300] {
            moments.push((k, l));
        }
    }
    for &(k, l) in &moments {
        let w = Weibull::new(k, l).expect("valid parameters");
        m.row("Weibull::mean", Class::C, &[f(k), f(l)], &[f(w.mean())]);
        m.row("Weibull::variance", Class::C, &[f(k), f(l)], &[f(w.variance())]);
    }
    let cases: Vec<(Vec<f64>, Weibull)> =
        fixed.iter().map(|&(k, l)| (vec![k, l], Weibull::new(k, l).expect("valid"))).collect();
    streams(m, "Weibull::sample", Class::C, &cases);
}

/// The x a gamma-family row is taken at, for shape a and scale b: across [0, 4·mean], at powers of two, around the mode
/// (the mean ± up to 12 standard deviations), the mean and its neighbours, and far out.
fn gamma_points(a: f64, b: f64) -> Vec<f64> {
    let mean = a * b;
    let sd = a.sqrt() * b;
    let mut xs: Vec<f64> = (-4..=64).map(|j| mean * (f64::from(j) / 16.0)).collect();
    xs.extend((-40..=12).map(|j| b * f64::from(j).exp2()));
    xs.extend((-24..=24).map(|j| mean + sd * (f64::from(j) / 2.0)));
    xs.extend(around(mean));
    xs.extend([b * 1e-300, mean * 1e3, mean * 1e10]);
    xs
}

fn gamma(m: &mut Module, g: &mut Seeds) {
    for &a in &param_grid() {
        for &b in &param_grid() {
            match Gamma::new(black_box(a), black_box(b)) {
                Ok(d) => m.row("Gamma::new", Class::A, &[f(a), f(b)], &[f(d.alpha), f(d.beta)]),
                Err(e) => m.row("Gamma::new", Class::A, &[f(a), f(b)], &[err(&e)]),
            }
        }
    }
    let fixed: Vec<(f64, f64)> = vec![
        (1.0, 1.0),
        (2.0, 1.0),
        (0.5, 1.0),
        (2.0, 3.0),
        (5.0, 0.5),
        (0.1, 1.0),
        (10.0, 2.0),
        (100.0, 1.0),
        (1e3, 1.0),
        (1e-3, 1.0),
        (1e5, 1e-3),
        (1.0, 1e-300),
        (3.0, 1e300),
        (2e9, 1.0),
        (f64::from_bits(1), 1.0),
        (f64::MAX, 1.0),
    ];
    let seeded: Vec<(f64, f64)> = (0..40).map(|_| (g.log_uniform(-2.0, 3.0), g.log_uniform(-3.0, 3.0))).collect();
    let point = |m: &mut Module, a: f64, b: f64, x: f64| {
        let d = Gamma::new(a, b).expect("valid parameters");
        m.row("Gamma::pdf", Class::C, &[f(a), f(b), f(x)], &[f(d.pdf(black_box(x)))]);
        m.row("Gamma::cdf", Class::C, &[f(a), f(b), f(x)], &[f(d.cdf(black_box(x)))]);
    };
    for &(a, b) in &fixed {
        for x in gamma_points(a, b) {
            point(m, a, b, x);
        }
        for &x in &specials() {
            point(m, a, b, x);
        }
    }
    for &(a, b) in &seeded {
        for j in -8..=8 {
            point(m, a, b, a * b + a.sqrt() * b * f64::from(j));
        }
    }
    for _ in 0..1000 {
        let a = g.log_uniform(-2.0, 3.0);
        let b = g.log_uniform(-3.0, 3.0);
        let x = b * a * g.log_uniform(-3.0, 0.7);
        point(m, a, b, x);
    }
    for &(a, b) in fixed.iter().chain(&seeded) {
        let d = Gamma::new(a, b).expect("valid parameters");
        m.row("Gamma::mean", Class::A, &[f(a), f(b)], &[f(d.mean())]);
        m.row("Gamma::variance", Class::A, &[f(a), f(b)], &[f(d.variance())]);
    }
    let cases: Vec<(Vec<f64>, Gamma)> =
        fixed.iter().map(|&(a, b)| (vec![a, b], Gamma::new(a, b).expect("valid"))).collect();
    streams(m, "Gamma::sample", Class::C, &cases);
}

fn chi_squared(m: &mut Module, g: &mut Seeds) {
    for &df in &param_grid() {
        match ChiSquared::new(black_box(df)) {
            Ok(d) => m.row("ChiSquared::new", Class::A, &[f(df)], &[f(d.df)]),
            Err(e) => m.row("ChiSquared::new", Class::A, &[f(df)], &[err(&e)]),
        }
    }
    let fixed: Vec<f64> =
        vec![2.0, 1.0, 3.0, 4.0, 5.0, 10.0, 30.0, 100.0, 1e3, 1e5, 0.5, 0.1, 1e-3, 4e9, 1e-300, f64::from_bits(1)];
    let seeded: Vec<f64> = (0..40).map(|_| g.log_uniform(-2.0, 4.0)).collect();
    let point = |m: &mut Module, df: f64, x: f64| {
        let d = ChiSquared::new(df).expect("valid parameters");
        m.row("ChiSquared::pdf", Class::C, &[f(df), f(x)], &[f(d.pdf(black_box(x)))]);
        m.row("ChiSquared::cdf", Class::C, &[f(df), f(x)], &[f(d.cdf(black_box(x)))]);
    };
    for &df in &fixed {
        for x in gamma_points(df / 2.0, 2.0) {
            point(m, df, x);
        }
        for &x in &specials() {
            point(m, df, x);
        }
    }
    for &df in &seeded {
        for j in -8..=8 {
            point(m, df, df + (2.0 * df).sqrt() * f64::from(j));
        }
    }
    for _ in 0..1000 {
        let df = g.log_uniform(-2.0, 4.0);
        let x = df * g.log_uniform(-3.0, 0.7);
        point(m, df, x);
    }
    for &df in fixed.iter().chain(&seeded) {
        let d = ChiSquared::new(df).expect("valid parameters");
        m.row("ChiSquared::mean", Class::A, &[f(df)], &[f(d.mean())]);
        m.row("ChiSquared::variance", Class::A, &[f(df)], &[f(d.variance())]);
    }
    let cases: Vec<(Vec<f64>, ChiSquared)> =
        fixed.iter().map(|&df| (vec![df], ChiSquared::new(df).expect("valid"))).collect();
    streams(m, "ChiSquared::sample", Class::C, &cases);
}

/// The oracle's Marsaglia–Tsang hook, at shapes either side of the boost (α < 1) and up to 1e300.
#[allow(clippy::cast_possible_wrap)]
fn gamma_standard(m: &mut Module) {
    let shapes = [1e-3, 0.1, 0.5, 0.999_999, 1.0, 1.5, 2.5, 10.0, 1e3, 1e9, 1e300];
    for seed in [1u64, 42, u64::MAX] {
        for &a in &shapes {
            let mut r = SimpleRng::new(seed);
            let xs: Vec<f64> = (0..1024).map(|_| oracle::sample_gamma_standard(a, &mut r)).collect();
            m.row("sample_gamma_standard", Class::C, &[f(a), i(seed as i64), i(1024)], &draws(&xs));
        }
    }
}

fn beta(m: &mut Module, g: &mut Seeds) {
    for &a in &param_grid() {
        for &b in &param_grid() {
            match Beta::new(black_box(a), black_box(b)) {
                Ok(d) => m.row("Beta::new", Class::A, &[f(a), f(b)], &[f(d.alpha), f(d.beta)]),
                Err(e) => m.row("Beta::new", Class::A, &[f(a), f(b)], &[err(&e)]),
            }
        }
    }
    let fixed: Vec<(f64, f64)> = vec![
        (2.0, 5.0),
        (1.0, 1.0),
        (0.5, 0.5),
        (2.0, 2.0),
        (5.0, 1.0),
        (0.1, 0.1),
        (30.0, 0.7),
        (400.0, 350.0),
        (1e3, 1e3),
        (1e-3, 1.0),
        (1.0, 1e5),
        (1e5, 1e5),
        (2e9, 3.0),
        (f64::from_bits(1), 1.0),
        (f64::MAX, 1.0),
    ];
    let seeded: Vec<(f64, f64)> = (0..40).map(|_| (g.log_uniform(-2.0, 3.0), g.log_uniform(-2.0, 3.0))).collect();
    let point = |m: &mut Module, a: f64, b: f64, x: f64| {
        let d = Beta::new(a, b).expect("valid parameters");
        m.row("Beta::pdf", Class::C, &[f(a), f(b), f(x)], &[f(d.pdf(black_box(x)))]);
        m.row("Beta::cdf", Class::C, &[f(a), f(b), f(x)], &[f(d.cdf(black_box(x)))]);
    };
    for &(a, b) in &fixed {
        let mean = a / (a + b);
        let sd = (a * b / ((a + b) * (a + b) * (a + b + 1.0))).sqrt();
        let mut xs: Vec<f64> = (-4..=68).map(|j| f64::from(j) / 64.0).collect();
        xs.extend((1..=60).flat_map(|j| [f64::from(-j).exp2(), 1.0 - f64::from(-j).exp2()]));
        xs.extend((-16..=16).map(|j| mean + sd * (f64::from(j) / 2.0)));
        xs.extend(around(0.0).into_iter().chain(around(1.0)));
        xs.extend(specials());
        for x in xs {
            point(m, a, b, x);
        }
    }
    for &(a, b) in &seeded {
        for j in 0..=16 {
            point(m, a, b, f64::from(j) / 16.0);
        }
    }
    for _ in 0..1000 {
        let a = g.log_uniform(-2.0, 3.0);
        let b = g.log_uniform(-2.0, 3.0);
        let x = g.unit();
        point(m, a, b, x);
    }
    for &(a, b) in fixed.iter().chain(&seeded) {
        let d = Beta::new(a, b).expect("valid parameters");
        m.row("Beta::mean", Class::A, &[f(a), f(b)], &[f(d.mean())]);
        m.row("Beta::variance", Class::A, &[f(a), f(b)], &[f(d.variance())]);
    }
    let cases: Vec<(Vec<f64>, Beta)> =
        fixed.iter().map(|&(a, b)| (vec![a, b], Beta::new(a, b).expect("valid"))).collect();
    streams(m, "Beta::sample", Class::C, &cases);
}

/// Student-t's points: |t| ≤ 20 by quarters, far out (t² overflows from 1.35e154), and the specials.
fn t_points() -> Vec<f64> {
    let mut ts: Vec<f64> = (-80..=80).map(|k| f64::from(k) / 4.0).collect();
    for z in [1e3, 1e10, 1e20, 1e155, 1e300] {
        ts.extend([z, -z]);
    }
    ts.extend(specials());
    ts
}

fn student_t(m: &mut Module, g: &mut Seeds) {
    for &df in &param_grid() {
        match StudentT::new(black_box(df)) {
            Ok(d) => m.row("StudentT::new", Class::A, &[f(df)], &[f(d.df)]),
            Err(e) => m.row("StudentT::new", Class::A, &[f(df)], &[err(&e)]),
        }
    }
    let fixed: Vec<f64> = vec![
        3.0,
        1.0,
        2.0,
        3.5,
        5.0,
        10.0,
        19.999_999_999_999_996,
        20.0,
        30.0,
        100.0,
        1e3,
        1e6,
        0.5,
        0.1,
        1e-3,
        2e9,
        3e9,
        f64::from_bits(1),
    ];
    let seeded: Vec<f64> = (0..40).map(|_| g.log_uniform(-1.0, 4.0)).collect();
    let point = |m: &mut Module, df: f64, x: f64| {
        let d = StudentT::new(df).expect("valid parameters");
        m.row("StudentT::pdf", Class::C, &[f(df), f(x)], &[f(d.pdf(black_box(x)))]);
        m.row("StudentT::cdf", Class::C, &[f(df), f(x)], &[f(d.cdf(black_box(x)))]);
    };
    for &df in &fixed {
        for x in t_points() {
            point(m, df, x);
        }
    }
    for &df in &seeded {
        for k in -16..=16 {
            point(m, df, f64::from(k) / 2.0);
        }
    }
    for _ in 0..1000 {
        let df = g.log_uniform(-1.0, 4.0);
        let x = g.signed_log_uniform(-3.0, 3.0);
        point(m, df, x);
    }
    for &df in fixed.iter().chain(&seeded) {
        let d = StudentT::new(df).expect("valid parameters");
        m.row("StudentT::mean", Class::A, &[f(df)], &[f(d.mean())]);
        m.row("StudentT::variance", Class::A, &[f(df)], &[f(d.variance())]);
    }
    let cases: Vec<(Vec<f64>, StudentT)> =
        fixed.iter().map(|&df| (vec![df], StudentT::new(df).expect("valid"))).collect();
    streams(m, "StudentT::sample", Class::C, &cases);
}

fn f_distribution(m: &mut Module, g: &mut Seeds) {
    for &d1 in &param_grid() {
        for &d2 in &param_grid() {
            match FDistribution::new(black_box(d1), black_box(d2)) {
                Ok(d) => m.row("FDistribution::new", Class::A, &[f(d1), f(d2)], &[f(d.d1), f(d.d2)]),
                Err(e) => m.row("FDistribution::new", Class::A, &[f(d1), f(d2)], &[err(&e)]),
            }
        }
    }
    let fixed: Vec<(f64, f64)> = vec![
        (4.0, 9.0),
        (1.0, 1.0),
        (2.0, 2.0),
        (5.0, 2.0),
        (1.0, 30.0),
        (30.0, 1.0),
        (10.0, 20.0),
        (100.0, 100.0),
        (1e3, 1e4),
        (0.5, 0.5),
        (0.1, 3.0),
        (1e5, 1e5),
        (2e9, 5.0),
        (3.0, 4.5),
        (3.0, 4.0),
        (f64::from_bits(1), 1.0),
    ];
    let seeded: Vec<(f64, f64)> = (0..40).map(|_| (g.log_uniform(-1.0, 3.0), g.log_uniform(-1.0, 3.0))).collect();
    let point = |m: &mut Module, d1: f64, d2: f64, x: f64| {
        let d = FDistribution::new(d1, d2).expect("valid parameters");
        m.row("FDistribution::pdf", Class::C, &[f(d1), f(d2), f(x)], &[f(d.pdf(black_box(x)))]);
        m.row("FDistribution::cdf", Class::C, &[f(d1), f(d2), f(x)], &[f(d.cdf(black_box(x)))]);
    };
    for &(d1, d2) in &fixed {
        let mut xs: Vec<f64> = (-4..=96).map(|j| f64::from(j) / 16.0).collect();
        xs.extend((-40..=20).map(|j| f64::from(j).exp2()));
        xs.extend([1e10, 1e300]);
        xs.extend(specials());
        for x in xs {
            point(m, d1, d2, x);
        }
    }
    for &(d1, d2) in &seeded {
        for j in 0..=16 {
            point(m, d1, d2, f64::from(j) / 4.0);
        }
    }
    for _ in 0..1000 {
        let d1 = g.log_uniform(-1.0, 3.0);
        let d2 = g.log_uniform(-1.0, 3.0);
        let x = g.log_uniform(-3.0, 2.0);
        point(m, d1, d2, x);
    }
    for &(d1, d2) in fixed.iter().chain(&seeded) {
        let d = FDistribution::new(d1, d2).expect("valid parameters");
        m.row("FDistribution::mean", Class::A, &[f(d1), f(d2)], &[f(d.mean())]);
        m.row("FDistribution::variance", Class::A, &[f(d1), f(d2)], &[f(d.variance())]);
    }
    let cases: Vec<(Vec<f64>, FDistribution)> =
        fixed.iter().map(|&(d1, d2)| (vec![d1, d2], FDistribution::new(d1, d2).expect("valid"))).collect();
    streams(m, "FDistribution::sample", Class::C, &cases);
}

/// The oracle's Student-t and F tails on their own grids: the hypothesis tests (M4) call them directly, and
/// `f_cdf_sf`'s survival is no public method's.
fn tails(m: &mut Module, g: &mut Seeds) {
    for df in [0.5, 1.0, 3.5, 30.0, 1e6, 2e9] {
        for t in t_points() {
            m.row("student_t_two_tailed", Class::C, &[f(t), f(df)], &[f(oracle::student_t_two_tailed(t, df))]);
            m.row("student_t_cdf", Class::C, &[f(t), f(df)], &[f(oracle::student_t_cdf(t, df))]);
        }
    }
    for &df in &specials() {
        for t in [0.0, 1.5, -2.0, f64::NAN] {
            m.row("student_t_two_tailed", Class::C, &[f(t), f(df)], &[f(oracle::student_t_two_tailed(t, df))]);
        }
    }
    for _ in 0..500 {
        let df = g.log_uniform(-1.0, 5.0);
        let t = g.signed_log_uniform(-3.0, 3.0);
        m.row("student_t_two_tailed", Class::C, &[f(t), f(df)], &[f(oracle::student_t_two_tailed(t, df))]);
    }
    let pairs = [(4.0, 9.0), (1.0, 1.0), (0.5, 30.0), (100.0, 3.0), (1e4, 1e4)];
    for (d1, d2) in pairs {
        let mut xs: Vec<f64> = (-4..=96).map(|j| f64::from(j) / 16.0).collect();
        xs.extend((-40..=20).map(|j| f64::from(j).exp2()));
        xs.extend(specials());
        for x in xs {
            let (c, sf) = oracle::f_cdf_sf(x, d1, d2);
            m.row("f_cdf_sf", Class::C, &[f(x), f(d1), f(d2)], &[f(c), f(sf)]);
        }
    }
    for _ in 0..500 {
        let d1 = g.log_uniform(-1.0, 4.0);
        let d2 = g.log_uniform(-1.0, 4.0);
        let x = g.log_uniform(-4.0, 3.0);
        let (c, sf) = oracle::f_cdf_sf(x, d1, d2);
        m.row("f_cdf_sf", Class::C, &[f(x), f(d1), f(d2)], &[f(c), f(sf)]);
    }
}

fn poisson(m: &mut Module, g: &mut Seeds) {
    for &l in &param_grid() {
        match Poisson::new(black_box(l)) {
            Ok(d) => m.row("Poisson::new", Class::A, &[f(l)], &[f(d.lambda)]),
            Err(e) => m.row("Poisson::new", Class::A, &[f(l)], &[err(&e)]),
        }
    }
    let fixed: Vec<f64> = vec![
        2.5,
        1.0,
        0.5,
        3.5,
        10.0,
        29.999_999_999_999_996,
        30.0,
        31.0,
        100.0,
        1e3,
        1e6,
        1e12,
        1e-3,
        1e-300,
        f64::from_bits(1),
        f64::MAX,
    ];
    let seeded: Vec<f64> = (0..40).map(|_| g.log_uniform(-2.0, 4.0)).collect();
    let point = |m: &mut Module, l: f64, x: f64| {
        let d = Poisson::new(l).expect("valid parameters");
        m.row("Poisson::pdf", Class::C, &[f(l), f(x)], &[f(d.pdf(black_box(x)))]);
        m.row("Poisson::cdf", Class::C, &[f(l), f(x)], &[f(d.cdf(black_box(x)))]);
    };
    for &l in &fixed {
        let sd = l.sqrt();
        let mut xs: Vec<f64> = (0..=60).map(f64::from).collect();
        xs.extend((-24..=24).map(|j| (l + sd * (f64::from(j) / 2.0)).floor()));
        xs.extend([0.5, 2.5, 3.000_000_000_000_000_4, 2.999_999_999_999_999_6, -0.5, -1.0, -0.0]);
        xs.extend([l, l.floor(), l + 0.5, 1e15, 9_007_199_254_740_992.0, 1e300]);
        xs.extend(specials());
        for x in xs {
            point(m, l, x);
        }
    }
    for &l in &seeded {
        let sd = l.sqrt();
        for j in -12..=12 {
            point(m, l, (l + sd * (f64::from(j) / 2.0)).floor());
        }
    }
    for k in 0..1000 {
        let l = g.log_uniform(-2.0, 5.0);
        let x = (l * g.log_uniform(-2.0, 0.5)).floor();
        point(m, l, if k % 10 == 0 { x + 0.25 } else { x });
    }
    for &l in fixed.iter().chain(&seeded) {
        let d = Poisson::new(l).expect("valid parameters");
        m.row("Poisson::mean", Class::A, &[f(l)], &[f(d.mean())]);
        m.row("Poisson::variance", Class::A, &[f(l)], &[f(d.variance())]);
    }
    let cases: Vec<(Vec<f64>, Poisson)> =
        fixed.iter().map(|&l| (vec![l], Poisson::new(l).expect("valid"))).collect();
    streams(m, "Poisson::sample", Class::C, &cases);
}

/// The oracle's PTRS hook at rates from its 10 up, past the sampler's switch, to 1e15.
#[allow(clippy::cast_possible_wrap)]
fn poisson_ptrs(m: &mut Module) {
    let rates = [10.0, 15.0, 30.0, 31.0, 100.0, 1e4, 1e8, 1e12, 1e15];
    for seed in [1u64, 42, u64::MAX] {
        for &l in &rates {
            let mut r = SimpleRng::new(seed);
            let xs: Vec<f64> = (0..1024).map(|_| oracle::sample_poisson_ptrs(l, &mut r)).collect();
            m.row("sample_poisson_ptrs", Class::C, &[f(l), i(seed as i64), i(1024)], &draws(&xs));
        }
    }
}

#[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
fn binomial(m: &mut Module, g: &mut Seeds) {
    let ns: [u64; 11] = [0, 1, 2, 10, 64, 65, 1000, 1 << 53, (1 << 53) + 1, 1 << 63, u64::MAX];
    let eps = f64::EPSILON;
    let mut ps = param_grid();
    ps.extend([1.0 - eps / 2.0, 1.0 + eps, 0.3]);
    for &n in &ns {
        for &p in &ps {
            match Binomial::new(black_box(n), black_box(p)) {
                Ok(d) => m.row("Binomial::new", Class::A, &[i(n as i64), f(p)], &[i(d.n as i64), f(d.p)]),
                Err(e) => m.row("Binomial::new", Class::A, &[i(n as i64), f(p)], &[err(&e)]),
            }
        }
    }
    let fixed: Vec<(u64, f64)> = vec![
        (10, 0.3),
        (1, 0.5),
        (2, 0.5),
        (20, 0.4),
        (64, 0.5),
        (65, 0.5),
        (100, 0.01),
        (1000, 0.5),
        (1_000_000, 0.3),
        (1_000_000_000_000, 0.5),
        ((1 << 53) + 1, 0.5),
        (u64::MAX, 0.5),
        (10, 0.0),
        (10, 1.0),
        (50, 1e-300),
        (50, 1.0 - eps / 2.0),
        (30, f64::from_bits(1)),
    ];
    let seeded: Vec<(u64, f64)> = (0..40).map(|_| (g.log_uniform(0.0, 4.0) as u64 + 1, g.unit())).collect();
    let point = |m: &mut Module, n: u64, p: f64, x: f64| {
        let d = Binomial::new(n, p).expect("valid parameters");
        m.row("Binomial::pdf", Class::C, &[i(n as i64), f(p), f(x)], &[f(d.pdf(black_box(x)))]);
        m.row("Binomial::cdf", Class::C, &[i(n as i64), f(p), f(x)], &[f(d.cdf(black_box(x)))]);
    };
    for &(n, p) in &fixed {
        let nf = n as f64;
        let mean = nf * p;
        let sd = (nf * p * (1.0 - p)).sqrt();
        let mut xs: Vec<f64> = (0..=40).map(f64::from).filter(|&k| k <= nf).collect();
        xs.extend((0..=10).map(|k| nf - f64::from(k)).filter(|&k| k >= 0.0));
        xs.extend((-24..=24).map(|j| (mean + sd * (f64::from(j) / 2.0)).floor()));
        xs.extend([nf, nf + 1.0, 0.5, 2.5, 3.000_000_000_000_000_4, -1.0, -0.0, mean, 1e300]);
        xs.extend(specials());
        for x in xs {
            point(m, n, p, x);
        }
    }
    for &(n, p) in &seeded {
        let nf = n as f64;
        let sd = (nf * p * (1.0 - p)).sqrt();
        for j in -12..=12 {
            point(m, n, p, (nf * p + sd * (f64::from(j) / 2.0)).floor());
        }
    }
    for k in 0..1000 {
        let n = g.log_uniform(0.0, 5.0) as u64 + 1;
        let p = g.unit();
        let x = (n as f64 * g.unit()).floor();
        point(m, n, p, if k % 10 == 0 { x + 0.5 } else { x });
    }
    for &(n, p) in fixed.iter().chain(&seeded) {
        let d = Binomial::new(n, p).expect("valid parameters");
        m.row("Binomial::mean", Class::A, &[i(n as i64), f(p)], &[f(d.mean())]);
        m.row("Binomial::variance", Class::A, &[i(n as i64), f(p)], &[f(d.variance())]);
    }
    for &seed in &SEEDS {
        for (k, &(n, p)) in fixed.iter().enumerate() {
            let count = if k == 0 { 10_000 } else { 256 };
            let d = Binomial::new(n, p).expect("valid parameters");
            let mut r = SimpleRng::new(seed);
            let xs: Vec<f64> = (0..count).map(|_| d.sample(&mut r)).collect();
            m.row("Binomial::sample", Class::C, &[i(n as i64), f(p), i(seed as i64), i(count as i64)], &draws(&xs));
        }
    }
}

pub fn emit() -> Module {
    let mut m = Module::new("dist");
    let mut g = Seeds::new(0x510e_527f_ade6_82d1);
    normal(&mut m, &mut g);
    uniform(&mut m, &mut g);
    exponential(&mut m, &mut g);
    bernoulli(&mut m, &mut g);
    cauchy(&mut m, &mut g);
    weibull(&mut m, &mut g);
    gamma(&mut m, &mut g);
    chi_squared(&mut m, &mut g);
    gamma_standard(&mut m);
    beta(&mut m, &mut g);
    student_t(&mut m, &mut g);
    f_distribution(&mut m, &mut g);
    tails(&mut m, &mut g);
    poisson(&mut m, &mut g);
    poisson_ptrs(&mut m);
    binomial(&mut m, &mut g);
    crate::m_mvn::emit(&mut m, &mut g);
    m
}
