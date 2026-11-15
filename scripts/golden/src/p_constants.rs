//! Proofs that the extracted constants are the oracle's (port plan §5 step 6).
//!
//! Each proof transcribes an oracle function in Rust's operation order, reading only the extracted constants, and
//! compares it bit for bit with the oracle's own hook over a grid that crosses every branch. Compiled by the same
//! rustc as the oracle, the transcription folds constants the same way (`PI.ln()`, `(2π).sqrt()`). One wrong digit
//! in any coefficient fails its proof, and the generator writes nothing (the `GOLDEN_NUDGE` mutation test).
//!
//! Proven this way:
//! - the fdlibm erf/erfc table (55 values), against `oracle::erf` and `oracle::erfc`;
//! - the Lanczos table, `G` and `LN_SQRT_2PI`, against `oracle::ln_gamma`;
//! - Acklam's tables, `P_LOW` and `P_HIGH`, against `oracle::z_quantile`;
//! - the KS series constants, against `oracle::kolmogorov_sf`;
//! - `POISSON_PTRS_THRESHOLD` and `BINOMIAL_DIRECT_MAX_N`, by the draws on either side of each threshold.
//!
//! The rest (the convergence settings, the two tolerances, the Markov limits, `ai`'s bounds) are exact literals
//! whose behaviour the module rows test where those modules land (M2, M3, M5, M6, M7). The std constants come
//! from Rust itself.

use std::f64::consts::PI;
use std::hint::black_box;

use pramana::distribution::{Binomial, Distribution, Poisson};
use pramana::{Rng, SimpleRng, oracle};

use crate::consts::{Const, f};
use crate::grids::Rng as GridRng;

/// One proof's verdict: its name, and the first mismatch if any.
pub struct Verdict {
    pub name: &'static str,
    pub checked: usize,
    pub failure: Option<String>,
}

fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

fn verdict(name: &'static str, cases: &[f64], ours: impl Fn(f64) -> f64, theirs: impl Fn(f64) -> f64) -> Verdict {
    for &x in cases {
        let (a, b) = (ours(black_box(x)), theirs(black_box(x)));
        if !same(a, b) {
            return Verdict {
                name,
                checked: cases.len(),
                failure: Some(format!("at {x:?}: extracted constants give {a:?}, the oracle {b:?}")),
            };
        }
    }
    Verdict { name, checked: cases.len(), failure: None }
}

/// The doubles at and either side of each point.
fn around(points: &[f64]) -> Vec<f64> {
    let mut v = Vec::new();
    for &p in points {
        for x in [p, -p] {
            v.extend([x, f64::from_bits(x.to_bits().wrapping_add(1)), f64::from_bits(x.to_bits().wrapping_sub(1))]);
        }
    }
    v
}

// ── fdlibm erf / erfc (rust-old/src/math.rs), with the table read from the extracted constants ──

struct Fd<'a>(&'a [Const]);

