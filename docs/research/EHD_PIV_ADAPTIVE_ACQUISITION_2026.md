# EHD-PIV-ADAPT-001 — risk-aware adaptive multi-fidelity diagnostic scheduling

**Issue:** #7097  
**Implementation:** `tools/ehd-piv-adaptive-acquisition/`  
**Status:** deterministic reference and manufactured tests authored; exact-head CI pending.

## Paradigm

Treat the measurement system as an adaptive instrument, not a passive pipeline. Low-cost sentinel measurements identify when high-cost diagnostics add decision-relevant evidence. The scheduler then selects the least-cost set of diagnostics that can satisfy all currently active evidence requirements.

This is related to the 2026 HOLODROPS/HOLODROPS-TRACK/ePIV design: event-based PIV continuously monitors cloud motion, and is used to identify scientifically valuable windows for targeted 3D holographic particle tracking. That is an adjacent instrumentation precedent, not an EHD baseline or an evidence of this scheduler's performance.

## Deterministic v0.1 policy

The sentinel record binds acquisition, dependence cluster, protocol and observer-state IDs. It carries:
- field uncertainty ratio;
- out-of-domain score;
- independent-diagnostic model disagreement;
- conservation residual;
- age of last high-fidelity acquisition;
- observed bulk, near-wall, shear-layer and wake regions;
- reference evidence, diagnostic quality and observer-state qualification.

The action catalog carries action cost, latency, capabilities, availability, observer impact and evidence-receipt requirements.

The controller converts triggers and evidence target into required capabilities, then performs a bounded exhaustive set-cover search (maximum 16 action options). It returns the lowest-cost feasible set, using latency and stable action IDs as tie-breakers. Any action with potentially perturbative or unknown observer impact must be accompanied by observer requalification after the measurement. An already-unqualified observer state requires requalification before measurement; when both conditions hold, both boundaries are required. If no complete capability cover fits the budget/latency/action limits, the controller abstains.

The controller does not actuate equipment or infer that an action is physically safe. A field-covariance target always requires high-resolution PIV, coverage of declared regions and reference qualification, even when the sentinel snapshot is otherwise benign.

## Hard evidence constraints

- Thrust claims require direct force, complete control-volume, independent velocity and electrical-power capabilities.
- Integrated-force claims require direct force and complete control-volume capabilities.
- Three-dimensional velocity claims require volumetric particle tracking and an independent velocity reference.
- Field covariance claims cannot proceed when reference evidence is unqualified.
- Unknown or unqualified observer state requires requalification.
- Every selected diagnostic action requires an evidence receipt; a modality ID alone does not qualify it.

## Manufactured benchmark

The 21 authored tests exercise benign sentinel continuation, uncertainty escalation, out-of-domain 3D escalation, reference/observer requalification, hard force/thrust capability requirements, missing direct-force resource, budget abstention, potentially perturbative diagnostics, incomplete telemetry, stale high-fidelity data, cost/tie-break choice, missing spatial regions, duplicate IDs, receiptless actions, non-finite data and exhaustive-search bounds.

## Competitive benchmark

At matched total diagnostic cost, compare fixed periodic high-fidelity acquisition, sentinel-only operation, fixed thresholds, this capability-aware policy, budget-matched random scheduling, and a later robust EVSI/expected-information-gain policy.

The goal is not merely reducing high-resolution acquisition counts. Success requires matched or better event detection, diagnostic evidence coverage, observer-state qualification and abstention behavior. Report all misses and false triggers by event class; do not collapse them into one score.

## Prospective gates

Targets from #7097 are not achieved results:
- all hard-gate manufactured events trigger requalification or abstention;
- zero PIV-only force/thrust claims;
- zero acceptance of stale/replayed receipts;
- at least 300 independent benign dependence-cluster windows for a first false-trigger campaign;
- at least 20% lower high-fidelity acquisition cost versus periodic scheduling at matched event detection and coverage, or explicitly report failure to meet the target.

## Claim boundary

This is a deterministic manufactured scheduling policy, not globally optimal active sensing, a Bayesian utility estimator, a physical EHD operating controller, a passivity proof or experiment authorization.
