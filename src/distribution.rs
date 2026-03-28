//! Probability distributions.
//!
//! Provides continuous and discrete distributions with PDF/PMF, CDF, mean,
//! variance, and sampling capabilities.

use crate::error::PramanaError;
use crate::math;
use crate::math::erf;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, SQRT_2};

/// A probability distribution that can compute density, cumulative probability,
/// moments, and draw samples.
pub trait Distribution {
    /// Probability density (or mass) function at `x`.
    fn pdf(&self, x: f64) -> f64;

    /// Cumulative distribution function at `x`: P(X <= x).
    fn cdf(&self, x: f64) -> f64;

    /// Expected value (mean) of the distribution.
    fn mean(&self) -> f64;

    /// Variance of the distribution.
    fn variance(&self) -> f64;

    /// Draw a single sample from this distribution.
    fn sample(&self, rng: &mut impl Rng) -> f64;
}

// ---------------------------------------------------------------------------
// Normal distribution
// ---------------------------------------------------------------------------

/// Normal (Gaussian) distribution with parameters `mean` and `std_dev`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Normal {
    /// Mean (mu).
    pub mean: f64,
    /// Standard deviation (sigma). Must be positive.
    pub std_dev: f64,
}

impl Normal {
    /// Creates a new Normal distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `std_dev <= 0`.
    pub fn new(mean: f64, std_dev: f64) -> Result<Self, PramanaError> {
        if std_dev <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "std_dev must be positive".into(),
            ));
        }
        Ok(Self { mean, std_dev })
    }
}

impl Distribution for Normal {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        let z = (x - self.mean) / self.std_dev;
        (1.0 / (self.std_dev * (2.0 * PI).sqrt())) * (-0.5 * z * z).exp()
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        let z = (x - self.mean) / (self.std_dev * SQRT_2);
        0.5 * (1.0 + erf(z))
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.mean
    }

    #[inline]
    fn variance(&self) -> f64 {
        self.std_dev * self.std_dev
    }

    /// Samples using the Box-Muller transform.
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        // Box-Muller transform
        let u1 = rng.next_f64().max(f64::MIN_POSITIVE); // avoid log(0)
        let u2 = rng.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
        self.mean + self.std_dev * z
    }
}

// ---------------------------------------------------------------------------
// Uniform distribution
// ---------------------------------------------------------------------------

/// Continuous uniform distribution on `[min, max]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Uniform {
    /// Lower bound.
    pub min: f64,
    /// Upper bound.
    pub max: f64,
}

impl Uniform {
    /// Creates a new Uniform distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `min >= max`.
    pub fn new(min: f64, max: f64) -> Result<Self, PramanaError> {
        if min >= max {
            return Err(PramanaError::InvalidParameter(
                "min must be less than max".into(),
            ));
        }
        Ok(Self { min, max })
    }
}

impl Distribution for Uniform {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x >= self.min && x <= self.max {
            1.0 / (self.max - self.min)
        } else {
            0.0
        }
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x < self.min {
            0.0
        } else if x > self.max {
            1.0
        } else {
            (x - self.min) / (self.max - self.min)
        }
    }

    #[inline]
    fn mean(&self) -> f64 {
        (self.min + self.max) / 2.0
    }

    #[inline]
    fn variance(&self) -> f64 {
        let range = self.max - self.min;
        range * range / 12.0
    }

    #[inline]
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        self.min + rng.next_f64() * (self.max - self.min)
    }
}

// ---------------------------------------------------------------------------
// Exponential distribution
// ---------------------------------------------------------------------------

/// Exponential distribution with rate parameter `lambda`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Exponential {
    /// Rate parameter (lambda). Must be positive.
    pub lambda: f64,
}

impl Exponential {
    /// Creates a new Exponential distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `lambda <= 0`.
    pub fn new(lambda: f64) -> Result<Self, PramanaError> {
        if lambda <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "lambda must be positive".into(),
            ));
        }
        Ok(Self { lambda })
    }
}

impl Distribution for Exponential {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            0.0
        } else {
            self.lambda * (-self.lambda * x).exp()
        }
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            0.0
        } else {
            1.0 - (-self.lambda * x).exp()
        }
    }

    #[inline]
    fn mean(&self) -> f64 {
        1.0 / self.lambda
    }

    #[inline]
    fn variance(&self) -> f64 {
        1.0 / (self.lambda * self.lambda)
    }

    /// Samples using the inverse CDF (inverse transform sampling).
    #[inline]
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        let u = rng.next_f64().max(f64::MIN_POSITIVE);
        -(1.0 - u).ln() / self.lambda
    }
}

// ---------------------------------------------------------------------------
// Poisson distribution
// ---------------------------------------------------------------------------

/// Poisson distribution with expected value `lambda`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Poisson {
    /// Expected number of events (lambda). Must be positive.
    pub lambda: f64,
}

impl Poisson {
    /// Creates a new Poisson distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `lambda <= 0`.
    pub fn new(lambda: f64) -> Result<Self, PramanaError> {
        if lambda <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "lambda must be positive".into(),
            ));
        }
        Ok(Self { lambda })
    }
}

