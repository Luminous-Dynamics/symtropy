# CMNI reference kernel

This is a **research-only numerical prototype** for a homogeneous screened D–D thick-target yield integral.

It deliberately does not:
- implement first-principles nuclear physics or a validated nuclear-data library;
- supply a real astrophysical S-factor or material stopping-power dataset;
- represent spatially heterogeneous defects, implantation or hydrogen transport;
- reproduce any published experiment;
- estimate power gain, calorimetric excess heat, or net energy.

The caller must inject sourced, versioned S-factor and stopping-power functions, target density, screening model, and uncertainty metadata. Current tests use synthetic constants only to verify numerical behavior and input rejection.

Run with:

```sh
cargo test --manifest-path research/cmni/reference-kernel/Cargo.toml
```

The kernel is intentionally outside the production workspace pending independent physics review, source-data reconstruction, and numerical verification. See [the physical twin design](../../docs/research/cmni/PHYSICS_TWIN.md).


## Current research-kernel capabilities

- Homogeneous screened thick-target yield integral with explicit center-of-mass energy and stopping-power frame contract.
- Relative-yield normalization (Y(E)/Y(E_{ref})).
- Closed-form homogeneous ALR enhancement and Eq. 13 two-environment mixture.
- Separate literal-Eq.16 and prose/Fig.3c-direction sigmoid variants for the 2026 model reproducibility audit.
- Weighted log-space objective.
- First-order prompt D⁺/D₂⁺ dissociation component mapper with ion-current fractions converted to deuteron-flux fractions.

The D₂⁺ routine is a conditional sensitivity model, not an observed beam decomposition. It does not model dissociation depth or species-specific stopping. The code has not been executed with Cargo in the current work environment; the tests are pending a Rust-capable runner.


## Current research-kernel capabilities (updated)

- Homogeneous screened thick-target yield integral with explicit center-of-mass energy and stopping-power frame contract.
- Relative-yield normalization (Y(E)/Y(E_{ref})).
- Closed-form homogeneous ALR enhancement and Eq. 13 two-environment mixture.
- Separate literal-Eq.16 and prose/Fig.3c-direction sigmoid variants for the 2026 model reproducibility audit.
- Weighted log-space objective.
- D⁺/D₂⁺ prompt-dissociation component mapper, with ion-current fractions converted to deuteron-flux fractions.
- Generic weighted beam-spectrum yield function accepting an arbitrary discrete measured or hypothesized energy distribution; no Gaussian assumption is made.

The D₂⁺ routine is a conditional sensitivity model, not an observed beam decomposition. It does not model dissociation depth or species-specific stopping. The generic spectrum interface can represent a measured distribution only once the species-resolved energy histogram is supplied with provenance and uncertainty.

There are **21 Rust tests authored** in the current source. They have not been executed in the current environment because `cargo`, `rustc`, and `nix` are unavailable. The prototype remains outside the production workspace pending Rust-capable execution, independent physics review, source-data reconstruction, and numerical verification.


## EJ-309 detector observation scaffold

- Source metadata fixture: `research/cmni/fixtures/chen-2025-ej309-detector-published-metadata.json`.
- Pure PSD gate helper: `neutron_psd_gate_accepts`, using caller-supplied 5-keVee slice calibration values, `mu_gamma + 5*sigma_gamma`, upper offset, and minimum light-output threshold.
- The helper tests gate arithmetic and invalid calibration handling only. It does not model scintillation, PMT waveforms, detector efficiency, MCNP/Geant4 transport, or GRAVEL unfolding.
- **26 Rust tests are authored but not executed** in this environment because `cargo`, `rustc`, and `nix` are not installed.


## New statistical and spatial-model functions

