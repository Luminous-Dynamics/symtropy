# WDN verified intervention yield T0 protocol v0.1

**Tracking issue:** #5969 WDN-VERIFIED-INTERVENTION-YIELD-001  
**Parents:** #4963 leak diagnostics, #5018 control robustness  
**Evidence class:** synthetic-only; no field workflow admitted.

## 1. Paradigm: the endpoint is a verified intervention

Evaluate the chain: alert → independent investigation → confirmed asset/fault → repair completion → independent post-repair verification → matched-condition hydraulic change → follow-up recurrence and service outcome.

An alert is never ground truth. A work-order closure is not proof of a repair. An immediate verification pass is not evidence of persistence. A pre/post difference under changed pressure, demand, instrumentation or network configuration is not attributable to the repair.

## 2. Independent evidence chain

- Alert binds event ID, topology revision, DMA, candidate asset, first-seen time and evidence receipt.
- Inspection binds the same event ID, confirmed asset, finding, fault class, crew, inspection time, evidence receipt and a reviewer distinct from the inspector.
- Repair/work order binds the same event and confirmed asset, completion state, repair receipt, completion time, crew, crew-hours and cost.
- Post-repair verifier must be independent of both inspection and repair crew and bind a verification receipt to the same event.
- Pre/post measurements bind meter ID, calibration, method, boundary, units, pressure, demand-profile digest, time windows, validity and immutable raw-record digest.
- Any raw-record digest reused across measurement slots or events invalidates every affected event; event IDs must be globally unique in the frozen input.

## 3. Matched-condition flow comparison

Quantitative comparison requires:
1. Same instrument identity, current calibration reference, method, physical boundary and units for pre/post windows.
2. Identical demand-profile digest.
3. Complete, valid, chronological, non-overlapping windows: pre window before repair completion; post window after repair; verification after post window.
4. Service pressure difference within the case's pre-registered tolerance.
5. No confounding valve, pump, source, demand-management, adjacent-repair or meter intervention.

If any gate fails, flow change is indeterminate and not emitted as a verified outcome. v0.1 does not normalize pressure using a generic exponent; a future site-specific normalization model would require separate calibration and validation.

Given pre/post boundary flow rates `Q_pre`, `Q_post` and standard uncertainties `u_pre`, `u_post`:

`observed_matched_condition_change = Q_pre - Q_post`

`lower_bound_change = max(0, Q_pre - Q_post - k*(u_pre + u_post))`

v0.1 uses k=2 as a declared conservative expansion factor. This is not a confidence interval, probability, statistical test or causal estimate. Numeric values are rounded to 1e-9 L/s for deterministic receipts.

Do not annualize short-window change. The metric is **observed matched-condition flow reduction**, not causal water saved. Causal attribution and persistence models remain NotRun.

## 4. Repair and service dispositions

Keep separate counts for:
- independently confirmed leaks and confirmed no-leak false dispatches;
- asset localization match/mismatch;
- repair completion;
- independent post-repair verification;
- verified repair (including verified repairs with service-floor harm);
- recurrence during follow-up;
- observed and uncertainty-screened matched-condition flow reduction;
- service interruption, affected connections, crew-hours and repair cost.

A verified repair with a service-floor violation still counts as a physically verified repair, but the harm is separately reported and prevents it from being hidden in a single positive score. A recurrence overrides the durable verified-repair disposition even when the immediate post-repair check passed.

## 5. Frozen 14-case challenge set

1. Positive independently confirmed, repaired and verified leak.
2. Confirmed no-leak false dispatch.
3. Alert candidate asset differs from confirmed/repaired asset.
4. Repair authorized but not completed.
5. Work order completed without independent post-repair verification.
6. Repair asset differs from inspected asset.
7. Post pressure outside match envelope.
8. Concurrent valve intervention confounds comparison.
9. Stale calibration/measurement evidence.
10. Raw-record digest replay across two events.
11. Follow-up recurrence after immediate pass.
12. Unresolved inspection evidence.
13. Apparent reduction inside measurement uncertainty.
14. Repair verified but service floor violated.

## 6. Reporting

Report a vector and Pareto frontier, not a scalar leaderboard: alert count/budget, completed investigations, independent confirmed leaks, false dispatches, asset match, completed repairs, independent verifications, verified repairs, recurrence, observed/lower-bound L/s change, crew-hours, repair cost, interruptions, affected connections and service-floor violations.

Currency in the synthetic fixture is ZAR minor units (cents). No cost per m³ or annualized benefit is inferred by T0.

## 7. Claim ceiling

A PASS validates only synthetic evidence handling. It does not qualify a detector, real crew workflow, contractor, repair, field flow reduction, m³/day saved, causal impact, live network or emergency response. No SCADA connection, dispatch, work-order mutation or actuation is authorized.
