//! M3: the Rust twin of tests/dist.bcyr. 1.3.0's distributions (on glibc 2.44's primitives) on the same seeded
//! arguments, regions and loop: 4,096 values (or pairs, drawn alternately) per row, cycled 250 times, ns per call,
//! each row's sum as bits, and an FNV-1a digest of its 4,096 results' bits. A draw row takes its uniforms from one
//! SimpleRng seeded 42, and its digest from a fresh one. Build with `--features roundtrip`, as the golden corpus reads
//! JSON (serde_json's `float_roundtrip`, a correctly rounded parse); its default parse can miss by an ulp, which the
//! JSON row's digest shows. Run pinned, interleaved with `cyrius bench tests/dist.bcyr`:
//!   taskset -c 5 cargo run --release --locked --features roundtrip --bin m3_dist

use pramana::distribution::{
    Bernoulli, Beta, Binomial, Cauchy, ChiSquared, Distribution, Exponential, FDistribution, Gamma, MultivariateNormal,
    Normal, Poisson, StudentT, Uniform, Weibull,
};
use pramana::{SimpleRng, oracle};
use std::hint::black_box;
use std::sync::LazyLock;
use std::time::Instant;

/// Criterion's 3-D covariance (rust-old/benches), each entry times s.
fn cov3(s: f64) -> Vec<Vec<f64>> {
    vec![vec![1.0 * s, 0.3 * s, 0.1 * s], vec![0.3 * s, 2.0 * s, 0.5 * s], vec![0.1 * s, 0.5 * s, 3.0 * s]]
}

/// Criterion's 3-D MultivariateNormal, and a d = 16 one (1/(1 + |i − j|), plus 16 on the diagonal), built once.
static M3: LazyLock<MultivariateNormal> = LazyLock::new(|| MultivariateNormal::new(vec![0.0; 3], cov3(1.0)).unwrap());
#[allow(clippy::cast_precision_loss)]
static M16: LazyLock<MultivariateNormal> = LazyLock::new(|| {
    let c = (0..16)
        .map(|i: usize| (0..16).map(|j: usize| 1.0 / (1 + i.abs_diff(j)) as f64 + if i == j { 16.0 } else { 0.0 }).collect())
        .collect();
    MultivariateNormal::new(vec![0.0; 16], c).unwrap()
});

struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545F4914F6CDD1D)) >> 11) as f64 / 9007199254740992.0
    }
}

fn row(
    r: &mut Rng,
    name: &str,
    f: fn(f64, f64, &mut SimpleRng) -> f64,
    (lo, hi): (f64, f64),
    (lo2, hi2): (f64, f64),
) {
    let xy: Vec<(f64, f64)> = (0..4096)
        .map(|_| {
            let x = lo + (hi - lo) * r.unit();
            (x, lo2 + (hi2 - lo2) * r.unit())
        })
        .collect();
    let mut g = SimpleRng::new(42);
    let mut s = 0.0;
    let t0 = Instant::now();
    for _ in 0..250 {
        for &(x, y) in &xy {
            s += f(black_box(x), black_box(y), &mut g);
        }
    }
    let ns = t0.elapsed().as_nanos() as f64 / 1_024_000.0;
    println!("  {name}, the sum: {:x}", s.to_bits());
    let mut g = SimpleRng::new(42);
    let mut dg: u64 = 0xcbf2_9ce4_8422_2325;
    for &(x, y) in &xy {
        dg = (dg ^ f(x, y, &mut g).to_bits()).wrapping_mul(0x0100_0000_01b3);
    }
    println!("  {name}, the digest: {dg:x}");
    println!("  {name}: {ns:.1} ns");
}