- shared_reference_log_covariance: constructs the common-denominator covariance for log relative yields, assuming independent numerator counts and one shared reference yield.
- correlated_weighted_log_space_chi_square: generalized least-squares score using Cholesky factorization; rejects non-symmetric and non-positive-definite covariance.
- total_deuteron_rate_per_second: converts a singly charged D+/D2+ ion-current mixture into deuteron arrival rate.
- GaussianBeamFootprint: current-normalized axisymmetric Gaussian radial flux density, explicitly a baseline that must be replaced by a measured 2-D beam map when available.
- Literal Eq. 16 sigmoid enhancement is implemented separately from the prose-direction sigmoid and Eq. 13 mixture.
- Arbitrary 2-D beam-map cells can be reconciled against an independent Faraday-cup total; current mismatch fails closed.
- 1-D finite-volume diffusion with zero-flux boundaries, explicit stability guard and an exact local constant-source / first-order-loss update in src/material_state.rs.

There are now **45 authored Rust tests** in the prototype: 37 in the main reference kernel and 8 in the material-state module. They remain unexecuted in the current environment because cargo, rustc, and nix are unavailable. Independent calculations in this session checked the shared-reference covariance algebra, diagonal GLS limit, Gaussian profile normalization and both sigmoid asymptotes; these checks are not a Rust test pass.


## Measurement observability

- `DepthResponseKernel` and `apply_depth_observation_operator` forward-project a latent 1-D profile through arbitrary normalized response rows. These can represent a non-Gaussian, asymmetric or energy-dependent depth-response function.
- Seven authored tests cover identity recovery, broadening, constant-profile invariance and malformed-kernel rejection.
- The synthetic fixture `research/cmni/fixtures/depth-response-observability-synthetic.json` deliberately separates literature metadata from artificial response weights.
- Isotope scope matters: the 6.385-MeV `1H(15N,αγ)12C` resonance measures protium; deuterium needs a D-sensitive reaction/measurement and its own calibrated response.
- Lateral spatial point-spread and depth response are distinct and not yet combined in the current kernel.

There are now **64 authored Rust tests**: 37 in the root reference kernel, 12 in detector.rs, 7 in measurement.rs and 8 in material_state.rs. They remain unexecuted because cargo/rustc/nix are unavailable in this environment.


## Detector response matrix forward model

- DetectorResponseMatrix maps incident true-neutron-energy bin rates into expected measured pulse-height bin rates.
- Rows are measured bins; columns are true-energy bins. Each column sum is an explicitly represented detection probability/efficiency and must not exceed one.
- Background rates and exposure time are separate from the matrix.
- Seven authored tests cover identity response, energy migration, efficiency loss, expected counts, invalid probabilities, ragged matrices and background/exposure validation.
- Synthetic fixture: research/cmni/fixtures/detector-response-matrix-forward-synthetic.json.

The forward matrix is a numerical interface only. There is no Geant4/MCNP/DRiFT model, no scintillation or PSD response, and no unfolding algorithm in this prototype.


## Detector response matrix forward operator

- DetectorResponseMatrix maps true incident neutron-energy bin rates to expected measured pulse-height bin rates.
- Rows are measured pulse-height bins; columns are true-energy bins. Column sums represent accepted-event probability / efficiency and must not exceed one.
- Background count rates and exposure time are independent inputs.
- Seven tests cover identity, energy migration, efficiency loss, expected counts, invalid probabilities, ragged matrices and background/exposure validation.
- A synthetic 2 × 2 fixture is in research/cmni/fixtures/detector-response-matrix-forward-synthetic.json.

This is a generic forward operator only. It does not model scintillator light production, proton quenching, detector geometry/transport, PSD, threshold, pileup, dead time or any Geant4/MCNP response. No spectrum unfolding is implemented.


## Nonnegative Poisson unfolding comparator

