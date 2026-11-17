# WDN verified intervention yield T0 protocol v0.1

**Tracking issue:** #5969 WDN-VERIFIED-INTERVENTION-YIELD-001  
**Parents:** #4963 leak diagnostics, #5018 control robustness  
**Status:** synthetic-only software contract.

## 1. Paradigm

Move the benchmark endpoint from a model alert to an independently verified intervention outcome. Distinguish: alert; independent inspection; confirmed asset/fault; repair completion; independent post-repair verification; matched-condition hydraulic change; follow-up recurrence; service and crew burden.
An alert is never ground truth. A closed work order is never proof of a successful repair. An observed pre/post difference is not causal attribution unless its counterfactual assumptions are independently qualified.

## 2. Independent evidence chain

- Alert binds event, topology revision, DMA and candidate asset.
- Inspection record binds event and the physically confirmed asset; reviewer identity and confirmation evidence are required.
- Repair record binds the same event and confirmed asset; `completed` is distinct from `authorized_not_completed` and `partial`.
- Post-repair verification must be independent of the repair crew and bound to the same event/asset.
- Pre/post hydraulic measurements bind to meter, calibration, method, boundary, units, pressure, demand-profile digest, time windows and raw digests.
- Duplicate raw-record replay across events invalidates all affected events.

## 3. Matched-condition comparison

A quantitative observed change is emitted only when pre/post use the same meter/calibration/boundary/demand profile, valid non-overlapping windows, repair time between windows, pressure difference inside the pre-registered tolerance, and no confounding valve/pump/source/demand/adjacent-repair interventions. Otherwise the result is indeterminate.

For flow-rate values Q_pre and Q_post, point observed change is `Q_pre - Q_post`. Conservative lower change is `max(0, Q_pre - Q_post - k*(u_pre + u_post))`. v0.1 uses k=2 only as a declared expansion factor, not a confidence interval or probability.

Do not annualize a short change. Label it observed matched-condition flow reduction. Causal attribution and persistence are outside T0.

## 4. Frozen 14-case challenge

1. Positive independently confirmed and repaired leak.
2. Confirmed no-leak false dispatch.
3. Alert candidate asset differs from confirmed/repaired asset.
4. Repair not completed.
5. Work order closed/completed without independent post-repair verification.
6. Repair references wrong asset.
7. Post pressure outside match envelope.
8. Concurrent valve intervention confounds post-flow measurement.
9. Stale calibration/measurement evidence.
10. Raw digest replay across events.
11. Follow-up recurrence after immediate pass.
12. Inspection evidence unresolved/indeterminate.
13. Apparent reduction inside measurement uncertainty.
14. Repair passes verification but violates service floor.

## 5. Metrics

Report per case and event: alert count, completed investigations, independent confirmed leak count, false dispatches, asset match, repair completion, independent verification, recurrence, observed/lower-bound flow reduction, crew hours, repair cost, service interruption and hard-floor violations. Do not collapse to one score; compare a Pareto frontier.

## 6. Claim ceiling

A T0 PASS validates only synthetic evidence handling. It does not qualify a detector, real crew workflow, contractor, repair, field flow reduction, m³/day saved, causal impact, live network or emergency response. No SCADA connection, dispatch, work-order mutation or actuation is authorized.
