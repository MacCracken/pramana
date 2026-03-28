//! Criterion benchmarks for pramana.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use pramana::descriptive;
use pramana::distribution::{Distribution, Normal};
use pramana::markov::MarkovChain;
use pramana::monte_carlo::{self, SimpleRng};

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

criterion_group!(
    benches,
    bench_normal_pdf_1000,
    bench_descriptive_stats_10000,
    bench_monte_carlo_pi_100000,
    bench_markov_step_1000
);
criterion_main!(benches);
