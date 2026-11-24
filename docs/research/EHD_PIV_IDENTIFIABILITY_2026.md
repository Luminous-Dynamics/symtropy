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

The physical condition number is computed from the largest and smallest **profiled** physical singular values, not from the unprofiled maximum. Otherwise a nuisance projection that uniformly reduces physical sensitivity could be mistaken for severe conditioning. A manufactured regression test covers this case.

The crucial difference is separating joint rank from physical rank after nuisance profiling. Two nuisance parameters may be mutually confounded while a flow parameter remains well identified. Conversely, a joint fit can be ill-conditioned because a physical parameter and camera gain explain almost the same image change.

## Fail-closed acceptance semantics

- No positive physical claim if nuisance-profiled physical rank is deficient, the weakest physical singular value is below the declared absolute floor, or its condition exceeds the configured limit.
- No informative sensitivity is distinct from rank deficiency.
- No priors/regularizers are added to create a false positive result.
- Correlated noise must be whitened before this kernel is called; a diagonal/noise-independent assumption must not be silently imposed.
- Numerical non-convergence, invalid scales, ragged matrices and non-finite input fail closed.

## Manufactured tests

The 27 current deterministic tests cover orthogonal and exact/near-collinear physical/nuisance effects, nuisance-only rank deficiency, characteristic scaling, row permutation, positive/negative confounding, zero-sensitivity parameters, underdetermined observations, malformed/non-finite inputs, one-column decomposition, Jacobi non-convergence, the distinction between absent physical sensitivity and nuisance confounding, correlated-noise whitening, positive-definite covariance validation, no-jitter rejection, and conditional innovation/Schur-complement controls for candidate measurements.

## V0.3: conditional innovation whitening for candidate measurements

The crate now also accepts raw sensitivity blocks and the full covariance partition for existing and candidate observations. For existing residuals (y_0) and candidate residuals (y_1), define:

[
Sigma =
egin{bmatrix}
Sigma_{00} & Sigma_{01}\
Sigma_{10} & Sigma_{11}
end{bmatrix},
qquad
Sigma_{01}=Sigma_{10}^{	op}.
]

The conditional sensitivity and covariance are:

[
J_{1|0}=J_1-Sigma_{10}Sigma_{00}^{-1}J_0,
qquad
S_{1|0}=Sigma_{11}-Sigma_{10}Sigma_{00}^{-1}Sigma_{01}.
]

The API `whiten_conditional_innovation_sensitivities` returns (L_{1|0}^{-1}J_{1|0}), where (S_{1|0}=L_{1|0}L_{1|0}^{	op}). This is the incremental information in the candidate action after conditioning on the already observed images. It can be passed into `rank_candidate_measurement_actions`.

Manufactured tests cover a highly correlated repeat, an independent observation, malformed cross-covariance and a non-positive-definite Schur complement. This bounded dense implementation is limited to 512 total rows. It does not add jitter or “repair” a non-SPD Schur complement. A production-scale image pipeline needs qualified block/structured/low-rank covariance operators and frame-common-mode terms.

## V0.2: explicit dense covariance whitening reference

The crate now includes `whiten_sensitivity_with_covariance(J, Sigma, tol)`, which checks covariance dimensions, finiteness, symmetry and strict positive definiteness, performs Cholesky (Sigma=LL^	op), and returns (J_w=L^{-1}J). Therefore (J_w^	op J_w=J^	opSigma^{-1}J). It does not symmetrize the input, add diagonal jitter, or repair a singular covariance. The dense routine is capped at 512 observations to bound (O(n^3)) factorization cost.

This is a small reference path for proving the effect of correlated observation noise, not the production image-covariance implementation. The next scalable adapter should use independently qualified block-sparse, Toeplitz/FFT or low-rank-plus-diagonal covariance structure, including cross-frame/common-mode components. Do not whiten each image pixel independently unless the measurement model actually supports that covariance assumption.

## V0.1: candidate measurement ranking

The crate now accepts a set of candidate actions. Each action supplies new residual-sensitivity rows that are already whitened as conditional innovations, including their cross-covariance with existing measurements. For each action, the kernel appends those rows, recomputes nuisance-profiled physical information and ranks actions lexicographically by physical-rank gain, smallest physical singular-value gain per unit cost, remaining weakest-direction sensitivity and stable action ID. Actions that only add nuisance sensitivity receive no physical-information credit. Candidate search is bounded to 256 actions and 65,536 total residual rows.

This is a local E-optimal-style heuristic after nuisance profiling, not full Bayesian expected value of sample information (EVSI), D-optimal design, posterior-variance optimization or hardware actuation.

## V1: from an audit to richer experiment selection

For candidate acquisition action (a), estimate the new whitened sensitivity rows (J_a). The updated local information is:

`F_new = F_old + J_a^T J_a`

Future candidate actions can be ranked more broadly by:
- increase in the smallest nuisance-profiled physical singular value;
- reduction in profiled physical condition number;
- increase in effective physical rank;
- D-optimal information gain where the model supports it;
- expected uncertainty reduction for the actual engineering functional;
- acquisition, illumination, compute and observer-intervention cost.

This is more principled than selecting images solely because a predictive model is uncertain. The model should request the measurement that resolves the physical/nuisance ambiguity.

## Claim boundary

V0 assesses local linear sensitivity only. It does not prove global/structural identifiability, global uniqueness, posterior unimodality, correct physical model, data authenticity or calibrated covariance. The Jacobian and whitening provenance remain caller-supplied and require independent qualification in a production pipeline.
