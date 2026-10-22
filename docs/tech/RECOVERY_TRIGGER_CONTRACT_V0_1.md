---
title: Versioned Signposts and Adaptive-Recovery Triggers
version: 0.1
status: research-design
date: 2026-10-17
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1896
---

# Versioned Signposts and Adaptive-Recovery Triggers v0.1

## Purpose

RDO-003 evaluates a policy once a declared observation class is available. RDO-004 specifies **when to re-evaluate that policy**, when evidence is too stale or incomplete to support a trigger decision, when adaptation is already too late for the planned response window, and when changed criteria invalidate the old decision regime.

A trigger is not an operation. A trigger requests review, inspection, reassessment, or replanning; it never grants permission to cut, lift, reuse, repair, isolate, close infrastructure, or perform other physical work.

## Paradigm shifts

### From prediction to signposts and pre-planned contingency

A point prediction of the future can be wrong while a decision remains robust if a monitored signpost detects the conditions under which the plan should change. Each trigger therefore binds a measurable signpost to a versioned predicate, a response lead time and a predeclared review pathway.

### From one threshold to stateful boundary semantics

For domains where a valid physical or operational profile supports hysteresis:
- an inactive trigger enters when the declared enter predicate passes;
- an active trigger remains active until the distinct exit predicate passes;
- equality behavior is explicitly specified;
- state-transition receipts preserve the prior state, observation and exact profile version.

This prevents trivial noise near one threshold from causing unexplained flip-flopping. Hysteresis must not be added reflexively: a real hazard cannot be delayed by debounce merely to make a dashboard stable. For fast-moving or safety-critical hazards, the domain profile may require immediate review without smoothing or persistence windows.

### From static objective weights to regime-aware decisions

The criteria used to evaluate a policy can change. If current decision regime version differs from the trigger profile's criteria version, do not apply old thresholds and issue a seemingly current result. Reassess under the new regime, issue a new decision receipt, and preserve the old result as prior evidence. Criteria changes may include safety standards, intended use, ecological priorities, asset requirements or accepted service levels—not only numeric objective weights.

### From detecting eventually to detecting in time

A trigger may be logically correct but operationally too late. Compare the declared remaining time to the relevant decision boundary with the response lead time. If less time remains than the minimum time needed to respond, report `TRIGGER_LATE_REASSESSMENT_REQUIRED`. Do not pretend that firing the trigger restored the lost option.

## V0.1 evaluation order

The bundled `recovery-trigger-corpus-v0.1.json` freezes 12 synthetic boundary fixtures across forest weather/operation review, PV degradation review and infrastructure crack-growth review. The evaluation order is intentional:

1. Compare current decision-regime version with the trigger profile's criteria version. A mismatch requires reassessment.
2. Require observation units to match the exact profile; never infer a conversion from a similarly named unit.
3. Require valid freshness and sufficient coverage. The reference boundary is stale only when age is strictly greater than the maximum; coverage is sufficient at exactly the minimum.
4. Apply the previous-state-specific enter or exit predicate using explicit comparison semantics.
5. If active, compare remaining time with the required response lead time; an active trigger that is already late requires reassessment.
6. Return a review-only disposition plus structured reason code.

This fixture specification does not yet define a production controller, measurement adapter, verified clock, field-calibrated threshold, or domain permit. All values are invented boundary-test values.

## Fixture expectations

- Entry equality: value exactly equal to an inclusive `gte` enter threshold triggers review.
- Just below entry: does not trigger.
- Hysteresis band: an active trigger stays active until the exit predicate is satisfied.
- Exit equality: when the exit operator is strict `lt`, a value equal to the exit threshold has not cleared the trigger.
- Clear state: a value below the exit threshold requests review of the state transition; it does not auto-resume operations.
- Stale measurement: hold for fresh evidence, even when the old value would have triggered.
- Insufficient coverage: hold; do not infer that unobserved zones passed.
- Criteria-regime mismatch: re-evaluate the profile before applying old thresholds.
- Unit mismatch: hold and resolve the measurement/profile incompatibility.
- Late trigger: require urgent reassessment; no claim that response can still complete before the boundary.
- PV and infrastructure profiles: exercise the same protocol with separate units and profile identities.

Every case explicitly sets `expected_actuation=false` and `NO_ACTUATION_AUTHORIZED`.

## Benchmark targets

The initial goals are logical contracts, not field-accuracy claims:

- **0 unauthorized downstream actions** from trigger evaluation alone.
- **100% exact boundary correctness** for equality, just-below/above, state entry/exit, freshness, coverage and regime-mismatch fixtures.
- **100% lineage completeness** from source observation through profile version, predicate outcome, prior state and review disposition.
- **100% profile/units rejection** on incompatible measurements; no implicit unit conversion.
- **100% regime-version enforcement**: any criteria-version mismatch forces reassessment.
- **0 stale / insufficient-coverage clearances**.
- Report late-detection fraction, false-trigger rate, missed-trigger rate and option-loss consequences only against representative, labeled and held-out domain data. The synthetic corpus tests the logic branches only.
- Measure how much lead time remains at trigger, not just trigger-classification accuracy.
- Do not use debounce or hysteresis without profile-specific justification and a test showing that genuine hazards are not delayed.

After the predicate contract is stable, the next engineering slice should add a standalone deterministic Rust reference evaluator, a separately coded oracle, exact-head receipts and tests for duplicate/out-of-order evidence, invalid age/coverage, equality boundaries, hysteresis transitions, and criteria-regime changes.

## Research grounding

- Nakashima et al., *Dynamic Adaptive Pathways Planning with Changing Criteria for Cross-Sectoral Climate Adaptation* (2026, DOI 10.1016/j.procs.2026.08.180) examines changing evaluation criteria as well as changing conditions; its findings motivate versioning the decision regime, not transferring its numeric values: https://doi.org/10.1016/j.procs.2026.08.180
- *Robust adaptive pathways for long-term flood control in delta cities: addressing pluvial flood risks under future deep uncertainty* (2025) explicitly uses observable signposts and triggers for transitions: https://hess.copernicus.org/articles/29/6647/2025/
- Raso, Kwakkel and Timmermans, *Assessing the Capacity of Adaptive Policy Pathways to Adapt on Time by Mapping Trigger Values to Their Outcomes* (2019) examines trigger classification versus consequences of early/late adaptation: https://www.mdpi.com/2071-1050/11/6/1716
- Chen et al., *Cost-effective and reliable power system transitions under deep uncertainty* (2026, DOI 10.1016/j.ijepes.2026.111766) distinguishes feasibility-related uncertainty from outcome uncertainty and compares multi-scenario robust strategies: https://doi.org/10.1016/j.ijepes.2026.111766

These sources establish motivation, not forestry, PV or infrastructure trigger thresholds for this repository.

## Related work

- [RDO-003 / PR #1895](https://github.com/Luminous-Dynamics/symtropy/pull/1895): observation-constrained adaptive policy regret.
- [RDO-002 / PR #1876](https://github.com/Luminous-Dynamics/symtropy/pull/1876): interval-robust Pareto frontier.
- [RECOVERY-ORACLE-001 / PR #1851](https://github.com/Luminous-Dynamics/symtropy/pull/1851): evidence envelope, gate semantics and receipts.

## Non-goals

- no autonomous actuation;
- no empirical thresholds from synthetic fixtures;
- no assumption that the criteria regime cannot change;
- no blanket debounce/hysteresis rule across hazard types;
- no claim that trigger detection alone proves an operation remains feasible;
- no fabricated probability of future transition.
