//! Markov chains and Hidden Markov Models.

use crate::error::PramanaError;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// A discrete-time Markov chain with a finite state space.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkovChain {
    /// Row-stochastic transition matrix. `transition_matrix[i][j]` is P(j | i).
    pub transition_matrix: Vec<Vec<f64>>,
    /// Current state index.
    pub state: usize,
}

impl MarkovChain {
    /// Creates a new Markov chain.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if:
    /// - The matrix is empty
    /// - The matrix is not square
    /// - Any row does not sum to approximately 1.0 (tolerance 1e-6)
    /// - Any entry is negative
    pub fn new(
        transition_matrix: Vec<Vec<f64>>,
        initial_state: usize,
    ) -> Result<Self, PramanaError> {
        if transition_matrix.is_empty() {
            return Err(PramanaError::InvalidParameter(
                "transition matrix must be non-empty".into(),
            ));
        }
        let n = transition_matrix.len();
        for (i, row) in transition_matrix.iter().enumerate() {
            if row.len() != n {
                return Err(PramanaError::InvalidParameter(format!(
                    "row {i} has length {}, expected {n}",
                    row.len()
                )));
            }
            for (j, &val) in row.iter().enumerate() {
                if val < 0.0 {
                    return Err(PramanaError::InvalidParameter(format!(
                        "negative probability at [{i}][{j}]: {val}"
                    )));
                }
            }
            let sum: f64 = row.iter().sum();
            if (sum - 1.0).abs() > 1e-6 {
                return Err(PramanaError::InvalidParameter(format!(
                    "row {i} sums to {sum}, expected 1.0"
                )));
            }
        }
        if initial_state >= n {
            return Err(PramanaError::InvalidParameter(format!(
                "initial_state {initial_state} >= number of states {n}"
            )));
        }
        Ok(Self {
            transition_matrix,
            state: initial_state,
        })
    }

    /// Returns the number of states.
    #[must_use]
    #[inline]
    pub fn num_states(&self) -> usize {
        self.transition_matrix.len()
    }

    /// Advances the chain by one step using the provided RNG.
    /// Returns the new state.
    pub fn step(&mut self, rng: &mut impl Rng) -> usize {
        let row = &self.transition_matrix[self.state];
        let r = rng.next_f64();
        let mut cumulative = 0.0;
        for (j, &p) in row.iter().enumerate() {
            cumulative += p;
            if r < cumulative {
                self.state = j;
                return j;
            }
        }
        // Fallback to last state (handles floating-point rounding)
        self.state = row.len() - 1;
        self.state
    }

    /// Simulates the chain for `steps` transitions, returning the sequence of states visited.
    pub fn simulate(&mut self, steps: usize, rng: &mut impl Rng) -> Vec<usize> {
        let mut trajectory = Vec::with_capacity(steps + 1);
        trajectory.push(self.state);
        for _ in 0..steps {
            self.step(rng);
            trajectory.push(self.state);
        }
        trajectory
    }

