//! Shared mathematical helpers (crate-internal).

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
}
