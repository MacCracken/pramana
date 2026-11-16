//! M2: the Rust twin of tests/special.bcyr. 1.3.0's erf, erfc, normal_cdf and normal_sf (through the `oracle` hooks,
//! on glibc 2.44's exp) on the same seeded arguments, regions and loop: 4,096 values per row, cycled 250 times, ns per
//! call, and each row's sum as bits. Run pinned, interleaved with `cyrius bench tests/special.bcyr`:
//!   taskset -c 5 cargo run --release --locked --bin m2_special

use pramana::oracle;
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

fn row(r: &mut Rng, name: &str, f: fn(f64) -> f64, lo: f64, hi: f64) {
    let xs: Vec<f64> = (0..4096).map(|_| lo + (hi - lo) * r.unit()).collect();
    let mut s = 0.0;
    let t0 = Instant::now();
    for _ in 0..250 {
        for &x in &xs {
            s += f(black_box(x));
        }
    }
    let ns = t0.elapsed().as_nanos() as f64 / 1_024_000.0;
    println!("  {name}, the sum: {:x}", s.to_bits());
    println!("  {name}: {ns:.1} ns");
}

fn main() {
    let mut r = Rng(0x2545F4914F6CDD1D);
    row(&mut r, "the loop alone, x -> x", |x| x, -0.84375, 0.84375);
    row(&mut r, "erf, |x| < 0.84375", oracle::erf, -0.84375, 0.84375);
    row(&mut r, "erf, 0.84375 <= x < 1.25", oracle::erf, 0.84375, 1.25);
    row(&mut r, "erf, 1.25 <= x < 6", oracle::erf, 1.25, 6.0);
    row(&mut r, "erfc, 0.25 <= x < 0.84375", oracle::erfc, 0.25, 0.84375);
    row(&mut r, "erfc, 1.25 <= x < 2.857", oracle::erfc, 1.25, 2.857);
    row(&mut r, "erfc, 2.857 <= x < 27", oracle::erfc, 2.857, 27.0);
    row(&mut r, "normal_cdf, z in [-10, 10)", oracle::normal_cdf, -10.0, 10.0);
    row(&mut r, "normal_cdf, z in [-38, -1.2)", oracle::normal_cdf, -38.0, -1.2);
    row(&mut r, "normal_sf, z in [0, 38)", oracle::normal_sf, 0.0, 38.0);
}
