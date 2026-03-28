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

// ---------------------------------------------------------------------------
// Correlation matrix
// ---------------------------------------------------------------------------

/// Computes the Pearson correlation matrix for a set of variables.
///
/// Each inner slice is one variable's observations (all must have the same length).
/// Returns a symmetric matrix where entry \[i\]\[j\] is the correlation between
/// variable i and variable j.
///
/// # Errors
///
/// Returns `InvalidSample` if fewer than 2 variables or fewer than 2 observations.
/// Returns `DimensionMismatch` if variables have different lengths.
#[must_use = "returns the correlation matrix"]
pub fn correlation_matrix(variables: &[&[f64]]) -> Result<Vec<Vec<f64>>, PramanaError> {
    let p = variables.len();
    if p < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 variables".into(),
        ));
    }
    let n = variables[0].len();
    if n < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations".into(),
        ));
    }
    for (i, var) in variables.iter().enumerate() {
        if var.len() != n {
            return Err(PramanaError::DimensionMismatch(format!(
                "variable {i} has length {}, expected {n}",
                var.len()
            )));
        }
    }

    // Compute means and std devs
    let means: Vec<f64> = variables
        .iter()
        .map(|v| v.iter().sum::<f64>() / n as f64)
        .collect();
    let std_devs: Vec<f64> = variables
        .iter()
        .zip(&means)
        .map(|(v, &m)| {
            let var = v.iter().map(|&x| (x - m) * (x - m)).sum::<f64>() / n as f64;
            var.sqrt()
        })
        .collect();

    let mut matrix = vec![vec![0.0; p]; p];
    for i in 0..p {
        matrix[i][i] = 1.0;
        for j in (i + 1)..p {
            if std_devs[i] == 0.0 || std_devs[j] == 0.0 {
                matrix[i][j] = 0.0;
                matrix[j][i] = 0.0;
            } else {
                let cov: f64 = variables[i]
                    .iter()
                    .zip(variables[j].iter())
                    .map(|(&xi, &xj)| (xi - means[i]) * (xj - means[j]))
                    .sum::<f64>()
                    / n as f64;
                let r = cov / (std_devs[i] * std_devs[j]);
                matrix[i][j] = r;
                matrix[j][i] = r;
            }
        }
    }
    Ok(matrix)
}

// ---------------------------------------------------------------------------
// Principal Component Analysis
// ---------------------------------------------------------------------------

/// Result of a Principal Component Analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcaResult {
    /// Principal component directions (eigenvectors), one per row. Sorted by
    /// descending eigenvalue. Shape: p × p.
    pub components: Vec<Vec<f64>>,
    /// Eigenvalues (variances explained) in descending order.
    pub eigenvalues: Vec<f64>,
    /// Proportion of variance explained by each component.
    pub explained_variance_ratio: Vec<f64>,
}

