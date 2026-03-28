//! Linear and polynomial regression.

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

// ---------------------------------------------------------------------------
// Polynomial regression
// ---------------------------------------------------------------------------

/// A fitted polynomial model: y = a₀ + a₁x + a₂x² + ...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolynomialModel {
    /// Polynomial coefficients `[a₀, a₁, a₂, ...]` where y = a₀ + a₁x + a₂x² + ...
    pub coefficients: Vec<f64>,
    /// Degree of the polynomial.
    pub degree: usize,
    /// Coefficient of determination (R-squared).
    pub r_squared: f64,
}

/// Fits a polynomial of the given `degree` to the data using least squares (QR decomposition).
///
/// Returns a `PolynomialModel` with `degree + 1` coefficients.
///
/// # Errors
///
/// Returns `DimensionMismatch` if `x` and `y` have different lengths.
/// Returns `InvalidParameter` if `degree` is 0 (use `linear_regression` instead) or
/// greater than `x.len() - 1`.
/// Returns `InvalidSample` if fewer than `degree + 1` data points.
#[must_use = "returns the fitted model"]
pub fn polynomial_regression(
    x: &[f64],
    y: &[f64],
    degree: usize,
) -> Result<PolynomialModel, PramanaError> {
    if x.len() != y.len() {
        return Err(PramanaError::DimensionMismatch(
            "x and y must have the same length".into(),
        ));
    }
    if degree == 0 {
        return Err(PramanaError::InvalidParameter(
            "degree must be at least 1".into(),
        ));
    }
    let n = degree + 1;
    if x.len() < n {
        return Err(PramanaError::InvalidSample(format!(
            "need at least {} data points for degree {degree}",
            n
        )));
    }

    let coefficients = hisab::num::least_squares_poly(x, y, degree)
        .map_err(|e| PramanaError::ComputationError(format!("polynomial fit failed: {e}")))?;

    // R-squared
    let mean_y: f64 = y.iter().sum::<f64>() / y.len() as f64;
    let ss_tot: f64 = y.iter().map(|&yi| (yi - mean_y).powi(2)).sum();
    let ss_res: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| {
            let pred = eval_polynomial(&coefficients, xi);
            (yi - pred).powi(2)
        })
        .sum();

    let r_squared = if ss_tot.abs() < 1e-30 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Ok(PolynomialModel {
        coefficients,
        degree,
        r_squared,
    })
}

/// Predicts y for a given x using the fitted polynomial model.
#[must_use]
#[inline]
pub fn predict_poly(model: &PolynomialModel, x: f64) -> f64 {
    eval_polynomial(&model.coefficients, x)
}

/// Computes the residuals for a polynomial fit.
///
/// # Errors
///
/// Returns `DimensionMismatch` if `x` and `y` have different lengths.
#[must_use = "returns the residual vector"]
pub fn residuals_poly(
    model: &PolynomialModel,
    x: &[f64],
    y: &[f64],
) -> Result<Vec<f64>, PramanaError> {
    if x.len() != y.len() {
        return Err(PramanaError::DimensionMismatch(
            "x and y must have the same length".into(),
        ));
    }
    Ok(x.iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| yi - predict_poly(model, xi))
        .collect())
}

/// Evaluates a polynomial with coefficients `[a₀, a₁, ...]` at `x` using Horner's method.
#[must_use]
#[inline]
fn eval_polynomial(coeffs: &[f64], x: f64) -> f64 {
    // Horner's method: a₀ + x*(a₁ + x*(a₂ + ...))
    let mut result = 0.0;
    for &c in coeffs.iter().rev() {
        result = result * x + c;
    }
    result
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

    // --- Polynomial regression ---

    #[test]
    fn poly_recovers_quadratic() {
        // y = 1 + 2x + 3x²
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + 2.0 * xi + 3.0 * xi * xi).collect();
        let model = polynomial_regression(&x, &y, 2).unwrap();
        assert_eq!(model.degree, 2);
        assert_eq!(model.coefficients.len(), 3);
        assert!(
            (model.coefficients[0] - 1.0).abs() < 1e-6,
            "a0 = {}",
            model.coefficients[0]
        );
        assert!(
            (model.coefficients[1] - 2.0).abs() < 1e-6,
            "a1 = {}",
            model.coefficients[1]
        );
        assert!(
            (model.coefficients[2] - 3.0).abs() < 1e-6,
            "a2 = {}",
            model.coefficients[2]
        );
        assert!((model.r_squared - 1.0).abs() < 1e-6);
    }

    #[test]
    fn poly_predict() {
        // y = 1 + 2x + 3x²
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + 2.0 * xi + 3.0 * xi * xi).collect();
        let model = polynomial_regression(&x, &y, 2).unwrap();
        let pred = predict_poly(&model, 5.0);
        let expected = 1.0 + 10.0 + 75.0;
        assert!((pred - expected).abs() < 1e-4, "pred = {pred}");
    }

    #[test]
    fn poly_residuals() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y: Vec<f64> = x.iter().map(|&xi| xi * xi).collect();
        let model = polynomial_regression(&x, &y, 2).unwrap();
        let r = residuals_poly(&model, &x, &y).unwrap();
        for (i, &ri) in r.iter().enumerate() {
            assert!(ri.abs() < 1e-6, "residual[{i}] = {ri}");
        }
    }

    #[test]
    fn poly_cubic() {
        // y = x³
        let x: Vec<f64> = (-5..=5).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi.powi(3)).collect();
        let model = polynomial_regression(&x, &y, 3).unwrap();
        assert!((model.r_squared - 1.0).abs() < 1e-6);
        // Leading coefficient should be ~1, others ~0
        assert!(
            (model.coefficients[3] - 1.0).abs() < 1e-4,
            "a3 = {}",
            model.coefficients[3]
        );
    }

    #[test]
    fn poly_invalid_params() {
        let x = [1.0, 2.0, 3.0];
        let y = [1.0, 4.0, 9.0];
        // degree 0
        assert!(polynomial_regression(&x, &y, 0).is_err());
        // not enough points for degree
        assert!(polynomial_regression(&x, &y, 3).is_err());
        // dimension mismatch
        assert!(polynomial_regression(&x, &[1.0, 2.0], 2).is_err());
    }

    #[test]
    fn poly_serde_roundtrip() {
        let model = PolynomialModel {
            coefficients: vec![1.0, 2.0, 3.0],
            degree: 2,
            r_squared: 0.99,
        };
        let json = serde_json::to_string(&model).unwrap();
        let model2: PolynomialModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model.coefficients, model2.coefficients);
        assert_eq!(model.degree, model2.degree);
        assert_eq!(model.r_squared, model2.r_squared);
    }
}
