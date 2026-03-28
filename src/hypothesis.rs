//! Hypothesis testing: t-tests and chi-squared test.

use crate::descriptive;
use crate::error::PramanaError;
use crate::math::erfc;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Result of a statistical hypothesis test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Name of the test performed.
    pub test_name: String,
    /// The test statistic.
    pub statistic: f64,
    /// Approximate p-value.
    pub p_value: f64,
    /// Degrees of freedom.
    pub degrees_of_freedom: f64,
    /// Significance level used for the rejection decision.
    pub reject_at_alpha: f64,
    /// Whether the null hypothesis is rejected at the given alpha.
    pub reject: bool,
}

/// One-sample t-test: tests whether the population mean equals `mu_0`.
///
/// Two-tailed test at the given significance level `alpha`.
///
/// # Errors
///
/// Returns `InvalidSample` if `data` has fewer than 2 elements or zero variance.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
#[must_use = "returns the test result"]
pub fn t_test_one_sample(data: &[f64], mu_0: f64, alpha: f64) -> Result<TestResult, PramanaError> {
    validate_alpha(alpha)?;
    if data.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations".into(),
        ));
    }
    let n = data.len() as f64;
    let m = descriptive::mean(data)?;

    // Sample variance (Bessel-corrected)
    let sample_var = data.iter().map(|&x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0);
    if sample_var == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero variance in sample".into(),
        ));
    }
    let se = (sample_var / n).sqrt();
    let t = (m - mu_0) / se;
    let df = n - 1.0;
    let p = two_tailed_t_pvalue(t, df);

    Ok(TestResult {
        test_name: "one-sample t-test".into(),
        statistic: t,
        p_value: p,
        degrees_of_freedom: df,
        reject_at_alpha: alpha,
        reject: p < alpha,
    })
}

/// Two-sample independent t-test (Welch's t-test, unequal variances).
///
/// Two-tailed test at the given significance level `alpha`.
///
/// # Errors
///
/// Returns `InvalidSample` if either sample has fewer than 2 elements or zero variance.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
#[must_use = "returns the test result"]
pub fn t_test_two_sample(a: &[f64], b: &[f64], alpha: f64) -> Result<TestResult, PramanaError> {
    validate_alpha(alpha)?;
    if a.len() < 2 || b.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations in each sample".into(),
        ));
    }
    let n1 = a.len() as f64;
    let n2 = b.len() as f64;
    let m1 = descriptive::mean(a)?;
    let m2 = descriptive::mean(b)?;
    let var1 = a.iter().map(|&x| (x - m1) * (x - m1)).sum::<f64>() / (n1 - 1.0);
    let var2 = b.iter().map(|&x| (x - m2) * (x - m2)).sum::<f64>() / (n2 - 1.0);

    if var1 == 0.0 && var2 == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero variance in both samples".into(),
        ));
    }

    let se = (var1 / n1 + var2 / n2).sqrt();
    let t = (m1 - m2) / se;

    // Welch-Satterthwaite degrees of freedom
    let num = (var1 / n1 + var2 / n2).powi(2);
    let denom = (var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0);
    let df = if denom == 0.0 { 1.0 } else { num / denom };

    let p = two_tailed_t_pvalue(t, df);

    Ok(TestResult {
        test_name: "two-sample Welch t-test".into(),
        statistic: t,
        p_value: p,
        degrees_of_freedom: df,
        reject_at_alpha: alpha,
        reject: p < alpha,
    })
}

/// Chi-squared goodness-of-fit test.
///
/// Tests whether `observed` frequencies match `expected` frequencies at the
/// given significance level `alpha`.
///
/// # Errors
///
/// Returns `DimensionMismatch` if slices differ in length.
/// Returns `InvalidSample` if `expected` contains zeros or slices are empty.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
#[must_use = "returns the test result"]
pub fn chi_squared_test(
    observed: &[f64],
    expected: &[f64],
    alpha: f64,
) -> Result<TestResult, PramanaError> {
    validate_alpha(alpha)?;
    if observed.len() != expected.len() {
        return Err(PramanaError::DimensionMismatch(
            "observed and expected must have the same length".into(),
        ));
    }
    if observed.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    for &e in expected {
        if e <= 0.0 {
            return Err(PramanaError::InvalidSample(
                "expected frequencies must be positive".into(),
            ));
        }
    }

    let chi2: f64 = observed
        .iter()
        .zip(expected.iter())
        .map(|(&o, &e)| (o - e) * (o - e) / e)
        .sum();

    let df = (observed.len() - 1) as f64;
    let p = chi_squared_pvalue(chi2, df);

    Ok(TestResult {
        test_name: "chi-squared test".into(),
        statistic: chi2,
        p_value: p,
        degrees_of_freedom: df,
        reject_at_alpha: alpha,
        reject: p < alpha,
    })
}

