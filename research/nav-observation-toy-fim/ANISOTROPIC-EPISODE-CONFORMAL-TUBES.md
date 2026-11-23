# Anisotropic Episode-Level Conformal Tubes

## Purpose

This module adds a shape-adaptive extension to the constant-radius isotropic family in `dynamic_episode_conformal_family.rs`. Each predicted position has a frozen symmetric positive-definite shape matrix (A_t). A single scalar conformal quantile scales all per-position ellipses for the episode.

The matrix is a **geometric shape descriptor**, not automatically a covariance or calibrated information matrix. It may encode anchor geometry or a separately trained/calibrated directional scale model, but it must be generated without access to the current episode's truth.

## Score and set

For calibration episode i and position t:

`r_(i,t) = sqrt((xhat_(i,t)-x_(i,t))^T A_(i,t)^(-1) (xhat_(i,t)-x_(i,t)))`

The single episode score is:

`S_i = max_t r_(i,t)`.

For requested miscoverage eta, select:

`k = ceil((n+1)*(1-eta))`

and use the k-th ordered episode score q_eta. The ellipse at position t is:

`C_t(eta) = {x: (x-xhat_t)^T A_t^(-1) (x-xhat_t) <= q_eta^2}`.

The event is a whole-episode hit only when every truth position lies inside its corresponding ellipse. A missing truth sample, mismatched prediction/truth/shape length, or non-finite truth point is an evaluator error, never a hit.

The per-position ellipse area is:

`area_t = pi * q_eta^2 * sqrt(det(A_t))`.

The module reports total area across the episode so area/coverage Pareto frontiers can be benchmarked against the isotropic tube.

## Frozen-shape and calibration contract

- The predictor, preprocessing, shape generator, matrix units and score definition are frozen before calibration.
- Calibration and evaluation episode IDs and independence-group IDs are disjoint.
- The shape-definition digest binds how (A_t) is generated and interpreted.
- Evaluation must present the same predictor and shape-definition commitments.
- Calibration uses one maximum normalized error per independent trajectory episode, not one score per packet or point.
- The supplied source manifest digest is checked as nonzero and retained; this module does not authenticate its underlying bytes.
- The module does not prove episode independence/exchangeability or ground-truth quality.

A shape matrix derived from HBR Hessian curvature is an exploratory geometric descriptor only. Do not label it covariance, uncertainty or protection-level evidence unless an independent calibration/validation process establishes that interpretation.

## Numerical contract

Every shape matrix must be finite, symmetric within a frozen relative tolerance, positive definite and above a frozen minimum relative eigenvalue ratio. Ill-conditioned or singular matrices are rejected. The normalized residual is computed with a scaled Cholesky solve, avoiding explicit matrix inversion.

The exact symmetry tolerance and eigenvalue floor are retained with each output tube. Evaluation does not get to weaken the shape policy after calibration.

## Level semantics

- Non-finite or eta<=0: unbounded/unsupported abstention.
- 0<eta<1: finite-sample split-conformal rank and ellipse family.
- eta>=1: explicit empty bounded set, with whole-episode miss indicator one. This is distinct from abstention and is needed so an adaptive controller's feedback is not silently changed.

If the finite-sample rank is greater than n, the family abstains instead of emitting a fictitious finite ellipse. The sequential running/rolling policies still own the anytime miscoverage guarantee; this constructor does not create that guarantee by itself.

## Benchmark

Compare on identical frozen predictions, calibration/test episodes, and target environments:
1. Isotropic constant-radius tube (#6979).
2. Fixed anisotropic shape from anchor geometry.
3. Shape derived from a frozen directional residual model.
4. Shape derived from local estimator curvature as an exploratory ablation only.
5. Each shape family with and without deterministic common-cause fault envelopes.

Report:
- whole-episode coverage and independent evaluation confidence bounds;
- total ellipse area, mean/max area and axis lengths;
- anisotropy ratio and orientation;
- area versus requested eta;
- qualified availability and abstention fraction;
- false-safe/fault-specific results under clock, anchor, calibration, firmware and NLOS common-cause faults;
- held-out environment/device/anchor geometry shift;
- CPU p50/p95/p99, memory and energy.

The success criterion is lower area at matched whole-episode coverage and matched availability, without increased false-safe common-cause fault exposure. Do not use one weighted aggregate score.

## Current status

Implemented:
- episode-level maximum normalized Mahalanobis calibration score;
- SPD/symmetry/conditioning validation;
- scaled Cholesky solve;
- finite-sample quantile selection and insufficient-rank abstention;
- per-position ellipse construction and complete-episode scoring;
- ellipse area calculation;
- predictor/shape-definition/source/decision commitments;
- tests for anisotropy, area, rank monotonicity, invalid shapes, group reuse and full-episode truth completeness.

Not demonstrated:
- integration with real anchor-geometry shape descriptors or a learned shape model;
- admitted Bregar/IDLab/outdoor calibration/evaluation data;
- held-out coverage/area trade-offs;
- proof that any Hessian-derived shape represents covariance;
- physical UWB, PHMI or safety qualification.

Keep this research-only until source-complete calibration episodes and untouched cross-environment evaluation are available.
