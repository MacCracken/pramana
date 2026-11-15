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

// ── Bodh bridges (psychology) ──────────────────────────────────────────────

/// Convert psychometric item scores to Cronbach's alpha components.
///
/// Given per-item variances and total-score variance, returns the
/// alpha coefficient. `k` is the number of items.
///
/// `alpha = (k / (k-1)) × (1 − sum_item_var / total_var)`
#[must_use]
pub fn item_variances_to_alpha(k: usize, sum_item_variance: f64, total_variance: f64) -> f64 {
    if k < 2 || total_variance < 1e-15 {
        return 0.0;
    }
    let kf = k as f64;
    (kf / (kf - 1.0)) * (1.0 - sum_item_variance / total_variance)
}

/// Convert signal detection rates to d-prime components.
///
/// Given z-scored hit rate and false alarm rate, returns `d' = z_hit - z_fa`.
/// Caller is responsible for probit transformation.
#[must_use]
#[inline]
pub fn z_rates_to_d_prime(z_hit: f64, z_false_alarm: f64) -> f64 {
    z_hit - z_false_alarm
}

/// Convert paired trait scores to a correlation-ready format.
///
/// Given two equal-length slices of trait scores (e.g., anxiety and
/// depression), returns `(mean_a, mean_b, covariance, var_a, var_b)`.
/// Returns zeros if slices are empty or different lengths.
#[must_use]
pub fn trait_scores_to_correlation(a: &[f64], b: &[f64]) -> (f64, f64, f64, f64, f64) {
    if a.is_empty() || a.len() != b.len() {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    let n = a.len() as f64;
    let mean_a = a.iter().sum::<f64>() / n;
    let mean_b = b.iter().sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for i in 0..a.len() {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }
    if n > 1.0 {
        let denom = n - 1.0;
        (mean_a, mean_b, cov / denom, var_a / denom, var_b / denom)
    } else {
        (mean_a, mean_b, 0.0, 0.0, 0.0)
    }
}

/// Convert learning trial data (trial number, response time) to
/// power-law fit parameters.
///
/// Returns `(a, b)` where `T = a × N^(-b)`, estimated via log-log
/// linear regression. Points with `trial == 0` or a non-positive or non-finite
/// time are skipped. Returns `(0, 0)` if fewer than 2 usable points remain or
/// all usable trials are equal.
#[must_use]
pub fn learning_trials_to_power_law(trials: &[(u32, f64)]) -> (f64, f64) {
    // Log-log transform: ln(T) = ln(a) - b × ln(N)
    let points = trials
        .iter()
        .filter(|&&(trial, time)| trial > 0 && time > 0.0 && time.is_finite())
        .map(|&(trial, time)| (f64::from(trial).ln(), time.ln()));
    match fit_line(points) {
        Some((slope, intercept)) => (intercept.exp(), -slope),
        None => (0.0, 0.0),
    }
}

/// Least-squares line through `points`, returning `(slope, intercept)`.
///
/// Uses centered sums over the points actually supplied; `None` if fewer than
/// 2 points or zero spread in x.
fn fit_line(points: impl Iterator<Item = (f64, f64)> + Clone) -> Option<(f64, f64)> {
    let (count, sum_x, sum_y) = points
        .clone()
        .fold((0usize, 0.0, 0.0), |(c, sx, sy), (x, y)| {
            (c + 1, sx + x, sy + y)
        });
    if count < 2 {
        return None;
    }
    let n = count as f64;
    let (mean_x, mean_y) = (sum_x / n, sum_y / n);
    let (sxx, sxy) = points.fold((0.0, 0.0), |(sxx, sxy), (x, y)| {
        let dx = x - mean_x;
        (sxx + dx * dx, sxy + dx * (y - mean_y))
    });
    if !(sxx > 1e-15 && sxx.is_finite()) {
        return None;
    }
    let slope = sxy / sxx;
    Some((slope, mean_y - slope * mean_x))
}

/// Convert response time distribution parameters to Hick's law coefficients.
///
/// Given mean response times for different numbers of choices,
/// returns `(a, b)` where `RT = a + b × log2(n)`.
/// Input: slice of `(num_choices, mean_rt)` pairs. Points with zero choices or
/// a non-finite response time are skipped. Returns `(0, 0)` if fewer than 2
/// usable points remain or all usable choice counts are equal.
#[must_use]
pub fn response_times_to_hick_params(data: &[(usize, f64)]) -> (f64, f64) {
    let points = data
        .iter()
        .filter(|&&(choices, rt)| choices > 0 && rt.is_finite())
        .map(|&(choices, rt)| ((choices as f64).log2(), rt));
    match fit_line(points) {
        Some((b, a)) => (a, b),
        None => (0.0, 0.0),
    }
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

    // ── Bodh bridges ──

    #[test]
    fn alpha_high_reliability() {
        // 3 items, item variances sum to 0.1, total variance = 10.0
        // alpha = (3/2) × (1 − 0.1/10) = 1.5 × 0.99 = 1.485
        let alpha = item_variances_to_alpha(3, 0.1, 10.0);
        assert!((alpha - 1.485).abs() < 0.001);
    }

    #[test]
    fn alpha_single_item() {
        assert!((item_variances_to_alpha(1, 1.0, 2.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn d_prime_from_z() {
        let d = z_rates_to_d_prime(1.28, -1.28);
        assert!((d - 2.56).abs() < 0.01);
    }

    #[test]
    fn trait_correlation_basic() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [2.0, 4.0, 6.0, 8.0, 10.0];
        let (ma, mb, cov, va, vb) = trait_scores_to_correlation(&a, &b);
        assert!((ma - 3.0).abs() < 1e-10);
        assert!((mb - 6.0).abs() < 1e-10);
        assert!(cov > 0.0); // positive covariance
        assert!(va > 0.0);
        assert!(vb > 0.0);
    }

    #[test]
    fn trait_correlation_empty() {
        let (m, _, _, _, _) = trait_scores_to_correlation(&[], &[]);
        assert!(m.abs() < f64::EPSILON);
    }

    #[test]
    fn learning_power_law_fit() {
        // Generate perfect power law data: T = 10 × N^(-0.3)
        let trials: Vec<(u32, f64)> = (1..=10)
            .map(|n| (n, 10.0 * (n as f64).powf(-0.3)))
            .collect();
        let (a, b) = learning_trials_to_power_law(&trials);
        assert!((a - 10.0).abs() < 0.5);
        assert!((b - 0.3).abs() < 0.05);
    }

    #[test]
    fn hick_params_fit() {
        // Generate perfect Hick's data: RT = 0.2 + 0.1 × log2(n)
        let data: Vec<(usize, f64)> = vec![
            (2, 0.2 + 0.1 * 1.0),
            (4, 0.2 + 0.1 * 2.0),
            (8, 0.2 + 0.1 * 3.0),
        ];
        let (a, b) = response_times_to_hick_params(&data);
        assert!((a - 0.2).abs() < 0.01);
        assert!((b - 0.1).abs() < 0.01);
    }

    #[test]
    fn power_law_skipped_points_do_not_bias_fit() {
        // T = 10 · N^-1 on the valid points; (0, 5.0) and (3, -1.0) are skipped.
        // The old code divided by the total slice length (including skipped
        // points) and returned (3.51, 0.094).
        let (a, b) =
            learning_trials_to_power_law(&[(0, 5.0), (1, 10.0), (2, 5.0), (3, -1.0), (4, 2.5)]);
        assert!((a - 10.0).abs() < 1e-9, "a = {a}");
        assert!((b - 1.0).abs() < 1e-12, "b = {b}");
        // Only one usable point.
        assert_eq!(
            learning_trials_to_power_law(&[(0, 1.0), (2, 3.0)]),
            (0.0, 0.0)
        );
    }

    #[test]
    fn hick_skipped_points_do_not_bias_fit() {
        // RT = 200 + 150·log2(n) on the valid points.
        let (a, b) = response_times_to_hick_params(&[
            (0, 999.0),
            (1, 200.0),
            (2, 350.0),
            (4, 500.0),
            (8, f64::NAN),
        ]);
        assert!((a - 200.0).abs() < 1e-9, "a = {a}");
        assert!((b - 150.0).abs() < 1e-9, "b = {b}");
    }
}
