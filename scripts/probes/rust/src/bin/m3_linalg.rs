//! M3 part d1: the Rust twin of tests/linalg.bcyr. hisab 1.4.0's `cholesky` and `cholesky_solve`, as pramana 1.3.0
//! calls them, each result a fresh `Vec`, on the same seeded SPD matrices (A = BᵀB + n·I, B uniform in [−1, 1),
//! summed in k order, from the bench's xorshift64*) at n = 4, 16 and 64. Per row: ns per call, the sum of one output
//! element over the calls as bits, and an FNV-1a digest of one call's output bits. Run pinned, interleaved with
//! `cyrius bench tests/linalg.bcyr`:
//!   taskset -c 5 cargo run --release --locked --bin m3_linalg

use pramana::hisab::num::{cholesky, cholesky_solve};
use std::hint::black_box;
use std::time::Instant;

struct G(u64);
impl G {
    fn unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / 9_007_199_254_740_992.0
    }
}

fn spd(g: &mut G, n: usize) -> Vec<Vec<f64>> {
    let b: Vec<Vec<f64>> = (0..n).map(|_| (0..n).map(|_| -1.0 + 2.0 * g.unit()).collect()).collect();
    let mut a = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let mut s = 0.0;
            for k in 0..n {
                s += b[k][i] * b[k][j];
            }
            a[i][j] = if i == j { s + n as f64 } else { s };
        }
    }
    a
}

fn digest(xs: &[f64]) -> u64 {
    let mut d: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in xs {
        d = (d ^ x.to_bits()).wrapping_mul(0x0100_0000_01b3);
    }
    d
}

fn main() {
    let mut g = G(0x2545_F491_4F6C_DD1D);
    for (n, reps) in [(4usize, 200_000u32), (16, 20_000), (64, 500)] {
        let a = spd(&mut g, n);
        let b: Vec<f64> = (0..n).map(|_| -1.0 + 2.0 * g.unit()).collect();
        let mut s = 0.0;
        let t0 = Instant::now();
        for _ in 0..reps {
            let l = cholesky(black_box(&a)).unwrap();
            s += l[n - 1][n - 1];
        }
        let ns = t0.elapsed().as_nanos() as f64 / f64::from(reps);
        let l = cholesky(&a).unwrap();
        let flat: Vec<f64> = l.iter().flatten().copied().collect();
        println!("  cholesky, n = {n}, the sum: {:x}", s.to_bits());
        println!("  cholesky, n = {n}, the digest: {:x}", digest(&flat));
        println!("  cholesky, n = {n}: {ns:.1} ns");
        let mut s2 = 0.0;
        let t1 = Instant::now();
        for _ in 0..reps {
            let x = cholesky_solve(black_box(&l), black_box(&b)).unwrap();
            s2 += x[0];
        }
        let ns2 = t1.elapsed().as_nanos() as f64 / f64::from(reps);
        let x = cholesky_solve(&l, &b).unwrap();
        println!("  cholesky_solve, n = {n}, the sum: {:x}", s2.to_bits());
        println!("  cholesky_solve, n = {n}, the digest: {:x}", digest(&x));
        println!("  cholesky_solve, n = {n}: {ns2:.1} ns");
    }
}
