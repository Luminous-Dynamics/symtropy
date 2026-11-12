# Scan-to-BIM residual uncertainty adapter v0.1

**Status:** deterministic covariance-projection primitive with explicit qualification dispositions; no real survey calibration or uncertainty acceptance claim.  
**Issue:** #4227 — SCAN-BIM-RESIDUAL-UNCERTAINTY-001.  
**Stack:** after #4225 area-uniform sampler, #4205 directed accuracy and #4188 candidate/evaluation receipt.  
**Survey evidence owner:** #1607, PR #1609.  
**Research date:** 2026-11-08.

## Why this is a separate evaluation dimension

A geometric residual is not an uncertainty statement. A 12 mm point-to-surface difference is not equally informative when the underlying measurement standard uncertainty is 2 mm versus 30 mm. More importantly, millions of points captured by one registered scan do not constitute millions of independent measurements: a shared pose or registration error can move all points together.

Scan-to-BIM must report the geometric residual and its evidence-qualified uncertainty separately. It must not convert point count or a low residual into a calibrated confidence claim.

## Owner boundary: consume #1607, do not duplicate it

The upstream survey owner already defines:
- explicit coordinate frames, axis semantics/order, handedness, horizontal/vertical reference and units;
- `SurveyObservationV1` and `SurveyUncertaintyV1`, including standard deviations, absolute bounds and 3×3 covariance;
- covariance finiteness, symmetry and positive-semidefinite validation;
- content-addressed transform operations and resource artifacts;
- processing receipts with fit observations and independent checkpoints separated;
- control adjustment using weighted SVD, full formal covariance and held-out checkpoint records;
- RMSE_H/RMSE_V, detection limit and changed-volume uncertainty fields.

The Scan-to-BIM adapter consumes those evidence products and digests. It does not implement another survey observation schema, covariance validator, adjustment engine, geodesy/CRS engine or registration routine.

### An important gap in the existing survey transform record

The current `SurveyTransformV1` binds the exact operation and resource artifacts and may declare horizontal/vertical 95% accuracy values. Those scalar accuracy fields are **not** a full 6×6 rigid-pose covariance, do not encode anisotropy/correlation, and cannot be transformed into one without additional statistical assumptions.

Accordingly, the first adapter version can qualify residual covariance only when the caller can show that transform uncertainty is already propagated into the sample covariance or that no uncertain transform was required. A scalar-only transform accuracy result produces `Unqualified`, not a fabricated covariance.

## Implemented local-normal residual model

For sample point covariance `Σp`, target/surface point covariance `Σs`, target unit normal `n`, and cross-covariance `Cps = Cov(p_sample, p_target)`, the normal residual variance is:

\[
\sigma_d^2 = n^T(\Sigma_p + \Sigma_s - C_{ps} - C_{ps}^T)n
\]

The implementation reports a **conditional** normal-residual covariance: the supplied target normal is treated as fixed. Target-normal estimation uncertainty is not propagated in v0.1.

The implementation:
- validates both input 3×3 covariance matrices as finite, symmetric and positive-semidefinite;
- converts covariance and cross-covariance from each declared native linear unit to m²;
- requires an identical non-empty frame ID and a unit target normal;
- preserves full off-diagonal terms;
- validates the 6×6 joint sample/target covariance block when cross-correlation or independence is declared;
- checks that the projected variance is finite and non-negative within a scale-relative numerical tolerance;
- returns variance in m² and one-sigma standard uncertainty in metres only when qualification conditions are met. The output is conditional on the target normal being fixed; it is not a full surface/registration uncertainty budget.

A declared independence attestation is content-addressed and distinct from a missing cross-covariance. The code does not infer independence from the absence of data. If correlation is unknown, the output carries `UnknownSampleTargetCorrelation` and omits the numeric sigma.

## Transform and correlation dispositions

The result is one of two top-level dispositions:

| Disposition | Meaning | Numeric standard uncertainty |
|---|---|---|
| `QualifiedConditionalCovariance` | Same frame, compatible units, valid PSD covariance, cross-covariance or explicit independence evidence, and transform uncertainty accounted for | Present |
| `Unqualified` | One or more required evidence relationships are missing or only scalar accuracy is supplied | Absent |

Unqualification reasons are retained:
- `UnknownSampleTargetCorrelation`;
- `TransformCovarianceUnavailable`;
- `TransformOnlyScalarAccuracy`.

Invalid frame, normal, digest, covariance or joint covariance is an error, not an unqualified success.

