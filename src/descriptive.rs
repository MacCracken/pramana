//! Descriptive statistics and kernel density estimation.

use crate::error::PramanaError;
use serde::{Deserialize, Serialize};

/// Computes the arithmetic mean of the data.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the computed mean"]
pub fn mean(data: &[f64]) -> Result<f64, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    Ok(sum(data) / data.len() as f64)
}

/// Computes the median of the data.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the computed median"]
pub fn median(data: &[f64]) -> Result<f64, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n.is_multiple_of(2) {
        Ok((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    } else {
        Ok(sorted[n / 2])
    }
}

/// Returns the mode (most frequently occurring value) of the data.
///
/// For floating-point data, values are compared with exact equality.
/// If multiple values share the highest frequency, the first encountered is returned.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the computed mode"]
pub fn mode(data: &[f64]) -> Result<f64, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    // Simple O(n^2) approach that avoids pulling in a HashMap dependency.
    let mut best_val = data[0];
    let mut best_count: usize = 0;

    for &val in data {
        let count = data
            .iter()
            .filter(|&&v| v.to_bits() == val.to_bits())
            .count();
        if count > best_count {
            best_count = count;
            best_val = val;
        }
    }
    Ok(best_val)
}

/// Computes the population variance of the data.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the computed variance"]
pub fn variance(data: &[f64]) -> Result<f64, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    let m = mean(data)?;
    let n = data.len() as f64;
    let var = data.iter().map(|&x| (x - m) * (x - m)).sum::<f64>() / n;
    Ok(var)
}

/// Computes the population standard deviation of the data.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the computed standard deviation"]
pub fn std_dev(data: &[f64]) -> Result<f64, PramanaError> {
    variance(data).map(|v| v.sqrt())
}

/// Computes the skewness of the data (Fisher's definition, population).
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty or has zero variance.
#[must_use = "returns the computed skewness"]
pub fn skewness(data: &[f64]) -> Result<f64, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    let m = mean(data)?;
    let n = data.len() as f64;
    let s = std_dev(data)?;
    if s == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero variance, skewness undefined".into(),
        ));
    }
    let skew = data.iter().map(|&x| ((x - m) / s).powi(3)).sum::<f64>() / n;
    Ok(skew)
}

/// Computes the excess kurtosis of the data (Fisher's definition, population).
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty or has zero variance.
#[must_use = "returns the computed kurtosis"]
pub fn kurtosis(data: &[f64]) -> Result<f64, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    let m = mean(data)?;
    let n = data.len() as f64;
    let s = std_dev(data)?;
    if s == 0.0 {
        return Err(PramanaError::InvalidSample(
            "zero variance, kurtosis undefined".into(),
        ));
    }
    let kurt = data.iter().map(|&x| ((x - m) / s).powi(4)).sum::<f64>() / n - 3.0;
    Ok(kurt)
}

