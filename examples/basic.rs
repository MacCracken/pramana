//! Basic usage of pramana: compute descriptive statistics and sample from a distribution.

use pramana::descriptive;
use pramana::distribution::{Distribution, Normal};
use pramana::rng::SimpleRng;

fn main() {
    // Dataset
    let data = [2.3, 4.1, 3.7, 5.2, 4.8, 3.1, 4.5, 3.9, 5.0, 4.2];

    // Descriptive statistics
    let m = descriptive::mean(&data).expect("non-empty data");
    let s = descriptive::std_dev(&data).expect("non-empty data");
    let med = descriptive::median(&data).expect("non-empty data");
    let v = descriptive::variance(&data).expect("non-empty data");

    println!("Dataset: {data:?}");
    println!("Mean:     {m:.4}");
    println!("Std Dev:  {s:.4}");
    println!("Variance: {v:.4}");
    println!("Median:   {med:.4}");

    // Fit a normal distribution to the data
    let normal = Normal::new(m, s).expect("valid parameters");
    println!(
        "\nFitted Normal(mu={:.4}, sigma={:.4})",
        normal.mean, normal.std_dev
    );
    println!("  PDF at mean: {:.6}", normal.pdf(m));
    println!("  CDF at mean: {:.6}", normal.cdf(m));
    println!("  Variance:    {:.6}", normal.variance());

    // Sample from the fitted distribution
    let mut rng = SimpleRng::new(42);
    let samples: Vec<f64> = (0..10).map(|_| normal.sample(&mut rng)).collect();
    println!("\n10 samples from the fitted distribution:");
    for (i, s) in samples.iter().enumerate() {
        println!("  [{i}] {s:.4}");
    }
}
