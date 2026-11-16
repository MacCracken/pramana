//! `special.tsv` — 1.3.0's special functions, `src/math.rs`, through the hidden `oracle` hooks (M2; port plan D3,
//! §3.4, ADR 009). The rows are named by Rust's module, `math::<fn>`, though the file is `special` (the port's
//! `src/special.cyr`), since `math.tsv` already holds the std f64 operations.
//!
//! Part a1, the error function family:
//! - `erf(x)` and `erfc(x)` on the specials; every high-word threshold fdlibm branches on (2^-56, 2^-28, 1/4,
//!   0.84375, 1.25, 1/0.35, 6, 28), with the five doubles either side and 24 seeded low words in the high word at and
//!   below it, both signs; seeded sweeps of every branch's interval, both signs; tiny and wide log-uniform sweeps;
//!   and the grid k/32 on [−30, 30].
//! - `normal_cdf(z)` and `normal_sf(z)` on that grid times √2 (so −z/√2 lands on or next to every threshold), the
//!   grid k/16 on [−40, 40] (Φ underflows below −38.4), and a seeded sweep of [−40, 40].
//!
//! Part a2, lnΓ and its relatives:
//! - `ln_gamma(x)`: the reflection's switch at ½ and its 1e-300 cutoff (x = 1e-300/π with 40 doubles either side, both
//!   signs), the negative integers and half-integers with their neighbours, the roots at 1 and 2, k/16 on [−20, 30],
//!   the integers to 200, seeded sweeps of [−10, ½), [½, 10) and [10, 200), and log-uniform sweeps to 10^306 and to
//!   −10^15.
//! - `stirling_error(a)`: the switch at 10 with its neighbours, the integers to 100, seeded sweeps either side of 10,
//!   and log-uniform sweeps from 10^-310 to 10^300.
//! - `bd0(x, m)`: x = 0; x = m·r for twelve ratios r from 0 to 1000 on 300 seeded m in [10^-3, 10^12]; x a few
//!   doubles from m (u tiny); integer counts against seeded means; and independent seeded pairs.
//! - `ln_gamma_kernel(a, x)`: the switch at a = 10 with its neighbours; x = a·r for nine ratios on 300 seeded a in
//!   [10^-3, 10^12]; x = a(1 + k/64) near the mode; seeded pairs.
//! - `ln_beta(a, b)`: a grid of 19 shapes from 10^-3 to 10^12 (10 with its neighbours among them) taken pairwise, so
//!   every branch of R's lbeta scheme is hit from both sides; seeded pairs, 1,000 of them with both shapes in
//!   [10, 60), where the Stirling corrections are largest against the result; and NaN, ±0 and negative shapes, which
//!   pin Rust's NaN-ignoring min and max.
//!
//! Part a3, the regularized incomplete gamma (`gamma_inc(a, x)`, two outputs, P then Q):
//! - the specials in either argument;
//! - the switch x = a + 1, with three doubles either side, on 20 shapes from 10^-3 to 10^6;
//! - the Wilson–Hilferty switch: a = 10^9 with its neighbours and a(1 ± 10^-12), against x = a + j√a;
//! - 50 seeded shapes past 10^9 (to 10^12) against x = a + j√a, |j| ≤ 6;
//! - x = a·r for eleven ratios r from 10^-3 to 100 on 300 seeded a in [10^-3, 10^9];
//! - x = a + j√a, |j| ≤ 4, near the mode, on 60 seeded a in [1, 10^9], where the loops run longest;
//! - the χ² shapes a = k/2 (k to 120) against x = a·r, and the prefactor's underflow at both ends (x to 10^4, and x
//!   down to the smallest subnormal);
//! - shapes from 10^-300 to 10^-8 against x up to 3, where P below x = a + 1 is within about 0.2a of 1, so the series'
//!   clamp to [0, 1] can bind;
//! - seeded pairs.
//!
//! `regularized_lower_incomplete_gamma` and `regularized_upper_incomplete_gamma` get 400 seeded pairs each. √a is
//! correctly rounded (IEEE 754), as × and ÷ are, so it may build a grid.
//!
//! Part a4, the regularized incomplete beta (`beta_inc(a, b, x, y)`, two outputs, I then 1 − I):
//! - the specials in each of the four arguments;
//! - 17 shapes from 10^-3 to 10^5 (10 with its neighbours) taken pairwise, against eleven x from 10^-10 to 1 − 10^-10;
//! - the side switch x = (a + 1)/(a + b + 2), with two doubles either side, on 100 seeded shape pairs;
//! - both shapes from 10 to 10^6 (the bd0 prefactor) with x at the mean ± j standard deviations;
//! - the callers' forms, with x and y each computed exactly rather than y = 1 − x: Student's t (ν/2, ½, ν/(ν + t²),
//!   t²/(ν + t²)) and F with two numerator degrees of freedom (1, d/2, 2x/(2x + d), d/(2x + d));
//! - y not equal to 1 − x, which 1.3.0 does not check;
//! - shapes down to 10^-300, and shapes large enough that the prefactor underflows;
//! - seeded rows.
//!
//! `regularized_incomplete_beta(x, a, b)` gets 400 seeded rows and the out-of-range x.
//!
//! Every row is class C: each function calls glibc's exp, ln, ln_1p or sin. `validate.py` transcribes them,
//! re-derives every row through glibc, and writes `special.cr.tsv` with the rows whose value changes when every
//! primitive is correctly rounded.

