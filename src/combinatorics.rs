//! Combinatorial functions: factorial, permutations, combinations, Stirling's approximation.

use crate::error::PramanaError;
use std::f64::consts::PI;

/// Computes n! (factorial).
///
/// Returns 1 for n = 0.
///
/// # Errors
///
/// Returns `ComputationError` if the result overflows `u128` (n > 34).
#[must_use = "returns the factorial"]
pub fn factorial(n: u64) -> Result<u128, PramanaError> {
    if n > 34 {
        return Err(PramanaError::ComputationError(format!(
            "factorial({n}) overflows u128"
        )));
    }
    let mut result: u128 = 1;
    for i in 2..=n as u128 {
        result = result
            .checked_mul(i)
            .ok_or_else(|| PramanaError::ComputationError(format!("factorial({n}) overflow")))?;
    }
    Ok(result)
}

/// Computes P(n, r) = n! / (n-r)! (permutations of r items from n).
///
/// # Errors
///
/// Returns `InvalidParameter` if `r > n`.
/// Returns `ComputationError` on overflow.
#[must_use = "returns the number of permutations"]
pub fn permutations(n: u64, r: u64) -> Result<u128, PramanaError> {
    if r > n {
        return Err(PramanaError::InvalidParameter(format!(
            "r ({r}) must be <= n ({n})"
        )));
    }
    let mut result: u128 = 1;
    for i in (n - r + 1)..=n {
        result = result.checked_mul(i as u128).ok_or_else(|| {
            PramanaError::ComputationError(format!("permutations({n}, {r}) overflow"))
        })?;
    }
    Ok(result)
}

/// Computes C(n, r) = n! / (r! * (n-r)!) (combinations, "n choose r").
///
/// Uses the multiplicative formula to avoid intermediate overflow.
///
/// # Errors
///
/// Returns `InvalidParameter` if `r > n`.
/// Returns `ComputationError` on overflow.
#[must_use = "returns the number of combinations"]
pub fn combinations(n: u64, r: u64) -> Result<u128, PramanaError> {
    if r > n {
        return Err(PramanaError::InvalidParameter(format!(
            "r ({r}) must be <= n ({n})"
        )));
    }
    // Use the smaller of r and n-r to minimize multiplications
    let r = r.min(n - r);
    let mut result: u128 = 1;
    for i in 0..r {
        result = result.checked_mul((n - i) as u128).ok_or_else(|| {
            PramanaError::ComputationError(format!("combinations({n}, {r}) overflow"))
        })?;
        result /= (i + 1) as u128;
    }
    Ok(result)
}

/// Stirling's approximation to n!: sqrt(2*pi*n) * (n/e)^n.
///
/// Returns the approximation as a floating-point value.
#[must_use = "returns Stirling's approximation"]
#[inline]
pub fn stirling_approx(n: u64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let n_f = n as f64;
    (2.0 * PI * n_f).sqrt() * (n_f / std::f64::consts::E).powf(n_f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0).unwrap(), 1);
        assert_eq!(factorial(1).unwrap(), 1);
        assert_eq!(factorial(5).unwrap(), 120);
        assert_eq!(factorial(10).unwrap(), 3_628_800);
        assert_eq!(factorial(20).unwrap(), 2_432_902_008_176_640_000);
    }

    #[test]
    fn test_factorial_overflow() {
        assert!(factorial(35).is_err());
    }

    #[test]
    fn test_permutations() {
        assert_eq!(permutations(5, 3).unwrap(), 60);
        assert_eq!(permutations(10, 2).unwrap(), 90);
        assert_eq!(permutations(5, 0).unwrap(), 1);
        assert_eq!(permutations(5, 5).unwrap(), 120);
    }

    #[test]
    fn test_combinations() {
        assert_eq!(combinations(10, 3).unwrap(), 120);
        assert_eq!(combinations(5, 2).unwrap(), 10);
        assert_eq!(combinations(5, 0).unwrap(), 1);
        assert_eq!(combinations(5, 5).unwrap(), 1);
        assert_eq!(combinations(20, 10).unwrap(), 184_756);
    }

    #[test]
    fn test_combinations_r_gt_n() {
        assert!(combinations(3, 5).is_err());
    }

    #[test]
    fn test_stirling() {
        // Stirling for n=10 should be close to 10! = 3628800
        let approx = stirling_approx(10);
        let exact = 3_628_800.0;
        let relative_error = (approx - exact).abs() / exact;
        assert!(
            relative_error < 0.01,
            "relative error {relative_error} too large"
        );
    }
}