- DetectorResponseMatrix now includes `poisson_em_unfold`, an unregularized Poisson expectation-maximization / Richardson-Lucy-style comparator for nonnegative source spectra with fixed measured-bin background expectations.
- The algorithm uses the forward response matrix and column efficiencies; it rejects unobservable true-energy bins and invalid starting spectra.
- Iteration count is an explicit early-stopping regularization choice. This is not GRAVEL and should not be used to claim superiority without held-out truth, matrix mismatch tests and uncertainty coverage.
- Five additional tests cover identity response recovery, a migrated two-bin spectrum, known background, unobservable bins and invalid iteration/initial values.
- Synthetic deterministic benchmark: `research/cmni/fixtures/poisson-em-detector-unfolding-synthetic.json`.


## Local identifiability and response-calibration uncertainty

- `poisson_fisher_information` builds local Fisher information for conditionally independent Poisson channels from caller-supplied expected counts and Jacobians.
- `combine_fisher_information` adds independent run contributions; `identifiability_summary` reports eigenvalues, effective rank, log determinant and condition number using a symmetric Jacobi eigensolver.
- `DetectorResponseEnsemble` propagates equally weighted response-matrix calibration realizations into predictive mean counts and total covariance using the law of total covariance;
- `DetectorResponseEnsemble::log_predictive_probability` evaluates observed integer counts under a finite equal-weight mixture of conditional Poisson likelihoods using log-sum-exp.
- Synthetic fixtures: `research/cmni/fixtures/identifiability-design-synthetic.json`, `research/cmni/fixtures/poisson-model-discrimination-synthetic.json` and `research/cmni/fixtures/detector-response-ensemble-uncertainty-synthetic.json`.
- Eight local-identifiability tests, six discrete Poisson-KL model-discrimination tests and eleven response-uncertainty tests have been authored. The total is now **89 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty). They have not been executed; no Cargo test or CI pass is claimed.
- The Poisson Fisher helper assumes fixed response and conditionally independent Poisson bins. It does not, by itself, account for the cross-bin covariance from uncertain response matrices. The response-ensemble log score uses equal fixed weights and fixed background; it is not a general nuisance-aware hierarchical likelihood or an unfolding algorithm.
- Neither module proves global identifiability, distinguishes all competing physical mechanisms, nor provides experimental evidence.

See [IDENTIFIABILITY_AND_DESIGN.md](../../../docs/research/cmni/IDENTIFIABILITY_AND_DESIGN.md) for the proposed design architecture, SOTA comparison and benchmark ladder.


## Multifidelity and robust experiment-design primitives

- `nested_control_variate_estimate` implements a two-level nested approximate-control-variate estimator for a scalar expectation, using paired high/low samples and extra low-fidelity samples.
- The control-variate coefficient `beta` must be frozen independently of production samples. The result includes an estimate, nested-sample variance estimate, high-fidelity baseline variance and variance-reduction factor.
- `robust_design_scores` evaluates supplied design utilities across a finite declared nuisance-scenario set; `select_maximin_design` chooses the design with best worst-case utility, breaking ties by lower cost.
- Synthetic fixture: `research/cmni/fixtures/multifidelity-design-synthetic.json`.
- Eleven additional tests cover the nested estimator, variance identity, malformed inputs, finite-scenario maximin selection and cost/error handling. The total is now **100 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design). They remain unexecuted in this environment.
- This is not a full multifidelity EIG method, ACV budget optimizer or robust Bayesian design engine. It does not learn beta, bound fidelity discrepancy, or establish continuous worst-case guarantees.


## Covariance-aware reference-spectrum comparison

- `covariance_profiled_spectrum_shape_chi_square` compares nonnegative reference and predicted spectra with a full positive-definite covariance matrix and analytically profiles one nonnegative multiplicative scale.
- The output includes the profiled scale, chi-square, degrees of freedom and reduced chi-square. The scale is a nuisance term for shape comparison; absolute fluence must be scored separately.
- Synthetic fixture: `research/cmni/fixtures/spectrum-shape-covariance-synthetic.json`.
- Eight tests cover exact shape recovery, normalization invariance, covariance sensitivity, malformed covariance, and invalid inputs. The total is now **108 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics). These are not executed in the current environment.
- Reference-spectrum versions are data, not immutable truth. The 2025 AIACHNE re-evaluation of the 252Cf PFNS documents missing historical 1987 input/covariance records and a changed evaluation below 300 keV; use versioned spectrum and covariance artifacts.


