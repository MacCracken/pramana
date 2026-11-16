//! M1 parts b–d: the Rust twin of tests/crmath.bcyr, tests/mathx.bcyr and tests/fmtf.bcyr. Rust's f64 methods (glibc 2.44's libm) on the same seeded
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

// tests/mathx.bcyr's rows: the same seed, regions and order.
fn mathx() {
    let mut r = Rng(0x2545F4914F6CDD1D);
    let mut xs = vec![0.0; 4096];
    let mut ys = vec![0.0; 4096];
    let mut ws = vec![0u64; 4096];
    r.fill(&mut xs, -10.0, 10.0);
    r.fill(&mut ys, -10.0, 10.0);
    for w in ws.iter_mut() {
        r.unit();
        *w = r.0.wrapping_mul(0x2545F4914F6CDD1D);
    }
    row("max, x and y in [-10, 10)", &xs, &ys, |x, y| x.max(y));
    row("clamp, x in [-10, 10) to [-1, 2]", &xs, &ys, |x, _| x.clamp(-1.0, 2.0));
    r.fill(&mut xs, -1000000.0, 1000000.0);
    row("fract, x in [-1e6, 1e6)", &xs, &ys, |x, _| x.fract());
    r.fill(&mut xs, 0.9, 1.1);
    row("powi(25), x in [0.9, 1.1)", &xs, &ys, |x, _| x.powi(25));
    let mut s = 0.0;
    let t0 = Instant::now();
    for _ in 0..250 {
        for w in ws.iter() {
            s += black_box(*w) as f64;
        }
    }
    black_box(s);
    println!("  u64 as f64, any word: {:.1} ns", t0.elapsed().as_nanos() as f64 / 1_024_000.0);
    r.fill(&mut xs, -1000.0, 100000000000000000000.0);
    let mut n = 0usize;
    let t1 = Instant::now();
    for _ in 0..250 {
        for x in xs.iter() {
            n = n.wrapping_add(black_box(*x) as usize);
        }
    }
    black_box(n);
    println!("  as usize, x in [-1e3, 1e20): {:.1} ns", t1.elapsed().as_nanos() as f64 / 1_024_000.0);
    r.fill(&mut xs, -10.0, 10.0);
    let mut c = 0i64;
    let t2 = Instant::now();
    for _ in 0..250 {
        for i in 0..4096 {
            c += black_box(xs[i]).total_cmp(&black_box(ys[i])) as i64;
        }
    }
    black_box(c);
    println!("  total_cmp, x and y in [-10, 10): {:.1} ns", t2.elapsed().as_nanos() as f64 / 1_024_000.0);
}

// tests/fmtf.bcyr's rows: serde_json's text written into one reused buffer.
fn fmt() {
    let mut r = Rng(0x2545F4914F6CDD1D);
    let mut xs = vec![0.0; 4096];
    r.fill(&mut xs, 0.0, 1000000.0);
    let time = |name: &str, xs: &[f64]| {
        let mut buf: Vec<u8> = Vec::with_capacity(64);
        let mut total = 0usize;
        let t0 = Instant::now();
        for _ in 0..250 {
            for x in xs {
                buf.clear();
                serde_json::to_writer(&mut buf, black_box(x)).expect("an f64 always writes");
                total += buf.len();
            }
        }
        black_box(total);
        println!("  {name}: {:.1} ns", t0.elapsed().as_nanos() as f64 / 1_024_000.0);
    };
    time("to_json, x in [0, 1e6)", &xs);
    for x in xs.iter_mut() {
        r.unit();
        let mut bits = r.0.wrapping_mul(0x2545F4914F6CDD1D);
        if (bits >> 52) & 0x7FF == 0x7FF {
            bits &= 0xBFFF_FFFF_FFFF_FFFF;
        }
        *x = f64::from_bits(bits);
    }
    time("to_json, any finite bits", &xs);
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("fmt") {
        fmt();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("mathx") {
        mathx();
        return;
    }
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
