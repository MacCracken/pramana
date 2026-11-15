//! Hypothesis testing and confidence intervals.
//!
//! All tests reject non-finite input data and NaN significance/confidence
//! levels with an error instead of returning a NaN p-value (which would make
//! `reject` silently `false`).

use crate::descriptive;
use crate::distribution::{f_cdf_sf, student_t_cdf, student_t_two_tailed};
use crate::error::PramanaError;
use crate::math::{normal_cdf, regularized_upper_incomplete_gamma};
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
/// Returns `InvalidSample` if `data` has fewer than 2 elements, contains
/// non-finite values, or has zero variance.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)` or `mu_0` is not finite.
#[must_use = "returns the test result"]
pub fn t_test_one_sample(data: &[f64], mu_0: f64, alpha: f64) -> Result<TestResult, PramanaError> {
    validate_alpha(alpha)?;
    if !mu_0.is_finite() {
        return Err(PramanaError::InvalidParameter("mu_0 must be finite".into()));
    }
    if data.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations".into(),
        ));
    }
    let n = data.len() as f64;
    let (m, sample_var) = mean_and_sample_variance(data, "data")?;
    if sample_var == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero variance in sample".into(),
        ));
    }
    let se = (sample_var / n).sqrt();
    let t = (m - mu_0) / se;
    let df = n - 1.0;
    let p = student_t_two_tailed(t, df);

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
/// Returns `InvalidSample` if either sample has fewer than 2 elements or
/// contains non-finite values, or if both samples have zero variance.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
#[must_use = "returns the test result"]
pub fn t_test_two_sample(a: &[f64], b: &[f64], alpha: f64) -> Result<TestResult, PramanaError> {
    validate_alpha(alpha)?;
    let welch = welch_stats(a, b)?;
    let p = student_t_two_tailed(welch.diff / welch.se, welch.df);

    Ok(TestResult {
        test_name: "two-sample Welch t-test".into(),
        statistic: welch.diff / welch.se,
        p_value: p,
        degrees_of_freedom: welch.df,
        reject_at_alpha: alpha,
        reject: p < alpha,
    })
}

/// Shared Welch two-sample statistics.
struct Welch {
    /// Difference of means (a - b).
    diff: f64,
    /// Standard error of the difference.
    se: f64,
    /// Welch-Satterthwaite degrees of freedom.
    df: f64,
}

fn welch_stats(a: &[f64], b: &[f64]) -> Result<Welch, PramanaError> {
    if a.len() < 2 || b.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations in each sample".into(),
        ));
    }
    let n1 = a.len() as f64;
    let n2 = b.len() as f64;
    let (m1, var1) = mean_and_sample_variance(a, "a")?;
    let (m2, var2) = mean_and_sample_variance(b, "b")?;

    if var1 == 0.0 && var2 == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero variance in both samples".into(),
        ));
    }

    let v1 = var1 / n1;
    let v2 = var2 / n2;
    let se = (v1 + v2).sqrt();

    // Welch-Satterthwaite degrees of freedom
    let num = (v1 + v2).powi(2);
    let denom = v1.powi(2) / (n1 - 1.0) + v2.powi(2) / (n2 - 1.0);
    let df = if denom == 0.0 { 1.0 } else { num / denom };

    Ok(Welch {
        diff: m1 - m2,
        se,
        df,
    })
}

/// Chi-squared goodness-of-fit test.
///
/// Tests whether `observed` frequencies match `expected` frequencies at the
/// given significance level `alpha`. The p-value is the exact upper tail of
/// the chi-squared distribution with `k - 1` degrees of freedom.
///
/// # Errors
///
/// Returns `DimensionMismatch` if slices differ in length.
/// Returns `InvalidSample` if slices are empty, `expected` contains values
/// that are not positive and finite, or `observed` contains negative or
/// non-finite values.
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
    if expected.iter().any(|&e| !(e.is_finite() && e > 0.0)) {
        return Err(PramanaError::InvalidSample(
            "expected frequencies must be positive and finite".into(),
        ));
    }
    if observed.iter().any(|&o| !(o.is_finite() && o >= 0.0)) {
        return Err(PramanaError::InvalidSample(
            "observed frequencies must be non-negative and finite".into(),
        ));
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

/// Validates that alpha is a valid significance level in `(0, 1)` (NaN rejected).
fn validate_alpha(alpha: f64) -> Result<(), PramanaError> {
    if !(alpha > 0.0 && alpha < 1.0) {
        return Err(PramanaError::InvalidParameter(
            "alpha must be in (0, 1)".into(),
        ));
    }
    Ok(())
}

/// Validates that a confidence level is in `(0, 1)` (NaN rejected).
fn validate_confidence(confidence_level: f64) -> Result<(), PramanaError> {
    if !(confidence_level > 0.0 && confidence_level < 1.0) {
        return Err(PramanaError::InvalidParameter(
            "confidence_level must be in (0, 1)".into(),
        ));
    }
    Ok(())
}

/// Returns `InvalidSample` unless every summary statistic is finite.
///
/// Any NaN or infinite input makes the mean (and hence the variance)
/// non-finite, so checking the aggregates is equivalent to scanning the data
/// -- without the extra pass. It also catches finite data whose sums overflow.
fn require_finite_summary(stats: &[f64], name: &str) -> Result<(), PramanaError> {
    if stats.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(PramanaError::InvalidSample(format!(
            "{name} contains non-finite values (or its sums overflow)"
        )))
    }
}

