//! The `oracle` hooks are read-only and run the crate's own code paths (port plan M0 step 0).
//!
//! Each test proves one of two things:
//! - a hook gives, bit for bit, what the public API computes through the same private code;
//! - a hook's sampler draws exactly the uniforms the public sampler draws, so the
//!   golden-value generator can record draw counts with this file's `Counting` wrapper.
#![cfg(feature = "oracle")]

use pramana::descriptive::{Kernel, KernelDensity};
use pramana::distribution::{
    Beta, ChiSquared, Distribution, FDistribution, Gamma, MultivariateNormal, Normal, Poisson,
    StudentT,
};
use pramana::markov::HiddenMarkovModel;
use pramana::{Rng, SimpleRng, hypothesis, oracle};
use std::f64::consts::PI;

/// Counts the uniforms a sampler draws.
struct Counting {
    inner: SimpleRng,
    draws: u64,
}

impl Counting {
    fn new(seed: u64) -> Self {
        Self {
            inner: SimpleRng::new(seed),
            draws: 0,
        }
    }
}

impl Rng for Counting {
    fn next_f64(&mut self) -> f64 {
        self.draws += 1;
        self.inner.next_f64()
    }
}

/// `n` seeded points in `[lo, hi)`.
fn points(seed: u64, lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let mut rng = SimpleRng::new(seed);
    (0..n).map(|_| lo + (hi - lo) * rng.next_f64()).collect()
}

fn assert_bits(a: f64, b: f64, what: &str) {
    assert_eq!(a.to_bits(), b.to_bits(), "{what}: {a:e} vs {b:e}");
}

#[test]
fn normal_cdf_and_sf_are_the_public_cdf() {
    let normal = Normal::new(0.0, 1.0).unwrap();
    for z in points(1, -40.0, 10.0, 4_000) {
        assert_bits(normal.cdf(z), oracle::normal_cdf(z), "normal_cdf");
        // normal_cdf(−z) evaluates ½·erfc(−(−z)/√2) = ½·erfc(z/√2) = normal_sf(z).
        assert_bits(normal.cdf(-z), oracle::normal_sf(z), "normal_sf");
    }
    // erf has no public path: check it is wired to erf, not erfc.
    assert_bits(oracle::erf(0.0), 0.0, "erf(0)");
    assert!((oracle::erf(1.0) - 0.842_700_792_949_714_9).abs() <= f64::EPSILON);
    assert!((oracle::erfc(1.0) - 0.157_299_207_050_285_13).abs() <= 1e-17);
}

#[test]
fn incomplete_gamma_hooks_are_the_gamma_and_chi_squared_cdfs() {
    for (i, a) in [0.05, 0.5, 1.0, 2.5, 30.0, 400.0, 1e5]
        .into_iter()
        .enumerate()
    {
        let gamma = Gamma::new(a, 1.0).unwrap();
        for x in points(10 + i as u64, 1e-3, 3.0 * a + 10.0, 200) {
            let (p, q) = oracle::gamma_inc(a, x);
            assert_bits(gamma.cdf(x), p, "Gamma cdf = P");
            assert_bits(oracle::regularized_lower_incomplete_gamma(a, x), p, "P");
            assert_bits(oracle::regularized_upper_incomplete_gamma(a, x), q, "Q");
        }
    }
    // χ²(df).cdf(x) = P(df/2, x/2), and the test's p-value is Q(df/2, χ²/2).
    let chi = ChiSquared::new(7.0).unwrap();
    for x in points(20, 0.01, 40.0, 200) {
        assert_bits(chi.cdf(x), oracle::gamma_inc(3.5, x / 2.0).0, "chi2 cdf");
    }
    let observed = [18.0, 22.0, 30.0, 10.0, 20.0];
    let expected = [20.0; 5];
    let t = hypothesis::chi_squared_test(&observed, &expected, 0.05).unwrap();
    assert_bits(
        t.p_value,
        oracle::chi_squared_pvalue(t.statistic, t.degrees_of_freedom),
        "chi-squared p",
    );
}