use std::hint::black_box;

use crate::grids::{Rng, pow2, specials};
use crate::tsv::{Class, Module, f};
use pramana::oracle;

/// fdlibm's high-word branch points in erf and erfc.
const THRESHOLDS: [u64; 8] = [
    0x3c70_0000, // 2^-56: erfc's 1 − x
    0x3e30_0000, // 2^-28: erf's tiny branch
    0x3fd0_0000, // 1/4: erfc's two small forms
    0x3feb_0000, // 0.84375: the small rational ends
    0x3ff4_0000, // 1.25: erfc_mid ends
    0x4006_db6d, // ≈ 1/0.35: the RA/SA and RB/SB tails
    0x4018_0000, // 6: erf saturates
    0x403c_0000, // 28: erfc saturates
];

/// The branch intervals' bounds, for the seeded sweeps: [2^-28, 1/4), …, [28, 40). 20/7 is 1/0.35, correctly rounded.
fn intervals() -> [(f64, f64); 7] {
    let tail = 20.0 / 7.0;
    [(pow2(-28), 0.25), (0.25, 0.843_75), (0.843_75, 1.25), (1.25, tail), (tail, 6.0), (6.0, 28.0), (28.0, 40.0)]
}

fn sort_dedup(v: &mut Vec<f64>) {
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| a.to_bits() == b.to_bits());
}

fn erf_grid() -> Vec<f64> {
    let mut v = specials();
    for &t in &THRESHOLDS {
        let at = t << 32;
        for k in 0..=4 {
            for b in [at + k, at - 1 - k] {
                let x = f64::from_bits(b);
                v.push(x);
                v.push(-x);
            }
        }
    }
    let mut g = Rng::new(0x51ec_1a1f_e7f0_a1b3);
    for &t in &THRESHOLDS {
        for hw in [t, t - 1] {
            for _ in 0..24 {
                let x = f64::from_bits((hw << 32) | (g.word() & 0xffff_ffff));
                v.push(x);
                v.push(-x);
            }
        }
    }
    for (lo, hi) in intervals() {
        for _ in 0..200 {
            let x = g.uniform(lo, hi);
            v.push(x);
            v.push(-x);
        }
    }
    for _ in 0..200 {
        v.push(g.signed_log_uniform(-17.0, -8.0));
        v.push(g.signed_log_uniform(-320.0, -17.0));
        v.push(g.signed_log_uniform(-300.0, 300.0));
    }
    for k in -960..=960 {
        v.push(f64::from(k) / 32.0);
    }
    v.push(pow2(-1074));
    sort_dedup(&mut v);
    v
}

fn normal_grid(xs: &[f64]) -> Vec<f64> {
    let mut v: Vec<f64> = xs.iter().map(|&x| x * std::f64::consts::SQRT_2).collect();
    for k in -640..=640 {
        v.push(f64::from(k) / 16.0);
    }
    let mut g = Rng::new(0x0c3d_f00d_5eed_2b1a);
    for _ in 0..400 {
        v.push(g.uniform(-40.0, 40.0));
    }
    sort_dedup(&mut v);
    v
}

