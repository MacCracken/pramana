//! p0_semantics.rs — M0 step 4: Rust 1.3.0's value for each case `p0_semantics.cyr` probes, as the oracle computes
//! it. That means a release build (LLVM rewrites some calls only when optimizing) on the recording toolchain, with
//! std → the host glibc. Inputs pass through `black_box`, so LLVM cannot fold a call at compile time.
//!
//!   rustc +1.100.0 -O --edition 2024 -o build/p0_rust scripts/probes/p0_semantics.rs && ./build/p0_rust > build/p0_rust.tsv
//!
//! Prints: case <TAB> type <TAB> bits (hex). Types: f64 (compared by ulp), u32 / u64 / i64 (compared as words).

use std::hint::black_box as bb;

fn f(case: &str, x: f64) {
    println!("{case}\tf64\t{:#x}", x.to_bits());
}

fn w(case: &str, ty: &str, x: u64) {
    println!("{case}\t{ty}\t{x:#x}");
}

/// `SimpleRng::next_f64` (rust-old/src/rng.rs): xorshift64, then the top 53 bits over 2^53.
fn xorshift(state: &mut u64) -> f64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// `f64::total_cmp`'s key: the bits with the magnitude flipped for negatives.
fn total_key(x: f64) -> i64 {
    let mut k = x.to_bits() as i64;
    k ^= (((k >> 63) as u64) >> 1) as i64;
    k
}

