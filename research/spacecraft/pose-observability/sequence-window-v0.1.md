# Sequence-Window SE(3) Observability v0.1

Issue: [#7932 — transported sequence-window observability](https://github.com/Luminous-Dynamics/symtropy/issues/7932)

Single-frame baseline: [#7931 — local pinhole FIM scorer](https://github.com/Luminous-Dynamics/symtropy/pull/7931)

## Why this exists

A local 6×6 FIM only describes one linearization and one observation set. It cannot be summed across camera frames by simply adding the numeric matrices: each frame's left-camera tangent coordinates are different.

Sequence observability must distinguish:
- new geometric diversity (parallax / a changed view that constrains a previously weak direction);
- repeated independent capture at the same view (may reduce stochastic variance but does not change the observable subspace);
- replay/reprocessing of the same pixels (must contribute zero additional information);
- correlated observations and shared calibration/model bias (must not be assumed independent).

## Coordinate contract

The single-frame tool uses the local left perturbation:
`T_k' = exp(ξ_k^) T_k`

and scaled dimensionless parameter:
`η_k = D^{-1} ξ_k`, where `D=diag(L,L,L,1,1,1)` and `η=[v/L,ω]`.

For each frame k, let `A_k = T_camera_k_camera_ref` be the rigid transform that maps reference-camera coordinates into frame-k camera coordinates, so `T_k=A_k T_ref`. A left perturbation transported from the reference camera to frame k is:
`ξ_k = Ad(A_k) ξ_ref`
and the scaled transport is:
`η_k = D^{-1} Ad(A_k) D η_ref`.

Thus:
`J_k_ref = J_k_local D^{-1} Ad(A_k) D`.

The convention, transform direction, frame identities, reference epoch, length scale and exact transform digest are compulsory. Finite-difference controls must independently verify the adjoint map and Jacobian chain.

## Noise / dependence model

For independent observations with block-diagonal pixel covariance:
`I_ref = Σ_k J_k_refᵀ Σ_k⁻¹ J_k_ref`.

For correlated observations, stack all residuals and use the declared full block covariance:
`I_ref = J_stackᵀ Σ_stack⁻¹ J_stack`.

Do not set off-diagonal covariance to zero merely because frames have different IDs. The dependence enum must distinguish:

| Dependence | Treatment |
|---|---|
| `INDEPENDENT_CAPTURE` | Separate exposures with evidence supporting the declared noise model; stochastic precision can accumulate. |
| `SAME_FRAME_REPROCESSING` | Deduplicate by source-byte and evidence identity; second processing contributes no new evidence. |
| `TEMPORALLY_CORRELATED` | Model block covariance or use a declared conservative grouping/upper bound. |
| `COMMON_CAUSE` | Preserve dependency and fail closed for independence claims; different frames do not make calibration/target bias independent. |
| `DEPENDENCE_UNKNOWN` | No default independence. OUT_OF_SCOPE / conservative lower-bound only. |

## Diagnostics

Report:
- full sequence 6×6 information matrix and six eigenvalues;
- reference-frame weakest-mode vector, min/max eigenvalues, rank, condition and logdet only if full rank;
- weakest eigenvalue and weak-subspace angle after every additional view;
- geometric-diversity change versus repeated-capture precision gain;
- number of unique raw frame digests, unique source-evidence IDs, unique camera transforms and declared dependence class;
- exact block covariance model / digest and the reference tangent-frame ID;
- per-frame residual and per-frame/local FIM for audit, but do not confuse their sum with the transported result.

Use principal angles between weak-mode subspaces, not raw eigenvector dot products when eigenvalues are repeated or nearly repeated.

## Frozen sequence controls

1. Two independent captures at identical geometry and equal sigma: information eigenvectors/subspace should remain unchanged; eigenvalues scale with independent sample count.
2. Same raw frame replayed under a different frame label: rejected or deduplicated; must not double information.
3. Two translated views with known (A_k): transported stacked-Jacobian FIM equals direct numerical finite-difference FIM at the reference pose.
4. Same two views summed without adjoint transport: test-only negative control produces a discrepancy.
5. Pure rotation and pure translation sequences: verify which weak modes improve and which do not.
6. Repeated symmetric target features: ambiguity persists despite more frames if geometry is not diverse.
7. Shared target-model / intrinsic / extrinsic bias across all frames: FIM can be high while absolute pose is wrong.
8. Temporally correlated overlapping frames: block covariance prevents double-counting.
9. Wrong transform direction, stale transform digest, time reversal, missing frame and reference-frame mismatch: reject.
10. Unknown dependence: no nominal independent sum.

## Relationship to next-best-view

Candidate-view FIM objectives are useful geometry design criteria:
- E-optimal: maximize the minimum eigenvalue / weakest direction.
- D-optimal: maximize logdet only when full rank and under matched parameter scaling.
- Condition-aware: penalize severe anisotropy and persistent weak modes.

These are not expected value of sample information. A view may improve FIM but fail to change the downstream decision, be unreachable, be unsafe, or cost more than it is worth. The EVSI contract #7927 must evaluate decision-risk reduction, resource cost and physical feasibility separately.

## Promotion plan

- P0: schemas and convention contract; frame/transform/covariance identities.
- P1: pure SE(3) adjoint transport and independent finite-difference challenger.
- P2: stacked covariance and cross-frame dependence controls.
- P3: CFI-Space integration, exact replay/dedup and counterfactual geometry.
- P4: public-real event/RGB sequences with audited target geometry and labels.
- P5: blind holdout, HIL and exact sensor/time/compute envelope.

No sequence FIM output may promote pose confidence, shrink a conformal region, clear a common-cause gate or authorize a sensing/maneuver action.

## Claim ceiling

This is a design contract only. It does not claim the transform/measurement dependence model is identified, that frames are independent, that FIM equals posterior covariance, that target geometry is true, or that any viewpoint is physically feasible. High sequence information can coexist with systematic common-cause bias.
