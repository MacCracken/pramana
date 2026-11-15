//! Soorat integration — visualization data structures for statistical analysis.

use serde::{Deserialize, Serialize};

/// Distribution curve data for line plot rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistributionCurve {
    /// X-axis sample points.
    pub x_values: Vec<f64>,
    /// PDF values at each point.
    pub pdf: Vec<f64>,
    /// CDF values at each point.
    pub cdf: Vec<f64>,
    /// Distribution name.
    pub name: String,
}

/// MCMC trace for scatter/line rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McmcTrace {
    /// Sample values at each iteration.
    pub samples: Vec<f64>,
    /// Log-likelihood at each iteration (for convergence visualization).
    pub log_likelihoods: Vec<f64>,
    /// Parameter name.
    pub parameter: String,
}

/// Regression fit for line+ribbon rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegressionFit {
    /// Observed x values.
    pub x_observed: Vec<f64>,
    /// Observed y values.
    pub y_observed: Vec<f64>,
    /// Predicted y values (fitted line).
    pub y_predicted: Vec<f64>,
    /// Upper confidence band.
    pub y_upper: Vec<f64>,
    /// Lower confidence band.
    pub y_lower: Vec<f64>,
    /// R-squared value.
    pub r_squared: f64,
}

/// Histogram data for bar chart rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistogramData {
    /// Bin edges (len = n_bins + 1).
    pub bin_edges: Vec<f64>,
    /// Counts per bin.
    pub counts: Vec<u64>,
    /// Total sample count.
    pub total: u64,
}

impl HistogramData {
    /// Create from raw data with uniform binning.
    ///
    /// Non-finite values (NaN, ±inf) are skipped and not counted in `total`
    /// (previously NaN landed in the first bin). If no finite values remain,
    /// or `n_bins` is 0, the histogram is empty. Each bin is at least two ulps
    /// wide at the data's magnitude, so edges are always strictly increasing
    /// (even for constant data).
    #[must_use]
    pub fn from_data(data: &[f64], n_bins: usize) -> Self {
        let empty = Self {
            bin_edges: Vec::new(),
            counts: Vec::new(),
            total: 0,
        };
        let finite = || data.iter().copied().filter(|v| v.is_finite());
        let Some(min) = finite().reduce(f64::min) else {
            return empty;
        };
        if n_bins == 0 {
            return empty;
        }
        let max = finite().fold(min, f64::max);
        // Floor on the range so every bin is at least 2 ulps wide at this
        // magnitude: a constant (or nearly constant) sample still gets
        // distinct, increasing bin edges.
        let magnitude = min.abs().max(max.abs()).max(1.0);
        let floor = magnitude * f64::EPSILON * 2.0 * n_bins as f64;
        let range = (max - min).max(floor);
        let bin_width = range / n_bins as f64;

        let bin_edges: Vec<f64> = (0..=n_bins).map(|i| min + i as f64 * bin_width).collect();
        let mut counts = vec![0u64; n_bins];

        let mut total = 0u64;
        for v in finite() {
            // (v - min) / width is in [0, n_bins]; truncation picks the bin.
            let idx = (((v - min) / bin_width) as usize).min(n_bins - 1);
            counts[idx] += 1;
            total += 1;
        }

        Self {
            bin_edges,
            counts,
            total,
        }
    }
}

/// Correlation matrix for heatmap rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    /// NxN correlation values (-1 to 1). Flattened row-major.
    pub values: Vec<f64>,
    /// Matrix dimension (N).
    pub size: usize,
    /// Variable names.
    pub labels: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 2.5, 3.5];
        let hist = HistogramData::from_data(&data, 4);
        assert_eq!(hist.bin_edges.len(), 5); // 4 bins + 1
        assert_eq!(hist.counts.len(), 4);
        assert_eq!(hist.total, 7);
        assert_eq!(hist.counts.iter().sum::<u64>(), 7);
    }

    #[test]
    fn histogram_empty() {
        let hist = HistogramData::from_data(&[], 10);
        assert!(hist.counts.is_empty());
        assert_eq!(hist.total, 0);
    }

    #[test]
    fn histogram_single_value() {
        let hist = HistogramData::from_data(&[5.0, 5.0, 5.0], 3);
        assert_eq!(hist.total, 3);
    }

    #[test]
    fn distribution_curve_serializes() {
        let curve = DistributionCurve {
            x_values: vec![0.0, 1.0, 2.0],
            pdf: vec![0.0, 0.5, 0.0],
            cdf: vec![0.0, 0.5, 1.0],
            name: "Normal(0,1)".into(),
        };
        let json = serde_json::to_string(&curve);
        assert!(json.is_ok());
    }

    #[test]
    fn mcmc_trace_serializes() {
        let trace = McmcTrace {
            samples: vec![1.0, 1.1, 0.9, 1.05],
            log_likelihoods: vec![-10.0, -9.5, -9.8, -9.3],
            parameter: "mu".into(),
        };
        let json = serde_json::to_string(&trace);
        assert!(json.is_ok());
    }

    #[test]
    fn correlation_matrix_manual() {
        let mat = CorrelationMatrix {
            values: vec![1.0, 0.8, 0.8, 1.0],
            size: 2,
            labels: vec!["x".into(), "y".into()],
        };
        assert_eq!(mat.values.len(), 4);
    }

    #[test]
    fn regression_fit_manual() {
        let fit = RegressionFit {
            x_observed: vec![1.0, 2.0, 3.0],
            y_observed: vec![2.1, 3.9, 6.1],
            y_predicted: vec![2.0, 4.0, 6.0],
            y_upper: vec![2.5, 4.5, 6.5],
            y_lower: vec![1.5, 3.5, 5.5],
            r_squared: 0.998,
        };
        assert!(fit.r_squared > 0.99);
    }

    #[test]
    fn histogram_skips_non_finite_values() {
        let h = HistogramData::from_data(&[f64::NAN, 1.0, 2.0, f64::INFINITY, 3.0], 2);
        assert_eq!(h.total, 3);
        assert_eq!(h.counts.iter().sum::<u64>(), 3);
        assert_eq!(h.bin_edges.first().copied(), Some(1.0));
        assert_eq!(h.bin_edges.last().copied(), Some(3.0));
        let h = HistogramData::from_data(&[f64::NAN, f64::NAN], 4);
        assert!(h.counts.is_empty());
        assert_eq!(h.total, 0);
    }

    #[test]
    fn histogram_constant_large_values_have_distinct_edges() {
        let h = HistogramData::from_data(&[1e12, 1e12, 1e12], 4);
        assert_eq!(h.total, 3);
        assert!(h.bin_edges.windows(2).all(|w| w[1] > w[0]));
    }
}
