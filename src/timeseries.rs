//! Time series analysis: moving average, exponential smoothing, autocorrelation, ARIMA.

use crate::error::PramanaError;
use serde::{Deserialize, Serialize};

/// Computes the simple moving average with the given window size.
///
/// Returns a vector of length `data.len() - window + 1`.
///
/// # Errors
///
/// Returns `InvalidParameter` if `window` is 0 or greater than `data.len()`.
#[must_use = "returns the moving average series"]
pub fn moving_average(data: &[f64], window: usize) -> Result<Vec<f64>, PramanaError> {
    if window == 0 {
        return Err(PramanaError::InvalidParameter(
            "window must be positive".into(),
        ));
    }
    if window > data.len() {
        return Err(PramanaError::InvalidParameter(format!(
            "window {window} exceeds data length {}",
            data.len()
        )));
    }

    let mut result = Vec::with_capacity(data.len() - window + 1);
    let mut sum: f64 = data[..window].iter().sum();
    result.push(sum / window as f64);

    for i in window..data.len() {
        sum += data[i] - data[i - window];
        result.push(sum / window as f64);
    }

    Ok(result)
}

/// Applies simple exponential smoothing to the data.
///
/// s_0 = data\[0\], s_t = alpha * data\[t\] + (1 - alpha) * s_{t-1}.
///
/// # Errors
///
/// Returns `InvalidParameter` if `alpha` is not in `(0, 1]`.
/// Returns `InvalidSample` if `data` is empty.
#[must_use = "returns the smoothed series"]
pub fn exponential_smoothing(data: &[f64], alpha: f64) -> Result<Vec<f64>, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    if alpha <= 0.0 || alpha > 1.0 {
        return Err(PramanaError::InvalidParameter(
            "alpha must be in (0, 1]".into(),
        ));
    }

    let mut result = Vec::with_capacity(data.len());
    result.push(data[0]);
    for i in 1..data.len() {
        let prev = result[i - 1];
        result.push(alpha * data[i] + (1.0 - alpha) * prev);
    }
    Ok(result)
}

/// Computes the autocorrelation of the data at the given lag.
///
/// Uses the standard definition: r(k) = c(k) / c(0), where c(k) is the
/// autocovariance at lag k.
///
/// # Errors
///
/// Returns `InvalidParameter` if `lag >= data.len()` or `lag == 0`.
/// Returns `InvalidSample` if `data` has fewer than 2 elements.
#[must_use = "returns the autocorrelation coefficient"]
pub fn autocorrelation(data: &[f64], lag: usize) -> Result<f64, PramanaError> {
    if data.len() < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 data points".into(),
        ));
    }
    if lag == 0 {
        return Ok(1.0);
    }
    if lag >= data.len() {
        return Err(PramanaError::InvalidParameter(format!(
            "lag {lag} must be less than data length {}",
            data.len()
        )));
    }

    let n = data.len() as f64;
    let mean: f64 = data.iter().sum::<f64>() / n;

    let c0: f64 = data.iter().map(|&x| (x - mean) * (x - mean)).sum::<f64>() / n;
    if c0 == 0.0 {
        return Ok(0.0);
    }

    let ck: f64 = data
        .iter()
        .zip(data[lag..].iter())
        .map(|(&x, &y)| (x - mean) * (y - mean))
        .sum::<f64>()
        / n;

    Ok(ck / c0)
}

// ---------------------------------------------------------------------------
// Differencing
// ---------------------------------------------------------------------------

/// Applies d-th order differencing to a time series.
///
/// First-order differencing: z_t = y_t - y_{t-1}.
/// Higher orders apply differencing recursively.
///
/// # Errors
///
/// Returns `InvalidParameter` if `d` is 0.
/// Returns `InvalidSample` if the series is too short for the given order.
#[must_use = "returns the differenced series"]
pub fn difference(data: &[f64], d: usize) -> Result<Vec<f64>, PramanaError> {
    if d == 0 {
        return Err(PramanaError::InvalidParameter(
            "differencing order must be positive".into(),
        ));
    }
    if data.len() <= d {
        return Err(PramanaError::InvalidSample(format!(
            "need more than {d} data points for order-{d} differencing"
        )));
    }
    let mut result = data.to_vec();
    for _ in 0..d {
        let prev = result.clone();
        result = prev.windows(2).map(|w| w[1] - w[0]).collect();
    }
    Ok(result)
}

