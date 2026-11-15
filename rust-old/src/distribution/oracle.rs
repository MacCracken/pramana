//! `distribution.rs`'s private samplers, CDF helpers and the `MultivariateNormal`
//! factorization, exposed unchanged for the oracle (see `crate::oracle`).

use super::MultivariateNormal;
use crate::rng::Rng;

/// One standard normal variate by Box–Muller (cosine branch), drawing two uniforms. This is
/// the variate every normal-based sampler in the crate uses.
pub fn standard_normal(rng: &mut impl Rng) -> f64 {
    super::standard_normal(rng)
}

/// Poisson sampling by Hörmann's PTRS, as `Poisson::sample` uses it for λ > 30.
///
/// Returns NaN, drawing nothing, unless λ is finite and at least 10 (PTRS's range). The
/// crate never passes such a λ, and the sampler would loop forever on NaN.
pub fn sample_poisson_ptrs(lambda: f64, rng: &mut impl Rng) -> f64 {
    if !(lambda.is_finite() && lambda >= 10.0) {
        return f64::NAN;
    }
    super::sample_poisson_ptrs(lambda, rng)
}

/// A Gamma(α, 1) variate by Marsaglia–Tsang, with the Ahrens–Dieter boost for α < 1. It
/// backs the Gamma, Beta, χ², Student-t and F samplers and the Binomial splitting.
///
/// Returns NaN, drawing nothing, unless α is finite and positive. The crate never passes
/// such an α, and the sampler would loop forever on NaN or α ≤ −1.
pub fn sample_gamma_standard(alpha: f64, rng: &mut impl Rng) -> f64 {
    if !(alpha.is_finite() && alpha > 0.0) {
        return f64::NAN;
    }
    super::sample_gamma_standard(alpha, rng)
}

/// The Student-t CDF at `t` with `df` degrees of freedom.
#[must_use]
pub fn student_t_cdf(t: f64, df: f64) -> f64 {
    super::student_t_cdf(t, df)
}

/// The two-tailed Student-t p-value `P(|T| ≥ |t|)`.
#[must_use]
pub fn student_t_two_tailed(t: f64, df: f64) -> f64 {
    super::student_t_two_tailed(t, df)
}

/// Both tails `(CDF, SF)` of the F distribution at `x`.
#[must_use]
pub fn f_cdf_sf(x: f64, d1: f64, d2: f64) -> (f64, f64) {
    super::f_cdf_sf(x, d1, d2)
}

/// `MultivariateNormal`'s private factorization: the lower Cholesky factor `L` (rows) of
/// the covariance, and `ln det Σ = 2·Σ ln Lᵢᵢ`, exactly as the constructor stored them.
#[must_use]
pub fn mvn_factor(mvn: &MultivariateNormal) -> (&[Vec<f64>], f64) {
    (&mvn.cholesky_l, mvn.log_det)
}
