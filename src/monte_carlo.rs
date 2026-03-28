//! Monte Carlo methods: integration, simulation, and MCMC.

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

// ---------------------------------------------------------------------------
// MCMC: Metropolis-Hastings
// ---------------------------------------------------------------------------

/// Result of an MCMC sampling run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McmcResult {
    /// The chain of samples (after burn-in).
    pub samples: Vec<Vec<f64>>,
    /// Acceptance rate (fraction of proposals accepted).
    pub acceptance_rate: f64,
}

/// Runs the Metropolis-Hastings algorithm to sample from a target distribution.
///
/// The target distribution is specified by its (unnormalized) log-density
/// `log_target`. The proposal is a symmetric Gaussian random walk with the
/// given `proposal_std` for each dimension.
///
/// Returns `n_samples` samples after discarding `burn_in` initial samples.
///
/// # Arguments
///
/// * `log_target` - Function returning the log of the (unnormalized) target density.
/// * `initial` - Starting point (determines dimensionality).
/// * `proposal_std` - Standard deviation of the Gaussian proposal in each dimension.
/// * `n_samples` - Number of samples to collect (after burn-in).
/// * `burn_in` - Number of initial samples to discard.
/// * `rng` - Random number generator.
///
/// # Errors
///
/// Returns `InvalidParameter` if `initial` is empty, `proposal_std <= 0`,
/// or `n_samples` is 0.
pub fn metropolis_hastings(
    log_target: impl Fn(&[f64]) -> f64,
    initial: &[f64],
    proposal_std: f64,
    n_samples: usize,
    burn_in: usize,
    rng: &mut impl Rng,
) -> Result<McmcResult, PramanaError> {
    let dim = initial.len();
    if dim == 0 {
        return Err(PramanaError::InvalidParameter(
            "initial point must be non-empty".into(),
        ));
    }
    if proposal_std <= 0.0 {
        return Err(PramanaError::InvalidParameter(
            "proposal_std must be positive".into(),
        ));
    }
    if n_samples == 0 {
        return Err(PramanaError::InvalidParameter(
            "n_samples must be positive".into(),
        ));
    }

    let total = n_samples + burn_in;
    let mut current = initial.to_vec();
    let mut log_current = log_target(&current);
    let mut samples = Vec::with_capacity(n_samples);
    let mut accepted: u64 = 0;

    for step in 0..total {
        // Propose: current + N(0, proposal_std) per dimension
        let proposal: Vec<f64> = current
            .iter()
            .map(|&xi| {
                let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
                let u2 = rng.next_f64();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                xi + proposal_std * z
            })
            .collect();

        let log_proposal = log_target(&proposal);

        // Accept/reject
        let log_alpha = log_proposal - log_current;
        let accept = if log_alpha >= 0.0 {
            true
        } else {
            rng.next_f64() < log_alpha.exp()
        };

        if accept {
            current = proposal;
            log_current = log_proposal;
            accepted += 1;
        }

        if step >= burn_in {
            samples.push(current.clone());
        }
    }

    let acceptance_rate = accepted as f64 / total as f64;

    Ok(McmcResult {
        samples,
        acceptance_rate,
    })
}

// ---------------------------------------------------------------------------
// MCMC: Gibbs sampling
// ---------------------------------------------------------------------------