## Feature-conditioned measurement-bias forward layer

- `apply_feature_conditioned_multiplicative_bias` applies caller-supplied log-linear experiment-feature coefficients to physical expected counts using a channel-specific multiplicative factor `exp(X beta)`.
- The function keeps measurement-feature coefficients outside physical-yield parameters and never alters source observations.
- It does not estimate coefficients, apply a sparse prior, infer causality, or provide correction uncertainty; those require a separate model-fitting and holdout process.
- Synthetic fixture: `research/cmni/fixtures/feature-conditioned-measurement-bias-synthetic.json`.
- Seven additional tests cover zero-bias identity, multiplicative factors, channel-specific coefficients, malformed dimensions, nonpositive predictions and invalid/overflowing inputs. The total is now **115 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics, 7 measurement-bias). Not executed in this environment.


## Small-feature Bayesian measurement-bias inference

- `fit_sparse_bayesian_poisson_bias` fits a Poisson log-link multiplicative-bias model with independent Bernoulli feature inclusion and Gaussian coefficient slab priors.
- It exactly enumerates all feature subsets for up to 12 candidate columns, finds each subset posterior mode by damped Newton iterations, and uses a Laplace approximation for subset marginal evidence and coefficient covariance.
- The returned posterior includes feature inclusion probabilities, model probabilities, coefficient mean/covariance and the MAP feature subset.
- Synthetic fixture: `research/cmni/fixtures/sparse-bayesian-poisson-bias-synthetic.json`. It includes planted-bias, no-bias and collinear-feature cases.
- This is not the AIACHNE horseshoe method. It is a bounded comparator; inference is conditional on fixed physical means, a fixed design matrix and independent Poisson counts. It does not infer causality or propagate uncertainty in the physical model / feature dictionary.
- Eight additional tests cover planted signal recovery, null controls, collinear feature ambiguity, prior and input validation, bounded enumeration, and posterior normalization. The total is now **123 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics, 7 measurement-bias, 8 sparse-bias-inference). Tests are not executed in the current environment.


## Published-form grouped energy bias basis

- `build_grouped_energy_bias_design` implements three Gaussian widths in log10 energy (0.3, 0.15, 0.05) with 10, 50 and 100 log-spaced centres, respectively: 160 functions per feature-level group.
- Each observation can belong to multiple feature-level groups, preserving redundant candidate explanations rather than forcing a single category.
- The resulting matrix has 160 columns per group. `project_grouped_energy_bias_field` maps the flattened group coefficients to the observation-level log-bias vector consumed by the raw-scale likelihood. `project_grouped_energy_bias_field` projects group coefficients to the per-observation delta(E) consumed by the raw-scale likelihood. No horseshoe/BMC inference is included.
- Synthetic fixture: `research/cmni/fixtures/grouped-energy-bias-basis-synthetic.json`.

## Correlated Gaussian log-ratio sparse model averaging

- `fit_sparse_bayesian_correlated_log_bias` supports a full positive-definite covariance matrix in log-ratio space, always-included fixed effects (such as per-experiment normalization), and sparse feature columns.
- It exactly enumerates subsets (maximum 12 sparse columns) and analytically integrates proper Gaussian coefficient priors for each subset, then returns model probabilities, feature inclusion and mixture coefficient covariance.
- This is a correlated-data log-Gaussian comparator, not the raw-scale AIACHNE likelihood. If transforming a raw relative covariance by the delta method, record that approximation explicitly. It does not implement BMC or the horseshoe prior.
- Synthetic fixture: `research/cmni/fixtures/correlated-gaussian-log-bias-synthetic.json`.
- Ten energy-basis tests and eight correlated-Gaussian tests have been added. The raw-scale PFNS likelihood has six tests. The total is now **147 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics, 7 measurement-bias, 8 sparse-Poisson-bias, 10 energy-bias-basis, 8 correlated-Gaussian-bias, 6 raw-scale-likelihood). Tests have not been executed in this environment.