impl Distribution for Poisson {
    /// Probability mass function: P(X = k) = (lambda^k * e^-lambda) / k!
    ///
    /// Returns 0 for non-integer `x` (exact float comparison; e.g. `3.0` is
    /// accepted but `3.0000000000000004` is not).
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x < 0.0 || x.fract() != 0.0 {
            return 0.0;
        }
        let k = x as u64;
        // Use log-space to avoid overflow: exp(k*ln(lambda) - lambda - ln(k!))
        let log_pmf = (k as f64) * self.lambda.ln() - self.lambda - ln_factorial(k);
        log_pmf.exp()
    }

    /// CDF: P(X <= x) = sum_{k=0}^{floor(x)} PMF(k).
    fn cdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            return 0.0;
        }
        let n = x.floor() as u64;
        let mut sum = 0.0;
        for k in 0..=n {
            let log_pmf = (k as f64) * self.lambda.ln() - self.lambda - ln_factorial(k);
            sum += log_pmf.exp();
        }
        sum.min(1.0)
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.lambda
    }

    #[inline]
    fn variance(&self) -> f64 {
        self.lambda
    }

    /// Samples using Knuth's algorithm for small lambda, or a normal
    /// approximation for lambda > 30 (where Knuth's exp(-lambda) underflows).
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        if self.lambda > 30.0 {
            // Normal approximation: Poisson(lambda) ~ N(lambda, lambda)
            let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
            let u2 = rng.next_f64();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
            let sample = self.lambda + self.lambda.sqrt() * z;
            return sample.round().max(0.0);
        }
        // Knuth's algorithm for small lambda
        let l = (-self.lambda).exp();
        let mut k: u64 = 0;
        let mut p = 1.0;
        loop {
            k += 1;
            p *= rng.next_f64();
            if p <= l {
                break;
            }
        }
        (k - 1) as f64
    }
}

/// Natural log of n! computed iteratively.
#[inline]
fn ln_factorial(n: u64) -> f64 {
    if n <= 1 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 2..=n {
        sum += (i as f64).ln();
    }
    sum
}

// ---------------------------------------------------------------------------
// Binomial distribution
// ---------------------------------------------------------------------------

/// Binomial distribution: number of successes in `n` independent Bernoulli trials
/// each with success probability `p`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Binomial {
    /// Number of trials.
    pub n: u64,
    /// Probability of success on each trial.
    pub p: f64,
}

impl Binomial {
    /// Creates a new Binomial distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `p` is not in `[0, 1]` or `n` is 0.
    pub fn new(n: u64, p: f64) -> Result<Self, PramanaError> {
        if n == 0 {
            return Err(PramanaError::InvalidParameter("n must be positive".into()));
        }
        if !(0.0..=1.0).contains(&p) {
            return Err(PramanaError::InvalidParameter("p must be in [0, 1]".into()));
        }
        Ok(Self { n, p })
    }
}

impl Distribution for Binomial {
    /// PMF: C(n, k) * p^k * (1-p)^(n-k), computed in log-space.
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x < 0.0 || x.fract() != 0.0 || x > self.n as f64 {
            return 0.0;
        }
        let k = x as u64;
        let log_pmf = ln_binomial_coeff(self.n, k)
            + (k as f64) * self.p.ln()
            + ((self.n - k) as f64) * (1.0 - self.p).ln();
        log_pmf.exp()
    }

    /// CDF: P(X <= x) = sum_{k=0}^{floor(x)} PMF(k).
    fn cdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            return 0.0;
        }
        if x >= self.n as f64 {
            return 1.0;
        }
        let upper = x.floor() as u64;
        let mut sum = 0.0;
        for k in 0..=upper {
            let log_pmf = ln_binomial_coeff(self.n, k)
                + (k as f64) * self.p.ln()
                + ((self.n - k) as f64) * (1.0 - self.p).ln();
            sum += log_pmf.exp();
        }
        sum.min(1.0)
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.n as f64 * self.p
    }

    #[inline]
    fn variance(&self) -> f64 {
        self.n as f64 * self.p * (1.0 - self.p)
    }

    /// Samples by simulating `n` Bernoulli trials.
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        let mut successes: u64 = 0;
        for _ in 0..self.n {
            if rng.next_f64() < self.p {
                successes += 1;
            }
        }
        successes as f64
    }
}

/// Natural log of C(n, k) = n! / (k! * (n-k)!), computed in log-space.
#[inline]
fn ln_binomial_coeff(n: u64, k: u64) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    ln_factorial(n) - ln_factorial(k) - ln_factorial(n - k)
}

// ---------------------------------------------------------------------------
// Bernoulli distribution
// ---------------------------------------------------------------------------

/// Bernoulli distribution: single trial with success probability `p`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Bernoulli {
    /// Probability of success.
    pub p: f64,
}

impl Bernoulli {
    /// Creates a new Bernoulli distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `p` is not in `[0, 1]`.
    pub fn new(p: f64) -> Result<Self, PramanaError> {
        if !(0.0..=1.0).contains(&p) {
            return Err(PramanaError::InvalidParameter("p must be in [0, 1]".into()));
        }
        Ok(Self { p })
    }
}

impl Distribution for Bernoulli {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if (x - 0.0).abs() < f64::EPSILON {
            1.0 - self.p
        } else if (x - 1.0).abs() < f64::EPSILON {
            self.p
        } else {
            0.0
        }
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            0.0
        } else if x < 1.0 {
            1.0 - self.p
        } else {
            1.0
        }
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.p
    }

    #[inline]
    fn variance(&self) -> f64 {
        self.p * (1.0 - self.p)
    }

    #[inline]
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        if rng.next_f64() < self.p { 1.0 } else { 0.0 }
    }
}