/// x and the `k` doubles on either side of it, by bit pattern.
fn around(v: &mut Vec<f64>, x: f64, k: u64) {
    let b = x.to_bits();
    for d in 0..=k {
        v.push(f64::from_bits(b + d));
        if b & 0x7fff_ffff_ffff_ffff >= d {
            v.push(f64::from_bits(b - d));
        }
    }
}

fn ln_gamma_grid() -> Vec<f64> {
    let mut v = specials();
    around(&mut v, 0.5, 8);
    let cut = 1e-300 / std::f64::consts::PI;
    around(&mut v, cut, 40);
    around(&mut v, -cut, 40);
    for n in 0..=30 {
        let x = -f64::from(n);
        around(&mut v, x, 4);
        around(&mut v, x - 0.5, 2);
    }
    around(&mut v, 1.0, 8);
    around(&mut v, 2.0, 8);
    for k in -320..=480 {
        v.push(f64::from(k) / 16.0);
    }
    for n in 1..=200 {
        v.push(f64::from(n));
    }
    let mut g = Rng::new(0x6a09_e667_f3bc_c908);
    for _ in 0..400 {
        v.push(g.uniform(-10.0, 0.5));
        v.push(g.uniform(0.5, 10.0));
        v.push(g.uniform(10.0, 200.0));
    }
    for _ in 0..300 {
        v.push(g.log_uniform(-310.0, 306.0));
        v.push(-g.log_uniform(-310.0, 15.0));
    }
    sort_dedup(&mut v);
    v
}

fn stirling_grid() -> Vec<f64> {
    let mut v = specials();
    around(&mut v, 10.0, 8);
    for n in 1..=100 {
        v.push(f64::from(n));
    }
    let mut g = Rng::new(0xbb67_ae85_84ca_a73b);
    for _ in 0..300 {
        v.push(g.uniform(0.5, 10.0));
        v.push(g.uniform(10.0, 1000.0));
        v.push(g.log_uniform(-3.0, 300.0));
    }
    for _ in 0..100 {
        v.push(g.log_uniform(-310.0, -3.0));
        v.push(g.uniform(-10.0, 0.0));
    }
    sort_dedup(&mut v);
    v
}

/// Pairs over two lists, every combination.
fn cross(xs: &[f64], ys: &[f64]) -> Vec<(f64, f64)> {
    xs.iter().flat_map(|&x| ys.iter().map(move |&y| (x, y))).collect()
}

fn bd0_pairs() -> Vec<(f64, f64)> {
    let mut v = cross(&specials(), &[1.0, 2.5]);
    v.extend(cross(&[0.0, 1.0, 2.5], &specials()));
    let mut g = Rng::new(0x3c6e_f372_fe94_f82b);
    let ratios = [0.0, 0.001, 0.1, 0.5, 0.9, 0.99, 1.0, 1.01, 1.1, 2.0, 10.0, 1000.0];
    for _ in 0..300 {
        let m = g.log_uniform(-3.0, 12.0);
        v.extend(ratios.iter().map(|&r| (m * r, m)));
    }
    for _ in 0..100 {
        let m = g.log_uniform(-3.0, 12.0);
        let b = m.to_bits();
        for d in 1..=4 {
            v.push((f64::from_bits(b + d), m));
            v.push((f64::from_bits(b - d), m));
        }
    }
    for _ in 0..300 {
        let k = g.int(0, 100);
        #[allow(clippy::cast_precision_loss)]
        v.push((k as f64, g.uniform(0.1, 100.0)));
    }
    for _ in 0..300 {
        v.push((g.log_uniform(-3.0, 12.0), g.log_uniform(-3.0, 12.0)));
    }
    v
}

