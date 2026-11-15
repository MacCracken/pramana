//! Probability distributions.
//!
//! Provides continuous and discrete distributions with PDF/PMF, CDF, mean,
//! variance, and sampling capabilities.
//!
//! Every constructor validates its parameters (finite, in range), and
//! deserialization goes through the same constructor, so a `Distribution`
//! value can never hold invalid parameters by way of serde.

use crate::error::PramanaError;
use crate::math;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

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
// Parameter validation helpers
// ---------------------------------------------------------------------------

/// `InvalidParameter` unless `value` is finite.
fn require_finite(value: f64, name: &str) -> Result<(), PramanaError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(PramanaError::InvalidParameter(format!(
            "{name} must be finite, got {value}"
        )))
    }
}

/// `InvalidParameter` unless `value` is finite and strictly positive.
fn require_positive(value: f64, name: &str) -> Result<(), PramanaError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(PramanaError::InvalidParameter(format!(
            "{name} must be positive and finite, got {value}"
        )))
    }
}

/// `InvalidParameter` unless `value` is a probability in `[0, 1]`.
fn require_probability(value: f64, name: &str) -> Result<(), PramanaError> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(PramanaError::InvalidParameter(format!(
            "{name} must be in [0, 1], got {value}"
        )))
    }
}

/// Generates a private serde "repr" struct plus a `TryFrom` impl that routes
/// deserialization through the validating constructor. Used together with
/// `#[serde(try_from = "...")]`; the wire format is unchanged.
macro_rules! validated_deserialize {
    ($ty:ident, $repr:ident { $($field:ident : $fty:ty),* $(,)? }, $ctor:expr) => {
        #[derive(Deserialize)]
        struct $repr {
            $($field: $fty),*
        }

        impl TryFrom<$repr> for $ty {
            type Error = PramanaError;

            fn try_from(repr: $repr) -> Result<Self, PramanaError> {
                let $repr { $($field),* } = repr;
                $ctor
            }
        }
    };
}

/// Draws a standard normal variate via Box-Muller (cosine branch).
#[inline]
fn standard_normal(rng: &mut impl Rng) -> f64 {
    let u1 = rng.next_f64().max(f64::MIN_POSITIVE); // avoid log(0)
    let u2 = rng.next_f64();
    (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
}

// ---------------------------------------------------------------------------
// Normal distribution
// ---------------------------------------------------------------------------

/// Normal (Gaussian) distribution with parameters `mean` and `std_dev`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "NormalRepr")]
#[non_exhaustive]
pub struct Normal {
    /// Mean (mu).
    pub mean: f64,
    /// Standard deviation (sigma). Must be positive.
    pub std_dev: f64,
}

validated_deserialize!(
    Normal,
    NormalRepr {
        mean: f64,
        std_dev: f64
    },
    Normal::new(mean, std_dev)
);

impl Normal {
    /// Creates a new Normal distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `mean` is not finite or `std_dev` is not
    /// positive and finite.
    pub fn new(mean: f64, std_dev: f64) -> Result<Self, PramanaError> {
        require_finite(mean, "mean")?;
        require_positive(std_dev, "std_dev")?;
        Ok(Self { mean, std_dev })
    }
}

impl Distribution for Normal {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        let z = (x - self.mean) / self.std_dev;
        (1.0 / (self.std_dev * (2.0 * PI).sqrt())) * (-0.5 * z * z).exp()
    }

    /// CDF via `erfc`, accurate to full relative precision in both tails.
    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        math::normal_cdf((x - self.mean) / self.std_dev)
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
        self.mean + self.std_dev * standard_normal(rng)
    }
}

// ---------------------------------------------------------------------------
// Uniform distribution
// ---------------------------------------------------------------------------

/// Continuous uniform distribution on `[min, max]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "UniformRepr")]
#[non_exhaustive]
pub struct Uniform {
    /// Lower bound.
    pub min: f64,
    /// Upper bound.
    pub max: f64,
}

validated_deserialize!(
    Uniform,
    UniformRepr { min: f64, max: f64 },
    Uniform::new(min, max)
);