/// Reverses d-th order differencing given the original series prefix.
///
/// Reconstructs the integrated series from the differenced values and
/// the first `d` values of the original series.
///
/// # Errors
///
/// Returns `InvalidParameter` if `d` is 0.
/// Returns `InvalidSample` if `original_prefix` has fewer than `d` elements.
#[must_use = "returns the integrated series"]
pub fn integrate(
    differenced: &[f64],
    original_prefix: &[f64],
    d: usize,
) -> Result<Vec<f64>, PramanaError> {
    if d == 0 {
        return Err(PramanaError::InvalidParameter(
            "integration order must be positive".into(),
        ));
    }
    if original_prefix.len() < d {
        return Err(PramanaError::InvalidSample(format!(
            "need at least {d} prefix values for order-{d} integration"
        )));
    }
    let mut result = differenced.to_vec();
    // For each integration step, prepend the appropriate original value and cumsum
    for step in (0..d).rev() {
        let seed = original_prefix[step];
        let mut integrated = Vec::with_capacity(result.len() + 1);
        integrated.push(seed);
        for &val in &result {
            let prev = *integrated.last().unwrap_or(&0.0);
            integrated.push(prev + val);
        }
        result = integrated;
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// ARIMA model
// ---------------------------------------------------------------------------

/// A fitted ARIMA(p, d, q) model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArimaModel {
    /// AR coefficients φ₁, ..., φₚ.
    pub ar_coefficients: Vec<f64>,
    /// Differencing order.
    pub d: usize,
    /// Intercept (mean of the differenced series for d > 0, or series mean for d = 0).
    pub intercept: f64,
    /// Residual variance.
    pub residual_variance: f64,
}

/// Fits an ARIMA(p, d, 0) model to the data using Yule-Walker equations for
/// the AR component.
///
/// This supports autoregressive models with differencing. For pure AR models
/// use `d = 0`.
///
/// # Errors
///
/// Returns `InvalidParameter` if `p` is 0.
/// Returns `InvalidSample` if data is too short for the given p and d.
#[must_use = "returns the fitted ARIMA model"]
pub fn arima_fit(data: &[f64], p: usize, d: usize) -> Result<ArimaModel, PramanaError> {
    if p == 0 {
        return Err(PramanaError::InvalidParameter(
            "AR order p must be positive".into(),
        ));
    }

    // Apply differencing
    let z = if d > 0 {
        difference(data, d)?
    } else {
        data.to_vec()
    };

    if z.len() <= p {
        return Err(PramanaError::InvalidSample(format!(
            "need more than {p} observations after differencing (got {})",
            z.len()
        )));
    }

    let n = z.len() as f64;
    let mean_z: f64 = z.iter().sum::<f64>() / n;

    // Compute autocovariances c(0), c(1), ..., c(p)
    let centered: Vec<f64> = z.iter().map(|&v| v - mean_z).collect();
    let c0: f64 = centered.iter().map(|x| x * x).sum::<f64>() / n;
    if c0 < 1e-30 {
        return Err(PramanaError::InvalidSample(
            "zero variance in (differenced) series".into(),
        ));
    }

    let mut autocov = Vec::with_capacity(p + 1);
    autocov.push(c0);
    for lag in 1..=p {
        let ck: f64 = centered
            .iter()
            .zip(centered[lag..].iter())
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / n;
        autocov.push(ck);
    }

    // Build Toeplitz matrix R where R[i][j] = autocov[|i-j|] / c0 = autocorrelation
    let mut r_matrix = vec![vec![0.0; p]; p];
    for (i, row) in r_matrix.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = autocov[i.abs_diff(j)] / c0;
        }
    }

    // Right-hand side: [r(1)/c0, r(2)/c0, ..., r(p)/c0] = autocorrelations
    let rhs: Vec<f64> = (1..=p).map(|k| autocov[k] / c0).collect();

    // Solve R * phi = rhs via Cholesky
    let ar_coefficients = match hisab::num::cholesky(&r_matrix) {
        Ok(l) => hisab::num::cholesky_solve(&l, &rhs).map_err(|e| {
            PramanaError::ComputationError(format!("Yule-Walker solve failed: {e}"))
        })?,
        Err(e) => {
            return Err(PramanaError::ComputationError(format!(
                "autocorrelation matrix not positive-definite: {e}"
            )));
        }
    };

    // Compute intercept: c = mean * (1 - sum(phi))
    let phi_sum: f64 = ar_coefficients.iter().sum();
    let intercept = mean_z * (1.0 - phi_sum);

    // Compute residual variance
    let mut residual_ss = 0.0;
    let mut count = 0.0;
    for t in p..z.len() {
        let mut pred = intercept;
        for (k, &phi) in ar_coefficients.iter().enumerate() {
            pred += phi * z[t - 1 - k];
        }
        let resid = z[t] - pred;
        residual_ss += resid * resid;
        count += 1.0;
    }
    let residual_variance = if count > 0.0 {
        residual_ss / count
    } else {
        0.0
    };

    Ok(ArimaModel {
        ar_coefficients,
        d,
        intercept,
        residual_variance,
    })
}

