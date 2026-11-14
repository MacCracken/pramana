//! Shared mathematical helpers (crate-internal).
//!
//! Special functions used by the distribution, hypothesis-testing and
//! regression modules. See ADR 002 for why these are hand-rolled.
//!
//! Accuracy notes:
//! - `erf`/`erfc` are a port of FreeBSD msun `s_erf.c` (< 1 ulp).
//! - Incomplete gamma/beta return the lower **and** upper tail, each computed
//!   directly, so upper-tail p-values do not lose precision to `1 - cdf`.
//! - Series/continued-fraction iteration budgets scale with `sqrt(shape)`
//!   (they converge in `O(sqrt(shape))` steps), so large degrees of freedom
//!   stay accurate instead of silently truncating.

use std::f64::consts::{PI, SQRT_2};

/// ln(sqrt(2π)).
pub(crate) const LN_SQRT_2PI: f64 = 0.918_938_533_204_672_8;

/// Relative convergence tolerance for series and continued fractions.
const CONVERGENCE_EPS: f64 = 1e-15;

/// Lentz's "tiny" guard against zero denominators.
const LENTZ_TINY: f64 = 1e-300;

/// Hard cap on series / continued-fraction iterations (bounds worst-case time).
const MAX_ITERATIONS: f64 = 1_000_000.0;

/// Above this shape parameter the incomplete gamma function uses the
/// Wilson–Hilferty normal approximation (error far below 1e-10 here) instead of
/// a series that would need ~10^5+ iterations.
const GAMMA_WILSON_HILFERTY_SHAPE: f64 = 1e9;

/// Iteration budget for a series/continued fraction converging in `O(sqrt(shape))`.
#[inline]
fn iteration_budget(shape: f64) -> usize {
    let root = if shape.is_finite() && shape > 0.0 {
        shape.sqrt()
    } else {
        0.0
    };
    // Truncation is intended: the budget is a bounded, positive iteration count.
    (200.0 + 20.0 * root).min(MAX_ITERATIONS) as usize
}

// ---------------------------------------------------------------------------
// Error function
// ---------------------------------------------------------------------------
//
// origin: FreeBSD /usr/src/lib/msun/src/s_erf.c, via the Rust `libm` crate
// (MIT licensed).
//
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunPro, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

