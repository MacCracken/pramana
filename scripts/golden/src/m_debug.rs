//! `debug.tsv` — Rust's derived `Debug` of 1.3.0's types, `{:?}` and `{:#?}` (port plan D12; M1 part f): each
//! milestone adds its own types' rows.
//!
//! - `debug::SimpleRng`: the generator from 25 seeds (0, 1, 42, 2^63, u64::MAX and 20 seeded words), in both forms.
//! - `debug::PramanaError`: each variant with an empty message, the error pramana's codes carry (ADR 007 row 8, as
//!   amended), in both forms.
//!
//! - `debug::Normal` (M3 part a1): 1.3.0's Normal at 24 (μ, σ), the special parameters among them, in both forms.
//! - `debug::Uniform`, `debug::Exponential`, `debug::Bernoulli` (M3 part a2): each at up to 16 parameters (a seeded
//!   Uniform whose width rounds to 0 is dropped), the special ones among them, in both forms.
//! - `debug::Cauchy`, `debug::Weibull` (M3 part a3): each at 16 parameters, the special ones among them, in both
//!   forms.
//! - `debug::f64` (M3 part a1): an f64 field's form, `{:?}`, on `fmt.tsv`'s 25,200 doubles. `{:#?}` prints an f64
//!   the same way.
//!
//! Inputs: the seed, the variant or the fields, then `i:0` for `{:?}` or `i:1` for `{:#?}` (an f64 alone for
//! `debug::f64`). Output: the text. Class A.

use crate::grids::Rng as Seeds;
use crate::tsv::{Class, Module, Tok, i, s};
use crate::tsv::f;
use pramana::distribution::{Bernoulli, Cauchy, Exponential, Normal, Uniform, Weibull};
use pramana::{PramanaError, SimpleRng};

fn both<T: std::fmt::Debug>(x: &T) -> [String; 2] {
    [format!("{x:?}"), format!("{x:#?}")]
}