fn kernel_pairs() -> Vec<(f64, f64)> {
    let mut v = cross(&specials(), &[1.0, 15.0]);
    v.extend(cross(&[1.0, 15.0], &specials()));
    let mut tens = Vec::new();
    around(&mut tens, 10.0, 6);
    v.extend(cross(&tens, &[1.0, 10.0, 15.0]));
    let mut g = Rng::new(0xa54f_f53a_5f1d_36f1);
    let ratios = [0.001, 0.1, 0.5, 0.9, 1.0, 1.1, 2.0, 10.0, 1000.0];
    for _ in 0..300 {
        let a = g.log_uniform(-3.0, 12.0);
        v.extend(ratios.iter().map(|&r| (a, a * r)));
    }
    for _ in 0..100 {
        let a = g.log_uniform(-3.0, 12.0);
        for k in -8..=8 {
            v.push((a, a * (1.0 + f64::from(k) / 64.0)));
        }
    }
    for _ in 0..300 {
        v.push((g.log_uniform(-3.0, 12.0), g.log_uniform(-3.0, 12.0)));
    }
    v
}

fn ln_beta_pairs() -> Vec<(f64, f64)> {
    let mut shapes = vec![0.001, 0.01, 0.1, 0.5, 1.0, 1.5, 2.0, 3.0, 5.0, 9.5, 10.5, 20.0, 100.0, 1e3, 1e6, 1e9, 1e12];
    around(&mut shapes, 10.0, 1);
    sort_dedup(&mut shapes);
    let mut v = cross(&shapes, &shapes);
    v.extend(cross(&specials(), &[1.0, 15.0]));
    v.extend(cross(&[1.0, 15.0], &specials()));
    v.extend([(0.0, -0.0), (-0.0, 0.0), (-0.5, 2.0), (-2.5, 12.0), (12.0, -2.5), (-15.5, 30.0)]);
    let mut g = Rng::new(0x510e_527f_ade6_82d1);
    for _ in 0..600 {
        v.push((g.log_uniform(-3.0, 12.0), g.log_uniform(-3.0, 12.0)));
    }
    for _ in 0..300 {
        v.push((g.uniform(0.5, 20.0), g.uniform(0.5, 20.0)));
    }
    for _ in 0..1000 {
        v.push((g.uniform(10.0, 60.0), g.uniform(10.0, 60.0)));
    }
    v
}

fn gamma_pairs() -> Vec<(f64, f64)> {
    let mids = [0.5, 1.0, 30.0];
    let mut v = cross(&specials(), &mids);
    v.extend(cross(&mids, &specials()));
    let shapes = [
        0.001, 0.01, 0.1, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 5.0, 9.5, 10.0, 10.5, 20.0, 50.0, 100.0, 1e3, 1e4, 1e5, 1e6,
    ];
    for &a in &shapes {
        let mut xs = Vec::new();
        around(&mut xs, a + 1.0, 3);
        v.extend(xs.iter().map(|&x| (a, x)));
    }
    let wh = 1e9;
    let mut was = Vec::new();
    around(&mut was, wh, 2);
    was.push(wh * (1.0 - 1e-12));
    was.push(wh * (1.0 + 1e-12));
    for &a in &was {
        for j in [-6.0, -3.0, -1.0, 0.0, 1.0, 3.0, 6.0] {
            v.push((a, wh + j * wh.sqrt()));
        }
    }
    let mut g = Rng::new(0x9b05_688c_2b3e_6c1f);
    for _ in 0..50 {
        let a = g.log_uniform(9.0, 12.0);
        for j in -6..=6 {
            v.push((a, a + f64::from(j) * a.sqrt()));
        }
    }
    let ratios = [0.001, 0.1, 0.5, 0.9, 0.99, 1.0, 1.01, 1.1, 2.0, 10.0, 100.0];
    for _ in 0..300 {
        let a = g.log_uniform(-3.0, 9.0);
        v.extend(ratios.iter().map(|&r| (a, a * r)));
    }
    for _ in 0..60 {
        let a = g.log_uniform(0.0, 9.0);
        for j in -4..=4 {
            v.push((a, a + f64::from(j) * a.sqrt()));
        }
    }
    for k in 1..=120 {
        let a = f64::from(k) / 2.0;
        for r in [0.25, 0.5, 1.0, 2.0, 4.0] {
            v.push((a, a * r));
        }
    }
    v.extend(cross(&[0.5, 1.0, 5.0, 50.0], &[700.0, 750.0, 800.0, 1000.0, 1e4]));
    v.extend(cross(&[0.001, 0.5, 1.0, 100.0, 1000.0], &[1e-300, 1e-310, f64::from_bits(1)]));
    v.extend(cross(
        &[1e-300, 1e-100, 1e-30, 1e-20, 1e-16, 1e-12, 1e-8],
        &[1e-10, 0.001, 0.1, 0.5, 0.9, 0.99, 0.999_999, 1.0, 1.5, 3.0],
    ));
    for _ in 0..500 {
        v.push((g.log_uniform(-3.0, 6.0), g.log_uniform(-3.0, 6.0)));
    }
    v
}