// ---------------------------------------------------------------------------
// Gamma distribution
// ---------------------------------------------------------------------------

/// Gamma distribution with shape parameter `alpha` and scale parameter `beta`.
///
/// The PDF is: f(x) = x^(alpha-1) * exp(-x/beta) / (beta^alpha * Gamma(alpha))
/// for x > 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Gamma {
    /// Shape parameter (alpha/k). Must be positive.
    pub alpha: f64,
    /// Scale parameter (beta/theta). Must be positive.
    pub beta: f64,
}

impl Gamma {
    /// Creates a new Gamma distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `alpha <= 0` or `beta <= 0`.
    pub fn new(alpha: f64, beta: f64) -> Result<Self, PramanaError> {
        if alpha <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "alpha must be positive".into(),
            ));
        }
        if beta <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "beta must be positive".into(),
            ));
        }
        Ok(Self { alpha, beta })
    }
}

impl Distribution for Gamma {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let ln_pdf = (self.alpha - 1.0) * x.ln()
            - x / self.beta
            - self.alpha * self.beta.ln()
            - math::ln_gamma(self.alpha);
        ln_pdf.exp()
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        math::regularized_lower_incomplete_gamma(self.alpha, x / self.beta)
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.alpha * self.beta
    }

    #[inline]
    fn variance(&self) -> f64 {
        self.alpha * self.beta * self.beta
    }

    /// Samples using the Marsaglia-Tsang method (alpha >= 1) with
    /// Ahrens-Dieter transformation for alpha < 1.
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        sample_gamma_standard(self.alpha, rng) * self.beta
    }
}

/// Samples from Gamma(alpha, 1) — standard Gamma with unit scale.
fn sample_gamma_standard(alpha: f64, rng: &mut impl Rng) -> f64 {
    // Ahrens-Dieter boost for alpha < 1 (iterative, no recursion):
    // Gamma(alpha, 1) = Gamma(alpha + 1, 1) * U^(1/alpha)
    let (effective_alpha, boost) = if alpha < 1.0 {
        let u = rng.next_f64().max(f64::MIN_POSITIVE);
        (alpha + 1.0, u.powf(1.0 / alpha))
    } else {
        (alpha, 1.0)
    };
    // Marsaglia and Tsang's method for effective_alpha >= 1
    let d = effective_alpha - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    let sample = loop {
        // Box-Muller for a standard normal variate
        let (x, v) = loop {
            let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
            let u2 = rng.next_f64();
            let x = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
            let v = 1.0 + c * x;
            if v > 0.0 {
                break (x, v * v * v);
            }
        };
        let u = rng.next_f64().max(f64::MIN_POSITIVE);
        if u < 1.0 - 0.0331 * x * x * x * x {
            break d * v;
        }
        if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            break d * v;
        }
    };
    sample * boost
}

// ---------------------------------------------------------------------------
// Beta distribution
// ---------------------------------------------------------------------------

/// Beta distribution on [0, 1] with shape parameters `alpha` and `beta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Beta {
    /// Shape parameter alpha. Must be positive.
    pub alpha: f64,
    /// Shape parameter beta. Must be positive.
    pub beta: f64,
}

impl Beta {
    /// Creates a new Beta distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `alpha <= 0` or `beta <= 0`.
    pub fn new(alpha: f64, beta: f64) -> Result<Self, PramanaError> {
        if alpha <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "alpha must be positive".into(),
            ));
        }
        if beta <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "beta must be positive".into(),
            ));
        }
        Ok(Self { alpha, beta })
    }
}

impl Distribution for Beta {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 || x >= 1.0 {
            return 0.0;
        }
        let ln_pdf = (self.alpha - 1.0) * x.ln() + (self.beta - 1.0) * (1.0 - x).ln()
            - math::ln_beta(self.alpha, self.beta);
        ln_pdf.exp()
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        math::regularized_incomplete_beta(x, self.alpha, self.beta)
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    #[inline]
    fn variance(&self) -> f64 {
        let ab = self.alpha + self.beta;
        self.alpha * self.beta / (ab * ab * (ab + 1.0))
    }

    /// Samples via the Gamma ratio: X/(X+Y) where X ~ Gamma(alpha,1), Y ~ Gamma(beta,1).
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        let x = sample_gamma_standard(self.alpha, rng);
        let y = sample_gamma_standard(self.beta, rng);
        x / (x + y)
    }
}

// ---------------------------------------------------------------------------
// Chi-squared distribution
// ---------------------------------------------------------------------------

/// Chi-squared distribution with `df` degrees of freedom.
///
/// This is a special case of the Gamma distribution: Chi²(df) = Gamma(df/2, 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChiSquared {
    /// Degrees of freedom. Must be positive.
    pub df: f64,
}

impl ChiSquared {
    /// Creates a new Chi-squared distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `df <= 0`.
    pub fn new(df: f64) -> Result<Self, PramanaError> {
        if df <= 0.0 {
            return Err(PramanaError::InvalidParameter("df must be positive".into()));
        }
        Ok(Self { df })
    }
}