/// Computes the p-th percentile using linear interpolation.
///
/// `p` must be in `[0, 100]`.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
/// Returns `InvalidParameter` if `p` is not in `[0, 100]`.
#[must_use = "returns the computed percentile"]
pub fn percentile(data: &[f64], p: f64) -> Result<f64, PramanaError> {
    if data.is_empty() {
        return Err(PramanaError::InvalidSample("empty data".into()));
    }
    if !(0.0..=100.0).contains(&p) {
        return Err(PramanaError::InvalidParameter(
            "percentile must be in [0, 100]".into(),
        ));
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 1 {
        return Ok(sorted[0]);
    }
    let rank = (p / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;
    Ok(sorted[lower] * (1.0 - frac) + sorted[upper] * frac)
}

/// Returns (Q1, Q2/median, Q3).
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the computed quartiles"]
pub fn quartiles(data: &[f64]) -> Result<(f64, f64, f64), PramanaError> {
    let q1 = percentile(data, 25.0)?;
    let q2 = percentile(data, 50.0)?;
    let q3 = percentile(data, 75.0)?;
    Ok((q1, q2, q3))
}

/// Interquartile range: Q3 - Q1.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the computed IQR"]
pub fn iqr(data: &[f64]) -> Result<f64, PramanaError> {
    let (q1, _, q3) = quartiles(data)?;
    Ok(q3 - q1)
}

/// Returns the minimum value.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the minimum value"]
pub fn min(data: &[f64]) -> Result<f64, PramanaError> {
    data.iter()
        .copied()
        .reduce(f64::min)
        .ok_or_else(|| PramanaError::InvalidSample("empty data".into()))
}

/// Returns the maximum value.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the maximum value"]
pub fn max(data: &[f64]) -> Result<f64, PramanaError> {
    data.iter()
        .copied()
        .reduce(f64::max)
        .ok_or_else(|| PramanaError::InvalidSample("empty data".into()))
}

/// Returns the range: max - min.
///
/// # Errors
///
/// Returns `InvalidSample` if the slice is empty.
#[must_use = "returns the range"]
pub fn range(data: &[f64]) -> Result<f64, PramanaError> {
    Ok(max(data)? - min(data)?)
}

/// Returns the sum of all elements.
#[must_use = "returns the sum"]
pub fn sum(data: &[f64]) -> f64 {
    data.iter().sum()
}

// ---------------------------------------------------------------------------
// Kernel density estimation
// ---------------------------------------------------------------------------

/// Kernel function for density estimation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Kernel {
    /// Gaussian kernel: K(u) = exp(-u²/2) / sqrt(2π).
    Gaussian,
    /// Epanechnikov kernel: K(u) = 3/4 * (1 - u²) for |u| ≤ 1.
    Epanechnikov,
    /// Uniform (rectangular) kernel: K(u) = 1/2 for |u| ≤ 1.
    Uniform,
    /// Triangular kernel: K(u) = 1 - |u| for |u| ≤ 1.
    Triangular,
}

impl Kernel {
    /// Evaluates the kernel function at `u`.
    #[must_use]
    #[inline]
    fn evaluate(self, u: f64) -> f64 {
        match self {
            Kernel::Gaussian => (-0.5 * u * u).exp() / (2.0 * std::f64::consts::PI).sqrt(),
            Kernel::Epanechnikov => {
                if u.abs() <= 1.0 {
                    0.75 * (1.0 - u * u)
                } else {
                    0.0
                }
            }
            Kernel::Uniform => {
                if u.abs() <= 1.0 {
                    0.5
                } else {
                    0.0
                }
            }
            Kernel::Triangular => {
                let abs_u = u.abs();
                if abs_u <= 1.0 { 1.0 - abs_u } else { 0.0 }
            }
        }
    }
}

/// A kernel density estimator fitted to data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KernelDensity {
    /// The data points.
    pub data: Vec<f64>,
    /// Bandwidth (smoothing parameter).
    pub bandwidth: f64,
    /// Kernel function used.
    pub kernel: Kernel,
}

impl KernelDensity {
    /// Creates a KDE with a specified bandwidth.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `data` is empty.
    /// Returns `InvalidParameter` if `bandwidth <= 0`.
    pub fn new(data: Vec<f64>, bandwidth: f64, kernel: Kernel) -> Result<Self, PramanaError> {
        if data.is_empty() {
            return Err(PramanaError::InvalidSample("empty data".into()));
        }
        if bandwidth <= 0.0 {
            return Err(PramanaError::InvalidParameter(
                "bandwidth must be positive".into(),
            ));
        }
        Ok(Self {
            data,
            bandwidth,
            kernel,
        })
    }

    /// Creates a KDE with bandwidth selected by Silverman's rule of thumb.
    ///
    /// h = 0.9 * min(σ, IQR/1.34) * n^(-1/5)
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `data` has fewer than 2 elements or zero variance.
    pub fn silverman(data: Vec<f64>, kernel: Kernel) -> Result<Self, PramanaError> {
        if data.len() < 2 {
            return Err(PramanaError::InvalidSample(
                "need at least 2 data points for Silverman bandwidth".into(),
            ));
        }
        let s = std_dev(&data)?;
        if s == 0.0 {
            return Err(PramanaError::InvalidSample(
                "zero variance: cannot compute bandwidth".into(),
            ));
        }
        let iqr_val = iqr(&data)?;
        let spread = if iqr_val > 0.0 {
            s.min(iqr_val / 1.34)
        } else {
            s
        };
        let n = data.len() as f64;
        let bandwidth = 0.9 * spread * n.powf(-0.2);

        Ok(Self {
            data,
            bandwidth,
            kernel,
        })
    }

    /// Evaluates the estimated density at a single point.
    #[must_use]
    pub fn evaluate(&self, x: f64) -> f64 {
        let n = self.data.len() as f64;
        let h = self.bandwidth;
        let sum: f64 = self
            .data
            .iter()
            .map(|&xi| self.kernel.evaluate((x - xi) / h))
            .sum();
        sum / (n * h)
    }

