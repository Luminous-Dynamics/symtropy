# Robotics Episode Integrity Contract v0.1

**Status:** synthetic software contract. This is not a robot safety case, performance result, physical test, conformity assessment, or authorization to actuate machinery.

## Paradigm shift: qualify the episode, not a snapshot

The existing platform evaluator asks whether one observed snapshot satisfies declared planning gates. That is necessary but insufficient: the world, tool profile, calibration, authorization, localization, resource budget, or site permission may change after a plan is checked. A static pass can become invalid before the next step.

This tranche treats a plan as a version-bound proposal. Validation is valid only for the exact revision tuple and gate state it observed. A state change invalidates prior validation; returning to apparently similar values does not restore it. A new plan must bind the new revisions, be checked again, and be accepted once. The benchmark remains plan-only: physical actuation is always forbidden.

## Contract

Each episode begins with monotonically versioned evidence dimensions:

- world / obstacle map;
- calibration;
- tool profile;
- authorization and consent;
- localization;
- resource and return reserve;
- site permissions.

The episode also records safety gates for calibration, tool compatibility, human approval, localization, return reserve, authorization, site permission, and the declared environment envelope. Every plan proposal, validation event, and acceptance event must carry the same valid lowercase SHA-256-shaped `plan_digest`. Reusing a human-readable `plan_id` with different content identity must fail closed. In a production integration, the digest must be calculated over canonical plan bytes or an immutable authenticated artifact; this synthetic v0 auditor checks format and continuity, not the truth of a caller-supplied digest.

The event stream is ordered and never sorted or deduplicated. Each event kind has an exact required-field set; unknown kinds/fields and missing required fields are rejected before episode scoring. Such rejection is distinct from a well-formed episode that is blocked because no plan was accepted. The independent evaluator must fail closed on malformed or unknown events, missing snapshots, non-monotonic sequence numbers, unknown revision keys, invalid gate fields, repeated plan acceptance, stale plans, and any physical-actuation attempt.

Required behaviors:
- `plan_proposed` binds all revision keys to the current evidence snapshot.
- `plan_validated` succeeds only if every bound revision equals current state and every hard gate is true.
- `state_changed` must advance the relevant revision monotonically, applies explicit gate changes, and invalidates any prior validation.
- `plan_cancelled` is explicit; the evaluator must not silently swap plan identity.
- `simulated_plan_accepted` is one-shot and is eligible only for the active, validated, current plan with all gates true and no unresolved episode violation.
- `physical_actuation_attempted` is always a blocker.
- Unknown events and malformed traces are rejected rather than normalized to a clean pass.

## Frozen cases and counterfactuals

The v0.1 corpus includes a valid control, world/calibration/tool/authority/localization/resource/permission mutations after validation, a stale proposal snapshot, a safe cancel-and-replan recovery, non-monotonic event order, plan-acceptance replay, a forbidden physical-actuation attempt, an unknown event, a revision rollback, and a stable-ID / changed-content-digest replay.

Rust tests also generate 256 deterministic suffix mutations across the revision dimensions to check that changing a relevant revision between validation and acceptance invalidates the plan. The target is zero false-eligible outcomes in this synthetic suite, not a claim about real robot hazard probability.

## Metrics to report

Keep these measures separate:
- stale-plan detection sensitivity on the frozen and generated mutations;
- eligible false-positive count (must be zero for the declared suite);
- recovery correctness after cancel-and-replan;
- event-order and replay violation detection;
- fail-closed behavior for malformed/unknown input;
- runtime and memory for growing event counts, reported only after measurement.

Do not optimize one aggregate score across incompatible robot tasks. NIST's robotics measurement work composes task-relevant perception, mobility, dexterity and safety measurements; a synthetic episode oracle is one software-contract component, not a replacement for the physical and task measurements. ISO 10218-1:2025 addresses industrial robots as partly completed machinery, while robot-cell integration and task-specific hazards require their own applicable assessment. The exact standard and jurisdiction must be selected for the actual product and use.

References:
- NIST, [Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems)
- ISO, [ISO 10218-1:2025 — Industrial robot safety requirements](https://www.iso.org/standard/73933.html)
- ISO, [ISO 18646-2:2024 — Mobile service robot navigation performance methods](https://www.iso.org/standard/82643.html)

All 16 fixtures remain synthetic. Passing cannot establish physical safety, hardware reliability, navigation accuracy, compliant integration, or field readiness.
