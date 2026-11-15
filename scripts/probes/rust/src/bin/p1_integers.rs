//! P1 (port plan §5 step 9): integer semantics.
//! - `SimpleRng`: 10^6 draws from each seed, through the oracle's own type, folded into a 64-bit FNV-1a digest of
//!   the draws' bits;
//! - `factorial`, `permutations`, `combinations` (u128 results): the oracle's own functions on a grid. Each case
//!   prints the outcome (0 ok, 1 InvalidParameter, 2 ComputationError) and the result's high and low words.

use pramana::combinatorics::{combinations, factorial, permutations};
use pramana::{PramanaError, Rng, SimpleRng};

fn w(case: &str, x: u64) {
    println!("{case}\tu64\t{x:#x}");
}

fn result(case: &str, r: Result<u128, PramanaError>) {
    match r {
        Ok(v) => {
            w(&format!("{case}.kind"), 0);
            w(&format!("{case}.hi"), (v >> 64) as u64);
            w(&format!("{case}.lo"), v as u64);
        }
        Err(PramanaError::InvalidParameter(_)) => w(&format!("{case}.kind"), 1),
        Err(_) => w(&format!("{case}.kind"), 2),
    }
}

fn seeds() -> [u64; 5] {
    [1, 42, 1 << 63, u64::MAX, 0x9e37_79b9_7f4a_7c15]
}

fn ns() -> Vec<u64> {
    let mut v: Vec<u64> = (0..=40).collect();
    v.extend([60, 66, 100, 130, 131, 132, 133, 1000, 1 << 32, (1 << 32) + 1, 1 << 63, u64::MAX - 1, u64::MAX]);
    v
}

fn rs(n: u64) -> Vec<u64> {
    let mut v = vec![0, 1, 2, 3, 5, 10, 20, 33, 34, 35, 64, 65, 66, 67];
    v.extend([n.saturating_sub(2), n.saturating_sub(1), n, n.saturating_add(1)]);
    v.sort_unstable();
    v.dedup();
    v
}

fn main() {
    for seed in seeds() {
        let mut rng = SimpleRng::new(seed);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for _ in 0..1_000_000 {
            h = (h ^ rng.next_f64().to_bits()).wrapping_mul(0x0000_0100_0000_01b3);
        }
        w(&format!("rng[{seed:#x}].digest1e6"), h);
    }
    for n in 0..=36_u64 {
        result(&format!("factorial({n:#x})"), factorial(n));
    }
    for n in ns() {
        for r in rs(n) {
            result(&format!("permutations({n:#x},{r:#x})"), permutations(n, r));
            result(&format!("combinations({n:#x},{r:#x})"), combinations(n, r));
        }
    }
}
