# Persistent Mission-Series Benchmark v0.1

**Status:** research protocol and synthetic contract fixture. No physical robot measurements or safety qualification are claimed.

## The paradigm shift

A policy can score well on isolated trials while accumulating recovery burden, operator dependence, battery depletion, downtime, maintenance, and hidden-state resets across an actual shift. Evaluate **mission series without invisible episode resets**, not only single episodes. The primary object is a time-ordered chain of missions where resources and system state carry over.

This benchmark complements the fault-response dynamics audit: that audit measures reaction to a fault event; this one measures the *longitudinal cost and continuity* of operating for a sequence of missions. It aligns with NIST's emergency-response robot performance programme, which identifies endurance, reliability, durability, autonomy, logistics and safety as distinct mission capabilities, and with 2026 assessments that argue deployed robot capability must be evaluated across reliability, speed, cost and transfer rather than by attractive demos alone:
- NIST: [Performance of Emergency Response Robots](https://www.nist.gov/programs-projects/performance-emergency-response-robots)
- Epoch AI: [Where Autonomy Works: Evaluating Robot Capabilities in 2026](https://epoch.ai/publications/where-autonomy-works-evaluating-robot-capabilities-in-2026)
- Long-horizon benchmark review: [A scoping review of reinforcement learning methods for long horizon robotic manipulation](https://link.springer.com/article/10.1007/s44430-026-00037-1)

## Experiment contract

A comparison group contains two time-aligned series: a conservative baseline and a candidate. They must share scenario family, seed, initial resource state, and the exact mission schedule. Each series contains at least four ordered missions with no simulator teleport or implicit episode reset. If there is a legitimate restart or maintenance reset, log the event, authorization, reset reason, downtime and resulting state epoch.

For each mission, record:
- success and useful work units, with the work-unit definition frozen before experiments;
- battery energy at start/end, energy consumed, and energy added by charging;
- elapsed time, recovery downtime, maintenance downtime, and operator interventions;
- safety violations and any reset, authorization, reason, and state epoch;
- prior/current opaque state tokens so the declared chain can be checked for continuity.

Opaque state tokens are linkage markers, not signatures or proof that the physical state is truthful. The receipt hashes the exact input bytes; physical telemetry needs its own trustworthy timestamping, calibration and provenance.

## Metrics, reported separately

The audit reports per-series:
1. Mission completion rate.
2. Useful-work units per Wh consumed.
3. Total useful work and total energy consumed.
4. Recovery minutes and maintenance minutes separately.
5. Operator interventions per series.
6. Authorized/unauthorized reset counts and safety violations.
7. End-of-series battery reserve fraction.
8. Linear slope of useful-work units across mission order as an early degradation indicator.

For each matched group, the audit reports candidate-minus-baseline differences for completion rate, useful work per Wh, recovery time, maintenance time, operator interventions, end reserve, and useful-work slope. These are diagnostics, not a blended score. Means can hide tail behavior; real studies should additionally publish per-mission distributions, median/p95 downtime, confidence intervals, and family-clustered uncertainty.

## Fail-closed conditions

The audit fails data integrity on:
- missing or duplicate series/mission identifiers, unpaired comparison groups, differing scenario/seed/schedule or initial conditions;
- broken prior-token chain, non-contiguous mission indices, unexplained state-epoch changes;
- inconsistent energy accounting, out-of-range battery state, non-finite/negative quantities, or energy measurements outside the declared tolerance.

The safety gate fails on any declared safety violation or any reset that lacks explicit authorization. An authorized reset is not invisible or free: it must increment the state epoch and its recovery/maintenance burden remains in the metrics. A real deployment study should also treat a missing episode or sensor log as missing evidence, never as a zero-cost success.

## Initial benchmark targets (hypotheses, not results)

- **Zero** safety violations and **zero** unauthorized resets on the qualification suite.
- **100% trace continuity:** no hidden episode restart, resource discontinuity, or unexplained state-epoch jump.
- Candidate improves useful work per Wh and/or completion rate on held-out scenario families without worsening hard safety gates.
- Cut total recovery + maintenance burden by **at least 20%** against a matched conservative baseline while keeping operator interventions and end reserve visible.
- Report work-degradation slope and p50/p95 recovery downtime across full shifts; do not optimize only the first mission.
- Demonstrate the direction of improvement on held-out families and then compare with simulator, hardware-in-loop and field evidence. Synthetic fixtures only test the audit contract.

Targets are prospective research goals, not standards limits. A percentage improvement from four synthetic groups is not statistical evidence and must not be presented as SOTA.

## Minimum evidence ladder

1. JSON/schema and mutation tests for accounting, continuity, reset authorization, and pairing.
2. Deterministic simulation with fixed mission order plus held-out terrain/task families and seeds.
3. Hardware-in-loop with synchronized energy, state and wall-clock instrumentation.
4. Low-consequence physical trials under a separately approved test plan, including stop criteria and operator rescue.
5. Replication on independent hardware, policies and operators.

Keep simulator and physical results separate. Publish exact policy/model revision, firmware, sensors, simulator versions, battery-health measurement method, reset policy, mission schedule, missing trials and raw receipts. Do not claim superiority over industry or SOTA until a same-task, same-platform, same-work-definition, same-evaluation-budget comparison supports it.

## Claim ceiling

The fixture in this change is hand-authored synthetic data. A passing audit establishes internal consistency of the supplied records and the evaluator only. It does **not** establish robot reliability, physical safety, battery-model validity, wear prediction, standards compliance, generalization or readiness for deployment.
