//! `fmt.tsv` — the text serde_json 1.0.154 writes for an f64: the JSON every pramana type serializes to, which the
//! port writes byte for byte through `src/fmtf.cyr` (port plan D5 as amended after probe P7, ADR 007 row 7).
//!
//! serde_json prints the shortest digits that parse back to the same double, the closest to the exact value among
//! them and, on an exact tie between two, the even one (zmij). The form is positional for decimal exponents −5 … 15,
//! with ".0" on an integer, and otherwise `d.ddde±x`; NaN and ±∞ are `null`. Rust's `Display` (`{}`) of an f64 reaches
//! no output the port keeps: 1.3.0 uses it only inside error messages, which D6 drops. `Debug` (D12) and the `ai`
//! tools' `{:.4}` (M7) get their rows with the modules that print them.
//!
//! The inputs (probe P7 checked 280,562 doubles once; these keep the classes that found differences): the specials,
//! every power of two and its neighbours, every power of ten from 1e-323 to 1e308 and its neighbours, the integers to
//! 1,000, short decimals, a log-uniform sweep across the form switches, and seeded bit patterns, some negated.
//! Class A: the text, byte for byte.

use std::hint::black_box;

use crate::grids::{Rng, pow2, specials};
use crate::tsv::{Class, Module, f, s};

pub fn inputs() -> Vec<f64> {
    let mut v = specials();
    let near = |v: &mut Vec<f64>, x: f64| {
        let b = x.to_bits();
        v.push(f64::from_bits(b - 1));
        v.push(x);
        v.push(f64::from_bits(b + 1));
    };
    for k in -1074..=1023 {
        let x = pow2(k);
        if k == -1074 {
            v.push(x);
            v.push(f64::from_bits(2));
        } else {
            near(&mut v, x);
        }
    }
    for k in -323..=308 {
        let x: f64 = format!("1e{k}").parse().expect("a power of ten");
        near(&mut v, x);
    }
    v.extend((0..=1000).map(f64::from));
    for k in 1..=5000 {
        v.push(format!("{k}e-3").parse().expect("a short decimal"));
    }
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);
    v.extend((0..5000).map(|_| rng.log_uniform(-8.0, 18.0)));
    let mut bits = Vec::new();
    while bits.len() < 5_000 {
        let lo = rng.int(0, 0xFFFF_FFFF) as u64;
        let hi = rng.int(0, 0xFFFF_FFFF) as u64;
        let x = f64::from_bits((hi << 32) | lo);
        if x.is_finite() {
            bits.push(x);
        }
    }
    v.extend(bits.iter().take(1000).map(|x| -x));
    v.extend(bits);
    v
}

pub fn emit() -> Module {
    let mut m = Module::new("fmt");
    for x in inputs() {
        let json = serde_json::to_string(&black_box(x)).expect("serde_json writes every f64");
        m.row("f64::to_json", Class::A, &[f(x)], &[s(json)]);
    }
    m
}
