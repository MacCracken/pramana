//! Markov chains and Hidden Markov Models.

use crate::error::PramanaError;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Consecutive non-shrinking residuals after which `steady_state` switches to
/// the lazy (aperiodic) chain.
const STEADY_STATE_STALL_LIMIT: usize = 8;

/// Upper bound on up-front trajectory allocation in [`MarkovChain::simulate`];
/// longer trajectories grow the vector normally.
const MAX_PREALLOCATED_STEPS: usize = 1 << 16;

/// A discrete-time Markov chain with a finite state space.
///
/// Deserialization re-validates the transition matrix and state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "MarkovChainRepr")]
#[non_exhaustive]
pub struct MarkovChain {
    /// Row-stochastic transition matrix. `transition_matrix[i][j]` is P(j | i).
    pub transition_matrix: Vec<Vec<f64>>,
    /// Current state index.
    pub state: usize,
}

#[derive(Deserialize)]
struct MarkovChainRepr {
    transition_matrix: Vec<Vec<f64>>,
    state: usize,
}

impl TryFrom<MarkovChainRepr> for MarkovChain {
    type Error = PramanaError;

    fn try_from(repr: MarkovChainRepr) -> Result<Self, PramanaError> {
        MarkovChain::new(repr.transition_matrix, repr.state)
    }
}

impl MarkovChain {
    /// Creates a new Markov chain.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if:
    /// - The matrix is empty
    /// - The matrix is not square
    /// - Any entry is negative or non-finite
    /// - Any row does not sum to approximately 1.0 (tolerance 1e-6)
    /// - `initial_state` is out of range
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
            validate_stochastic_row(row, &format!("row {i}"))?;
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
    ///
    /// If `state` has been set out of range through the public field, the
    /// chain stays where it is instead of panicking.
    pub fn step(&mut self, rng: &mut impl Rng) -> usize {
        let Some(row) = self.transition_matrix.get(self.state) else {
            return self.state;
        };
        let r = rng.next_f64();
        let mut cumulative = 0.0;
        for (j, &p) in row.iter().enumerate() {
            cumulative += p;
            if r < cumulative {
                self.state = j;
                return j;
            }
        }
        // Floating-point rounding left the row sum just below r: fall back to
        // the last state that is actually reachable.
        if let Some(j) = row.iter().rposition(|&p| p > 0.0) {
            self.state = j;
        }
        self.state
    }

    /// Simulates the chain for `steps` transitions, returning the sequence of states visited.
    pub fn simulate(&mut self, steps: usize, rng: &mut impl Rng) -> Vec<usize> {
        let mut trajectory =
            Vec::with_capacity(steps.saturating_add(1).min(MAX_PREALLOCATED_STEPS));
        trajectory.push(self.state);
        for _ in 0..steps {
            self.step(rng);
            trajectory.push(self.state);
        }
        trajectory
    }

    /// Computes the steady-state (stationary) distribution by power iteration.
    ///
    /// Plain power iteration oscillates forever on periodic chains (e.g.
    /// bipartite random walks). If the stationarity residual `‖πP − π‖₁`
    /// stops shrinking, the iteration switches to the "lazy" chain
    /// `(I + P) / 2`, which has the same stationary distribution but is
    /// aperiodic. Convergence is declared when the residual drops below 1e-10.
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
        let mut next = vec![0.0; n];
        let mut lazy = false;
        let mut prev_residual = f64::INFINITY;
        let mut stalled = 0;

