// Seeded inputs for the accuracy baseline (scripts/port-baseline/run.sh). Prints
// "name input... output" lines with every f64 as its bit pattern; check.c and
// beta_check.py compare them with MPFR and exact rationals.
#![allow(dead_code)]
mod math;
use std::io::Write;

// xorshift64* for reproducible inputs
struct R(u64);
impl R {
    fn u(&mut self) -> f64 {
        self.0 ^= self.0 >> 12; self.0 ^= self.0 << 25; self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545F4914F6CDD1D)) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 { a + (b - a) * self.u() }
    fn logr(&mut self, a: f64, b: f64) -> f64 { (a.ln() + (b.ln() - a.ln()) * self.u()).exp() }
}

fn h(x: f64) -> String { format!("{:016x}", x.to_bits()) }

fn main() {
    let mut r = R(0x9E3779B97F4A7C15);
    if std::env::args().nth(1).as_deref() == Some("beta") { beta(&mut r); return; }
    if std::env::args().nth(1).as_deref() == Some("ncdf") { for _ in 0..50_000 { let z = r.range(-38.0, 8.0); println!("ncdf {} {}", h(z), h(math::normal_cdf(z))); } for _ in 0..50_000 { let z = r.range(-5.0, 5.0); println!("ncdf_core {} {}", h(z), h(math::normal_cdf(z))); } return; }
    let out = std::io::stdout();
    let mut o = std::io::BufWriter::new(out.lock());
    let n = 100_000;
    // elementary functions via Rust std (glibc on this host)
    for _ in 0..n { let x = r.range(-700.0, 700.0); writeln!(o, "exp {} {}", h(x), h(x.exp())).unwrap(); }
    for _ in 0..n { let x = r.logr(1e-300, 1e300); writeln!(o, "log {} {}", h(x), h(x.ln())).unwrap(); }
    for _ in 0..n { let x = if r.u() < 0.5 { r.logr(1e-20, 1e3) } else { -r.logr(1e-20, 0.999) }; writeln!(o, "log1p {} {}", h(x), h(x.ln_1p())).unwrap(); }
    for _ in 0..n { let x = if r.u() < 0.5 { r.logr(1e-20, 700.0) } else { -r.logr(1e-20, 40.0) }; writeln!(o, "expm1 {} {}", h(x), h(x.exp_m1())).unwrap(); }
    for _ in 0..n { let x = r.range(-10.0, 10.0); writeln!(o, "sin {} {}", h(x), h(x.sin())).unwrap(); }
    for _ in 0..n { let x = r.range(-10.0, 10.0); writeln!(o, "cos {} {}", h(x), h(x.cos())).unwrap(); }
    for _ in 0..n { let x = r.range(-1.5, 1.5); writeln!(o, "tan {} {}", h(x), h(x.tan())).unwrap(); }
    for _ in 0..n { let x = r.logr(1e-3, 1e3); let y = r.range(-50.0, 50.0); writeln!(o, "pow {} {} {}", h(x), h(y), h(x.powf(y))).unwrap(); }
    // pramana 1.3.0 special functions
    for _ in 0..n { let x = if r.u() < 0.8 { r.range(-6.0, 6.0) } else { let s = r.logr(1e-300, 1.0); if r.u() < 0.5 { s } else { -s } }; writeln!(o, "erf {} {}", h(x), h(math::erf(x))).unwrap(); }
    for _ in 0..n { let x = r.range(-6.0, 27.0); writeln!(o, "erfc {} {}", h(x), h(math::erfc(x))).unwrap(); }
    for _ in 0..n { let x = r.range(0.5, 10.0); writeln!(o, "lgamma {} {}", h(x), h(math::ln_gamma(x))).unwrap(); }
    for _ in 0..n / 4 { let x = r.logr(10.0, 1e15); writeln!(o, "lgamma_big {} {}", h(x), h(math::ln_gamma(x))).unwrap(); }
    for _ in 0..n / 4 { let x = r.range(1e-300_f64.ln(), 0.5_f64.ln()).exp(); writeln!(o, "lgamma_small {} {}", h(x), h(math::ln_gamma(x))).unwrap(); }
    for _ in 0..n / 4 { let k = r.range(-20.0, 0.0).floor(); let x = k + r.range(0.001, 0.999); writeln!(o, "lgamma_neg {} {}", h(x), h(math::ln_gamma(x))).unwrap(); }
    for _ in 0..n / 4 { let d = r.logr(1e-12, 0.1); let x = if r.u() < 0.5 { 1.0 } else { 2.0 } + if r.u() < 0.5 { d } else { -d }; writeln!(o, "lgamma_near12 {} {}", h(x), h(math::ln_gamma(x))).unwrap(); }
    // regularized incomplete gamma, both tails
    for _ in 0..20_000 {
        let a = r.logr(0.01, 1000.0);
        let x = (a * r.range(-3.0, 3.0) / a.sqrt().max(1.0) + a).max(a * 1e-3).max(1e-6) * r.range(0.999, 1.001);
        let (p, q) = math::gamma_inc(a, x);
        writeln!(o, "gamma_p {} {} {}", h(a), h(x), h(p)).unwrap();
        writeln!(o, "gamma_q {} {} {}", h(a), h(x), h(q)).unwrap();
    }
}

fn beta(r: &mut R) {
    // integer shapes, dyadic x so 1 - x is exact; columns: a b k p q (k = x * 2^20)
    for _ in 0..4000 {
        let a = (r.u() * 60.0) as u32 + 1; let b = (r.u() * 60.0) as u32 + 1;
        let k = (r.u() * ((1u64 << 20) - 1) as f64) as u64 + 1;
        let x = k as f64 / (1u64 << 20) as f64; let y = 1.0 - x;
        let (p, q) = math::beta_inc(a as f64, b as f64, x, y);
        println!("{a} {b} {k} {:016x} {:016x}", p.to_bits(), q.to_bits());
    }
}