impl Distribution for ChiSquared {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let half_df = self.df / 2.0;
        let ln_pdf =
            (half_df - 1.0) * x.ln() - x / 2.0 - half_df * 2.0_f64.ln() - math::ln_gamma(half_df);
        ln_pdf.exp()
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        math::regularized_lower_incomplete_gamma(self.df / 2.0, x / 2.0)
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.df
    }

    #[inline]
    fn variance(&self) -> f64 {
        2.0 * self.df
    }

    /// Samples via Gamma(df/2, 2).
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        sample_gamma_standard(self.df / 2.0, rng) * 2.0
    }
}

// ---------------------------------------------------------------------------
// Student's t-distribution
// ---------------------------------------------------------------------------

/// Student's t-distribution with `df` degrees of freedom.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StudentT {
    /// Degrees of freedom. Must be positive.
    pub df: f64,
}

impl StudentT {
    /// Creates a new Student's t-distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `df <= 0`.
    pub fn new(df: f64) -> Result<Self, PramanaError> {
        if df <= 0.0 {
            return Err(PramanaError::InvalidParameter("df must be positive".into()));
        }
        Ok(Self { df })
    }
}

impl Distribution for StudentT {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        let half_dfp1 = (self.df + 1.0) / 2.0;
        let ln_pdf = math::ln_gamma(half_dfp1)
            - 0.5 * (self.df * PI).ln()
            - math::ln_gamma(self.df / 2.0)
            - half_dfp1 * (1.0 + x * x / self.df).ln();
        ln_pdf.exp()
    }

    /// CDF via the regularized incomplete beta function.
    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        let ibeta =
            math::regularized_incomplete_beta(self.df / (self.df + x * x), self.df / 2.0, 0.5);
        if x >= 0.0 {
            1.0 - 0.5 * ibeta
        } else {
            0.5 * ibeta
        }
    }

    /// Returns 0 for df > 1, NaN otherwise (undefined).
    #[inline]
    fn mean(&self) -> f64 {
        if self.df > 1.0 { 0.0 } else { f64::NAN }
    }

    /// Returns df/(df-2) for df > 2, infinity for 1 < df <= 2, NaN otherwise.
    #[inline]
    fn variance(&self) -> f64 {
        if self.df > 2.0 {
            self.df / (self.df - 2.0)
        } else if self.df > 1.0 {
            f64::INFINITY
        } else {
            f64::NAN
        }
    }

    /// Samples as Z / sqrt(V/df) where Z ~ N(0,1), V ~ Chi²(df).
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        // Standard normal via Box-Muller
        let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
        let u2 = rng.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
        // Chi-squared(df) via Gamma
        let v = sample_gamma_standard(self.df / 2.0, rng) * 2.0;
        z / (v / self.df).sqrt()
    }
}

// ---------------------------------------------------------------------------
// F-distribution
// ---------------------------------------------------------------------------

/// F-distribution (Fisher-Snedecor) with `d1` and `d2` degrees of freedom.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FDistribution {
    /// Numerator degrees of freedom. Must be positive.
    pub d1: f64,
    /// Denominator degrees of freedom. Must be positive.
    pub d2: f64,
}

impl FDistribution {
    /// Creates a new F-distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `d1 <= 0` or `d2 <= 0`.
    pub fn new(d1: f64, d2: f64) -> Result<Self, PramanaError> {
        if d1 <= 0.0 {
            return Err(PramanaError::InvalidParameter("d1 must be positive".into()));
        }
        if d2 <= 0.0 {
            return Err(PramanaError::InvalidParameter("d2 must be positive".into()));
        }
        Ok(Self { d1, d2 })
    }
}

impl Distribution for FDistribution {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let half_d1 = self.d1 / 2.0;
        let half_d2 = self.d2 / 2.0;
        let ln_pdf = half_d1 * (self.d1 / self.d2).ln() + (half_d1 - 1.0) * x.ln()
            - (half_d1 + half_d2) * (1.0 + self.d1 * x / self.d2).ln()
            - math::ln_beta(half_d1, half_d2);
        ln_pdf.exp()
    }

    /// CDF via the regularized incomplete beta function.
    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let u = self.d1 * x / (self.d1 * x + self.d2);
        math::regularized_incomplete_beta(u, self.d1 / 2.0, self.d2 / 2.0)
    }

    /// Returns d2/(d2-2) for d2 > 2, NaN otherwise (undefined).
    #[inline]
    fn mean(&self) -> f64 {
        if self.d2 > 2.0 {
            self.d2 / (self.d2 - 2.0)
        } else {
            f64::NAN
        }
    }

    /// Returns the variance for d2 > 4, NaN otherwise (undefined).
    #[inline]
    fn variance(&self) -> f64 {
        if self.d2 > 4.0 {
            2.0 * self.d2 * self.d2 * (self.d1 + self.d2 - 2.0)
                / (self.d1 * (self.d2 - 2.0) * (self.d2 - 2.0) * (self.d2 - 4.0))
        } else {
            f64::NAN
        }
    }

    /// Samples as (X1/d1) / (X2/d2) where X1 ~ Chi²(d1), X2 ~ Chi²(d2).
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        let x1 = sample_gamma_standard(self.d1 / 2.0, rng) * 2.0;
        let x2 = sample_gamma_standard(self.d2 / 2.0, rng) * 2.0;
        (x1 / self.d1) / (x2 / self.d2)
    }
}