        for _ in 0..max_iter {
            // next[j] = sum_i pi[i] * P[i][j]
            next.iter_mut().for_each(|v| *v = 0.0);
            for (&pi_i, row) in pi.iter().zip(&self.transition_matrix) {
                for (next_j, &p) in next.iter_mut().zip(row) {
                    *next_j += pi_i * p;
                }
            }

            let residual: f64 = pi.iter().zip(&next).map(|(&a, &b)| (a - b).abs()).sum();
            if residual < tol {
                let total: f64 = next.iter().sum();
                if total > 0.0 {
                    next.iter_mut().for_each(|v| *v /= total);
                }
                return Ok(next);
            }

            if !lazy {
                // A residual that no longer shrinks means oscillation.
                stalled = if residual >= 0.999 * prev_residual {
                    stalled + 1
                } else {
                    0
                };
                lazy = stalled >= STEADY_STATE_STALL_LIMIT;
                prev_residual = residual;
            }

            if lazy {
                // Lazy step: π ← (π + πP) / 2
                for (p, &q) in pi.iter_mut().zip(&next) {
                    *p = 0.5 * (*p + q);
                }
            } else {
                std::mem::swap(&mut pi, &mut next);
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
///
/// The forward/backward recursions are scaled per time step (Rabiner 1989),
/// so likelihoods and Baum-Welch updates stay accurate for long sequences.
/// Deserialization re-validates all parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "HiddenMarkovModelRepr")]
#[non_exhaustive]
pub struct HiddenMarkovModel {
    /// Transition matrix A (N × N).
    pub transition: Vec<Vec<f64>>,
    /// Emission matrix B (N × M).
    pub emission: Vec<Vec<f64>>,
    /// Initial state distribution π (length N).
    pub initial: Vec<f64>,
}

#[derive(Deserialize)]
struct HiddenMarkovModelRepr {
    transition: Vec<Vec<f64>>,
    emission: Vec<Vec<f64>>,
    initial: Vec<f64>,
}

impl TryFrom<HiddenMarkovModelRepr> for HiddenMarkovModel {
    type Error = PramanaError;

    fn try_from(repr: HiddenMarkovModelRepr) -> Result<Self, PramanaError> {
        HiddenMarkovModel::new(repr.transition, repr.emission, repr.initial)
    }
}

/// Scaled forward variables: `alpha[t]` is normalized to sum to 1 and
/// `scales[t]` is the normalizer, so `ln P(O) = Σ ln scales[t]`.
struct ScaledForward {
    alpha: Vec<Vec<f64>>,
    scales: Vec<f64>,
}

impl HiddenMarkovModel {
    /// Creates a new HMM.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if matrices are empty, not row-stochastic,
    /// or contain negative or non-finite values.
    /// Returns `DimensionMismatch` if the shapes are inconsistent.
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

    /// Number of observation symbols (0 for a model with no emission rows).
    #[must_use]
    #[inline]
    pub fn num_symbols(&self) -> usize {
        self.emission.first().map_or(0, Vec::len)
    }

    /// Checks that the (publicly mutable) matrices have consistent shapes and
    /// that `observations` is non-empty with in-range symbols, so the
    /// algorithms below can index without panicking.
    fn check_inputs(&self, observations: &[usize]) -> Result<(), PramanaError> {
        let n = self.num_states();
        let m = self.num_symbols();
        if n == 0
            || m == 0
            || self.transition.len() != n
            || self.transition.iter().any(|row| row.len() != n)
            || self.emission.len() != n
            || self.emission.iter().any(|row| row.len() != m)
        {
            return Err(PramanaError::DimensionMismatch(
                "HMM matrices have inconsistent shapes".into(),
            ));
        }
        if observations.is_empty() {
            return Err(PramanaError::InvalidSample(
                "observations must be non-empty".into(),
            ));
        }
        if let Some((t, &o)) = observations.iter().enumerate().find(|&(_, &o)| o >= m) {
            return Err(PramanaError::InvalidSample(format!(
                "observation[{t}] = {o} >= num_symbols {m}"
            )));
        }
        Ok(())
    }

    /// Forward algorithm: computes ln P(observations | model).
    ///
    /// Uses per-step scaling, so the result is accurate for arbitrarily long
    /// sequences. Returns `-inf` if the sequence has zero probability.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `observations` is empty or contains
    /// out-of-range symbol indices.
    /// Returns `DimensionMismatch` if the model's matrices are inconsistent.
    #[must_use = "returns the log-likelihood"]
    pub fn forward_log_likelihood(&self, observations: &[usize]) -> Result<f64, PramanaError> {
        self.check_inputs(observations)?;
        let fwd = self.forward_scaled(observations);
        Ok(log_likelihood(&fwd.scales))
    }

    /// Scaled forward pass. Inputs must already be validated by `check_inputs`.
    fn forward_scaled(&self, observations: &[usize]) -> ScaledForward {
        let n = self.num_states();
        let t_len = observations.len();
        let mut alpha = vec![vec![0.0; n]; t_len];
        let mut scales = vec![0.0; t_len];

        for (t, &o) in observations.iter().enumerate() {
            let (done, rest) = alpha.split_at_mut(t);
            let current = &mut rest[0];
            if let Some(prev) = done.last() {
                // Induction: alpha_t(j) = [Σ_i alpha_{t-1}(i) a_ij] b_j(o_t)
                for (j, cur_j) in current.iter_mut().enumerate() {
                    let sum: f64 = prev
                        .iter()
                        .zip(&self.transition)
                        .map(|(&a_prev, row)| a_prev * row[j])
                        .sum();
                    *cur_j = sum * self.emission[j][o];
                }
            } else {
                // Initialization: alpha_0(i) = π_i b_i(o_0)
                for (i, cur_i) in current.iter_mut().enumerate() {
                    *cur_i = self.initial[i] * self.emission[i][o];
                }
            }
            let scale: f64 = current.iter().sum();
            scales[t] = scale;
            if scale > 0.0 {
                let inv = 1.0 / scale;
                current.iter_mut().for_each(|v| *v *= inv);
            }
        }

        ScaledForward { alpha, scales }
    }

    /// Viterbi algorithm: finds the most likely hidden state sequence.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `observations` is empty or contains
    /// out-of-range symbol indices.
    /// Returns `DimensionMismatch` if the model's matrices are inconsistent.
    #[must_use = "returns the most likely state sequence"]
    pub fn viterbi(&self, observations: &[usize]) -> Result<Vec<usize>, PramanaError> {
        self.check_inputs(observations)?;
        let t_len = observations.len();
        let n = self.num_states();

        let safe_ln = |p: f64| if p > 0.0 { p.ln() } else { f64::NEG_INFINITY };
        let log_transition: Vec<Vec<f64>> = self
            .transition
            .iter()
            .map(|row| row.iter().map(|&p| safe_ln(p)).collect())
            .collect();

        // Work in log-space to avoid underflow
        let mut delta = vec![vec![f64::NEG_INFINITY; n]; t_len];
        let mut psi = vec![vec![0usize; n]; t_len];

        // Initialization
        for (i, di) in delta[0].iter_mut().enumerate() {
            *di = safe_ln(self.initial[i]) + safe_ln(self.emission[i][observations[0]]);
        }

        // Recursion
        for t in 1..t_len {
            let (done, rest) = delta.split_at_mut(t);
            let prev = &done[t - 1];
            for (j, dj) in rest[0].iter_mut().enumerate() {
                let le = safe_ln(self.emission[j][observations[t]]);
                let mut best_val = f64::NEG_INFINITY;
                let mut best_i = 0;
                for (i, (&d_prev, la_row)) in prev.iter().zip(&log_transition).enumerate() {
                    let val = d_prev + la_row[j];
                    if val > best_val {
                        best_val = val;
                        best_i = i;
                    }
                }
                *dj = best_val + le;
                psi[t][j] = best_i;
            }
        }

        // Backtrack
        let mut path = vec![0usize; t_len];
        path[t_len - 1] = delta[t_len - 1]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map_or(0, |(i, _)| i);

        for t in (0..t_len - 1).rev() {
            path[t] = psi[t + 1][path[t + 1]];
        }

        Ok(path)
    }

    /// Baum-Welch algorithm: re-estimates HMM parameters from observations.
    ///
    /// Runs `max_iter` iterations of the EM algorithm using scaled
    /// forward/backward variables (no underflow on long sequences). Returns a
    /// new HMM with updated parameters. States that are never visited keep
    /// their previous transition/emission rows.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `observations` is empty or contains
    /// out-of-range symbol indices.
    /// Returns `InvalidParameter` if `max_iter` is 0.
    /// Returns `DimensionMismatch` if the model's matrices are inconsistent.
    /// Returns `ComputationError` if the observation sequence has zero
    /// probability under the model.
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
        self.check_inputs(observations)?;
        let t_len = observations.len();
        let n = self.num_states();
        let m = self.num_symbols();

        let mut hmm = self.clone();

        for _ in 0..max_iter {
            // Scaled forward
            let ScaledForward { alpha, scales } = hmm.forward_scaled(observations);
            if scales.iter().any(|&s| s <= 0.0 || !s.is_finite()) {
                return Err(PramanaError::ComputationError(
                    "observation sequence has zero probability under the model".into(),
                ));
            }

            // Scaled backward: beta_hat_t(i) = Σ_j a_ij b_j(o_{t+1}) beta_hat_{t+1}(j) / c_{t+1}
            let mut beta = vec![vec![1.0; n]; t_len];
            for t in (0..t_len - 1).rev() {
                let (head, tail) = beta.split_at_mut(t + 1);
                let next = &tail[0];
                let o_next = observations[t + 1];
                let scale_next = scales[t + 1];
                for (i, bi) in head[t].iter_mut().enumerate() {
                    let sum: f64 = hmm.transition[i]
                        .iter()
                        .zip(&hmm.emission)
                        .zip(next)
                        .map(|((&a_ij, e_j), &b_next)| a_ij * e_j[o_next] * b_next)
                        .sum();
                    *bi = sum / scale_next;
                }
            }

            // Accumulate expected counts.
            let mut gamma_first = vec![0.0; n];
            let mut gamma_sum_head = vec![0.0; n]; // Σ_{t<T-1} γ_t(i)
            let mut gamma_sum_all = vec![0.0; n]; // Σ_t γ_t(i)
            let mut xi_sum = vec![vec![0.0; n]; n];
            let mut emission_num = vec![vec![0.0; m]; n];

            for t in 0..t_len {
                let norm: f64 = alpha[t].iter().zip(&beta[t]).map(|(a, b)| a * b).sum();
                if norm > 0.0 {
                    for i in 0..n {
                        let g = alpha[t][i] * beta[t][i] / norm;
                        if t == 0 {
                            gamma_first[i] = g;
                        }
                        if t + 1 < t_len {
                            gamma_sum_head[i] += g;
                        }
                        gamma_sum_all[i] += g;
                        emission_num[i][observations[t]] += g;
                    }
                }

                if t + 1 < t_len {
                    let o_next = observations[t + 1];
                    // ξ_t(i, j) ∝ α̂_t(i) a_ij b_j(o_{t+1}) β̂_{t+1}(j); normalize per t.
                    let xi = |i: usize, j: usize| {
                        alpha[t][i]
                            * hmm.transition[i][j]
                            * hmm.emission[j][o_next]
                            * beta[t + 1][j]
                    };
                    let denom: f64 = (0..n)
                        .flat_map(|i| (0..n).map(move |j| (i, j)))
                        .map(|(i, j)| xi(i, j))
                        .sum();
                    if denom > 0.0 {
                        for (i, acc_row) in xi_sum.iter_mut().enumerate() {
                            for (j, acc) in acc_row.iter_mut().enumerate() {
                                *acc += xi(i, j) / denom;
                            }
                        }
                    }
                }
            }

            // Re-estimate
            hmm.initial.copy_from_slice(&gamma_first);
            for i in 0..n {
                if gamma_sum_head[i] > 0.0 {
                    for j in 0..n {
                        hmm.transition[i][j] = xi_sum[i][j] / gamma_sum_head[i];
                    }
                }
                if gamma_sum_all[i] > 0.0 {
                    for k in 0..m {
                        hmm.emission[i][k] = emission_num[i][k] / gamma_sum_all[i];
                    }
                }
            }
        }

        Ok(hmm)
    }
}

/// `Σ ln scales[t]`, or `-inf` if any step had zero probability.
fn log_likelihood(scales: &[f64]) -> f64 {
    if scales.iter().any(|&s| s <= 0.0) {
        return f64::NEG_INFINITY;
    }
    scales.iter().map(|s| s.ln()).sum()
}

/// Validates that a vector is a valid probability distribution
/// (finite, non-negative, sums to ~1).
fn validate_stochastic_row(row: &[f64], name: &str) -> Result<(), PramanaError> {
    for (j, &val) in row.iter().enumerate() {
        if !val.is_finite() {
            return Err(PramanaError::InvalidParameter(format!(
                "non-finite value in {name}[{j}]: {val}"
            )));
        }
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

/// Read-only hooks for the Cyrius port's golden-value generator (see `crate::oracle`).
#[cfg(feature = "oracle")]
pub(crate) mod oracle;

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

    // --- Hardening regression tests -------------------------------------

    /// Brute-force P(O) by summing over all hidden paths.
    fn brute_force_likelihood(hmm: &HiddenMarkovModel, obs: &[usize]) -> f64 {
        let n = hmm.num_states();
        let mut total = 0.0;
        let paths = n.pow(obs.len() as u32);
        for code in 0..paths {
            let mut c = code;
            let mut prev = 0;
            let mut p = 1.0;
            for (t, &o) in obs.iter().enumerate() {
                let s = c % n;
                c /= n;
                p *= if t == 0 {
                    hmm.initial[s]
                } else {
                    hmm.transition[prev][s]
                };
                p *= hmm.emission[s][o];
                prev = s;
            }
            total += p;
        }
        total
    }

    #[test]
    fn scaled_forward_matches_brute_force() {
        let hmm = example_hmm();
        let obs = [0, 1, 1, 0, 1];
        let ll = hmm.forward_log_likelihood(&obs).unwrap();
        let exact = brute_force_likelihood(&hmm, &obs).ln();
        assert!((ll - exact).abs() < 1e-12, "{ll} vs {exact}");
    }

    #[test]
    fn long_sequence_log_likelihood_is_finite() {
        // The unscaled forward pass underflowed to -inf after a few hundred steps.
        let hmm = example_hmm();
        let m = hmm.num_symbols();
        let obs: Vec<usize> = (0..5000).map(|i| (i * 7 + i / 3) % m).collect();
        let ll = hmm.forward_log_likelihood(&obs).unwrap();
        assert!(ll.is_finite() && ll < 0.0, "ll = {ll}");
        // Additivity sanity: doubling the sequence roughly doubles |ll|.
        let ll_half = hmm.forward_log_likelihood(&obs[..2500]).unwrap();
        assert!((ll / ll_half - 2.0).abs() < 0.05);
    }

    #[test]
    fn baum_welch_long_sequence_stays_valid_and_improves() {
        // Previously the unscaled recursions underflowed and returned initial = [0, 0].
        let hmm = example_hmm();
        let m = hmm.num_symbols();
        let obs: Vec<usize> = (0..3000).map(|i| (i * 7 + i / 3) % m).collect();
        let ll_before = hmm.forward_log_likelihood(&obs).unwrap();
        let trained = hmm.baum_welch(&obs, 5).unwrap();
        let sum: f64 = trained.initial.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "initial sums to {sum}");
        for row in trained.transition.iter().chain(&trained.emission) {
            assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-9);
            assert!(row.iter().all(|v| v.is_finite() && *v >= 0.0));
        }
        let ll_after = trained.forward_log_likelihood(&obs).unwrap();
        assert!(
            ll_after >= ll_before - 1e-6,
            "EM decreased likelihood: {ll_before} -> {ll_after}"
        );
    }

    #[test]
    fn baum_welch_impossible_sequence_errors() {
        // State 0 only emits symbol 0, state 1 only symbol 1; start in state 0
        // and never leave, so observing symbol 1 is impossible.
        let hmm = HiddenMarkovModel::new(
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![1.0, 0.0],
        )
        .unwrap();
        assert_eq!(
            hmm.forward_log_likelihood(&[0, 1]).unwrap(),
            f64::NEG_INFINITY
        );
        assert!(matches!(
            hmm.baum_welch(&[0, 1], 3),
            Err(PramanaError::ComputationError(_))
        ));
    }

    #[test]
    fn hmm_inconsistent_public_fields_do_not_panic() {
        let mut hmm = example_hmm();
        hmm.transition.pop();
        assert!(hmm.forward_log_likelihood(&[0]).is_err());
        assert!(hmm.viterbi(&[0]).is_err());
        assert!(hmm.baum_welch(&[0], 1).is_err());
        let mut hmm = example_hmm();
        hmm.emission.clear();
        assert_eq!(hmm.num_symbols(), 0);
        assert!(hmm.viterbi(&[0]).is_err());
    }

    #[test]
    fn hmm_rejects_non_finite_and_invalid_deserialization() {
        assert!(
            HiddenMarkovModel::new(
                vec![vec![f64::NAN, 1.0], vec![0.5, 0.5]],
                vec![vec![1.0], vec![1.0]],
                vec![0.5, 0.5],
            )
            .is_err()
        );
        let json =
            r#"{"transition":[[0.5,0.5],[0.5,0.5]],"emission":[[1.0],[1.0]],"initial":[0.9,0.9]}"#;
        assert!(serde_json::from_str::<HiddenMarkovModel>(json).is_err());
    }

    #[test]
    fn markov_chain_rejects_nan_and_invalid_deserialization() {
        assert!(MarkovChain::new(vec![vec![f64::NAN, 1.0], vec![0.5, 0.5]], 0).is_err());
        // state out of range
        let json = r#"{"transition_matrix":[[1.0]],"state":5}"#;
        assert!(serde_json::from_str::<MarkovChain>(json).is_err());
        // empty matrix
        let json = r#"{"transition_matrix":[],"state":0}"#;
        assert!(serde_json::from_str::<MarkovChain>(json).is_err());
    }

    #[test]
    fn markov_step_with_out_of_range_state_does_not_panic() {
        let mut mc = MarkovChain::new(vec![vec![0.5, 0.5], vec![0.5, 0.5]], 0).unwrap();
        mc.state = 99;
        let mut rng = SimpleRng::new(1);
        assert_eq!(mc.step(&mut rng), 99);
        assert_eq!(mc.simulate(3, &mut rng), vec![99, 99, 99, 99]);
    }

    #[test]
    fn markov_step_rounding_fallback_skips_unreachable_states() {
        // Row sums to 1 - 1e-9 (within tolerance); r can exceed the cumulative
        // sum, and the fallback must not land on the zero-probability state.
        let mut mc = MarkovChain::new(
            vec![
                vec![0.0, 1.0 - 1e-9, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![1.0, 0.0, 0.0],
            ],
            0,
        )
        .unwrap();
        struct AlmostOne;
        impl Rng for AlmostOne {
            fn next_f64(&mut self) -> f64 {
                1.0 - 1e-12
            }
        }
        assert_eq!(mc.step(&mut AlmostOne), 1);
    }

    #[test]
    fn steady_state_periodic_chain_converges() {
        // Bipartite walk: plain power iteration oscillates forever from uniform.
        let mc = MarkovChain::new(
            vec![
                vec![0.0, 1.0, 0.0],
                vec![0.5, 0.0, 0.5],
                vec![0.0, 1.0, 0.0],
            ],
            0,
        )
        .unwrap();
        let pi = mc.steady_state().unwrap();
        for (got, want) in pi.iter().zip([0.25, 0.5, 0.25]) {
            assert!((got - want).abs() < 1e-9, "{pi:?}");
        }
        // Two-cycle
        let mc = MarkovChain::new(vec![vec![0.0, 1.0], vec![1.0, 0.0]], 0).unwrap();
        let pi = mc.steady_state().unwrap();
        assert!((pi[0] - 0.5).abs() < 1e-9);
    }
}
