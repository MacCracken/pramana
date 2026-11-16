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
//! Every row is class C: each function calls glibc's exp. `validate.py` transcribes them, re-derives every row with
//! glibc's exp, and writes `special.cr.tsv` with the rows whose value changes when exp is correctly rounded.

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
    m
}