/// (a, b, x, y) rows for `beta_inc`.
fn beta_rows() -> Vec<[f64; 4]> {
    let mut v: Vec<[f64; 4]> = Vec::new();
    for &s in &specials() {
        v.push([2.0, 3.0, s, 1.0 - s]);
        v.push([s, 3.0, 0.3, 0.7]);
        v.push([2.0, s, 0.3, 0.7]);
        v.push([2.0, 3.0, 0.3, s]);
        v.push([s, s, 0.5, 0.5]);
    }
    let mut shapes = vec![0.001, 0.01, 0.1, 0.5, 1.0, 1.5, 2.0, 5.0, 9.5, 10.5, 20.0, 100.0, 1e3, 1e5];
    around(&mut shapes, 10.0, 1);
    sort_dedup(&mut shapes);
    let xs = [1e-10, 1e-4, 0.01, 0.1, 0.3, 0.5, 0.7, 0.9, 0.99, 0.9999, 1.0 - 1e-10];
    for &a in &shapes {
        for &b in &shapes {
            for &x in &xs {
                v.push([a, b, x, 1.0 - x]);
            }
        }
    }
    let mut g = Rng::new(0xcbbb_9d5d_c105_9ed8);
    for _ in 0..100 {
        let a = g.log_uniform(-3.0, 5.0);
        let b = g.log_uniform(-3.0, 5.0);
        let split = (a + 1.0) / (a + b + 2.0);
        let mut near = Vec::new();
        around(&mut near, split, 2);
        v.extend(near.iter().map(|&x| [a, b, x, 1.0 - x]));
    }
    for _ in 0..100 {
        let a = g.log_uniform(1.0, 6.0);
        let b = g.log_uniform(1.0, 6.0);
        let s = a + b;
        let mean = a / s;
        let sd = (a * b / (s * s * (s + 1.0))).sqrt();
        for j in -4..=4 {
            let x = mean + f64::from(j) * sd;
            if x > 0.0 && x < 1.0 {
                v.push([a, b, x, 1.0 - x]);
            }
        }
    }
    for nu in [1.0, 2.0, 5.0, 10.0, 30.0, 100.0, 1e3, 1e6] {
        for t in [0.01, 0.5, 1.0, 2.0, 5.0, 20.0, 1e3] {
            let t2 = t * t;
            v.push([nu / 2.0, 0.5, nu / (nu + t2), t2 / (nu + t2)]);
        }
    }
    for d in [10.0, 1e4, 1e6, 1e8] {
        for x in [0.1, 1.0, 3.0, 30.0] {
            v.push([1.0, d / 2.0, 2.0 * x / (2.0 * x + d), d / (2.0 * x + d)]);
        }
    }
    v.extend([[2.0, 3.0, 0.3, 0.5], [2.0, 3.0, 0.3, 0.8], [20.0, 30.0, 0.4, 0.5], [20.0, 30.0, 0.6, 0.3]]);
    for a in [1e-300, 1e-20, 1e-8] {
        for b in [1e-300, 1e-8, 1.0, 10.0] {
            for x in [0.001, 0.3, 0.5, 0.7, 0.999] {
                v.push([a, b, x, 1.0 - x]);
            }
        }
    }
    v.extend([[1e3, 1e3, 0.01, 0.99], [1e5, 1e5, 0.3, 0.7], [1e5, 1e5, 0.7, 0.3], [5.0, 1e6, 0.5, 0.5]]);
    for _ in 0..1000 {
        let a = g.log_uniform(-3.0, 5.0);
        let b = g.log_uniform(-3.0, 5.0);
        let x = g.unit();
        v.push([a, b, x, 1.0 - x]);
    }
    v
}

