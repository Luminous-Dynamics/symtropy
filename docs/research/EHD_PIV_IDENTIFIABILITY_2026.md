# EHD-PIV-ID-001 — Physical identifiability after measurement-nuisance profiling

**Status:** V0 local linear identifiability reference kernel; not a global identifiability proof.

**Issue:** [#7270](https://github.com/Luminous-Dynamics/symtropy/issues/7270)

## Why this is necessary

The particle-image operator in [#7256](https://github.com/Luminous-Dynamics/symtropy/issues/7256) makes the measurement chain explicit. Yet an inverse method can only infer physical parameters if its observations distinguish their effects from nuisance effects.

For example:
- flow brightness / seeding density can be confounded with camera gain;
- particle image diameter can be confounded with optical PSF width;
- sheet thickness / particle z-position can be confounded with illumination attenuation;
- displacement can be confounded with timing or mapping calibration;
- interrogation filtering, noise and out-of-plane loss can imitate attenuation of small-scale structures.

A small residual and converged optimizer do not resolve these ambiguities. A posterior can look narrow because of a prior even when the data do not identify the parameter.

## Research basis

1. Wang & Hao (2025), [A Systematic Computational Framework for Practical Identifiability Analysis in Mathematical Models Arising from Biology](https://doi.org/10.1002/advs.202504346): FIM/eigenvector treatment of practical identifiability and experimental design.
2. Heitzman-Breen, Dukic & Bortz (2026), [A Practical Identifiability Criterion Leveraging Weak-Form Parameter Estimation](https://doi.org/10.1007/s11538-026-01639-x): practical identifiability criteria; FIM and profile-likelihood approaches have different assumptions and computational costs.
3. [Optimal experimental design for parameter estimation in the presence of observation noise](https://doi.org/10.1016/j.mbs.2025.109571), Mathematical Biosciences 392 (February 2026): observation-noise correlations can change optimal observation schedules.
4. Cameron (2022), [Theoretical description of PIV measurement errors](https://doi.org/10.1007/s11600-022-00901-9): PIV error depends jointly on particle image size, out-of-plane displacement, light-sheet distribution, seeding, noise, sensor fill factor and interrogation settings.

## V0 implementation

Crate: `tools/ehd-piv-identifiability/`.

The caller supplies a row-major sensitivity matrix after whitening residual rows using a qualified observation covariance. V0 bounds the reference calculation to 65,536 rows and 64 parameters. Columns are multiplied by declared characteristic parameter scales so each column represents one characteristic parameter perturbation in unit-noise residual coordinates.

The Rust kernel performs a one-sided Jacobi SVD and reports:
- joint singular values, numerical rank, condition and weak right-singular parameter combinations;
- nuisance numerical rank and its observation-space sensitivity span;
- physical singular values after projection out of nuisance span;
- nuisance-profiled physical rank and condition;
- physical/nuisance signed sensitivity correlations;
- per-parameter scaled sensitivity norm.

The crucial difference is separating joint rank from physical rank after nuisance profiling. Two nuisance parameters may be mutually confounded while a flow parameter remains well identified. Conversely, a joint fit can be ill-conditioned because a physical parameter and camera gain explain almost the same image change.

## Fail-closed acceptance semantics

- No positive physical claim if nuisance-profiled physical rank is deficient, the weakest physical singular value is below the declared absolute floor, or its condition exceeds the configured limit.
- No informative sensitivity is distinct from rank deficiency.
- No priors/regularizers are added to create a false positive result.
- Correlated noise must be whitened before this kernel is called; a diagonal/noise-independent assumption must not be silently imposed.
- Numerical non-convergence, invalid scales, ragged matrices and non-finite input fail closed.

## Manufactured tests

The 14 current deterministic tests cover orthogonal and exact/near-collinear physical/nuisance effects, nuisance-only rank deficiency, characteristic scaling, row permutation, positive/negative confounding, zero-sensitivity parameters, underdetermined observations, malformed/non-finite inputs, one-column decomposition, Jacobi non-convergence and the distinction between absent physical sensitivity and nuisance confounding.

## V1: from an audit to experiment selection

For candidate acquisition action (a), estimate the new whitened sensitivity rows (J_a). The updated local information is:

`F_new = F_old + J_a^T J_a`

Candidate actions can then be ranked by:
- increase in the smallest nuisance-profiled physical singular value;
- reduction in profiled physical condition number;
- increase in effective physical rank;
- D-optimal information gain where the model supports it;
- expected uncertainty reduction for the actual engineering functional;
- acquisition, illumination, compute and observer-intervention cost.

This is more principled than selecting images solely because a predictive model is uncertain. The model should request the measurement that resolves the physical/nuisance ambiguity.

## Claim boundary

V0 assesses local linear sensitivity only. It does not prove global/structural identifiability, global uniqueness, posterior unimodality, correct physical model, data authenticity or calibrated covariance. The Jacobian and whitening provenance remain caller-supplied and require independent qualification in a production pipeline.
