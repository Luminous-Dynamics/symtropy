# SAFRB ↔ Power Fabric electricity-service trajectory interop

**Work item:** [SAFRB-SERVICE-TRAJECTORY-INTEROP-001 #7338](https://github.com/Luminous-Dynamics/symtropy/issues/7338)  
**SAFRB owner:** [v0.8 core evaluator PR #7294](https://github.com/Luminous-Dynamics/symtropy/pull/7294)  
**Generic trajectory owner:** [POWER-RESTORATION-SERVICE-TRAJECTORY-001 #6449 / PR #6450](https://github.com/Luminous-Dynamics/symtropy/pull/6450)  
**Research cut:** 24 November 2026  
**Status:** adapter contract/research only; no integration implementation or end-to-end claim

## 1. Existing systems to compose, not clone

SAFRB v0.8 calculates time-bounded energy delivery intervals and critical-floor outcomes at one declared service boundary. Power Fabric #6450 separately validates an electricity service trajectory against a frozen restoration benchmark case and run receipt. The bridge contract #6235 owns typed translation and time alignment between planned work, observed work, modelled service and measured service. #6352 owns run/attempt receipt accounting; #6405/#6427 own offline artifact and complete claim verification; #6317 owns branch and beneficiary hard-service-floor/risk qualification.

Do not add a second generic service trajectory, load mapping, weighted score, source signature verifier or beneficiary cascade engine to SAFRB.

## 2. Critical schema discovery

The current #6450 v0.1 `ServiceSample` contains:
- `demandKwh`;
- one optional `deliveredKwh`;
- status (`known`, `unavailable`, `unknown`, `conflicting`);
- optional `uncertaintyKwh`;
- source evidence references/digests and conflict records.

Its known sample validation requires `0 < deliveredKwh ≤ demandKwh`; unavailable requires explicit `deliveredKwh = 0`; unknown/conflicting carry no numeric delivery. The weighted scorer multiplies the supplied `deliveredKwh` by the load priority weight and does not derive conservative lower/upper values from `uncertaintyKwh`. That field is checked for finite, non-negative form but is not an interval-bound operator in the weighted score.

**Consequence:** setting `uncertaintyKwh = upper - lower` while setting `deliveredKwh = lower` can be a useful descriptive projection, but the generic score remains a lower-bound point sum. It does not become an interval-aware scorer. The original v0.8 interval and floor must remain independently verifiable in a separate SAFRB bound certificate.

## 3. Exact mapping

| SAFRB v0.8 | Power Fabric #6450 | Rule |
|---|---|---|
| One `serviceId` + `boundaryId` | One frozen case `loadId` | Require an explicit one-to-one case-bound mapping. Reject ambiguous/multiple loads; multi-load aggregation belongs to #6450's digest-bound cascade mapping. |
| Window `startUnixS`, `durationSeconds` | `firstIntervalStartUnixMs`, `intervalDurationSeconds`, `step` | Convert seconds to milliseconds with checked arithmetic; every SAFRB interval must match exactly one frozen case step. |
| `targetWh` | Case-derived `demandKwh` | Require exact demand agreement within the generic auditor's declared numeric tolerance: `activePowerKw × intervalSeconds / 3600` kWh; `targetWh / 1000` must match that denominator. |
| `deliveredLowerWh` | `deliveredKwh` | If lower bound is positive and evidence is admitted, map `lower / 1000` only. It is a conservative lower-bound projection, not an asserted point estimate of actual delivery. |
| `deliveredUpperWh` | SAFRB bound sidecar | Preserve in the signed/content-addressed v0.8 certificate. Never substitute upper bound into the generic weighted score. |
| `deliveredUpperWh - deliveredLowerWh` | Optional `uncertaintyKwh` | Descriptive metadata only. Never claim #6450's current score has propagated this interval. |
| `criticalFloorWh` | SAFRB bound sidecar and #6317 floor input | A separate hard predicate. Do not turn it into a weighted-average penalty or allow aggregate weighted kWh to override a failed/indeterminate floor. |
| `Evidence` + source IDs | `sourceArtifacts`, producer, evidence provider | Reconcile exact source bytes/digests, producer roles, instrument calibration or model provenance under #6405/#6427. |

## 4. Three-valued conservative projection

For each interval, retain the original SAFRB outcome and apply these adapter rules:

1. **Positive lower bound:** if `deliveredLowerWh > 0`, and case demand, evidence class, source identity and artifacts pass, project `deliveredLowerWh / 1000` as the generic `deliveredKwh`. Keep the upper bound and floor in the SAFRB sidecar.
2. **Zero lower, positive upper:** `0 ≤ actual ≤ upper` does not prove positive delivery. Emit an Unknown/inconclusive service sample and block a completed service score for that interval. Do not report unavailable and do not replace the value with the midpoint.
3. **Both bounds zero:** do not automatically label unavailable from the numeric interval alone. Require an independently admissible source record explicitly establishing zero delivered service for the interval; otherwise retain Unknown.
4. **Upper bound over demand / target mismatch:** reject the mapping. Do not saturate/clamp a contradictory interval into a successful sample.
5. **Scenario or reported evidence:** retain as fixture/claim only. Never promote it into modelled or measured field service.

The sidecar certificate is required even when the lower-bound projection is accepted. The generic trajectory provides the conservative service score; the SAFRB certificate preserves the interval and hard-floor verdict. Those are complementary, not interchangeable, products.

## 5. Evidence class is not self-authenticating

SAFRB v0.8 currently checks that a measured evidence SHA reference is 64 hex characters and that an upstream artifact ID exists. It does not resolve source bytes, verify signatures, establish instrument calibration or independently validate physical truth.

Therefore, a v0.8 `EvidenceClass::Measured` enum alone cannot authorize #6450's `measured_service` stream. Measured admission requires the #6450 instrument and calibration contract plus an EvidenceProvider role with exact source bytes/digests verified through #6405/#6427. Modelled service similarly requires a pinned model/source/configuration, distinct evidence-provider identity and the applicable independent validation receipt.

## 6. Run and artifact reconciliation

For an accepted adapter run:
- every v0.8 window maps to one matching case step;
- every projected sample maps to the exact load and demand denominator;
- every projected sample's source reference/digest exists in the service trajectory's source-artifact inventory;
- the v0.8 bound certificate is separately content-addressed and its digest is referenced by the trajectory or parent artifact inventory;
- #6352 per-decision `serviceTrajectory` artifacts reconcile exactly to #6450 `parentServiceArtifacts`;
- the final #6352 trajectory reference, #6450 canonical trajectory and final SAFRB sidecar bind the same case, run, service, boundary and horizon;
- #6405/#6427 independently verify bytes, signatures, issuer roles and parent links;
- #6317 consumes the accepted per-branch service evidence and SAFRB hard-floor results without duplicating the underlying service measurement.

The current v0.8 library and #6450 contract are on separate draft branches. This design must not be represented as implemented or interoperable until an adapter has tests against accepted schema versions and the complete exact-head chain passes.

## 7. Adversarial conformance matrix

| Attack / mismatch | Required result |
|---|---|
| Wrong case, run, load or boundary | Reject |
| Duplicate load mapping or one-to-many implicit mapping | Reject |
| Target differs from case demand by 1 Wh | Reject |
| Wh-to-kWh or second-to-millisecond factor error | Reject |
| Lower zero, upper positive | Unknown/inconclusive; no completed score |
| Both bounds zero without independent zero-delivery evidence | Unknown/inconclusive |
| Upper bound or midpoint used as delivered score | Reject |
| Measured enum plus shape-valid but unresolved digest | Reject |
| Scenario fixture or reported-only source promoted to field | Reject |
| Weighted score passes while critical floor fails/uncertain | Preserve floor failure/uncertainty; no successful continuity claim |
| Missing per-decision parent artifact or mismatch in exact parent set | Reject |
| Equivalent interval split / absolute clock shift | Invariant outcomes |

## 8. Research and benchmark consequences

The right comparative test is not simply whether the SAFRB weighted kWh total matches #6450. Evaluate the assurance overlay separately:

- false positive rate for completed service claims;
- false-positive critical-floor passes;
- percentage of interval width discarded by lower-bound projection (reported as information loss, not hidden);
- unresolved/Unknown sample rate;
- false rejection rate and reason categories;
- exact source/parent artifact coverage;
- adapter and independent-verifier p50/p95 latency;
- weighted energy, critical-floor continuity, worst beneficiary cohort, EENS and restoration latency as separate outcomes.

The research hypothesis is that preserving an independently checkable interval certificate alongside the conservative point projection reduces unsupported service claims without sacrificing auditability. It must be tested against the same frozen runs both with and without the SAFRB sidecar, with all incomplete and unresolved samples retained in the denominator.

## 9. Claim boundary

This mapping is a proposed contract only. It does not currently connect SAFRB v0.8 to #6450, produce a signed bound certificate, qualify electricity power quality, prove AC or three-phase feasibility, establish protection coordination/black-start safety, verify field measurements, grant resource access or demonstrate superiority over industry planners.
