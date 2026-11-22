# EHD robust experiment design — nuisance-adjusted maximin reference v0.1

**Benchmark:** EHD-OED-002  
**Status:** manufactured reference; authored Rust tests; Rust execution and exact-head CI pending  
**Parent:** EHD-OED-001 / #6660; stacked on the D/E tournament branch

## Why this increment exists

The existing D/E tournament shows that a single nominal information criterion can hide a weak parameter direction. This increment adds two further failure modes to the same matched-budget selection problem:

1. A measurement channel can contain a nuisance response that is correlated with the physical response. Treating that nuisance as physical information can make a design appear identifiable when it is not.
2. A design that is excellent at one nominal noise level can be fragile when calibration/noise conditions change.

The implementation profiles one declared scalar nuisance response and selects an allocation by its weakest physical information direction in the worst declared scenario. It is deliberately small enough to inspect and independently recompute.

This is not a new general uncertainty engine and does not duplicate the existing evidence ledger.

## Mathematics

For a local linear observation model

\[
y = X\theta + q\eta + \epsilon,\qquad
\epsilon\sim\mathcal{N}(0,\Sigma),
\]

where \(\theta\) contains two physical parameters and \(\eta\) is one nuisance parameter, whiten with \(W=\Sigma^{-1}\). The nuisance-adjusted physical information is the Schur complement

\[
F_{\mathrm{eff}}
= X^\mathsf{T}WX
- X^\mathsf{T}Wq\,(q^\mathsf{T}Wq)^{-1}q^\mathsf{T}WX.
\]

This is the information remaining after the nuisance direction is profiled out. It is not valid to count q as another physical sensitivity, silently set a missing nuisance channel to zero, or use the unadjusted determinant as a substitute.

For scenarios \(s\in\mathcal S\), the selection objective is

\[
d^\star
=\arg\max_{d\in\mathcal D_B}
\min_{s\in\mathcal S}
\lambda_{\min}\!\left(F_{\mathrm{eff}}(d,s)\right),
\]

subject to full physical rank in **every** declared scenario and the same observation budget B. Rank-deficient allocations are excluded, not assigned an optimistic pseudo-inverse score. Ties are lexicographically resolved.

The present reference supports diagonal observation covariance, one scalar nuisance channel, two physical parameters, equal per-observation cost and bounded exhaustive discrete allocations. Multiple nuisance channels, correlated covariance, nonlinear profile likelihood, model discrimination and continuous/sequential design are intentionally unsupported.

## Manufactured tournament

Four abstract physical sensitivity rows are reused from the D/E reference:

| Candidate | Physical sensitivity |
|---:|---:|
| 0 | [-3, -3] |
| 1 | [-3, -2] |
| 2 | [-3, -1] |
| 3 | [-2, 0] |

The nuisance sensitivity is [1, 1, 1, 1] in both scenarios. At the same three-observation budget, the two frozen noise scenarios are:

| Scenario | Observation variances |
|---|---|
| nominal_unit_noise | [1, 1, 1, 1] |
| candidate_2_eightfold_variance | [1, 1, 8, 1] |

There are 20 allocations with replacement. Independent numerical recomputation of the stated Schur-complement objective gives:

| Result | Selection | Weakest effective information |
|---|---|---:|
| Nominal D-optimal comparator before nuisance projection | [0, 2, 3] | Not comparable to the profiled objective |
| Nominal E-optimal comparator before nuisance projection | [0, 3, 3] | Rank-deficient after nuisance projection |
| Nuisance-adjusted maximin | **[0, 1, 3]** | **0.0632501080311** |

Only three allocations remain full rank after nuisance projection in both scenarios. The maximin choice is not the nominal D-optimal choice. This is a deterministic manufactured counterexample, not evidence that candidate indices correspond to real EHD settings.

## Adversarial controls

The reference rejects or detects:
- a nominal E-optimal design that loses a physical direction once the offset-like nuisance response is profiled;
- a set with no allocation full rank across all scenarios;
- duplicate scenario identities;
- absence of information in the declared nuisance channel;
- invalid variances, non-finite sensitivities, mismatched scenario candidate counts and unbounded search sizes.

## Paradigm-shift path

The next sequence should preserve distinct physical, nuisance, discrepancy and metrology layers:

