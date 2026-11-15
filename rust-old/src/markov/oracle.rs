//! The hidden Markov model's private scaled forward pass, exposed unchanged for the oracle
//! (see `crate::oracle`).

use super::HiddenMarkovModel;
use crate::error::PramanaError;

/// Rabiner's scaled forward pass, after the same input checks `forward_log_likelihood`
/// makes. Returns `(alpha, scales)`: one normalized row per observation, plus its scale.
///
/// # Errors
///
/// The same errors as `forward_log_likelihood`: `DimensionMismatch` for inconsistent
/// matrices, and `InvalidSample` for empty or out-of-range observations.
pub fn hmm_forward_scaled(
    hmm: &HiddenMarkovModel,
    observations: &[usize],
) -> Result<(Vec<Vec<f64>>, Vec<f64>), PramanaError> {
    hmm.check_inputs(observations)?;
    let forward = hmm.forward_scaled(observations);
    Ok((forward.alpha, forward.scales))
}

/// The log-likelihood `Σ ln cₜ` from the forward pass's scales.
#[must_use]
pub fn hmm_log_likelihood(scales: &[f64]) -> f64 {
    super::log_likelihood(scales)
}
