//! Time series analysis: moving average, exponential smoothing, autocorrelation.

use crate::error::PramanaError;

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
}