/// Validates that alpha is a valid significance level in `(0, 1)`.
fn validate_alpha(alpha: f64) -> Result<(), PramanaError> {
    if alpha <= 0.0 || alpha >= 1.0 {
        return Err(PramanaError::InvalidParameter(
            "alpha must be in (0, 1)".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Approximate p-value computation
// ---------------------------------------------------------------------------

/// Approximation of the two-tailed p-value for a t-distribution.
///
/// Uses the approximation: for large df, t ~ normal. For small df, use
/// the regularized incomplete beta function approximation.
fn two_tailed_t_pvalue(t: f64, df: f64) -> f64 {
    // Use the relationship: p = I(df/(df+t^2); df/2, 1/2)
    // where I is the regularized incomplete beta function.
    let x = df / (df + t * t);
    let p = regularized_incomplete_beta(x, df / 2.0, 0.5);
    // Two-tailed: this gives us directly the two-tailed p-value
    p.clamp(0.0, 1.0)
}

/// Approximation of the upper-tail p-value for a chi-squared distribution.
///
/// P(X > chi2) where X ~ chi-squared(df).
/// Uses the Wilson-Hilferty normal approximation.
fn chi_squared_pvalue(chi2: f64, df: f64) -> f64 {
    if df <= 0.0 {
        return 1.0;
    }
    // Wilson-Hilferty approximation
    let z = ((chi2 / df).powf(1.0 / 3.0) - (1.0 - 2.0 / (9.0 * df))) / (2.0 / (9.0 * df)).sqrt();
    // Upper tail of standard normal
    normal_upper_tail(z)
}

/// P(Z > z) for standard normal Z.
fn normal_upper_tail(z: f64) -> f64 {
    0.5 * erfc(z / std::f64::consts::SQRT_2)
}

/// Regularized incomplete beta function approximation using a continued fraction.
///
/// I_x(a, b) = B_x(a, b) / B(a, b)
///
/// This uses the Lentz continued fraction method.
fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Use the symmetry relation if x > (a+1)/(a+b+2) for better convergence
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - regularized_incomplete_beta(1.0 - x, b, a);
    }

    let ln_prefix = a * x.ln() + b * (1.0 - x).ln() - ln_beta(a, b) - a.ln();
    let prefix = ln_prefix.exp();

    // Lentz continued fraction for I_x(a, b)
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
        let numerator_even =
            m_f64 * (b - m_f64) * x / ((a + 2.0 * m_f64 - 1.0) * (a + 2.0 * m_f64));

        d = 1.0 + numerator_even * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + numerator_even / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        h *= d * c;

        // Odd step
        let numerator_odd =
            -((a + m_f64) * (a + b + m_f64) * x) / ((a + 2.0 * m_f64) * (a + 2.0 * m_f64 + 1.0));

        d = 1.0 + numerator_odd * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + numerator_odd / c;
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

/// Natural log of the beta function B(a, b) = Gamma(a)*Gamma(b)/Gamma(a+b).
fn ln_beta(a: f64, b: f64) -> f64 {
    ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
}

/// Stirling's approximation to ln(Gamma(x)) for x > 0.
/// Uses the Lanczos approximation for better accuracy.
fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation with g=7, n=9
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_test_one_sample_zero_mean() {
        // Data centered around 0 should not reject H0: mu=0
        let data = [-1.0, -0.5, 0.0, 0.5, 1.0];
        let result = t_test_one_sample(&data, 0.0, 0.05).unwrap();
        assert!(!result.reject, "should not reject for centered data");
    }

    #[test]
    fn t_test_one_sample_shifted() {
        // Data clearly above 0 should reject H0: mu=0
        let data = [10.0, 10.1, 9.9, 10.2, 9.8, 10.0, 10.1, 9.9];
        let result = t_test_one_sample(&data, 0.0, 0.05).unwrap();
        assert!(result.reject, "should reject for shifted data");
    }

    #[test]
    fn t_test_two_sample_same() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [1.1, 2.1, 2.9, 4.1, 4.9];
        let result = t_test_two_sample(&a, &b, 0.05).unwrap();
        assert!(
            !result.reject,
            "should not reject for similar distributions"
        );
    }

    #[test]
    fn chi_squared_good_fit() {
        // Observed matches expected well
        let observed = [50.0, 50.0, 50.0, 50.0];
        let expected = [50.0, 50.0, 50.0, 50.0];
        let result = chi_squared_test(&observed, &expected, 0.05).unwrap();
        assert!(!result.reject, "perfect fit should not reject");
        assert!((result.statistic).abs() < 1e-10, "chi2 should be 0");
    }

    #[test]
    fn chi_squared_dimension_mismatch() {
        assert!(chi_squared_test(&[1.0, 2.0], &[1.0], 0.05).is_err());
    }

    #[test]
    fn invalid_alpha() {
        let data = [1.0, 2.0, 3.0];
        assert!(t_test_one_sample(&data, 0.0, 0.0).is_err());
        assert!(t_test_one_sample(&data, 0.0, 1.0).is_err());
        assert!(t_test_one_sample(&data, 0.0, -0.1).is_err());
    }

    #[test]
    fn test_result_serde() {
        let r = TestResult {
            test_name: "test".into(),
            statistic: 1.5,
            p_value: 0.05,
            degrees_of_freedom: 4.0,
            reject_at_alpha: 0.05,
            reject: true,
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: TestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r.test_name, r2.test_name);
        assert_eq!(r.statistic, r2.statistic);
    }
}
