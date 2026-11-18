# WDN repair outcome and impact benchmark T0 protocol v0.1

**Tracking issue:** #6022  
**Diagnosis benchmark:** #4963  
**Safe active observability:** #6015 / draft PR #6021  
**Controller abstention:** #5018  
**Date:** 18 November 2026  
**Evidence class:** synthetic-only evaluator contract; physical data not admitted.

## 1. Objective

Measure whether a diagnosis pipeline produces better field outcomes, not just better-looking alerts. Preserve the whole episode from frozen model output through independent inspection, repair, and post-repair measurement. A predicted location is never used as its own ground truth.

## 2. Four distinct outcome states

1. `inspection_confirmation`: leak_confirmed / no_leak_found / inconclusive.
2. `repair_completion`: completed / not_started / incomplete, with independently verified work-order and repair evidence.
3. `observed_flow_change`: descriptive target pre/post change, irrespective of causal attribution.
4. `adjusted_flow_reduction`: matched-control difference-in-differences with an uncertainty-screened lower bound, only when every comparability gate passes.

Never collapse these into a single success boolean. A confirmed leak can remain unrepaired; a completed repair can leave residual leakage; a flow reduction can be confounded; a false-positive dispatch remains a measurable operational cost.

## 3. Matched-control estimator

For target-site and untreated-control flow in L/s:

`adjusted_reduction_lps = (target_pre_lps - target_post_lps) - (control_pre_lps - control_post_lps)`

Use same declared physical boundary, comparable interval duration, pressure band, demand profile and pre-trend. The control selection is predeclared before viewing post-repair data. Report all four source readings and raw digests.

For a conservative v0.1 measurement screen, add the four standard uncertainties rather than assuming independent errors:

`lower_reduction_lps = max(0, adjusted_reduction_lps - k * (u_target_pre + u_target_post + u_control_pre + u_control_post))`

`observed_horizon_lower_volume_l = lower_reduction_lps * followup_duration_seconds`

v0.1 fixes `k=2` as an uncertainty expansion factor only. It is not a confidence interval, hypothesis test or causal probability. It does not justify annualizing a short observed horizon.

Where there is no matched control, report descriptive pre/post change only and set `causal_claim_eligibility=NotEligible`. Where pre-trends, pressure, demand, time, calibration, intervention log or measurement quality fail, return `indeterminate`.

## 4. Conditions for adjusted reduction

- independent leak confirmation and independently verified repair completion;
- exact same target flow boundary before/after;
- target and control measurement windows valid and non-overlapping in time;
- control area not treated or exposed to the same repair/intervention;
- matched interval duration;
- pressure difference within predeclared tolerance;
- demand difference within predeclared tolerance;
- pre-trend diagnostic passes its declared tolerance;
- no unmodelled pump/valve, DMA boundary, pressure-management, meter, demand-management or topology changes;
- calibration validity covers the complete measurement windows;
- unique event/work-order/meter/record identities and immutable raw-record digests;
- sufficiently long declared post-repair observation horizon.

These conditions are necessary but do not automatically prove causality. Report the identifying assumptions and retain `causal_claim_eligibility=NotEligible` until independent methodological review authorizes a stronger claim.

## 5. Benchmark outcomes

### Diagnosis-to-field
- independently confirmed leak precision/recall;
- Top-1/Top-5 inspection hit rate;
- no-leak-found and inconclusive dispatch rates;
- crew minutes and elapsed hours to first confirmation;
- false excavation and unnecessary isolation counts.

### Repair/service
- independently verified repair completion;
- post-repair residual leak flow;
- adjusted L/s reduction and conservative lower bound;
- observed-horizon lower-bound litres only;
- interruptions and low-pressure minutes;
- crew-hours, repair cost and cost per supported L/s reduction;
- repeat failure within a declared follow-up horizon.

### System economics
- confirmed leaks per crew-day;
- lower-bound L/s reduction per crew-day;
- cost per confirmed leak;
- observed-horizon litres per crew-day;
- alert-to-confirmation and confirmation-to-repair latency.

## 6. Frozen 14-case synthetic challenge pack

1. Confirmed, independently repaired leak with stable matched control.
2. False alert; inspection finds no leak.
3. Leak confirmed but no repair completed.
4. Repair completed with non-zero residual leak.
5. Target and control both shift similarly; adjusted reduction near zero.
6. Target-only pump/valve change confounds comparison.
7. Pressure mismatch.
8. Demand mismatch.
9. Pre-trend/parallel-trend diagnostic rejected.
10. Calibration validity expires during a measurement window.
11. Duplicate raw-record digest or event/work-order identity.
12. Post-repair observation shorter than the declared minimum.
13. Positive point estimate but lower uncertainty bound is zero.
14. No matched control; descriptive-only outcome, no adjusted recovery claim.

## 7. Claim ceiling

A synthetic PASS validates the calculation and abstention contract only. It does not prove actual water savings, a causal repair effect, utility-wide performance, field repair success, or regulatory compliance. A stronger physical claim requires independent inspection, utility-approved evidence access, calibrated meters, matched controls and methodological review. No annualized saving, potable-water, SANS 241 or deployment claim is emitted by T0.