#[test]
fn incomplete_beta_and_t_and_f_hooks_are_the_public_cdfs() {
    for (i, (a, b)) in [(0.5, 0.5), (2.0, 5.0), (30.0, 0.7), (400.0, 350.0)]
        .into_iter()
        .enumerate()
    {
        let beta = Beta::new(a, b).unwrap();
        for x in points(30 + i as u64, 1e-6, 1.0, 200) {
            let i_x = oracle::regularized_incomplete_beta(x, a, b);
            assert_bits(beta.cdf(x), i_x, "Beta cdf");
            assert_bits(oracle::beta_inc(a, b, x, 1.0 - x).0, i_x, "beta_inc");
        }
    }
    for df in [1.0, 3.5, 30.0, 1e6] {
        let t = StudentT::new(df).unwrap();
        for x in points(40, -50.0, 50.0, 200) {
            assert_bits(t.cdf(x), oracle::student_t_cdf(x, df), "t cdf");
        }
    }
    let f = FDistribution::new(4.0, 9.0).unwrap();
    for x in points(50, 1e-3, 30.0, 200) {
        assert_bits(f.cdf(x), oracle::f_cdf_sf(x, 4.0, 9.0).0, "F cdf");
    }
}

#[test]
fn ln_gamma_is_the_one_the_gamma_pdf_uses() {
    // Gamma::pdf is exp((α−1)·ln x − x/β − α·ln β − lnΓ(α)), in this order.
    for (alpha, beta) in [(0.7, 1.3), (5.0, 2.0), (120.0, 0.25)] {
        let gamma = Gamma::new(alpha, beta).unwrap();
        for x in points(60, 1e-3, 3.0 * alpha * beta, 200) {
            let expected =
                ((alpha - 1.0) * x.ln() - x / beta - alpha * beta.ln() - oracle::ln_gamma(alpha))
                    .exp();
            assert_bits(gamma.pdf(x), expected, "Gamma pdf via ln_gamma");
        }
    }
    // The other kernels: wiring checks against their definitions. The direct Stirling
    // remainder cancels lnΓ(a) against its leading terms, so its error scales with |lnΓ(a)|.
    for a in [10.0, 37.5, 1e4] {
        let ln_g = oracle::ln_gamma(a);
        let direct = ln_g - ((a - 0.5) * f64::ln(a) - a + 0.5 * (2.0 * PI).ln());
        let tolerance = 1e-13 * ln_g.abs().max(1.0);
        assert!(
            (oracle::stirling_error(a) - direct).abs() < tolerance,
            "stirling_error({a})"
        );
    }
    assert_bits(oracle::bd0(7.0, 7.0), 0.0, "bd0(x, x)");
    let (a, b) = (2.5, 4.0);
    let naive = oracle::ln_gamma(a) + oracle::ln_gamma(b) - oracle::ln_gamma(a + b);
    assert!((oracle::ln_beta(a, b) - naive).abs() < 1e-13, "ln_beta");
    let x = 3.0;
    let naive = a * f64::ln(x) - x - oracle::ln_gamma(a);
    assert!(
        (oracle::ln_gamma_kernel(a, x) - naive).abs() < 1e-13,
        "ln_gamma_kernel"
    );
}

#[test]
fn sampler_hooks_draw_what_the_public_samplers_draw() {
    // Normal(0, 1) is 0 + 1·z for z = standard_normal: the same bits.
    let normal = Normal::new(0.0, 1.0).unwrap();
    let (mut r1, mut r2) = (Counting::new(7), Counting::new(7));
    for _ in 0..10_000 {
        assert_bits(
            normal.sample(&mut r1),
            oracle::standard_normal(&mut r2),
            "normal",
        );
    }
    assert_eq!((r1.draws, r2.draws), (20_000, 20_000));

    // Poisson above λ = 30 is PTRS.
    for lambda in [31.0, 100.0, 1e4, 1e8] {
        let poisson = Poisson::new(lambda).unwrap();
        let (mut r1, mut r2) = (Counting::new(11), Counting::new(11));
        for _ in 0..5_000 {
            let draw = poisson.sample(&mut r1);
            assert_bits(draw, oracle::sample_poisson_ptrs(lambda, &mut r2), "PTRS");
        }
        assert_eq!(r1.draws, r2.draws, "PTRS draw count at λ = {lambda}");
    }

    // Gamma(α, 1) is sample_gamma_standard(α) · 1.
    for alpha in [0.3, 1.0, 2.5, 50.0] {
        let gamma = Gamma::new(alpha, 1.0).unwrap();
        let (mut r1, mut r2) = (Counting::new(13), Counting::new(13));
        for _ in 0..5_000 {
            let draw = gamma.sample(&mut r1);
            assert_bits(draw, oracle::sample_gamma_standard(alpha, &mut r2), "gamma");
        }
        assert_eq!(r1.draws, r2.draws, "gamma draw count at α = {alpha}");
    }
}

