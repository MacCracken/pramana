//! Shared mathematical helpers (crate-internal).

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Error function
// ---------------------------------------------------------------------------

/// Approximation of the error function using the Abramowitz & Stegun formula 7.1.26.
/// Maximum error: |epsilon| < 1.5e-7.
#[must_use]
#[inline]
pub(crate) fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    const P: f64 = 0.3275911;
    const A1: f64 = 0.254829592;
    const A2: f64 = -0.284496736;
    const A3: f64 = 1.421413741;
    const A4: f64 = -1.453152027;
    const A5: f64 = 1.061405429;

    let t = 1.0 / (1.0 + P * x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let y = 1.0 - (A1 * t + A2 * t2 + A3 * t3 + A4 * t4 + A5 * t5) * (-x * x).exp();
    sign * y
}

/// Complementary error function: erfc(x) = 1 - erf(x).
///
/// Uses the same Abramowitz & Stegun 7.1.26 polynomial directly for better
/// numerical precision in the tails.
#[must_use]
#[inline]
pub(crate) fn erfc(x: f64) -> f64 {
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }

    const P: f64 = 0.3275911;
    const A1: f64 = 0.254829592;
    const A2: f64 = -0.284496736;
    const A3: f64 = 1.421413741;
    const A4: f64 = -1.453152027;
    const A5: f64 = 1.061405429;

    let t = 1.0 / (1.0 + P * x);
    let poly = A1 * t + A2 * t.powi(2) + A3 * t.powi(3) + A4 * t.powi(4) + A5 * t.powi(5);
    poly * (-x * x).exp()
}

// ---------------------------------------------------------------------------
// Gamma and beta functions
// ---------------------------------------------------------------------------

/// Natural log of Gamma(x) via the Lanczos approximation (g=7, n=9).
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
    0.5 * (2.0 * PI).ln() + (t.ln() * (x + 0.5)) - t + sum.ln()
}

/// Natural log of B(a, b) = Gamma(a) * Gamma(b) / Gamma(a + b).
#[must_use]
#[inline]
pub(crate) fn ln_beta(a: f64, b: f64) -> f64 {
    ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
}

// ---------------------------------------------------------------------------
// Regularized incomplete beta function  I_x(a, b)
// ---------------------------------------------------------------------------

/// Regularized incomplete beta function I_x(a, b) via Lentz continued fraction.
#[must_use]
pub(crate) fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Use the symmetry relation for better convergence
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - regularized_incomplete_beta(1.0 - x, b, a);
    }

    let ln_prefix = a * x.ln() + b * (1.0 - x).ln() - ln_beta(a, b) - a.ln();
    let prefix = ln_prefix.exp();

    let max_iter = 200;
    let eps = 1e-14;
    let tiny = 1e-30;

    let mut c = 1.0;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=max_iter {
        let m_f64 = m as f64;

        // Even step
        let num_even = m_f64 * (b - m_f64) * x / ((a + 2.0 * m_f64 - 1.0) * (a + 2.0 * m_f64));
        d = 1.0 + num_even * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + num_even / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        h *= d * c;

        // Odd step
        let num_odd =
            -((a + m_f64) * (a + b + m_f64) * x) / ((a + 2.0 * m_f64) * (a + 2.0 * m_f64 + 1.0));
        d = 1.0 + num_odd * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + num_odd / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;

        if (delta - 1.0).abs() < eps {
            break;
        }
    }

    (prefix * h).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Regularized lower incomplete gamma function  P(a, x)
// ---------------------------------------------------------------------------

/// Regularized lower incomplete gamma P(a, x) = gamma(a, x) / Gamma(a).
///
/// Uses the series expansion when x < a + 1 and the continued fraction
/// (for Q = 1 - P) otherwise.
#[must_use]
pub(crate) fn regularized_lower_incomplete_gamma(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x.is_infinite() {
        return 1.0;
    }
    if x < a + 1.0 {
        gamma_series_p(a, x)
    } else {
        1.0 - gamma_cf_q(a, x)
    }
}

/// Series expansion for P(a, x).
fn gamma_series_p(a: f64, x: f64) -> f64 {
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    for _ in 0..200 {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() < sum.abs() * 1e-14 {
            break;
        }
    }
    let log_prefix = a * x.ln() - x - ln_gamma(a);
    (sum * log_prefix.exp()).clamp(0.0, 1.0)
}

/// Continued-fraction evaluation of Q(a, x) = 1 - P(a, x) via modified Lentz.
fn gamma_cf_q(a: f64, x: f64) -> f64 {
    let tiny = 1e-30;

    let mut b = x + 1.0 - a;
    let mut c = 1.0 / tiny;
    let mut d = 1.0 / b;
    let mut h = d;

    for i in 1..=200 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < tiny {
            d = tiny;
        }
        c = b + an / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-14 {
            break;
        }
    }

    let log_prefix = a * x.ln() - x - ln_gamma(a);
    (h * log_prefix.exp()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

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
                (erf(x) + erfc(x) - 1.0).abs() < 1e-7,
                "erf({x}) + erfc({x}) != 1"
            );
        }
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
    fn incomplete_gamma_known_values() {
        // P(1, 1) = 1 - e^{-1} ≈ 0.6321
        let p = regularized_lower_incomplete_gamma(1.0, 1.0);
        assert!((p - (1.0 - (-1.0_f64).exp())).abs() < 1e-6, "P(1,1) = {p}");
        // P(a, 0) = 0
        assert_eq!(regularized_lower_incomplete_gamma(3.0, 0.0), 0.0);
        // P(a, inf) = 1
        assert_eq!(regularized_lower_incomplete_gamma(3.0, f64::INFINITY), 1.0);
        // P(3, 3) ≈ 0.5768 (from tables)
        let p = regularized_lower_incomplete_gamma(3.0, 3.0);
        assert!((p - 0.5768).abs() < 0.001, "P(3,3) = {p}");
    }

    #[test]
    fn incomplete_beta_known_values() {
        // I_0.5(1, 1) = 0.5
        let v = regularized_incomplete_beta(0.5, 1.0, 1.0);
        assert!((v - 0.5).abs() < 1e-6, "I_0.5(1,1) = {v}");
        // I_x(1, 1) = x
        let v = regularized_incomplete_beta(0.3, 1.0, 1.0);
        assert!((v - 0.3).abs() < 1e-6, "I_0.3(1,1) = {v}");
    }
}