/// Copies `data` and sorts it ascending.
///
/// Returns `InvalidSample` if `data` contains NaN or infinite values. Because
/// the copy is NaN-free, `partial_cmp` is a total order here (the sort cannot
/// panic) and it sorts faster than `total_cmp`.
fn sorted_finite(data: &[f64], name: &str) -> Result<Vec<f64>, PramanaError> {
    if let Some(i) = data.iter().position(|v| !v.is_finite()) {
        return Err(PramanaError::InvalidSample(format!(
            "{name}[{i}] is not finite"
        )));
    }
    let mut sorted = data.to_vec();
    sorted.sort_unstable_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    Ok(sorted)
}

/// Mean and Bessel-corrected sample variance; `data.len() >= 2`.
fn mean_and_sample_variance(data: &[f64], name: &str) -> Result<(f64, f64), PramanaError> {
    let n = data.len() as f64;
    let m = descriptive::mean(data)?;
    let var = data.iter().map(|&x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0);
    require_finite_summary(&[m, var], name)?;
    Ok((m, var))
}

// ---------------------------------------------------------------------------
// p-value computation
// ---------------------------------------------------------------------------

/// Upper-tail p-value P(X > chi2) for X ~ chi-squared(df), computed exactly
/// via the regularized upper incomplete gamma function Q(df/2, chi2/2).
fn chi_squared_pvalue(chi2: f64, df: f64) -> f64 {
    if df <= 0.0 {
        return 1.0;
    }
    regularized_upper_incomplete_gamma(df / 2.0, chi2 / 2.0)
}

// ---------------------------------------------------------------------------
// ANOVA
// ---------------------------------------------------------------------------

/// Result of a one-way ANOVA test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnovaResult {
    /// Between-group sum of squares.
    pub ss_between: f64,
    /// Within-group sum of squares.
    pub ss_within: f64,
    /// Total sum of squares.
    pub ss_total: f64,
    /// Between-group degrees of freedom (k - 1).
    pub df_between: usize,
    /// Within-group degrees of freedom (N - k).
    pub df_within: usize,
    /// Between-group mean square (SS_between / df_between).
    pub ms_between: f64,
    /// Within-group mean square (SS_within / df_within).
    pub ms_within: f64,
    /// F-statistic (MS_between / MS_within).
    pub f_statistic: f64,
    /// p-value from the F-distribution.
    pub p_value: f64,
    /// Significance level used.
    pub reject_at_alpha: f64,
    /// Whether the null hypothesis (all group means equal) is rejected.
    pub reject: bool,
}

/// One-way analysis of variance (ANOVA).
///
/// Tests whether the means of `k` groups are all equal. Each element of
/// `groups` is a slice of observations for one group.
///
/// If every group has zero spread but the group means differ, the F-statistic
/// is infinite and the p-value is 0.
///
/// # Errors
///
/// Returns `InvalidSample` if fewer than 2 groups, any group is empty,
/// total observations are fewer than the number of groups + 1, any value is
/// non-finite, or all observations are identical (zero total variance).
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
#[must_use = "returns the ANOVA result"]
pub fn one_way_anova(groups: &[&[f64]], alpha: f64) -> Result<AnovaResult, PramanaError> {
    validate_alpha(alpha)?;
    let k = groups.len();
    if k < 2 {
        return Err(PramanaError::InvalidSample("need at least 2 groups".into()));
    }
    for (i, group) in groups.iter().enumerate() {
        if group.is_empty() {
            return Err(PramanaError::InvalidSample(format!("group {i} is empty")));
        }
    }
    let n_total: usize = groups.iter().map(|g| g.len()).sum();
    if n_total <= k {
        return Err(PramanaError::InvalidSample(
            "need more observations than groups".into(),
        ));
    }

    // Grand mean
    let grand_sum: f64 = groups.iter().flat_map(|g| g.iter()).sum();
    let grand_mean = grand_sum / n_total as f64;

    // SS between and SS within
    let mut ss_between = 0.0;
    let mut ss_within = 0.0;
    for group in groups {
        let ni = group.len() as f64;
        let group_mean: f64 = group.iter().sum::<f64>() / ni;
        ss_between += ni * (group_mean - grand_mean).powi(2);
        for &x in *group {
            ss_within += (x - group_mean).powi(2);
        }
    }
    let ss_total = ss_between + ss_within;
    require_finite_summary(&[grand_mean, ss_between, ss_within], "groups")?;
    if ss_total == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero total variance: all observations are identical".into(),
        ));
    }

    let df_between = k - 1;
    let df_within = n_total - k;
    let ms_between = ss_between / df_between as f64;
    let ms_within = ss_within / df_within as f64;

    let f_statistic = if ms_within > 0.0 {
        ms_between / ms_within
    } else {
        f64::INFINITY
    };

    // p-value: P(F > f_statistic), computed directly (no 1 - cdf cancellation)
    let p_value = f_cdf_sf(f_statistic, df_between as f64, df_within as f64)
        .1
        .clamp(0.0, 1.0);

    Ok(AnovaResult {
        ss_between,
        ss_within,
        ss_total,
        df_between,
        df_within,
        ms_between,
        ms_within,
        f_statistic,
        p_value,
        reject_at_alpha: alpha,
        reject: p_value < alpha,
    })
}