impl Fd<'_> {
    fn k(&self, n: &str) -> f64 {
        f(self.0, &format!("math::fdlibm::{n}"))
    }

    fn erfc_mid(&self, x: f64) -> f64 {
        let k = |n| self.k(n);
        let s = x.abs() - 1.0;
        let p = k("PA0") + s * (k("PA1") + s * (k("PA2") + s * (k("PA3") + s * (k("PA4") + s * (k("PA5") + s * k("PA6"))))));
        let q = 1.0 + s * (k("QA1") + s * (k("QA2") + s * (k("QA3") + s * (k("QA4") + s * (k("QA5") + s * k("QA6"))))));
        1.0 - k("ERX") - p / q
    }

    fn erfc_tail(&self, ix: u32, x: f64) -> f64 {
        let k = |n| self.k(n);
        if ix < 0x3ff4_0000 {
            return self.erfc_mid(x);
        }
        let x = x.abs();
        let s = 1.0 / (x * x);
        let (r, big_s) = if ix < 0x4006_db6d {
            (
                k("RA0") + s * (k("RA1") + s * (k("RA2") + s * (k("RA3") + s * (k("RA4") + s * (k("RA5") + s * (k("RA6") + s * k("RA7"))))))),
                1.0 + s * (k("SA1") + s * (k("SA2") + s * (k("SA3") + s * (k("SA4") + s * (k("SA5") + s * (k("SA6") + s * (k("SA7") + s * k("SA8")))))))),
            )
        } else {
            (
                k("RB0") + s * (k("RB1") + s * (k("RB2") + s * (k("RB3") + s * (k("RB4") + s * (k("RB5") + s * k("RB6")))))),
                1.0 + s * (k("SB1") + s * (k("SB2") + s * (k("SB3") + s * (k("SB4") + s * (k("SB5") + s * (k("SB6") + s * k("SB7"))))))),
            )
        };
        let z = f64::from_bits(x.to_bits() & 0xffff_ffff_0000_0000);
        (-z * z - 0.5625).exp() * ((z - x) * (z + x) + r / big_s).exp() / x
    }

    #[allow(clippy::cast_possible_truncation)]
    fn erf(&self, x: f64) -> f64 {
        let k = |n| self.k(n);
        let hx = (x.to_bits() >> 32) as u32;
        let negative = hx >> 31 != 0;
        let ix = hx & 0x7fff_ffff;
        if ix >= 0x7ff0_0000 {
            return if x.is_nan() { x } else if negative { -1.0 } else { 1.0 };
        }
        if ix < 0x3feb_0000 {
            if ix < 0x3e30_0000 {
                return 0.125 * (8.0 * x + k("EFX8") * x);
            }
            let z = x * x;
            let r = k("PP0") + z * (k("PP1") + z * (k("PP2") + z * (k("PP3") + z * k("PP4"))));
            let s = 1.0 + z * (k("QQ1") + z * (k("QQ2") + z * (k("QQ3") + z * (k("QQ4") + z * k("QQ5")))));
            return x + x * (r / s);
        }
        let y = if ix < 0x4018_0000 { 1.0 - self.erfc_tail(ix, x) } else { 1.0 - f64::MIN_POSITIVE };
        if negative { -y } else { y }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn erfc(&self, x: f64) -> f64 {
        let k = |n| self.k(n);
        let hx = (x.to_bits() >> 32) as u32;
        let negative = hx >> 31 != 0;
        let ix = hx & 0x7fff_ffff;
        if ix >= 0x7ff0_0000 {
            return if x.is_nan() { x } else if negative { 2.0 } else { 0.0 };
        }
        if ix < 0x3feb_0000 {
            if ix < 0x3c70_0000 {
                return 1.0 - x;
            }
            let z = x * x;
            let r = k("PP0") + z * (k("PP1") + z * (k("PP2") + z * (k("PP3") + z * k("PP4"))));
            let s = 1.0 + z * (k("QQ1") + z * (k("QQ2") + z * (k("QQ3") + z * (k("QQ4") + z * k("QQ5")))));
            let y = r / s;
            if negative || ix < 0x3fd0_0000 {
                return 1.0 - (x + x * y);
            }
            return 0.5 - (x - 0.5 + x * y);
        }
        if ix < 0x403c_0000 {
            let tail = self.erfc_tail(ix, x);
            return if negative { 2.0 - tail } else { tail };
        }
        if negative { 2.0 - f64::MIN_POSITIVE } else { 0.0 }
    }
}

// ── Lanczos ln_gamma (rust-old/src/math.rs) ──

fn ln_gamma(c: &[Const], x: f64) -> f64 {
    if x < 0.5 {
        let sin_val = (PI * x).sin();
        if sin_val.abs() < 1e-300 {
            return f64::INFINITY;
        }
        return PI.ln() - sin_val.abs().ln() - ln_gamma(c, 1.0 - x);
    }
    let coeff = |i: usize| f(c, &format!("math::ln_gamma::COEFFICIENTS[{i}]"));
    let x = x - 1.0;
    let mut sum = coeff(0);
    for i in 1..9 {
        #[allow(clippy::cast_precision_loss)]
        let fi = i as f64;
        sum += coeff(i) / (x + fi);
    }
    let t = x + f(c, "math::ln_gamma::G") + 0.5;
    f(c, "math::LN_SQRT_2PI") + (t.ln() * (x + 0.5)) - t + sum.ln()
}

// ── Acklam's z_quantile (rust-old/src/hypothesis.rs) ──

