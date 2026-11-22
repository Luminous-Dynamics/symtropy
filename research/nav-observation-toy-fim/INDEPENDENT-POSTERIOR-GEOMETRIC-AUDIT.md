# Independent 3D range-set posterior audit

**Status:** deterministic geometric consistency kernel and synthetic tests added; no physical ranges or anchor survey admitted.

**Parent:** MILUV source/clock/DS-TWR work #6814, #6815, #6843.

## 1. Why a separate posterior auditor

A robust or high-performing front end can still output a falsely confident pose when its measurement model is wrong, its inputs share a common cause, or the geometry is poorly conditioned.

The independent auditor consumes only:
- the candidate 3D position and an externally justified absolute position-error radius;
- qualified lower/upper range bounds;
- anchor coordinates and absolute anchor-position error bounds;
- an externally assigned dependency/failure-domain ID for each range observation.

It does not consume the estimator covariance, front-end health flag, or evaluator truth. It therefore provides an independent consistency check rather than accepting the estimator's own confidence claim.

## 2. Conservative range-set intersection

For candidate position (hat p), candidate position-error radius (e_p), anchor position (hat a_i), anchor survey-error radius (e_{a_i}), and measured range interval ([r_i^L,r_i^U]), compute:

[
d_i=|hat p-hat a_i|_2
]

The predicted distance set is conservatively bounded by:

[
D_i=[max(0,d_i-e_p-e_{a_i}), d_i+e_p+e_{a_i}]
]

The interval conflict is the distance between the two closed intervals:

[
c_i=max(0, r_i^L-D_i^U, D_i^L-r_i^U)
]

A constraint is compatible only when (c_i) is no larger than the predeclared conflict tolerance. The output retains per-anchor predicted/observed sets and each conflict value.

This is a necessary geometric consistency condition, not proof that the candidate is true. Broad range or pose sets can intersect while still admitting many different locations.

## 3. Dependency-aware geometry

For each anchor, form the unit line-of-sight direction (u_i=(hat a_i-hat p)/d_i).

A naive information sum (sum_i u_i u_i^T) would let repeated or correlated observations inflate geometric support. Instead, group observations by the supplied common-cause/dependency ID. Within each group, average its direction outer products; then average the group matrices equally:

[
G=rac{1}{K}sum_{g=1}^{K}
left(rac{1}{n_g}sum_{iin g}u_i u_i^Tight)
]

where (K) is the number of declared dependency groups.

The trace of (G) is one. Its eigenvalues therefore lie between zero and one (with an ideal isotropic 3D value of one third on each axis). A very small minimum eigenvalue indicates a poorly supported direction, even when every range interval happens to intersect the candidate.

The module computes the symmetric 3x3 eigenvalues using bounded Jacobi rotations. This is a geometric directional-support diagnostic. It is **not** a Fisher information matrix, a covariance, a protection level, or a statistical independence proof.

Dependency labels are external assumptions. The audit cannot discover a common obstruction or prove that two groups are independent.

## 4. Fail-closed admission

The caller must predeclare:
- minimum number of range observations;
- minimum number of dependency groups;
- minimum acceptable geometry eigenvalue;
- maximum permissible range-set conflict.

The audit fails to pass if any range set conflicts or geometry is insufficient. Too few observations or dependency groups return distinct errors rather than being confused with malformed configuration.

The current implementation is 3D and requires at least four observations and four declared dependency groups. Real systems may need higher thresholds, dimensional priors or explicit mixed-anchor/robot dependency models, but these must be justified by the operating geometry.

## 5. Tests

Eight deterministic synthetic tests cover:
1. A consistent 3D candidate with non-collinear anchor geometry.
2. An interval contradiction.
3. Expansion of predicted ranges by anchor and candidate-position error bounds.
4. Collinear geometry despite individually consistent ranges.
5. Too few declared dependency groups.
6. Duplicate observations within one dependency group not increasing geometric support.
7. Duplicate anchor identity and candidate/anchor coincidence.
8. Invalid intervals, candidate bounds and configuration.

These tests demonstrate the mathematical contract only; they are not physical measurements or safety qualification.

## 6. Integration path

1. Connect only source-clean, admitted range observations and anchor survey receipts.
2. Preserve each range observation's identity, clock/STS/session evidence, calibration revision and dependency group.
3. Audit every candidate emitted by the UWB/VIO or multi-robot front end.
4. Feed geometric conflicts and unsupported directions into the protection-level and mission-authority gate.
5. Evaluate on sealed experiment-level datasets with leave-one-anchor, obstacle and failure-domain splits.
6. Compare posterior-audit outcomes against the 2026 decoupled NLOS backend optimizer and robust factor-graph baselines, using identical inputs.
7. Report accuracy, range-set compatibility, geometry sufficiency, interval coverage, abstention, false-safe exposure and compute cost separately.

## 7. Claim boundary

This module checks bounded geometric consistency and directional support. It does not estimate or certify a posterior distribution, prove independence, detect all common causes, guarantee true-state containment, or authorize safety-critical motion by itself. Its inputs must be qualified independently and its thresholds matched to a declared mission contract.
