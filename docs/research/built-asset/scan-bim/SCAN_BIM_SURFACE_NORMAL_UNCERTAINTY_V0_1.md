# Surface-normal-aware point-to-plane residual uncertainty v0.1

**Issue:** #4729 — SCAN-BIM-SURFACE-NORMAL-UNCERTAINTY-001  
**Implementation:** `tools/scan-bim-replay-contract/src/residual_uncertainty.rs`  
**Depends on:** #4228 fixed-normal conditional coordinate residual covariance; #1607 survey covariance and frame provenance.  
**Status:** P0 Rust contract and synthetic known-answer/adversarial tests added; exact-head Rust/CI validation pending.

## 1. The additional uncertainty term

#4228 computes the directional coordinate-residual variance conditional on a fixed target normal. A fitted surface normal is itself an estimated quantity, so the fixed-normal result is incomplete when the measured point has nonzero tangential displacement from the local fit origin.

Let:
- (\delta = p-s\) be the nominal sample-minus-surface displacement in metres;
- (n\) be the nominal unit surface normal;
- (\Delta\delta\) be the coordinate perturbation;
- (\eta\) be a small tangent-plane perturbation of the unit normal, expressed in radians.

Because a unit-normal perturbation is tangent to its nominal direction, (n^T\eta=0). The first-order residual perturbation is:

`\Delta d \approx n^T\Delta\delta + \delta^T\eta`

The first-order variance is:

`\sigma_d^2 \approx n^T\Sigma_\delta n + \delta^T\Sigma_n\delta + 2n^T C_{\delta n}\delta`

where:
- (\Sigma_\delta) is the relative sample/target coordinate covariance derived from #4228's full covariance and cross-covariance inputs;
- (\Sigma_n) is the 3×3 tangent-plane normal covariance in radians²;
- (C_{\delta n}=\operatorname{Cov}(\Delta\delta,\eta)) is the metre·radian cross-covariance.

The implementation retains all three contributions separately. It does not diagonalize, RSS-combine, or silently drop a cross term.

## 2. Evidence contract

The new `SurfaceNormalUncertaintyInput` composes the existing #4228 `NormalResidualCovarianceInput` rather than replacing its coordinate/correlation/transform owner.

It requires:
- sample observation identity and digest-bound relative-point receipt/population;
- target surface, patch and fit revision;
- exact surface-fit output digest, support-population digest and coordinate frame;
- normal covariance, covariance receipt digest and an exact normal-covariance scope;
- estimator profile and configuration digests;
- estimator-specific maximum RMS angular standard uncertainty;
- estimator-specific maximum tangential offset from the fit origin;
- either a relative-point/normal cross-covariance receipt or an independence attestation, both bound to the exact observation, target, patch, fit revision, surface-fit bytes, support population, relative-point receipt and point population.

The normal covariance must be finite, symmetric, positive-semidefinite and tangent to (n). The cross-covariance must be finite and tangent in its normal-perturbation columns. When coordinate covariance is qualified, the full 6×6 joint covariance of ((\Delta\delta,\eta)) must be positive-semidefinite.

An omitted cross-covariance is **not** treated as independence. Unknown correlation returns an unqualified result with no combined numeric sigma. Coordinate uncertainty that is already unqualified cannot be used to construct a joint PSD claim with an implicit zero sample/target cross-covariance.

## 3. Estimator applicability envelope

A first-order delta-method covariance is local. The caller must provide two estimator-specific limits:
- maximum RMS angular standard uncertainty (\sqrt{\operatorname{tr}(\Sigma_n)});
- maximum supported tangential displacement (\|\delta - n(n^T\delta)\|).

Exceeding either limit returns `Unqualified`; no universal angular or distance threshold is invented. These limits are part of the estimator profile/configuration and must be justified by its validation.

This P0 contract does not estimate normals, infer the support region, estimate local curvature, or validate that the supplied fit is planar. It checks the declared evidence and its mathematical compatibility.

## 4. Output

`SurfaceNormalResidualUncertainty` preserves:
- #4228 coordinate-only variance and its unqualification reasons;
- normal-orientation variance contribution;
- signed relative-point/normal cross-covariance contribution;
- combined first-order variance and standard uncertainty only when every prerequisite qualifies;
- exact surface, patch, fit revision, observation, support-population, point-population, estimator profile/configuration and receipt identities;
- explicit estimator applicability limits and observed tangential offset.

The result disposition is either `QualifiedFirstOrderCovariance` or `Unqualified`. It is a first-order covariance result, not a calibrated 95% interval.

## 5. P0 known-answer and adversarial checks

The source currently contains 26 Rust unit tests across the fixed-normal and normal-aware residual primitives, including:
- zero normal covariance reproduces #4228 coordinate-only variance exactly;
- tangent covariance adds the hand-calculated (\delta^T\Sigma_n\delta) contribution;
- normal-direction displacement has zero first-order orientation contribution;
- positive and negative relative-point/normal cross-covariance produce the correct signed contribution;
- unknown relative-point/normal correlation emits no total sigma;
- non-tangent normal covariance and cross-covariance are rejected;
- non-PSD coordinate, normal and joint covariance blocks are rejected;
- exact normal-covariance receipt scope and exact correlation-attestation scope are enforced;
- stale fit revision / observation / support / point-population binding is rejected;
- angular and tangential-offset applicability limits trigger abstention;
- metre/mm relative-point conversions preserve the result;
- an unqualified coordinate covariance cannot be used to build a fictitious joint covariance block;
- an analytic first-order Jacobian is checked against a finite-difference normalized-normal perturbation.

These are synthetic numerical/contract tests only. The new implementation has not yet been Rust/CI qualified on its final PR head.

## 6. P1 independent physical validation

Use independently surveyed planar and curved target patches and held-out fit support:
1. Keep surface-fit points separate from residual validation points.
2. Vary patch extent, curvature, point density, range, incidence, occlusion and sensor/registration lineage.
3. Compare fixed-normal #4228 variance with the normal-aware first-order result.
4. Evaluate interval coverage and width by task/stratum only after the uncertainty model and coverage factor are pre-registered.
5. Include out-of-domain and abstention rates.
6. Group results by independent building/patch/capture lineage; do not count neighboring points from one fit as independent observations.

Baselines remain separate:
- fixed-normal coordinate covariance (#4228);
- source-faithful plane-fit / total-least-squares normal covariance;
- any point-level learned uncertainty predictor calibrated on building-disjoint checkpoints.

Report bias, covariance coverage, interval width, abstention, runtime and support size separately. Do not collapse them into a single score.

## 7. Claim ceiling

P0 proves first-order covariance contract behavior on synthetic inputs. It does not estimate surface normals, authenticate referenced artifacts, establish the surface-fit model's local planarity, model full spatial covariance over an entire cloud, propagate nonlinear registration Jacobians, establish absolute survey accuracy or deliver calibrated 95% coverage. P1 claims must name the estimator, configuration, sensor, surface classes, operating envelope, independent reference and building population.
