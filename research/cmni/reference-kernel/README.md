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
- `DetectorResponseEnsemble` propagates equally weighted response-matrix calibration realizations into predictive mean counts and total covariance using the law of total covariance.
- Synthetic fixtures: `research/cmni/fixtures/identifiability-design-synthetic.json` and `research/cmni/fixtures/detector-response-ensemble-uncertainty-synthetic.json`.
- Eight local-identifiability tests and six response-ensemble tests have been authored. The total is now **78 authored Rust tests** (37 root, 12 detector, 7 measurement, 8 material-state, 8 experiment-design, 6 response-uncertainty). They have not been executed; no Cargo test or CI pass is claimed.
- The Poisson Fisher helper assumes fixed response and conditionally independent Poisson bins. It does not, by itself, account for the cross-bin covariance from uncertain response matrices. The ensemble-moment helper is a forward covariance primitive, not a hierarchical likelihood or an unfolding algorithm.
- Neither module proves global identifiability, distinguishes all competing physical mechanisms, nor provides experimental evidence.

See [IDENTIFIABILITY_AND_DESIGN.md](../../docs/research/cmni/IDENTIFIABILITY_AND_DESIGN.md) for the proposed design architecture, SOTA comparison and benchmark ladder.
