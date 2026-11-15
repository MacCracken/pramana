//! Error types for pramana.

use serde::{Deserialize, Serialize};

/// Errors that can occur in pramana operations.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[non_exhaustive]
pub enum PramanaError {
    /// A parameter was invalid (e.g., negative standard deviation).
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// The input sample was invalid (e.g., empty slice).
    #[error("invalid sample: {0}")]
    InvalidSample(String),

    /// An iterative algorithm failed to converge.
    #[error("convergence failure: {0}")]
    ConvergenceFailure(String),

    /// Dimension mismatch between inputs.
    #[error("dimension mismatch: {0}")]
    DimensionMismatch(String),

    /// A computation produced an invalid result (e.g., NaN, overflow).
    #[error("computation error: {0}")]
    ComputationError(String),
}