/// fdlibm `s_erf.c` coefficients, transcribed verbatim (21 significant
/// digits, as in the C source) so they can be checked digit-for-digit.
#[allow(clippy::excessive_precision)]
mod fdlibm {
    pub(super) const ERX: f64 = 8.450_629_115_104_675_292_97e-01;
    // Coefficients for approximation to erf on [0, 0.84375]
    #[cfg(test)]
    pub(super) const EFX8: f64 = 1.027_033_336_764_100_690_53e+00;
    pub(super) const PP0: f64 = 1.283_791_670_955_125_585_61e-01;
    pub(super) const PP1: f64 = -3.250_421_072_470_014_993_70e-01;
    pub(super) const PP2: f64 = -2.848_174_957_559_851_047_66e-02;
    pub(super) const PP3: f64 = -5.770_270_296_489_441_591_57e-03;
    pub(super) const PP4: f64 = -2.376_301_665_665_016_260_84e-05;
    pub(super) const QQ1: f64 = 3.979_172_239_591_553_528_19e-01;
    pub(super) const QQ2: f64 = 6.502_224_998_876_729_444_85e-02;
    pub(super) const QQ3: f64 = 5.081_306_281_875_765_627_76e-03;
    pub(super) const QQ4: f64 = 1.324_947_380_043_216_445_26e-04;
    pub(super) const QQ5: f64 = -3.960_228_278_775_368_123_20e-06;
    // Coefficients for approximation to erf in [0.84375, 1.25]
    pub(super) const PA0: f64 = -2.362_118_560_752_659_440_77e-03;
    pub(super) const PA1: f64 = 4.148_561_186_837_483_316_66e-01;
    pub(super) const PA2: f64 = -3.722_078_760_357_013_238_47e-01;
    pub(super) const PA3: f64 = 3.183_466_199_011_617_536_74e-01;
    pub(super) const PA4: f64 = -1.108_946_942_823_966_774_76e-01;
    pub(super) const PA5: f64 = 3.547_830_432_561_823_593_71e-02;
    pub(super) const PA6: f64 = -2.166_375_594_868_790_843_00e-03;
    pub(super) const QA1: f64 = 1.064_208_804_008_442_282_86e-01;
    pub(super) const QA2: f64 = 5.403_979_177_021_710_489_37e-01;
    pub(super) const QA3: f64 = 7.182_865_441_419_626_628_68e-02;
    pub(super) const QA4: f64 = 1.261_712_198_087_616_421_12e-01;
    pub(super) const QA5: f64 = 1.363_708_391_202_905_073_62e-02;
    pub(super) const QA6: f64 = 1.198_449_984_679_910_741_70e-02;
    // Coefficients for approximation to erfc in [1.25, 1/0.35]
    pub(super) const RA0: f64 = -9.864_944_034_847_148_227_05e-03;
    pub(super) const RA1: f64 = -6.938_585_727_071_817_643_72e-01;
    pub(super) const RA2: f64 = -1.055_862_622_532_329_098_14e+01;
    pub(super) const RA3: f64 = -6.237_533_245_032_600_603_96e+01;
    pub(super) const RA4: f64 = -1.623_966_694_625_734_703_55e+02;
    pub(super) const RA5: f64 = -1.846_050_929_067_110_359_94e+02;
    pub(super) const RA6: f64 = -8.128_743_550_630_659_342_46e+01;
    pub(super) const RA7: f64 = -9.814_329_344_169_145_485_92e+00;
    pub(super) const SA1: f64 = 1.965_127_166_743_925_712_92e+01;
    pub(super) const SA2: f64 = 1.376_577_541_435_190_426_00e+02;
    pub(super) const SA3: f64 = 4.345_658_774_752_292_288_21e+02;
    pub(super) const SA4: f64 = 6.453_872_717_332_678_803_36e+02;
    pub(super) const SA5: f64 = 4.290_081_400_275_678_333_86e+02;
    pub(super) const SA6: f64 = 1.086_350_055_417_794_351_34e+02;
    pub(super) const SA7: f64 = 6.570_249_770_319_281_701_35e+00;
    pub(super) const SA8: f64 = -6.042_441_521_485_809_874_38e-02;
    // Coefficients for approximation to erfc in [1/0.35, 28]
    pub(super) const RB0: f64 = -9.864_942_924_700_099_285_97e-03;
    pub(super) const RB1: f64 = -7.992_832_376_805_230_065_74e-01;
    pub(super) const RB2: f64 = -1.775_795_491_775_475_198_89e+01;
    pub(super) const RB3: f64 = -1.606_363_848_558_219_160_62e+02;
    pub(super) const RB4: f64 = -6.375_664_433_683_896_277_22e+02;
    pub(super) const RB5: f64 = -1.025_095_131_611_077_249_54e+03;
    pub(super) const RB6: f64 = -4.835_191_916_086_513_970_19e+02;
    pub(super) const SB1: f64 = 3.033_806_074_348_245_829_24e+01;
    pub(super) const SB2: f64 = 3.257_925_129_965_739_188_26e+02;
    pub(super) const SB3: f64 = 1.536_729_586_084_436_959_94e+03;
    pub(super) const SB4: f64 = 3.199_858_219_508_595_539_08e+03;
    pub(super) const SB5: f64 = 2.553_050_406_433_164_425_83e+03;
    pub(super) const SB6: f64 = 4.745_285_412_069_553_672_15e+02;
    pub(super) const SB7: f64 = -2.244_095_244_658_581_833_62e+01;
}

use fdlibm::*;

/// High 32 bits of an `f64`.
#[inline]
fn high_word(x: f64) -> u32 {
    // Truncation is intended: keep only the upper half of the bit pattern.
    (x.to_bits() >> 32) as u32
}

/// erfc on [0.84375, 1.25]: `1 - erx - P1/Q1`.
#[inline]
fn erfc_mid(x: f64) -> f64 {
    let s = x.abs() - 1.0;
    let p = PA0 + s * (PA1 + s * (PA2 + s * (PA3 + s * (PA4 + s * (PA5 + s * PA6)))));
    let q = 1.0 + s * (QA1 + s * (QA2 + s * (QA3 + s * (QA4 + s * (QA5 + s * QA6)))));
    1.0 - ERX - p / q
}