fn main() {
    let mut r = Rng(0x2545F4914F6CDD1D);
    let u = (0.0, 1.0);
    let sd = (0.1, 10.0);
    let wide = (-10.0, 10.0);
    row(&mut r, "Normal::pdf, N(0, 1), x in [-10, 10)", |x, _, _| Normal::new(0.0, 1.0).unwrap().pdf(x), wide, u);
    row(&mut r, "Normal::cdf, N(0, 1), x in [-10, 10)", |x, _, _| Normal::new(0.0, 1.0).unwrap().cdf(x), wide, u);
    row(&mut r, "Normal::cdf, N(0, 1), x in [-38, -1.2)", |x, _, _| Normal::new(0.0, 1.0).unwrap().cdf(x), (-38.0, -1.2), u);
    row(&mut r, "Normal::sample, mean in [-10, 10), sd in [0.1, 10)", |x, y, g| Normal::new(x, y).unwrap().sample(g), wide, sd);
    row(
        &mut r,
        "Normal, to JSON and back, mean in [-10, 10), sd in [0.1, 10)",
        |x, y, _| {
            let t = serde_json::to_string(&Normal::new(x, y).unwrap()).unwrap();
            serde_json::from_str::<Normal>(&t).unwrap().mean
        },
        wide,
        sd,
    );
    row(
        &mut r,
        "Normal, Debug, mean in [-10, 10), sd in [0.1, 10)",
        |x, y, _| format!("{:?}", Normal::new(x, y).unwrap()).len() as f64,
        wide,
        sd,
    );
    // part a2
    row(&mut r, "Uniform::cdf, U(-1, 5), x in [-2, 6)", |x, _, _| Uniform::new(-1.0, 5.0).unwrap().cdf(x), (-2.0, 6.0), u);
    row(&mut r, "Uniform::sample, min in [-10, 10), width in [0.1, 10)", |x, y, g| Uniform::new(x, x + y).unwrap().sample(g), wide, sd);
    row(&mut r, "Exponential::pdf, Exp(2.5), x in [0, 10)", |x, _, _| Exponential::new(2.5).unwrap().pdf(x), (0.0, 10.0), u);
    row(&mut r, "Exponential::cdf, Exp(2.5), x in [0, 10)", |x, _, _| Exponential::new(2.5).unwrap().cdf(x), (0.0, 10.0), u);
    row(&mut r, "Exponential::cdf, Exp(2.5), x in [0, 0.01)", |x, _, _| Exponential::new(2.5).unwrap().cdf(x), (0.0, 0.01), u);
    row(&mut r, "Exponential::sample, lambda in [0.1, 10)", |_, y, g| Exponential::new(y).unwrap().sample(g), u, sd);
    row(&mut r, "Bernoulli::cdf, p = 0.3, x in [-1, 2)", |x, _, _| Bernoulli::new(0.3).unwrap().cdf(x), (-1.0, 2.0), u);
    row(&mut r, "Bernoulli::sample, p in [0, 1)", |_, y, g| Bernoulli::new(y).unwrap().sample(g), u, u);
    // part a3
    row(&mut r, "Cauchy::pdf, Cauchy(0, 1), x in [-10, 10)", |x, _, _| Cauchy::new(0.0, 1.0).unwrap().pdf(x), wide, u);
    row(&mut r, "Cauchy::cdf, Cauchy(0, 1), x in [-10, 10)", |x, _, _| Cauchy::new(0.0, 1.0).unwrap().cdf(x), wide, u);
    row(&mut r, "Cauchy::sample, x0 in [-10, 10), gamma in [0.1, 10)", |x, y, g| Cauchy::new(x, y).unwrap().sample(g), wide, sd);
    row(&mut r, "Weibull::pdf, Weibull(2, 1), x in [0, 3)", |x, _, _| Weibull::new(2.0, 1.0).unwrap().pdf(x), (0.0, 3.0), u);
    row(&mut r, "Weibull::cdf, Weibull(2, 1), x in [0, 3)", |x, _, _| Weibull::new(2.0, 1.0).unwrap().cdf(x), (0.0, 3.0), u);
    row(&mut r, "Weibull::mean, k in [0.1, 10), lambda = 1", |_, y, _| Weibull::new(y, 1.0).unwrap().mean(), u, sd);
    row(&mut r, "Weibull::sample, lambda in [0.1, 10), k in [0.1, 10)", |x, y, g| Weibull::new(y, x).unwrap().sample(g), sd, sd);
    // part b1
    row(&mut r, "Gamma::pdf, Gamma(2.5, 2), x in [0, 20)", |x, _, _| Gamma::new(2.5, 2.0).unwrap().pdf(x), (0.0, 20.0), u);
    row(&mut r, "Gamma::cdf, Gamma(2.5, 2), x in [0, 20)", |x, _, _| Gamma::new(2.5, 2.0).unwrap().cdf(x), (0.0, 20.0), u);
    row(&mut r, "Gamma::sample, alpha in [0.1, 10), beta = 1", |_, y, g| Gamma::new(y, 1.0).unwrap().sample(g), u, sd);
    row(&mut r, "ChiSquared::cdf, chi2(5), x in [0, 20)", |x, _, _| ChiSquared::new(5.0).unwrap().cdf(x), (0.0, 20.0), u);
    row(&mut r, "ChiSquared::sample, df in [1, 30)", |_, y, g| ChiSquared::new(y).unwrap().sample(g), u, (1.0, 30.0));
    // part b2
    row(&mut r, "Beta::pdf, Beta(2, 5), x in [0, 1)", |x, _, _| Beta::new(2.0, 5.0).unwrap().pdf(x), u, u);
    row(&mut r, "Beta::cdf, Beta(2, 5), x in [0, 1)", |x, _, _| Beta::new(2.0, 5.0).unwrap().cdf(x), u, u);
    row(&mut r, "Beta::sample, alpha = beta in [0.5, 10)", |_, y, g| Beta::new(y, y).unwrap().sample(g), u, (0.5, 10.0));
    row(&mut r, "StudentT::pdf, t(5), x in [-10, 10)", |x, _, _| StudentT::new(5.0).unwrap().pdf(x), wide, u);
    row(&mut r, "StudentT::cdf, t(5), x in [-10, 10)", |x, _, _| StudentT::new(5.0).unwrap().cdf(x), wide, u);
    row(&mut r, "StudentT::sample, df in [1, 30)", |_, y, g| StudentT::new(y).unwrap().sample(g), u, (1.0, 30.0));
    row(&mut r, "FDistribution::cdf, F(4, 9), x in [0, 10)", |x, _, _| FDistribution::new(4.0, 9.0).unwrap().cdf(x), (0.0, 10.0), u);
    row(&mut r, "FDistribution::sample, d1 = d2 in [1, 30)", |_, y, g| FDistribution::new(y, y).unwrap().sample(g), u, (1.0, 30.0));
    // part c1
    row(&mut r, "Poisson::pdf, Poisson(4.5), k = floor(x), x in [0, 20)", |x, _, _| Poisson::new(4.5).unwrap().pdf(x.floor()), (0.0, 20.0), u);
    row(&mut r, "Poisson::cdf, Poisson(4.5), x in [0, 20)", |x, _, _| Poisson::new(4.5).unwrap().cdf(x), (0.0, 20.0), u);
    row(&mut r, "Poisson::sample, Knuth, lambda in [1, 30)", |_, y, g| Poisson::new(y).unwrap().sample(g), u, (1.0, 30.0));
    row(&mut r, "Poisson::sample, PTRS, lambda in [31, 1000)", |_, y, g| Poisson::new(y).unwrap().sample(g), u, (31.0, 1000.0));
    // part c2
    row(&mut r, "Binomial::pdf, Binomial(20, 0.3), k = floor(x), x in [0, 21)", |x, _, _| Binomial::new(20, 0.3).unwrap().pdf(x.floor()), (0.0, 21.0), u);
    row(&mut r, "Binomial::cdf, Binomial(20, 0.3), x in [0, 20)", |x, _, _| Binomial::new(20, 0.3).unwrap().cdf(x), (0.0, 20.0), u);
    row(&mut r, "Binomial::sample, simulated, n = 50, p in [0, 1)", |_, y, g| Binomial::new(50, y).unwrap().sample(g), u, u);
    row(&mut r, "Binomial::sample, Beta-split, n = 1e6, p in [0, 1)", |_, y, g| Binomial::new(1_000_000, y).unwrap().sample(g), u, u);
    // part d2
    let (_, _) = (M3.dim(), M16.dim());
    row(&mut r, "MultivariateNormal::pdf, 3-D, built once, at (x, y, 0.5), x in [0, 10), y in [-1, 1)", |x, y, _| M3.pdf(&[x, y, 0.5]).unwrap(), (0.0, 10.0), (-1.0, 1.0));
    row(&mut r, "MultivariateNormal::log_pdf, 3-D, built once, the same points", |x, y, _| M3.log_pdf(&[x, y, 0.5]).unwrap(), (0.0, 10.0), (-1.0, 1.0));
    row(&mut r, "MultivariateNormal::sample, 3-D, built once (the sum of a draw)", |_, _, g| {
        let s = M3.sample(g);
        s[0] + s[1] + s[2]
    }, u, u);
    row(&mut r, "MultivariateNormal::new and drop, 3-D, covariance times y in [0.1, 10)", |_, y, _| {
        oracle::mvn_factor(&MultivariateNormal::new(vec![0.0; 3], cov3(y)).unwrap()).1
    }, u, sd);
    row(&mut r, "MultivariateNormal::log_pdf, d = 16, built once, at (x, y, x, y, ...), in [-3, 3)", |x, y, _| {
        let mut p = [0.0; 16];
        for (i, v) in p.iter_mut().enumerate() {
            *v = if i % 2 == 0 { x } else { y };
        }
        M16.log_pdf(&p).unwrap()
    }, (-3.0, 3.0), (-3.0, 3.0));
    row(&mut r, "MultivariateNormal, to JSON and back, 3-D, mean (x, y, 0), in [-10, 10)", |x, y, _| {
        let t = serde_json::to_string(&MultivariateNormal::new(vec![x, y, 0.0], cov3(1.0)).unwrap()).unwrap();
        serde_json::from_str::<MultivariateNormal>(&t).unwrap().mean_vec()[0]
    }, wide, wide);
}
