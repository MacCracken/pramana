#!/usr/bin/env python3
"""Check the port's public surface against Rust 1.3.0's, module by module (port plan §7: every milestone's gate).

    python3 -I -B scripts/surface_parity.py [--check]

For each ported module of `rust-old/src/` (PORTED), this lists every public item:
- each `pub fn`, `pub struct`, `pub enum`, `pub trait` and `pub type` outside the tests;
- each `pub fn` in an inherent `impl T` block, and each method of a `pub trait` and of an `impl Trait for T`;
- each derive on a public type (`Debug`, `Clone`, `Serialize`, `Deserialize`, and `thiserror::Error` as `Error`).

SURFACE maps each one, by key, to the Cyrius symbols that port it (`fn f`, `struct S`, `enum E`, `trait T`, `method
S.m`), or to a note naming the decision that ports it without a symbol. It checks that every listed Rust item has an
entry, that every entry is for an item 1.3.0 has, and that `src/` defines every symbol named. It prints the table;
with `--check` (the gate) it exits 1 on any failure. Stdlib only.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST = ROOT / "rust-old" / "src"
SRC = ROOT / "src"

# The modules ported so far, in the plan's order. A milestone adds its modules here with their SURFACE entries.
PORTED = ["error", "rng", "combinatorics", "bridge", "distribution", "descriptive", "hypothesis", "regression",
          "timeseries", "bayesian", "markov", "monte_carlo"]   # M6's gate
# Modules in progress, part by part: an item without a SURFACE entry is listed as pending, not failed, until the
# module moves to PORTED at its milestone's gate.
PARTIAL = {}

SURFACE = {
    # descriptive.rs (M4 part a1; ADR 007 rows 1, 3 and 14)
    "descriptive::mean": ["fn pmn_mean"],
    "descriptive::median": ["fn pmn_median", "fn _pmn_select_pair"],
    "descriptive::mode": ["fn pmn_mode"],
    "descriptive::variance": ["fn pmn_variance"],
    "descriptive::std_dev": ["fn pmn_std_dev"],
    "descriptive::skewness": ["fn pmn_skewness"],
    "descriptive::kurtosis": ["fn pmn_kurtosis"],
    "descriptive::percentile": ["fn pmn_percentile"],
    "descriptive::quartiles": ["fn pmn_quartiles"],
    "descriptive::iqr": ["fn pmn_iqr"],
    "descriptive::min": ["fn pmn_min"],
    "descriptive::max": ["fn pmn_max"],
    "descriptive::range": ["fn pmn_range"],
    "descriptive::sum": ["fn pmn_sum"],
    # descriptive.rs's KDE and correlation (M4 part a2; D5, D11, D12)
    "descriptive::Kernel": ["enum PmnKernel", "fn pmn_kernel_name"],
    "descriptive::Kernel#Clone": "a kernel is a word: copying it clones it",
    "descriptive::Kernel#Debug": ["fn pmn_kernel_debug"],
    "descriptive::Kernel#Serialize": ["fn pmn_kernel_to_json"],
    "descriptive::Kernel#Deserialize": ["fn pmn_kernel_from_json"],
    "descriptive::KernelDensity": ["struct PmnKernelDensity", "fn PmnKernelDensity_drop"],
    "descriptive::KernelDensity#Clone": ["fn PmnKernelDensity_clone"],
    "descriptive::KernelDensity#Debug": ["method PmnKernelDensity.debug"],
    "descriptive::KernelDensity#Serialize": ["method PmnKernelDensity.to_json"],
    "descriptive::KernelDensity#Deserialize": ["fn PmnKernelDensity_from_json"],
    "descriptive::KernelDensity.new": ["fn PmnKernelDensity_new"],
    "descriptive::KernelDensity.silverman": ["fn PmnKernelDensity_silverman"],
    "descriptive::KernelDensity.evaluate": ["method PmnKernelDensity.evaluate"],
    "descriptive::KernelDensity.evaluate_grid": ["method PmnKernelDensity.evaluate_grid"],
    "descriptive::correlation_matrix": ["fn pmn_correlation_matrix"],
    # markov.rs (M6 part a; D5, D7, D11, D12)
    "markov::MarkovChain": ["struct PmnMarkovChain", "fn PmnMarkovChain_drop", "method PmnMarkovChain.transition_matrix"],
    "markov::MarkovChain#Clone": ["fn PmnMarkovChain_clone"],
    "markov::MarkovChain#Debug": ["method PmnMarkovChain.debug"],
    "markov::MarkovChain#Serialize": ["method PmnMarkovChain.to_json"],
    "markov::MarkovChain#Deserialize": ["fn PmnMarkovChain_from_json"],
    "markov::MarkovChain.new": ["fn PmnMarkovChain_new"],
    "markov::MarkovChain.num_states": ["method PmnMarkovChain.num_states"],
    "markov::MarkovChain.step": ["method PmnMarkovChain.step"],
    "markov::MarkovChain.simulate": ["method PmnMarkovChain.simulate"],
    "markov::MarkovChain.steady_state": ["method PmnMarkovChain.steady_state"],
    "markov::HiddenMarkovModel": ["struct PmnHiddenMarkovModel", "fn PmnHiddenMarkovModel_drop",
                                  "method PmnHiddenMarkovModel.transition", "method PmnHiddenMarkovModel.emission",
                                  "method PmnHiddenMarkovModel.initial"],
    "markov::HiddenMarkovModel#Clone": ["fn PmnHiddenMarkovModel_clone"],
    "markov::HiddenMarkovModel#Debug": ["method PmnHiddenMarkovModel.debug"],
    "markov::HiddenMarkovModel#Serialize": ["method PmnHiddenMarkovModel.to_json"],
    "markov::HiddenMarkovModel#Deserialize": ["fn PmnHiddenMarkovModel_from_json"],
    "markov::HiddenMarkovModel.new": ["fn PmnHiddenMarkovModel_new"],
    "markov::HiddenMarkovModel.num_states": ["method PmnHiddenMarkovModel.num_states"],
    "markov::HiddenMarkovModel.num_symbols": ["method PmnHiddenMarkovModel.num_symbols"],
    "markov::HiddenMarkovModel.forward_log_likelihood": ["method PmnHiddenMarkovModel.forward_log_likelihood"],
    "markov::HiddenMarkovModel.viterbi": ["method PmnHiddenMarkovModel.viterbi"],
    "markov::HiddenMarkovModel.baum_welch": ["method PmnHiddenMarkovModel.baum_welch"],
    # monte_carlo.rs (M6 part b; D5, D7, D11, D12)
    "monte_carlo::EstimateResult": ["struct PmnEstimateResult"],
    "monte_carlo::EstimateResult#Clone": "a plain struct: copying it clones it",
    "monte_carlo::EstimateResult#Debug": ["method PmnEstimateResult.debug"],
    "monte_carlo::EstimateResult#Serialize": ["method PmnEstimateResult.to_json"],
    "monte_carlo::EstimateResult#Deserialize": ["fn PmnEstimateResult_from_json"],
    "monte_carlo::monte_carlo_integrate": ["fn pmn_monte_carlo_integrate"],
    "monte_carlo::monte_carlo_pi": ["fn pmn_monte_carlo_pi"],
    "monte_carlo::McmcResult": ["struct PmnMcmcResult", "fn PmnMcmcResult_drop", "method PmnMcmcResult.n_samples",
                                "method PmnMcmcResult.sample", "method PmnMcmcResult.sample_len",
                                "method PmnMcmcResult.samples"],
    "monte_carlo::McmcResult#Clone": ["fn PmnMcmcResult_clone"],
    "monte_carlo::McmcResult#Debug": ["method PmnMcmcResult.debug"],
    "monte_carlo::McmcResult#Serialize": ["method PmnMcmcResult.to_json"],
    "monte_carlo::McmcResult#Deserialize": ["fn PmnMcmcResult_from_json"],
    "monte_carlo::metropolis_hastings": ["fn pmn_metropolis_hastings"],
    "monte_carlo::gibbs_sampling": ["fn pmn_gibbs_sampling", "struct PmnRngRef"],
    # bayesian.rs (M5 part c)
    "bayesian::BayesianUpdate": ["struct PmnBayesianUpdate"],
    "bayesian::BayesianUpdate#Clone": "a plain struct: copying it clones it",
    "bayesian::BayesianUpdate#Debug": ["method PmnBayesianUpdate.debug"],
    "bayesian::BayesianUpdate#Serialize": ["method PmnBayesianUpdate.to_json"],
    "bayesian::BayesianUpdate#Deserialize": ["fn PmnBayesianUpdate_from_json"],
    "bayesian::BayesianUpdate.new": ["fn PmnBayesianUpdate_new"],
    "bayesian::BayesianUpdate.posterior": ["method PmnBayesianUpdate.posterior"],
    "bayesian::bayes_theorem": ["fn pmn_bayes_theorem"],
    "bayesian::naive_bayes_classify": ["fn pmn_naive_bayes_classify"],
    # timeseries.rs (M5 part b; D11's caller buffers)
    "timeseries::moving_average": ["fn pmn_moving_average"],
    "timeseries::exponential_smoothing": ["fn pmn_exponential_smoothing"],
    "timeseries::autocorrelation": ["fn pmn_autocorrelation"],
    "timeseries::difference": ["fn pmn_difference"],
    "timeseries::integrate": ["fn pmn_integrate"],
    "timeseries::ArimaModel": ["struct PmnArimaModel", "fn PmnArimaModel_drop", "method PmnArimaModel.ar_coefficients"],
    "timeseries::ArimaModel#Clone": ["fn PmnArimaModel_clone"],
    "timeseries::ArimaModel#Debug": ["method PmnArimaModel.debug"],
    "timeseries::ArimaModel#Serialize": ["method PmnArimaModel.to_json"],
    "timeseries::ArimaModel#Deserialize": ["fn PmnArimaModel_from_json"],
    "timeseries::arima_fit": ["fn pmn_arima_fit"],
    "timeseries::arima_forecast": ["fn pmn_arima_forecast"],
    # regression.rs (M5 part a; hisab's least_squares_poly, ledger L5)
    "regression::LinearModel": ["struct PmnLinearModel"],
    "regression::LinearModel#Clone": "a plain struct: copying it clones it",
    "regression::LinearModel#Debug": ["method PmnLinearModel.debug"],
    "regression::LinearModel#Serialize": ["method PmnLinearModel.to_json"],
    "regression::LinearModel#Deserialize": ["fn PmnLinearModel_from_json"],
    "regression::linear_regression": ["fn pmn_linear_regression"],
    "regression::predict": ["fn pmn_predict"],
    "regression::residuals": ["fn pmn_residuals"],
    "regression::PolynomialModel": ["struct PmnPolynomialModel", "fn PmnPolynomialModel_drop",
                                    "method PmnPolynomialModel.coefficients"],
    "regression::PolynomialModel#Clone": ["fn PmnPolynomialModel_clone"],
    "regression::PolynomialModel#Debug": ["method PmnPolynomialModel.debug"],
    "regression::PolynomialModel#Serialize": ["method PmnPolynomialModel.to_json"],
    "regression::PolynomialModel#Deserialize": ["fn PmnPolynomialModel_from_json"],
    "regression::polynomial_regression": ["fn pmn_polynomial_regression", "fn _pmn_least_squares_poly"],
    "regression::predict_poly": ["fn pmn_predict_poly"],
    "regression::residuals_poly": ["fn pmn_residuals_poly"],
    "regression::LogisticModel": ["struct PmnLogisticModel", "fn PmnLogisticModel_drop",
                                  "method PmnLogisticModel.coefficients"],
    "regression::LogisticModel#Clone": ["fn PmnLogisticModel_clone"],
    "regression::LogisticModel#Debug": ["method PmnLogisticModel.debug"],
    "regression::LogisticModel#Serialize": ["method PmnLogisticModel.to_json"],
    "regression::LogisticModel#Deserialize": ["fn PmnLogisticModel_from_json"],
    "regression::logistic_regression": ["fn pmn_logistic_regression", "fn _pmn_sigmoid"],
    "regression::predict_logistic_proba": ["fn pmn_predict_logistic_proba"],
    "regression::predict_logistic_class": ["fn pmn_predict_logistic_class"],
    # hypothesis.rs's tests (M4 part b1; D7's callback, ADR 007 rows 1, 14 and 15)
    "hypothesis::TestResult": ["struct PmnTestResult", "fn PmnTestResult_drop"],
    "hypothesis::TestResult#Clone": ["fn PmnTestResult_clone"],
    "hypothesis::TestResult#Debug": ["method PmnTestResult.debug"],
    "hypothesis::TestResult#Serialize": ["method PmnTestResult.to_json"],
    "hypothesis::TestResult#Deserialize": ["fn PmnTestResult_from_json"],
    "hypothesis::AnovaResult": ["struct PmnAnovaResult"],
    "hypothesis::AnovaResult#Clone": "a plain struct: copying it clones it",
    "hypothesis::AnovaResult#Debug": ["method PmnAnovaResult.debug"],
    "hypothesis::AnovaResult#Serialize": ["method PmnAnovaResult.to_json"],
    "hypothesis::AnovaResult#Deserialize": ["fn PmnAnovaResult_from_json"],
    "hypothesis::KsResult": ["struct PmnKsResult"],
    "hypothesis::KsResult#Clone": "a plain struct: copying it clones it",
    "hypothesis::KsResult#Debug": ["method PmnKsResult.debug"],
    "hypothesis::KsResult#Serialize": ["method PmnKsResult.to_json"],
    "hypothesis::KsResult#Deserialize": ["fn PmnKsResult_from_json"],
    "hypothesis::t_test_one_sample": ["fn pmn_t_test_one_sample"],
    "hypothesis::t_test_two_sample": ["fn pmn_t_test_two_sample", "fn _pmn_welch"],
    "hypothesis::chi_squared_test": ["fn pmn_chi_squared_test"],
    "hypothesis::one_way_anova": ["fn pmn_one_way_anova"],
    "hypothesis::ks_two_sample": ["fn pmn_ks_two_sample", "fn _pmn_kolmogorov_sf"],
    "hypothesis::ks_one_sample": ["fn pmn_ks_one_sample"],
    # hypothesis.rs's intervals (M4 part b2)
    "hypothesis::ConfidenceInterval": ["struct PmnConfidenceInterval"],
    "hypothesis::ConfidenceInterval#Clone": "a plain struct: copying it clones it",
    "hypothesis::ConfidenceInterval#Debug": ["method PmnConfidenceInterval.debug"],
    "hypothesis::ConfidenceInterval#Serialize": ["method PmnConfidenceInterval.to_json"],
    "hypothesis::ConfidenceInterval#Deserialize": ["fn PmnConfidenceInterval_from_json"],
    "hypothesis::ci_mean": ["fn pmn_ci_mean", "fn _pmn_t_quantile"],
    "hypothesis::ci_two_means": ["fn pmn_ci_two_means"],
    "hypothesis::ci_proportion": ["fn pmn_ci_proportion", "fn _pmn_z_quantile"],
    # descriptive.rs's PCA (M4 part a3; hisab's Jacobi, ledger L6)
    "descriptive::pca": ["fn pmn_pca", "fn _pmn_eigen_symmetric"],
    "descriptive::PcaResult": ["struct PmnPcaResult", "fn PmnPcaResult_drop", "method PmnPcaResult.component",
                               "method PmnPcaResult.eigenvalues", "method PmnPcaResult.explained_variance_ratio"],
    "descriptive::PcaResult#Clone": ["fn PmnPcaResult_clone"],
    "descriptive::PcaResult#Debug": ["method PmnPcaResult.debug"],
    "descriptive::PcaResult#Serialize": ["method PmnPcaResult.to_json"],
    "descriptive::PcaResult#Deserialize": ["fn PmnPcaResult_from_json"],
    # error.rs (M1 part a; D6, ADR 007 rows 1 and 8)
    "error::PramanaError": ["enum PmnError", "fn pmn_err_name"],
    "error::PramanaError#Debug": ["fn pmn_err_debug"],
    "error::PramanaError#Clone": "a code is a word: copying it clones it (D6)",
    "error::PramanaError#Serialize": ["fn pmn_err_to_json"],
    "error::PramanaError#Deserialize": ["fn pmn_err_from_json"],
    "error::PramanaError#Error": ["fn pmn_err_message"],
    # rng.rs (M1 part e1; D7 as amended)
    "rng::Rng": ["trait PmnRng"],
    "rng::Rng.next_f64": ["method PmnRng.next_f64"],
    "rng::SimpleRng": ["struct PmnSimpleRng"],
    "rng::SimpleRng.new": ["fn PmnSimpleRng_new"],
    "rng::SimpleRng.next_f64": ["method PmnSimpleRng.next_f64"],
    "rng::SimpleRng#Debug": ["method PmnSimpleRng.debug"],
    "rng::SimpleRng#Clone": "a one-word struct: copying it clones it",
    "rng::SimpleRng#Serialize": ["method PmnSimpleRng.to_json"],
    "rng::SimpleRng#Deserialize": ["fn PmnSimpleRng_from_json"],
    # combinatorics.rs (M1 part e2; D10)
    "combinatorics::factorial": ["fn pmn_factorial", "struct PmnU128"],
    "combinatorics::permutations": ["fn pmn_permutations"],
    "combinatorics::combinations": ["fn pmn_combinations"],
    "combinatorics::stirling_approx": ["fn pmn_stirling_approx"],
    # bridge.rs (M1 part e3)
    "bridge::ensemble_spread_to_confidence": ["fn pmn_ensemble_spread_to_confidence"],
    "bridge::observation_error_to_likelihood": ["fn pmn_observation_error_to_likelihood"],
    "bridge::yields_to_mean_variance": ["fn pmn_yields_to_mean_variance"],
    "bridge::replicates_to_t_statistic": ["fn pmn_replicates_to_t_statistic"],
    "bridge::uncertainty_to_gaussian": ["fn pmn_uncertainty_to_gaussian"],
    "bridge::sensor_noise_to_std_dev": ["fn pmn_sensor_noise_to_std_dev"],
    "bridge::item_variances_to_alpha": ["fn pmn_item_variances_to_alpha"],
    "bridge::z_rates_to_d_prime": ["fn pmn_z_rates_to_d_prime"],
    "bridge::trait_scores_to_correlation": ["fn pmn_trait_scores_to_correlation"],
    "bridge::learning_trials_to_power_law": ["fn pmn_learning_trials_to_power_law"],
    "bridge::response_times_to_hick_params": ["fn pmn_response_times_to_hick_params"],
    # distribution.rs (M3; D7 as amended, D11, D5, D12). Part a1: the trait and Normal.
    "distribution::Distribution": ["trait PmnDistribution"],
    "distribution::Distribution.pdf": ["method PmnDistribution.pdf"],
    "distribution::Distribution.cdf": ["method PmnDistribution.cdf"],
    "distribution::Distribution.mean": ["method PmnDistribution.mean"],
    "distribution::Distribution.variance": ["method PmnDistribution.variance"],
    "distribution::Distribution.sample": ["method PmnDistribution.sample"],
    "distribution::Normal": ["struct PmnNormal"],
    "distribution::Normal.new": ["fn PmnNormal_new"],
    "distribution::Normal.pdf": ["method PmnNormal.pdf"],
    "distribution::Normal.cdf": ["method PmnNormal.cdf"],
    "distribution::Normal.mean": ["method PmnNormal.mean"],
    "distribution::Normal.variance": ["method PmnNormal.variance"],
    "distribution::Normal.sample": ["method PmnNormal.sample"],
    "distribution::Normal#Debug": ["method PmnNormal.debug"],
    "distribution::Normal#Clone": "a struct of two f64s: copying it clones it",
    "distribution::Normal#Serialize": ["method PmnNormal.to_json"],
    "distribution::Normal#Deserialize": ["fn PmnNormal_from_json"],
    # Part a2: Uniform, Exponential and Bernoulli.
    "distribution::Uniform": ["struct PmnUniform"],
    "distribution::Uniform.new": ["fn PmnUniform_new"],
    "distribution::Uniform.pdf": ["method PmnUniform.pdf"],
    "distribution::Uniform.cdf": ["method PmnUniform.cdf"],
    "distribution::Uniform.mean": ["method PmnUniform.mean"],
    "distribution::Uniform.variance": ["method PmnUniform.variance"],
    "distribution::Uniform.sample": ["method PmnUniform.sample"],
    "distribution::Uniform#Debug": ["method PmnUniform.debug"],
    "distribution::Uniform#Clone": "a struct of two f64s: copying it clones it",
    "distribution::Uniform#Serialize": ["method PmnUniform.to_json"],
    "distribution::Uniform#Deserialize": ["fn PmnUniform_from_json"],
    "distribution::Exponential": ["struct PmnExponential"],
    "distribution::Exponential.new": ["fn PmnExponential_new"],
    "distribution::Exponential.pdf": ["method PmnExponential.pdf"],
    "distribution::Exponential.cdf": ["method PmnExponential.cdf"],
    "distribution::Exponential.mean": ["method PmnExponential.mean"],
    "distribution::Exponential.variance": ["method PmnExponential.variance"],
    "distribution::Exponential.sample": ["method PmnExponential.sample"],
    "distribution::Exponential#Debug": ["method PmnExponential.debug"],
    "distribution::Exponential#Clone": "a struct of one f64: copying it clones it",
    "distribution::Exponential#Serialize": ["method PmnExponential.to_json"],
    "distribution::Exponential#Deserialize": ["fn PmnExponential_from_json"],
    "distribution::Bernoulli": ["struct PmnBernoulli"],
    "distribution::Bernoulli.new": ["fn PmnBernoulli_new"],
    "distribution::Bernoulli.pdf": ["method PmnBernoulli.pdf"],
    "distribution::Bernoulli.cdf": ["method PmnBernoulli.cdf"],
    "distribution::Bernoulli.mean": ["method PmnBernoulli.mean"],
    "distribution::Bernoulli.variance": ["method PmnBernoulli.variance"],
    "distribution::Bernoulli.sample": ["method PmnBernoulli.sample"],
    "distribution::Bernoulli#Debug": ["method PmnBernoulli.debug"],
    "distribution::Bernoulli#Clone": "a struct of one f64: copying it clones it",
    "distribution::Bernoulli#Serialize": ["method PmnBernoulli.to_json"],
    "distribution::Bernoulli#Deserialize": ["fn PmnBernoulli_from_json"],
    # Part a3: Cauchy and Weibull.
    "distribution::Cauchy": ["struct PmnCauchy"],
    "distribution::Cauchy.new": ["fn PmnCauchy_new"],
    "distribution::Cauchy.pdf": ["method PmnCauchy.pdf"],
    "distribution::Cauchy.cdf": ["method PmnCauchy.cdf"],
    "distribution::Cauchy.mean": ["method PmnCauchy.mean"],
    "distribution::Cauchy.variance": ["method PmnCauchy.variance"],
    "distribution::Cauchy.sample": ["method PmnCauchy.sample"],
    "distribution::Cauchy#Debug": ["method PmnCauchy.debug"],
    "distribution::Cauchy#Clone": "a struct of two f64s: copying it clones it",
    "distribution::Cauchy#Serialize": ["method PmnCauchy.to_json"],
    "distribution::Cauchy#Deserialize": ["fn PmnCauchy_from_json"],
    "distribution::Weibull": ["struct PmnWeibull"],
    "distribution::Weibull.new": ["fn PmnWeibull_new"],
    "distribution::Weibull.pdf": ["method PmnWeibull.pdf"],
    "distribution::Weibull.cdf": ["method PmnWeibull.cdf"],
    "distribution::Weibull.mean": ["method PmnWeibull.mean"],
    "distribution::Weibull.variance": ["method PmnWeibull.variance"],
    "distribution::Weibull.sample": ["method PmnWeibull.sample"],
    "distribution::Weibull#Debug": ["method PmnWeibull.debug"],
    "distribution::Weibull#Clone": "a struct of two f64s: copying it clones it",
    "distribution::Weibull#Serialize": ["method PmnWeibull.to_json"],
    "distribution::Weibull#Deserialize": ["fn PmnWeibull_from_json"],
    # Part b1: Gamma and ChiSquared.
    "distribution::Gamma": ["struct PmnGamma"],
    "distribution::Gamma.new": ["fn PmnGamma_new"],
    "distribution::Gamma.pdf": ["method PmnGamma.pdf"],
    "distribution::Gamma.cdf": ["method PmnGamma.cdf"],
    "distribution::Gamma.mean": ["method PmnGamma.mean"],
    "distribution::Gamma.variance": ["method PmnGamma.variance"],
    "distribution::Gamma.sample": ["method PmnGamma.sample"],
    "distribution::Gamma#Debug": ["method PmnGamma.debug"],
    "distribution::Gamma#Clone": "a struct of two f64s: copying it clones it",
    "distribution::Gamma#Serialize": ["method PmnGamma.to_json"],
    "distribution::Gamma#Deserialize": ["fn PmnGamma_from_json"],
    "distribution::ChiSquared": ["struct PmnChiSquared"],
    "distribution::ChiSquared.new": ["fn PmnChiSquared_new"],
    "distribution::ChiSquared.pdf": ["method PmnChiSquared.pdf"],
    "distribution::ChiSquared.cdf": ["method PmnChiSquared.cdf"],
    "distribution::ChiSquared.mean": ["method PmnChiSquared.mean"],
    "distribution::ChiSquared.variance": ["method PmnChiSquared.variance"],
    "distribution::ChiSquared.sample": ["method PmnChiSquared.sample"],
    "distribution::ChiSquared#Debug": ["method PmnChiSquared.debug"],
    "distribution::ChiSquared#Clone": "a struct of one f64: copying it clones it",
    "distribution::ChiSquared#Serialize": ["method PmnChiSquared.to_json"],
    "distribution::ChiSquared#Deserialize": ["fn PmnChiSquared_from_json"],
    # Part b2: Beta, StudentT and FDistribution.
    "distribution::Beta": ["struct PmnBeta"],
    "distribution::Beta.new": ["fn PmnBeta_new"],
    "distribution::Beta.pdf": ["method PmnBeta.pdf"],
    "distribution::Beta.cdf": ["method PmnBeta.cdf"],
    "distribution::Beta.mean": ["method PmnBeta.mean"],
    "distribution::Beta.variance": ["method PmnBeta.variance"],
    "distribution::Beta.sample": ["method PmnBeta.sample"],
    "distribution::Beta#Debug": ["method PmnBeta.debug"],
    "distribution::Beta#Clone": "a struct of two f64s: copying it clones it",
    "distribution::Beta#Serialize": ["method PmnBeta.to_json"],
    "distribution::Beta#Deserialize": ["fn PmnBeta_from_json"],
    "distribution::StudentT": ["struct PmnStudentT"],
    "distribution::StudentT.new": ["fn PmnStudentT_new"],
    "distribution::StudentT.pdf": ["method PmnStudentT.pdf"],
    "distribution::StudentT.cdf": ["method PmnStudentT.cdf"],
    "distribution::StudentT.mean": ["method PmnStudentT.mean"],
    "distribution::StudentT.variance": ["method PmnStudentT.variance"],
    "distribution::StudentT.sample": ["method PmnStudentT.sample"],
    "distribution::StudentT#Debug": ["method PmnStudentT.debug"],
    "distribution::StudentT#Clone": "a struct of one f64: copying it clones it",
    "distribution::StudentT#Serialize": ["method PmnStudentT.to_json"],
    "distribution::StudentT#Deserialize": ["fn PmnStudentT_from_json"],
    "distribution::FDistribution": ["struct PmnFDistribution"],
    "distribution::FDistribution.new": ["fn PmnFDistribution_new"],
    "distribution::FDistribution.pdf": ["method PmnFDistribution.pdf"],
    "distribution::FDistribution.cdf": ["method PmnFDistribution.cdf"],
    "distribution::FDistribution.mean": ["method PmnFDistribution.mean"],
    "distribution::FDistribution.variance": ["method PmnFDistribution.variance"],
    "distribution::FDistribution.sample": ["method PmnFDistribution.sample"],
    "distribution::FDistribution#Debug": ["method PmnFDistribution.debug"],
    "distribution::FDistribution#Clone": "a struct of two f64s: copying it clones it",
    "distribution::FDistribution#Serialize": ["method PmnFDistribution.to_json"],
    "distribution::FDistribution#Deserialize": ["fn PmnFDistribution_from_json"],
    # Part c1: Poisson.
    "distribution::Poisson": ["struct PmnPoisson"],
    "distribution::Poisson.new": ["fn PmnPoisson_new"],
    "distribution::Poisson.pdf": ["method PmnPoisson.pdf"],
    "distribution::Poisson.cdf": ["method PmnPoisson.cdf"],
    "distribution::Poisson.mean": ["method PmnPoisson.mean"],
    "distribution::Poisson.variance": ["method PmnPoisson.variance"],
    "distribution::Poisson.sample": ["method PmnPoisson.sample"],
    "distribution::Poisson#Debug": ["method PmnPoisson.debug"],
    "distribution::Poisson#Clone": "a struct of one f64: copying it clones it",
    "distribution::Poisson#Serialize": ["method PmnPoisson.to_json"],
    "distribution::Poisson#Deserialize": ["fn PmnPoisson_from_json"],
    # Part c2: Binomial.
    "distribution::Binomial": ["struct PmnBinomial"],
    "distribution::Binomial.new": ["fn PmnBinomial_new"],
    "distribution::Binomial.pdf": ["method PmnBinomial.pdf"],
    "distribution::Binomial.cdf": ["method PmnBinomial.cdf"],
    "distribution::Binomial.mean": ["method PmnBinomial.mean"],
    "distribution::Binomial.variance": ["method PmnBinomial.variance"],
    "distribution::Binomial.sample": ["method PmnBinomial.sample"],
    "distribution::Binomial#Debug": ["method PmnBinomial.debug"],
    "distribution::Binomial#Clone": "a struct of a u64 word and an f64: copying it clones it",
    "distribution::Binomial#Serialize": ["method PmnBinomial.to_json"],
    "distribution::Binomial#Deserialize": ["fn PmnBinomial_from_json"],
    # Part d2: MultivariateNormal, the one owned distribution (D11): a freelist block, freed by its drop.
    "distribution::MultivariateNormal": ["struct PmnMultivariateNormal", "fn PmnMultivariateNormal_drop"],
    "distribution::MultivariateNormal.new": ["fn PmnMultivariateNormal_new"],
    "distribution::MultivariateNormal.dim": ["method PmnMultivariateNormal.dim"],
    "distribution::MultivariateNormal.pdf": ["method PmnMultivariateNormal.pdf"],
    "distribution::MultivariateNormal.log_pdf": ["method PmnMultivariateNormal.log_pdf"],
    "distribution::MultivariateNormal.sample": ["method PmnMultivariateNormal.sample"],
    "distribution::MultivariateNormal.mean_vec": ["method PmnMultivariateNormal.mean_vec"],
    "distribution::MultivariateNormal.covariance_matrix": ["method PmnMultivariateNormal.covariance_matrix"],
    "distribution::MultivariateNormal#Debug": ["method PmnMultivariateNormal.debug"],
    "distribution::MultivariateNormal#Clone": ["fn PmnMultivariateNormal_clone"],
    "distribution::MultivariateNormal#Serialize": ["method PmnMultivariateNormal.to_json"],
    "distribution::MultivariateNormal#Deserialize": ["fn PmnMultivariateNormal_from_json"],
}

DERIVES = {"Debug", "Clone", "Serialize", "Deserialize", "Error"}


def library_part(text):
    """The module's code before its `#[cfg(test)]` block."""
    at = text.find("#[cfg(test)]")
    return text if at < 0 else text[:at]