fn z_quantile(c: &[Const], p: f64) -> f64 {
    if p.is_nan() {
        return f64::NAN;
    }
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    let t = |n: &str, i: usize| f(c, &format!("hypothesis::z_quantile::{n}[{i}]"));
    let (a, b, cc, d) = (|i| t("A", i), |i| t("B", i), |i| t("C", i), |i| t("D", i));
    let p_low = f(c, "hypothesis::z_quantile::P_LOW");
    let p_high = f(c, "hypothesis::z_quantile::P_HIGH");
    let x = if p < p_low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((cc(0) * q + cc(1)) * q + cc(2)) * q + cc(3)) * q + cc(4)) * q + cc(5))
            / ((((d(0) * q + d(1)) * q + d(2)) * q + d(3)) * q + 1.0)
    } else if p <= p_high {
        let q = p - 0.5;
        let r = q * q;
        (((((a(0) * r + a(1)) * r + a(2)) * r + a(3)) * r + a(4)) * r + a(5)) * q
            / (((((b(0) * r + b(1)) * r + b(2)) * r + b(3)) * r + b(4)) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((cc(0) * q + cc(1)) * q + cc(2)) * q + cc(3)) * q + cc(4)) * q + cc(5))
            / ((((d(0) * q + d(1)) * q + d(2)) * q + d(3)) * q + 1.0)
    };
    let density = (-0.5 * x * x).exp() / (2.0 * PI).sqrt();
    if density > 0.0 && density.is_finite() {
        let u = (oracle::normal_cdf(x) - p) / density;
        x - u / (1.0 + 0.5 * x * u)
    } else {
        x
    }
}

// ── kolmogorov_sf (rust-old/src/hypothesis.rs) ──

fn kolmogorov_sf(c: &[Const], lambda: f64) -> f64 {
    if lambda.is_nan() {
        return f64::NAN;
    }
    if lambda <= 0.0 {
        return 1.0;
    }
    if lambda < 1.18 {
        let pi2_over_8 = f(c, "hypothesis::kolmogorov_sf::PI2_OVER_8");
        let sqrt_2pi = f(c, "hypothesis::kolmogorov_sf::SQRT_2PI");
        let y = (-pi2_over_8 / (lambda * lambda)).exp();
        let cdf = sqrt_2pi / lambda * (y + y.powi(9) + y.powi(25) + y.powi(49));
        (1.0 - cdf).clamp(0.0, 1.0)
    } else {
        let x = (-2.0 * lambda * lambda).exp();
        (2.0 * (x - x.powi(4) + x.powi(9))).clamp(0.0, 1.0)
    }
}

/// Counts the uniforms a sampler draws.
struct Counting(SimpleRng, u64);

impl Rng for Counting {
    fn next_f64(&mut self) -> f64 {
        self.1 += 1;
        self.0.next_f64()
    }
}

