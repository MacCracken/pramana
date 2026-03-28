//! Criterion benchmarks for pramana.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use pramana::bayesian;
use pramana::combinatorics;
use pramana::descriptive;
use pramana::distribution::{Distribution, Normal, Poisson};
use pramana::hypothesis;
use pramana::markov::MarkovChain;
use pramana::monte_carlo::{self, SimpleRng};
use pramana::regression;
use pramana::timeseries;

fn bench_normal_pdf_1000(c: &mut Criterion) {
    let normal = Normal::new(0.0, 1.0).unwrap();
    c.bench_function("distribution/normal_pdf_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let x = (i as f64 - 500.0) / 100.0;
                black_box(normal.pdf(x));
            }
        });
    });
}

fn bench_descriptive_stats_10000(c: &mut Criterion) {
    let data: Vec<f64> = (0..10_000).map(|i| (i as f64) * 0.1).collect();
    c.bench_function("descriptive/stats_10000", |b| {
        b.iter(|| {
            let _ = black_box(descriptive::mean(black_box(&data)));
            let _ = black_box(descriptive::variance(black_box(&data)));
            let _ = black_box(descriptive::std_dev(black_box(&data)));
        });
    });
}

fn bench_monte_carlo_pi_100000(c: &mut Criterion) {
    c.bench_function("monte_carlo/pi_100000", |b| {
        b.iter(|| {
            let mut rng = SimpleRng::new(42);
            black_box(monte_carlo::monte_carlo_pi(100_000, &mut rng).unwrap());
        });
    });
}

fn bench_markov_step_1000(c: &mut Criterion) {
    let matrix = vec![
        vec![0.7, 0.2, 0.1],
        vec![0.3, 0.4, 0.3],
        vec![0.2, 0.3, 0.5],
    ];
    c.bench_function("markov/step_1000", |b| {
        b.iter(|| {
            let mut chain = MarkovChain::new(matrix.clone(), 0).unwrap();
            let mut rng = SimpleRng::new(42);
            for _ in 0..1000 {
                black_box(chain.step(&mut rng));
            }
        });
    });
}

fn bench_hypothesis_t_test_1000(c: &mut Criterion) {
    let data: Vec<f64> = (0..1000).map(|i| (i as f64) * 0.01 + 5.0).collect();
    c.bench_function("hypothesis/t_test_one_sample_1000", |b| {
        b.iter(|| {
            black_box(hypothesis::t_test_one_sample(black_box(&data), 5.0, 0.05).unwrap());
        });
    });
}

fn bench_regression_1000(c: &mut Criterion) {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&xi| 2.5 * xi + 1.3).collect();
    c.bench_function("regression/linear_1000", |b| {
        b.iter(|| {
            black_box(regression::linear_regression(black_box(&x), black_box(&y)).unwrap());
        });
    });
}

fn bench_bayesian_naive_bayes(c: &mut Criterion) {
    let features = vec![0.5; 10];
    let priors = vec![0.25; 4];
    let likelihoods: Vec<Vec<f64>> = (0..4).map(|c| vec![0.1 * (c + 1) as f64; 10]).collect();
    c.bench_function("bayesian/naive_bayes_4class_10feat", |b| {
        b.iter(|| {
            black_box(
                bayesian::naive_bayes_classify(
                    black_box(&features),
                    black_box(&priors),
                    black_box(&likelihoods),
                )
                .unwrap(),
            );
        });
    });
}

fn bench_combinatorics(c: &mut Criterion) {
    c.bench_function("combinatorics/combinations_30_15", |b| {
        b.iter(|| {
            black_box(combinatorics::combinations(30, 15).unwrap());
        });
    });
}

fn bench_timeseries_moving_average_10000(c: &mut Criterion) {
    let data: Vec<f64> = (0..10_000).map(|i| (i as f64).sin()).collect();
    c.bench_function("timeseries/moving_average_10000_w50", |b| {
        b.iter(|| {
            black_box(timeseries::moving_average(black_box(&data), 50).unwrap());
        });
    });
}

fn bench_poisson_large_lambda_sample(c: &mut Criterion) {
    let p = Poisson::new(100.0).unwrap();
    c.bench_function("distribution/poisson_sample_lambda100", |b| {
        b.iter(|| {
            let mut rng = SimpleRng::new(42);
            for _ in 0..1000 {
                black_box(p.sample(&mut rng));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_normal_pdf_1000,
    bench_descriptive_stats_10000,
    bench_monte_carlo_pi_100000,
    bench_markov_step_1000,
    bench_hypothesis_t_test_1000,
    bench_regression_1000,
    bench_bayesian_naive_bayes,
    bench_combinatorics,
    bench_timeseries_moving_average_10000,
    bench_poisson_large_lambda_sample,
);
criterion_main!(benches);
