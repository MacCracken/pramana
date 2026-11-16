//! `combinatorics.tsv` — 1.3.0's counting functions (M1 part e2; port plan D10).
//!
//! - `factorial(n)` for n in 0 … 40, 2^32, 2^63 and u64::MAX: n > 34 is a ComputationError.
//! - `permutations(n, r)` and `combinations(n, r)` on probe P1's grid: n in 0 … 40, the u128 edge 130 … 133, 2^32,
//!   2^63 and u64::MAX, each against an r grid (0, 1, 2, 3, 5, 10, 20, 33 … 35, 64 … 67, and n − 2, n − 1, n,
//!   n + 1), and 200 seeded pairs. `combinations(130, 65)` overflows in an intermediate product, as in Rust.
//! - `stirling_approx(n)` for n in 0 … 200 (finite to 170, then ∞), every power of two and seeded words: class C
//!   (it calls powf), corrected against MPFR in `combinatorics.cr.tsv` by `validate.py`.
//!
//! A u128 is two `i:` tokens, its low word then its high word, each read as an i64.

use crate::grids::Rng;
use crate::tsv::{Class, Module, Tok, err, f, i};
use pramana::combinatorics::{combinations, factorial, permutations, stirling_approx};

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
fn u128_toks(x: u128) -> Vec<Tok> {
    vec![i(x as u64 as i64), i((x >> 64) as u64 as i64)]
}

fn outcome(r: Result<u128, pramana::PramanaError>) -> Vec<Tok> {
    match r {
        Ok(x) => u128_toks(x),
        Err(e) => vec![err(&e)],
    }
}

fn r_grid(n: u64) -> Vec<u64> {
    let mut v: Vec<u64> = vec![0, 1, 2, 3, 5, 10, 20, 33, 34, 35, 64, 65, 66, 67];
    v.extend([n.saturating_sub(2), n.saturating_sub(1), n, n.saturating_add(1)]);
    v.sort_unstable();
    v.dedup();
    v
}

#[allow(clippy::cast_possible_wrap)]
pub fn emit() -> Module {
    let mut m = Module::new("combinatorics");
    let mut ns: Vec<u64> = (0..=40).collect();
    ns.extend([130, 131, 132, 133, 1 << 32, 1 << 63, u64::MAX]);
    for &n in &ns {
        m.row("combinatorics::factorial", Class::A, &[i(n as i64)], &outcome(factorial(n)));
    }
    let mut pairs: Vec<(u64, u64)> = Vec::new();
    for &n in &ns {
        for r in r_grid(n) {
            pairs.push((n, r));
        }
    }
    let mut g = Rng::new(0xa54f_f53a_5f1d_36f1);
    for _ in 0..200 {
        let n = g.int(0, 140) as u64;
        let r = g.int(0, 141) as u64;
        pairs.push((n, r));
    }
    for &(n, r) in &pairs {
        m.row("combinatorics::permutations", Class::A, &[i(n as i64), i(r as i64)], &outcome(permutations(n, r)));
    }
    for &(n, r) in &pairs {
        m.row("combinatorics::combinations", Class::A, &[i(n as i64), i(r as i64)], &outcome(combinations(n, r)));
    }
    let mut sn: Vec<u64> = (0..=200).collect();
    sn.extend((0..64).map(|k| 1u64 << k));
    sn.push(u64::MAX);
    sn.extend((0..60).map(|k| g.word() >> (k % 64)));
    sn.sort_unstable();
    sn.dedup();
    for &n in &sn {
        m.row("combinatorics::stirling_approx", Class::C, &[i(n as i64)], &[f(stirling_approx(n))]);
    }
    m
}
