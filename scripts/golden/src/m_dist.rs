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
//! Inputs: the parameters, then x (or the seed and the count). Outputs: the value, or the draws as a sequence.

use std::hint::black_box;

use crate::grids::{Rng as Seeds, specials};
use crate::tsv::{Class, Module, Tok, err, f, i};
use pramana::distribution::{Bernoulli, Cauchy, Distribution, Exponential, Normal, Uniform, Weibull};
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

pub fn emit() -> Module {
    let mut m = Module::new("dist");
    let mut g = Seeds::new(0x510e_527f_ade6_82d1);
    normal(&mut m, &mut g);
    uniform(&mut m, &mut g);
    exponential(&mut m, &mut g);
    bernoulli(&mut m, &mut g);
    cauchy(&mut m, &mut g);
    weibull(&mut m, &mut g);
    m
}
