# Orbital admitted observation handoff v0.1

## Purpose and paradigm shift

The inspection process must preserve every commanded acquisition attempt and its realized public outcome. Recording only successful feature measurements conditions the downstream statistical history on acquisition success and can create selection bias in precisely the scenes or operating states where visibility, motion, calibration, photometry, timing or capture reliability are weakest.

A failed acquisition is not a valid physical measurement. It may still be an informative **observation-process outcome** if the frozen conditional observation model explicitly includes that outcome and missingness mechanism. Otherwise the process must abstain; it must never silently discard the attempt or coerce its outcome into a nearby success class.

## Contract boundary

The handoff is deliberately a small adapter:
1. bind the acquisition attempt, action and exact public-history cutoff;
2. bind the #7426 same-scope result and the qualified owner receipts;
3. validate the typed public outcome and missing-channel record;
4. verify that the exact outcome and missingness semantics are included in a pinned #6887 conditional-kernel profile;
5. either pass the exact outcome as a public sequential observation or refuse to pass it.

This layer does not compute probabilities, likelihoods, e-values, alpha spending, confidence sequences, surviving worlds, decisions or actions. #6887 alone owns those operations. #7292 remains the source-bound downstream world-set receipt adapter.

## Kernel profile is authoritative for required evidence owners

Each predeclared profile binds:
- profile ID and revision;
- profile receipt digest reference and current/revoked disposition;
- supported public outcome kinds;
- missingness semantics declared/not declared;
- whether action realization/readback is required;
- the exact set of required owner kinds.

The attempt's owner bindings must reproduce that required-owner set exactly. A sender cannot downgrade calibration, timing, feature, metrology or another required gate by setting a per-attempt `required=false`. Extra non-required owner bindings must be explicitly marked NOT_REQUIRED. Duplicate owners, missing required owners and mismatched required flags are invalid.

The profile digest is a source reference only. This synthetic adapter does not authenticate the profile issuer or fetch the underlying profile bytes. Production use must consume the exact approved profile registry and verified custody path from the canonical evidence owner.

## Attempt and cutoff identity

Each attempt binds campaign ID, unique attempt ID, sequence number, declared campaign attempt count, action ID, policy revision, command receipt, action realization/readback, decision cutoff ID, sequence watermark, prior-history digest, and acquisition scope receipt.

A decision uses only the public history before the attempt. The attempt sequence must equal its event watermark and prior-history maximum sequence must be strictly less than the attempt sequence. Lookahead, replayed attempts and duplicate sequence numbers fail closed.

For this v0.1 frozen campaign contract, each campaign receipt inventory must contain exactly the contiguous sequence 1..N for the declared count. A missing attempt or sequence gap is an error, not an implicit missing-at-random observation. This structural check is not a physical claim that the acquisition system captured every real-world attempt; #6659/#6715 own that qualification.

## Typed public outcome

Measurement outcomes:
- MEASUREMENT_AVAILABLE;
- NO_INDICATION;
- INDICATION_REPORTED.

These require a payload digest reference, at least one observed channel, no missing channels, a coherent acquisition scope, verified action readback when the profile requires it, and every required owner gate PASS and current.

Observation-process failures include:
- OCCLUDED;
- CAPTURE_DROPPED;
- TRIGGER_OR_FRAME_UNASSOCIATED;
- SENSOR_SATURATED;
- PHOTOMETRY_OUT_OF_PROFILE;
- MOTION_BUDGET_EXCEEDED;
- CALIBRATION_INVALID;
- FEATURE_NOT_IDENTIFIABLE;
- METROLOGY_INDETERMINATE;
- ACTION_NOT_REALIZED;
- OTHER_DECLARED_FAILURE.

These may be handed forward only when the exact kind occurs in the frozen kernel support and the corresponding owner gate / realization state is consistent. Examples: OCCLUDED must bind a failed geometry owner; MOTION_BUDGET_EXCEEDED must bind a failed motion owner; ACTION_NOT_REALIZED must contradict neither command nor readback state.

An event outside the frozen kernel support is ABSTAIN_UNMODELLED_OUTCOME. A known scope or calibration failure prevents a physical measurement from being admitted. Matching a digest-shaped string is never authentication. Common-cause channel failures remain joint public outcome/missingness records; they are not factorized by this adapter.

## Dispositions

- MEASUREMENT_ADMISSIBLE: source-bound measurement payload; all required gates pass.
- MODELLED_OBSERVATION_PROCESS_OUTCOME: acquisition failure is explicitly included in the frozen observation kernel; it is handed forward as a failure outcome, not a measurement.
- ABSTAIN_UNMODELLED_OUTCOME: outcome/missingness/kernel/source/gate semantics are unresolved or outside the declared support.
- INADMISSIBLE_MEASUREMENT: scope or a known physical measurement gate fails.
- INVALID_INPUT: malformed, duplicate, contradictory, lookahead, replayed or incomplete attempt record.

Both MEASUREMENT_ADMISSIBLE and MODELLED_OBSERVATION_PROCESS_OUTCOME can be eligible for #6887's declared conditional update. The adapter itself performs no statistical update. An unmodelled outcome must never be silently omitted and then treated as though no attempt occurred.

## Frozen adversarial corpus and tests

The 32 cases cover successful measurements, negative/no-indication examinations, positive indications, calibration/timing/motion/capture/geometry/photometry/feature failures, unmodelled occlusion, unknown metrology, action realization mismatch, cutoff lookahead, scope mismatch, joint missing-channel failures, revoked owner and kernel sources, unknown outcomes, malformed digests, duplicate channels, owner-order invariance, required-owner bypass, missingness-not-declared, and incomplete kernel owner sets.

Additional tests mutate the campaign fixture to prove duplicate attempt IDs and non-contiguous sequence inventories are rejected.

## Competitive benchmark and prospective targets

Compare the same complete attempt stream and frozen kernel under:
- survivor-only ingestion (intentional selection-bias negative control);
- generic missing-at-random coercion (misspecified control);
- typed public outcome handoff with exact missingness;
- #6887 sequential process fed the complete stream;
- exhaustive tiny-domain sequential oracle (evaluator-only upper bound).

Report omitted-attempt rate, sequence-gap/replay rate, false singleton decision commitment, anytime-valid world survival, outcome/failure-stratified error, appropriate abstention, unnecessary abstention, decision-set size, probes/exposure to resolution, latency, energy, replay and source completeness separately.

Prospective synthetic gates: zero missing/duplicate attempt records admitted; zero unmodelled outcomes silently coerced; zero required-owner bypass; zero cutoff lookahead; exact receipt equality; zero e-process/statistical updates from this adapter. Physical POD and false-alarm claims require appropriate modality-specific data and confidence bounds; no synthetic result supports them.

## Claim ceiling

The evaluator checks only declared synthetic receipt semantics. It does not authenticate sources, establish physical capture completeness, validate the #6887 conditional model, assert missing-at-random, prove the world is in the model family, calculate POD, establish feature/metrology truth, accept/reject hardware or authorize an orbital mission.