## Raw-scale correlated Gaussian PFNS bias likelihood

- `raw_scale_gaussian_multiplicative_bias_log_likelihood` evaluates one experiment with mean `mu * exp(delta)` and covariance `diag(mu) * Sigma_relative * diag(mu)`.
- The relative covariance remains anchored to the un-biased baseline mean; it is not silently scaled by the bias term. The baseline mean should already include the declared experiment normalization.
- This is the raw-scale observation likelihood primitive needed for a faithful PFNS bias analysis. It does not yet implement the AIACHNE horseshoe prior or Bayesian Monte Carlo sampler.
- Synthetic fixture: `research/cmni/fixtures/raw-scale-pfns-bias-likelihood-synthetic.json`.
- Six tests cover zero-bias likelihood, mean-only bias effect, full covariance, baseline covariance anchoring, positive-definiteness and invalid inputs. The total is now **147 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics, 7 measurement-bias, 8 sparse-Poisson-bias, 10 energy-bias-basis, 8 correlated-Gaussian-bias, 6 raw-scale-likelihood). Not executed in this environment.


## Raw-scale horseshoe posterior target and analytic gradients

- `raw_scale_horseshoe_log_posterior` combines the raw-scale correlated Gaussian likelihood with Normal coefficient priors conditional on local/global scales, half-Cauchy local scales, and either a half-normal or half-Cauchy global-scale prior.
- The function parameterizes local and global scales in log coordinates and returns analytic gradients for coefficients, every log-local scale, and log-global scale, including coordinate Jacobian terms.
- It does not implement HMC / BMC / NUTS sampling, warmup adaptation, multiple-chain diagnostics, effective sample size or posterior convergence assessment.
- Synthetic oracle: baseline mean [10,20], relative covariance [[0.04,0.01],[0.01,0.09]], one feature and planted coefficient 0.1. Independent finite-difference calculations confirm the gradient for both global priors to approximately 1e-10.
- Five tests cover both global prior choices and finite-difference agreement for beta, local-scale and global-scale gradients. The total is now **152 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics, 7 measurement-bias, 8 sparse-Poisson-bias, 10 energy-bias-basis, 8 correlated-Gaussian-bias, 6 raw-scale-likelihood, 5 horseshoe-posterior). These are authored, not executed.


## Prepared horseshoe evaluation and fixed-step HMC comparator

- `PreparedRawScaleHorseshoeModel` constructs and caches the full baseline relative-covariance Cholesky factor once, then repeatedly evaluates the raw-scale likelihood / horseshoe posterior and analytic gradients without refactorizing the covariance at each step.
- `run_horseshoe_hmc_chain` provides a seeded, fixed-step Hamiltonian Monte Carlo comparator with caller-specified discarded transitions, retained draws, step size and leapfrog count. It records acceptance and divergences.
- `split_rhat_for_coefficient` provides a basic split-Rhat screening statistic for independent chains. It is not rank-normalized or folded Rhat.
- The sampler has no warmup adaptation, dense/diagonal mass-matrix adaptation, NUTS tree building, effective sample size, posterior predictive diagnostics or tau0 stability screening. Run multiple independent seeds and dispersed initials; do not interpret samples until those diagnostics are added and the chains are validated.
- Three prepared-model tests and five HMC tests have been authored. The total is now **160 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics, 7 measurement-bias, 8 sparse-Poisson-bias, 10 energy-bias-basis, 8 correlated-Gaussian-bias, 6 raw-scale-likelihood, 5 horseshoe-posterior, 3 prepared-horseshoe, 5 horseshoe-HMC). Tests remain unexecuted.


