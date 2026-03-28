//! Probability distributions.
//!
//! Provides continuous and discrete distributions with PDF/PMF, CDF, mean,
//! variance, and sampling capabilities.

use crate::error::PramanaError;
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
}
