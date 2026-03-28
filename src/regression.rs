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

// ---------------------------------------------------------------------------
// Logistic regression
// ---------------------------------------------------------------------------

/// A fitted binary logistic regression model.
///
/// The model predicts P(y = 1 | x) = sigmoid(β₀ + β₁x₁ + β₂x₂ + ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticModel {
    /// Coefficients including intercept: `[β₀, β₁, β₂, ...]`.
    /// β₀ is the intercept; β₁..βₚ correspond to each feature.
    pub coefficients: Vec<f64>,
    /// Number of IRLS iterations performed.
    pub iterations: usize,
    /// Whether the algorithm converged within the iteration limit.
    pub converged: bool,
}

/// Fits a binary logistic regression model via iteratively reweighted least squares (IRLS).
///
/// Each row of `features` is one observation with `p` feature values. The `labels`
/// vector must contain values in {0, 1}.
///
/// The model includes an intercept term automatically. L2 regularization
/// (`l2_reg`) penalizes large coefficients (not applied to the intercept).
/// Use `l2_reg = 0.0` for unregularized logistic regression.
///
/// # Errors
///
/// Returns `DimensionMismatch` if `features` rows don't all have the same length,
/// or if `features.len() != labels.len()`.
/// Returns `InvalidSample` if fewer than 2 data points, or labels are not 0/1.
/// Returns `InvalidParameter` if `max_iter` is 0 or `l2_reg` is negative.
#[must_use = "returns the fitted model"]
pub fn logistic_regression(
    features: &[Vec<f64>],
    labels: &[f64],
    l2_reg: f64,
    max_iter: usize,
) -> Result<LogisticModel, PramanaError> {
    let n = features.len();
    if n != labels.len() {
        return Err(PramanaError::DimensionMismatch(
            "features and labels must have the same length".into(),
        ));
    }
    if n < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 data points".into(),
        ));
    }
    if max_iter == 0 {
        return Err(PramanaError::InvalidParameter(
            "max_iter must be positive".into(),
        ));
    }
    if l2_reg < 0.0 {
        return Err(PramanaError::InvalidParameter(
            "l2_reg must be non-negative".into(),
        ));
    }
    let p = features[0].len();
    for (i, row) in features.iter().enumerate() {
        if row.len() != p {
            return Err(PramanaError::DimensionMismatch(format!(
                "feature row {i} has length {}, expected {p}",
                row.len()
            )));
        }
    }
    for (i, &label) in labels.iter().enumerate() {
        if label != 0.0 && label != 1.0 {
            return Err(PramanaError::InvalidSample(format!(
                "label[{i}] = {label}, expected 0 or 1"
            )));
        }
    }

    // Build design matrix X with intercept column: X[i] = [1, features[i][0], ...]
    let dim = p + 1;
    let x: Vec<Vec<f64>> = features
        .iter()
        .map(|row| {
            let mut xrow = Vec::with_capacity(dim);
            xrow.push(1.0);
            xrow.extend_from_slice(row);
            xrow
        })
        .collect();

    // Initialize coefficients to zero
    let mut beta = vec![0.0; dim];
    let tol = 1e-8;
    let mut converged = false;
    let mut iter = 0;

    for _ in 0..max_iter {
        iter += 1;

        // Compute probabilities p_i = sigmoid(x_i . beta)
        let probs: Vec<f64> = x
            .iter()
            .map(|xi| {
                let z: f64 = xi.iter().zip(&beta).map(|(xij, bj)| xij * bj).sum();
                sigmoid(z)
            })
            .collect();

        // Gradient: g_j = sum_i x_ij * (y_i - p_i) - l2_reg * beta_j
        // (L2 penalty not applied to intercept at j=0)
        let mut gradient = vec![0.0; dim];
        for (i, xi) in x.iter().enumerate() {
            let residual = labels[i] - probs[i];
            for (j, &xij) in xi.iter().enumerate() {
                gradient[j] += xij * residual;
            }
        }
        for j in 1..dim {
            gradient[j] -= l2_reg * beta[j];
        }

        // Negative Hessian: -H_jk = sum_i x_ij * w_i * x_ik + l2_reg * I_{j>0}
        // (positive-definite, suitable for Cholesky)
        let mut neg_hessian = vec![vec![0.0; dim]; dim];
        for (i, xi) in x.iter().enumerate() {
            let w = probs[i] * (1.0 - probs[i]);
            for (j, &xij) in xi.iter().enumerate() {
                for (k, &xik) in xi.iter().enumerate().skip(j) {
                    let val = xij * w * xik;
                    neg_hessian[j][k] += val;
                    if k != j {
                        neg_hessian[k][j] += val;
                    }
                }
            }
        }
        // Add L2 regularization to diagonal (skip intercept) + small epsilon
        let eps = 1e-10;
        neg_hessian[0][0] += eps;
        for (j, row) in neg_hessian.iter_mut().enumerate().skip(1) {
            row[j] += l2_reg + eps;
        }

        // Solve (-H) * delta = gradient via Cholesky
        let delta = match hisab::num::cholesky(&neg_hessian) {
            Ok(l) => match hisab::num::cholesky_solve(&l, &gradient) {
                Ok(d) => d,
                Err(_) => break,
            },
            Err(_) => break,
        };

        // Update beta
        let mut max_change = 0.0_f64;
        for (bj, dj) in beta.iter_mut().zip(&delta) {
            *bj += dj;
            max_change = max_change.max(dj.abs());
        }

        if max_change < tol {
            converged = true;
            break;
        }
    }

    Ok(LogisticModel {
        coefficients: beta,
        iterations: iter,
        converged,
    })
}

