//! `math.rs`'s special functions, exposed unchanged for the oracle (see `crate::oracle`).

/// `erf(x)`: the fdlibm port.
#[must_use]
pub fn erf(x: f64) -> f64 {
    super::erf(x)
}

/// `erfc(x)`: the fdlibm port.
#[must_use]
pub fn erfc(x: f64) -> f64 {
    super::erfc(x)
}

/// Standard normal CDF, `½·erfc(−z/√2)`.
#[must_use]
pub fn normal_cdf(z: f64) -> f64 {
    super::normal_cdf(z)
}

/// Standard normal survival function, `½·erfc(z/√2)`.
#[must_use]
pub fn normal_sf(z: f64) -> f64 {
    super::normal_sf(z)
}

/// `ln|Γ(x)|` by Lanczos (g = 7, n = 9), with reflection for x < 0.5.
#[must_use]
pub fn ln_gamma(x: f64) -> f64 {
    super::ln_gamma(x)
}

/// The Stirling-series remainder `ln Γ(a) − [(a − ½) ln a − a + ln √(2π)]`.
#[must_use]
pub fn stirling_error(a: f64) -> f64 {
    super::stirling_error(a)
}

/// Loader's deviance term `bd0(x, m)`.
#[must_use]
pub fn bd0(x: f64, m: f64) -> f64 {
    super::bd0(x, m)
}

/// `ln(xᵃ e⁻ˣ / Γ(a))`, the incomplete-gamma prefactor kernel.
#[must_use]
pub fn ln_gamma_kernel(a: f64, x: f64) -> f64 {
    super::ln_gamma_kernel(a, x)
}

/// `ln B(a, b)`.
#[must_use]
pub fn ln_beta(a: f64, b: f64) -> f64 {
    super::ln_beta(a, b)
}

/// `I_x(a, b)`, the regularized incomplete beta function.
#[must_use]
pub fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    super::regularized_incomplete_beta(x, a, b)
}

/// Both tails `(I_x(a, b), 1 − I_x(a, b))`, with `y = 1 − x` passed separately.
#[must_use]
pub fn beta_inc(a: f64, b: f64, x: f64, y: f64) -> (f64, f64) {
    super::beta_inc(a, b, x, y)
}

/// `P(a, x)`, the regularized lower incomplete gamma function.
#[must_use]
pub fn regularized_lower_incomplete_gamma(a: f64, x: f64) -> f64 {
    super::regularized_lower_incomplete_gamma(a, x)
}

/// `Q(a, x)`, the regularized upper incomplete gamma function.
#[must_use]
pub fn regularized_upper_incomplete_gamma(a: f64, x: f64) -> f64 {
    super::regularized_upper_incomplete_gamma(a, x)
}

/// Both tails `(P(a, x), Q(a, x))`.
#[must_use]
pub fn gamma_inc(a: f64, x: f64) -> (f64, f64) {
    super::gamma_inc(a, x)
}
