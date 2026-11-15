//! The KDE kernels, exposed unchanged for the oracle (see `crate::oracle`).

use super::Kernel;

/// The unscaled kernel `K(u)` that `KernelDensity::evaluate` sums.
#[must_use]
pub fn kernel_evaluate(kernel: Kernel, u: f64) -> f64 {
    kernel.evaluate(u)
}