/// Performs Principal Component Analysis on column-oriented data.
///
/// Each inner slice is one variable (column) of observations. All must have
/// the same length. The data is centered (mean-subtracted) before computing
/// the covariance matrix. Eigendecomposition is via `hisab::num::eigen_symmetric`.
///
/// # Errors
///
/// Returns `InvalidSample` if fewer than 2 variables or fewer than 2 observations.
/// Returns `DimensionMismatch` if variables have different lengths.
#[must_use = "returns the PCA result"]
pub fn pca(variables: &[&[f64]]) -> Result<PcaResult, PramanaError> {
    let p = variables.len();
    if p < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 variables".into(),
        ));
    }
    let n = variables[0].len();
    if n < 2 {
        return Err(PramanaError::InvalidSample(
            "need at least 2 observations".into(),
        ));
    }
    for (i, var) in variables.iter().enumerate() {
        if var.len() != n {
            return Err(PramanaError::DimensionMismatch(format!(
                "variable {i} has length {}, expected {n}",
                var.len()
            )));
        }
    }

    // Compute covariance matrix (population covariance)
    let means: Vec<f64> = variables
        .iter()
        .map(|v| v.iter().sum::<f64>() / n as f64)
        .collect();
    let mut cov = vec![vec![0.0; p]; p];
    for i in 0..p {
        for j in i..p {
            let c: f64 = variables[i]
                .iter()
                .zip(variables[j].iter())
                .map(|(&xi, &xj)| (xi - means[i]) * (xj - means[j]))
                .sum::<f64>()
                / n as f64;
            cov[i][j] = c;
            cov[j][i] = c;
        }
    }

    // Eigendecomposition (Jacobi for symmetric matrices)
    let eigen = hisab::num::eigen_symmetric(&cov, 1e-12, 1000)
        .map_err(|e| PramanaError::ComputationError(format!("eigendecomposition failed: {e}")))?;

    let evecs = eigen
        .eigenvectors
        .ok_or_else(|| PramanaError::ComputationError("eigenvectors not available".into()))?;

    // eigen_symmetric already sorts by descending magnitude; we want descending value
    // (for PCA, eigenvalues of a covariance matrix are non-negative)
    let evals = &eigen.eigenvalues_real;
    let mut indices: Vec<usize> = (0..evals.len()).collect();
    indices.sort_by(|&a, &b| {
        evals[b]
            .partial_cmp(&evals[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let eigenvalues: Vec<f64> = indices.iter().map(|&i| evals[i].max(0.0)).collect();
    let total_var: f64 = eigenvalues.iter().sum();
    let explained_variance_ratio: Vec<f64> = if total_var > 0.0 {
        eigenvalues.iter().map(|&v| v / total_var).collect()
    } else {
        vec![0.0; p]
    };

    // Eigenvectors: evecs[i] is the i-th eigenvector (already sorted by eigen_symmetric)
    // Re-order to match our sorted indices
    let components: Vec<Vec<f64>> = indices.iter().map(|&idx| evecs[idx].clone()).collect();

    Ok(PcaResult {
        components,
        eigenvalues,
        explained_variance_ratio,
    })
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

    // --- Correlation matrix ---

    #[test]
    fn corr_perfect_positive() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x
        let m = correlation_matrix(&[&x, &y]).unwrap();
        assert!((m[0][0] - 1.0).abs() < 1e-10);
        assert!((m[1][1] - 1.0).abs() < 1e-10);
        assert!((m[0][1] - 1.0).abs() < 1e-8, "r = {}", m[0][1]);
        assert!((m[1][0] - 1.0).abs() < 1e-8);
    }

    #[test]
    fn corr_perfect_negative() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [5.0, 4.0, 3.0, 2.0, 1.0];
        let m = correlation_matrix(&[&x, &y]).unwrap();
        assert!((m[0][1] + 1.0).abs() < 1e-8, "r = {}", m[0][1]);
    }

    #[test]
    fn corr_symmetric() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [4.0, 3.0, 2.0, 5.0];
        let c = [1.0, 5.0, 2.0, 4.0];
        let m = correlation_matrix(&[&a, &b, &c]).unwrap();
        assert_eq!(m.len(), 3);
        for (i, row) in m.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                assert!((val - m[j][i]).abs() < 1e-10, "not symmetric at [{i}][{j}]");
            }
        }
    }

    #[test]
    fn corr_invalid_params() {
        let x = [1.0, 2.0];
        assert!(correlation_matrix(&[&x]).is_err()); // < 2 vars
        assert!(correlation_matrix(&[&[1.0], &[2.0, 3.0]]).is_err()); // dim mismatch
    }

    // --- PCA ---

    #[test]
    fn pca_variance_sums_to_one() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.1, 3.9, 6.1, 7.9, 10.1]; // ~2x + noise
        let result = pca(&[&x, &y]).unwrap();
        let total: f64 = result.explained_variance_ratio.iter().sum();
        assert!((total - 1.0).abs() < 1e-6, "variance ratios sum to {total}");
    }

    #[test]
    fn pca_dominant_component() {
        // Two variables where y ≈ 2x: first PC should explain almost all variance
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 2.0 * xi + 0.01 * xi.sin()).collect();
        let result = pca(&[&x, &y]).unwrap();
        assert!(
            result.explained_variance_ratio[0] > 0.99,
            "first PC explains {:.4}",
            result.explained_variance_ratio[0]
        );
    }

    #[test]
    fn pca_eigenvalues_descending() {
        let a = [1.0, 3.0, 2.0, 5.0, 4.0];
        let b = [5.0, 1.0, 4.0, 2.0, 3.0];
        let result = pca(&[&a, &b]).unwrap();
        for w in result.eigenvalues.windows(2) {
            assert!(w[0] >= w[1], "eigenvalues not descending");
        }
    }

    #[test]
    fn pca_components_orthogonal() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [5.0, 3.0, 1.0, 4.0, 2.0];
        let result = pca(&[&a, &b]).unwrap();
        // Dot product of component 0 and component 1 should be ~0
        let dot: f64 = result.components[0]
            .iter()
            .zip(&result.components[1])
            .map(|(a, b)| a * b)
            .sum();
        assert!(dot.abs() < 1e-6, "components not orthogonal: dot = {dot}");
    }

    #[test]
    fn pca_serde_roundtrip() {
        let result = PcaResult {
            components: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            eigenvalues: vec![2.0, 1.0],
            explained_variance_ratio: vec![0.667, 0.333],
        };
        let json = serde_json::to_string(&result).unwrap();
        let r2: PcaResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.eigenvalues, r2.eigenvalues);
        assert_eq!(result.components, r2.components);
    }

    #[test]
    fn pca_invalid_params() {
        let x = [1.0, 2.0];
        assert!(pca(&[&x]).is_err()); // < 2 vars
    }
}