#[allow(clippy::cast_possible_wrap)]
pub fn emit() -> Module {
    let mut m = Module::new("debug");
    let mut g = Seeds::new(0x1f83_d9ab_fb41_bd6b);
    let mut seeds: Vec<u64> = vec![0, 1, 42, 1 << 63, u64::MAX];
    seeds.extend((0..20).map(|k| g.word() >> (k % 64)));
    for &seed in &seeds {
        for (alt, text) in both(&SimpleRng::new(seed)).iter().enumerate() {
            m.row("debug::SimpleRng", Class::A, &[i(seed as i64), i(alt as i64)], &[s(text.clone())]);
        }
    }
    for (name, e) in [
        ("InvalidParameter", PramanaError::InvalidParameter(String::new())),
        ("InvalidSample", PramanaError::InvalidSample(String::new())),
        ("ConvergenceFailure", PramanaError::ConvergenceFailure(String::new())),
        ("DimensionMismatch", PramanaError::DimensionMismatch(String::new())),
        ("ComputationError", PramanaError::ComputationError(String::new())),
    ] {
        for (alt, text) in both(&e).iter().enumerate() {
            m.row(
                "debug::PramanaError",
                Class::A,
                &[Tok::E("PramanaError", name.to_string()), i(alt as i64)],
                &[s(text.clone())],
            );
        }
    }
    let mut normals: Vec<(f64, f64)> = vec![
        (0.0, 1.0),
        (-0.0, 1.0),
        (1.5, 2.3),
        (-3.5, 0.25),
        (1e-300, 1e300),
        (f64::MAX, f64::MIN_POSITIVE),
        (1e16, 1e-4),
        (9.999_999_999_999_999e15, 9.999_999_999_999_999e-5),
        (f64::from_bits(1), 5e-324),
        (2.980_232_238_769_531_3e-8, 1.0),
    ];
    normals.extend((0..14).map(|_| (g.signed_log_uniform(-20.0, 20.0), g.log_uniform(-20.0, 20.0))));
    for &(mu, sd) in &normals {
        let n = Normal::new(mu, sd).expect("valid parameters");
        for (alt, text) in both(&n).iter().enumerate() {
            m.row("debug::Normal", Class::A, &[f(mu), f(sd), i(alt as i64)], &[s(text.clone())]);
        }
    }
    let mut uniforms: Vec<(f64, f64)> =
        vec![(0.0, 1.0), (-0.0, 1.0), (-1.0, 5.0), (-f64::MAX, 0.0), (1e-300, 1e300), (1e16, 1e17)];
    uniforms.extend((0..10).map(|_| {
        let lo = g.signed_log_uniform(-20.0, 20.0);
        (lo, lo + g.log_uniform(-20.0, 20.0))
    }));
    uniforms.retain(|&(lo, hi)| Uniform::new(lo, hi).is_ok());
    for &(lo, hi) in &uniforms {
        let u = Uniform::new(lo, hi).expect("valid parameters");
        for (alt, text) in both(&u).iter().enumerate() {
            m.row("debug::Uniform", Class::A, &[f(lo), f(hi), i(alt as i64)], &[s(text.clone())]);
        }
    }
    let mut lambdas: Vec<f64> = vec![1.0, 2.5, 1e-4, 9.999_999_999_999_999e-5, f64::MAX, f64::from_bits(1)];
    lambdas.extend((0..10).map(|_| g.log_uniform(-20.0, 20.0)));
    for &l in &lambdas {
        let e = Exponential::new(l).expect("valid parameters");
        for (alt, text) in both(&e).iter().enumerate() {
            m.row("debug::Exponential", Class::A, &[f(l), i(alt as i64)], &[s(text.clone())]);
        }
    }
    let mut ps: Vec<f64> = vec![0.0, -0.0, 1.0, 0.7, 1.0 - f64::EPSILON / 2.0, f64::from_bits(1)];
    ps.extend((0..10).map(|_| g.unit()));
    for &p in &ps {
        let b = Bernoulli::new(p).expect("valid parameters");
        for (alt, text) in both(&b).iter().enumerate() {
            m.row("debug::Bernoulli", Class::A, &[f(p), i(alt as i64)], &[s(text.clone())]);
        }
    }
    let mut cauchys: Vec<(f64, f64)> =
        vec![(0.0, 1.0), (-0.0, 1.0), (2.0, 3.0), (f64::MAX, f64::from_bits(1)), (1e-300, 1e300), (1e16, 1e-4)];
    cauchys.extend((0..10).map(|_| (g.signed_log_uniform(-20.0, 20.0), g.log_uniform(-20.0, 20.0))));
    for &(x0, ga) in &cauchys {
        let c = Cauchy::new(x0, ga).expect("valid parameters");
        for (alt, text) in both(&c).iter().enumerate() {
            m.row("debug::Cauchy", Class::A, &[f(x0), f(ga), i(alt as i64)], &[s(text.clone())]);
        }
    }
    let mut weibulls: Vec<(f64, f64)> =
        vec![(1.0, 1.0), (2.0, 3.0), (0.5, 1e-4), (f64::MAX, f64::from_bits(1)), (1e-300, 1e300), (1e16, 0.1)];
    weibulls.extend((0..10).map(|_| (g.log_uniform(-20.0, 20.0), g.log_uniform(-20.0, 20.0))));
    for &(k, l) in &weibulls {
        let w = Weibull::new(k, l).expect("valid parameters");
        for (alt, text) in both(&w).iter().enumerate() {
            m.row("debug::Weibull", Class::A, &[f(k), f(l), i(alt as i64)], &[s(text.clone())]);
        }
    }
    for x in crate::m_fmt::inputs() {
        m.row("debug::f64", Class::A, &[f(x)], &[s(format!("{x:?}"))]);
    }
    m
}
