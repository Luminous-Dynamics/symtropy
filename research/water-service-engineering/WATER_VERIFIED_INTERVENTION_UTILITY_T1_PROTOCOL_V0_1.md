# Water verified-intervention utility T1 protocol v0.1

**Tracking issue:** #5951 WATER-VERIFIED-INTERVENTION-UTILITY-001  
**Parents:** #4963 leak diagnostics, #5047 observability qualification, #5893 survival frontier and #5018 controller robustness  
**Evidence class:** synthetic-first known-answer contract; no field action is authorized.

## 1. Paradigm: benchmark the verified intervention, not the alert

Leak detection, localization, inspection, confirmation, repair and verified savings are different events. An alert is not a confirmed leak. A pipe guess is not a safe inspection scope. A work order is not proof of repair. A post-repair flow reduction is not verified savings unless operating conditions and independent metrology are comparable.

The benchmark therefore evaluates the complete evidence chain:

`event → detection → observability → bounded localization → safe decision → inspection → confirmation → repair → independent verification → persistent service benefit`

An action may be `inspect`, `prioritize`, or `abstain`. A pipe claim is considered issued only when a leak was detected and the action is not `abstain`. An abstained event is not counted as a true positive, correct localization, inspection, repair or verified intervention. A system must not issue pipe-level certainty when the state is unobservable, indeterminate, topology-invalid, or its estimator is outside the declared scope.

## 2. Localization eligibility

Pipe/within-pipe claims require one of:
- current direct observation with a valid topology; or
- an independently qualified estimator whose declared scope matches the current topology and sensor set.

Structural-only observability may support a broader inspection region, but it does not establish a direct physical observation. Unobservable, indeterminate and topology-invalid dispositions prohibit pipe-level and within-pipe claims. Candidate sets with more than one asset are not counted as correct pipe localization even when the true asset happens to be in the set.

`abstain` is a legitimate safe action, but it is not a true positive, correct localization, completed inspection or verified intervention.

## 3. Inspection and repair evidence

An inspection result is one of `confirmed_leak`, `no_leak`, `inconclusive`, or `not_attempted`. Crew/travel time, cost, confirmed asset IDs and confirmation time are bound to the event. False dispatches are inspections that return `no_leak` on a non-leak event; inconclusive inspections are reported separately.

A repair is a separate record with an intervention ID, asset ID, repair time, cost and outage minutes. Repair credit requires a confirmed leak and exact asset agreement between inspection and repair. An asserted repair without matching confirmation is rejected.

## 4. Independent post-repair verification

Pre-repair baseline and post-repair readings must bind to instrument identity, calibration, method revision, raw-record digest, physical boundary and time window. The post-repair observation must be compared under matched:
- demand (within the declared relative tolerance);
- pressure (within the declared relative tolerance);
- pump/valve operating state;
- topology revision;
- meter, unit, method and calibration scope.

Replaying a raw record is invalid, including reuse of one post-repair digest across multiple follow-up days. The pre-repair baseline window must end before the repair timestamp and every credited post-repair window must start after it. The baseline and follow-up windows must not overlap. A simulator counterfactual cannot replace a physical observation.

Per follow-up day, point reduction is `pre_repair_leak_flow - post_repair_leak_flow`. The conservative lower bound is:

`max(0, point_reduction - k * (u_pre + u_post))`

v0.1 uses `k=2` as an uncertainty expansion factor only; it is not a 95% confidence interval or population-level inference. Keep 7-, 30- and 90-day follow-up results separate. Missing or incomparable follow-up is `indeterminate`, not a zero-loss repair.

## 5. Comparator arms

Future matched submissions must evaluate the same held-out event IDs, topology, sensor budget, alarm budget, observation window and crew-hour budget for:

1. Operator rule/threshold process.
2. Classical minimum-night-flow / district balance / pressure-management process, where applicable.
3. Hydraulic-model residual baseline.
4. Best matched graph/transformer benchmark baseline.
5. Luminous detection + observability + abstention + inspection prioritization + repair verification.

The T1 fixture identifies a submission arm explicitly but does not claim that the synthetic fixture is a fair industry comparison. A real tournament requires utility-approved common data and a preregistered intervention protocol.

## 6. Metrics

Report separately:
- event-level TP, FP, FN, recall and detection delay;
- abstentions, unobservable-event coverage and false-safe pipe claims;
- zone/pipe/within-pipe localization only at ground-truth-supported resolution;
- inspections, confirmed leaks, no-leak false dispatches and inconclusive inspections;
- crew hours, crew-days, cost per confirmed leak and confirmed leaks per crew-day;
- time to confirmation and time to repair;
- repaired assets and actual outage minutes;
- independently verified lower-bound reduction at 7, 30 and 90 days;
- cost per verified cubic metre saved, only where the lower-bound saved volume is positive.

Do not combine these into one opaque accuracy or utility score. The operator must be able to see the trade-off among safe abstention, false dispatches, confirmed interventions, service disruption and verified saved water.

## 7. Frozen 12-scenario challenge

1. Observable leak, correct pipe, confirmed inspection, repaired, persistent 7/30/90-day reduction.
2. Common-cause failure makes the state unobservable; pipe-level output must be flagged false-safe.
3. Non-leak pressure change with a false alert and a no-leak inspection.
4. Leak missed by the detector.
5. Structural-only observability with multiple candidate pipes; pipe certainty is ineligible.
6. Estimator scope mismatch; pipe certainty is ineligible.
7. Confirmed leak and repair with only a 7-day follow-up; no 30/90-day claim.
8. Apparent post-repair reduction under unmatched pressure; verification indeterminate.
9. Replayed raw pre/post measurement digest; verification invalid.
10. Repair asset differs from confirmed inspection asset; no repair credit.
11. Zone-level localization with a confirmed leak but no independent post-repair record.
12. Prioritized but not inspected event; no confirmation, repair or saved-volume credit.

Additional adversarial unit tests also mutate a detected event to `abstain`, move a follow-up window before repair, and replay one post-repair digest across follow-up days; all must remain uncredited.

## 8. Stretch targets — proposed, not achieved

- At least 25% fewer false dispatches at matched confirmed-leak recall than the strongest local baseline.
- At least 25% lower P95 time-to-confirmation without worsening service-floor violations.
- At least 20% more independently confirmed leaks per crew-day at the same crew-hour budget.
- Zero false-safe pipe-level recommendations in the declared unobservable/topology-invalid challenge set.
- Positive lower-bound post-repair reduction at the declared follow-up horizon, with a preregistered uncertainty method.

These are research targets, not standards or results. They require matched baselines, independent utility data, and field verification.

## 9. Claim ceiling

The T1 evaluator is synthetic-only. `physical_qualification_status`, `statistical_inference_status` and `claim_eligibility` remain `NotRun` / `NotEligible`. A pass does not prove utility performance, safe field operation, leak detection SOTA, actual repair, NRW reduction, water savings or regulatory compliance. No work-order dispatch, SCADA write, crew dispatch or actuator interface is added.