/// Seeded (a, x) pairs for the two wrappers: a in [10^-2, 10^4], x = a·u with u in [0.1, 3).
fn wrapper_pairs(seed: u64) -> Vec<(f64, f64)> {
    let mut g = Rng::new(seed);
    (0..400)
        .map(|_| {
            let a = g.log_uniform(-2.0, 4.0);
            (a, a * g.uniform(0.1, 3.0))
        })
        .collect()
}

pub fn emit() -> Module {
    let mut m = Module::new("special");
    let xs = erf_grid();
    for &x in &xs {
        m.row("math::erf", Class::C, &[f(x)], &[f(oracle::erf(black_box(x)))]);
    }
    for &x in &xs {
        m.row("math::erfc", Class::C, &[f(x)], &[f(oracle::erfc(black_box(x)))]);
    }
    let zs = normal_grid(&xs);
    for &z in &zs {
        m.row("math::normal_cdf", Class::C, &[f(z)], &[f(oracle::normal_cdf(black_box(z)))]);
    }
    for &z in &zs {
        m.row("math::normal_sf", Class::C, &[f(z)], &[f(oracle::normal_sf(black_box(z)))]);
    }
    for &x in &ln_gamma_grid() {
        m.row("math::ln_gamma", Class::C, &[f(x)], &[f(oracle::ln_gamma(black_box(x)))]);
    }
    for &a in &stirling_grid() {
        m.row("math::stirling_error", Class::C, &[f(a)], &[f(oracle::stirling_error(black_box(a)))]);
    }
    for &(x, mu) in &bd0_pairs() {
        m.row("math::bd0", Class::C, &[f(x), f(mu)], &[f(oracle::bd0(black_box(x), black_box(mu)))]);
    }
    for &(a, x) in &kernel_pairs() {
        let r = oracle::ln_gamma_kernel(black_box(a), black_box(x));
        m.row("math::ln_gamma_kernel", Class::C, &[f(a), f(x)], &[f(r)]);
    }
    for &(a, b) in &ln_beta_pairs() {
        m.row("math::ln_beta", Class::C, &[f(a), f(b)], &[f(oracle::ln_beta(black_box(a), black_box(b)))]);
    }
    for &(a, x) in &gamma_pairs() {
        let (p, q) = oracle::gamma_inc(black_box(a), black_box(x));
        m.row("math::gamma_inc", Class::C, &[f(a), f(x)], &[f(p), f(q)]);
    }
    for &(a, x) in &wrapper_pairs(0x1f83_d9ab_fb41_bd6b) {
        let p = oracle::regularized_lower_incomplete_gamma(black_box(a), black_box(x));
        m.row("math::regularized_lower_incomplete_gamma", Class::C, &[f(a), f(x)], &[f(p)]);
    }
    for &(a, x) in &wrapper_pairs(0x5be0_cd19_137e_2179) {
        let q = oracle::regularized_upper_incomplete_gamma(black_box(a), black_box(x));
        m.row("math::regularized_upper_incomplete_gamma", Class::C, &[f(a), f(x)], &[f(q)]);
    }
    for r in &beta_rows() {
        let (p, q) = oracle::beta_inc(black_box(r[0]), black_box(r[1]), black_box(r[2]), black_box(r[3]));
        m.row("math::beta_inc", Class::C, &[f(r[0]), f(r[1]), f(r[2]), f(r[3])], &[f(p), f(q)]);
    }
    let mut g = Rng::new(0x629a_292a_367c_d507);
    let mut ib: Vec<[f64; 3]> = [-1.0, -0.0, 0.0, 1.0, 2.0, f64::NAN, f64::INFINITY]
        .iter()
        .map(|&x| [x, 2.0, 2.0])
        .collect();
    for _ in 0..400 {
        let x = g.unit();
        ib.push([x, g.log_uniform(-2.0, 3.0), g.log_uniform(-2.0, 3.0)]);
    }
    for r in &ib {
        let p = oracle::regularized_incomplete_beta(black_box(r[0]), black_box(r[1]), black_box(r[2]));
        m.row("math::regularized_incomplete_beta", Class::C, &[f(r[0]), f(r[1]), f(r[2])], &[f(p)]);
    }
    m
}
