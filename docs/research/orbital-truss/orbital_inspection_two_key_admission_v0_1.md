# Orbital inspection two-key action admission v0.1

## Purpose

Compose two independent questions without allowing one to compensate for the other:

- Can the vehicle execute this action within the separately qualified physical-safety envelope?
- Can the resulting observation be admitted as evidence for this exact characteristic and acquisition?

The adapter is a finite reference gate, not a controller or physics engine.

## Action classes

### INSPECTION

Requires all of:
- physical-safety disposition PASS for the exact action, vehicle, trajectory revision and horizon;
- post-action event portfolio PASS and complete for the same action/horizon;
- measurement acquisition PASS for the exact action, acquisition, camera, asset and feature;
- cross-receipt scope-coherence PASS.

Any FAIL rejects the inspection action. Any UNKNOWN, STALE, absent or incomplete required receipt returns INDETERMINATE. A known identity mismatch is OUT_OF_SCOPE. Malformed source binding is INVALID_INPUT.

### RECOVERY

HOLD, SAFE-RETREAT or ABORT-INSPECTION may be admitted when the physical-safety owner supplies PASS for the exact action/vehicle/trajectory/horizon. A recovery action carries no post-action inspection portfolio or measurement claim and must not populate acquisition/camera/asset/feature measurement scope. Safety FAIL rejects it; safety UNKNOWN/STALE/missing does not admit it.

## Non-compensatory semantics

Decision value, delta-v, duration and compute cost are retained in the receipt but are not used to decide admissibility. They can be optimized only after the independent gate has filtered the candidate set. A very high predicted information value cannot promote an unsafe action. A cheap or fuel-efficient action cannot promote an evidence-inadmissible measurement.

The physical-safety interface and measurement-evidence interface are intentionally separate:
- #6828 owns decision-grade collision-risk evidence, not an automatic safe-action shield;
- #7264 owns coupled post-maneuver event portfolio effects;
- #7393/#7178 own inspection recourse and decision-value prioritization;
- #7339/#5631 own calibration and clock semantics;
- #7384/#7399/#7422 own acquisition-motion and timing bounds;
- #7425 owns cross-receipt identity/interval coherence;
- Mycelix owns custody/authentication concerns.

If no applicable trajectory/keep-out shield or safety admissibility receipt exists, the physical-safety gate is UNKNOWN. The adapter must not infer collision-free execution from a collision-probability estimate or from an information score.

## Frozen corpus

Twenty synthetic cases cover:
1. all gates PASS;
2. physical-safety FAIL and UNKNOWN;
3. measurement acquisition FAIL and UNKNOWN;
4. acquisition/camera/asset/feature scope mismatch;
5. incomplete post-action portfolio;
6. stale measurement and missing physical-safety/portfolio/measurement receipts;
7. safety-receipt trajectory mismatch;
8. safe recovery with no measurement claim and recovery with unknown safety;
9. high information value but failed safety;
10. high-resource/low-value but otherwise admissible inspection;
11. malformed receipt digest;
12. post-action portfolio FAIL;
13. scope-coherence FAIL.

The independent Python oracle and Rust evaluator must agree on the full receipt. Case ordering and source input ordering do not alter canonical output.

## Matched orbital comparator tournament

Reproduce Stephenson and Schaub (2026), DOI 10.1007/s40295-026-00631-w, on its declared scenario before extension:
- shielded closed-loop RL;
- offline PRM–MIQCQP;
- hybrid offline plan plus onboard receding-horizon adaptation.

Then compare the following under the same dynamics, candidate action pool, starting state, keep-out constraint, propellant/time budgets and sensing configuration:
- physical-safety shield alone;
- measurement-evidence gate alone as a diagnostic ablation in a protected simulator only;
- conjunctive physical-safety plus measurement-evidence admission;
- #7393 recourse / #7178 decision-value ranking over the doubly admissible candidates;
- exact finite oracle for small candidate sets.

Report fuel/delta-v, time, shield intervention rate and magnitude, keep-out violations, post-action portfolio completeness, evidence rejection/indeterminate rates, eligible characteristic yield, correct decision/abstention, reacquisition count, compute, communication and onboard latency separately. The objective is a non-dominated frontier with zero hard-gate violations, not a weighted scalar score.

## Claim ceiling

The evaluator demonstrates only deterministic composition of declared synthetic upstream dispositions. It does not calculate or prove a safe set, collision probability, reachable tube, operational action, measurement truth, POD/NDE, metrology conformity, or flight readiness.