1. **Multichannel nuisance profiling:** extend from one scalar nuisance to a block of instrument offset, drift, cross-axis leakage and cable/feedthrough responses. Use a stable QR/SVD Schur complement; singular nuisance blocks must abstain or use a preregistered generalized-inverse policy.
2. **Correlated calibration covariance:** support full covariance only with a versioned calibration artifact, covariance factorization receipt and an independent whitening oracle. Shared calibration creates cross-channel covariance; it must not be approximated as independent noise.
3. **Practical identifiability:** add profile-likelihood intervals and parameter-boundary tests. Local Fisher rank is a local diagnostic, not proof of global or practical identifiability.
4. **Model discrimination:** compare physical closure A/B and instrument-artifact hypotheses using held-out predictive distributions / expected log Bayes factor or a preregistered divergence. Return NotDiscriminable when models are observationally equivalent.
5. **Sequential active design:** choose the next admissible diagnostic based on unresolved physical directions and unresolved model pairs, while reserving a sealed confirmatory cohort.
6. **Decision-aware design:** optimize measured capability under real acquisition time, sensor availability, calibration burden and safety/admissibility limits. Never allow the optimizer to manufacture an unmeasured channel or propose an inadmissible operating condition.

## Industry and research comparators

- **NISTIR 8298** supplies an industrial CFD V&V/UQ procedure frame; use it for solver verification, validation data, solution uncertainty and physical model adequacy rather than claiming the design selector replaces CFD V&V.
- **ASME V&V 20** provides the accuracy-at-validation-point comparison frame with experimental and numerical solution uncertainty. Add multivariate validation metrics where the observable is a vector/time history, not just a scalar thrust value.
- **ASME VVUQ 20.1-2024** is a relevant multivariate validation comparator; freeze data alignment, covariance, weights and vector metrics before comparison.
- **preCICE** is a coupling framework comparator for partitioned multiphysics and conservative mapping; the EHD programme should compare matched conservation residuals, convergence and coupling stability, not just runtime.
- **COMSOL / kinetic plasma solvers** are specialist solver references. Compare with pinned equation sets, boundary conditions, chemistry, mesh/time convergence and independent measurements. A source transcription or manufactured oracle is not a solver reproduction.

Reference pointers:
- NISTIR 8298: https://doi.org/10.6028/NIST.IR.8298
- ASME V&V 20-2009 (R2021): https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer
- ASME VVUQ 20.1-2024: https://www.asme.org/codes-standards/find-codes-standards/vvuq201-vvuq-20-1-multivariate-metric-for-validation
- preCICE documentation: https://precice.org/configuration-transport
- Practical identifiability and model-discrimination design (Liu, Maini & Baker, 2026): https://doi.org/10.1016/j.mbs.2026.109710

## Benchmark programme and target gates

All numerical targets below are prospective. They are not achieved results.

| Gate | Benchmark | Target / acceptance |
|---|---|---|
| B0 | Manufactured nuisance Schur complement | Match independent oracle to declared floating-point tolerance |
| B1 | Physical/nuisance confounding | No false physical identification when nuisance projection removes rank |
| B2 | Robustness | Rank 2 in every frozen scenario; report per-scenario spectrum and worst-case score |
| B3 | Calibration covariance | Whitened covariance agrees with an independent factorization oracle; reject invalid/non-PSD input |
| B4 | Practical identifiability | 90–98% empirical coverage for nominal 95% intervals on preregistered held-out cases |
| B5 | Artifact discrimination | ≤1% false positives on frozen sham/dummy/reversal negative controls, with confidence intervals |
| B6 | Model discrimination | Abstain on observationally indistinguishable closures; report pairwise separability |
| B7 | Sequential design | Improvement in held-out parameter recovery/model discrimination at matched acquisition cost |
| B8 | Validation | Compare predicted and measured force, velocity, current/real power, momentum flux and heat/exergy terms under a frozen boundary |

Do not compare these targets against solver runtimes or published thrust maxima. The fair comparator is a matched dataset, matched physics and metrology boundary, matched compute/acquisition budget, and independent replay.

## Integration boundaries

- #6660 owns robust design, identifiability and model discrimination.
- #6680/#6710 provide the two-parameter reference kernel and nominal D/E tournament.
- #6640 owns species-resolved collisional energy accounting.
- #6641/#6650 own thrust observability and control-volume momentum.
- #6712/#6724 own second-law accounting and the restricted neutral ideal-gas exergy kernel.
- #6721 owns the Wang 10-Torr source cohort and its claim-boundary audit.

This increment adds a small robust-design comparator. It does not supersede the exergy, momentum, discrepancy or source-atlas workstreams.