#[test]
fn sampler_guards_refuse_without_drawing() {
    let mut rng = Counting::new(3);
    for lambda in [f64::NAN, f64::INFINITY, 9.99, 0.0, -5.0] {
        assert!(oracle::sample_poisson_ptrs(lambda, &mut rng).is_nan());
    }
    for alpha in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert!(oracle::sample_gamma_standard(alpha, &mut rng).is_nan());
    }
    assert_eq!(rng.draws, 0);
}

#[test]
fn hypothesis_hooks_are_the_tests_internals() {
    let a = points(70, 4.0, 9.0, 23);
    let b = points(71, 5.0, 12.0, 31);

    // Welch: the statistic is diff/se, and the p-value the two-tailed t at Welch's df.
    let (diff, se, df) = oracle::welch_stats(&a, &b).unwrap();
    let t = hypothesis::t_test_two_sample(&a, &b, 0.05).unwrap();
    assert_bits(t.statistic, diff / se, "Welch t");
    assert_bits(t.degrees_of_freedom, df, "Welch df");
    assert_bits(
        t.p_value,
        oracle::student_t_two_tailed(diff / se, df),
        "Welch p",
    );
    assert!(oracle::welch_stats(&a[..1], &b).is_err());

    // ci_two_means: diff ∓ t_quantile(1 − α/2, df)·se.
    let ci = hypothesis::ci_two_means(&a, &b, 0.95).unwrap();
    let alpha = 1.0 - 0.95;
    let margin = oracle::t_quantile(1.0 - alpha / 2.0, df) * se;
    assert_bits(ci.lower, diff - margin, "ci_two_means lower");
    assert_bits(ci.upper, diff + margin, "ci_two_means upper");

    // ci_proportion: p̂ ± z_quantile(1 − α/2)·√(p̂(1 − p̂)/n), clamped to [0, 1].
    let (successes, n) = (37_u64, 120_u64);
    let ci = hypothesis::ci_proportion(successes, n, 0.9).unwrap();
    let p_hat = successes as f64 / n as f64;
    let alpha = 1.0 - 0.9;
    let z = oracle::z_quantile(1.0 - alpha / 2.0);
    let se = (p_hat * (1.0 - p_hat) / n as f64).sqrt();
    assert_bits(ci.lower, (p_hat - z * se).max(0.0), "ci_proportion lower");
    assert_bits(ci.upper, (p_hat + z * se).min(1.0), "ci_proportion upper");

    // KS: p = ks_pvalue(D, n·m/(n + m)) = Q_KS((√nₑ + 0.12 + 0.11/√nₑ)·D).
    let ks = hypothesis::ks_two_sample(&a, &b, 0.05).unwrap();
    let (na, nb) = (a.len() as f64, b.len() as f64);
    let n_eff = (na * nb) / (na + nb);
    assert_bits(ks.p_value, oracle::ks_pvalue(ks.statistic, n_eff), "KS p");
    let sqrt_n = n_eff.sqrt();
    let lambda = (sqrt_n + 0.12 + 0.11 / sqrt_n) * ks.statistic;
    assert_bits(ks.p_value, oracle::kolmogorov_sf(lambda), "Q_KS");
}

#[test]
fn quantile_hooks_reproduce_the_frozen_1_3_0_bits() {
    // A 1-ulp change in a quantile can vanish in a confidence interval's rounding, so the
    // quantiles are also pinned to the bits Rust 1.3.0 computes (recorded 2026-11-15, rustc
    // 1.100.0, glibc 2.44). The crate is frozen, so these never change.
    let z_rows: [(f64, u64); 6] = [
        (0.95, 0x3ffa_5152_0967_6abd),
        (0.975, 0x3fff_5c03_31ee_ff83),
        (0.995, 0x4004_9b4c_64d6_9166),
        (1e-10, 0xc019_7203_597a_2155),
        (0.500_000_001, 0x3e25_8822_3b6c_f6d4),
        (0.999_999, 0x4013_0381_a979_b412),
    ];
    for (p, bits) in z_rows {
        assert_bits(oracle::z_quantile(p), f64::from_bits(bits), "z_quantile");
    }
    let t_rows: [(f64, f64, u64); 4] = [
        (0.975, 3.0, 0x4009_75a6_6893_c680),
        (0.995, 29.5, 0x4006_066a_bc6a_3a40),
        (0.9, 1.0, 0x4008_9f18_8bdc_d880),
        (1e-6, 7.0, 0xc02c_7ba1_e904_7180),
    ];
    for (p, df, bits) in t_rows {
        assert_bits(
            oracle::t_quantile(p, df),
            f64::from_bits(bits),
            "t_quantile",
        );
    }
}

