//! Monte Carlo methods: integration and simulation.

use crate::error::PramanaError;
use crate::rng::Rng;
pub use crate::rng::SimpleRng;
use serde::{Deserialize, Serialize};

/// Result of a Monte Carlo estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateResult {
    /// The estimated value.
    pub value: f64,
    /// Standard error of the estimate.
    pub std_error: f64,
}

/// Estimates a definite integral of `f` over `[a, b]` using Monte Carlo sampling.
///
/// The integral is approximated as `(b - a) * mean(f(x_i))` where `x_i` are
/// uniformly sampled from `[a, b]`.
///
/// # Errors
///
/// Returns `InvalidParameter` if `n_samples` is 0 or `a >= b`.
#[must_use = "returns the integration estimate"]
pub fn monte_carlo_integrate(
    f: impl Fn(f64) -> f64,
    a: f64,
    b: f64,
    n_samples: usize,
    rng: &mut impl Rng,
) -> Result<EstimateResult, PramanaError> {
    if n_samples == 0 {
        return Err(PramanaError::InvalidParameter(
            "n_samples must be positive".into(),
        ));
    }
    if a >= b {
        return Err(PramanaError::InvalidParameter(
            "lower bound must be less than upper bound".into(),
        ));
    }

    let width = b - a;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for _ in 0..n_samples {
        let x = a + rng.next_f64() * width;
        let val = f(x);
        sum += val;
        sum_sq += val * val;
    }

    let n = n_samples as f64;
    let mean = sum / n;
    let value = width * mean;

    // Variance of f(x_i)
    let var = sum_sq / n - mean * mean;
    let std_error = width * (var / n).sqrt();

    Ok(EstimateResult { value, std_error })
}

/// Estimates pi using the Monte Carlo circle method.
///
/// Samples random points in the unit square and counts those falling
/// inside the unit circle (x^2 + y^2 <= 1). Pi ~ 4 * (inside / total).
///
/// # Errors
///
/// Returns `InvalidParameter` if `n_samples` is 0.
#[must_use = "returns the pi estimate"]
pub fn monte_carlo_pi(n_samples: usize, rng: &mut impl Rng) -> Result<f64, PramanaError> {
    if n_samples == 0 {
        return Err(PramanaError::InvalidParameter(
            "n_samples must be positive".into(),
        ));
    }

    let mut inside = 0u64;
    for _ in 0..n_samples {
        let x = rng.next_f64() * 2.0 - 1.0;
        let y = rng.next_f64() * 2.0 - 1.0;
        if x * x + y * y <= 1.0 {
            inside += 1;
        }
    }

    Ok(4.0 * inside as f64 / n_samples as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monte_carlo_pi() {
        let mut rng = SimpleRng::new(42);
        let pi = monte_carlo_pi(100_000, &mut rng).unwrap();
        assert!(
            (pi - std::f64::consts::PI).abs() < 0.05,
            "pi estimate {pi} too far from actual"
        );
    }

    #[test]
    fn test_monte_carlo_integrate_x_squared() {
        // Integral of x^2 from 0 to 1 = 1/3
        let mut rng = SimpleRng::new(42);
        let result = monte_carlo_integrate(|x| x * x, 0.0, 1.0, 100_000, &mut rng).unwrap();
        assert!(
            (result.value - 1.0 / 3.0).abs() < 0.01,
            "integral estimate {} too far from 1/3",
            result.value
        );
        assert!(result.std_error > 0.0);
    }

    #[test]
    fn test_zero_samples() {
        let mut rng = SimpleRng::new(1);
        assert!(monte_carlo_pi(0, &mut rng).is_err());
        assert!(monte_carlo_integrate(|x| x, 0.0, 1.0, 0, &mut rng).is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let est = EstimateResult {
            value: 3.15,
            std_error: 0.01,
        };
        let json = serde_json::to_string(&est).unwrap();
        let est2: EstimateResult = serde_json::from_str(&json).unwrap();
        assert_eq!(est.value, est2.value);
    }
}