The primitive expects covariance artifacts to be supplied in the frame in which the geometric residual is computed. It does not perform coordinate transforms. `IncludedInSampleCovariance` is an upstream assertion bound to a transform receipt digest; its content and provenance must be verified by the survey evidence owner before use.

## Numerical and adversarial validation

The synthetic test suite includes:
- anisotropic covariance projected on a face normal;
- non-diagonal covariance under an oblique normal;
- explicit cross-covariance reducing the residual variance relative to the independent case;
- unknown correlation returning no numeric sigma;
- unavailable transform covariance and scalar-only 95% accuracy remaining unqualified;
- metre/mm covariance and mixed-unit cross-covariance conversion;
- frame mismatch and non-unit normal rejection;
- non-PSD sample covariance and an invalid 6×6 joint covariance block;
- malformed covariance receipt digest rejection.

These are numerical contract tests, not survey instrument or point-cloud registration qualification.

## Research baselines and implications

Zhou, Gu & Dong (2026), *Point cloud registration with enhanced accuracy via a synthetic stochastic model*, Expert Systems with Applications 302, article 130539, DOI 10.1016/j.eswa.2025.130539, argue that diagonal/block-diagonal error models can be over-optimistic when source/reference point-cloud errors are correlated. Their synthetic variance–covariance matrix distinguishes non-correlating, functionally correlating and stochastically correlating errors and uses a Gauss–Helmert model in which both clouds are error-affected. The study reports up to about 70% lower parameter RMSE in its simulations and improvement over five simplified stochastic baselines in an indoor TLS case. These results are study-specific; reproducing them requires matching the stochastic model, data and observation setup. Source: https://www.sciencedirect.com/science/article/pii/S0957417425041545

Xu, Wysocki & Holst (2026), *From Propagation to Prediction: Point-level Uncertainty Evaluation of MLS Point Clouds under Limited Ground Truth*, ISPRS Annals XI-1-2026, 463–471, DOI 10.5194/isprs-annals-XI-1-2026-463-2026, investigate predicting point-level MLS uncertainty from geometric features where ground truth is limited. Their XGBoost model is reported as approximately three times faster than Random Forest with comparable accuracy in their test. This is a predictive quality-control baseline, not a replacement for instrument calibration, covariance propagation or independent absolute checkpoints. Source: https://isprs-annals.copernicus.org/articles/XI-1-2026/463/2026/

The right competitive comparison is therefore at least three separate tracks:
1. propagated physical/survey covariance and transform uncertainty;
2. data-driven per-point uncertainty prediction, calibrated on spatially/building-disjoint independent checkpoints;
3. empirical residual and selective-risk calibration on genuinely held-out buildings.

Do not collapse these into one probability or confidence scalar.

## Deliberate omissions

This v0.1 primitive does not:
- parse #1607 survey envelopes or opaque uncertainty artifacts itself;
- create or solve a CRS/pose transformation;
- propagate a 6×6 transform covariance unless the upstream adapter has already incorporated it into the supplied point covariance;
- account for target-plane normal uncertainty or surface-fit uncertainty beyond the supplied target point covariance;
- model spatial covariance across an entire point cloud or infer cross-covariance from shared acquisition IDs;
- derive a 95% confidence interval or convert one sigma into a hard tolerance;
- calibrate the uncertainty method against independent checkpoints;
- decide whether a BIM element is accepted or a model mutation is authorized.

An upcoming adapter can consume the versioned #1607 envelope and link transform/covariance artifacts, but it must preserve the current fail-closed semantics and carry the fixed-normal conditionality into any downstream report. A full stochastic-registration reproduction should remain its own benchmark track.

## Next gates

1. Connect the adapter to the #1607 content-addressed survey envelope without duplicating its schema types.
2. Obtain a typed transform-pose covariance artifact or keep the result unqualified when only scalar transform accuracy is present.
3. Validate transform Jacobian propagation against finite differences and independent analytic rigid-pose cases.
4. Add local surface-normal uncertainty and spatially correlated point-cloud covariance.
5. Calibrate predicted/propagated uncertainty on spatially and building-disjoint held-out checkpoints.
6. Report risk–coverage and interval coverage by sensor, distance, incidence angle, range, surface class and occlusion state.

## Claim ceiling

The implementation computes a directional residual standard uncertainty from declared, compatible covariance evidence. It is not a calibrated 95% interval, official survey result, building-code tolerance decision, proof of absolute accuracy, or evidence of SOTA/industry superiority.