// ---------------------------------------------------------------------------
// Kolmogorov-Smirnov test
// ---------------------------------------------------------------------------

/// Result of a Kolmogorov-Smirnov test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KsResult {
    /// The KS statistic D: maximum absolute difference between the CDFs.
    pub statistic: f64,
    /// Approximate p-value.
    pub p_value: f64,
    /// Significance level used.
    pub reject_at_alpha: f64,
    /// Whether the null hypothesis (distributions are equal) is rejected.
    pub reject: bool,
}

/// Two-sample Kolmogorov-Smirnov test.
///
/// Tests whether two samples come from the same continuous distribution by
/// comparing their empirical CDFs. Tied values (within or across samples) are
/// handled by evaluating the ECDFs only after all observations at a value.
///
/// # Errors
///
/// Returns `InvalidSample` if either sample is empty or contains non-finite values.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
#[must_use = "returns the KS test result"]
pub fn ks_two_sample(a: &[f64], b: &[f64], alpha: f64) -> Result<KsResult, PramanaError> {
    validate_alpha(alpha)?;
    if a.is_empty() || b.is_empty() {
        return Err(PramanaError::InvalidSample(
            "both samples must be non-empty".into(),
        ));
    }
    let sa = sorted_finite(a, "a")?;
    let sb = sorted_finite(b, "b")?;

    let na = sa.len() as f64;
    let nb = sb.len() as f64;
    let mut d_max: f64 = 0.0;
    let mut ia = 0;
    let mut ib = 0;

    while ia < sa.len() && ib < sb.len() {
        // Advance past every observation equal to the next smallest value in
        // *both* samples before comparing the ECDFs.
        let v = sa[ia].min(sb[ib]);
        while ia < sa.len() && sa[ia] == v {
            ia += 1;
        }
        while ib < sb.len() && sb[ib] == v {
            ib += 1;
        }
        d_max = d_max.max((ia as f64 / na - ib as f64 / nb).abs());
    }

    let n_eff = (na * nb) / (na + nb);
    let p_value = ks_pvalue(d_max, n_eff);

    Ok(KsResult {
        statistic: d_max,
        p_value,
        reject_at_alpha: alpha,
        reject: p_value < alpha,
    })
}

/// One-sample Kolmogorov-Smirnov test.
///
/// Tests whether a sample comes from the distribution described by the
/// given CDF function.
///
/// # Errors
///
/// Returns `InvalidSample` if the sample is empty or contains non-finite values.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`, or if `cdf`
/// returns NaN for any sample point.
#[must_use = "returns the KS test result"]
pub fn ks_one_sample(
    data: &[f64],
    cdf: impl Fn(f64) -> f64,
    alpha: f64,
) -> Result<KsResult, PramanaError> {
    validate_alpha(alpha)?;
    if data.is_empty() {
        return Err(PramanaError::InvalidSample(
            "sample must be non-empty".into(),
        ));
    }
    let sorted = sorted_finite(data, "data")?;

    let n = sorted.len() as f64;
    let mut d_max: f64 = 0.0;

    for (i, &x) in sorted.iter().enumerate() {
        let theoretical = cdf(x);
        if theoretical.is_nan() {
            return Err(PramanaError::InvalidParameter(format!(
                "cdf returned NaN at x = {x}"
            )));
        }
        let theoretical = theoretical.clamp(0.0, 1.0);
        let ecdf_before = i as f64 / n;
        let ecdf_after = (i + 1) as f64 / n;
        d_max = d_max.max((ecdf_after - theoretical).abs());
        d_max = d_max.max((theoretical - ecdf_before).abs());
    }

    let p_value = ks_pvalue(d_max, n);

    Ok(KsResult {
        statistic: d_max,
        p_value,
        reject_at_alpha: alpha,
        reject: p_value < alpha,
    })
}

/// Approximate p-value for the KS statistic using the Kolmogorov distribution
/// with Stephens' small-sample correction λ = (√n + 0.12 + 0.11/√n)·d.
///
/// Uses the two classical series for the Kolmogorov survival function, each
/// in the range where it converges to double precision within four terms:
/// - λ ≥ 1.18: Q(λ) = 2 Σ (-1)^{k-1} exp(-2k²λ²)
/// - λ < 1.18: Q(λ) = 1 - (√(2π)/λ) Σ exp(-(2k-1)²π²/(8λ²))
fn ks_pvalue(d: f64, n_eff: f64) -> f64 {
    if d <= 0.0 {
        return 1.0;
    }
    let sqrt_n = n_eff.sqrt();
    let lambda = (sqrt_n + 0.12 + 0.11 / sqrt_n) * d;
    kolmogorov_sf(lambda)
}

