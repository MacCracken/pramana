//! M3: the Rust twin of tests/dist.bcyr. 1.3.0's distributions (on glibc 2.44's primitives) on the same seeded
//! arguments, regions and loop: 4,096 values (or pairs, drawn alternately) per row, cycled 250 times, ns per call,
//! each row's sum as bits, and an FNV-1a digest of its 4,096 results' bits. A draw row takes its uniforms from one
//! SimpleRng seeded 42, and its digest from a fresh one. Build with `--features roundtrip`, as the golden corpus reads
//! JSON (serde_json's `float_roundtrip`, a correctly rounded parse); its default parse can miss by an ulp, which the
//! JSON row's digest shows. Run pinned, interleaved with `cyrius bench tests/dist.bcyr`:
//!   taskset -c 5 cargo run --release --locked --features roundtrip --bin m3_dist

use pramana::SimpleRng;
use pramana::distribution::{Bernoulli, Distribution, Exponential, Normal, Uniform};
use std::hint::black_box;
use std::time::Instant;

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
}
