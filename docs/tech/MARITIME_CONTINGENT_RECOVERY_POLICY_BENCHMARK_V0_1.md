# Maritime Contingent Recovery Policy Benchmark v0.1

**Status:** research contract; acceptance thresholds are proposed, not measured.  
**Canonical tracker:** [MARITIME-REC-001 #1867](https://github.com/Luminous-Dynamics/symtropy/issues/1867).  
**Composes with:** [MARITIME-JIT-001 #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858) and [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671).

## 1. Paradigm shift: schedule → contingent plan

A static schedule assumes one future. A robust schedule protects against a declared set of futures, but can waste scarce capacity if it simply reserves the worst case. This benchmark investigates a middle ground: a **bounded contingent policy** containing a primary plan, a finite set of prequalified recovery options, explicit observation/event triggers, branch-specific constraints, and a declared rule for when to abstain.

For example, after an evidence-qualified berth delay, the policy may compare a compatible alternate berth window, a reserved energy slot, a retiming option, or a declared service miss. If those options share one scarce crane, charger, berth, battery pack, or onward connection, the policy must allocate that physical resource jointly. No option is free merely because it is written down.

Research in 2026 has examined cascading Asia–Europe container-network risks under Red Sea disruption and post-typhoon port-recovery optimization. The IMO/IAPH/IHMA Port Call Optimization Guide launched in March 2026 promotes harmonized electronic exchange of operational and nautical data, building on port-to-port trials. These sources motivate coordinated recovery; they do not establish expected gains for this implementation.

## 2. Ownership boundaries

| Concern | Owner |
|---|---|
| Vessel/performance alternatives and engineering admissibility | Symthaea, including [MARINE-FREIGHT-001 #7444](https://github.com/Luminous-Dynamics/symthaea/issues/7444) |
| Deterministic optimization, policy branches, capacities, scenario replay and completeness | Symtropy |
| Authoritative bookings/commitments, participant permissions, shipment events, custody and accepted outcomes | Mycelix [LOG-COMMONS-S0 #4901](https://github.com/Luminous-Dynamics/mycelix/issues/4901), [LOG-COMMONS-S1 #5044](https://github.com/Luminous-Dynamics/mycelix/issues/5044) |
| Read-only spatial/temporal display, provenance lineage and uncertainty | Sol Atlas [MARITIME-ATLAS-001 #115](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115) |

Compose with existing [JIT arrival/port-energy #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858), [historical disruption replay #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671), [forecast applicability #1650](https://github.com/Luminous-Dynamics/symtropy/issues/1650), [physical-resource allocation #1639](https://github.com/Luminous-Dynamics/symtropy/issues/1639), and [deterministic maritime scenarios #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612). This task must not create a second optimizer, simulator, logistics event model, evidence authority, or trust score.

## 3. Frozen finite model

Begin with small synthetic scenarios and bounded catalogues of already-declared alternatives. Each primary plan binds:
- frozen demand, route/resource snapshot, policy version and decision-time information cutoff;
- selected voyage/service profiles and their declared admissibility boundary;
- canonical physical resource identities, units, operation intervals and evidence validity;
- objective vector and deterministic tie-breaker;
- available contingency options, their capacity requirements, costs, expiry and required approvals.

Each recovery branch binds:
- trigger event/observation type and admissible evidence conditions;
- exactly what was knowable at branch time (no future-event leakage);
- a finite action/option set and the branch's full resource ledger;
- changes to served/unserved demand, delay, energy, cost, reserve use, and downstream obligations;
- conditions for selecting, rejecting, or abstaining from the branch;
- required owner authorization, without exercising any external side effect.

The policy is a finite decision tree or equivalent finite-state rule set. Declare branch-depth, event-count, candidate-count and compute budgets. Incomplete enumeration is visible and must not be represented as a globally optimal policy.

## 4. Hard constraints and objective vector

Hard constraints remain non-negotiable: qualified or explicitly synthetic candidate-envelope boundaries; interface compatibility; typed units/time bases; decision-time evidence validity; canonical physical-resource capacity including cross-edge aliases; no duplicate reserve consumption; and explicit accounting of served/unserved demand. Missing or contradictory information must not become an available resource or a zero delay.

Report each dimension separately:
- service fulfilled/unfulfilled by declared demand class;
- delay, missed windows, and scenario-wise tail/worst-case outcome;
- energy/fuel and cost within explicit system boundaries;
- reserve-option cost and consumption, idle capacity and option expiration;
- physical-resource conflicts and remaining feasible options;
- recovery time to a predeclared service threshold;
- plan churn / number of replanning changes;
- demand displaced by one participant/class to preserve another;
- candidates, branches, compute cost, and search completeness.

Do not invent probabilities for hand-authored scenarios. If scenario probabilities are not supported by evidence, report per-scenario and worst-case outcomes. A single composite “resilience score” is not a sufficient result. Report the cost of flexibility explicitly and show who bears any service displacement.

## 5. Frozen machine-readable corpus

The first frozen synthetic corpus is [`benchmarks/fixtures/MARITIME_CONTINGENT_RECOVERY_V0_1.json`](https://github.com/Luminous-Dynamics/symtropy/blob/research/maritime-contingent-recovery-v0-1/benchmarks/fixtures/MARITIME_CONTINGENT_RECOVERY_V0_1.json). It contains 12 named conformance cases covering alternate berth recovery, one reserve competing across demands, physical-resource aliasing, expired options, stale forecasts, contradictory evidence, duplicate/replayed triggers, changed payload under one idempotency key, no-disruption plan stability, search truncation, half-open intervals, and input permutation.

These examples are a versioned starter corpus, not yet an executable test result. The eventual independent harness must parse and validate the fixtures, add independently hand-checked complete tiny search spaces, and prove that every expected disposition is exercised. Any schema or expectation change must be versioned and reviewed with a changed fixture manifest/digest.

## 6. Baselines and independent oracle

Compare the same frozen cases against:
1. fixed primary plan with no recourse;
2. greedy replanning after every received update;
3. earliest-feasible local alternative;
4. bounded recourse policy under test;
5. an independent exhaustive enumerator on complete tiny instances.

The tiny-instance oracle must not reuse production branch-feasibility, resource-ledger, objective-aggregation or state-transition helpers. Freeze tie-breaking and the baseline implementations before the evaluation split.

## 7. Required hostile cases

- no disruption (recourse must not create needless plan churn);
- one berth delay with one feasible alternate interval;
- two concurrent disruptions competing for one reserve resource;
- alternate option expires before activation;
- distinct graph edge IDs alias the same physical crane/charger/berth/resource;
- one attractive branch violates a hard constraint;
- capacity evidence is revoked/expired/superseded after policy generation;
- recently downloaded but stale forecast or target outside valid interval;
- event arrives out of order and must not rewrite history;
- unknown/contradictory trigger evidence requires abstention;
- replayed trigger/idempotency key must not consume reserves twice;
- one priority class crowds out other demand contrary to the frozen allocation policy;
- branch-budget exhaustion must not claim complete/optimal policy search;
- input-order permutations yield the same canonical digest;
- noisy irrelevant updates trigger churn under the greedy baseline but not unbounded hidden state in the proposed policy.

Keep negative controls. A hash or valid signature proves integrity/issuer attribution, not physical truth or permission to execute a branch.

## 8. Proposed acceptance gates

These are targets, **not measured results**:
- 100% agreement with the exhaustive oracle on complete frozen tiny policy spaces for reachable branches, hard feasibility, resource ledger and objective vector.
- Zero hard-constraint violations and zero overcommitment of canonical physical resources.
- 100% correct abstention/indeterminate outcomes for the frozen stale, unknown, superseded and contradictory evidence cases.
- 100/100 deterministic replay and equivalent input permutations yield byte-identical reports for a frozen manifest.
- Zero double consumption of a reserve on repeated/replayed triggers.
- Every report separates base-plan cost from option cost, reports unserved/displaced demand, plan churn, scenario outcomes, and search completeness.
- Compare policies on matched scenarios; report service preserved, tail delay, recovery time, option cost, displacement, and churn. No one dimension may be silently substituted for another.
- Larger-scale tests report p50/p95 runtime, peak memory, branch count/depth, candidates examined, truncation and per-scenario outcomes. Set latency targets only after a pinned-host baseline exists.
- Historical-event performance remains gated by [#1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671); a narrative-only case is a qualitative stress test, not empirical recovery validation.

## 9. Authority and claim ceiling

This is a deterministic reference benchmark, not an operational policy engine. Plans and branches are proposals, not bookings, dispatches, navigation advice or physical commands. Mycelix's separate owner-authorized flow remains responsible for real resource commitments. No automatic outreach, inventory mutation, payment, custody mutation, dispatch or actuation is permitted in this phase.

Passing synthetic oracle checks proves only bounded policy correctness on the frozen model. It does not demonstrate real-world resilience, fairness, safety, emissions savings, seaworthiness, regulatory compliance or predictive validity.

## 10. Research grounding

- IMO/IAPH/IHMA, *Port Call Optimization Guide launched at IMO*, 26 March 2026: https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/
- IMO GreenVoyage2050, *Ship-Port Interface Measures Portal* (describes JIT arrival dependency on berth, fairway and nautical-service availability): https://greenvoyage2050.imo.org/ship-port-interface-measures-portal/
- Dai et al. (2026), *Cascading risks and resilience optimization of Asia–Europe container shipping under the Red Sea crisis*: https://doi.org/10.3389/fmars.2026.1961389
- *Enhancing port resilience to typhoon disasters: A framework for recovery optimization and shipping network impact assessment*, Transport Policy 180 (May 2026): https://doi.org/10.1016/j.tranpol.2026.104029

These publications motivate the question and scenario families; no effect size or conclusion is presumed transferable to this benchmark.
