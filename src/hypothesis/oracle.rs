//! `hypothesis.rs`'s private statistics, p-values and quantiles, exposed unchanged for
//! the oracle (see `crate::oracle`).

use crate::error::PramanaError;

/// Welch's statistics for two samples, as the Welch t-test and `ci_two_means` compute them:
/// `(mean difference, standard error, Welch–Satterthwaite df)`.
///
/// # Errors
///
/// The same errors as the crate's own computation: `InvalidSample` for fewer than two
/// observations, non-finite summaries, or zero variance in both samples.
pub fn welch_stats(a: &[f64], b: &[f64]) -> Result<(f64, f64, f64), PramanaError> {
    let w = super::welch_stats(a, b)?;
    Ok((w.diff, w.se, w.df))
}

/// The upper-tail χ² p-value `Q(df/2, χ²/2)`.
#[must_use]
pub fn chi_squared_pvalue(chi2: f64, df: f64) -> f64 {
    super::chi_squared_pvalue(chi2, df)
}

/// The two-sample KS p-value for statistic `d` and effective size `n_eff`.
#[must_use]
pub fn ks_pvalue(d: f64, n_eff: f64) -> f64 {
    super::ks_pvalue(d, n_eff)
}

/// The Kolmogorov survival function `Q_KS(λ)`, from its two series split at λ = 1.18.
#[must_use]
pub fn kolmogorov_sf(lambda: f64) -> f64 {
    super::kolmogorov_sf(lambda)
}

/// The Student-t quantile by bisection on the t CDF.
#[must_use]
pub fn t_quantile(p: f64, df: f64) -> f64 {
    super::t_quantile(p, df)
}

/// The standard normal quantile: Acklam's approximation plus one Halley step.
#[must_use]
pub fn z_quantile(p: f64) -> f64 {
    super::z_quantile(p)
}