// ---------------------------------------------------------------------------
// Cauchy distribution
// ---------------------------------------------------------------------------

/// Cauchy distribution with location `x0` and scale `gamma`.
///
/// The Cauchy distribution has no defined mean or variance (returns NaN).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Cauchy {
    /// Location parameter (median).
    pub x0: f64,
    /// Scale parameter. Must be positive.
    pub gamma: f64,
}

impl Cauchy {
    /// Creates a new Cauchy distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `gamma <= 0`.
    pub fn new(x0: f64, gamma: f64) -> Result<Self, PramanaError> {
        if gamma <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "gamma must be positive".into(),
            ));
        }
        Ok(Self { x0, gamma })
    }
}

impl Distribution for Cauchy {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        let z = (x - self.x0) / self.gamma;
        1.0 / (PI * self.gamma * (1.0 + z * z))
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        (1.0 / PI) * ((x - self.x0) / self.gamma).atan() + 0.5
    }

    /// Undefined for Cauchy; returns NaN.
    #[inline]
    fn mean(&self) -> f64 {
        f64::NAN
    }

    /// Undefined for Cauchy; returns NaN.
    #[inline]
    fn variance(&self) -> f64 {
        f64::NAN
    }

    /// Samples via inverse CDF: x0 + gamma * tan(pi * (U - 0.5)).
    #[inline]
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        let u = rng.next_f64();
        self.x0 + self.gamma * (PI * (u - 0.5)).tan()
    }
}

// ---------------------------------------------------------------------------
// Weibull distribution
// ---------------------------------------------------------------------------

/// Weibull distribution with shape `k` and scale `lambda`.
///
/// The PDF is: f(x) = (k/lambda) * (x/lambda)^(k-1) * exp(-(x/lambda)^k)
/// for x >= 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Weibull {
    /// Shape parameter. Must be positive.
    pub k: f64,
    /// Scale parameter. Must be positive.
    pub lambda: f64,
}

impl Weibull {
    /// Creates a new Weibull distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `k <= 0` or `lambda <= 0`.
    pub fn new(k: f64, lambda: f64) -> Result<Self, PramanaError> {
        if k <= 0.0 {
            return Err(PramanaError::InvalidParameter("k must be positive".into()));
        }
        if lambda <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "lambda must be positive".into(),
            ));
        }
        Ok(Self { k, lambda })
    }
}

