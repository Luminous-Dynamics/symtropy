# Common-cause recovery and capability lifecycle regret protocol v0.1

Companion to [MCSC-RESILIENCE-REGRET-001 #7747](https://github.com/Luminous-Dynamics/symtropy/issues/7747).

## 1. Scope

This benchmark measures how a maritime productive-capability portfolio maintains and restores **receiver-accepted service** under correlated disruptions, capability expiry, changed configuration, authority revocation, partial repair and delayed requalification.

It composes:
- Mycelix #5762 for multi-valued/transitive common-cause dependency identity and cut-set completeness;
- Mycelix #6533 for immutable resource offers, time-bounded calendar and reservation claims;
- Symtropy #5710 for configuration-bound qualification invalidation and minimum requalification;
- Symtropy #7554/#7557 for receiver-level outcomes and protected service floors;
- Mycelix #6343 for shared port-city service-equivalence and PCC evidence.

It does not implement a second dependency graph, resource registry, qualification graph, receiver evaluator, PCC verifier or energy optimizer.

## 2. Paradigm shift

The benchmark's unit of value is not nameplate capacity or provider count. It is a **qualified option to deliver an accepted service quantity during a specified execution window, under a declared dependency and authority envelope**.

Two offers do not establish redundancy when they share a claim-critical upstream dependency. A machine does not contribute current capacity when its calibration, operator competence, offer validity, reservation or execution authority will expire before the required work finishes.

Resilience is the ability to preserve a protected minimum service and restore full service under the same demand and evidence cut, with measured time and resources required for recovery.

## 3. Decision-regret vector

For each scenario ω and policy π, preserve a service-and-recovery vector:

- expected, inspection-qualified, receiver-accepted, on-time and unresolved units;
- service deficit in unit-hours and time below protected floor;
- time to minimum viable service and time to full service;
- accepted-but-late units and cancellation/substitution;
- requalification actions, elapsed time and cost;
- repair, standby, substitution, renewal and whole-life cost;
- energy, emissions, material loss and water/heat externalities;
- common-cause cut-set coverage and false-independence admission;
- plan change, evidence freshness and authority state.

Compare candidate policy outcomes with a clairvoyant per-scenario oracle (upper bound only), a non-clairvoyant robust policy, a no-build/current-portfolio baseline and a deliberately naive independence-assumption control. Report regret as a vector. Do not collapse it to one number unless an authorized decision-maker supplies a transparent preference model after all hard constraints pass.

No failure probability is assigned to v0.1. Scenario frequencies, expected loss and risk-adjusted ROI require source-grounded incident/repair/availability data and a declared sampling model.

## 3A. Executable regret comparator

The MCSC Rust reference now exposes `compare_recovery_regret(candidate, reference)`. It accepts two outcome records only when the frozen instance identity/version, scenario ID, service ID, deadline, horizon and expected-unit denominator are identical.

For a finite scenario family, `worst_case_recovery_regret(&[...])` constructs one adverse bound per dimension. It uses the larger candidate-minus-reference delta for loss, lateness, deficit hours, recovery time, cost, emissions and requalification burden; it uses the smaller delta for inspection-qualified and receiver-accepted quantities. Each bound names its worst scenario. The reducer rejects duplicate scenario IDs and any mismatch in frozen instance/version, service, deadline, horizon, denominator or candidate/reference policy. It assigns no scenario weights.

It computes candidate-minus-reference deltas separately for:
- unmet expected units at the deadline;
- inspection-qualified units;
- receiver-accepted units;
- late accepted units;
- unresolved units;
- service deficit unit-hours;
- time to minimum viable service and full service;
- recovery cost and emissions;
- requalification action count.

Positive deltas are worse for the candidate. This vector is not scalarized. The reference may be a clairvoyant per-scenario admissible upper bound, but is not an implementable policy and has no probability weight. Validation rejects receiver acceptance above inspection-qualified quantity, inconsistent accepted/on-time/unresolved quantities and impossible recovery times. Unit tests cover an eleven-dimensional known answer, context mismatches, invalid inputs and zero-regret equality.

This API is only a comparison primitive. The 20-case corpus currently validates scenario coverage and claim ceilings; no full recovery optimizer, scenario probability model or empirical regret result is claimed.

## 4. Shared cause and multi-stage recovery semantics

Use owner-native identities and transitive closure. Preserve exact common dependency IDs; a set digest cannot replace the identity members. Distinguish:
- shared root cause from coincident independent events;
- claim-critical dependency from evidence lineage, optional and fallback paths;
- known dependencies from unknown remainder;
- resource physically present from qualified, calendar-available, reserved and authorized;
- evidence invalidation from physical failure;
- recovery initiated from capacity restored;
- partial service from full service.

When dependency completeness is unknown, return INDETERMINATE and withhold numeric cut-set or resilience claims. Do not assume a provider graph is independent because provider IDs or site names differ.

Partial restoration keeps the original expected-service denominator. Report 60% service as 60%, not as a new 100% denominator. A second event during recovery starts from the actual residual state, not from the original baseline.

Historical replay uses only evidence first available at the original decision cutoff. A subsequently discovered revocation changes current admissibility but does not rewrite what was knowable at the earlier time.

## 5. Policy comparison

At minimum, reproduce:
1. deterministic/no-build current portfolio;
2. nominal optimizer with independent-failure assumptions;
3. common-cause-aware portfolio using #5762 closure;
4. lifecycle-aware portfolio using #6533 and #5710;
5. finite-scenario robust/adversarial policy with no invented probabilities;
6. clairvoyant per-scenario admissible oracle as an information upper bound.

All policies share exact service IDs, expected-unit denominators, deadlines, quality constraints, time windows and decision-time evidence. A policy cannot improve by dropping demand, weakening quality, changing the denominator, using future forecasts or treating an offer as reserved capacity.

## 6. Frozen v0.1 corpus

The companion `common-cause-recovery-regret-v0.tsv` contains 20 synthetic scenarios spanning feeder/substation, shared supplier/material, NDT/calibration, workforce, rail, fibre, cooling/water, heat sink, configuration/firmware, calibration/skill expiry, authority revocation, late qualification, partial recovery, sequential events, late evidence and unknown dependency inventory.

These cases define expected dispositions and claim ceilings, not probabilities or real failure rates.

## 7. Proposed qualification targets

- 100% exact common-dependency identity preservation against #5762 on tiny tractable graphs.
- Zero false-independence credit when the frozen graph contains a shared claim-critical dependency.
- Zero stale/expired/revoked/wrong-configuration capability admission.
- Zero protected service denominator deletion across partial restoration and censoring.
- 100% deterministic replay for an identical owner receipt, scenario ID, model version and knowledge cutoff.
- Report false invalidation and false retention separately for requalification.
- Report worst-case and scenario-specific vector regret; no distributional risk claim without sourced probabilities.
- Held-out facility, supplier, workforce, utility and corridor families before generalization.

These are targets, not achieved results.

## 8. Claim ceiling and next evidence

A passing synthetic corpus qualifies only the declared reference rules. It does not establish physical independence, actual spare capacity, real production recovery, port uptime or causal regional benefit.

The next integration should ingest #5762's exact owner dependency closure and #5710's invalidation/requalification receipt by digest and freshness; then feed admissible service options into #7554. No second schema or estimator should be introduced where the canonical owner already supplies the contract.