/// Runs Gibbs sampling to draw from a multivariate distribution.
///
/// Each dimension is updated in turn by sampling from its full conditional
/// distribution. The user provides a vector of conditional samplers, one per
/// dimension. Each sampler receives the current state and the RNG, and returns
/// the new value for that coordinate.
///
/// # Arguments
///
/// * `conditionals` - One sampler per dimension: `fn(state: &[f64], rng) -> f64`.
/// * `initial` - Starting point (length must match `conditionals`).
/// * `n_samples` - Number of samples to collect (after burn-in).
/// * `burn_in` - Number of initial samples to discard.
/// * `rng` - Random number generator.
///
/// # Errors
///
/// Returns `InvalidParameter` if `conditionals` is empty, lengths mismatch,
/// or `n_samples` is 0.
pub fn gibbs_sampling<F>(
    conditionals: &[F],
    initial: &[f64],
    n_samples: usize,
    burn_in: usize,
    rng: &mut impl Rng,
) -> Result<McmcResult, PramanaError>
where
    F: Fn(&[f64], &mut dyn Rng) -> f64,
{
    let dim = conditionals.len();
    if dim == 0 {
        return Err(PramanaError::InvalidParameter(
            "need at least one conditional sampler".into(),
        ));
    }
    if initial.len() != dim {
        return Err(PramanaError::DimensionMismatch(format!(
            "initial has length {}, expected {dim}",
            initial.len()
        )));
    }
    if n_samples == 0 {
        return Err(PramanaError::InvalidParameter(
            "n_samples must be positive".into(),
        ));
    }

    let total = n_samples + burn_in;
    let mut state = initial.to_vec();
    let mut samples = Vec::with_capacity(n_samples);

    for step in 0..total {
        for (j, cond) in conditionals.iter().enumerate() {
            state[j] = cond(&state, rng);
        }
        if step >= burn_in {
            samples.push(state.clone());
        }
    }

    Ok(McmcResult {
        samples,
        acceptance_rate: 1.0, // Gibbs always accepts
    })
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

    // --- Metropolis-Hastings ---

    #[test]
    fn mh_samples_standard_normal() {
        // Sample from N(0,1) using log-target = -x²/2
        let mut rng = SimpleRng::new(42);
        let result =
            metropolis_hastings(|x| -0.5 * x[0] * x[0], &[0.0], 1.0, 50_000, 5_000, &mut rng)
                .unwrap();
        assert_eq!(result.samples.len(), 50_000);
        assert!(result.acceptance_rate > 0.1 && result.acceptance_rate < 0.9);

        // Check sample mean ≈ 0
        let mean: f64 =
            result.samples.iter().map(|s| s[0]).sum::<f64>() / result.samples.len() as f64;
        assert!(mean.abs() < 0.1, "sample mean = {mean}");

        // Check sample variance ≈ 1
        let var: f64 = result
            .samples
            .iter()
            .map(|s| (s[0] - mean).powi(2))
            .sum::<f64>()
            / result.samples.len() as f64;
        assert!((var - 1.0).abs() < 0.2, "sample variance = {var}");
    }

    #[test]
    fn mh_2d_target() {
        // Sample from bivariate with log-target = -(x² + y²)/2
        let mut rng = SimpleRng::new(123);
        let result = metropolis_hastings(
            |x| -0.5 * (x[0] * x[0] + x[1] * x[1]),
            &[0.0, 0.0],
            0.5,
            20_000,
            2_000,
            &mut rng,
        )
        .unwrap();
        assert_eq!(result.samples[0].len(), 2);
        let mean_x: f64 =
            result.samples.iter().map(|s| s[0]).sum::<f64>() / result.samples.len() as f64;
        let mean_y: f64 =
            result.samples.iter().map(|s| s[1]).sum::<f64>() / result.samples.len() as f64;
        assert!(mean_x.abs() < 0.15, "mean_x = {mean_x}");
        assert!(mean_y.abs() < 0.15, "mean_y = {mean_y}");
    }

    #[test]
    fn mh_invalid_params() {
        let mut rng = SimpleRng::new(1);
        // empty initial
        assert!(metropolis_hastings(|_| 0.0, &[], 1.0, 100, 0, &mut rng).is_err());
        // proposal_std <= 0
        assert!(metropolis_hastings(|_| 0.0, &[0.0], 0.0, 100, 0, &mut rng).is_err());
        // n_samples = 0
        assert!(metropolis_hastings(|_| 0.0, &[0.0], 1.0, 0, 0, &mut rng).is_err());
    }

    #[test]
    fn mh_serde_roundtrip() {
        let result = McmcResult {
            samples: vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            acceptance_rate: 0.5,
        };
        let json = serde_json::to_string(&result).unwrap();
        let r2: McmcResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.samples, r2.samples);
        assert_eq!(result.acceptance_rate, r2.acceptance_rate);
    }

    // --- Gibbs sampling ---

    // --- Gibbs sampling ---

    type GibbsCond = Box<dyn Fn(&[f64], &mut dyn Rng) -> f64>;

    #[test]
    fn gibbs_independent_normals() {
        // Sample from two independent N(0,1) using Gibbs with known conditionals
        use std::f64::consts::PI;
        fn sample_normal(rng: &mut dyn Rng) -> f64 {
            let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
            let u2 = rng.next_f64();
            (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
        }
        let conditionals: Vec<GibbsCond> = vec![
            Box::new(|_, rng: &mut dyn Rng| sample_normal(rng)),
            Box::new(|_, rng: &mut dyn Rng| sample_normal(rng)),
        ];
        let mut rng = SimpleRng::new(42);
        let result = gibbs_sampling(&conditionals, &[0.0, 0.0], 20_000, 1_000, &mut rng).unwrap();
        assert_eq!(result.samples.len(), 20_000);
        assert!((result.acceptance_rate - 1.0).abs() < 1e-10);

        let mean_x: f64 =
            result.samples.iter().map(|s| s[0]).sum::<f64>() / result.samples.len() as f64;
        assert!(mean_x.abs() < 0.1, "mean_x = {mean_x}");
    }

    #[test]
    fn gibbs_invalid_params() {
        let mut rng = SimpleRng::new(1);
        let conds: Vec<GibbsCond> = vec![];
        assert!(gibbs_sampling(&conds, &[], 100, 0, &mut rng).is_err());

        let conds: Vec<GibbsCond> = vec![Box::new(|_, _: &mut dyn Rng| 0.0)];
        // dimension mismatch
        assert!(gibbs_sampling(&conds, &[0.0, 0.0], 100, 0, &mut rng).is_err());
        // n_samples = 0
        assert!(gibbs_sampling(&conds, &[0.0], 0, 0, &mut rng).is_err());
    }
}