impl Distribution for Weibull {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            return 0.0;
        }
        if x == 0.0 {
            return if self.k == 1.0 {
                1.0 / self.lambda
            } else if self.k > 1.0 {
                0.0
            } else {
                f64::INFINITY
            };
        }
        let z = x / self.lambda;
        (self.k / self.lambda) * z.powf(self.k - 1.0) * (-z.powf(self.k)).exp()
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        1.0 - (-(x / self.lambda).powf(self.k)).exp()
    }

    /// Mean = lambda * Gamma(1 + 1/k).
    #[inline]
    fn mean(&self) -> f64 {
        self.lambda * math::ln_gamma(1.0 + 1.0 / self.k).exp()
    }

    /// Variance = lambda^2 * [Gamma(1 + 2/k) - Gamma(1 + 1/k)^2].
    #[inline]
    fn variance(&self) -> f64 {
        let g1 = math::ln_gamma(1.0 + 1.0 / self.k).exp();
        let g2 = math::ln_gamma(1.0 + 2.0 / self.k).exp();
        self.lambda * self.lambda * (g2 - g1 * g1)
    }

    /// Samples via inverse CDF: lambda * (-ln(1 - U))^(1/k).
    #[inline]
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        let u = rng.next_f64().max(f64::MIN_POSITIVE);
        self.lambda * (-(1.0 - u).ln()).powf(1.0 / self.k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SimpleRng;

    #[test]
    fn normal_pdf_at_mean() {
        let n = Normal::new(0.0, 1.0).unwrap();
        let expected = 1.0 / (2.0 * PI).sqrt();
        assert!((n.pdf(0.0) - expected).abs() < 1e-10);
    }

    #[test]
    fn normal_cdf_at_mean() {
        let n = Normal::new(0.0, 1.0).unwrap();
        assert!((n.cdf(0.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn normal_invalid_std_dev() {
        assert!(Normal::new(0.0, 0.0).is_err());
        assert!(Normal::new(0.0, -1.0).is_err());
    }

    #[test]
    fn uniform_pdf_and_cdf() {
        let u = Uniform::new(0.0, 10.0).unwrap();
        assert!((u.pdf(5.0) - 0.1).abs() < 1e-10);
        assert!((u.cdf(5.0) - 0.5).abs() < 1e-10);
        assert_eq!(u.pdf(-1.0), 0.0);
        assert_eq!(u.cdf(-1.0), 0.0);
        assert_eq!(u.cdf(11.0), 1.0);
    }

    #[test]
    fn exponential_mean_and_variance() {
        let e = Exponential::new(2.0).unwrap();
        assert!((e.mean() - 0.5).abs() < 1e-10);
        assert!((e.variance() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn poisson_pmf() {
        // P(X=3) for lambda=2: (2^3 * e^-2) / 3! = 8 * e^-2 / 6
        let p = Poisson::new(2.0).unwrap();
        let expected = 8.0 * std::f64::consts::E.powf(-2.0) / 6.0;
        assert!((p.pdf(3.0) - expected).abs() < 1e-10);
    }

    #[test]
    fn binomial_mean_and_variance() {
        let b = Binomial::new(10, 0.3).unwrap();
        assert!((b.mean() - 3.0).abs() < 1e-10);
        assert!((b.variance() - 2.1).abs() < 1e-10);
    }

    #[test]
    fn bernoulli_pdf() {
        let b = Bernoulli::new(0.7).unwrap();
        assert!((b.pdf(1.0) - 0.7).abs() < 1e-10);
        assert!((b.pdf(0.0) - 0.3).abs() < 1e-10);
    }

    #[test]
    fn normal_sample_finite() {
        let n = Normal::new(0.0, 1.0).unwrap();
        let mut rng = SimpleRng::new(42);
        for _ in 0..1000 {
            let s = n.sample(&mut rng);
            assert!(s.is_finite());
        }
    }

    #[test]
    fn serde_roundtrip_normal() {
        let n = Normal::new(1.5, 2.3).unwrap();
        let json = serde_json::to_string(&n).unwrap();
        let n2: Normal = serde_json::from_str(&json).unwrap();
        assert_eq!(n.mean, n2.mean);
        assert_eq!(n.std_dev, n2.std_dev);
    }

    #[test]
    fn serde_roundtrip_uniform() {
        let u = Uniform::new(-1.0, 5.0).unwrap();
        let json = serde_json::to_string(&u).unwrap();
        let u2: Uniform = serde_json::from_str(&json).unwrap();
        assert_eq!(u.min, u2.min);
        assert_eq!(u.max, u2.max);
    }

    #[test]
    fn serde_roundtrip_exponential() {
        let e = Exponential::new(2.5).unwrap();
        let json = serde_json::to_string(&e).unwrap();
        let e2: Exponential = serde_json::from_str(&json).unwrap();
        assert_eq!(e.lambda, e2.lambda);
    }

    #[test]
    fn serde_roundtrip_poisson() {
        let p = Poisson::new(3.5).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let p2: Poisson = serde_json::from_str(&json).unwrap();
        assert_eq!(p.lambda, p2.lambda);
    }

    #[test]
    fn serde_roundtrip_binomial() {
        let b = Binomial::new(20, 0.4).unwrap();
        let json = serde_json::to_string(&b).unwrap();
        let b2: Binomial = serde_json::from_str(&json).unwrap();
        assert_eq!(b.n, b2.n);
        assert_eq!(b.p, b2.p);
    }

    #[test]
    fn serde_roundtrip_bernoulli() {
        let b = Bernoulli::new(0.7).unwrap();
        let json = serde_json::to_string(&b).unwrap();
        let b2: Bernoulli = serde_json::from_str(&json).unwrap();
        assert_eq!(b.p, b2.p);
    }

    #[test]
    fn poisson_large_lambda_sample() {
        // Verify sampling doesn't hang for large lambda
        let p = Poisson::new(100.0).unwrap();
        let mut rng = SimpleRng::new(42);
        let mut sum = 0.0;
        let n = 10_000;
        for _ in 0..n {
            let s = p.sample(&mut rng);
            assert!(s >= 0.0);
            assert!(s.is_finite());
            sum += s;
        }
        let sample_mean = sum / n as f64;
        assert!(
            (sample_mean - 100.0).abs() < 5.0,
            "sample mean {sample_mean} too far from lambda=100"
        );
    }

    // --- Gamma ---

    #[test]
    fn gamma_pdf_known() {
        // Gamma(1, 1) = Exponential(1): pdf(1) = e^{-1}
        let g = Gamma::new(1.0, 1.0).unwrap();
        assert!((g.pdf(1.0) - (-1.0_f64).exp()).abs() < 1e-8);
    }

    #[test]
    fn gamma_cdf_known() {
        // Gamma(1, 1) CDF at x = 1: 1 - e^{-1}
        let g = Gamma::new(1.0, 1.0).unwrap();
        let expected = 1.0 - (-1.0_f64).exp();
        assert!((g.cdf(1.0) - expected).abs() < 1e-6, "got {}", g.cdf(1.0));
    }

    #[test]
    fn gamma_mean_and_variance() {
        let g = Gamma::new(3.0, 2.0).unwrap();
        assert!((g.mean() - 6.0).abs() < 1e-10);
        assert!((g.variance() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn gamma_sample_mean() {
        let g = Gamma::new(5.0, 2.0).unwrap();
        let mut rng = SimpleRng::new(42);
        let n = 50_000;
        let sum: f64 = (0..n).map(|_| g.sample(&mut rng)).sum();
        let sample_mean = sum / n as f64;
        assert!(
            (sample_mean - 10.0).abs() < 0.5,
            "sample mean {sample_mean} too far from 10"
        );
    }

    #[test]
    fn gamma_invalid_params() {
        assert!(Gamma::new(0.0, 1.0).is_err());
        assert!(Gamma::new(1.0, 0.0).is_err());
        assert!(Gamma::new(-1.0, 1.0).is_err());
    }

    #[test]
    fn gamma_small_alpha_sample() {
        // Verify no stack overflow for alpha < 1
        let g = Gamma::new(0.1, 1.0).unwrap();
        let mut rng = SimpleRng::new(42);
        let n = 10_000;
        let mut sum = 0.0;
        for _ in 0..n {
            let s = g.sample(&mut rng);
            assert!(s >= 0.0);
            assert!(s.is_finite());
            sum += s;
        }
        let sample_mean = sum / n as f64;
        assert!(
            (sample_mean - 0.1).abs() < 0.05,
            "sample mean {sample_mean} too far from 0.1"
        );
    }

    #[test]
    fn serde_roundtrip_gamma() {
        let g = Gamma::new(3.0, 2.0).unwrap();
        let json = serde_json::to_string(&g).unwrap();
        let g2: Gamma = serde_json::from_str(&json).unwrap();
        assert_eq!(g.alpha, g2.alpha);
        assert_eq!(g.beta, g2.beta);
    }

    // --- Beta ---

    #[test]
    fn beta_mean_and_variance() {
        let b = Beta::new(2.0, 5.0).unwrap();
        assert!((b.mean() - 2.0 / 7.0).abs() < 1e-10);
        let expected_var = 2.0 * 5.0 / (49.0 * 8.0);
        assert!((b.variance() - expected_var).abs() < 1e-10);
    }

    #[test]
    fn beta_cdf_endpoints() {
        let b = Beta::new(2.0, 3.0).unwrap();
        assert_eq!(b.cdf(0.0), 0.0);
        assert_eq!(b.cdf(1.0), 1.0);
    }

    #[test]
    fn beta_uniform_is_beta_1_1() {
        // Beta(1,1) = Uniform(0,1), PDF should be 1 everywhere on (0,1)
        let b = Beta::new(1.0, 1.0).unwrap();
        assert!((b.pdf(0.5) - 1.0).abs() < 1e-8);
        assert!((b.cdf(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn beta_sample_in_range() {
        let b = Beta::new(2.0, 5.0).unwrap();
        let mut rng = SimpleRng::new(42);
        for _ in 0..10_000 {
            let s = b.sample(&mut rng);
            assert!((0.0..=1.0).contains(&s), "out of range: {s}");
        }
    }

    #[test]
    fn beta_small_alpha_sample() {
        // Verify no stack overflow for alpha < 1 (uses Gamma internally)
        let b = Beta::new(0.5, 0.5).unwrap();
        let mut rng = SimpleRng::new(42);
        for _ in 0..10_000 {
            let s = b.sample(&mut rng);
            assert!((0.0..=1.0).contains(&s), "out of range: {s}");
        }
    }

    #[test]
    fn serde_roundtrip_beta() {
        let b = Beta::new(2.0, 5.0).unwrap();
        let json = serde_json::to_string(&b).unwrap();
        let b2: Beta = serde_json::from_str(&json).unwrap();
        assert_eq!(b.alpha, b2.alpha);
        assert_eq!(b.beta, b2.beta);
    }

    // --- Chi-squared ---

    #[test]
    fn chi_squared_mean_and_variance() {
        let c = ChiSquared::new(5.0).unwrap();
        assert!((c.mean() - 5.0).abs() < 1e-10);
        assert!((c.variance() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn chi_squared_cdf_known() {
        // Chi²(2) CDF at x = 2: 1 - e^{-1} ≈ 0.6321
        let c = ChiSquared::new(2.0).unwrap();
        let expected = 1.0 - (-1.0_f64).exp();
        assert!((c.cdf(2.0) - expected).abs() < 1e-4, "got {}", c.cdf(2.0));
    }

    #[test]
    fn chi_squared_sample_mean() {
        let c = ChiSquared::new(10.0).unwrap();
        let mut rng = SimpleRng::new(42);
        let n = 50_000;
        let sum: f64 = (0..n).map(|_| c.sample(&mut rng)).sum();
        let sample_mean = sum / n as f64;
        assert!(
            (sample_mean - 10.0).abs() < 0.5,
            "sample mean {sample_mean}"
        );
    }

    #[test]
    fn serde_roundtrip_chi_squared() {
        let c = ChiSquared::new(5.0).unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let c2: ChiSquared = serde_json::from_str(&json).unwrap();
        assert_eq!(c.df, c2.df);
    }

    // --- Student-t ---

    #[test]
    fn student_t_symmetric_pdf() {
        let t = StudentT::new(5.0).unwrap();
        assert!((t.pdf(1.0) - t.pdf(-1.0)).abs() < 1e-10);
        assert!((t.cdf(0.0) - 0.5).abs() < 1e-6, "CDF at 0 = {}", t.cdf(0.0));
    }

    #[test]
    fn student_t_mean_and_variance() {
        let t = StudentT::new(5.0).unwrap();
        assert!((t.mean()).abs() < 1e-10);
        assert!((t.variance() - 5.0 / 3.0).abs() < 1e-10);
        // df = 1: undefined mean
        let t1 = StudentT::new(1.0).unwrap();
        assert!(t1.mean().is_nan());
        assert!(t1.variance().is_nan());
        // df = 1.5: infinite variance
        let t15 = StudentT::new(1.5).unwrap();
        assert!((t15.mean()).abs() < 1e-10);
        assert!(t15.variance().is_infinite());
    }

    #[test]
    fn student_t_sample_symmetric() {
        let t = StudentT::new(10.0).unwrap();
        let mut rng = SimpleRng::new(42);
        let n = 50_000;
        let sum: f64 = (0..n).map(|_| t.sample(&mut rng)).sum();
        let sample_mean = sum / n as f64;
        assert!(
            sample_mean.abs() < 0.1,
            "sample mean {sample_mean} should be near 0"
        );
    }

    #[test]
    fn serde_roundtrip_student_t() {
        let t = StudentT::new(5.0).unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let t2: StudentT = serde_json::from_str(&json).unwrap();
        assert_eq!(t.df, t2.df);
    }

    // --- F-distribution ---

    #[test]
    fn f_distribution_mean() {
        let f = FDistribution::new(5.0, 10.0).unwrap();
        assert!((f.mean() - 10.0 / 8.0).abs() < 1e-10);
        // d2 <= 2: mean undefined
        let f2 = FDistribution::new(5.0, 2.0).unwrap();
        assert!(f2.mean().is_nan());
    }

    #[test]
    fn f_distribution_variance() {
        // F(5, 10): var = 2*100*(5+10-2) / (5*64*6) = 2600/1920 ≈ 1.3542
        let f = FDistribution::new(5.0, 10.0).unwrap();
        let expected = 2.0 * 100.0 * 13.0 / (5.0 * 64.0 * 6.0);
        assert!(
            (f.variance() - expected).abs() < 1e-10,
            "variance = {}",
            f.variance()
        );
        // d2 <= 4: variance undefined
        let f2 = FDistribution::new(5.0, 4.0).unwrap();
        assert!(f2.variance().is_nan());
    }

    #[test]
    fn f_distribution_cdf_nonneg() {
        let f = FDistribution::new(5.0, 10.0).unwrap();
        assert_eq!(f.cdf(0.0), 0.0);
        // CDF should be close to 1 for very large x
        assert!(f.cdf(100.0) > 0.99);
    }

    #[test]
    fn f_distribution_sample_positive() {
        let f = FDistribution::new(5.0, 10.0).unwrap();
        let mut rng = SimpleRng::new(42);
        for _ in 0..10_000 {
            let s = f.sample(&mut rng);
            assert!(s > 0.0, "F sample must be positive: {s}");
        }
    }

    #[test]
    fn serde_roundtrip_f_distribution() {
        let f = FDistribution::new(5.0, 10.0).unwrap();
        let json = serde_json::to_string(&f).unwrap();
        let f2: FDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(f.d1, f2.d1);
        assert_eq!(f.d2, f2.d2);
    }

    // --- Cauchy ---

    #[test]
    fn cauchy_pdf_and_cdf() {
        let c = Cauchy::new(0.0, 1.0).unwrap();
        // PDF at 0: 1/(pi)
        assert!((c.pdf(0.0) - 1.0 / PI).abs() < 1e-10);
        // CDF at 0: 0.5
        assert!((c.cdf(0.0) - 0.5).abs() < 1e-10);
        // CDF symmetry
        assert!((c.cdf(1.0) + c.cdf(-1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cauchy_undefined_moments() {
        let c = Cauchy::new(0.0, 1.0).unwrap();
        assert!(c.mean().is_nan());
        assert!(c.variance().is_nan());
    }

    #[test]
    fn cauchy_sample_finite() {
        let c = Cauchy::new(0.0, 1.0).unwrap();
        let mut rng = SimpleRng::new(42);
        for _ in 0..10_000 {
            let s = c.sample(&mut rng);
            assert!(s.is_finite());
        }
    }

    #[test]
    fn serde_roundtrip_cauchy() {
        let c = Cauchy::new(2.0, 3.0).unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let c2: Cauchy = serde_json::from_str(&json).unwrap();
        assert_eq!(c.x0, c2.x0);
        assert_eq!(c.gamma, c2.gamma);
    }

    // --- Weibull ---

    #[test]
    fn weibull_exponential_case() {
        // Weibull(k=1, lambda) = Exponential(1/lambda)
        let w = Weibull::new(1.0, 2.0).unwrap();
        assert!((w.mean() - 2.0).abs() < 1e-8);
        // CDF at x = 2: 1 - e^{-1}
        let expected = 1.0 - (-1.0_f64).exp();
        assert!((w.cdf(2.0) - expected).abs() < 1e-8);
    }

    #[test]
    fn weibull_mean_and_variance() {
        let w = Weibull::new(2.0, 1.0).unwrap();
        // Mean = Gamma(1 + 0.5) = Gamma(1.5) = sqrt(pi)/2 ≈ 0.8862
        let expected_mean = std::f64::consts::PI.sqrt() / 2.0;
        assert!(
            (w.mean() - expected_mean).abs() < 1e-6,
            "mean = {}",
            w.mean()
        );
    }

    #[test]
    fn weibull_sample_nonnegative() {
        let w = Weibull::new(2.0, 3.0).unwrap();
        let mut rng = SimpleRng::new(42);
        for _ in 0..10_000 {
            let s = w.sample(&mut rng);
            assert!(s >= 0.0, "Weibull sample must be non-negative: {s}");
        }
    }

    #[test]
    fn serde_roundtrip_weibull() {
        let w = Weibull::new(2.0, 3.0).unwrap();
        let json = serde_json::to_string(&w).unwrap();
        let w2: Weibull = serde_json::from_str(&json).unwrap();
        assert_eq!(w.k, w2.k);
        assert_eq!(w.lambda, w2.lambda);
    }
}
