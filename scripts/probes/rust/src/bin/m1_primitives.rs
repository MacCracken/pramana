//! M1 part b: the Rust twin of tests/crmath.bcyr. Rust's f64 methods (glibc 2.44's libm) on the same seeded
//! arguments, regions and loop: 4,096 values per row, cycled 250 times, ns per call. Run pinned, interleaved with
//! `cyrius bench tests/crmath.bcyr`:
//!   taskset -c 5 cargo run --release --locked --bin m1_primitives

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

    fn fill(&mut self, buf: &mut [f64], lo: f64, hi: f64) {
        for v in buf.iter_mut() {
            *v = lo + (hi - lo) * self.unit();
        }
    }
}

fn row(name: &str, xs: &[f64], ys: &[f64], f: impl Fn(f64, f64) -> f64) {
    let mut s = 0.0;
    let t0 = Instant::now();
    for _ in 0..250 {
        for i in 0..4096 {
            s += f(black_box(xs[i]), black_box(ys[i]));
        }
    }
    let ns = t0.elapsed().as_nanos() as f64 / 1_024_000.0;
    black_box(s);
    println!("  {name}: {ns:.1} ns");
}

fn main() {
    let mut r = Rng(0x2545F4914F6CDD1D);
    let mut xs = vec![0.0; 4096];
    let mut ys = vec![0.0; 4096];
    r.fill(&mut xs, -20.0, 20.0);
    row("exp, x in [-20, 20)", &xs, &ys, |x, _| x.exp());
    r.fill(&mut xs, 0.001, 1000.0);
    row("ln, x in [0.001, 1000)", &xs, &ys, |x, _| x.ln());
    r.fill(&mut xs, -1.0, 1.0);
    row("exp_m1, x in [-1, 1)", &xs, &ys, |x, _| x.exp_m1());
    r.fill(&mut xs, 0.5, 2.0);
    r.fill(&mut ys, -10.0, 10.0);
    row("powf, x in [0.5, 2), y in [-10, 10)", &xs, &ys, |x, y| x.powf(y));
    r.fill(&mut xs, -10.0, 10.0);
    row("sin, x in [-10, 10)", &xs, &ys, |x, _| x.sin());
    row("cos, x in [-10, 10)", &xs, &ys, |x, _| x.cos());
    row("atan, x in [-10, 10)", &xs, &ys, |x, _| x.atan());
    r.fill(&mut xs, -0.5, 1.0);
    row("ln_1p, x in [-0.5, 1)", &xs, &ys, |x, _| x.ln_1p());
    r.fill(&mut xs, -0.0009765625, 0.0009765625);
    row("ln_1p, x in [-1/1024, 1/1024)", &xs, &ys, |x, _| x.ln_1p());
    r.fill(&mut xs, -10.0, 10.0);
    row("tan, x in [-10, 10)", &xs, &ys, |x, _| x.tan());
    r.fill(&mut xs, 0.001, 1000.0);
    row("log2, x in [0.001, 1000)", &xs, &ys, |x, _| x.log2());
}
