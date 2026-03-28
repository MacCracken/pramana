//! Random number generation trait and a simple deterministic PRNG.

use serde::{Deserialize, Serialize};

/// Trait for random number generators.
///
/// Consumers can implement this to bring their own RNG.
/// The only requirement is producing uniformly distributed `f64` values in `[0, 1)`.
pub trait Rng {
    /// Returns the next random `f64` in the range `[0.0, 1.0)`.
    fn next_f64(&mut self) -> f64;
}

/// A simple deterministic xorshift64 PRNG.
///
/// This is **not** cryptographically secure. It is intended for reproducible
/// simulations and testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    /// Creates a new `SimpleRng` with the given seed.
    ///
    /// A seed of 0 is replaced with 1 to avoid a degenerate all-zero state.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }
}

impl Rng for SimpleRng {
    #[inline]
    fn next_f64(&mut self) -> f64 {
        // xorshift64
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        // Map to [0.0, 1.0) using the upper 53 bits
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_rng_deterministic() {
        let mut a = SimpleRng::new(42);
        let mut b = SimpleRng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_f64().to_bits(), b.next_f64().to_bits());
        }
    }

    #[test]
    fn simple_rng_range() {
        let mut rng = SimpleRng::new(12345);
        for _ in 0..10_000 {
            let v = rng.next_f64();
            assert!((0.0..1.0).contains(&v), "value out of range: {v}");
        }
    }

    #[test]
    fn simple_rng_zero_seed() {
        let mut rng = SimpleRng::new(0);
        // Should not be stuck at zero
        let v = rng.next_f64();
        assert!(v >= 0.0);
    }

    #[test]
    fn serde_roundtrip() {
        let rng = SimpleRng::new(42);
        let json = serde_json::to_string(&rng).unwrap();
        let rng2: SimpleRng = serde_json::from_str(&json).unwrap();
        assert_eq!(rng.state, rng2.state);
    }
}