fn weather_hmm() -> HiddenMarkovModel {
    HiddenMarkovModel::new(
        vec![vec![0.7, 0.3], vec![0.4, 0.6]],
        vec![vec![0.1, 0.4, 0.5], vec![0.6, 0.3, 0.1]],
        vec![0.6, 0.4],
    )
    .unwrap()
}

#[test]
fn hmm_forward_hook_is_the_likelihood_path_and_changes_nothing() {
    let hmm = weather_hmm();
    let mut rng = SimpleRng::new(17);
    let observations: Vec<usize> = (0..2_000)
        .map(|_| (rng.next_f64() * 3.0) as usize % 3)
        .collect();

    let (alpha, scales) = oracle::hmm_forward_scaled(&hmm, &observations).unwrap();
    assert_eq!((alpha.len(), scales.len()), (2_000, 2_000));
    assert_bits(
        hmm.forward_log_likelihood(&observations).unwrap(),
        oracle::hmm_log_likelihood(&scales),
        "HMM log-likelihood",
    );
    assert!(oracle::hmm_forward_scaled(&hmm, &[]).is_err());
    assert!(oracle::hmm_forward_scaled(&hmm, &[0, 3]).is_err());

    // Read-only: a fit after the hooks equals a fit without them.
    let plain =
        serde_json::to_string(&weather_hmm().baum_welch(&observations, 25).unwrap()).unwrap();
    let hooked_model = weather_hmm();
    let _ = oracle::hmm_forward_scaled(&hooked_model, &observations).unwrap();
    let fitted = hooked_model.baum_welch(&observations, 25).unwrap();
    let _ = oracle::hmm_forward_scaled(&fitted, &observations).unwrap();
    assert_eq!(plain, serde_json::to_string(&fitted).unwrap());
    assert_eq!(
        serde_json::to_string(&weather_hmm()).unwrap(),
        serde_json::to_string(&hooked_model).unwrap()
    );
}

#[test]
fn mvn_factor_is_the_stored_cholesky_and_reading_it_changes_nothing() {
    let covariance = vec![
        vec![4.0, 1.2, 0.3],
        vec![1.2, 2.5, -0.4],
        vec![0.3, -0.4, 1.1],
    ];
    let mvn = MultivariateNormal::new(vec![1.0, -2.0, 0.5], covariance.clone()).unwrap();
    let (l, log_det) = oracle::mvn_factor(&mvn);
    assert_eq!(
        l,
        pramana::hisab::num::cholesky(&covariance)
            .unwrap()
            .as_slice()
    );
    let expected: f64 = l
        .iter()
        .enumerate()
        .map(|(i, row)| row[i].ln())
        .sum::<f64>()
        * 2.0;
    assert_bits(log_det, expected, "log det");

    // Read-only: the same draws with the factor read between them.
    let (mut r1, mut r2) = (SimpleRng::new(5), SimpleRng::new(5));
    for _ in 0..2_000 {
        let plain = mvn.sample(&mut r1);
        let _ = oracle::mvn_factor(&mvn);
        let hooked = mvn.sample(&mut r2);
        assert_eq!(
            plain.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            hooked.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn kernel_hook_is_what_kde_sums() {
    // One point at 0 with bandwidth 1: evaluate(x) = K((x − 0)/1) / (1·1) = K(x).
    for kernel in [
        Kernel::Gaussian,
        Kernel::Epanechnikov,
        Kernel::Uniform,
        Kernel::Triangular,
    ] {
        let kde = KernelDensity::new(vec![0.0], 1.0, kernel).unwrap();
        for x in points(80, -3.0, 3.0, 500) {
            assert_bits(
                kde.evaluate(x),
                oracle::kernel_evaluate(kernel, x),
                "kernel",
            );
        }
    }
}
