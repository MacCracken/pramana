//! P5 (port plan §5 step 9, §8.2): hisab 1.4.0's four routines, as pramana 1.3.0 calls them, on seeded inputs.
//! The Cyrius twin computes the same inputs with ganita's counterparts; p0_compare.py gives the ulp distance per
//! element. Inputs use only + − × ÷ and a shared xorshift64*, so both sides build identical matrices.

use pramana::hisab::num::{cholesky, cholesky_solve, eigen_symmetric, least_squares_poly};

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
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

fn f(case: &str, x: f64) {
    println!("{case}\tf64\t{:#x}", x.to_bits());
}

/// A = BᵀB + n·I, B uniform in [−1, 1), summed in k order.
fn spd(g: &mut G, n: usize) -> Vec<Vec<f64>> {
    let b: Vec<Vec<f64>> = (0..n).map(|_| (0..n).map(|_| g.uniform(-1.0, 1.0)).collect()).collect();
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

fn main() {
    for &n in &[2usize, 3, 4, 6, 8, 10] {
        for seed in 1..=10u64 {
            let mut g = G(seed * 7919 + n as u64);
            let a = spd(&mut g, n);
            let rhs: Vec<f64> = (0..n).map(|_| g.uniform(-1.0, 1.0)).collect();
            let l = cholesky(&a).unwrap();
            for i in 0..n {
                for j in 0..=i {
                    f(&format!("chol[{n},{seed}][{i},{j}]"), l[i][j]);
                }
            }
            let x = cholesky_solve(&l, &rhs).unwrap();
            for (i, v) in x.iter().enumerate() {
                f(&format!("solve[{n},{seed}][{i}]"), *v);
            }
            let e = eigen_symmetric(&a, 1e-12, 1000).unwrap();
            for (i, v) in e.eigenvalues_real.iter().enumerate() {
                f(&format!("eigval[{n},{seed}][{i}]"), *v);
            }
        }
    }
    for degree in 1..=4usize {
        for seed in 1..=10u64 {
            let mut g = G(seed * 104_729 + degree as u64);
            let m = 40;
            let xs: Vec<f64> = (0..m).map(|_| g.uniform(-2.0, 2.0)).collect();
            let ys: Vec<f64> = xs.iter().map(|&x| 1.0 + 0.5 * x - 0.25 * x * x + g.uniform(-0.1, 0.1)).collect();
            let c = least_squares_poly(&xs, &ys, degree).unwrap();
            for (i, v) in c.iter().enumerate() {
                f(&format!("lsq[{degree},{seed}][{i}]"), *v);
            }
        }
    }
}
