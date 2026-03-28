//! Bayesian inference and naive Bayes classification.

use crate::error::PramanaError;
use serde::{Deserialize, Serialize};

/// A single Bayesian update step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayesianUpdate {
    /// Prior probability P(A).
    pub prior: f64,
    /// Likelihood P(B|A).
    pub likelihood: f64,
    /// Evidence P(B).
    pub evidence: f64,
}

impl BayesianUpdate {
    /// Creates a new `BayesianUpdate`.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if evidence is zero or any probability is not in `[0, 1]`.
    pub fn new(prior: f64, likelihood: f64, evidence: f64) -> Result<Self, PramanaError> {
        if evidence == 0.0 {
            return Err(PramanaError::InvalidParameter(
                "evidence P(B) must be non-zero".into(),
            ));
        }
        if !(0.0..=1.0).contains(&prior) {
            return Err(PramanaError::InvalidParameter(
                "prior must be in [0, 1]".into(),
            ));
        }
        if !(0.0..=1.0).contains(&likelihood) {
            return Err(PramanaError::InvalidParameter(
                "likelihood must be in [0, 1]".into(),
            ));
        }
        if !(0.0..=1.0).contains(&evidence) {
            return Err(PramanaError::InvalidParameter(
                "evidence must be in [0, 1]".into(),
            ));
        }
        Ok(Self {
            prior,
            likelihood,
            evidence,
        })
    }

    /// Computes the posterior probability P(A|B) = P(B|A) * P(A) / P(B).
    #[must_use]
    #[inline]
    pub fn posterior(&self) -> f64 {
        self.likelihood * self.prior / self.evidence
    }
}

/// Computes Bayes' theorem: P(A|B) = P(B|A) * P(A) / P(B).
///
/// # Errors
///
/// Returns `InvalidParameter` if `evidence` is zero.
#[must_use = "returns the posterior probability"]
pub fn bayes_theorem(prior: f64, likelihood: f64, evidence: f64) -> Result<f64, PramanaError> {
    if evidence == 0.0 {
        return Err(PramanaError::InvalidParameter(
            "evidence P(B) must be non-zero".into(),
        ));
    }
    Ok(likelihood * prior / evidence)
}

/// Naive Bayes classifier.
///
/// Given a feature vector, class priors, and per-class per-feature likelihoods,
/// returns the index of the most probable class.
///
/// # Arguments
///
/// * `features` - Feature values for the observation (length F).
/// * `class_priors` - Prior probability for each class (length C, should sum to 1).
/// * `class_likelihoods` - For each class, the PDF value for each feature.
///   Shape: `C x F`. Each inner Vec has length F matching `features`.
///
/// # Errors
///
/// Returns `InvalidSample` if `class_priors` is empty.
/// Returns `DimensionMismatch` if inner likelihood vectors do not match feature count.
#[must_use = "returns the predicted class index"]
pub fn naive_bayes_classify(
    features: &[f64],
    class_priors: &[f64],
    class_likelihoods: &[Vec<f64>],
) -> Result<usize, PramanaError> {
    if class_priors.is_empty() {
        return Err(PramanaError::InvalidSample(
            "need at least one class".into(),
        ));
    }
    if class_priors.len() != class_likelihoods.len() {
        return Err(PramanaError::DimensionMismatch(
            "class_priors and class_likelihoods must have the same length".into(),
        ));
    }
    for (i, likelihoods) in class_likelihoods.iter().enumerate() {
        if likelihoods.len() != features.len() {
            return Err(PramanaError::DimensionMismatch(format!(
                "class {} likelihoods length {} != features length {}",
                i,
                likelihoods.len(),
                features.len()
            )));
        }
    }

    // Compute log-posterior for each class to avoid underflow
    let mut best_class = 0;
    let mut best_log_posterior = f64::NEG_INFINITY;

    for (c, prior) in class_priors.iter().enumerate() {
        if *prior <= 0.0 {
            continue;
        }
        let mut log_posterior = prior.ln();
        for (f, &likelihood) in class_likelihoods[c].iter().enumerate() {
            if likelihood <= 0.0 {
                // Use a small epsilon to avoid log(0)
                log_posterior += 1e-300_f64.ln();
            } else {
                let _ = f; // suppress unused; f is used for the iteration
                log_posterior += likelihood.ln();
            }
        }
        if log_posterior > best_log_posterior {
            best_log_posterior = log_posterior;
            best_class = c;
        }
    }

    Ok(best_class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayes_theorem() {
        // P(A) = 0.01, P(B|A) = 0.9, P(B) = 0.1
        // P(A|B) = 0.9 * 0.01 / 0.1 = 0.09
        let result = bayes_theorem(0.01, 0.9, 0.1).unwrap();
        assert!((result - 0.09).abs() < 1e-10);
    }

    #[test]
    fn test_bayesian_update() {
        let update = BayesianUpdate::new(0.5, 0.8, 0.6).unwrap();
        let posterior = update.posterior();
        let expected = 0.8 * 0.5 / 0.6;
        assert!((posterior - expected).abs() < 1e-10);
    }

    #[test]
    fn test_bayes_zero_evidence() {
        assert!(bayes_theorem(0.5, 0.5, 0.0).is_err());
    }

    #[test]
    fn test_naive_bayes() {
        // Two classes, two features
        let features = [1.0, 0.5];
        let priors = [0.5, 0.5];
        // Class 0 has high likelihood for these features
        let likelihoods = vec![vec![0.9, 0.8], vec![0.1, 0.2]];
        let class = naive_bayes_classify(&features, &priors, &likelihoods).unwrap();
        assert_eq!(class, 0);
    }

    #[test]
    fn serde_roundtrip() {
        let update = BayesianUpdate::new(0.3, 0.7, 0.4).unwrap();
        let json = serde_json::to_string(&update).unwrap();
        let update2: BayesianUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(update.prior, update2.prior);
        assert_eq!(update.likelihood, update2.likelihood);
        assert_eq!(update.evidence, update2.evidence);
    }
}