impl Uniform {
    /// Creates a new Uniform distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if either bound is not finite, `min >= max`,
    /// or the width `max - min` overflows.
    pub fn new(min: f64, max: f64) -> Result<Self, PramanaError> {
        require_finite(min, "min")?;
        require_finite(max, "max")?;
        if min >= max {
            return Err(PramanaError::InvalidParameter(
                "min must be less than max".into(),
            ));
        }
        if !(max - min).is_finite() {
            return Err(PramanaError::InvalidParameter("max - min overflows".into()));
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
#[serde(try_from = "ExponentialRepr")]
#[non_exhaustive]
pub struct Exponential {
    /// Rate parameter (lambda). Must be positive.
    pub lambda: f64,
}

validated_deserialize!(
    Exponential,
    ExponentialRepr { lambda: f64 },
    Exponential::new(lambda)
);

impl Exponential {
    /// Creates a new Exponential distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `lambda` is not positive and finite.
    pub fn new(lambda: f64) -> Result<Self, PramanaError> {
        require_positive(lambda, "lambda")?;
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

    /// CDF `1 - exp(-λx)`, computed with `exp_m1` for precision near 0.
    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            0.0
        } else {
            -(-self.lambda * x).exp_m1()
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
#[serde(try_from = "PoissonRepr")]
#[non_exhaustive]
pub struct Poisson {
    /// Expected number of events (lambda). Must be positive.
    pub lambda: f64,
}

validated_deserialize!(Poisson, PoissonRepr { lambda: f64 }, Poisson::new(lambda));

/// Above this rate, Poisson sampling uses the PTRS transformed-rejection
/// sampler instead of Knuth's multiplication method.
const POISSON_PTRS_THRESHOLD: f64 = 30.0;

impl Poisson {
    /// Creates a new Poisson distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `lambda` is not positive and finite.
    pub fn new(lambda: f64) -> Result<Self, PramanaError> {
        require_positive(lambda, "lambda")?;
        Ok(Self { lambda })
    }
}

impl Distribution for Poisson {
    /// Probability mass function: P(X = k) = (lambda^k * e^-lambda) / k!
    ///
    /// Returns 0 for non-integer `x` (exact float comparison; e.g. `3.0` is
    /// accepted but `3.0000000000000004` is not). Evaluated in log-space without
    /// cancellation, in O(1) for any `k`.
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x.is_nan() {
            return f64::NAN;
        }
        if x < 0.0 || x.fract() != 0.0 || x.is_infinite() {
            return 0.0;
        }
        if x == 0.0 {
            return (-self.lambda).exp();
        }
        // ln P(X = k) = k ln λ - λ - ln k! = [(k+1) ln λ - λ - lnΓ(k+1)] - ln λ
        (math::ln_gamma_kernel(x + 1.0, self.lambda) - self.lambda.ln()).exp()
    }

    /// CDF: P(X <= x) = Q(floor(x) + 1, lambda), the regularized upper
    /// incomplete gamma function. O(sqrt(lambda)) at worst, never O(x).
    fn cdf(&self, x: f64) -> f64 {
        if x.is_nan() {
            return f64::NAN;
        }
        if x < 0.0 {
            return 0.0;
        }
        math::regularized_upper_incomplete_gamma(x.floor() + 1.0, self.lambda)
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.lambda
    }

    #[inline]
    fn variance(&self) -> f64 {
        self.lambda
    }

    /// Samples exactly: Knuth's multiplication method for `lambda <= 30`, and
    /// Hörmann's PTRS transformed rejection for larger `lambda`.
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        if self.lambda > POISSON_PTRS_THRESHOLD {
            return sample_poisson_ptrs(self.lambda, rng);
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

/// Poisson sampling by transformed rejection with squeeze (PTRS).
///
/// W. Hörmann, "The transformed rejection method for generating Poisson
/// random variables", Insurance: Mathematics and Economics 12 (1993).
/// Valid (and efficient) for `lambda >= 10`.
fn sample_poisson_ptrs(lambda: f64, rng: &mut impl Rng) -> f64 {
    let slam = lambda.sqrt();
    let loglam = lambda.ln();
    let b = 0.931 + 2.53 * slam;
    let a = -0.059 + 0.02483 * b;
    let inv_alpha = 1.1239 + 1.1328 / (b - 3.4);
    let vr = 0.9277 - 3.6224 / (b - 2.0);

    loop {
        let u = rng.next_f64() - 0.5;
        let v = rng.next_f64();
        let us = 0.5 - u.abs();
        let k = ((2.0 * a / us + b) * u + lambda + 0.43).floor();
        if us >= 0.07 && v <= vr {
            return k;
        }
        if k < 0.0 || !k.is_finite() || (us < 0.013 && v > us) {
            continue;
        }
        let lhs = v.ln() + inv_alpha.ln() - (a / (us * us) + b).ln();
        // ln P(X = k) = k ln λ - λ - ln k!, via the cancellation-free kernel
        // (the direct form loses ~1e-3 absolute accuracy at λ ~ 1e12).
        let rhs = if k == 0.0 {
            -lambda
        } else {
            math::ln_gamma_kernel(k + 1.0, lambda) - loglam
        };
        if lhs <= rhs {
            return k;
        }
    }
}

// ---------------------------------------------------------------------------
// Binomial distribution
// ---------------------------------------------------------------------------

/// Binomial distribution: number of successes in `n` independent Bernoulli trials
/// each with success probability `p`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "BinomialRepr")]
#[non_exhaustive]
pub struct Binomial {
    /// Number of trials.
    pub n: u64,
    /// Probability of success on each trial.
    pub p: f64,
}

validated_deserialize!(
    Binomial,
    BinomialRepr { n: u64, p: f64 },
    Binomial::new(n, p)
);

/// Up to this many trials, Binomial sampling simulates the Bernoulli trials
/// directly; above it, the problem is split recursively with Beta order
/// statistics (exact, O(log n)).
const BINOMIAL_DIRECT_MAX_N: u64 = 64;

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
        require_probability(p, "p")?;
        Ok(Self { n, p })
    }
}

impl Distribution for Binomial {
    /// PMF: C(n, k) * p^k * (1-p)^(n-k).
    ///
    /// Uses Loader's saddle-point form (Stirling remainders + deviance terms),
    /// which is O(1) and accurate even for very large `n`.
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x.is_nan() {
            return f64::NAN;
        }
        let n = self.n as f64;
        if x < 0.0 || x.fract() != 0.0 || x > n {
            return 0.0;
        }
        let p = self.p;
        let q = 1.0 - p;
        if p == 0.0 {
            return if x == 0.0 { 1.0 } else { 0.0 };
        }
        if p == 1.0 {
            return if x == n { 1.0 } else { 0.0 };
        }
        if x == 0.0 {
            return (n * (-p).ln_1p()).exp();
        }
        if x == n {
            return (n * p.ln()).exp();
        }
        let n_minus_x = n - x;
        let lc = math::stirling_error(n)
            - math::stirling_error(x)
            - math::stirling_error(n_minus_x)
            - math::bd0(x, n * p)
            - math::bd0(n_minus_x, n * q);
        // ln(2π x (n - x) / n)
        let lf = 2.0 * math::LN_SQRT_2PI + x.ln() + (-x / n).ln_1p();
        (lc - 0.5 * lf).exp()
    }

    /// CDF: P(X <= x) = I_{1-p}(n - k, k + 1) with k = floor(x), via the
    /// regularized incomplete beta function (no O(n) summation).
    fn cdf(&self, x: f64) -> f64 {
        if x.is_nan() {
            return f64::NAN;
        }
        if x < 0.0 {
            return 0.0;
        }
        let n = self.n as f64;
        if x >= n {
            return 1.0;
        }
        if self.p == 0.0 {
            return 1.0;
        }
        if self.p == 1.0 {
            return 0.0;
        }
        let k = x.floor();
        math::beta_inc(n - k, k + 1.0, 1.0 - self.p, self.p).0
    }

    #[inline]
    fn mean(&self) -> f64 {
        self.n as f64 * self.p
    }

    #[inline]
    fn variance(&self) -> f64 {
        self.n as f64 * self.p * (1.0 - self.p)
    }

    /// Samples exactly. For `n <= 64` simulates the Bernoulli trials; larger
    /// `n` is reduced in O(log n) steps by Knuth's Beta order-statistic
    /// splitting (TAOCP Vol. 2, §3.4.1).
    fn sample(&self, rng: &mut impl Rng) -> f64 {
        let mut trials = self.n;
        let mut p = self.p;
        let mut successes: u64 = 0;

        while trials > BINOMIAL_DIRECT_MAX_N {
            if p <= 0.0 {
                return successes as f64;
            }
            if p >= 1.0 {
                return (successes + trials) as f64;
            }
            // The a-th smallest of `trials` uniforms is Beta(a, trials - a + 1).
            let a = 1 + trials / 2;
            let b = trials - a + 1;
            let ga = sample_gamma_standard(a as f64, rng);
            let gb = sample_gamma_standard(b as f64, rng);
            let x = if ga + gb > 0.0 { ga / (ga + gb) } else { 0.5 };
            if x >= p {
                // Successes all lie among the a - 1 uniforms below x.
                trials = a - 1;
                p = (p / x).clamp(0.0, 1.0);
            } else {
                // The a uniforms up to x are all successes; recurse above x.
                successes += a;
                trials = b - 1;
                p = ((p - x) / (1.0 - x)).clamp(0.0, 1.0);
            }
        }

        for _ in 0..trials {
            if rng.next_f64() < p {
                successes += 1;
            }
        }
        successes as f64
    }
}

// ---------------------------------------------------------------------------
// Bernoulli distribution
// ---------------------------------------------------------------------------

/// Bernoulli distribution: single trial with success probability `p`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "BernoulliRepr")]
#[non_exhaustive]
pub struct Bernoulli {
    /// Probability of success.
    pub p: f64,
}

validated_deserialize!(Bernoulli, BernoulliRepr { p: f64 }, Bernoulli::new(p));

impl Bernoulli {
    /// Creates a new Bernoulli distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `p` is not in `[0, 1]`.
    pub fn new(p: f64) -> Result<Self, PramanaError> {
        require_probability(p, "p")?;
        Ok(Self { p })
    }
}

impl Distribution for Bernoulli {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x.abs() < f64::EPSILON {
            1.0 - self.p
        } else if (x - 1.0).abs() < f64::EPSILON {
            self.p
        } else {
            0.0
        }
    }

    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x.is_nan() {
            f64::NAN
        } else if x < 0.0 {
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
#[serde(try_from = "GammaRepr")]
#[non_exhaustive]
pub struct Gamma {
    /// Shape parameter (alpha/k). Must be positive.
    pub alpha: f64,
    /// Scale parameter (beta/theta). Must be positive.
    pub beta: f64,
}

validated_deserialize!(
    Gamma,
    GammaRepr {
        alpha: f64,
        beta: f64
    },
    Gamma::new(alpha, beta)
);

impl Gamma {
    /// Creates a new Gamma distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `alpha` or `beta` is not positive and finite.
    pub fn new(alpha: f64, beta: f64) -> Result<Self, PramanaError> {
        require_positive(alpha, "alpha")?;
        require_positive(beta, "beta")?;
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
            let x = standard_normal(rng);
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
#[serde(try_from = "BetaRepr")]
#[non_exhaustive]
pub struct Beta {
    /// Shape parameter alpha. Must be positive.
    pub alpha: f64,
    /// Shape parameter beta. Must be positive.
    pub beta: f64,
}

validated_deserialize!(
    Beta,
    BetaRepr {
        alpha: f64,
        beta: f64
    },
    Beta::new(alpha, beta)
);

impl Beta {
    /// Creates a new Beta distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `alpha` or `beta` is not positive and finite.
    pub fn new(alpha: f64, beta: f64) -> Result<Self, PramanaError> {
        require_positive(alpha, "alpha")?;
        require_positive(beta, "beta")?;
        Ok(Self { alpha, beta })
    }
}

impl Distribution for Beta {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 || x >= 1.0 {
            return 0.0;
        }
        let ln_pdf = (self.alpha - 1.0) * x.ln() + (self.beta - 1.0) * (-x).ln_1p()
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
#[serde(try_from = "ChiSquaredRepr")]
#[non_exhaustive]
pub struct ChiSquared {
    /// Degrees of freedom. Must be positive.
    pub df: f64,
}

validated_deserialize!(ChiSquared, ChiSquaredRepr { df: f64 }, ChiSquared::new(df));

impl ChiSquared {
    /// Creates a new Chi-squared distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `df` is not positive and finite.
    pub fn new(df: f64) -> Result<Self, PramanaError> {
        require_positive(df, "df")?;
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
#[serde(try_from = "StudentTRepr")]
#[non_exhaustive]
pub struct StudentT {
    /// Degrees of freedom. Must be positive.
    pub df: f64,
}

validated_deserialize!(StudentT, StudentTRepr { df: f64 }, StudentT::new(df));

impl StudentT {
    /// Creates a new Student's t-distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `df` is not positive and finite.
    pub fn new(df: f64) -> Result<Self, PramanaError> {
        require_positive(df, "df")?;
        Ok(Self { df })
    }
}

/// Two-tailed p-value P(|T| >= |t|) for Student-t with `df` degrees of freedom.
///
/// Equals I_{df/(df+t²)}(df/2, 1/2); both incomplete-beta arguments are formed
/// without cancellation (and without inf/inf for huge |t|).
pub(crate) fn student_t_two_tailed(t: f64, df: f64) -> f64 {
    if t.is_nan() || df.is_nan() {
        return f64::NAN;
    }
    let t2 = t * t;
    let x = 1.0 / (1.0 + t2 / df);
    let y = 1.0 / (1.0 + df / t2);
    math::beta_inc(df / 2.0, 0.5, x, y).0.clamp(0.0, 1.0)
}

/// Student-t CDF with `df` degrees of freedom, accurate in both tails.
pub(crate) fn student_t_cdf(t: f64, df: f64) -> f64 {
    let two_tail = student_t_two_tailed(t, df);
    if t >= 0.0 {
        1.0 - 0.5 * two_tail
    } else {
        0.5 * two_tail
    }
}

impl Distribution for StudentT {
    #[inline]
    fn pdf(&self, x: f64) -> f64 {
        let half_dfp1 = (self.df + 1.0) / 2.0;
        // ln Γ((ν+1)/2) - ln Γ(ν/2) - ½ ln(νπ). For large ν the two lnΓ terms
        // cancel badly, so use the equivalent -ln B(ν/2, ½) - ½ ln ν, whose
        // Stirling form is exact there; for small ν the direct form is cheaper.
        // The kernel term is multiplied by (ν+1)/2, so for large ν it needs
        // `ln_1p`; for small ν the cheaper `ln` is just as accurate.
        let ln_pdf = if self.df >= 20.0 {
            -math::ln_beta(self.df / 2.0, 0.5)
                - 0.5 * self.df.ln()
                - half_dfp1 * (x * x / self.df).ln_1p()
        } else {
            math::ln_gamma(half_dfp1)
                - math::ln_gamma(self.df / 2.0)
                - 0.5 * (self.df * PI).ln()
                - half_dfp1 * (1.0 + x * x / self.df).ln()
        };
        ln_pdf.exp()
    }

    /// CDF via the regularized incomplete beta function.
    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        student_t_cdf(x, self.df)
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
        let z = standard_normal(rng);
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
#[serde(try_from = "FDistributionRepr")]
#[non_exhaustive]
pub struct FDistribution {
    /// Numerator degrees of freedom. Must be positive.
    pub d1: f64,
    /// Denominator degrees of freedom. Must be positive.
    pub d2: f64,
}

validated_deserialize!(
    FDistribution,
    FDistributionRepr { d1: f64, d2: f64 },
    FDistribution::new(d1, d2)
);

impl FDistribution {
    /// Creates a new F-distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `d1` or `d2` is not positive and finite.
    pub fn new(d1: f64, d2: f64) -> Result<Self, PramanaError> {
        require_positive(d1, "d1")?;
        require_positive(d2, "d2")?;
        Ok(Self { d1, d2 })
    }
}

/// `(CDF, survival)` of the F(d1, d2) distribution at `x`, each without cancellation.
pub(crate) fn f_cdf_sf(x: f64, d1: f64, d2: f64) -> (f64, f64) {
    if x.is_nan() {
        return (f64::NAN, f64::NAN);
    }
    if x <= 0.0 {
        return (0.0, 1.0);
    }
    // u = d1 x / (d1 x + d2), 1 - u = d2 / (d1 x + d2)
    let r = d1 * x / d2;
    let u = 1.0 / (1.0 + 1.0 / r);
    let v = 1.0 / (1.0 + r);
    math::beta_inc(d1 / 2.0, d2 / 2.0, u, v)
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
            - (half_d1 + half_d2) * (self.d1 * x / self.d2).ln_1p()
            - math::ln_beta(half_d1, half_d2);
        ln_pdf.exp()
    }

    /// CDF via the regularized incomplete beta function.
    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        f_cdf_sf(x, self.d1, self.d2).0
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
#[serde(try_from = "CauchyRepr")]
#[non_exhaustive]
pub struct Cauchy {
    /// Location parameter (median).
    pub x0: f64,
    /// Scale parameter. Must be positive.
    pub gamma: f64,
}

validated_deserialize!(
    Cauchy,
    CauchyRepr {
        x0: f64,
        gamma: f64
    },
    Cauchy::new(x0, gamma)
);

impl Cauchy {
    /// Creates a new Cauchy distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `x0` is not finite or `gamma` is not
    /// positive and finite.
    pub fn new(x0: f64, gamma: f64) -> Result<Self, PramanaError> {
        require_finite(x0, "x0")?;
        require_positive(gamma, "gamma")?;
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
#[serde(try_from = "WeibullRepr")]
#[non_exhaustive]
pub struct Weibull {
    /// Shape parameter. Must be positive.
    pub k: f64,
    /// Scale parameter. Must be positive.
    pub lambda: f64,
}

validated_deserialize!(
    Weibull,
    WeibullRepr {
        k: f64,
        lambda: f64
    },
    Weibull::new(k, lambda)
);

impl Weibull {
    /// Creates a new Weibull distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if `k` or `lambda` is not positive and finite.
    pub fn new(k: f64, lambda: f64) -> Result<Self, PramanaError> {
        require_positive(k, "k")?;
        require_positive(lambda, "lambda")?;
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

    /// CDF `1 - exp(-(x/λ)^k)`, computed with `exp_m1` for precision near 0.
    #[inline]
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        -(-(x / self.lambda).powf(self.k)).exp_m1()
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

// ---------------------------------------------------------------------------
// Multivariate normal distribution
// ---------------------------------------------------------------------------

/// Multivariate normal (Gaussian) distribution in d dimensions.
///
/// Parameterized by a mean vector (length d) and a d×d symmetric
/// positive-definite covariance matrix. The Cholesky factor and
/// log-determinant are pre-computed at construction time for efficient
/// sampling and density evaluation.
///
/// Serialization includes the cached factor for compatibility, but
/// deserialization recomputes (and re-validates) it from `mean` and
/// `covariance`; cached fields in the input are ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "MultivariateNormalRepr")]
#[non_exhaustive]
pub struct MultivariateNormal {
    /// Mean vector (length d).
    pub mean: Vec<f64>,
    /// Covariance matrix (d × d).
    pub covariance: Vec<Vec<f64>>,
    /// Pre-computed lower-triangular Cholesky factor L where Σ = LLᵀ.
    cholesky_l: Vec<Vec<f64>>,
    /// Pre-computed log of the determinant of the covariance matrix.
    log_det: f64,
    /// Dimensionality.
    dim: usize,
}

validated_deserialize!(
    MultivariateNormal,
    MultivariateNormalRepr {
        mean: Vec<f64>,
        covariance: Vec<Vec<f64>>,
    },
    MultivariateNormal::new(mean, covariance)
);

/// Relative tolerance for the covariance symmetry check.
const MVN_SYMMETRY_TOL: f64 = 1e-10;

impl MultivariateNormal {
    /// Creates a new multivariate normal distribution.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if:
    /// - `mean` is empty or contains non-finite values
    /// - `covariance` contains non-finite values
    /// - `covariance` is not symmetric (relative tolerance 1e-10) positive-definite
    ///
    /// Returns `DimensionMismatch` if `covariance` is not square or doesn't
    /// match the `mean` length.
    pub fn new(mean: Vec<f64>, covariance: Vec<Vec<f64>>) -> Result<Self, PramanaError> {
        let dim = mean.len();
        if dim == 0 {
            return Err(PramanaError::InvalidParameter(
                "mean vector must be non-empty".into(),
            ));
        }
        if let Some(i) = mean.iter().position(|v| !v.is_finite()) {
            return Err(PramanaError::InvalidParameter(format!(
                "mean[{i}] is not finite"
            )));
        }
        if covariance.len() != dim {
            return Err(PramanaError::DimensionMismatch(format!(
                "covariance has {} rows, expected {dim}",
                covariance.len()
            )));
        }
        for (i, row) in covariance.iter().enumerate() {
            if row.len() != dim {
                return Err(PramanaError::DimensionMismatch(format!(
                    "covariance row {i} has length {}, expected {dim}",
                    row.len()
                )));
            }
            if let Some(j) = row.iter().position(|v| !v.is_finite()) {
                return Err(PramanaError::InvalidParameter(format!(
                    "covariance[{i}][{j}] is not finite"
                )));
            }
        }
        // Symmetry check (relative tolerance, so it is scale-independent)
        for (i, row_i) in covariance.iter().enumerate() {
            for (j, &cov_ij) in row_i.iter().enumerate().skip(i + 1) {
                let cov_ji = covariance[j][i];
                let scale = cov_ij.abs().max(cov_ji.abs()).max(f64::MIN_POSITIVE);
                if (cov_ij - cov_ji).abs() > MVN_SYMMETRY_TOL * scale {
                    return Err(PramanaError::InvalidParameter(format!(
                        "covariance not symmetric: [{i}][{j}]={cov_ij} != [{j}][{i}]={cov_ji}"
                    )));
                }
            }
        }

        // Cholesky decomposition (validates positive-definiteness)
        let cholesky_l = hisab::num::cholesky(&covariance).map_err(|e| {
            PramanaError::InvalidParameter(format!("covariance is not positive-definite: {e}"))
        })?;

        // Log-determinant from Cholesky: det(Σ) = prod(L_ii)^2, so
        // log(det) = 2 * sum(log(L_ii))
        let log_det: f64 = cholesky_l
            .iter()
            .enumerate()
            .map(|(i, row)| row.get(i).copied().unwrap_or(f64::NAN).ln())
            .sum::<f64>()
            * 2.0;
        if !log_det.is_finite() {
            return Err(PramanaError::InvalidParameter(
                "covariance is not positive-definite (degenerate determinant)".into(),
            ));
        }

        Ok(Self {
            mean,
            covariance,
            cholesky_l,
            log_det,
            dim,
        })
    }

    /// Returns the dimensionality.
    #[must_use]
    #[inline]
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Probability density function at point `x`.
    ///
    /// # Errors
    ///
    /// Returns `DimensionMismatch` if `x.len() != dim`.
    #[must_use = "returns the density value"]
    pub fn pdf(&self, x: &[f64]) -> Result<f64, PramanaError> {
        Ok(self.log_pdf(x)?.exp())
    }

    /// Log probability density function at point `x`.
    ///
    /// More numerically stable than `pdf(x).ln()` for extreme values.
    ///
    /// # Errors
    ///
    /// Returns `DimensionMismatch` if `x.len() != dim`.
    #[must_use = "returns the log-density value"]
    pub fn log_pdf(&self, x: &[f64]) -> Result<f64, PramanaError> {
        if x.len() != self.dim {
            return Err(PramanaError::DimensionMismatch(format!(
                "x has length {}, expected {}",
                x.len(),
                self.dim
            )));
        }
        // diff = x - mean
        let diff: Vec<f64> = x.iter().zip(&self.mean).map(|(xi, mi)| xi - mi).collect();

        // cholesky_solve solves (L Lᵀ) w = diff, i.e. w = Σ⁻¹ diff, so the
        // squared Mahalanobis distance is diffᵀ Σ⁻¹ diff = diff · w.
        let w = hisab::num::cholesky_solve(&self.cholesky_l, &diff)
            .map_err(|e| PramanaError::ComputationError(format!("cholesky_solve failed: {e}")))?;
        let mahalanobis_sq: f64 = diff.iter().zip(&w).map(|(d, wi)| d * wi).sum();

        let d = self.dim as f64;
        Ok(-0.5 * (d * (2.0 * PI).ln() + self.log_det + mahalanobis_sq))
    }

    /// Draws a sample: μ + L·z where z ~ N(0, I).
    pub fn sample(&self, rng: &mut impl Rng) -> Vec<f64> {
        // Generate d independent standard normals via Box-Muller
        let mut z = Vec::with_capacity(self.dim);
        while z.len() < self.dim {
            let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
            let u2 = rng.next_f64();
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * PI * u2;
            z.push(r * theta.cos());
            if z.len() < self.dim {
                z.push(r * theta.sin());
            }
        }

        // result = mean + L * z (L is lower-triangular: row i uses z[0..=i])
        let mut result = self.mean.clone();
        for (i, (res_i, row)) in result.iter_mut().zip(&self.cholesky_l).enumerate() {
            let sum: f64 = row.iter().take(i + 1).zip(&z).map(|(l, zi)| l * zi).sum();
            *res_i += sum;
        }
        result
    }

    /// Returns the mean vector.
    #[must_use]
    #[inline]
    pub fn mean_vec(&self) -> &[f64] {
        &self.mean
    }

    /// Returns the covariance matrix.
    #[must_use]
    #[inline]
    pub fn covariance_matrix(&self) -> &[Vec<f64>] {
        &self.covariance
    }
}

/// Read-only hooks for the Cyrius port's golden-value generator (see `crate::oracle`).
#[cfg(feature = "oracle")]
pub(crate) mod oracle;

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

    // --- Multivariate Normal ---

    #[test]
    fn mvn_1d_matches_univariate() {
        // 1D multivariate normal should match scalar Normal
        let mvn = MultivariateNormal::new(vec![2.0], vec![vec![4.0]]).unwrap();
        let n = Normal::new(2.0, 2.0).unwrap(); // std_dev = sqrt(4)
        let x = 3.0;
        let mvn_pdf = mvn.pdf(&[x]).unwrap();
        let n_pdf = n.pdf(x);
        assert!(
            (mvn_pdf - n_pdf).abs() < 1e-10,
            "mvn_pdf={mvn_pdf}, n_pdf={n_pdf}"
        );
    }

    #[test]
    fn mvn_2d_pdf_at_mean() {
        // At the mean, the PDF should be 1 / (2π * sqrt(det(Σ)))
        let cov = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let mvn = MultivariateNormal::new(vec![0.0, 0.0], cov).unwrap();
        let pdf_at_mean = mvn.pdf(&[0.0, 0.0]).unwrap();
        let expected = 1.0 / (2.0 * PI); // det(I) = 1
        assert!(
            (pdf_at_mean - expected).abs() < 1e-10,
            "pdf={pdf_at_mean}, expected={expected}"
        );
    }

    #[test]
    fn mvn_sample_mean_convergence() {
        let mean = vec![1.0, -2.0];
        let cov = vec![vec![1.0, 0.3], vec![0.3, 2.0]];
        let mvn = MultivariateNormal::new(mean.clone(), cov).unwrap();
        let mut rng = SimpleRng::new(42);
        let n = 50_000;
        let mut sum = [0.0; 2];
        for _ in 0..n {
            let s = mvn.sample(&mut rng);
            assert_eq!(s.len(), 2);
            sum[0] += s[0];
            sum[1] += s[1];
        }
        let sample_mean = [sum[0] / n as f64, sum[1] / n as f64];
        assert!(
            (sample_mean[0] - 1.0).abs() < 0.1,
            "mean[0]={}",
            sample_mean[0]
        );
        assert!(
            (sample_mean[1] + 2.0).abs() < 0.1,
            "mean[1]={}",
            sample_mean[1]
        );
    }

    #[test]
    fn mvn_invalid_params() {
        // Empty mean
        assert!(MultivariateNormal::new(vec![], vec![]).is_err());
        // Mismatched dimensions
        assert!(MultivariateNormal::new(vec![0.0], vec![vec![1.0, 0.0]]).is_err());
        // Not symmetric
        assert!(
            MultivariateNormal::new(vec![0.0, 0.0], vec![vec![1.0, 0.5], vec![0.0, 1.0]]).is_err()
        );
        // Not positive-definite
        assert!(
            MultivariateNormal::new(vec![0.0, 0.0], vec![vec![1.0, 2.0], vec![2.0, 1.0]]).is_err()
        );
    }

    #[test]
    fn mvn_dimension_mismatch_pdf() {
        let mvn =
            MultivariateNormal::new(vec![0.0, 0.0], vec![vec![1.0, 0.0], vec![0.0, 1.0]]).unwrap();
        assert!(mvn.pdf(&[1.0]).is_err());
        assert!(mvn.pdf(&[1.0, 2.0, 3.0]).is_err());
    }

    #[test]
    fn mvn_log_pdf_consistency() {
        let cov = vec![vec![2.0, 0.5], vec![0.5, 3.0]];
        let mvn = MultivariateNormal::new(vec![1.0, -1.0], cov).unwrap();
        let x = [2.0, 0.0];
        let log_pdf = mvn.log_pdf(&x).unwrap();
        let pdf = mvn.pdf(&x).unwrap();
        assert!(
            (log_pdf - pdf.ln()).abs() < 1e-10,
            "log_pdf={log_pdf}, ln(pdf)={}",
            pdf.ln()
        );
    }

    #[test]
    fn serde_roundtrip_mvn() {
        let mvn =
            MultivariateNormal::new(vec![1.0, 2.0], vec![vec![4.0, 1.0], vec![1.0, 9.0]]).unwrap();
        let json = serde_json::to_string(&mvn).unwrap();
        let mvn2: MultivariateNormal = serde_json::from_str(&json).unwrap();
        assert_eq!(mvn.mean, mvn2.mean);
        assert_eq!(mvn.covariance, mvn2.covariance);
        assert_eq!(mvn.dim, mvn2.dim);
    }

    // --- Hardening regression tests -------------------------------------

    fn assert_rel(actual: f64, expected: f64, tol: f64, what: &str) {
        let err = ((actual - expected) / expected).abs();
        assert!(
            err <= tol,
            "{what}: got {actual:e}, expected {expected:e} (rel {err:e})"
        );
    }

    #[test]
    fn constructors_reject_non_finite_parameters() {
        let nan = f64::NAN;
        let inf = f64::INFINITY;
        assert!(Normal::new(nan, 1.0).is_err());
        assert!(Normal::new(0.0, nan).is_err());
        assert!(Normal::new(0.0, inf).is_err());
        assert!(Uniform::new(nan, 1.0).is_err());
        assert!(Uniform::new(f64::NEG_INFINITY, 0.0).is_err());
        assert!(Uniform::new(-1e308, 1e308).is_err(), "width overflows");
        assert!(Exponential::new(nan).is_err());
        assert!(Exponential::new(inf).is_err());
        assert!(Poisson::new(nan).is_err());
        assert!(Poisson::new(inf).is_err());
        assert!(Binomial::new(10, nan).is_err());
        assert!(Bernoulli::new(nan).is_err());
        assert!(Gamma::new(nan, 1.0).is_err());
        assert!(Gamma::new(1.0, inf).is_err());
        assert!(Beta::new(1.0, nan).is_err());
        assert!(ChiSquared::new(inf).is_err());
        assert!(StudentT::new(nan).is_err());
        assert!(FDistribution::new(1.0, nan).is_err());
        assert!(Cauchy::new(nan, 1.0).is_err());
        assert!(Cauchy::new(0.0, inf).is_err());
        assert!(Weibull::new(nan, 1.0).is_err());
        assert!(MultivariateNormal::new(vec![nan], vec![vec![1.0]]).is_err());
        assert!(MultivariateNormal::new(vec![0.0], vec![vec![nan]]).is_err());
        assert!(MultivariateNormal::new(vec![0.0], vec![vec![inf]]).is_err());
    }

    #[test]
    fn deserialize_rejects_invalid_parameters() {
        assert!(serde_json::from_str::<Normal>(r#"{"mean":0.0,"std_dev":-1.0}"#).is_err());
        assert!(serde_json::from_str::<Uniform>(r#"{"min":2.0,"max":1.0}"#).is_err());
        assert!(serde_json::from_str::<Exponential>(r#"{"lambda":0.0}"#).is_err());
        assert!(serde_json::from_str::<Poisson>(r#"{"lambda":-3.0}"#).is_err());
        assert!(serde_json::from_str::<Binomial>(r#"{"n":0,"p":0.5}"#).is_err());
        assert!(serde_json::from_str::<Binomial>(r#"{"n":5,"p":1.5}"#).is_err());
        assert!(serde_json::from_str::<Bernoulli>(r#"{"p":-0.1}"#).is_err());
        assert!(serde_json::from_str::<Gamma>(r#"{"alpha":0.0,"beta":1.0}"#).is_err());
        assert!(serde_json::from_str::<Beta>(r#"{"alpha":1.0,"beta":-2.0}"#).is_err());
        assert!(serde_json::from_str::<ChiSquared>(r#"{"df":0.0}"#).is_err());
        assert!(serde_json::from_str::<StudentT>(r#"{"df":-1.0}"#).is_err());
        assert!(serde_json::from_str::<FDistribution>(r#"{"d1":1.0,"d2":0.0}"#).is_err());
        assert!(serde_json::from_str::<Cauchy>(r#"{"x0":0.0,"gamma":0.0}"#).is_err());
        assert!(serde_json::from_str::<Weibull>(r#"{"k":-1.0,"lambda":1.0}"#).is_err());
        // Not positive-definite.
        assert!(
            serde_json::from_str::<MultivariateNormal>(
                r#"{"mean":[0,0],"covariance":[[1,2],[2,1]]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn mvn_deserialize_recomputes_cached_factor() {
        // A tampered `dim`/`cholesky_l` used to be trusted and made sample() panic.
        let json = r#"{"mean":[0,0,0],"covariance":[[1,0,0],[0,1,0],[0,0,1]],
                       "cholesky_l":[[1]],"log_det":5.0,"dim":1}"#;
        let mvn: MultivariateNormal = serde_json::from_str(json).unwrap();
        assert_eq!(mvn.dim(), 3);
        assert!(mvn.log_det.abs() < 1e-12);
        let mut rng = crate::rng::SimpleRng::new(9);
        assert_eq!(mvn.sample(&mut rng).len(), 3);
        // The old wire format (with cached fields) still deserializes.
        let original =
            MultivariateNormal::new(vec![1.0, 2.0], vec![vec![2.0, 0.5], vec![0.5, 1.0]]).unwrap();
        let back: MultivariateNormal =
            serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        assert_eq!(back.cholesky_l, original.cholesky_l);
    }

    #[test]
    fn mvn_symmetry_check_is_scale_relative() {
        // Large-scale covariance with relative asymmetry far below 1e-10 is accepted ...
        let big = 1e12;
        let ok = MultivariateNormal::new(
            vec![0.0, 0.0],
            vec![vec![big, 0.5 * big], vec![0.5 * big + 1e-3, big]],
        );
        assert!(ok.is_ok());
        // ... while a genuinely asymmetric small-scale matrix is rejected.
        let bad = MultivariateNormal::new(vec![0.0, 0.0], vec![vec![1e-6, 1e-7], vec![2e-7, 1e-6]]);
        assert!(bad.is_err());
    }

    #[test]
    fn normal_cdf_tail_precision() {
        let n = Normal::new(0.0, 1.0).unwrap();
        assert_rel(n.cdf(-6.0), 9.865_876_450_377_014e-10, 1e-13, "Φ(-6)");
        assert_rel(n.cdf(-1.0), 0.158_655_253_931_457_07, 1e-15, "Φ(-1)");
        assert!(n.cdf(-40.0) > 0.0 || n.cdf(-40.0) == 0.0);
        assert!((n.cdf(1.0) + n.cdf(-1.0) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn exponential_and_weibull_cdf_small_x() {
        let e = Exponential::new(1.0).unwrap();
        assert_rel(e.cdf(1e-20), 1e-20, 1e-15, "Exp cdf near 0");
        let w = Weibull::new(1.0, 1.0).unwrap();
        assert_rel(w.cdf(1e-20), 1e-20, 1e-15, "Weibull cdf near 0");
    }

    #[test]
    fn poisson_pmf_and_cdf_reference() {
        let p = Poisson::new(2.5).unwrap();
        // e^-2.5 · 2.5³ / 3!
        assert_rel(p.pdf(3.0), 0.213_763_017_249_736_45, 1e-14, "pmf(3; 2.5)");
        // cdf equals the pmf sum
        let sum: f64 = (0..=6).map(|k| p.pdf(k as f64)).sum();
        assert_rel(p.cdf(6.0), sum, 1e-14, "cdf(6; 2.5)");
        // 50-digit decimal references: Q(k + 1, λ)
        let p100 = Poisson::new(100.0).unwrap();
        assert_rel(
            p100.cdf(120.0),
            0.977_330_670_921_647_3,
            1e-13,
            "cdf(120; 100)",
        );
        let p1e4 = Poisson::new(1e4).unwrap();
        assert_rel(
            p1e4.cdf(1e4),
            0.502_659_581_219_007_6,
            1e-12,
            "cdf(1e4; 1e4)",
        );
    }

    #[test]
    fn poisson_huge_arguments_are_fast_and_bounded() {
        // Used to be O(x²) / effectively infinite loops.
        let p = Poisson::new(3.0).unwrap();
        assert_eq!(p.cdf(1e18), 1.0);
        assert_eq!(p.cdf(f64::INFINITY), 1.0);
        assert_eq!(p.pdf(1e15), 0.0);
        assert!(p.cdf(f64::NAN).is_nan());
        let big = Poisson::new(1e12).unwrap();
        assert!((big.cdf(1e12) - 0.5).abs() < 1e-3);
        assert!(big.pdf(1e12) > 0.0);
    }

    #[test]
    fn poisson_ptrs_sampling_moments() {
        for &lambda in &[31.0, 100.0, 1e6] {
            let p = Poisson::new(lambda).unwrap();
            let mut rng = crate::rng::SimpleRng::new(2024);
            let n = 20_000;
            let samples: Vec<f64> = (0..n).map(|_| p.sample(&mut rng)).collect();
            assert!(samples.iter().all(|&s| s >= 0.0 && s.fract() == 0.0));
            let mean = samples.iter().sum::<f64>() / n as f64;
            let var = samples.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
            // 5 standard errors
            assert!(
                (mean - lambda).abs() < 5.0 * (lambda / n as f64).sqrt(),
                "λ={lambda} mean={mean}"
            );
            assert!((var / lambda - 1.0).abs() < 0.06, "λ={lambda} var={var}");
        }
    }

    #[test]
    fn binomial_degenerate_p() {
        let b0 = Binomial::new(10, 0.0).unwrap();
        assert_eq!(b0.pdf(0.0), 1.0);
        assert_eq!(b0.pdf(3.0), 0.0);
        assert_eq!(b0.cdf(0.0), 1.0);
        let b1 = Binomial::new(10, 1.0).unwrap();
        assert_eq!(b1.pdf(10.0), 1.0);
        assert_eq!(b1.pdf(9.0), 0.0);
        assert_eq!(b1.cdf(9.0), 0.0);
        assert_eq!(b1.cdf(10.0), 1.0);
        let mut rng = crate::rng::SimpleRng::new(1);
        assert_eq!(Binomial::new(1000, 1.0).unwrap().sample(&mut rng), 1000.0);
        assert_eq!(Binomial::new(1000, 0.0).unwrap().sample(&mut rng), 0.0);
    }

    #[test]
    fn binomial_pmf_matches_exact_and_cdf_matches_sum() {
        let b = Binomial::new(20, 0.3).unwrap();
        let mut cumulative = 0.0;
        for k in 0..=20u64 {
            // exact C(20,k) 0.3^k 0.7^(20-k)
            let c = crate::combinatorics::combinations(20, k).unwrap() as f64;
            let exact = c * 0.3_f64.powi(k as i32) * 0.7_f64.powi(20 - k as i32);
            assert_rel(b.pdf(k as f64), exact, 1e-13, &format!("pmf({k})"));
            cumulative += exact;
            if k < 20 {
                assert_rel(b.cdf(k as f64), cumulative, 1e-12, &format!("cdf({k})"));
            }
        }
    }

    #[test]
    fn binomial_large_n_is_fast_and_accurate() {
        let n = 1_000_000u64;
        let b = Binomial::new(n, 0.5).unwrap();
        // Symmetry: P(X <= n/2) = 1/2 + P(X = n/2)/2.
        let half = (n / 2) as f64;
        assert_rel(b.cdf(half), 0.5 + 0.5 * b.pdf(half), 1e-12, "symmetric cdf");
        // P(X = n/2) ≈ 1/sqrt(π n / 2) (1 - 1/(4n))
        let approx = (1.0 / (PI * half).sqrt()) * (1.0 - 1.0 / (4.0 * n as f64));
        assert_rel(b.pdf(half), approx, 1e-9, "central pmf");
        // Sampling huge n terminates quickly and has the right mean.
        let big = Binomial::new(u64::MAX, 0.25).unwrap();
        let mut rng = crate::rng::SimpleRng::new(77);
        let s = big.sample(&mut rng);
        assert!((s / u64::MAX as f64 - 0.25).abs() < 1e-6);
    }

    #[test]
    fn binomial_large_n_sampling_moments() {
        let b = Binomial::new(10_000, 0.3).unwrap();
        let mut rng = crate::rng::SimpleRng::new(31337);
        let n = 20_000;
        let samples: Vec<f64> = (0..n).map(|_| b.sample(&mut rng)).collect();
        assert!(
            samples
                .iter()
                .all(|&s| (0.0..=10_000.0).contains(&s) && s.fract() == 0.0)
        );
        let mean = samples.iter().sum::<f64>() / n as f64;
        let var = samples.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        assert!(
            (mean - 3000.0).abs() < 5.0 * (2100.0 / n as f64).sqrt(),
            "mean={mean}"
        );
        assert!((var / 2100.0 - 1.0).abs() < 0.06, "var={var}");
    }

    #[test]
    fn student_t_cdf_closed_forms() {
        let t1 = StudentT::new(1.0).unwrap();
        let t2 = StudentT::new(2.0).unwrap();
        for &x in &[-30.0, -2.5, -0.3, 0.0, 0.7, 4.0] {
            assert!(
                (t1.cdf(x) - (0.5 + f64::atan(x) / PI)).abs() < 1e-15,
                "df=1 x={x}"
            );
            let df2 = 0.5 + x / (2.0 * (2.0 + x * x).sqrt());
            assert!((t2.cdf(x) - df2).abs() < 1e-15, "df=2 x={x}");
        }
        // Far tail keeps relative precision: 0.5 - atan(1e10)/π = atan(1e-10)/π.
        assert_rel(t1.cdf(-1e10), (1e-10_f64).atan() / PI, 1e-12, "df=1 tail");
        assert_eq!(t1.cdf(f64::INFINITY), 1.0);
        assert_eq!(t1.cdf(f64::NEG_INFINITY), 0.0);
        assert!(t1.cdf(f64::NAN).is_nan());
    }

    #[test]
    fn student_t_large_df_approaches_normal() {
        let t = StudentT::new(1e9).unwrap();
        let z = Normal::new(0.0, 1.0).unwrap();
        assert!((t.cdf(1.96) - z.cdf(1.96)).abs() < 1e-9);
        assert!((t.pdf(0.3) - z.pdf(0.3)).abs() < 1e-9);
    }

    #[test]
    fn student_t_pdf_matches_gamma_form() {
        for &df in &[0.5, 3.0, 30.0] {
            let t = StudentT::new(df).unwrap();
            let x: f64 = 1.3;
            let expected = (math::ln_gamma((df + 1.0) / 2.0)
                - math::ln_gamma(df / 2.0)
                - 0.5 * (df * PI).ln()
                - (df + 1.0) / 2.0 * (1.0 + x * x / df).ln())
            .exp();
            assert_rel(t.pdf(x), expected, 1e-13, &format!("t pdf df={df}"));
        }
    }

    #[test]
    fn chi_squared_large_df_cdf() {
        // Previously 0.11 because the incomplete-gamma series was truncated.
        let c = ChiSquared::new(1e6).unwrap();
        assert_rel(
            c.cdf(1e6),
            1.0 - 0.499_811_936_803_394_5,
            1e-11,
            "χ²(1e6) at 1e6",
        );
    }

    #[test]
    fn f_distribution_closed_form_and_tail() {
        // F(2, d2) CDF = 1 - (1 + 2x/d2)^(-d2/2)
        for &d2 in &[5.0_f64, 100.0, 1e6] {
            let f = FDistribution::new(2.0, d2).unwrap();
            let x: f64 = 2.5;
            let expected = -(-d2 / 2.0 * (2.0 * x / d2).ln_1p()).exp_m1();
            assert_rel(f.cdf(x), expected, 1e-12, &format!("F(2,{d2})"));
        }
        // Upper tail is computed directly (no 1 - cdf rounding to zero).
        let (_, sf) = f_cdf_sf(1e6, 2.0, 5.0);
        let expected = (1.0 + 2.0 * 1e6 / 5.0_f64).powf(-2.5);
        assert_rel(sf, expected, 1e-11, "F upper tail");
        assert_eq!(f_cdf_sf(f64::INFINITY, 2.0, 5.0), (1.0, 0.0));
    }
}