/// Predicts the probability P(y = 1) for a single observation.
#[must_use]
pub fn predict_logistic_proba(model: &LogisticModel, features: &[f64]) -> f64 {
    // β₀ + β₁x₁ + β₂x₂ + ...
    let z = model.coefficients[0]
        + model.coefficients[1..]
            .iter()
            .zip(features)
            .map(|(b, x)| b * x)
            .sum::<f64>();
    sigmoid(z)
}

/// Predicts the class label (0 or 1) at the given threshold.
#[must_use]
pub fn predict_logistic_class(model: &LogisticModel, features: &[f64], threshold: f64) -> u8 {
    if predict_logistic_proba(model, features) >= threshold {
        1
    } else {
        0
    }
}

/// Logistic sigmoid: 1 / (1 + exp(-x)).
#[must_use]
#[inline]
fn sigmoid(x: f64) -> f64 {
    // Numerically stable version
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let ex = x.exp();
        ex / (1.0 + ex)
    }
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

    // --- Logistic regression ---

    #[test]
    fn logistic_1d() {
        // Overlapping classes near the boundary for well-behaved optimization
        let features = vec![
            vec![-3.0],
            vec![-2.5],
            vec![-2.0],
            vec![-1.5],
            vec![-1.0],
            vec![-0.5],
            vec![0.5],
            vec![1.0],
            vec![1.5],
            vec![2.0],
            vec![2.5],
            vec![3.0],
            // Overlapping points near boundary
            vec![0.1],
            vec![-0.1],
        ];
        let labels = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0,
        ];
        let model = logistic_regression(&features, &labels, 1.0, 100).unwrap();
        assert!(model.converged);
        // Positive coefficient means higher x → higher probability
        assert!(
            model.coefficients[1] > 0.0,
            "β₁ should be positive: {}",
            model.coefficients[1]
        );
        // Predict: large positive → class 1, large negative → class 0
        assert_eq!(predict_logistic_class(&model, &[10.0], 0.5), 1);
        assert_eq!(predict_logistic_class(&model, &[-10.0], 0.5), 0);
    }

    #[test]
    fn logistic_proba_range() {
        let features: Vec<Vec<f64>> = (-5..=5).map(|i| vec![i as f64]).collect();
        let labels: Vec<f64> = (-5..=5).map(|i| if i >= 0 { 1.0 } else { 0.0 }).collect();
        let model = logistic_regression(&features, &labels, 1.0, 100).unwrap();
        // Probabilities must be in [0, 1]
        for i in -20..=20 {
            let p = predict_logistic_proba(&model, &[i as f64]);
            assert!((0.0..=1.0).contains(&p), "proba({i}) = {p} out of range");
        }
    }

    #[test]
    fn logistic_2d_features() {
        // Two features with some overlap near the boundary
        let features = vec![
            vec![1.0, 1.0],
            vec![2.0, 1.0],
            vec![1.0, 2.0],
            vec![-1.0, -1.0],
            vec![-2.0, -1.0],
            vec![-1.0, -2.0],
            vec![3.0, 0.0],
            vec![0.0, 3.0],
            vec![-3.0, 0.0],
            vec![0.0, -3.0],
            // Overlap near boundary
            vec![0.2, -0.2],
            vec![-0.2, 0.2],
        ];
        let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let model = logistic_regression(&features, &labels, 1.0, 100).unwrap();
        assert!(model.converged);
        // Both coefficients should be positive
        assert!(model.coefficients[1] > 0.0);
        assert!(model.coefficients[2] > 0.0);
    }

    #[test]
    fn logistic_invalid_params() {
        let f = vec![vec![1.0], vec![2.0]];
        let y = vec![0.0, 1.0];
        // max_iter = 0
        assert!(logistic_regression(&f, &y, 1.0, 0).is_err());
        // mismatched lengths
        assert!(logistic_regression(&f, &[0.0], 1.0, 10).is_err());
        // invalid label
        assert!(logistic_regression(&f, &[0.0, 0.5], 1.0, 10).is_err());
        // too few points
        assert!(logistic_regression(&[vec![1.0]], &[0.0], 1.0, 10).is_err());
    }

    #[test]
    fn logistic_serde_roundtrip() {
        let model = LogisticModel {
            coefficients: vec![0.5, -1.2, 3.0],
            iterations: 10,
            converged: true,
        };
        let json = serde_json::to_string(&model).unwrap();
        let model2: LogisticModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model.coefficients, model2.coefficients);
        assert_eq!(model.iterations, model2.iterations);
        assert_eq!(model.converged, model2.converged);
    }

    #[test]
    fn sigmoid_known_values() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        assert!(sigmoid(100.0) > 0.999);
        assert!(sigmoid(-100.0) < 0.001);
        // Symmetry: sigmoid(x) + sigmoid(-x) = 1
        for &x in &[-5.0, -1.0, 0.0, 1.0, 5.0] {
            assert!((sigmoid(x) + sigmoid(-x) - 1.0).abs() < 1e-10);
        }
    }
}