    /// Evaluates the estimated density at multiple points.
    #[must_use]
    pub fn evaluate_grid(&self, points: &[f64]) -> Vec<f64> {
        points.iter().map(|&x| self.evaluate(x)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        assert!((mean(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_median_odd() {
        assert!((median(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_median_even() {
        assert!((median(&[1.0, 2.0, 3.0, 4.0]).unwrap() - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_mode() {
        assert!((mode(&[1.0, 2.0, 2.0, 3.0]).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_variance() {
        // Variance of [1,2,3,4,5] = 2.0
        assert!((variance(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_std_dev() {
        let s = std_dev(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        assert!((s - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_empty_errors() {
        assert!(mean(&[]).is_err());
        assert!(median(&[]).is_err());
        assert!(variance(&[]).is_err());
    }

    #[test]
    fn test_percentile() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&data, 50.0).unwrap() - 3.0).abs() < 1e-10);
        assert!((percentile(&data, 0.0).unwrap() - 1.0).abs() < 1e-10);
        assert!((percentile(&data, 100.0).unwrap() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_min_max_range() {
        let data = [3.0, 1.0, 4.0, 1.0, 5.0];
        assert!((min(&data).unwrap() - 1.0).abs() < 1e-10);
        assert!((max(&data).unwrap() - 5.0).abs() < 1e-10);
        assert!((range(&data).unwrap() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_sum() {
        assert!((sum(&[1.0, 2.0, 3.0]) - 6.0).abs() < 1e-10);
        assert_eq!(sum(&[]), 0.0);
    }

    // --- KDE ---

    #[test]
    fn kde_gaussian_integrates_to_one() {
        // A Gaussian KDE should approximately integrate to 1
        let data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let kde = KernelDensity::new(data, 1.0, Kernel::Gaussian).unwrap();
        // Numerical integration via trapezoidal rule over a wide range
        let n = 10_000;
        let lo = -10.0;
        let hi = 15.0;
        let dx = (hi - lo) / n as f64;
        let integral: f64 = (0..n)
            .map(|i| {
                let x = lo + (i as f64 + 0.5) * dx;
                kde.evaluate(x) * dx
            })
            .sum();
        assert!(
            (integral - 1.0).abs() < 0.01,
            "KDE should integrate to ~1: {integral}"
        );
    }

    #[test]
    fn kde_peak_near_data() {
        // Single point — KDE should peak at that point
        let kde = KernelDensity::new(vec![5.0], 1.0, Kernel::Gaussian).unwrap();
        let at_peak = kde.evaluate(5.0);
        let away = kde.evaluate(10.0);
        assert!(at_peak > away, "density at data point should be highest");
    }

    #[test]
    fn kde_silverman_bandwidth() {
        let data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
        let kde = KernelDensity::silverman(data, Kernel::Gaussian).unwrap();
        assert!(kde.bandwidth > 0.0, "bandwidth = {}", kde.bandwidth);
    }

    #[test]
    fn kde_evaluate_grid() {
        let kde = KernelDensity::new(vec![0.0, 1.0], 0.5, Kernel::Gaussian).unwrap();
        let grid = kde.evaluate_grid(&[-1.0, 0.0, 0.5, 1.0, 2.0]);
        assert_eq!(grid.len(), 5);
        for &v in &grid {
            assert!(v >= 0.0);
        }
    }

    #[test]
    fn kde_epanechnikov_compact() {
        // Epanechnikov kernel has compact support: 0 outside |u| > 1
        let kde = KernelDensity::new(vec![0.0], 1.0, Kernel::Epanechnikov).unwrap();
        assert!(kde.evaluate(0.0) > 0.0);
        assert_eq!(kde.evaluate(2.0), 0.0);
        assert_eq!(kde.evaluate(-2.0), 0.0);
    }

    #[test]
    fn kde_all_kernels_nonneg() {
        let data = vec![1.0, 2.0, 3.0];
        for kernel in [
            Kernel::Gaussian,
            Kernel::Epanechnikov,
            Kernel::Uniform,
            Kernel::Triangular,
        ] {
            let kde = KernelDensity::new(data.clone(), 0.5, kernel).unwrap();
            for i in -20..=60 {
                let x = i as f64 * 0.1;
                assert!(kde.evaluate(x) >= 0.0, "{kernel:?} density negative at {x}");
            }
        }
    }

    #[test]
    fn kde_invalid_params() {
        assert!(KernelDensity::new(vec![], 1.0, Kernel::Gaussian).is_err());
        assert!(KernelDensity::new(vec![1.0], 0.0, Kernel::Gaussian).is_err());
        assert!(KernelDensity::new(vec![1.0], -1.0, Kernel::Gaussian).is_err());
        assert!(KernelDensity::silverman(vec![1.0], Kernel::Gaussian).is_err());
    }

    #[test]
    fn kde_serde_roundtrip() {
        let kde = KernelDensity::new(vec![1.0, 2.0, 3.0], 0.5, Kernel::Gaussian).unwrap();
        let json = serde_json::to_string(&kde).unwrap();
        let kde2: KernelDensity = serde_json::from_str(&json).unwrap();
        assert_eq!(kde.data, kde2.data);
        assert_eq!(kde.bandwidth, kde2.bandwidth);
    }
}
