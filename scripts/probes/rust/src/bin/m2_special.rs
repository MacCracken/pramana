//! M2: the Rust twin of tests/special.bcyr. 1.3.0's special functions (through the `oracle` hooks, on glibc 2.44's
//! primitives) on the same seeded arguments, regions and loop: 4,096 values (or pairs, drawn alternately) per row,
//! cycled 250 times, ns per call, each row's sum as bits, and an FNV-1a digest of its 4,096 results' bits. Run pinned, interleaved with `cyrius bench tests/special.bcyr`:
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

fn row(r: &mut Rng, name: &str, f: fn(f64, f64) -> f64, (lo, hi): (f64, f64), (lo2, hi2): (f64, f64)) {
    let xy: Vec<(f64, f64)> = (0..4096)
        .map(|_| {
            let x = lo + (hi - lo) * r.unit();
            (x, lo2 + (hi2 - lo2) * r.unit())
        })
        .collect();
    let mut s = 0.0;
    let t0 = Instant::now();
    for _ in 0..250 {
        for &(x, y) in &xy {
            s += f(black_box(x), black_box(y));
        }
    }
    let ns = t0.elapsed().as_nanos() as f64 / 1_024_000.0;
    println!("  {name}, the sum: {:x}", s.to_bits());
    // FNV-1a over the 4,096 results' bits, word by word, untimed: equal digests mean equal values.
    let mut dg: u64 = 0xcbf2_9ce4_8422_2325;
    for &(x, y) in &xy {
        dg = (dg ^ f(x, y).to_bits()).wrapping_mul(0x0100_0000_01b3);
    }
    println!("  {name}, the digest: {dg:x}");
    println!("  {name}: {ns:.1} ns");
}

fn main() {
    let mut r = Rng(0x2545F4914F6CDD1D);
    let u = (0.0, 1.0);
    row(&mut r, "the loop alone, x -> x", |x, _| x, (-0.84375, 0.84375), u);
    row(&mut r, "erf, |x| < 0.84375", |x, _| oracle::erf(x), (-0.84375, 0.84375), u);
    row(&mut r, "erf, 0.84375 <= x < 1.25", |x, _| oracle::erf(x), (0.84375, 1.25), u);
    row(&mut r, "erf, 1.25 <= x < 6", |x, _| oracle::erf(x), (1.25, 6.0), u);
    row(&mut r, "erfc, 0.25 <= x < 0.84375", |x, _| oracle::erfc(x), (0.25, 0.84375), u);
    row(&mut r, "erfc, 1.25 <= x < 2.857", |x, _| oracle::erfc(x), (1.25, 2.857), u);
    row(&mut r, "erfc, 2.857 <= x < 27", |x, _| oracle::erfc(x), (2.857, 27.0), u);
    row(&mut r, "normal_cdf, z in [-10, 10)", |x, _| oracle::normal_cdf(x), (-10.0, 10.0), u);
    row(&mut r, "normal_cdf, z in [-38, -1.2)", |x, _| oracle::normal_cdf(x), (-38.0, -1.2), u);
    row(&mut r, "normal_sf, z in [0, 38)", |x, _| oracle::normal_sf(x), (0.0, 38.0), u);
    row(&mut r, "ln_gamma, x in [0.5, 10)", |x, _| oracle::ln_gamma(x), (0.5, 10.0), u);
    row(&mut r, "ln_gamma, x in [-10, 0.5), reflected", |x, _| oracle::ln_gamma(x), (-10.0, 0.5), u);
    row(&mut r, "ln_gamma, x in [10, 1000)", |x, _| oracle::ln_gamma(x), (10.0, 1000.0), u);
    row(&mut r, "stirling_error, a in [0.5, 10)", |x, _| oracle::stirling_error(x), (0.5, 10.0), u);
    let series = "stirling_error, a in [10, 1000), the series";
    row(&mut r, series, |x, _| oracle::stirling_error(x), (10.0, 1000.0), u);
    row(&mut r, "bd0, x in [0, 100), m in [1, 100)", oracle::bd0, (0.0, 100.0), (1.0, 100.0));
    let k1 = "ln_gamma_kernel, a in [1, 10), x in [0.1, 20)";
    row(&mut r, k1, oracle::ln_gamma_kernel, (1.0, 10.0), (0.1, 20.0));
    let k2 = "ln_gamma_kernel, a in [10, 1000), x in [5, 1500)";
    row(&mut r, k2, oracle::ln_gamma_kernel, (10.0, 1000.0), (5.0, 1500.0));
    row(&mut r, "ln_beta, a and b in [0.5, 10)", oracle::ln_beta, (0.5, 10.0), (0.5, 10.0));
    row(&mut r, "ln_beta, a in [0.5, 10), b in [10, 1000)", oracle::ln_beta, (0.5, 10.0), (10.0, 1000.0));
    row(&mut r, "ln_beta, a and b in [10, 1000)", oracle::ln_beta, (10.0, 1000.0), (10.0, 1000.0));
    let gi = |a, x| {
        let (p, q) = oracle::gamma_inc(a, x);
        p - q
    };
    row(&mut r, "gamma_inc, a in [0.5, 10), x in [0.1, 20)", gi, (0.5, 10.0), (0.1, 20.0));
    row(&mut r, "gamma_inc, a in [10, 1000), x in [5, 1500)", gi, (10.0, 1000.0), (5.0, 1500.0));
    let wh = (1e9, 1.0001e9);
    row(&mut r, "gamma_inc, Wilson-Hilferty, a and x in [1e9, 1.0001e9)", gi, wh, wh);
    let bi = |a, b| {
        let s = a + b;
        let (p, q) = oracle::beta_inc(a, b, a / s, b / s);
        p - q
    };
    row(&mut r, "beta_inc, a and b in [0.5, 10), x at the mean", bi, (0.5, 10.0), (0.5, 10.0));
    row(&mut r, "beta_inc, a and b in [10, 1000), x at the mean", bi, (10.0, 1000.0), (10.0, 1000.0));
}
