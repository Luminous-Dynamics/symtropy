# Orbital Inspection Calibration-Chain Validity Contract v0.1

**Status:** research specification; not a calibration procedure or physical qualification  
**Parent:** [Symtropy #6845](https://github.com/Luminous-Dynamics/symtropy/issues/6845)  
**Work item:** [ORBITAL-CAL-VALIDITY-001 / #7339](https://github.com/Luminous-Dynamics/symtropy/issues/7339)  
**Related:** [#6925](https://github.com/Luminous-Dynamics/symtropy/pull/6925), [#7136](https://github.com/Luminous-Dynamics/symtropy/issues/7136), [#6846](https://github.com/Luminous-Dynamics/symtropy/issues/6846), [#5141](https://github.com/Luminous-Dynamics/symtropy/issues/5141), [#4671](https://github.com/Luminous-Dynamics/symtropy/issues/4671), [#4117](https://github.com/Luminous-Dynamics/symtropy/issues/4117)

## 1. The gap this contract closes

The current inspectability work has a sound geometric progression:

`support envelope + rectangular frustum -> NEXT_GATE_ELIGIBLE`

That result is deliberately not a visibility result. However, the geometry still assumes that the camera model, sensor-to-robot transform, robot kinematic state and relevant measurement setup are valid for the actual acquisition. A planned viewpoint can become inadmissible before execution because a lens, tool, mount, payload, robot configuration, datum, firmware, environment or calibration state changes.

This contract adds an upstream **calibration-validity precondition**. It is an adapter over existing evidence owners, not another calibration solver, uncertainty propagation engine, drift detector, robot controller or authority service.

The intended composition is:

`calibration validity at acquisition`
`  AND identity-matched support/frustum preconditions`
`  -> next-gate eligible candidate`
`  -> conservative occlusion / sampling / photometric / feature gates`
`  -> metrological decision under its own uncertainty profile`

Every arrow is a separately evidenced transition. No upstream pass promotes itself into a downstream claim.

## 2. Calibration is a scoped claim, not a Boolean

The evaluator must preserve at least these independently identified scopes:

| Scope | Example identity and applicability |
|---|---|
| Camera intrinsics | Camera serial, lens, focus, aperture, resolution/crop, distortion model, sensor mode |
| Camera-to-robot extrinsics | Camera, mount, tool, robot, hand-eye transform revision, frame convention |
| Robot base/kinematics | Robot serial, base frame, kinematic model/revision, configuration and payload/tool envelope |
| Asset/world datum | Asset and fixture revision, datum realization, registration transform and frame lineage |
| Measurement-system characterization | Scale, optical response, task-specific acceptance/reverification evidence and supported surface/material envelope |

A calibration valid for one scope does not imply validity for another. Robot repeatability is not absolute accuracy; a valid camera intrinsic calibration is not a valid hand-eye transform; an accurate transform does not establish feature-measurement uncertainty.

Each scope has its own certificate/evidence reference, source digest, validity interval, operating envelope, uncertainty reference and invalidation dependencies. Avoid a single global expiry date unless the source procedure explicitly defines one for the complete chain.

## 3. Inputs and state model

The v0.1 evaluator should consume canonical, schema-validated records equivalent to the following conceptual types.

### Calibration scope record

- `scope_id`, `scope_kind`, revision and exact covered instrument/robot/tool/mount/asset identities;
- calibration certificate ID, issuer/source identity, digest, procedure revision and calibration epoch;
- valid-from and valid-until timestamps, explicit interval semantics and time source;
- applicable software/firmware/model revisions and configuration digest;
- coordinate-frame IDs, units and transform identities where applicable;
- admitted condition envelope: temperature, warm-up state, payload, pose/workspace, optical mode and environment;
- uncertainty/covariance reference and its exact calibration revision;
- check-artifact policy and minimum required check evidence, where applicable;
- explicit dependency and invalidation-scope declarations.

### Acquisition-state record

- exact planned candidate and actual acquisition IDs;
- actual acquisition timestamp with bounded clock uncertainty;
- observed instrument, robot, camera, lens, mount, tool, payload, fixture and asset identities;
- actual environmental and operating conditions, with units and source identity;
- relevant software/firmware/algorithm revision;
- source snapshot and event-inventory completeness watermark.

The plan-time state and acquisition-time state are different inputs. A plan-time PASS is never cached as an acquisition-time PASS.

### Change/invalidation event

Each event carries an immutable ID/digest, event type, affected component/scope IDs, effective-time interval, observed-time interval, source/provenance, event completeness and a declared impact mapping. Examples include replacement, removal/reinstallation, collision/shock, datum movement, tool exchange, lens/focus change, firmware or algorithm update, payload change, thermal-envelope excursion, reference-check failure, certificate revocation and maintenance.

The evaluator must distinguish:
- an event definitely before acquisition;
- an event definitely after acquisition;
- an event whose time interval overlaps acquisition;
- an incomplete or untrusted event inventory.

An overlapping or incomplete history is not equivalent to “no change occurred”.

### Reference/check-artifact observation

Bind the artifact ID and certificate digest, characteristic IDs, instrument/calibration scope, observation digest, time, environment, residuals, expanded uncertainty, check-policy digest and sentinel outcome. The check policy determines maximum age/frequency and applicable operating envelope. Missing required checks, HOLD outcomes, wrong artifacts, expired certificates or uncertainty that consumes the allowed guard band make the affected scope ineligible or indeterminate.

The validity evaluator consumes check receipts; it does not calibrate, update coefficients, choose new control limits or reset the reference baseline.

## 4. Decision semantics

Suggested outcome enumeration:

- `VALID_WITHIN_SCOPE`: every required scope is identity-matched, current, within its declared domain, supported by complete event history and valid required checks.
- `EXPIRED`: the acquisition-time interval is definitely outside a required certificate's validity interval.
- `OUT_OF_SCOPE`: an actual condition is definitely outside the declared calibration envelope.
- `DRIFT_SUSPECTED`: a required independent check/sentinel reports a drift hold or an admitted drift criterion is breached.
- `IDENTITY_MISMATCH`: a required instrument, robot, camera, lens, transform, mount, datum or revision does not match.
- `INDETERMINATE`: required state, event history, clock bound, check evidence or scope mapping is missing, ambiguous or overlaps a validity boundary.
- `INVALID_INPUT`: malformed units, frame, digest, time interval, duplicate identity or invalid schema/semantic relation.

Use a deterministic, documented reason-code precedence so the independent oracle can reproduce the same result. Preserve all contributing reasons; precedence chooses the headline status but must not discard additional blockers.

### Time-boundary rule

Represent validity as a declared interval, preferably `[valid_from, valid_until)`, rather than relying on undocumented timestamp comparisons. If acquisition time is `[t-u, t+u]`:

- PASS the time test only when the entire interval is inside the validity interval;
- classify EXPIRED only when the entire interval is definitely after expiry;
- otherwise return INDETERMINATE.

The chosen interval convention and clock-uncertainty semantics must be frozen in the schema and fixtures.

### Event-time rule

Apply an invalidating event to an acquisition only when its effective-time interval is definitely at or before the acquisition interval. If the event and acquisition intervals overlap, return INDETERMINATE. Events definitely after acquisition do not retroactively invalidate that already-completed acquisition, but they invalidate later acquisitions within the affected scope.

A declared complete event inventory is required through the acquisition watermark. “No event supplied” is not evidence that no event occurred.

### Scope-impact rule

Use a reviewed, explicit event-to-scope impact table. A lens focus change may invalidate intrinsic/optical scopes without necessarily invalidating a robot-base calibration; a robot-base movement may invalidate world registration and hand-eye relationships. If the effect is unknown, invalidate the dependent chain conservatively. Do not invalidate unrelated scopes without a dependency reason.

## 5. Composition boundary

The adapter may emit:

- per-scope status and reason codes;
- complete calibration-chain status;
- exact acquisition-state and event-history digests;
- covariance/check-artifact references for downstream consumers;
- `may_enter_geometry_precondition=false|true` as a narrowly scoped routing field;
- required requalification evidence IDs or reason-coded missing evidence.

It must retain explicit non-claims:

- `visibility_assessed=false`
- `occlusion_assessed=false`
- `feature_measurability_assessed=false`
- `measurement_uncertainty_decided=false`
- `defect_detectability_assessed=false`
- `conformity_decided=false`
- `may_count_toward_visibility=false`
- `structural_acceptance_authorized=false`
- `flight_authorized=false`

The downstream gate must verify the receipt's exact camera/robot/asset identities, acquisition time, calibration revision and digest. A detached `VALID_WITHIN_SCOPE` string is not evidence.

## 6. Frozen synthetic challenge corpus

The first deterministic contract corpus should include:

1. Fully matching scopes, event history and check evidence inside every declared envelope.
2. View planned while valid but acquired after expiry.
3. Acquisition clock interval wholly inside, wholly outside, and straddling a validity boundary.
4. Camera serial, lens, focus, resolution, crop, optical mode or intrinsic revision mismatch.
5. Hand-eye transform, mount, tool, robot serial, robot base or frame-convention mismatch.
6. Payload and tool-centre-point changes inside and outside the admitted envelope.
7. Temperature/warm-up and workspace/pose conditions inside and outside the admitted envelope.
8. Collision, shock, mount release, removal/reinstallation or fixture movement after calibration.
9. Asset/datum revision, frame handedness and unit mutations.
10. Expired, revoked, future-issued, wrong-artifact or wrong-procedure certificate.
11. Pointwise reference-artifact guard violation.
12. Persistent drift caught by a frozen EWMA when individual check observations remain inside pointwise limits.
13. Missing, stale, delayed or out-of-envelope required check observation.
14. Artifact uncertainty consuming the remaining guard band.
15. Missing or incomplete change-event inventory.
16. Event timestamp overlapping acquisition because of clock uncertainty.
17. Replay of an old valid receipt after an invalidating event.
18. Attempted EWMA baseline reset, automatic coefficient correction or HOLD-to-CONTINUE rewrite.
19. Missing or calibration-revision-mismatched covariance reference.
20. A scope valid for camera geometry but not for a specified measurement characteristic, and the converse.

Add input permutation, canonicalization, unit-scaling where semantically valid, rigid-frame reparameterization and exact replay tests. A separate standard-library Python oracle must independently compute the statuses and complete canonical receipts. Python is test-only; production implementation remains Rust-first.

## 7. Benchmark ladder and scorecard

### D0 — deterministic contract correctness

- 100% Rust/oracle agreement on the frozen corpus.
- Zero false-valid outcomes on frozen critical-negative controls.
- Zero missing/unknown-event history promoted to valid.
- Zero invalidating event silently ignored.
- Zero stale plan-time PASS reused at acquisition time.
- Zero automatic recalibration or HOLD override.
- Exact deterministic replay and retained exact-head evidence.

### D1 — stateful model and fault injection

Replay complete event sequences with warm-up, temperature, payload/tool exchange, robot pose/workspace, clock drift, delayed events, maintenance and sensor degradation. Report invalidation detection latency by event class, false holds, requalification burden and the number of candidate views retained/rejected. Keep event generation, event delivery and event evaluation distinct so missing-event failures can be tested.

### D2 — matched industrial workflow comparison

Compare against:
- periodic acceptance/reverification-only workflow;
- periodic checks plus a pointwise check-standard rule;
- pointwise plus frozen EWMA/reference-artifact sentinel;
- full scope/event-aware validity adapter.

Run identical synthetic event histories and identical acceptance policies. Report missed invalidations, false holds, detection delay and requalification burden. Do not imply that commercial products lack internal state or capabilities based only on public brochures.

### D3 — traceable physical pilot

Only when an appropriate robot, optical sensor and calibrated check artifact are available, pre-register the procedure and test design. Include warm-up, payload, pose/workspace and environmental regimes. Use independent reference metrology and held-out conditions; report bias, repeatability, uncertainty, empirical interval coverage, guard-band decisions and requalification delay separately. A physical pilot is not required for D0/D1 completion.

### Metrics that must remain separate

- false-valid and false-invalid/hold counts;
- event-to-invalidation latency by event class;
- stale/out-of-scope acquisitions rejected;
- candidate availability after invalidation;
- check-artifact residual and uncertainty;
- uncertainty/covariance output from the metrology owner;
- time and cost to requalify;
- aborted/repeated inspection time and human review burden;
- cost per independently verified engineering characteristic, retaining failures, holds, repeats and abstentions in the denominator.

A zero false-valid count on a finite synthetic corpus is a contract result—not a field reliability rate.

## 8. Industry and standards positioning

Industrial inspection platforms already combine measurement, CAD/GD&T analysis, automated routines, calibration-related workflows and reporting. Examples documented by their vendors include ZEISS INSPECT with Virtual Measuring Room, Creaform Metrology Suite and FARO CAM2. This work should interoperate with such systems rather than replace their scanners, calibration processes or metrology software.

The differentiated question is narrower and testable: can a vendor-neutral machine-readable evidence adapter bind each inspection candidate to the exact calibration scope, current state, event history, reference checks and uncertainty evidence, then invalidate/requalify it without silently promoting a stale measurement?

Relevant standards and research context:

- [ISO 10360-13:2021](https://www.iso.org/standard/74957.html): acceptance/reverification tests for optical 3D coordinate measuring systems, with the standard's stated cooperative-surface scope.
- [ISO 10360-12:2016](https://www.iso.org/standard/63931.html): acceptance/reverification tests for articulated-arm CMMs, including optional optical distance sensors; the standard does not prescribe when or how often reverification must occur.
- [ISO 14253-1:2017](https://www.iso.org/standard/70137.html): uncertainty-aware conformity decision rules; calibration validity is not a conformity decision.
- Khirani et al. (2026), [thermal and payload effects on industrial-robot repeatability](https://doi.org/10.1016/j.measurement.2026.121565): two studied robot configurations showed materially different warm-up and payload sensitivities; do not generalize their numerical values into universal limits.
- Chekh et al. (18 November 2026), [Traceable Metrological Digital Twin for Robot-Based 3D Inspection](https://doi.org/10.3390/metrology6040088): a traceable robot/sensor calibration and uncertainty chain was cross-validated against CMM/VCMM references, while the authors explicitly limit claims about environmental and long-term stability.

No claim of ISO, GUM or vendor-system equivalence follows from implementing this contract.

## 9. Implementation sequence

1. Freeze the schema, scope-impact table, time-boundary rule and reason-code order.
2. Implement a small, dependency-light Rust evaluator in a dedicated nested benchmark crate; do not alter the existing #6925 geometry semantics.
3. Add canonical fixtures, the independent standard-library Python oracle, mutation tests and complete receipt equality.
4. Add a dedicated workflow for format, tests, strict Clippy, replay, oracle comparison, artifact retention and exact-head manifest.
5. Integrate at the candidate-acquisition boundary with #6925/#7136 only after the independent contract is stable.
6. Bind exact uncertainty/covariance references from #6846 and check-artifact validity from the admitted monitoring owner.
7. Later run a traceable physical pilot under an approved metrology procedure.

## 10. Claim ceiling

Initial PASS means only that a declared synthetic validity contract accepted the supplied calibration-scope evidence and acquisition-state history. It does not establish physical calibration, robot accuracy, optical measurement performance, ISO/GUM conformity, feature measurability, defect detection, NDE/POD, structural acceptance, certification or flight readiness. Physical claims require traceable artifacts, an independent reference system, held-out conditions and external review.