def block_end(text, at):
    """The index just past the `}` matching the first `{` at or after at."""
    i = text.index("{", at)
    depth = 0
    while True:
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1


def rust_items(module):
    """The public items of rust-old/src/<module>.rs, as SURFACE keys."""
    text = library_part((RUST / f"{module}.rs").read_text(encoding="utf-8"))
    text = re.sub(r"//[^\n]*", "", text)
    items = []
    public_types = set()
    for m in re.finditer(r"((?:#\[[^\]]*\]\s*)*)pub (struct|enum|trait|type) (\w+)", text):
        attrs, kind, name = m.groups()
        items.append(f"{module}::{name}")
        public_types.add(name)
        for d in re.findall(r"#\[derive\(([^)]*)\)\]", attrs):
            for word in re.split(r"[,\s]+", d):
                word = word.split("::")[-1]
                if word in DERIVES:
                    items.append(f"{module}::{name}#{word}")
        if kind == "trait":
            body = text[m.end():block_end(text, m.end())]
            items.extend(f"{module}::{name}.{f}" for f in re.findall(r"\bfn (\w+)", body))
    for m in re.finditer(r"^impl(?:<[^>]*>)?\s+(?:(\w+)(?:<[^>]*>)?\s+for\s+)?(\w+)\s*\{", text, re.M):
        trait, name = m.groups()
        if name not in public_types:
            continue
        body = text[m.end() - 1:block_end(text, m.end() - 1)]
        if trait is None:
            items.extend(f"{module}::{name}.{f}" for f in re.findall(r"\bpub fn (\w+)", body))
        elif trait in public_types or (RUST / f"{module}.rs").read_text(encoding="utf-8").count(f"pub trait {trait}"):
            items.extend(f"{module}::{name}.{f}" for f in re.findall(r"\bfn (\w+)", body))
    depth = 0
    for line in text.splitlines():
        if depth == 0:
            m = re.match(r"pub fn (\w+)", line.strip())
            if m:
                items.append(f"{module}::{m.group(1)}")
        depth += line.count("{") - line.count("}")
    return sorted(set(items))


