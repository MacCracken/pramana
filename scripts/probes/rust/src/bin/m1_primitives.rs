//! M1 parts b–d: the Rust twin of tests/crmath.bcyr, tests/mathx.bcyr, tests/fmtf.bcyr, tests/pool.bcyr, tests/json.bcyr, tests/rng.bcyr and tests/combinatorics.bcyr. Rust's f64 methods (glibc 2.44's libm) on the same seeded
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

// tests/pool.bcyr's counterpart: what a Rust function does in place of a pool claim, a heap Vec made and dropped.
fn pool() {
    let mut sink = 0u64;
    let t0 = Instant::now();
    for i in 0..1_000_000u64 {
        let mut v: Vec<u64> = Vec::with_capacity(black_box(32));
        v.push(i);
        sink = sink.wrapping_add(v[0]);
        drop(black_box(v));
    }
    black_box(sink);
    println!("  a 256-byte Vec, allocated and dropped: {:.1} ns", t0.elapsed().as_nanos() as f64 / 1_000_000.0);
}

// tests/json.bcyr's counterpart: serde_json::from_slice on byte-identical documents (each side prints the length and
// FNV-1a hash), 10^4 decodes per row. Build with `--features roundtrip` (float_roundtrip, as the golden rows).
fn json_row<T>(name: &str, doc: &[u8], f: impl Fn(&[u8]) -> serde_json::Result<T>) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in doc {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    println!("  {name}: {} bytes, fnv 0x{h:x}", doc.len());
    let t0 = Instant::now();
    for _ in 0..10_000 {
        black_box(f(black_box(doc)).expect("a document Rust wrote"));
    }
    println!("  {name}: {:.1} ns", t0.elapsed().as_nanos() as f64 / 10_000.0);
}

fn json() {
    let mut r = Rng(0x2545F4914F6CDD1D);
    let xs: Vec<f64> = (0..100).map(|_| 2_000_000.0 * r.unit() - 1_000_000.0).collect();
    let doc = serde_json::to_vec(&xs).unwrap();
    json_row("Vec<f64>, 100 values in [-1e6, 1e6)", &doc, |d: &[u8]| serde_json::from_slice::<Vec<f64>>(d));
    let m: Vec<Vec<f64>> = (0..10).map(|_| (0..10).map(|_| r.unit()).collect()).collect();
    let doc = serde_json::to_vec(&m).unwrap();
    json_row("Vec<Vec<f64>>, 10 x 10 in [0, 1)", &doc, |d: &[u8]| serde_json::from_slice::<Vec<Vec<f64>>>(d));
    let ws: Vec<u64> = (0..100)
        .map(|_| {
            r.unit();
            r.0.wrapping_mul(0x2545F4914F6CDD1D)
        })
        .collect();
    let doc = serde_json::to_vec(&ws).unwrap();
    json_row("Vec<u64>, 100 words", &doc, |d: &[u8]| serde_json::from_slice::<Vec<u64>>(d));
    let doc = format!("\"{}\"", "abc\\n\\u00e9\\\"xyz".repeat(8)).into_bytes();
    json_row("String, 8 x abc\\n\\u00e9\\\"xyz", &doc, |d: &[u8]| serde_json::from_slice::<String>(d));
    json_row("f64, 1.5", b"1.5", |d: &[u8]| serde_json::from_slice::<f64>(d));
    let doc = serde_json::to_vec(&pramana::distribution::Normal::new(0.5, 2.25).unwrap()).unwrap();
    json_row("Normal, its two fields and checks", &doc, |d: &[u8]| {
        serde_json::from_slice::<pramana::distribution::Normal>(d)
    });
    let doc = serde_json::to_vec(&pramana::descriptive::Kernel::Epanechnikov).unwrap();
    json_row("Kernel, a unit variant", &doc, |d: &[u8]| {
        serde_json::from_slice::<pramana::descriptive::Kernel>(d)
    });    let normal = pramana::distribution::Normal::new(0.5, 2.25).unwrap();
    let text = String::from("abc\ndef\"ghi\\jkl\tmn\u{e9}o\u{1}pq");
    let err = pramana::PramanaError::DimensionMismatch(String::new());
    write_row("write a Vec<f64>, the 100 values", |b| serde_json::to_writer(b, &xs));
    write_row("write a Normal", |b| serde_json::to_writer(b, &normal));
    write_row("write a String, 5 escapes in 24 bytes", |b| serde_json::to_writer(b, &text));
    write_row("write an error code", |b| serde_json::to_writer(b, &err));
}