fn main() {
    let nan = bb(f64::NAN);
    let inf = bb(f64::INFINITY);

    // min / max / clamp (30 min/max and 10 clamp sites)
    f("min(nan,1)", nan.min(bb(1.0)));
    f("min(1,nan)", bb(1.0f64).min(nan));
    f("max(nan,1)", nan.max(bb(1.0)));
    f("max(1,nan)", bb(1.0f64).max(nan));
    f("min(+0,-0)", bb(0.0f64).min(bb(-0.0)));
    f("max(+0,-0)", bb(0.0f64).max(bb(-0.0)));
    f("clamp(nan,0,1)", nan.clamp(bb(0.0), bb(1.0)));
    f("clamp(2,0,1)", bb(2.0f64).clamp(bb(0.0), bb(1.0)));
    f("clamp(-0,0,1)", bb(-0.0f64).clamp(bb(0.0), bb(1.0)));

    // exact operations
    f("floor(-0.5)", bb(-0.5f64).floor());
    f("floor(-0)", bb(-0.0f64).floor());
    f("floor(2.5)", bb(2.5f64).floor());
    f("ceil(-0.5)", bb(-0.5f64).ceil());
    f("trunc(-2.7)", bb(-2.7f64).trunc());
    f("sqrt(2)", bb(2.0f64).sqrt());
    f("sqrt(-0)", bb(-0.0f64).sqrt());
    f("abs(-0)", bb(-0.0f64).abs());

    // fract (the Poisson and Binomial pdfs' integer test)
    for (name, x) in [
        ("fract(2.75)", 2.75f64),
        ("fract(-2.75)", -2.75),
        ("fract(-0)", -0.0),
        ("fract(3)", 3.0),
        ("fract(1e300)", 1e300),
        ("fract(inf)", f64::INFINITY),
    ] {
        f(name, bb(x).fract());
    }

    // transcendentals (Rust std → glibc); the port replaces each with a correctly rounded function (ledger L1)
    for (name, x) in [("exp(-0.7)", -0.7f64), ("exp(1)", 1.0), ("exp(5.3)", 5.3), ("exp(239.014)", 239.014_397_531_312_97)] {
        f(name, bb(x).exp());
    }
    for (name, x) in [("ln(0.3)", 0.3f64), ("ln(2)", 2.0), ("ln(10)", 10.0)] {
        f(name, bb(x).ln());
    }
    for (name, x) in [("ln_1p(1e-10)", 1e-10f64), ("ln_1p(-0.5)", -0.5), ("ln_1p(0.2833)", 0.283_312_337_186_778_45)] {
        f(name, bb(x).ln_1p());
    }
    for (name, x) in [("exp_m1(1e-10)", 1e-10f64), ("exp_m1(-3)", -3.0), ("exp_m1(0.9204)", 0.920_438_866_414_895_58)] {
        f(name, bb(x).exp_m1());
    }
    f("pow(2,0.5)", bb(2.0f64).powf(bb(0.5)));
    f("pow(1.5,2.3)", bb(1.5f64).powf(bb(2.3)));
    f("pow(200,-0.2)", bb(200.0f64).powf(bb(-0.2)));
    for (s, c, x) in [("sin(0.5)", "cos(0.5)", 0.5f64), ("sin(1)", "cos(1)", 1.0), ("sin(3)", "cos(3)", 3.0)] {
        f(s, bb(x).sin());
        f(c, bb(x).cos());
    }
    for (name, x) in [("tan(0.5)", 0.5f64), ("tan(1.4939)", 1.493_861_655_465_847_7), ("tan(-1)", -1.0)] {
        f(name, bb(x).tan());
    }
    for (name, x) in [("atan(0.5)", 0.5f64), ("atan(-3)", -3.0), ("atan(1e10)", 1e10)] {
        f(name, bb(x).atan());
    }
    for (name, x) in [("log2(3)", 3.0f64), ("log2(8)", 8.0), ("log2(10)", 10.0)] {
        f(name, bb(x).log2());
    }
    f("cbrt(2)", bb(2.0f64).cbrt());
    f("cbrt(0.001)", bb(0.001f64).cbrt());
    f("cbrt(1e-300)", bb(1e-300f64).cbrt());

    // powi with the exponents rust-old/src uses: 2, 3, 4, 9, 25, 49
    f("powi(1.1,2)", bb(1.1f64).powi(2));
    f("powi(1.1,3)", bb(1.1f64).powi(3));
    f("powi(1.1,4)", bb(1.1f64).powi(4));
    f("powi(0.9,9)", bb(0.9f64).powi(9));
    f("powi(0.99,25)", bb(0.99f64).powi(25));
    f("powi(0.999,49)", bb(0.999f64).powi(49));

    // SimpleRng: xorshift64 words and draws
    for (name, seed) in [("1", 1u64), ("42", 42), ("2^63", 1 << 63), ("u64max", u64::MAX)] {
        let mut s = bb(seed);
        let d1 = xorshift(&mut s);
        w(&format!("rng[{name}].state1"), "u64", s);
        f(&format!("rng[{name}].draw1"), d1);
        let mut last = 0.0;
        for _ in 1..1000 {
            last = xorshift(&mut s);
        }
        w(&format!("rng[{name}].state1000"), "u64", s);
        f(&format!("rng[{name}].draw1000"), last);
    }

    // total_cmp keys (mode's sort)
    for (name, x) in [
        ("key(-nan)", -f64::NAN),
        ("key(-inf)", f64::NEG_INFINITY),
        ("key(-1)", -1.0f64),
        ("key(-0)", -0.0),
        ("key(+0)", 0.0),
        ("key(1)", 1.0),
        ("key(inf)", f64::INFINITY),
        ("key(nan)", f64::NAN),
    ] {
        w(name, "i64", total_key(bb(x)) as u64);
    }

    // u64 → f64 (Binomial's n is a u64)
    for (name, n) in [
        ("f64(12345u64)", 12345u64),
        ("f64(2^53+1)", (1u64 << 53) + 1),
        ("f64(2^63)", 1u64 << 63),
        ("f64(2^63+2^11+1)", (1u64 << 63) + (1 << 11) + 1),
        ("f64(u64max)", u64::MAX),
    ] {
        f(name, bb(n) as f64);
    }

    // f64 → usize, saturating (rank.floor() as usize, the iteration budget, the histogram bin)
    for (name, x) in [
        ("usize(floor(-3.7))", -3.7f64),
        ("usize(floor(7.9))", 7.9),
        ("usize(1e29)", 1e29),
        ("usize(0)", 0.0),
    ] {
        w(name, "u64", bb(x).floor() as usize as u64);
    }
    w("usize(nan)", "u64", nan as usize as u64);
    w("usize(inf)", "u64", inf as usize as u64);

    // the fdlibm erf port's word operations (rust-old/src/math.rs)
    for (name, x) in [("hi(-2.5)", -2.5f64), ("hi(1e-300)", 1e-300), ("hi(0.84375)", 0.84375)] {
        w(name, "u32", ((bb(x).to_bits() >> 32) as u32) as u64);
    }
    f("lowmask(1.2345678)", f64::from_bits(bb(1.234_567_8f64).to_bits() & 0xffff_ffff_0000_0000));

    // constants and exponent literals (the Cyrius lexer has no exponent form)
    f("EPSILON", f64::EPSILON);
    f("MIN_POSITIVE", f64::MIN_POSITIVE);
    f("MAX", f64::MAX);
    f("INFINITY", f64::INFINITY);
    f("NAN", f64::NAN);
    f("PI", std::f64::consts::PI);
    f("E", std::f64::consts::E);
    f("TAU", std::f64::consts::TAU);
    f("lit(1e-15)", 1e-15);
    f("lit(1e-300)", 1e-300);
    f("lit(1e300)", 1e300);
    f("lit(1e9)", 1e9);
    f("lit(0.1)", 0.1);
    f("lit(LN_SQRT_2PI)", 0.918_938_533_204_672_8);

    // the neutral element of f64 sums (Rust's Sum starts at -0.0)
    let empty: [f64; 0] = [];
    f("sum([])", bb(&empty).iter().sum::<f64>());
    f("sum([+0])", bb(&[0.0f64]).iter().sum::<f64>());
    f("sum([-0])", bb(&[-0.0f64]).iter().sum::<f64>());

    // comparisons
    w("lt(nan,1)", "u64", (nan < bb(1.0)) as u64);
    w("eq(nan,nan)", "u64", (nan == nan) as u64);
    w("ne(nan,nan)", "u64", (nan != nan) as u64);
    w("lt(-0,+0)", "u64", (bb(-0.0f64) < bb(0.0)) as u64);
    w("eq(-0,+0)", "u64", (bb(-0.0f64) == bb(0.0)) as u64);
    w("ge(nan,0)", "u64", (nan >= bb(0.0)) as u64);
}
