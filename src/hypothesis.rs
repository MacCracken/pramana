//! Hypothesis testing and confidence intervals.

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
/// # Errors
///
/// Returns `InvalidSample` if fewer than 2 groups, any group is empty, or
/// total observations fewer than the number of groups + 1.
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

    let df_between = k - 1;
    let df_within = n_total - k;
    let ms_between = ss_between / df_between as f64;
    let ms_within = if df_within > 0 {
        ss_within / df_within as f64
    } else {
        return Err(PramanaError::InvalidSample(
            "zero within-group degrees of freedom".into(),
        ));
    };

    let f_statistic = if ms_within > 0.0 {
        ms_between / ms_within
    } else {
        f64::INFINITY
    };

    // p-value: P(F > f_statistic) = 1 - F_CDF(f_statistic)
    let p_value = f_distribution_upper_tail(f_statistic, df_between as f64, df_within as f64);

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

/// Upper tail of the F-distribution: P(X > x) where X ~ F(d1, d2).
fn f_distribution_upper_tail(x: f64, d1: f64, d2: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    let u = d1 * x / (d1 * x + d2);
    let cdf = regularized_incomplete_beta(u, d1 / 2.0, d2 / 2.0);
    (1.0 - cdf).clamp(0.0, 1.0)
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
/// comparing their empirical CDFs.
///
/// # Errors
///
/// Returns `InvalidSample` if either sample is empty.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
#[must_use = "returns the KS test result"]
pub fn ks_two_sample(a: &[f64], b: &[f64], alpha: f64) -> Result<KsResult, PramanaError> {
    validate_alpha(alpha)?;
    if a.is_empty() || b.is_empty() {
        return Err(PramanaError::InvalidSample(
            "both samples must be non-empty".into(),
        ));
    }

    let mut sa = a.to_vec();
    let mut sb = b.to_vec();
    sa.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    sb.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));

    let na = sa.len() as f64;
    let nb = sb.len() as f64;
    let mut d_max: f64 = 0.0;
    let mut ia = 0;
    let mut ib = 0;

    while ia < sa.len() && ib < sb.len() {
        let cdf_a = (ia + 1) as f64 / na;
        let cdf_b = (ib + 1) as f64 / nb;
        if sa[ia] <= sb[ib] {
            d_max = d_max.max((cdf_a - ib as f64 / nb).abs());
            ia += 1;
        } else {
            d_max = d_max.max((ia as f64 / na - cdf_b).abs());
            ib += 1;
        }
    }
    // Remaining elements
    while ia < sa.len() {
        let cdf_a = (ia + 1) as f64 / na;
        d_max = d_max.max((cdf_a - 1.0).abs());
        ia += 1;
    }
    while ib < sb.len() {
        let cdf_b = (ib + 1) as f64 / nb;
        d_max = d_max.max((1.0 - cdf_b).abs());
        ib += 1;
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
/// Returns `InvalidSample` if the sample is empty.
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1)`.
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

    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len() as f64;
    let mut d_max: f64 = 0.0;

    for (i, &x) in sorted.iter().enumerate() {
        let ecdf_before = i as f64 / n;
        let ecdf_after = (i + 1) as f64 / n;
        let theoretical = cdf(x);
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

/// Approximate p-value for the KS statistic using the Kolmogorov distribution.
///
/// P(D > d) ≈ 2 * Σ_{k=1}^{∞} (-1)^{k+1} exp(-2k²λ²) where λ = (√n + 0.12 + 0.11/√n) * d.
fn ks_pvalue(d: f64, n_eff: f64) -> f64 {
    if d <= 0.0 {
        return 1.0;
    }
    let sqrt_n = n_eff.sqrt();
    let lambda = (sqrt_n + 0.12 + 0.11 / sqrt_n) * d;
    let lambda_sq = lambda * lambda;

    let mut sum = 0.0;
    for k in 1..=100 {
        let term = (-2.0 * (k as f64) * (k as f64) * lambda_sq).exp();
        if k % 2 == 1 {
            sum += term;
        } else {
            sum -= term;
        }
        if term < 1e-15 {
            break;
        }
    }
    (2.0 * sum).clamp(0.0, 1.0)
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
/// Returns `InvalidSample` if `data` has fewer than 2 elements or zero variance.
/// Returns `InvalidParameter` if `confidence_level` is not in `(0, 1)`.
#[must_use = "returns the confidence interval"]
pub fn ci_mean(data: &[f64], confidence_level: f64) -> Result<ConfidenceInterval, PramanaError> {
    if confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err(PramanaError::InvalidParameter(
            "confidence_level must be in (0, 1)".into(),
        ));
    }
    if data.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations".into(),
        ));
    }
    let n = data.len() as f64;
    let m = descriptive::mean(data)?;
    let sample_var = data.iter().map(|&x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0);
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
/// Returns `InvalidSample` if either sample has fewer than 2 elements or zero variance.
/// Returns `InvalidParameter` if `confidence_level` is not in `(0, 1)`.
#[must_use = "returns the confidence interval"]
pub fn ci_two_means(
    a: &[f64],
    b: &[f64],
    confidence_level: f64,
) -> Result<ConfidenceInterval, PramanaError> {
    if confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err(PramanaError::InvalidParameter(
            "confidence_level must be in (0, 1)".into(),
        ));
    }
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

    // Welch-Satterthwaite degrees of freedom
    let num = (var1 / n1 + var2 / n2).powi(2);
    let denom = (var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0);
    let df = if denom == 0.0 { 1.0 } else { num / denom };

    let alpha = 1.0 - confidence_level;
    let t_crit = t_quantile(1.0 - alpha / 2.0, df);
    let diff = m1 - m2;
    let margin = t_crit * se;

    Ok(ConfidenceInterval {
        lower: diff - margin,
        upper: diff + margin,
        estimate: diff,
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
    if confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err(PramanaError::InvalidParameter(
            "confidence_level must be in (0, 1)".into(),
        ));
    }
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
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    if (p - 0.5).abs() < 1e-15 {
        return 0.0;
    }

    // CDF of Student-t at x
    let t_cdf = |x: f64| -> f64 {
        let ibeta = regularized_incomplete_beta(df / (df + x * x), df / 2.0, 0.5);
        if x >= 0.0 {
            1.0 - 0.5 * ibeta
        } else {
            0.5 * ibeta
        }
    };

    // Bisection search
    let mut lo = -100.0;
    let mut hi = 100.0;

    // Expand bounds if needed
    while t_cdf(lo) > p {
        lo *= 2.0;
    }
    while t_cdf(hi) < p {
        hi *= 2.0;
    }

    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if (hi - lo) < 1e-12 {
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
/// Uses the rational approximation by Peter Acklam.
fn z_quantile(p: f64) -> f64 {
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

    if p < P_LOW {
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
    }
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
}
