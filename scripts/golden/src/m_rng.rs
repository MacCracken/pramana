//! `rng.tsv` — 1.3.0's `SimpleRng` (M1 part e1): xorshift64 (13, 7, 17), each draw mapped to [0, 1) by its top 53
//! bits. Integer code, so class A.
//!
//! - `SimpleRng::next_f64`: the first 256 draws from 40 seeds: 0 (which `new` maps to 1), 1, 2, 42, 12345, 2^63,
//!   u64::MAX, two well-known constants, and 31 seeded words.
//! - `SimpleRng::digest`: 10^6 draws from six seeds, folded word by word into an FNV-1a digest of their bits
//!   (h = (h ^ bits) × 0x100000001b3, from 0xcbf29ce484222325), the plan's "10^6 RNG words" (§7, M1).

use crate::grids::Rng as Seeds;
use crate::tsv::{Class, Module, Tok, f, i};
use pramana::{Rng, SimpleRng};

const DRAWS: usize = 256;
const LONG: u64 = 1_000_000;

#[allow(clippy::cast_possible_wrap)]
pub fn emit() -> Module {
    let mut m = Module::new("rng");
    let mut g = Seeds::new(0x3c6e_f372_fe94_f82b);
    let mut seeds: Vec<u64> =
        vec![0, 1, 2, 42, 12345, 1 << 63, u64::MAX, 0x9e37_79b9_7f4a_7c15, 88_172_645_463_325_252];
    seeds.extend((0..31).map(|k| g.word() >> (k % 8)));
    for &seed in &seeds {
        let mut r = SimpleRng::new(seed);
        let mut out = vec![Tok::V(DRAWS)];
        out.extend((0..DRAWS).map(|_| f(r.next_f64())));
        m.row("SimpleRng::next_f64", Class::A, &[i(seed as i64), i(DRAWS as i64)], &out);
    }
    for seed in [0u64, 42, 1 << 63, u64::MAX, 0x9e37_79b9_7f4a_7c15, 88_172_645_463_325_252] {
        let mut r = SimpleRng::new(seed);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for _ in 0..LONG {
            h = (h ^ r.next_f64().to_bits()).wrapping_mul(0x0000_0100_0000_01b3);
        }
        m.row("SimpleRng::digest", Class::A, &[i(seed as i64), i(LONG as i64)], &[i(h as i64)]);
    }
    m
}
