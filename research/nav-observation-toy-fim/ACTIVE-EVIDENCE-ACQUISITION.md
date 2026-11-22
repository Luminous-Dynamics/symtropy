# Active evidence acquisition: resource-bounded Pareto planner

**Status:** deterministic policy-selection kernel and synthetic tests added; no real sensor action model, energy model or physical acquisition result is qualified.

**Tracker:** [issue #6853](https://github.com/Luminous-Dynamics/symtropy/issues/6853).

## 1. Why the policy is different from "sample more"

When a navigation service becomes DEGRADED, the limiting evidence may be stale clock synchronization, NLOS uncertainty, poor anchor geometry, a shared environmental failure domain, or missing calibration support. Repeating the same measurement can add data without resolving the dominant integrity deficit.

An active navigation service should request the kind of evidence that can reduce the current protection-level upper bound while respecting mission latency, energy and dependency constraints.

## 2. Inputs are conditional predictions, not acquired evidence

Each action proposal carries:

- action identity and type (resynchronize, CIR capture, new anchor, passive observer, viewpoint change or VIO relocalization);
- declared dependency/failure-domain group;
- conditional post-action protection-level upper bound;
- worst-case acquisition latency and energy;
- availability and freshness/calibration precondition flags.

The post-action protection level is conditional on the action succeeding and the resulting observation being independently qualified. The scheduler does not assume the action succeeded.

## 3. Pareto decision, not arbitrary scalar utility

The current kernel first filters candidates by:
- availability and freshness/calibration preconditions;
- resource budgets;
- whether the action supplies a new dependency group when required;
- a strictly smaller conditional protection-level upper bound.

It then computes the Pareto frontier over:
1. conservative protection-level reduction;
2. worst-case latency;
3. worst-case energy.

No arbitrary unit conversion or hidden energy/time weights are introduced.

Recommendation rule:
- If a Pareto action can conditionally reach the target, choose the lowest-latency target-reaching action, then lowest energy, then stable action ID.
- Otherwise choose the largest conservative reduction, then latency, energy and stable ID. The result explicitly records that the target remains unmet.

The lexicographic tie-break is a declared research policy, not a universal optimum. Missions with different deadlines or energy priorities should use separately versioned policies.

## 4. The planner never grants navigation authority

The plan contains an explicit `authority_transition_not_authorized_by_plan` flag. It is always true.

Even when an action is recommended, the controller must:
1. execute the action;
2. obtain a new observation;
3. qualify provenance, clock, calibration, security and physical measurement integrity;
4. run the independent posterior audit;
5. re-evaluate protection levels and the mission contract;
6. only then consider NORMAL recovery.

If no action satisfies the resource and improvement constraints, the planner fails closed with `NoFeasibleAction`; it does not manufacture an improvement estimate.

## 5. Tests

Eight deterministic synthetic tests cover:
- lowest-latency selection among target-reaching Pareto actions;
- latency and energy budget filtering;
- mandatory new dependency-group requirement;
- selection of the best partial reduction when no action reaches target;
- removal of Pareto-dominated actions;
- already-satisfied target handling without authority grant;
- rejection of stale, unavailable and non-reducing actions;
- invalid and duplicate action identities.

These tests validate policy arithmetic and selection logic only. They do not validate action success models, real sensor energy, mission risk reduction or physical availability.

## 6. Integration and benchmarks

Connect action proposals to qualified sensor-specific models:
- synchronization holdover / clock requalification;
- CIR-acquisition rate and NLOS-set reduction;
- anchor/viewpoint geometry and the independent posterior auditor;
- passive observers with explicit common-cause dependency labels;
- VIO relocalization and independently sourced pose evidence.

Compare the Pareto policy against periodic acquisition, fixed action priority, largest-uncertainty-first and random policies. Measure time in DEGRADED, conditional and realized PL reduction, successful qualified recovery rate, false recovery, HMI, p99 acquisition latency, energy per successful recovery and resource Pareto frontiers.

## 7. Claim boundary

The module recommends an action using caller-supplied conditional bounds. It is not a learned value-of-information model, does not estimate action success probability, does not change protection levels itself and does not authorize movement. All real post-action bounds, resource costs and dependency labels require independent qualification.
