//! Linear regression.

use crate::error::PramanaError;
use serde::{Deserialize, Serialize};

/// A fitted linear model: y = slope * x + intercept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearModel {
    /// Slope of the regression line.
    pub slope: f64,
    /// Y-intercept of the regression line.
    pub intercept: f64,
    /// Coefficient of determination (R-squared).
    pub r_squared: f64,
}

/// Fits a simple linear regression y = slope * x + intercept using ordinary least squares.
///
/// # Errors
///
/// Returns `DimensionMismatch` if `x` and `y` have different lengths.
/// Returns `InvalidSample` if fewer than 2 data points or zero variance in `x`.
#[must_use = "returns the fitted model"]
pub fn linear_regression(x: &[f64], y: &[f64]) -> Result<LinearModel, PramanaError> {
    if x.len() != y.len() {
        return Err(PramanaError::DimensionMismatch(
            "x and y must have the same length".into(),
        ));
    }
    if x.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 data points".into(),
        ));
    }
    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(&xi, &yi)| xi * yi).sum();
    let sum_x2: f64 = x.iter().map(|&xi| xi * xi).sum();

    let mean_x = sum_x / n;
    let mean_y = sum_y / n;

    let denom = sum_x2 - sum_x * sum_x / n;
    if denom.abs() < 1e-30 {
        return Err(PramanaError::InvalidSample(
            "zero variance in x (all x values are equal)".into(),
        ));
    }

    let slope = (sum_xy - sum_x * sum_y / n) / denom;
    let intercept = mean_y - slope * mean_x;

    // R-squared
    let ss_tot: f64 = y.iter().map(|&yi| (yi - mean_y).powi(2)).sum();
    let ss_res: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| (yi - (slope * xi + intercept)).powi(2))
        .sum();

    let r_squared = if ss_tot.abs() < 1e-30 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Ok(LinearModel {
        slope,
        intercept,
        r_squared,
    })
}

/// Predicts y for a given x using the fitted linear model.
#[must_use]
#[inline]
pub fn predict(model: &LinearModel, x: f64) -> f64 {
    model.slope * x + model.intercept
}

/// Computes the residuals (y_i - predicted_i) for each data point.
///
/// # Errors
///
/// Returns `DimensionMismatch` if `x` and `y` have different lengths.
#[must_use = "returns the residual vector"]
pub fn residuals(model: &LinearModel, x: &[f64], y: &[f64]) -> Result<Vec<f64>, PramanaError> {
    if x.len() != y.len() {
        return Err(PramanaError::DimensionMismatch(
            "x and y must have the same length".into(),
        ));
    }
    Ok(x.iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| yi - predict(model, xi))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_line() {
        // y = 2x + 1
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [3.0, 5.0, 7.0, 9.0, 11.0];
        let model = linear_regression(&x, &y).unwrap();
        assert!((model.slope - 2.0).abs() < 1e-10);
        assert!((model.intercept - 1.0).abs() < 1e-10);
        assert!((model.r_squared - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_predict() {
        let model = LinearModel {
            slope: 2.0,
            intercept: 1.0,
            r_squared: 1.0,
        };
        assert!((predict(&model, 3.0) - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_residuals() {
        let model = LinearModel {
            slope: 1.0,
            intercept: 0.0,
            r_squared: 1.0,
        };
        let x = [1.0, 2.0, 3.0];
        let y = [1.1, 1.9, 3.2];
        let r = residuals(&model, &x, &y).unwrap();
        assert!((r[0] - 0.1).abs() < 1e-10);
        assert!((r[1] - -0.1).abs() < 1e-10);
        assert!((r[2] - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_dimension_mismatch() {
        assert!(linear_regression(&[1.0], &[1.0, 2.0]).is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let model = LinearModel {
            slope: 2.5,
            intercept: -1.3,
            r_squared: 0.98,
        };
        let json = serde_json::to_string(&model).unwrap();
        let model2: LinearModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model.slope, model2.slope);
        assert_eq!(model.intercept, model2.intercept);
    }
}
