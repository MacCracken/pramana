//! `debug.tsv` — Rust's derived `Debug` of 1.3.0's types, `{:?}` and `{:#?}` (port plan D12; M1 part f): each
//! milestone adds its own types' rows.
//!
//! - `debug::SimpleRng`: the generator from 25 seeds (0, 1, 42, 2^63, u64::MAX and 20 seeded words), in both forms.
//! - `debug::PramanaError`: each variant with an empty message, the error pramana's codes carry (ADR 007 row 8, as
//!   amended), in both forms.
//!
//! Inputs: the seed or the variant, then `i:0` for `{:?}` or `i:1` for `{:#?}`. Output: the text. Class A.

use crate::grids::Rng as Seeds;
use crate::tsv::{Class, Module, Tok, i, s};
use pramana::{PramanaError, SimpleRng};

fn both<T: std::fmt::Debug>(x: &T) -> [String; 2] {
    [format!("{x:?}"), format!("{x:#?}")]
}

#[allow(clippy::cast_possible_wrap)]
pub fn emit() -> Module {
    let mut m = Module::new("debug");
    let mut g = Seeds::new(0x1f83_d9ab_fb41_bd6b);
    let mut seeds: Vec<u64> = vec![0, 1, 42, 1 << 63, u64::MAX];
    seeds.extend((0..20).map(|k| g.word() >> (k % 64)));
    for &seed in &seeds {
        for (alt, text) in both(&SimpleRng::new(seed)).iter().enumerate() {
            m.row("debug::SimpleRng", Class::A, &[i(seed as i64), i(alt as i64)], &[s(text.clone())]);
        }
    }
    for (name, e) in [
        ("InvalidParameter", PramanaError::InvalidParameter(String::new())),
        ("InvalidSample", PramanaError::InvalidSample(String::new())),
        ("ConvergenceFailure", PramanaError::ConvergenceFailure(String::new())),
        ("DimensionMismatch", PramanaError::DimensionMismatch(String::new())),
        ("ComputationError", PramanaError::ComputationError(String::new())),
    ] {
        for (alt, text) in both(&e).iter().enumerate() {
            m.row(
                "debug::PramanaError",
                Class::A,
                &[Tok::E("PramanaError", name.to_string()), i(alt as i64)],
                &[s(text.clone())],
            );
        }
    }
    m
}