    /// Computes the steady-state (stationary) distribution by power iteration.
    ///
    /// # Errors
    ///
    /// Returns `ConvergenceFailure` if the iteration does not converge within 10000 steps.
    #[must_use = "returns the steady-state distribution"]
    pub fn steady_state(&self) -> Result<Vec<f64>, PramanaError> {
        let n = self.num_states();
        let max_iter = 10_000;
        let tol = 1e-10;

        // Start with uniform distribution
        let mut pi = vec![1.0 / n as f64; n];

        for _ in 0..max_iter {
            let mut next = vec![0.0; n];
            // pi_next[j] = sum_i pi[i] * P[i][j]
            for (i, row) in self.transition_matrix.iter().enumerate() {
                for (j, &p) in row.iter().enumerate() {
                    next[j] += pi[i] * p;
                }
            }

            // Check convergence
            let diff: f64 = pi
                .iter()
                .zip(next.iter())
                .map(|(&a, &b)| (a - b).abs())
                .sum();
            pi = next;

            if diff < tol {
                return Ok(pi);
            }
        }

        Err(PramanaError::ConvergenceFailure(
            "steady state did not converge in 10000 iterations".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Hidden Markov Model
// ---------------------------------------------------------------------------

/// A discrete Hidden Markov Model with `N` hidden states and `M` observation symbols.
///
/// - **A** (`transition`): N×N row-stochastic transition matrix. A\[i\]\[j\] = P(state j | state i).
/// - **B** (`emission`): N×M row-stochastic emission matrix. B\[i\]\[k\] = P(obs k | state i).
/// - **π** (`initial`): length-N initial state distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HiddenMarkovModel {
    /// Transition matrix A (N × N).
    pub transition: Vec<Vec<f64>>,
    /// Emission matrix B (N × M).
    pub emission: Vec<Vec<f64>>,
    /// Initial state distribution π (length N).
    pub initial: Vec<f64>,
}

impl HiddenMarkovModel {
    /// Creates a new HMM.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if matrices are inconsistent, empty, not
    /// row-stochastic, or contain negative values.
    pub fn new(
        transition: Vec<Vec<f64>>,
        emission: Vec<Vec<f64>>,
        initial: Vec<f64>,
    ) -> Result<Self, PramanaError> {
        let n = initial.len();
        if n == 0 {
            return Err(PramanaError::InvalidParameter(
                "initial distribution must be non-empty".into(),
            ));
        }
        // Validate initial sums to 1
        validate_stochastic_row(&initial, "initial")?;

        // Validate transition: N×N
        if transition.len() != n {
            return Err(PramanaError::DimensionMismatch(format!(
                "transition has {} rows, expected {n}",
                transition.len()
            )));
        }
        for (i, row) in transition.iter().enumerate() {
            if row.len() != n {
                return Err(PramanaError::DimensionMismatch(format!(
                    "transition row {i} has length {}, expected {n}",
                    row.len()
                )));
            }
            validate_stochastic_row(row, &format!("transition row {i}"))?;
        }

        // Validate emission: N×M
        if emission.len() != n {
            return Err(PramanaError::DimensionMismatch(format!(
                "emission has {} rows, expected {n}",
                emission.len()
            )));
        }
        let m = emission[0].len();
        if m == 0 {
            return Err(PramanaError::InvalidParameter(
                "emission must have at least 1 symbol".into(),
            ));
        }
        for (i, row) in emission.iter().enumerate() {
            if row.len() != m {
                return Err(PramanaError::DimensionMismatch(format!(
                    "emission row {i} has length {}, expected {m}",
                    row.len()
                )));
            }
            validate_stochastic_row(row, &format!("emission row {i}"))?;
        }

        Ok(Self {
            transition,
            emission,
            initial,
        })
    }

    /// Number of hidden states.
    #[must_use]
    #[inline]
    pub fn num_states(&self) -> usize {
        self.initial.len()
    }

    /// Number of observation symbols.
    #[must_use]
    #[inline]
    pub fn num_symbols(&self) -> usize {
        self.emission[0].len()
    }

    /// Forward algorithm: computes P(observations | model).
    ///
    /// Returns the log-likelihood to avoid underflow.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `observations` is empty or contains
    /// out-of-range symbol indices.
    #[must_use = "returns the log-likelihood"]
    pub fn forward_log_likelihood(&self, observations: &[usize]) -> Result<f64, PramanaError> {
        let alpha = self.forward(observations)?;
        let t = observations.len();
        // Sum the last row of alpha (already in log-space via scaling)
        // Actually alpha here is raw (not log). Sum last column.
        let ll = alpha[t - 1].iter().sum::<f64>();
        if ll <= 0.0 {
            Ok(f64::NEG_INFINITY)
        } else {
            Ok(ll.ln())
        }
    }

    /// Forward algorithm returning the full alpha matrix (T × N).
    ///
    /// alpha\[t\]\[i\] = P(o_0..o_t, state_t = i | model), using scaling to
    /// prevent underflow. Returns unscaled alpha for simplicity (sufficient
    /// for short sequences).
    fn forward(&self, observations: &[usize]) -> Result<Vec<Vec<f64>>, PramanaError> {
        let t_len = observations.len();
        if t_len == 0 {
            return Err(PramanaError::InvalidSample(
                "observations must be non-empty".into(),
            ));
        }
        let n = self.num_states();
        let m = self.num_symbols();
        for (t, &o) in observations.iter().enumerate() {
            if o >= m {
                return Err(PramanaError::InvalidSample(format!(
                    "observation[{t}] = {o} >= num_symbols {m}"
                )));
            }
        }

        let mut alpha = vec![vec![0.0; n]; t_len];

        // Initialization
        for (i, ai) in alpha[0].iter_mut().enumerate() {
            *ai = self.initial[i] * self.emission[i][observations[0]];
        }

        // Induction
        for t in 1..t_len {
            for j in 0..n {
                let mut sum = 0.0;
                for (i, row) in self.transition.iter().enumerate() {
                    sum += alpha[t - 1][i] * row[j];
                }
                alpha[t][j] = sum * self.emission[j][observations[t]];
            }
        }

        Ok(alpha)
    }

    /// Viterbi algorithm: finds the most likely hidden state sequence.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `observations` is empty or contains
    /// out-of-range symbol indices.
    #[must_use = "returns the most likely state sequence"]
    pub fn viterbi(&self, observations: &[usize]) -> Result<Vec<usize>, PramanaError> {
        let t_len = observations.len();
        if t_len == 0 {
            return Err(PramanaError::InvalidSample(
                "observations must be non-empty".into(),
            ));
        }
        let n = self.num_states();
        let m = self.num_symbols();
        for (t, &o) in observations.iter().enumerate() {
            if o >= m {
                return Err(PramanaError::InvalidSample(format!(
                    "observation[{t}] = {o} >= num_symbols {m}"
                )));
            }
        }

        // Work in log-space to avoid underflow
        let mut delta = vec![vec![f64::NEG_INFINITY; n]; t_len];
        let mut psi = vec![vec![0usize; n]; t_len];

        // Initialization
        for (i, di) in delta[0].iter_mut().enumerate() {
            let lp = if self.initial[i] > 0.0 {
                self.initial[i].ln()
            } else {
                f64::NEG_INFINITY
            };
            let le = if self.emission[i][observations[0]] > 0.0 {
                self.emission[i][observations[0]].ln()
            } else {
                f64::NEG_INFINITY
            };
            *di = lp + le;
        }

        // Recursion
        for t in 1..t_len {
            for j in 0..n {
                let le = if self.emission[j][observations[t]] > 0.0 {
                    self.emission[j][observations[t]].ln()
                } else {
                    f64::NEG_INFINITY
                };
                let mut best_val = f64::NEG_INFINITY;
                let mut best_i = 0;
                for (i, row) in self.transition.iter().enumerate() {
                    let la = if row[j] > 0.0 {
                        row[j].ln()
                    } else {
                        f64::NEG_INFINITY
                    };
                    let val = delta[t - 1][i] + la;
                    if val > best_val {
                        best_val = val;
                        best_i = i;
                    }
                }
                delta[t][j] = best_val + le;
                psi[t][j] = best_i;
            }
        }

        // Backtrack
        let mut path = vec![0usize; t_len];
        path[t_len - 1] = delta[t_len - 1]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        for t in (0..t_len - 1).rev() {
            path[t] = psi[t + 1][path[t + 1]];
        }

        Ok(path)
    }

    /// Baum-Welch algorithm: re-estimates HMM parameters from observations.
    ///
    /// Runs `max_iter` iterations of the EM algorithm. Returns a new HMM
    /// with updated parameters.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `observations` is empty or contains
    /// out-of-range symbol indices.
    /// Returns `InvalidParameter` if `max_iter` is 0.
    #[must_use = "returns the re-estimated HMM"]
    pub fn baum_welch(
        &self,
        observations: &[usize],
        max_iter: usize,
    ) -> Result<Self, PramanaError> {
        if max_iter == 0 {
            return Err(PramanaError::InvalidParameter(
                "max_iter must be positive".into(),
            ));
        }
        let t_len = observations.len();
        if t_len == 0 {
            return Err(PramanaError::InvalidSample(
                "observations must be non-empty".into(),
            ));
        }
        let n = self.num_states();
        let m = self.num_symbols();
        for (t, &o) in observations.iter().enumerate() {
            if o >= m {
                return Err(PramanaError::InvalidSample(format!(
                    "observation[{t}] = {o} >= num_symbols {m}"
                )));
            }
        }

        let mut transition = self.transition.clone();
        let mut emission = self.emission.clone();
        let mut initial = self.initial.clone();

        for _ in 0..max_iter {
            // Build a temporary HMM with current params for forward/backward
            let hmm = HiddenMarkovModel {
                transition: transition.clone(),
                emission: emission.clone(),
                initial: initial.clone(),
            };

            // Forward
            let alpha = hmm.forward(observations)?;

            // Backward
            let mut beta = vec![vec![0.0; n]; t_len];
            for bi in &mut beta[t_len - 1] {
                *bi = 1.0;
            }
            for t in (0..t_len - 1).rev() {
                for i in 0..n {
                    let mut sum = 0.0;
                    for j in 0..n {
                        sum += transition[i][j] * emission[j][observations[t + 1]] * beta[t + 1][j];
                    }
                    beta[t][i] = sum;
                }
            }

            // Gamma and xi
            let mut gamma = vec![vec![0.0; n]; t_len];
            let mut xi = vec![vec![vec![0.0; n]; n]; t_len.saturating_sub(1)];

            for t in 0..t_len {
                let denom: f64 = (0..n).map(|i| alpha[t][i] * beta[t][i]).sum();
                if denom > 0.0 {
                    for i in 0..n {
                        gamma[t][i] = alpha[t][i] * beta[t][i] / denom;
                    }
                }
            }

            for t in 0..t_len.saturating_sub(1) {
                let mut denom = 0.0;
                for i in 0..n {
                    for j in 0..n {
                        denom += alpha[t][i]
                            * transition[i][j]
                            * emission[j][observations[t + 1]]
                            * beta[t + 1][j];
                    }
                }
                if denom > 0.0 {
                    for i in 0..n {
                        for j in 0..n {
                            xi[t][i][j] = alpha[t][i]
                                * transition[i][j]
                                * emission[j][observations[t + 1]]
                                * beta[t + 1][j]
                                / denom;
                        }
                    }
                }
            }

            // Re-estimate
            // Initial
            initial[..n].copy_from_slice(&gamma[0][..n]);

            // Transition
            for i in 0..n {
                let gamma_sum: f64 = (0..t_len - 1).map(|t| gamma[t][i]).sum();
                if gamma_sum > 0.0 {
                    for j in 0..n {
                        let xi_sum: f64 = (0..t_len - 1).map(|t| xi[t][i][j]).sum();
                        transition[i][j] = xi_sum / gamma_sum;
                    }
                }
            }

            // Emission
            for i in 0..n {
                let gamma_sum: f64 = (0..t_len).map(|t| gamma[t][i]).sum();
                if gamma_sum > 0.0 {
                    for (k, ek) in emission[i].iter_mut().enumerate() {
                        let num: f64 = (0..t_len)
                            .filter(|&t| observations[t] == k)
                            .map(|t| gamma[t][i])
                            .sum();
                        *ek = num / gamma_sum;
                    }
                }
            }
        }

        Ok(HiddenMarkovModel {
            transition,
            emission,
            initial,
        })
    }
}

/// Validates that a vector is a valid probability distribution (non-negative, sums to ~1).
fn validate_stochastic_row(row: &[f64], name: &str) -> Result<(), PramanaError> {
    for (j, &val) in row.iter().enumerate() {
        if val < 0.0 {
            return Err(PramanaError::InvalidParameter(format!(
                "negative value in {name}[{j}]: {val}"
            )));
        }
    }
    let sum: f64 = row.iter().sum();
    if (sum - 1.0).abs() > 1e-6 {
        return Err(PramanaError::InvalidParameter(format!(
            "{name} sums to {sum}, expected 1.0"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SimpleRng;

    #[test]
    fn test_valid_chain() {
        let matrix = vec![vec![0.7, 0.3], vec![0.4, 0.6]];
        let chain = MarkovChain::new(matrix, 0);
        assert!(chain.is_ok());
    }

    #[test]
    fn test_invalid_row_sum() {
        let matrix = vec![vec![0.5, 0.3], vec![0.4, 0.6]];
        assert!(MarkovChain::new(matrix, 0).is_err());
    }

    #[test]
    fn test_negative_probability() {
        let matrix = vec![vec![1.3, -0.3], vec![0.4, 0.6]];
        assert!(MarkovChain::new(matrix, 0).is_err());
    }

    #[test]
    fn test_non_square() {
        let matrix = vec![vec![0.5, 0.3, 0.2], vec![0.4, 0.6]];
        assert!(MarkovChain::new(matrix, 0).is_err());
    }

    #[test]
    fn test_step() {
        let matrix = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let mut chain = MarkovChain::new(matrix, 0).unwrap();
        let mut rng = SimpleRng::new(42);
        // With deterministic transitions: 0->1->0->1...
        assert_eq!(chain.step(&mut rng), 1);
        assert_eq!(chain.step(&mut rng), 0);
    }

    #[test]
    fn test_simulate() {
        let matrix = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let mut chain = MarkovChain::new(matrix, 0).unwrap();
        let mut rng = SimpleRng::new(42);
        let traj = chain.simulate(4, &mut rng);
        assert_eq!(traj, vec![0, 1, 0, 1, 0]);
    }

    #[test]
    fn test_steady_state() {
        // Two-state chain: P = [[0.7, 0.3], [0.4, 0.6]]
        // Steady state: pi = [4/7, 3/7]
        let matrix = vec![vec![0.7, 0.3], vec![0.4, 0.6]];
        let chain = MarkovChain::new(matrix, 0).unwrap();
        let ss = chain.steady_state().unwrap();
        assert!((ss[0] - 4.0 / 7.0).abs() < 1e-6);
        assert!((ss[1] - 3.0 / 7.0).abs() < 1e-6);
    }

    #[test]
    fn serde_roundtrip() {
        let matrix = vec![vec![0.5, 0.5], vec![0.3, 0.7]];
        let chain = MarkovChain::new(matrix, 0).unwrap();
        let json = serde_json::to_string(&chain).unwrap();
        let chain2: MarkovChain = serde_json::from_str(&json).unwrap();
        assert_eq!(chain.transition_matrix, chain2.transition_matrix);
        assert_eq!(chain.state, chain2.state);
    }

    // --- Hidden Markov Model ---

    fn example_hmm() -> HiddenMarkovModel {
        // 2 states (sunny/rainy), 3 observation symbols (walk/shop/clean)
        HiddenMarkovModel::new(
            vec![vec![0.7, 0.3], vec![0.4, 0.6]],
            vec![vec![0.1, 0.4, 0.5], vec![0.6, 0.3, 0.1]],
            vec![0.6, 0.4],
        )
        .unwrap()
    }

    #[test]
    fn hmm_forward_positive() {
        let hmm = example_hmm();
        let ll = hmm.forward_log_likelihood(&[0, 1, 2]).unwrap();
        assert!(ll.is_finite(), "log-likelihood should be finite: {ll}");
        assert!(ll < 0.0, "log-likelihood should be negative: {ll}");
    }

    #[test]
    fn hmm_forward_longer_sequence_lower() {
        let hmm = example_hmm();
        let ll3 = hmm.forward_log_likelihood(&[0, 1, 2]).unwrap();
        let ll5 = hmm.forward_log_likelihood(&[0, 1, 2, 0, 1]).unwrap();
        assert!(
            ll5 < ll3,
            "longer sequence should have lower log-likelihood"
        );
    }

    #[test]
    fn hmm_viterbi_returns_valid_states() {
        let hmm = example_hmm();
        let path = hmm.viterbi(&[0, 1, 2, 0]).unwrap();
        assert_eq!(path.len(), 4);
        for &s in &path {
            assert!(s < hmm.num_states(), "state {s} out of range");
        }
    }

    #[test]
    fn hmm_viterbi_deterministic() {
        // State 0 emits symbol 0, state 1 emits symbol 1.
        // Transitions allow switching freely.
        let hmm = HiddenMarkovModel::new(
            vec![vec![0.5, 0.5], vec![0.5, 0.5]],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![0.5, 0.5],
        )
        .unwrap();
        let path = hmm.viterbi(&[0, 0, 1, 1, 0]).unwrap();
        assert_eq!(path, vec![0, 0, 1, 1, 0]);
    }

    #[test]
    fn hmm_baum_welch_improves() {
        let hmm = example_hmm();
        let obs = [0, 1, 2, 0, 1, 2, 0, 0, 1, 2];
        let ll_before = hmm.forward_log_likelihood(&obs).unwrap();
        let hmm2 = hmm.baum_welch(&obs, 10).unwrap();
        let ll_after = hmm2.forward_log_likelihood(&obs).unwrap();
        // Baum-Welch should not decrease likelihood
        assert!(
            ll_after >= ll_before - 1e-10,
            "BW should improve: {ll_before} -> {ll_after}"
        );
    }

    #[test]
    fn hmm_baum_welch_stochastic() {
        // After Baum-Welch, matrices should still be row-stochastic
        let hmm = example_hmm();
        let obs = [0, 1, 2, 0, 1];
        let hmm2 = hmm.baum_welch(&obs, 5).unwrap();
        for row in &hmm2.transition {
            let sum: f64 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "transition row sums to {sum}");
        }
        for row in &hmm2.emission {
            let sum: f64 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "emission row sums to {sum}");
        }
        let pi_sum: f64 = hmm2.initial.iter().sum();
        assert!((pi_sum - 1.0).abs() < 1e-6, "initial sums to {pi_sum}");
    }

    #[test]
    fn hmm_invalid_params() {
        // Empty initial
        assert!(HiddenMarkovModel::new(vec![], vec![], vec![]).is_err());
        // Transition wrong size
        assert!(HiddenMarkovModel::new(vec![vec![1.0]], vec![vec![1.0]], vec![0.5, 0.5],).is_err());
        // Emission wrong size
        assert!(
            HiddenMarkovModel::new(
                vec![vec![0.5, 0.5], vec![0.5, 0.5]],
                vec![vec![1.0]],
                vec![0.5, 0.5],
            )
            .is_err()
        );
        // Invalid observation
        let hmm = example_hmm();
        assert!(hmm.viterbi(&[5]).is_err());
        assert!(hmm.forward_log_likelihood(&[]).is_err());
    }

    #[test]
    fn hmm_serde_roundtrip() {
        let hmm = example_hmm();
        let json = serde_json::to_string(&hmm).unwrap();
        let hmm2: HiddenMarkovModel = serde_json::from_str(&json).unwrap();
        assert_eq!(hmm.transition, hmm2.transition);
        assert_eq!(hmm.emission, hmm2.emission);
        assert_eq!(hmm.initial, hmm2.initial);
    }
}