def cyrius_symbols():
    """Every symbol src/*.cyr defines: `fn f`, `struct S`, `enum E`, `trait T`, `method S.m` (impl blocks)."""
    syms = set()
    for path in sorted(SRC.glob("*.cyr")):
        text = re.sub(r"#[^\n]*", "", path.read_text(encoding="utf-8"))
        for m in re.finditer(r"^(fn|struct|enum|trait) (\w+)", text, re.M):
            syms.add(f"{m.group(1)} {m.group(2)}")
        for m in re.finditer(r"^impl (?:(\w+) for )?(\w+)\s*\{", text, re.M):
            body = text[m.end() - 1:block_end(text, m.end() - 1)]
            for f in re.findall(r"^\s+fn (\w+)[<(]", body, re.M):
                syms.add(f"method {m.group(2)}.{f}")
        for m in re.finditer(r"^trait (\w+)\s*\{", text, re.M):
            body = text[m.end() - 1:block_end(text, m.end() - 1)]
            for f in re.findall(r"\bfn (\w+)[<(]", body):
                syms.add(f"method {m.group(1)}.{f}")
    return syms


def main(argv):
    check = "--check" in argv
    syms = cyrius_symbols()
    failures = []
    seen = set()
    pending = 0
    for module in PORTED + list(PARTIAL):
        print(f"== {module}.rs" + (f" (in progress, {PARTIAL[module]})" if module in PARTIAL else ""))
        for item in rust_items(module):
            seen.add(item)
            port = SURFACE.get(item)
            if port is None and module in PARTIAL:
                pending += 1
                print(f"  {item:48s} pending ({PARTIAL[module]})")
            elif port is None:
                failures.append(f"{item}: no SURFACE entry")
                print(f"  {item:48s} UNMAPPED")
            elif isinstance(port, str):
                print(f"  {item:48s} {port}")
            else:
                missing = [s for s in port if s not in syms]
                failures.extend(f"{item}: src/ defines no {s}" for s in missing)
                status = "MISSING " + ", ".join(missing) if missing else ", ".join(port)
                print(f"  {item:48s} {status}")
    for key in sorted(set(SURFACE) - seen):
        failures.append(f"{key}: SURFACE lists an item 1.3.0 doesn't have")
    for msg in failures:
        print("FAILED:", msg)
    done = len(seen) - pending
    print(f"surface parity: {done} items in {len(PORTED) + len(PARTIAL)} modules, {pending} pending, "
          f"{len(failures)} failures")
    return 1 if check and failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
