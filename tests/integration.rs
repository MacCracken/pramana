//! Integration tests for pramana.

use pramana::bayesian;
use pramana::combinatorics;
use pramana::descriptive;
use pramana::distribution::{Distribution, Normal};
use pramana::markov::MarkovChain;
use pramana::monte_carlo::{self, SimpleRng};
use pramana::regression;

#[test]
fn normal_pdf_at_mean() {
    let n = Normal::new(0.0, 1.0).unwrap();
    let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
    assert!(
        (n.pdf(0.0) - expected).abs() < 1e-10,
        "Normal pdf at mean should be 1/(sigma*sqrt(2*pi))"
    );
}

#[test]
fn normal_cdf_at_mean() {
    let n = Normal::new(5.0, 2.0).unwrap();
    assert!(
        (n.cdf(5.0) - 0.5).abs() < 1e-6,
        "CDF at mean should be approximately 0.5"
    );
}

#[test]
fn mean_of_1_to_5() {
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let m = descriptive::mean(&data).unwrap();
    assert!((m - 3.0).abs() < 1e-10, "mean of [1..5] = 3.0");
}

#[test]
fn median_of_1_to_5() {
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let med = descriptive::median(&data).unwrap();
    assert!((med - 3.0).abs() < 1e-10, "median of [1..5] = 3.0");
}

#[test]
fn linear_regression_recovers_line() {
    // y = 2x + 1
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [3.0, 5.0, 7.0, 9.0, 11.0];
    let model = regression::linear_regression(&x, &y).unwrap();
    assert!(
        (model.slope - 2.0).abs() < 1e-10,
        "slope should be ~2, got {}",
        model.slope
    );
    assert!(
        (model.intercept - 1.0).abs() < 1e-10,
        "intercept should be ~1, got {}",
        model.intercept
    );
    assert!(
        (model.r_squared - 1.0).abs() < 1e-10,
        "R^2 should be ~1 for perfect fit"
    );
}

#[test]
fn factorial_10() {
    assert_eq!(combinatorics::factorial(10).unwrap(), 3_628_800);
}

#[test]
fn combinations_10_choose_3() {
    assert_eq!(combinatorics::combinations(10, 3).unwrap(), 120);
}

#[test]
fn bayes_theorem_basic() {
    // P(A|B) = P(B|A) * P(A) / P(B)
    let prior = 0.01;
    let likelihood = 0.9;
    let evidence = 0.1;
    let posterior = bayesian::bayes_theorem(prior, likelihood, evidence).unwrap();
    let expected = likelihood * prior / evidence;
    assert!(
        (posterior - expected).abs() < 1e-10,
        "Bayes theorem: P(A|B) = P(B|A)*P(A)/P(B)"
    );
}

#[test]
fn markov_chain_rows_must_sum_to_1() {
    let bad_matrix = vec![vec![0.5, 0.3], vec![0.4, 0.6]];
    assert!(
        MarkovChain::new(bad_matrix, 0).is_err(),
        "rows not summing to 1.0 should be rejected"
    );

    let good_matrix = vec![vec![0.7, 0.3], vec![0.4, 0.6]];
    assert!(
        MarkovChain::new(good_matrix, 0).is_ok(),
        "valid matrix should be accepted"
    );
}

#[test]
fn serde_roundtrip_normal() {
    let n = Normal::new(2.5, 1.3).unwrap();
    let json = serde_json::to_string(&n).unwrap();
    let n2: Normal = serde_json::from_str(&json).unwrap();
    assert_eq!(n.mean, n2.mean);
    assert_eq!(n.std_dev, n2.std_dev);
}

#[test]
fn serde_roundtrip_markov() {
    let matrix = vec![vec![0.6, 0.4], vec![0.3, 0.7]];
    let chain = MarkovChain::new(matrix.clone(), 0).unwrap();
    let json = serde_json::to_string(&chain).unwrap();
    let chain2: MarkovChain = serde_json::from_str(&json).unwrap();
    assert_eq!(chain.transition_matrix, chain2.transition_matrix);
    assert_eq!(chain.state, chain2.state);
}

#[test]
fn monte_carlo_pi_accuracy() {
    let mut rng = SimpleRng::new(42);
    let pi = monte_carlo::monte_carlo_pi(100_000, &mut rng).unwrap();
    assert!(
        (pi - std::f64::consts::PI).abs() < 0.1,
        "MC pi estimate {pi} should be within 0.1 of pi"
    );
}

#[test]
fn normal_sample_distribution() {
    // Draw many samples from N(0,1) and verify the sample mean is close to 0
    let n = Normal::new(0.0, 1.0).unwrap();
    let mut rng = SimpleRng::new(12345);
    let samples: Vec<f64> = (0..10_000).map(|_| n.sample(&mut rng)).collect();
    let m = descriptive::mean(&samples).unwrap();
    assert!(
        m.abs() < 0.1,
        "sample mean of N(0,1) should be close to 0, got {m}"
    );
}

#[test]
fn descriptive_empty_errors() {
    assert!(descriptive::mean(&[]).is_err());
    assert!(descriptive::median(&[]).is_err());
    assert!(descriptive::variance(&[]).is_err());
    assert!(descriptive::std_dev(&[]).is_err());
}

#[test]
fn combinatorics_edge_cases() {
    assert_eq!(combinatorics::factorial(0).unwrap(), 1);
    assert_eq!(combinatorics::factorial(1).unwrap(), 1);
    assert_eq!(combinatorics::combinations(5, 0).unwrap(), 1);
    assert_eq!(combinatorics::combinations(5, 5).unwrap(), 1);
    assert_eq!(combinatorics::permutations(5, 0).unwrap(), 1);
}