/// Survival function of the Kolmogorov distribution, Q_KS(λ) = P(K > λ).
fn kolmogorov_sf(lambda: f64) -> f64 {
    if lambda.is_nan() {
        return f64::NAN;
    }
    if lambda <= 0.0 {
        return 1.0;
    }
    if lambda < 1.18 {
        // π²/8 and √(2π)
        const PI2_OVER_8: f64 = 1.233_700_550_136_169_8;
        const SQRT_2PI: f64 = 2.506_628_274_631_000_2;
        let y = (-PI2_OVER_8 / (lambda * lambda)).exp();
        let cdf = SQRT_2PI / lambda * (y + y.powi(9) + y.powi(25) + y.powi(49));
        (1.0 - cdf).clamp(0.0, 1.0)
    } else {
        let x = (-2.0 * lambda * lambda).exp();
        (2.0 * (x - x.powi(4) + x.powi(9))).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Confidence intervals
// ---------------------------------------------------------------------------

/// A confidence interval with lower bound, upper bound, and confidence level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// Lower bound of the interval.
    pub lower: f64,
    /// Upper bound of the interval.
    pub upper: f64,
    /// Point estimate (e.g. sample mean).
    pub estimate: f64,
    /// Confidence level (e.g. 0.95 for a 95% CI).
    pub confidence_level: f64,
}

/// Computes a confidence interval for the population mean (one-sample t-interval).
///
/// Uses the Student-t distribution with `n - 1` degrees of freedom.
///
/// # Errors
///
/// Returns `InvalidSample` if `data` has fewer than 2 elements, contains
/// non-finite values, or has zero variance.
/// Returns `InvalidParameter` if `confidence_level` is not in `(0, 1)`.
#[must_use = "returns the confidence interval"]
pub fn ci_mean(data: &[f64], confidence_level: f64) -> Result<ConfidenceInterval, PramanaError> {
    validate_confidence(confidence_level)?;
    if data.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations".into(),
        ));
    }
    let n = data.len() as f64;
    let (m, sample_var) = mean_and_sample_variance(data, "data")?;
    if sample_var == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero variance in sample".into(),
        ));
    }
    let se = (sample_var / n).sqrt();
    let df = n - 1.0;
    let alpha = 1.0 - confidence_level;
    let t_crit = t_quantile(1.0 - alpha / 2.0, df);
    let margin = t_crit * se;

    Ok(ConfidenceInterval {
        lower: m - margin,
        upper: m + margin,
        estimate: m,
        confidence_level,
    })
}

/// Computes a confidence interval for the difference of two population means
/// (Welch's t-interval, unequal variances).
///
/// # Errors
///
/// Returns `InvalidSample` if either sample has fewer than 2 elements or
/// contains non-finite values, or if both samples have zero variance.
/// Returns `InvalidParameter` if `confidence_level` is not in `(0, 1)`.
#[must_use = "returns the confidence interval"]
pub fn ci_two_means(
    a: &[f64],
    b: &[f64],
    confidence_level: f64,
) -> Result<ConfidenceInterval, PramanaError> {
    validate_confidence(confidence_level)?;
    let welch = welch_stats(a, b)?;
    let alpha = 1.0 - confidence_level;
    let t_crit = t_quantile(1.0 - alpha / 2.0, welch.df);
    let margin = t_crit * welch.se;

    Ok(ConfidenceInterval {
        lower: welch.diff - margin,
        upper: welch.diff + margin,
        estimate: welch.diff,
        confidence_level,
    })
}

/// Computes a confidence interval for a population proportion using the
/// Wald (normal approximation) method.
///
/// `successes` is the number of successes and `n` is the total number of trials.
///
/// # Errors
///
/// Returns `InvalidParameter` if `confidence_level` is not in `(0, 1)`.
/// Returns `InvalidSample` if `n` is 0 or `successes > n`.
#[must_use = "returns the confidence interval"]
pub fn ci_proportion(
    successes: u64,
    n: u64,
    confidence_level: f64,
) -> Result<ConfidenceInterval, PramanaError> {
    validate_confidence(confidence_level)?;
    if n == 0 {
        return Err(PramanaError::InvalidSample("n must be positive".into()));
    }
    if successes > n {
        return Err(PramanaError::InvalidSample(
            "successes must not exceed n".into(),
        ));
    }

    let p_hat = successes as f64 / n as f64;
    let alpha = 1.0 - confidence_level;
    let z = z_quantile(1.0 - alpha / 2.0);
    let se = (p_hat * (1.0 - p_hat) / n as f64).sqrt();
    let margin = z * se;

    Ok(ConfidenceInterval {
        lower: (p_hat - margin).max(0.0),
        upper: (p_hat + margin).min(1.0),
        estimate: p_hat,
        confidence_level,
    })
}

// ---------------------------------------------------------------------------
// Quantile functions (inverse CDF)
// ---------------------------------------------------------------------------

