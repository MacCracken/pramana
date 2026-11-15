//! Read-only hooks for the Cyrius port's golden-value generator.
//!
//! **Not public API.** This module exists only behind the hidden `oracle` feature, which
//! is never published (port plan D1 and M0 step 0, in
//! `docs/development/pramana-port-plan.md`). It lets the generator record pramana's
//! private special functions, samplers, quantiles and internal state as golden rows.
//!
//! Every hook calls the crate's own code unchanged; nothing here has an algorithm of its
//! own. The only additions are the two sampler guards, which return NaN for parameters
//! the crate never passes instead of looping forever. Hooks take `&` borrows or plain
//! values, so they cannot change a model.
//!
//! `tests/oracle.rs` checks two things:
//! - each hook gives, bit for bit, what the public API computes through the same private code;
//! - each sampler hook consumes exactly the uniforms the public sampler does.

pub use crate::descriptive::oracle::kernel_evaluate;
pub use crate::distribution::oracle::{
    f_cdf_sf, mvn_factor, sample_gamma_standard, sample_poisson_ptrs, standard_normal,
    student_t_cdf, student_t_two_tailed,
};
pub use crate::hypothesis::oracle::{
    chi_squared_pvalue, kolmogorov_sf, ks_pvalue, t_quantile, welch_stats, z_quantile,
};
pub use crate::markov::oracle::{hmm_forward_scaled, hmm_log_likelihood};
pub use crate::math::oracle::{
    bd0, beta_inc, erf, erfc, gamma_inc, ln_beta, ln_gamma, ln_gamma_kernel, normal_cdf, normal_sf,
    regularized_incomplete_beta, regularized_lower_incomplete_gamma,
    regularized_upper_incomplete_gamma, stirling_error,
};