/// erfc(|x|) for 0.84375 <= |x| < 28, given the high word of |x|.
#[inline]
fn erfc_tail(ix: u32, x: f64) -> f64 {
    if ix < 0x3ff4_0000 {
        // |x| < 1.25
        return erfc_mid(x);
    }
    let x = x.abs();
    let s = 1.0 / (x * x);
    let (r, big_s) = if ix < 0x4006_db6d {
        // |x| < 1/0.35
        (
            RA0 + s * (RA1 + s * (RA2 + s * (RA3 + s * (RA4 + s * (RA5 + s * (RA6 + s * RA7)))))),
            1.0 + s
                * (SA1
                    + s * (SA2
                        + s * (SA3 + s * (SA4 + s * (SA5 + s * (SA6 + s * (SA7 + s * SA8))))))),
        )
    } else {
        (
            RB0 + s * (RB1 + s * (RB2 + s * (RB3 + s * (RB4 + s * (RB5 + s * RB6))))),
            1.0 + s * (SB1 + s * (SB2 + s * (SB3 + s * (SB4 + s * (SB5 + s * (SB6 + s * SB7)))))),
        )
    };
    // z = x with the low 32 bits cleared, so -z*z is exact.
    let z = f64::from_bits(x.to_bits() & 0xffff_ffff_0000_0000);
    (-z * z - 0.5625).exp() * ((z - x) * (z + x) + r / big_s).exp() / x
}

/// Error function erf(x) = 2/sqrt(π) ∫₀ˣ exp(-t²) dt, accurate to < 1 ulp.
///
/// The library itself only needs `erfc` (via `normal_cdf`/`normal_sf`); `erf`
/// is kept as the test reference for `erfc` (`erf + erfc = 1`).
#[cfg(test)]
#[must_use]
pub(crate) fn erf(x: f64) -> f64 {
    let hx = high_word(x);
    let negative = hx >> 31 != 0;
    let ix = hx & 0x7fff_ffff;
    if ix >= 0x7ff0_0000 {
        // erf(NaN) = NaN, erf(±inf) = ±1
        return if x.is_nan() {
            x
        } else if negative {
            -1.0
        } else {
            1.0
        };
    }
    if ix < 0x3feb_0000 {
        // |x| < 0.84375
        if ix < 0x3e30_0000 {
            // |x| < 2^-28: avoid underflow
            return 0.125 * (8.0 * x + EFX8 * x);
        }
        let z = x * x;
        let r = PP0 + z * (PP1 + z * (PP2 + z * (PP3 + z * PP4)));
        let s = 1.0 + z * (QQ1 + z * (QQ2 + z * (QQ3 + z * (QQ4 + z * QQ5))));
        return x + x * (r / s);
    }
    let y = if ix < 0x4018_0000 {
        // 0.84375 <= |x| < 6
        1.0 - erfc_tail(ix, x)
    } else {
        1.0 - f64::MIN_POSITIVE
    };
    if negative { -y } else { y }
}

/// Complementary error function erfc(x) = 1 - erf(x), computed directly so
/// that tail values keep full relative precision.
#[must_use]
pub(crate) fn erfc(x: f64) -> f64 {
    let hx = high_word(x);
    let negative = hx >> 31 != 0;
    let ix = hx & 0x7fff_ffff;
    if ix >= 0x7ff0_0000 {
        // erfc(NaN) = NaN, erfc(+inf) = 0, erfc(-inf) = 2
        return if x.is_nan() {
            x
        } else if negative {
            2.0
        } else {
            0.0
        };
    }
    if ix < 0x3feb_0000 {
        // |x| < 0.84375
        if ix < 0x3c70_0000 {
            // |x| < 2^-56
            return 1.0 - x;
        }
        let z = x * x;
        let r = PP0 + z * (PP1 + z * (PP2 + z * (PP3 + z * PP4)));
        let s = 1.0 + z * (QQ1 + z * (QQ2 + z * (QQ3 + z * (QQ4 + z * QQ5))));
        let y = r / s;
        if negative || ix < 0x3fd0_0000 {
            // x < 1/4
            return 1.0 - (x + x * y);
        }
        return 0.5 - (x - 0.5 + x * y);
    }
    if ix < 0x403c_0000 {
        // 0.84375 <= |x| < 28
        let tail = erfc_tail(ix, x);
        return if negative { 2.0 - tail } else { tail };
    }
    if negative {
        2.0 - f64::MIN_POSITIVE
    } else {
        0.0
    }
}

/// Standard normal CDF Φ(z), accurate in both tails.
#[must_use]
#[inline]
pub(crate) fn normal_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / SQRT_2)
}

/// Standard normal survival function 1 - Φ(z), accurate in the upper tail.
#[must_use]
#[inline]
pub(crate) fn normal_sf(z: f64) -> f64 {
    0.5 * erfc(z / SQRT_2)
}

// ---------------------------------------------------------------------------
// Gamma and beta functions
// ---------------------------------------------------------------------------

