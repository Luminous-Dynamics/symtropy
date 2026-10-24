# Causal dispatch and clairvoyance-gap benchmark v0.1

**Status:** research protocol / next implementation slice.  
**Tracked by:** [ENERGY-NONANTICIPATIVE-001 #2772](https://github.com/Luminous-Dynamics/symtropy/issues/2772).  
**Applies to:** fixed Symtropy candidate architectures, scenario profiles, Sol-Atlas dispatch, independent feasibility audits, and controller-policy evaluation.  
**Claim ceiling:** synthetic finite-model protocol until causal-policy traces and a calibrated site study are independently qualified.

## The missing distinction

The current candidate × scenario matrix evaluates each declared scenario separately. That is useful for robust architecture screening, but a scenario-specific optimizer may choose a dispatch schedule with knowledge of the entire future scenario. Such a schedule is a *wait-and-see oracle*, not necessarily a policy a real controller can execute.

The next benchmark therefore has three separate objects:

1. **Architecture:** fixed design/capacity decisions shared across all futures.
2. **Causal policy:** a rule mapping currently observable state/history and explicitly available forecasts to actions. It may adapt only when information becomes available.
3. **Wait-and-see oracle:** a separate diagnostic that may know the complete scenario before dispatch. It bounds what perfect foresight could achieve but is never counted as deployable-policy performance.

Do not conflate architecture robustness, policy feasibility, and perfect-foresight performance.

## Information sets and non-anticipativity

Each decision trace step must bind at least:

- candidate / architecture digest;
- scenario and time-step identifiers;
- observed-state digest;
- digest and availability time of each forecast consumed by the policy;
- an information-set identity derived by the verifier from the observable history prefix;
- selected action vector and its units;
- resulting-state digest and independent dispatch/audit receipt.

For branches whose observable histories are identical at decision time (t), the policy must emit the same action vector at (t). Formally, if two scenario traces have equivalent observable prefixes and identical declared decision context through (t), then their actions at (t) must match. Scenario labels and future mutations are evaluator metadata; they must never be exposed to the policy before the relevant event is observable.

After histories diverge because of a declared, observable event, actions may diverge. This is valid recourse, not a violation.

Information-set IDs must not be trusted merely because a trace supplied them. Derive them from canonicalized observable-prefix bytes plus the policy-visible forecast context. Reject both:
- two unequal observable prefixes placed in the same information set; and
- equivalent observable prefixes assigned conflicting information sets.

Unknown observation fields, forecast look-ahead, missing trace steps, duplicate time steps, incomplete time coverage, inconsistent architecture digests, missing actions, ambiguous units, and receipt/hash mismatches must fail closed or be reported UNKNOWN. Never silently drop them.

## Minimal counterexample fixture

The first known-answer fixture should contain two futures that are observationally identical before the event but prefer different actions under perfect foresight:

- A fixed battery architecture begins with one usable energy quantum.
- At decision time (t=0), the current observations and available forecasts are byte-identical across both branches.
- In branch A, the next interval contains an outage and essential demand; preserving the quantum is the service-protecting action.
- In branch B, no outage occurs in that interval and a declared flexible demand can be served with the quantum; spending it now improves the flexible-service objective.
- The future branch is revealed only after the (t=0) action is committed.

Expected outcomes:
- A wait-and-see oracle may choose different (t=0) actions for branches A and B; preserve its objective vectors but label them **non-causal reference only**.
- A purported single causal policy that chooses different (t=0) actions for A and B must be rejected.
- A causal policy that selects the same (t=0) action in both branches may pass non-anticipativity, but still must pass the physical feasibility auditor.
- Once the branch is actually observable, different subsequent actions are allowed.

The fixture must freeze the exact requests, canonical observable prefixes, action vectors, state transitions, objective vectors, causal-policy frontier, wait-and-see frontier, and expected gate verdicts. The non-anticipativity gate checks information consistency; it does not itself prove energy balance or feasibility.

## Pipeline and evidence contract

1. Freeze the tiny scenario-tree fixture and schema; hash the exact bytes.
2. Evaluate each causal policy using the same declared architecture, Sol-Atlas compiler/dispatcher, objective definitions, and independent feasibility auditor.
3. Run an independent non-anticipativity gate over the complete decision traces.
4. Run negative controls: future-branch leakage; a forecast timestamped after the action; forged information-set IDs; omitted/duplicated steps; inconsistent architecture digest; modified trace bytes with a stale receipt; and a policy that is causal but physically infeasible.
5. Preserve raw traces, per-step actions/states, per-scenario objective vectors, all failure/UNKNOWN rows, source hashes, verifier version, and SHA-256 receipts.
6. Add the gate to CI only after it produces frozen known answers and the independent implementation detects every negative control. A green report without these bytes is not evidence.

Never weaken existing dispatch or ledger gates to accommodate this layer.

## Metrics and preregistered acceptance targets

These are targets, not achieved results.

- **Causal consistency:** zero non-anticipativity violations on qualified policy traces.
- **Feasibility:** zero false-feasible outcomes; causal consistency and physical feasibility must pass independently.
- **Mutation detection:** 100% of registered future-leakage, information-set, incomplete-trace, and receipt-tampering controls detected.
- **Oracle recovery:** exact causal-policy and wait-and-see frontier recovery on enumerable fixtures, or UNKNOWN if the independent reference reaches its declared bound.
- **Coverage:** 100% of registered architecture × scenario × time-step entries accounted for, including explicit FAIL/UNKNOWN status.
- **Service first:** worst-case essential unserved quanta, longest continuous essential deficit, reserve shortfall, and outage-interval service are reported before cost/emissions comparisons.
- **Trade-offs:** preserve full objective vectors and per-scenario traces. Report coordinate-wise gaps and Pareto relations without post-hoc scalar weights.
- **Clairvoyance gap:** report per-objective difference between causal-policy results and the wait-and-see reference on the same finite domain. Clearly state objective direction and whether the gap is coordinate-wise / potentially non-attainable as a single vector.
- **Performance:** separately measure policy-generation overhead, unique policy proposals, architecture evaluations, exact dispatch evaluations, audit evaluations, cache hits, wall time, and peak memory under identical runner/toolchain conditions.
- **Generalization:** separate design/tuning, validation, and locked held-out scenario families. A held-out family may not be used to tune the policy or scenario generator.

Do not calculate expected cost, EVPI/VSS, or CVaR from the current deterministic stress set. Those require justified probabilities, information timing, and a properly specified scenario model.

## Industry comparison: scope-aligned, not slogan-driven

- **REopt API v3** is a relevant facility-scale techno-economic sizing/resilience reference. Match tariff, load, resource, equipment, horizon, outage, and resilience semantics before comparing outcomes.
- **SAM** is a useful detailed PV/battery performance and degradation reference. Its battery models can account for calendar/cycle degradation and replacement effects; the current simplified model cannot claim lifecycle superiority without a matched comparison.
- **PyPSA stochastic optimization** explicitly separates scenario-independent investment decisions from scenario-indexed operations and explains non-anticipativity. It is the most direct conceptual reference for shared architecture and scenario recourse, but its optimization model is not automatically a drop-in comparator for a small integer dispatch fixture.

Sources:
- NLR REopt API v3: https://developer.nlr.gov/docs/energy-optimization/reopt/v3/
- NREL SAM battery storage: https://sam.nrel.gov/index.php?Itemid=360&id=94&option=com_content&view=article
- PyPSA stochastic optimization: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/

A credible SOTA claim needs a calibrated site study with rights-cleared interval meter/load data, account-specific tariff and fixed charges, weather/resource inputs, outage records, equipment efficiencies and degradation, common model boundaries, and independent rerunnable comparison. Report where models are not equivalent instead of manufacturing a score.

## Implementation order

1. Define canonical trace schema and exact information-set derivation.
2. Freeze two-branch known-answer fixture and hostile controls.
3. Implement the independent non-anticipativity verifier and test against a deliberately clairvoyant trace.
4. Evaluate passing causal policies with the existing Sol-Atlas dispatcher and feasibility auditor.
5. Add causal-vs-wait-and-see objective-vector reporting with exact source digests.
6. Integrate into the end-to-end workflow while retaining all current fail-closed checks.
7. Add a larger-domain adaptive policy search only after the gate is qualified; evaluate on locked held-out scenario families.

**Current status:** protocol only. No implementation, compiled test, passing CI run, site result, or industry outperformance is claimed by this file.
