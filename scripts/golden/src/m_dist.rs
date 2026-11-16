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
//! Inputs: the parameters, then x (or the seed and the count). Outputs: the value, or the draws as a sequence.

use std::hint::black_box;

use crate::grids::{Rng as Seeds, specials};
use crate::tsv::{Class, Module, Tok, err, f, i};
use pramana::distribution::{Distribution, Normal};
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

pub fn emit() -> Module {
    let mut m = Module::new("dist");
    let mut g = Seeds::new(0x510e_527f_ade6_82d1);
    normal(&mut m, &mut g);
    m
}
