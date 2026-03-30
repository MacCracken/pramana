//! Cross-crate bridges — convert primitive values from other AGNOS science crates
//! into pramana statistics parameters and vice versa.

// ── Badal bridges (weather) ────────────────────────────────────────────────

/// Convert ensemble forecast spread (standard deviation) to confidence interval half-width.
///
/// For a Gaussian: 95% CI ≈ 1.96 × σ.
#[must_use]
#[inline]
pub fn ensemble_spread_to_confidence(std_dev: f64, confidence_z: f64) -> f64 {
    std_dev.abs() * confidence_z
}

/// Convert observation error (measurement uncertainty) to Gaussian likelihood weight.
///
/// weight = exp(-0.5 × (residual / σ)²) / (σ × sqrt(2π))
#[must_use]
pub fn observation_error_to_likelihood(residual: f64, std_dev: f64) -> f64 {
    if std_dev <= 0.0 {
        return 0.0;
    }
    let z = residual / std_dev;
    let two_pi_sqrt = std::f64::consts::TAU.sqrt();
    (-0.5 * z * z).exp() / (std_dev * two_pi_sqrt)
}

// ── Kimiya bridges (chemistry) ─────────────────────────────────────────────

/// Convert reaction yield measurements to mean and variance.
///
/// Returns `(mean, variance)` from a slice of yield values.
#[must_use]
pub fn yields_to_mean_variance(yields: &[f64]) -> (f64, f64) {
    if yields.is_empty() {
        return (0.0, 0.0);
    }
    let n = yields.len() as f64;
    let mean = yields.iter().sum::<f64>() / n;
    let variance = if yields.len() > 1 {
        yields.iter().map(|&y| (y - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    (mean, variance)
}

/// Convert replicate measurement data to a two-sample t-test p-value estimate.
///
/// Simplified: returns the t-statistic (caller compares to critical value).
/// `mean_a`, `std_a`, `n_a`: first sample stats.
/// `mean_b`, `std_b`, `n_b`: second sample stats.
#[must_use]
pub fn replicates_to_t_statistic(
    mean_a: f64,
    std_a: f64,
    n_a: usize,
    mean_b: f64,
    std_b: f64,
    n_b: usize,
) -> f64 {
    if n_a == 0 || n_b == 0 {
        return 0.0;
    }
    let se = ((std_a * std_a / n_a as f64) + (std_b * std_b / n_b as f64)).sqrt();
    if se < 1e-15 {
        return 0.0;
    }
    (mean_a - mean_b) / se
}

// ── Ushma bridges (thermodynamics) ─────────────────────────────────────────

/// Convert measurement uncertainty (±K) to Gaussian error model parameters.
///
/// Returns `(mean, std_dev)` assuming the uncertainty is a 95% confidence half-width.
#[must_use]
#[inline]
pub fn uncertainty_to_gaussian(measured_value: f64, uncertainty_k: f64) -> (f64, f64) {
    // 95% CI: uncertainty = 1.96σ → σ = uncertainty / 1.96
    let std_dev = uncertainty_k.abs() / 1.96;
    (measured_value, std_dev)
}

/// Convert sensor noise power spectral density (unit²/Hz) and bandwidth (Hz)
/// to total noise standard deviation.
///
/// σ = sqrt(PSD × BW)
#[must_use]
#[inline]
pub fn sensor_noise_to_std_dev(psd: f64, bandwidth_hz: f64) -> f64 {
    if psd <= 0.0 || bandwidth_hz <= 0.0 {
        return 0.0;
    }
    (psd * bandwidth_hz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_95() {
        let half_width = ensemble_spread_to_confidence(10.0, 1.96);
        assert!((half_width - 19.6).abs() < 0.01);
    }

    #[test]
    fn likelihood_at_mean() {
        let w = observation_error_to_likelihood(0.0, 1.0);
        // Should be 1/(sqrt(2π)) ≈ 0.3989
        assert!((w - 0.3989).abs() < 0.001);
    }

    #[test]
    fn likelihood_zero_std() {
        assert!((observation_error_to_likelihood(1.0, 0.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn yields_basic() {
        let (mean, var) = yields_to_mean_variance(&[10.0, 12.0, 11.0, 13.0]);
        assert!((mean - 11.5).abs() < 0.01);
        assert!(var > 0.0);
    }

    #[test]
    fn yields_empty() {
        let (m, v) = yields_to_mean_variance(&[]);
        assert!((m).abs() < f64::EPSILON);
        assert!((v).abs() < f64::EPSILON);
    }

    #[test]
    fn t_statistic_equal_means() {
        let t = replicates_to_t_statistic(10.0, 1.0, 30, 10.0, 1.0, 30);
        assert!(t.abs() < 0.01);
    }

    #[test]
    fn t_statistic_different_means() {
        let t = replicates_to_t_statistic(10.0, 1.0, 30, 12.0, 1.0, 30);
        assert!(t.abs() > 1.0);
    }

    #[test]
    fn uncertainty_gaussian() {
        let (mean, std) = uncertainty_to_gaussian(300.0, 2.0);
        assert!((mean - 300.0).abs() < f64::EPSILON);
        assert!((std - 2.0 / 1.96).abs() < 0.01);
    }

    #[test]
    fn sensor_noise() {
        // sqrt(1e-6 × 1000) = sqrt(0.001) ��� 0.0316
        let std = sensor_noise_to_std_dev(1e-6, 1000.0);
        assert!((std - 0.0316).abs() < 0.001);
    }

    #[test]
    fn sensor_noise_zero() {
        assert!((sensor_noise_to_std_dev(0.0, 1000.0)).abs() < f64::EPSILON);
    }
}
