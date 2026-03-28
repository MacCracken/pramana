//! # Pramana
//!
//! प्रमाण (Sanskrit: proof, measure, evidence) — Statistics and probability library
//! for the AGNOS ecosystem.
//!
//! Built on [hisab](https://docs.rs/hisab) for mathematical primitives.
//!
//! ## Modules
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`distribution`] | Probability distributions (Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli, Gamma, Beta, Chi-Squared, Student-t, F, Cauchy, Weibull, Multivariate Normal) |
//! | [`descriptive`] | Descriptive statistics, kernel density estimation |
//! | [`hypothesis`] | Hypothesis testing (t-tests, chi-squared) and confidence intervals |
//! | [`regression`] | Linear, polynomial, and logistic regression |
//! | [`bayesian`] | Bayesian inference and naive Bayes classification |
//! | [`combinatorics`] | Factorials, permutations, combinations, Stirling approximation |
//! | [`monte_carlo`] | Monte Carlo integration, Metropolis-Hastings MCMC, Gibbs sampling |
//! | [`markov`] | Markov chains, Hidden Markov Models (Forward, Viterbi, Baum-Welch) |
//! | [`timeseries`] | Moving average, exponential smoothing, autocorrelation |
//!
//! ## Quick Start
//!
//! ```
//! use pramana::{descriptive, distribution::{Normal, Distribution}, SimpleRng};
//!
//! // Descriptive statistics
//! let data = [1.0, 2.0, 3.0, 4.0, 5.0];
//! let m = descriptive::mean(&data).unwrap();
//! assert!((m - 3.0).abs() < 1e-10);
//!
//! // Fit and sample from a normal distribution
//! let s = descriptive::std_dev(&data).unwrap();
//! let normal = Normal::new(m, s).unwrap();
//! let mut rng = SimpleRng::new(42);
//! let sample = normal.sample(&mut rng);
//! assert!(sample.is_finite());
//! ```
//!
//! ## Hypothesis Testing
//!
//! ```
//! use pramana::hypothesis;
//!
//! let data = [10.0, 10.1, 9.9, 10.2, 9.8, 10.0, 10.1, 9.9];
//! let result = hypothesis::t_test_one_sample(&data, 0.0, 0.05).unwrap();
//! assert!(result.reject); // mean clearly differs from 0
//! ```
//!
//! ## Monte Carlo
//!
//! ```
//! use pramana::{monte_carlo, SimpleRng};
//!
//! let mut rng = SimpleRng::new(42);
//! let pi = monte_carlo::monte_carlo_pi(100_000, &mut rng).unwrap();
//! assert!((pi - std::f64::consts::PI).abs() < 0.1);
//! ```

pub mod bayesian;
pub mod combinatorics;
pub mod descriptive;
pub mod distribution;
pub mod error;
pub mod hypothesis;
pub mod markov;
pub(crate) mod math;
pub mod monte_carlo;
pub mod regression;
pub mod rng;
pub mod timeseries;

pub use error::PramanaError;
pub use rng::{Rng, SimpleRng};

/// Convenience alias for `Result<T, PramanaError>`.
pub type Result<T> = std::result::Result<T, PramanaError>;

/// Re-export hisab for consumers that need mathematical primitives.
pub use hisab;
