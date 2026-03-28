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
//! | [`distribution`] | Probability distributions (Normal, Uniform, Exponential, Poisson, Binomial, Bernoulli) |
//! | [`descriptive`] | Descriptive statistics (mean, median, mode, variance, skewness, kurtosis, percentiles) |
//! | [`hypothesis`] | Hypothesis testing (t-tests, chi-squared) |
//! | [`regression`] | Linear regression with R-squared |
//! | [`bayesian`] | Bayesian inference and naive Bayes classification |
//! | [`combinatorics`] | Factorials, permutations, combinations, Stirling approximation |
//! | [`monte_carlo`] | Monte Carlo integration and simulation |
//! | [`markov`] | Markov chains with steady-state analysis |
//! | [`timeseries`] | Moving average, exponential smoothing, autocorrelation |

pub mod bayesian;
pub mod combinatorics;
pub mod descriptive;
pub mod distribution;
pub mod error;
pub mod hypothesis;
pub mod markov;
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
