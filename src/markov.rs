//! Markov chains with transition matrices.

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
}