/// The sampler thresholds, by their draws: above POISSON_PTRS_THRESHOLD `Poisson::sample` is exactly PTRS (same
/// values, same draws), at it Knuth's method (a different draw count); a Binomial of n = BINOMIAL_DIRECT_MAX_N draws
/// exactly n uniforms per variate, and one more trial switches to Beta splitting (a different count).
fn thresholds(c: &[Const]) -> Verdict {
    let name = "sampler thresholds";
    let lam = f(c, "distribution::POISSON_PTRS_THRESHOLD");
    let above = f64::from_bits(lam.to_bits() + 1);
    let fail = |m: String| Verdict { name, checked: 4, failure: Some(m) };
    let (Ok(p_above), Ok(p_at)) = (Poisson::new(above), Poisson::new(lam)) else {
        return fail(format!("Poisson::new refused λ = {lam:?} or its successor"));
    };
    let (mut r1, mut r2) = (Counting(SimpleRng::new(9), 0), Counting(SimpleRng::new(9), 0));
    for _ in 0..200 {
        if !same(p_above.sample(&mut r1), oracle::sample_poisson_ptrs(above, &mut r2)) || r1.1 != r2.1 {
            return fail(format!("Poisson above λ = {lam:?} is not PTRS"));
        }
    }
    let (mut r3, mut r4) = (Counting(SimpleRng::new(9), 0), Counting(SimpleRng::new(9), 0));
    for _ in 0..200 {
        let _ = p_at.sample(&mut r3);
        let _ = oracle::sample_poisson_ptrs(lam, &mut r4);
    }
    if r3.1 == r4.1 {
        return fail(format!("Poisson at λ = {lam:?} draws like PTRS: the threshold is lower"));
    }
    let n = match c.iter().find(|k| k.path == "distribution::BINOMIAL_DIRECT_MAX_N").map(|k| k.val) {
        Some(crate::consts::Val::I(n)) => n,
        _ => return fail("no BINOMIAL_DIRECT_MAX_N".to_string()),
    };
    #[allow(clippy::cast_sign_loss)]
    let (Ok(b_at), Ok(b_above)) = (Binomial::new(n as u64, 0.3), Binomial::new(n as u64 + 1, 0.3)) else {
        return fail("Binomial::new refused".to_string());
    };
    let mut r5 = Counting(SimpleRng::new(9), 0);
    let _ = b_at.sample(&mut r5);
    #[allow(clippy::cast_sign_loss)]
    if r5.1 != n as u64 {
        return fail(format!("Binomial(n = {n}) drew {} uniforms, not {n}", r5.1));
    }
    let mut r6 = Counting(SimpleRng::new(9), 0);
    let _ = b_above.sample(&mut r6);
    #[allow(clippy::cast_sign_loss)]
    if r6.1 == n as u64 + 1 {
        return fail(format!("Binomial(n = {}) is still direct: the threshold is higher", n + 1));
    }
    Verdict { name, checked: 4, failure: None }
}

/// Every proof, in order.
pub fn prove(c: &[Const]) -> Vec<Verdict> {
    let fd = Fd(c);
    let mut erf_x = vec![0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1e-300, 1e-20];
    erf_x.extend(around(&[0.84375, 1.25, 1.0 / 0.35, 2.857_142_857_142_857, 6.0, 28.0, 0.25]));
    erf_x.extend(around(&[f64::from_bits(0x3e30_0000_0000_0000), f64::from_bits(0x3c70_0000_0000_0000)]));
    let mut g = GridRng::new(0xe4f1);
    erf_x.extend((0..4000).map(|_| g.uniform(-30.0, 30.0)));
    erf_x.extend((0..1000).map(|_| g.signed_log_uniform(-20.0, 1.0)));

    let mut lg_x = vec![0.5, 1.0, 2.0, 0.0, -0.0, f64::NAN, f64::INFINITY, -1.0, -2.5];
    lg_x.extend((0..3000).map(|_| g.uniform(-20.0, 200.0)));
    lg_x.extend((0..1000).map(|_| g.log_uniform(-300.0, 15.0)));

    let mut zq_p = vec![0.0, 1.0, 0.5, f64::NAN, -1.0, 2.0, 1e-300, 1.0 - 1e-16];
    let p_low = f(c, "hypothesis::z_quantile::P_LOW");
    for p in [p_low, f(c, "hypothesis::z_quantile::P_HIGH")] {
        zq_p.extend([p, f64::from_bits(p.to_bits() + 1), f64::from_bits(p.to_bits() - 1)]);
    }
    zq_p.extend((0..3000).map(|_| g.unit()));
    zq_p.extend((0..500).map(|_| g.log_uniform(-300.0, -2.0)));

    let mut ks_l = vec![0.0, -1.0, f64::NAN, f64::INFINITY, 1.18, 1.18 - 1e-12, 1.18 + 1e-12];
    ks_l.extend((0..3000).map(|_| g.uniform(0.0, 5.0)));

    vec![
        verdict("fdlibm erf table", &erf_x, |x| fd.erf(x), oracle::erf),
        verdict("fdlibm erfc table", &erf_x, |x| fd.erfc(x), oracle::erfc),
        verdict("Lanczos table, G, LN_SQRT_2PI", &lg_x, |x| ln_gamma(c, x), oracle::ln_gamma),
        verdict("Acklam tables, P_LOW, P_HIGH", &zq_p, |p| z_quantile(c, p), oracle::z_quantile),
        verdict("KS series constants", &ks_l, |l| kolmogorov_sf(c, l), oracle::kolmogorov_sf),
        thresholds(c),
    ]
}