/// Natural log of Gamma(x) via the Lanczos approximation (g=7, n=9).
///
/// For x < 0.5 the reflection formula is used and `ln|Γ(x)|` is returned;
/// poles (0, -1, -2, ...) give `+inf`.
#[must_use]
pub(crate) fn ln_gamma(x: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    const G: f64 = 7.0;

    if x < 0.5 {
        // Reflection formula
        let sin_val = (PI * x).sin();
        if sin_val.abs() < 1e-300 {
            return f64::INFINITY;
        }
        return PI.ln() - sin_val.abs().ln() - ln_gamma(1.0 - x);
    }

    let x = x - 1.0;
    let mut sum = COEFFICIENTS[0];
    for (i, &coeff) in COEFFICIENTS.iter().enumerate().skip(1) {
        sum += coeff / (x + i as f64);
    }

    let t = x + G + 0.5;
    LN_SQRT_2PI + (t.ln() * (x + 0.5)) - t + sum.ln()
}

/// Stirling-series remainder: `ln Γ(a) - [(a - ½) ln a - a + ln √(2π)]`.
///
/// Uses the asymptotic series (7 terms, full double precision) for `a >= 10`.
#[must_use]
pub(crate) fn stirling_error(a: f64) -> f64 {
    if a >= 10.0 {
        let r = 1.0 / a;
        let r2 = r * r;
        r * (1.0 / 12.0
            - r2 * (1.0 / 360.0
                - r2 * (1.0 / 1260.0
                    - r2 * (1.0 / 1680.0
                        - r2 * (1.0 / 1188.0 - r2 * (691.0 / 360_360.0 - r2 / 156.0))))))
    } else {
        ln_gamma(a) - ((a - 0.5) * a.ln() - a + LN_SQRT_2PI)
    }
}

/// Deviance term `x ln(x/m) + m - x` (Loader's `bd0`), computed stably.
///
/// Used by the saddle-point Binomial PMF; the absolute error is about
/// `|x - m| · ε` instead of `x · ε` for the naive formula.
#[must_use]
pub(crate) fn bd0(x: f64, m: f64) -> f64 {
    if x == 0.0 {
        return m;
    }
    let u = (x - m) / m;
    m * ((1.0 + u) * u.ln_1p() - u)
}

/// `a ln x - x - ln Γ(a)`, computed without catastrophic cancellation for large `a`.
///
/// This is the log of the prefactor shared by the incomplete gamma series and
/// continued fraction, and (with `a = k + 1`) of the Poisson PMF.
#[must_use]
pub(crate) fn ln_gamma_kernel(a: f64, x: f64) -> f64 {
    if a < 10.0 {
        return a * x.ln() - x - ln_gamma(a);
    }
    // a ln x - x - lnΓ(a) = -a·(u - ln(1+u)) + ½ ln a - ln√(2π) - stirling_error(a),
    // where u = (x - a) / a.
    let u = (x - a) / a;
    -a * (u - u.ln_1p()) + 0.5 * a.ln() - LN_SQRT_2PI - stirling_error(a)
}

/// Natural log of B(a, b) = Γ(a) Γ(b) / Γ(a + b).
///
/// Uses Stirling-series corrections when an argument is large, avoiding the
/// cancellation of `lnΓ(a) + lnΓ(b) - lnΓ(a + b)` (same scheme as R's `lbeta`).
#[must_use]
pub(crate) fn ln_beta(a: f64, b: f64) -> f64 {
    let p = a.min(b);
    let q = a.max(b);
    if p >= 10.0 {
        // Both large.
        let corr = stirling_error(p) + stirling_error(q) - stirling_error(p + q);
        -0.5 * q.ln()
            + LN_SQRT_2PI
            + corr
            + (p - 0.5) * (p / (p + q)).ln()
            + q * (-p / (p + q)).ln_1p()
    } else if q >= 10.0 {
        // p small, q large.
        let corr = stirling_error(q) - stirling_error(p + q);
        ln_gamma(p) + corr + p - p * (p + q).ln() + (q - 0.5) * (-p / (p + q)).ln_1p()
    } else {
        ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
    }
}

// ---------------------------------------------------------------------------
// Regularized incomplete beta function  I_x(a, b)
// ---------------------------------------------------------------------------

/// Regularized incomplete beta function I_x(a, b).
#[must_use]
pub(crate) fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    beta_inc(a, b, x, 1.0 - x).0
}

