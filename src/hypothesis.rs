//! Hypothesis testing: t-tests and chi-squared test.

use crate::descriptive;
use crate::error::PramanaError;
use crate::math::{erfc, regularized_incomplete_beta};
use serde::{Deserialize, Serialize};

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