/// Inverse CDF of the Student-t distribution at probability `p` with `df` degrees of freedom.
///
/// Uses bisection on the CDF computed via the regularized incomplete beta function.
fn t_quantile(p: f64, df: f64) -> f64 {
    if p.is_nan() || df.is_nan() {
        return f64::NAN;
    }
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    if (p - 0.5).abs() < 1e-15 {
        return 0.0;
    }

    let t_cdf = |x: f64| student_t_cdf(x, df);

    // Bisection search
    let mut lo = -100.0_f64;
    let mut hi = 100.0_f64;

    // Expand bounds if needed (bounded: the CDF reaches 0/1 at ±inf).
    while t_cdf(lo) > p && lo.is_finite() {
        lo *= 2.0;
    }
    while t_cdf(hi) < p && hi.is_finite() {
        hi *= 2.0;
    }

    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if (hi - lo) < 1e-12 * mid.abs().max(1.0) {
            return mid;
        }
        if t_cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Inverse CDF of the standard normal distribution at probability `p`.
///
/// Peter Acklam's rational approximation (relative error 1.15e-9) followed by
/// one Halley refinement step against the full-precision normal CDF.
fn z_quantile(p: f64) -> f64 {
    if p.is_nan() {
        return f64::NAN;
    }
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }

    // Rational approximation coefficients (Acklam)
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239e0,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838e0,
        -2.549_732_539_343_734e0,
        4.374_664_141_464_968e0,
        2.938_163_982_698_783e0,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996e0,
        3.754_408_661_907_416e0,
    ];

    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    let x = if p < P_LOW {
        // Lower tail
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= P_HIGH {
        // Central region
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        // Upper tail
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };

    // One Halley step: e = Φ(x) - p, u = e / φ(x).
    let density = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    if density > 0.0 && density.is_finite() {
        let u = (normal_cdf(x) - p) / density;
        x - u / (1.0 + 0.5 * x * u)
    } else {
        x
    }
}