/// Returns `(I_x(a, b), 1 - I_x(a, b))`, each computed without cancellation.
///
/// `y` must equal `1 - x`; passing it separately lets callers that know it
/// exactly (e.g. `t²/(ν + t²)`) avoid the rounding of `1 - x`.
/// Invalid shapes (non-positive or NaN) or NaN arguments give `(NaN, NaN)`.
#[must_use]
pub(crate) fn beta_inc(a: f64, b: f64, x: f64, y: f64) -> (f64, f64) {
    if x.is_nan() || y.is_nan() || a.is_nan() || b.is_nan() || a <= 0.0 || b <= 0.0 {
        return (f64::NAN, f64::NAN);
    }
    if x <= 0.0 {
        return (0.0, 1.0);
    }
    if y <= 0.0 {
        return (1.0, 0.0);
    }
    // The continued fraction converges fastest for x < (a+1)/(a+b+2); use the
    // symmetry I_x(a,b) = 1 - I_{1-x}(b,a) otherwise.
    if x < (a + 1.0) / (a + b + 2.0) {
        let p = beta_cf(a, b, x, y);
        (p, 1.0 - p)
    } else {
        let q = beta_cf(b, a, y, x);
        (1.0 - q, q)
    }
}

/// Lentz continued fraction for I_x(a, b) (valid for x < (a+1)/(a+b+2)).
fn beta_cf(a: f64, b: f64, x: f64, y: f64) -> f64 {
    // ln of x^a (1-x)^b / (a B(a, b)).
    let ln_prefix = if a >= 10.0 && b >= 10.0 {
        // Both shapes large: a ln x + b ln y - ln B(a, b) cancels badly, so use
        // the Stirling/deviance form (as in TOMS 708 `brcomp`):
        //   = -bd0(a, s x) - bd0(b, s y) + ½ ln(a b / s) - ln √(2π) - corr
        let s = a + b;
        let corr = stirling_error(a) + stirling_error(b) - stirling_error(s);
        -bd0(a, s * x) - bd0(b, s * y) + 0.5 * (a.ln() + b.ln() - s.ln())
            - LN_SQRT_2PI
            - corr
            - a.ln()
    } else {
        // Logs taken on the side that keeps precision.
        let ln_x = if y < 0.5 { (-y).ln_1p() } else { x.ln() };
        let ln_y = if x < 0.5 { (-x).ln_1p() } else { y.ln() };
        a * ln_x + b * ln_y - ln_beta(a, b) - a.ln()
    };
    let prefix = ln_prefix.exp();
    if prefix == 0.0 {
        return 0.0;
    }

    let max_iter = iteration_budget(a.max(b));
    let mut c = 1.0;
    // 1 - (a+b)x/(a+1), rewritten with y = 1 - x to avoid cancellation near x → 1.
    let mut d = ((1.0 - b) * x + (a + 1.0) * y) / (a + 1.0);
    if d.abs() < LENTZ_TINY {
        d = LENTZ_TINY;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=max_iter {
        let m_f64 = m as f64;
        let two_m = 2.0 * m_f64;

        // Even step
        let num_even = m_f64 * (b - m_f64) * x / ((a + two_m - 1.0) * (a + two_m));
        d = 1.0 + num_even * d;
        if d.abs() < LENTZ_TINY {
            d = LENTZ_TINY;
        }
        c = 1.0 + num_even / c;
        if c.abs() < LENTZ_TINY {
            c = LENTZ_TINY;
        }
        d = 1.0 / d;
        h *= d * c;

        // Odd step
        let num_odd = -((a + m_f64) * (a + b + m_f64) * x) / ((a + two_m) * (a + two_m + 1.0));
        d = 1.0 + num_odd * d;
        if d.abs() < LENTZ_TINY {
            d = LENTZ_TINY;
        }
        c = 1.0 + num_odd / c;
        if c.abs() < LENTZ_TINY {
            c = LENTZ_TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;

        if (delta - 1.0).abs() < CONVERGENCE_EPS {
            break;
        }
    }

    (prefix * h).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Regularized incomplete gamma functions  P(a, x), Q(a, x)
// ---------------------------------------------------------------------------

/// Regularized lower incomplete gamma P(a, x) = γ(a, x) / Γ(a).
#[must_use]
pub(crate) fn regularized_lower_incomplete_gamma(a: f64, x: f64) -> f64 {
    gamma_inc(a, x).0
}

/// Regularized upper incomplete gamma Q(a, x) = 1 - P(a, x), computed directly.
#[must_use]
pub(crate) fn regularized_upper_incomplete_gamma(a: f64, x: f64) -> f64 {
    gamma_inc(a, x).1
}

/// Returns `(P(a, x), Q(a, x))`, each computed without cancellation.
///
/// Uses the series expansion when `x < a + 1` and the continued fraction for
/// `Q` otherwise. Invalid shapes (non-positive or NaN) or NaN `x` give `(NaN, NaN)`.
#[must_use]
pub(crate) fn gamma_inc(a: f64, x: f64) -> (f64, f64) {
    if x.is_nan() || a.is_nan() || a <= 0.0 {
        return (f64::NAN, f64::NAN);
    }
    if x <= 0.0 {
        return (0.0, 1.0);
    }
    if x.is_infinite() {
        return (1.0, 0.0);
    }
    if a.is_infinite() {
        return (0.0, 1.0);
    }
    if a > GAMMA_WILSON_HILFERTY_SHAPE {
        // Wilson–Hilferty: (x/a)^(1/3) ~ N(1 - 1/(9a), 1/(9a)).
        let v = 1.0 / (9.0 * a);
        let z = ((x / a).cbrt() - (1.0 - v)) / v.sqrt();
        return (normal_cdf(z), normal_sf(z));
    }
    if x < a + 1.0 {
        let p = gamma_series_p(a, x);
        (p, 1.0 - p)
    } else {
        let q = gamma_cf_q(a, x);
        (1.0 - q, q)
    }
}

/// Series expansion for P(a, x).
fn gamma_series_p(a: f64, x: f64) -> f64 {
    let prefix = ln_gamma_kernel(a, x).exp();
    if prefix == 0.0 {
        return 0.0;
    }
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    for _ in 0..iteration_budget(a) {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() < sum.abs() * CONVERGENCE_EPS {
            break;
        }
    }
    (sum * prefix).clamp(0.0, 1.0)
}

/// Continued-fraction evaluation of Q(a, x) = 1 - P(a, x) via modified Lentz.
fn gamma_cf_q(a: f64, x: f64) -> f64 {
    let prefix = ln_gamma_kernel(a, x).exp();
    if prefix == 0.0 {
        return 0.0;
    }
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / LENTZ_TINY;
    let mut d = 1.0 / b;
    let mut h = d;

    for i in 1..=iteration_budget(a) {
        let i_f64 = i as f64;
        let an = -i_f64 * (i_f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < LENTZ_TINY {
            d = LENTZ_TINY;
        }
        c = b + an / c;
        if c.abs() < LENTZ_TINY {
            c = LENTZ_TINY;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < CONVERGENCE_EPS {
            break;
        }
    }

    (h * prefix).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative-error assertion helper.
    fn assert_rel(actual: f64, expected: f64, tol: f64, what: &str) {
        let err = if expected == 0.0 {
            actual.abs()
        } else {
            ((actual - expected) / expected).abs()
        };
        assert!(
            err <= tol,
            "{what}: got {actual:e}, expected {expected:e} (rel err {err:e})"
        );
    }

    #[test]
    fn erf_known_values() {
        assert!(erf(0.0).abs() < 1e-7);
        assert!((erf(10.0) - 1.0).abs() < 1e-7);
        assert!((erf(-10.0) + 1.0).abs() < 1e-7);
    }

    #[test]
    fn erfc_known_values() {
        assert!((erfc(0.0) - 1.0).abs() < 1e-7);
        assert!(erfc(10.0).abs() < 1e-7);
        assert!((erfc(-10.0) - 2.0).abs() < 1e-7);
    }

    #[test]
    fn erf_erfc_sum_to_one() {
        for &x in &[-2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0] {
            assert!(
                (erf(x) + erfc(x) - 1.0).abs() < 1e-15,
                "erf({x}) + erfc({x}) != 1"
            );
        }
    }

    #[test]
    fn erf_reference_values() {
        // Reference: C99 libm (glibc) erf/erfc.
        let cases = [
            (0.1, 0.112_462_916_018_284_9, 0.887_537_083_981_715),
            (0.5, 0.520_499_877_813_046_5, 0.479_500_122_186_953_5),
            (1.0, 0.842_700_792_949_714_9, 0.157_299_207_050_285_13),
            (3.0, 0.999_977_909_503_001_4, 2.209_049_699_858_544e-5),
            (5.0, 0.999_999_999_998_462_6, 1.537_459_794_428_035e-12),
        ];
        for (x, e, ec) in cases {
            assert_rel(erf(x), e, 4e-16, "erf");
            assert_rel(erfc(x), ec, 4e-16, "erfc");
            assert_rel(erf(-x), -e, 4e-16, "erf(-x)");
        }
        assert_rel(erfc(10.0), 2.088_487_583_762_545e-45, 1e-14, "erfc(10)");
        assert_rel(erfc(-1.0), 1.842_700_792_949_714_8, 4e-16, "erfc(-1)");
    }

    #[test]
    fn erf_special_values() {
        assert!(erf(f64::NAN).is_nan());
        assert!(erfc(f64::NAN).is_nan());
        assert_eq!(erf(f64::INFINITY), 1.0);
        assert_eq!(erf(f64::NEG_INFINITY), -1.0);
        assert_eq!(erfc(f64::INFINITY), 0.0);
        assert_eq!(erfc(f64::NEG_INFINITY), 2.0);
        assert_eq!(erfc(30.0), 0.0);
        // Tiny-argument path: erf(x) ≈ 2x/√π without underflow.
        assert_rel(erf(1e-300), 1e-300 * 2.0 / PI.sqrt(), 1e-15, "erf tiny");
    }

    #[test]
    fn normal_cdf_tails() {
        // Reference: 0.5 * erfc(-z / sqrt 2) from glibc.
        assert_rel(normal_cdf(-6.0), 9.865_876_450_377_014e-10, 1e-14, "Φ(-6)");
        assert_rel(normal_cdf(-1.0), 0.158_655_253_931_457_07, 1e-15, "Φ(-1)");
        assert_rel(normal_sf(6.0), 9.865_876_450_377_014e-10, 1e-14, "1-Φ(6)");
        assert!((normal_cdf(0.0) - 0.5).abs() < 1e-16);
    }

    #[test]
    fn ln_gamma_known_values() {
        // Gamma(1) = 1, ln(1) = 0
        assert!(ln_gamma(1.0).abs() < 1e-10);
        // Gamma(5) = 4! = 24, ln(24) ≈ 3.178
        assert!((ln_gamma(5.0) - 24.0_f64.ln()).abs() < 1e-8);
        // Gamma(0.5) = sqrt(pi)
        assert!((ln_gamma(0.5) - 0.5 * PI.ln()).abs() < 1e-8);
    }

    #[test]
    fn stirling_error_continuous_at_cutoff() {
        // Series branch (a >= 10) and direct branch (a < 10) must agree.
        let below = ln_gamma(10.0) - ((10.0 - 0.5) * 10.0_f64.ln() - 10.0 + LN_SQRT_2PI);
        assert!((stirling_error(10.0) - below).abs() < 1e-14);
    }

    #[test]
    fn ln_beta_matches_ln_gamma_and_is_stable() {
        for &(a, b) in &[
            (0.5, 0.5),
            (2.0, 3.0),
            (0.5, 12.0),
            (12.0, 0.5),
            (15.0, 25.0),
        ] {
            let naive = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);
            assert!((ln_beta(a, b) - naive).abs() < 1e-12, "ln_beta({a},{b})");
        }
        // B(a, 1) = 1/a exactly.
        assert_rel(ln_beta(1e7, 1.0), -(1e7_f64.ln()), 1e-14, "ln B(1e7, 1)");
    }

    #[test]
    fn ln_gamma_kernel_matches_direct_form() {
        for &(a, x) in &[(10.0, 9.0), (25.0, 30.0), (200.0, 190.0)] {
            let direct = a * f64::ln(x) - x - ln_gamma(a);
            assert!(
                (ln_gamma_kernel(a, x) - direct).abs() < 1e-11,
                "a={a} x={x}"
            );
        }
    }

    #[test]
    fn incomplete_gamma_known_values() {
        // P(1, 1) = 1 - e^{-1} ≈ 0.6321
        let p = regularized_lower_incomplete_gamma(1.0, 1.0);
        assert!((p - (1.0 - (-1.0_f64).exp())).abs() < 1e-15, "P(1,1) = {p}");
        // P(a, 0) = 0
        assert_eq!(regularized_lower_incomplete_gamma(3.0, 0.0), 0.0);
        // P(a, inf) = 1
        assert_eq!(regularized_lower_incomplete_gamma(3.0, f64::INFINITY), 1.0);
        // P(3, 3) (exact Poisson-sum reference)
        let p = regularized_lower_incomplete_gamma(3.0, 3.0);
        assert_rel(p, 0.576_809_918_873_156_5, 1e-14, "P(3,3)");
    }

    #[test]
    fn incomplete_gamma_upper_tail_reference() {
        // References: Q(n, x) = e^{-x} Σ_{i<n} x^i/i! evaluated in 50-digit decimal.
        let cases = [
            (10.0, 9.5, 0.521_826_022_237_207_4),
            (50.0, 100.0, 1.178_450_072_097_942_2e-8),
            (1000.0, 1010.0, 0.372_321_055_263_005_3),
            (5000.0, 5000.0, 0.498_119_365_966_182_6),
            (500_000.0, 500_000.0, 0.499_811_936_803_394_5),
        ];
        for (a, x, q) in cases {
            let (p_got, q_got) = gamma_inc(a, x);
            assert_rel(q_got, q, 1e-11, &format!("Q({a},{x})"));
            assert!((p_got + q_got - 1.0).abs() < 1e-15);
        }
    }

    #[test]
    fn incomplete_gamma_wilson_hilferty_continuity() {
        // Just below and just above the Wilson-Hilferty switch must agree.
        let a = GAMMA_WILSON_HILFERTY_SHAPE;
        let x = a + 2.0 * a.sqrt();
        let below = gamma_inc(a * (1.0 - 1e-12), x).0;
        let above = gamma_inc(a * (1.0 + 1e-12), x).0;
        assert!((below - above).abs() < 1e-8, "{below} vs {above}");
    }

    #[test]
    fn incomplete_gamma_invalid() {
        assert!(gamma_inc(f64::NAN, 1.0).0.is_nan());
        assert!(gamma_inc(1.0, f64::NAN).0.is_nan());
        assert!(gamma_inc(-1.0, 1.0).0.is_nan());
        assert_eq!(gamma_inc(f64::INFINITY, 5.0), (0.0, 1.0));
    }

    #[test]
    fn incomplete_beta_known_values() {
        // I_0.5(1, 1) = 0.5
        let v = regularized_incomplete_beta(0.5, 1.0, 1.0);
        assert!((v - 0.5).abs() < 1e-15, "I_0.5(1,1) = {v}");
        // I_x(1, 1) = x
        let v = regularized_incomplete_beta(0.3, 1.0, 1.0);
        assert!((v - 0.3).abs() < 1e-15, "I_0.3(1,1) = {v}");
        // I_x(a, 1) = x^a
        let v = regularized_incomplete_beta(0.7, 3.5, 1.0);
        assert_rel(v, 0.7_f64.powf(3.5), 1e-14, "I_0.7(3.5,1)");
        // I_x(1, b) = 1 - (1-x)^b
        let v = regularized_incomplete_beta(0.2, 1.0, 4.0);
        assert_rel(v, 1.0 - 0.8_f64.powi(4), 1e-14, "I_0.2(1,4)");
    }

    #[test]
    fn incomplete_beta_complement_keeps_tail_precision() {
        // 1 - I_x(1, b) = (1-x)^b: tiny upper tail must not round to 0.
        let (_, q) = beta_inc(1.0, 40.0, 0.9, 0.1);
        assert_rel(q, 0.1_f64.powi(40), 1e-13, "upper tail");
        let (p, q) = beta_inc(2.5, 3.5, 0.4, 0.6);
        assert!((p + q - 1.0).abs() < 1e-15);
    }

    #[test]
    fn incomplete_beta_large_shape() {
        // F(2, d2) CDF has the closed form 1 - (1 + 2x/d2)^(-d2/2), i.e.
        // I_u(1, d2/2) with u = 2x/(2x + d2).
        for &d2 in &[10.0_f64, 1e4, 1e6, 1e8] {
            let x: f64 = 3.0;
            let u = 2.0 * x / (2.0 * x + d2);
            let expected = 1.0 - (-d2 / 2.0 * (2.0 * x / d2).ln_1p()).exp();
            let got = beta_inc(1.0, d2 / 2.0, u, d2 / (2.0 * x + d2)).0;
            assert_rel(got, expected, 1e-12, &format!("F(2,{d2})"));
        }
    }

    #[test]
    fn incomplete_beta_invalid() {
        assert!(beta_inc(f64::NAN, 1.0, 0.5, 0.5).0.is_nan());
        assert!(beta_inc(1.0, 0.0, 0.5, 0.5).0.is_nan());
        assert!(regularized_incomplete_beta(f64::NAN, 1.0, 1.0).is_nan());
        assert_eq!(regularized_incomplete_beta(-1.0, 2.0, 2.0), 0.0);
        assert_eq!(regularized_incomplete_beta(2.0, 2.0, 2.0), 1.0);
    }
}