/// Forecasts future values using a fitted ARIMA model.
///
/// `history` is the original (undifferenced) series used for initialization.
/// Returns `steps` forecasted values.
///
/// # Errors
///
/// Returns `InvalidSample` if `history` is too short.
/// Returns `InvalidParameter` if `steps` is 0.
#[must_use = "returns the forecasted values"]
pub fn arima_forecast(
    model: &ArimaModel,
    history: &[f64],
    steps: usize,
) -> Result<Vec<f64>, PramanaError> {
    if steps == 0 {
        return Err(PramanaError::InvalidParameter(
            "steps must be positive".into(),
        ));
    }
    let p = model.ar_coefficients.len();

    // Get the differenced series for AR initialization
    let mut z = if model.d > 0 {
        difference(history, model.d)?
    } else {
        history.to_vec()
    };

    if z.len() < p {
        return Err(PramanaError::InvalidSample(format!(
            "history too short: need at least {} differenced values, got {}",
            p,
            z.len()
        )));
    }

    // Forecast in differenced space
    let mut forecasts_z = Vec::with_capacity(steps);
    for _ in 0..steps {
        let mut pred = model.intercept;
        let len = z.len();
        for (k, &phi) in model.ar_coefficients.iter().enumerate() {
            pred += phi * z[len - 1 - k];
        }
        forecasts_z.push(pred);
        z.push(pred);
    }

    // Integrate back if d > 0
    if model.d > 0 {
        // We need the last d values of the original series as prefix for integration
        let tail_start = history.len().saturating_sub(model.d);
        let prefix = &history[tail_start..];
        let integrated = integrate(&forecasts_z, prefix, model.d)?;
        // integrated has d prefix values + steps forecast values; skip the prefix
        Ok(integrated[model.d..].to_vec())
    } else {
        Ok(forecasts_z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = moving_average(&data, 3).unwrap();
        assert_eq!(ma.len(), 3);
        assert!((ma[0] - 2.0).abs() < 1e-10);
        assert!((ma[1] - 3.0).abs() < 1e-10);
        assert!((ma[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_moving_average_full_window() {
        let data = [1.0, 2.0, 3.0];
        let ma = moving_average(&data, 3).unwrap();
        assert_eq!(ma.len(), 1);
        assert!((ma[0] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_exponential_smoothing() {
        let data = [10.0, 12.0, 13.0, 12.0, 10.0];
        let smoothed = exponential_smoothing(&data, 0.5).unwrap();
        assert_eq!(smoothed.len(), 5);
        assert!((smoothed[0] - 10.0).abs() < 1e-10);
        // s_1 = 0.5 * 12 + 0.5 * 10 = 11
        assert!((smoothed[1] - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_autocorrelation_lag0() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((autocorrelation(&data, 0).unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_autocorrelation_constant() {
        // Constant data has zero variance -> return 0
        let data = [5.0, 5.0, 5.0, 5.0, 5.0];
        assert!((autocorrelation(&data, 1).unwrap()).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_window() {
        assert!(moving_average(&[1.0, 2.0], 0).is_err());
        assert!(moving_average(&[1.0, 2.0], 3).is_err());
    }

    #[test]
    fn test_invalid_alpha() {
        assert!(exponential_smoothing(&[1.0], 0.0).is_err());
        assert!(exponential_smoothing(&[1.0], 1.5).is_err());
    }

    // --- Differencing ---

    #[test]
    fn test_difference_order_1() {
        let data = [1.0, 3.0, 6.0, 10.0];
        let d = difference(&data, 1).unwrap();
        assert_eq!(d.len(), 3);
        assert!((d[0] - 2.0).abs() < 1e-10);
        assert!((d[1] - 3.0).abs() < 1e-10);
        assert!((d[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_difference_order_2() {
        // Second differences of [1, 3, 6, 10, 15] = first diffs [2, 3, 4, 5] = second diffs [1, 1, 1]
        let data = [1.0, 3.0, 6.0, 10.0, 15.0];
        let d = difference(&data, 2).unwrap();
        assert_eq!(d.len(), 3);
        for &v in &d {
            assert!((v - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_integrate_roundtrip() {
        let data = [1.0, 3.0, 6.0, 10.0, 15.0];
        let d = difference(&data, 1).unwrap();
        let reconstructed = integrate(&d, &data[..1], 1).unwrap();
        assert_eq!(reconstructed.len(), data.len());
        for (a, b) in reconstructed.iter().zip(&data) {
            assert!((a - b).abs() < 1e-10);
        }
    }

    // --- ARIMA ---

    #[test]
    fn arima_ar1_recovery() {
        // Generate AR(1) data: y_t = 0.7 * y_{t-1} + noise using SimpleRng
        use crate::rng::{Rng, SimpleRng};
        let mut rng = SimpleRng::new(42);
        let mut y = vec![0.0; 500];
        for t in 1..500 {
            // Approximate N(0,0.3) via Box-Muller
            let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
            let u2 = rng.next_f64();
            let noise = 0.3 * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            y[t] = 0.7 * y[t - 1] + noise;
        }
        let model = arima_fit(&y, 1, 0).unwrap();
        assert!(
            (model.ar_coefficients[0] - 0.7).abs() < 0.15,
            "phi = {}",
            model.ar_coefficients[0]
        );
    }

    #[test]
    fn arima_forecast_length() {
        let data: Vec<f64> = (0..50).map(|i| (i as f64).sin()).collect();
        let model = arima_fit(&data, 2, 0).unwrap();
        let fc = arima_forecast(&model, &data, 10).unwrap();
        assert_eq!(fc.len(), 10);
        for &v in &fc {
            assert!(v.is_finite(), "forecast value not finite: {v}");
        }
    }

    #[test]
    fn arima_with_differencing() {
        // Trend + AR noise: y_t = t + AR(1) noise
        use crate::rng::{Rng, SimpleRng};
        let mut rng = SimpleRng::new(123);
        let mut noise = vec![0.0; 200];
        for t in 1..200 {
            let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
            let u2 = rng.next_f64();
            let e = 0.2 * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            noise[t] = 0.5 * noise[t - 1] + e;
        }
        let data: Vec<f64> = (0..200).map(|i| i as f64 + noise[i]).collect();
        let model = arima_fit(&data, 1, 1).unwrap();
        let fc = arima_forecast(&model, &data, 5).unwrap();
        // Forecast should continue roughly near the trend
        for (i, &v) in fc.iter().enumerate() {
            let expected = 200.0 + i as f64;
            assert!(
                (v - expected).abs() < 10.0,
                "fc[{i}] = {v}, expected ~{expected}"
            );
        }
    }

    #[test]
    fn arima_invalid_params() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(arima_fit(&data, 0, 0).is_err()); // p = 0
        assert!(arima_fit(&data, 10, 0).is_err()); // p > n
        assert!(
            arima_forecast(
                &ArimaModel {
                    ar_coefficients: vec![0.5],
                    d: 0,
                    intercept: 0.0,
                    residual_variance: 1.0,
                },
                &data,
                0
            )
            .is_err()
        ); // steps = 0
    }

    #[test]
    fn arima_serde_roundtrip() {
        let model = ArimaModel {
            ar_coefficients: vec![0.5, -0.3],
            d: 1,
            intercept: 0.1,
            residual_variance: 0.5,
        };
        let json = serde_json::to_string(&model).unwrap();
        let m2: ArimaModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model.ar_coefficients, m2.ar_coefficients);
        assert_eq!(model.d, m2.d);
        assert_eq!(model.intercept, m2.intercept);
    }
}