/// Read-only hooks for the Cyrius port's golden-value generator (see `crate::oracle`).
#[cfg(feature = "oracle")]
pub(crate) mod oracle;

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

    // --- ANOVA ---

    #[test]
    fn anova_identical_groups() {
        // Same data in each group — should not reject
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [1.0, 2.0, 3.0, 4.0, 5.0];
        let c = [1.0, 2.0, 3.0, 4.0, 5.0];
        let result = one_way_anova(&[&a, &b, &c], 0.05).unwrap();
        assert!(!result.reject, "identical groups should not reject");
        assert!(result.f_statistic.abs() < 1e-10);
        assert!(result.ss_between.abs() < 1e-10);
    }

    #[test]
    fn anova_different_groups() {
        // Clearly different means
        let a = [1.0, 1.1, 0.9, 1.0, 1.2];
        let b = [10.0, 10.1, 9.9, 10.2, 9.8];
        let c = [100.0, 100.1, 99.9, 100.0, 100.2];
        let result = one_way_anova(&[&a, &b, &c], 0.05).unwrap();
        assert!(result.reject, "different groups should reject");
        assert!(result.f_statistic > 100.0);
        assert!(result.p_value < 0.001);
    }

    #[test]
    fn anova_two_groups_matches_f() {
        // With 2 groups, ANOVA F should equal the square of the two-sample t-statistic
        let a = [2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [4.0, 5.0, 6.0, 7.0, 8.0];
        let result = one_way_anova(&[&a, &b], 0.05).unwrap();
        assert_eq!(result.df_between, 1);
        assert_eq!(result.df_within, 8);
        assert!(result.f_statistic > 0.0);
    }

    #[test]
    fn anova_ss_decomposition() {
        // SST = SSB + SSW
        let a = [1.0, 3.0, 5.0];
        let b = [2.0, 4.0, 6.0];
        let c = [7.0, 8.0, 9.0];
        let result = one_way_anova(&[&a, &b, &c], 0.05).unwrap();
        assert!(
            (result.ss_total - result.ss_between - result.ss_within).abs() < 1e-10,
            "SST={} != SSB={} + SSW={}",
            result.ss_total,
            result.ss_between,
            result.ss_within
        );
    }

    #[test]
    fn anova_invalid_params() {
        let a = [1.0, 2.0];
        // Only 1 group
        assert!(one_way_anova(&[&a], 0.05).is_err());
        // Empty group
        let empty: &[f64] = &[];
        assert!(one_way_anova(&[&a, empty], 0.05).is_err());
        // Invalid alpha
        assert!(one_way_anova(&[&a, &a], 0.0).is_err());
    }

    #[test]
    fn anova_serde_roundtrip() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let result = one_way_anova(&[&a, &b], 0.05).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let r2: AnovaResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.f_statistic, r2.f_statistic);
        assert_eq!(result.df_between, r2.df_between);
        assert_eq!(result.df_within, r2.df_within);
    }

    // --- Kolmogorov-Smirnov ---

    #[test]
    fn ks_two_sample_same_distribution() {
        // Same data should not reject
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let b = [1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 10.5];
        let result = ks_two_sample(&a, &b, 0.05).unwrap();
        assert!(
            !result.reject,
            "similar distributions should not reject: D={}, p={}",
            result.statistic, result.p_value
        );
    }

    #[test]
    fn ks_two_sample_different() {
        // Clearly different distributions
        let a = [1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9];
        let b = [
            100.0, 100.1, 100.2, 100.3, 100.4, 100.5, 100.6, 100.7, 100.8, 100.9,
        ];
        let result = ks_two_sample(&a, &b, 0.05).unwrap();
        assert!(result.reject, "different distributions should reject");
        assert!((result.statistic - 1.0).abs() < 1e-10, "D should be 1.0");
    }

    #[test]
    fn ks_one_sample_uniform() {
        // Data from U(0,1) tested against U(0,1) CDF — should not reject
        let data: Vec<f64> = (1..=20).map(|i| i as f64 / 21.0).collect();
        let result = ks_one_sample(&data, |x| x.clamp(0.0, 1.0), 0.05).unwrap();
        assert!(
            !result.reject,
            "uniform data vs uniform CDF: D={}, p={}",
            result.statistic, result.p_value
        );
    }

    #[test]
    fn ks_one_sample_wrong_distribution() {
        // Data clustered near 0, tested against uniform — should reject
        let data = [0.01, 0.02, 0.03, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09, 0.1];
        let result = ks_one_sample(&data, |x| x.clamp(0.0, 1.0), 0.05).unwrap();
        assert!(
            result.reject,
            "mismatched distribution should reject: D={}, p={}",
            result.statistic, result.p_value
        );
    }

    #[test]
    fn ks_statistic_range() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let result = ks_two_sample(&a, &b, 0.05).unwrap();
        assert!(
            (0.0..=1.0).contains(&result.statistic),
            "D should be in [0,1]: {}",
            result.statistic
        );
        assert!(
            (0.0..=1.0).contains(&result.p_value),
            "p should be in [0,1]: {}",
            result.p_value
        );
    }

    #[test]
    fn ks_invalid_params() {
        let a = [1.0, 2.0];
        let empty: &[f64] = &[];
        assert!(ks_two_sample(&a, empty, 0.05).is_err());
        assert!(ks_two_sample(empty, &a, 0.05).is_err());
        assert!(ks_one_sample(empty, |x| x, 0.05).is_err());
        assert!(ks_two_sample(&a, &a, 0.0).is_err());
    }

    #[test]
    fn ks_serde_roundtrip() {
        let result = KsResult {
            statistic: 0.3,
            p_value: 0.1,
            reject_at_alpha: 0.05,
            reject: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        let r2: KsResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.statistic, r2.statistic);
        assert_eq!(result.p_value, r2.p_value);
        assert_eq!(result.reject, r2.reject);
    }

    // --- Confidence intervals ---

    #[test]
    fn ci_mean_contains_true_mean() {
        // Data from N(5, 1) — true mean = 5
        let data = [4.5, 5.2, 4.8, 5.1, 5.3, 4.9, 5.0, 5.2, 4.7, 5.1];
        let ci = ci_mean(&data, 0.95).unwrap();
        assert!(
            ci.lower < 5.0 && ci.upper > 5.0,
            "95% CI should contain 5.0: [{}, {}]",
            ci.lower,
            ci.upper
        );
        assert!((ci.confidence_level - 0.95).abs() < 1e-10);
        assert!(ci.lower < ci.estimate);
        assert!(ci.estimate < ci.upper);
    }

    #[test]
    fn ci_mean_wider_at_higher_confidence() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let ci90 = ci_mean(&data, 0.90).unwrap();
        let ci99 = ci_mean(&data, 0.99).unwrap();
        let width90 = ci90.upper - ci90.lower;
        let width99 = ci99.upper - ci99.lower;
        assert!(width99 > width90, "99% CI should be wider than 90% CI");
    }

    #[test]
    fn ci_two_means_overlapping() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [1.5, 2.5, 3.5, 4.5, 5.5];
        let ci = ci_two_means(&a, &b, 0.95).unwrap();
        // True difference is -0.5; CI should contain it
        assert!(
            ci.lower < -0.5 && ci.upper > -0.5,
            "CI should contain -0.5: [{}, {}]",
            ci.lower,
            ci.upper
        );
    }

    #[test]
    fn ci_two_means_disjoint() {
        let a = [100.0, 101.0, 99.0, 100.5, 100.2];
        let b = [1.0, 2.0, 1.5, 1.8, 2.2];
        let ci = ci_two_means(&a, &b, 0.95).unwrap();
        // Clearly different: CI should not contain 0
        assert!(ci.lower > 0.0, "CI lower should be > 0: {}", ci.lower);
    }

    #[test]
    fn ci_proportion_fair_coin() {
        // 50 heads out of 100 flips — should contain 0.5
        let ci = ci_proportion(50, 100, 0.95).unwrap();
        assert!(
            ci.lower < 0.5 && ci.upper > 0.5,
            "CI should contain 0.5: [{}, {}]",
            ci.lower,
            ci.upper
        );
        assert!((ci.estimate - 0.5).abs() < 1e-10);
    }

    #[test]
    fn ci_proportion_bounds() {
        // Edge: 0 successes
        let ci = ci_proportion(0, 100, 0.95).unwrap();
        assert!(ci.lower >= 0.0);
        assert!((ci.estimate).abs() < 1e-10);
        // Edge: all successes
        let ci = ci_proportion(100, 100, 0.95).unwrap();
        assert!(ci.upper <= 1.0);
        assert!((ci.estimate - 1.0).abs() < 1e-10);
    }

    #[test]
    fn ci_invalid_params() {
        let data = [1.0, 2.0, 3.0];
        assert!(ci_mean(&data, 0.0).is_err());
        assert!(ci_mean(&data, 1.0).is_err());
        assert!(ci_mean(&[1.0], 0.95).is_err());
        assert!(ci_proportion(5, 0, 0.95).is_err());
        assert!(ci_proportion(10, 5, 0.95).is_err());
    }

    #[test]
    fn ci_serde_roundtrip() {
        let ci = ConfidenceInterval {
            lower: 1.5,
            upper: 3.5,
            estimate: 2.5,
            confidence_level: 0.95,
        };
        let json = serde_json::to_string(&ci).unwrap();
        let ci2: ConfidenceInterval = serde_json::from_str(&json).unwrap();
        assert_eq!(ci.lower, ci2.lower);
        assert_eq!(ci.upper, ci2.upper);
        assert_eq!(ci.confidence_level, ci2.confidence_level);
    }

    #[test]
    fn z_quantile_known_values() {
        // z_0.975 ≈ 1.96 for a two-tailed 95% CI
        let z = z_quantile(0.975);
        assert!((z - 1.96).abs() < 0.01, "z_0.975 = {z}");
        // z_0.5 = 0
        assert!(z_quantile(0.5).abs() < 1e-6);
        // Symmetry
        assert!((z_quantile(0.025) + z_quantile(0.975)).abs() < 0.01);
    }

    #[test]
    fn t_quantile_known_values() {
        // For large df, t approaches z
        let t = t_quantile(0.975, 1000.0);
        assert!((t - 1.96).abs() < 0.02, "t_0.975(1000) = {t}");
        // t_0.5 = 0 for any df
        assert!(t_quantile(0.5, 5.0).abs() < 1e-6);
        // t_0.975(1) ≈ 12.706 (Cauchy)
        let t1 = t_quantile(0.975, 1.0);
        assert!((t1 - 12.706).abs() < 0.1, "t_0.975(1) = {t1}");
    }

    // --- Hardening regression tests -------------------------------------

    #[test]
    fn nan_alpha_and_confidence_are_rejected() {
        let data = [1.0, 2.0, 3.0, 4.0];
        assert!(t_test_one_sample(&data, 0.0, f64::NAN).is_err());
        assert!(t_test_two_sample(&data, &data, f64::NAN).is_err());
        assert!(chi_squared_test(&data, &data, f64::NAN).is_err());
        assert!(one_way_anova(&[&data, &data], f64::NAN).is_err());
        assert!(ks_two_sample(&data, &data, f64::NAN).is_err());
        assert!(ci_mean(&data, f64::NAN).is_err());
        assert!(ci_two_means(&data, &data, f64::NAN).is_err());
        assert!(ci_proportion(3, 10, f64::NAN).is_err());
    }

    #[test]
    fn non_finite_data_is_rejected() {
        let bad = [1.0, f64::NAN, 3.0];
        let good = [1.0, 2.0, 4.0];
        assert!(t_test_one_sample(&bad, 0.0, 0.05).is_err());
        assert!(t_test_one_sample(&good, f64::INFINITY, 0.05).is_err());
        assert!(t_test_two_sample(&good, &[1.0, f64::INFINITY], 0.05).is_err());
        assert!(one_way_anova(&[&good, &bad], 0.05).is_err());
        assert!(ks_two_sample(&good, &bad, 0.05).is_err());
        assert!(ks_one_sample(&bad, |x| x, 0.05).is_err());
        assert!(ci_mean(&bad, 0.95).is_err());
        assert!(chi_squared_test(&[1.0, -2.0], &[1.0, 1.0], 0.05).is_err());
        assert!(chi_squared_test(&[1.0, 2.0], &[1.0, f64::NAN], 0.05).is_err());
        // A CDF that returns NaN is reported instead of silently ignored.
        assert!(ks_one_sample(&good, |_| f64::NAN, 0.05).is_err());
    }

    #[test]
    fn chi_squared_pvalue_is_exact() {
        // df = 1: P(χ² > s) = erfc(sqrt(s/2)).
        let d = (3.841_458_820_694_124_f64 * 50.0).sqrt();
        let r = chi_squared_test(&[100.0 + d, 100.0 - d], &[100.0, 100.0], 0.05).unwrap();
        let exact = crate::math::erfc((r.statistic / 2.0).sqrt());
        assert!(((r.p_value - exact) / exact).abs() < 1e-13);
        assert!((r.p_value - 0.05).abs() < 1e-12);
        // df = 2: P(χ² > s) = exp(-s/2).
        let r = chi_squared_test(&[30.0, 10.0, 20.0], &[20.0, 20.0, 20.0], 0.05).unwrap();
        let exact = (-r.statistic / 2.0).exp();
        assert!(((r.p_value - exact) / exact).abs() < 1e-13);
    }

    #[test]
    fn ks_two_sample_identical_samples_have_zero_statistic() {
        // Used to report D = 1/3 because ties were not consumed together.
        let r = ks_two_sample(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0], 0.05).unwrap();
        assert_eq!(r.statistic, 0.0);
        assert_eq!(r.p_value, 1.0);
        assert!(!r.reject);
    }

    #[test]
    fn ks_two_sample_ties_across_samples() {
        // ECDFs after each distinct value: x=1: 0.5 vs 0.25; x=2: 1 vs 1.
        let r = ks_two_sample(&[1.0, 1.0, 2.0, 2.0], &[1.0, 2.0, 2.0, 2.0], 0.05).unwrap();
        assert!((r.statistic - 0.25).abs() < 1e-15);
        // Disjoint samples: D = 1.
        let r = ks_two_sample(&[1.0, 2.0], &[3.0, 4.0, 5.0], 0.05).unwrap();
        assert_eq!(r.statistic, 1.0);
    }

    #[test]
    fn kolmogorov_sf_reference_values() {
        // Reference: 2 Σ_{k<200} (-1)^{k-1} exp(-2k²λ²) in double precision.
        let cases = [
            (0.5, 0.963_945_243_664_875_1),
            (0.8, 0.544_142_411_574_198_1),
            (1.0, 0.269_999_671_677_354_56),
            (1.18, 0.123_453_809_429_765_7),
            (1.36, 0.049_485_876_755_377_876),
            (2.0, 0.000_670_925_255_779_695_3),
        ];
        for (lambda, expected) in cases {
            let got = kolmogorov_sf(lambda);
            assert!(
                ((got - expected) / expected).abs() < 1e-13,
                "Q_KS({lambda}) = {got}, expected {expected}"
            );
        }
        assert_eq!(kolmogorov_sf(0.0), 1.0);
        assert!((kolmogorov_sf(0.05) - 1.0).abs() < 1e-15);
        // Continuity across the series switch.
        assert!((kolmogorov_sf(1.18 - 1e-12) - kolmogorov_sf(1.18 + 1e-12)).abs() < 1e-11);
    }

    #[test]
    fn anova_zero_within_variance_rejects() {
        // Each group is constant but the means differ: F = inf, p = 0.
        // (Previously the p-value was NaN, so `reject` was silently false.)
        let r = one_way_anova(&[&[1.0, 1.0], &[2.0, 2.0], &[3.0, 3.0]], 0.05).unwrap();
        assert!(r.f_statistic.is_infinite());
        assert_eq!(r.p_value, 0.0);
        assert!(r.reject);
        // All observations identical: undefined, reported as an error.
        assert!(one_way_anova(&[&[4.0, 4.0], &[4.0, 4.0]], 0.05).is_err());
    }

    #[test]
    fn anova_tiny_p_value_is_not_rounded_to_zero() {
        let r = one_way_anova(&[&[0.0, 1.0, 2.0], &[100.0, 101.0, 102.0]], 0.05).unwrap();
        assert!(r.p_value > 0.0 && r.p_value < 1e-6, "p = {}", r.p_value);
        // F(1, 4) upper tail has the closed form via the t-distribution:
        // P(F > f) = P(|T_4| > sqrt(f)).
        let expected = student_t_two_tailed(r.f_statistic.sqrt(), 4.0);
        assert!(((r.p_value - expected) / expected).abs() < 1e-10);
    }

    #[test]
    fn z_quantile_full_precision() {
        assert!((z_quantile(0.975) - 1.959_963_984_540_054).abs() < 1e-14);
        assert!(
            ((z_quantile(1e-10) + 6.361_340_902_404_056) / 6.361_340_902_404_056).abs() < 1e-13
        );
        assert_eq!(z_quantile(0.5), 0.0);
        for &p in &[1e-300, 1e-20, 0.001, 0.3, 0.9, 0.999_999] {
            let z = z_quantile(p);
            let back = normal_cdf(z);
            assert!(((back - p) / p).abs() < 1e-12, "p={p} z={z} back={back}");
        }
        assert!(z_quantile(f64::NAN).is_nan());
    }

    #[test]
    fn t_quantile_precision() {
        // df = 1 (Cauchy): q(p) = tan(π (p - ½))
        let q = t_quantile(0.975, 1.0);
        assert!(((q - 12.706_204_736_174_696) / 12.706_204_736_174_696).abs() < 1e-10);
        // Round trip through the CDF.
        for &(p, df) in &[(0.9, 3.0), (0.995, 10.0), (0.01, 50.0), (0.975, 1e6)] {
            let q = t_quantile(p, df);
            assert!((student_t_cdf(q, df) - p).abs() < 1e-12, "p={p} df={df}");
        }
        assert!(t_quantile(f64::NAN, 3.0).is_nan());
    }
}