## Prepared raw-scale horseshoe model and HMC comparator

- `PreparedRawScaleHorseshoeModel` computes and caches the full baseline relative-covariance Cholesky factor once, then evaluates successive parameter states without repeating the cubic factorization.
- `run_horseshoe_hmc_chain` provides a deterministic seeded fixed-step HMC comparator with caller-specified step size / leapfrog steps, discarded initial transitions, retained draws, acceptance rate and divergence count.
- `split_rhat_for_coefficient` provides basic unranked split-Rhat only; it is a screening diagnostic, not the rank-normalized / folded Rhat recommended for final Bayesian inference.
- No adaptation, mass matrix learning, NUTS, effective sample size, MCSE or posterior predictive diagnostics are implemented. The sampler is not a reproduction of the AIACHNE BMC code.
- Three prepared-target tests and five HMC tests have been authored. Total: **160 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 14 experiment-design, 11 response-uncertainty, 11 multifidelity-design, 8 spectrum-metrics, 7 measurement-bias, 8 sparse-Poisson-bias, 10 energy-bias-basis, 8 correlated-Gaussian-bias, 6 raw-scale-likelihood, 5 horseshoe-posterior, 3 prepared-horseshoe, 5 horseshoe-HMC). Not executed in this environment.


## Modern multi-chain HMC diagnostics (research implementation)

- diagnose_horseshoe_coefficient requires at least four chains and computes rank-normalized split-Rhat, folded rank-normalized split-Rhat, bulk ESS, lower/upper tail-indicator ESS, raw-scale mean ESS, raw-unit MCSE, per-chain E-BFMI and total divergence count.
- The proposed qualification flag requires max(Rhat) < 1.01, bulk and tail ESS at least 100 × chain count, zero divergences and E-BFMI >= 0.3 in every chain. These are conservative project gates based on published Stan diagnostic guidance; they are not evidence that this implementation reproduces Stan exactly.
- Mean MCSE is computed on the original coefficient scale, not the rank-normalized scale. Bulk ESS is separately computed on rank-normalized values.
- Six sampler-diagnostic tests cover four-chain requirement, summary generation, folded-Rhat scale mismatch, E-BFMI failure, MCSE scaling and zero-ESS behavior. Total: **166 authored Rust tests** across 17 source files. Tests have not been executed in this environment.
- The diagnostics need independent comparison against Stan/posterior reference chains. They do not replace trace plots, posterior predictive checks, prior sensitivity or adapted HMC/NUTS.


## Modern multi-chain HMC diagnostics (research implementation)

- diagnose_horseshoe_coefficient requires at least four chains and computes rank-normalized split-Rhat, folded rank-normalized split-Rhat, bulk ESS, lower/upper tail-indicator ESS, raw-scale mean ESS, raw-unit MCSE, per-chain E-BFMI and total divergence count.
- The proposed qualification flag requires max(Rhat) < 1.01, bulk and tail ESS at least 100 × chain count, zero divergences and E-BFMI >= 0.3 in every chain. These are conservative project gates based on published Stan diagnostic guidance; they are not evidence that this implementation reproduces Stan exactly.
- Mean MCSE is computed on the original coefficient scale, not the rank-normalized scale. Bulk ESS is separately computed on rank-normalized values.
- Seven sampler-diagnostic tests cover four-chain requirement, summary generation, folded-Rhat scale mismatch, E-BFMI failure, raw-unit MCSE scaling, zero-ESS behavior and an IID positive control. The verified source inventory is now **167 authored Rust tests** across 17 files. Tests remain unexecuted in this environment.
- The diagnostics need independent comparison against Stan/posterior reference chains. They do not replace trace plots, posterior predictive checks, prior sensitivity or adapted HMC/NUTS.