// tests/json.bcyr's write rows: serde_json::to_writer into one cleared Vec, 10^4 times.
fn write_row(name: &str, f: impl Fn(&mut Vec<u8>) -> serde_json::Result<()>) {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let t0 = Instant::now();
    for _ in 0..10_000 {
        buf.clear();
        f(black_box(&mut buf)).expect("serde_json writes it");
    }
    let ns = t0.elapsed().as_nanos() as f64 / 10_000.0;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &c in &buf {
        h = (h ^ u64::from(c)).wrapping_mul(0x100_0000_01b3);
    }
    println!("  {name}: {} bytes, fnv 0x{h:x}", buf.len());
    println!("  {name}: {ns:.1} ns");
}

// tests/rng.bcyr's counterpart: SimpleRng's draws from seed 42, direct and through a generic `R: Rng`.
fn draw<R: pramana::Rng>(r: &mut R) -> f64 {
    r.next_f64()
}

fn rng() {
    use pramana::Rng as _;
    let mut g = pramana::SimpleRng::new(42);
    let mut s = 0.0;
    let mut last = 0.0;
    let t0 = Instant::now();
    for _ in 0..1_000_000 {
        last = black_box(&mut g).next_f64();
        s += last;
    }
    let ns = t0.elapsed().as_nanos() as f64 / 1_000_000.0;
    println!("  the 10^6th draw: 0x{:x}", last.to_bits());
    println!("  next_f64, a direct call: {ns:.2} ns");
    let mut g = pramana::SimpleRng::new(42);
    let t0 = Instant::now();
    for _ in 0..1_000_000 {
        last = draw(black_box(&mut g));
        s += last;
    }
    let ns = t0.elapsed().as_nanos() as f64 / 1_000_000.0;
    println!("  next_f64, through <R: Rng>: {ns:.2} ns");
    black_box(s);
}

// tests/combinatorics.bcyr's counterpart: 1.3.0's counting functions, 10^5 calls per row.
fn comb_row(name: &str, f: impl Fn() -> Result<u128, pramana::PramanaError>) {
    let t0 = Instant::now();
    let mut r = f();
    for _ in 0..100_000 {
        r = black_box(f());
    }
    let ns = t0.elapsed().as_nanos() as f64 / 100_000.0;
    match r {
        Ok(x) => println!("  {name}: {:x} {:x}", (x >> 64) as u64, x as u64),
        Err(e) => println!("  {name}: {e:?}"),
    }
    println!("  {name}: {ns:.1} ns");
}

fn comb() {
    use pramana::combinatorics::{combinations, factorial, permutations, stirling_approx};
    comb_row("factorial(34)", || factorial(black_box(34)));
    comb_row("permutations(30, 15)", || permutations(black_box(30), black_box(15)));
    comb_row("combinations(60, 30)", || combinations(black_box(60), black_box(30)));
    comb_row("combinations(130, 65), an overflow", || combinations(black_box(130), black_box(65)));
    let mut s = 0.0;
    let t0 = Instant::now();
    for _ in 0..100_000 {
        s += stirling_approx(black_box(100));
    }
    black_box(s);
    println!("  stirling_approx(100): {:.1} ns", t0.elapsed().as_nanos() as f64 / 100_000.0);
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("comb") {
        comb();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("rng") {
        rng();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("json") {
        json();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("pool") {
        pool();
        return;
    }
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
