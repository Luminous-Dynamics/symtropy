# TLS Precision Health Monitor v0.1

**Issue:** #4795 — SCAN-BIM-TLS-PRECISION-HEALTH-001  
**Composes:** measurement observability #4589, survey/change evidence #1607, TLS repeatability procedures in ISO 17123-9.  
**Status:** P0 synthetic monitor implementation; exact-head Rust/CI qualification pending.

## 1. Separate precision from accuracy

A longitudinal precision monitor asks whether the repeatability of one physical scanner unit changes under a pinned acquisition/control configuration. It does not establish absolute accuracy, calibration validity, registration validity, a survey result or task fitness.

The output explicitly holds `accuracy_disposition` and `task_fitness_disposition` at `NOT_EVALUATED`.

## 2. Statistical unit

One independent capture/check-target session is one observation for the longitudinal test. Individual points and repeated returns within a single capture are used only to derive per-target within-session standard-deviation summaries; they are not counted as independent sample units.

Each session must share:
- physical sensor unit, firmware and profile digest;
- acquisition configuration digest;
- control artifact and independent reference digest;
- coordinate frame;
- range, incidence and environment strata;
- exact reference-target set.

If any of those change, the comparison is invalid and needs a separately qualified baseline.

## 3. P0 test

For each target (t):

`R_t = median(SD_{current,t}) / median(SD_{baseline,t})`

The test statistic is:

`T = max_t |log(R_t)|`

A seeded permutation test shuffles independent session labels while preserving baseline/current group sizes. The max statistic handles family-wise false-alarm control over the target set. The reported Monte Carlo p-value uses the plus-one estimator.

The method assumes session-level independence and exchangeability under the null. It is not a robust guarantee against temporal dependence, changing environmental strata, unmodelled autocorrelation or altered reference/control conditions.

## 4. Frozen synthetic controls

- `no-change.json`: three reference targets across eight baseline and eight current independent synthetic sessions; should not signal at alpha 0.05.
- `precision-change.json`: the third target's within-session standard deviation increases by approximately 3x; should signal at alpha 0.05.
- The monitor never promotes either result to accuracy or task-fitness qualification.

## 5. P1 protocol

Use one named TLS unit and a stable, independently characterized check field. Acquire independent sessions over days and environmental strata while keeping the control target/reference and acquisition configuration fixed. Record operator, temperature, firmware, range, incidence, surface/target class, station geometry, reference uncertainty and session lineage.

Pre-register:
- number and spacing of independent sessions;
- alpha and false-alarm policy across target families;
- practical degradation threshold, separately from statistical significance;
- handling of missing targets, environmental strata and temporal dependence;
- reference-custodian and calibration responsibility.

A statistically significant precision shift triggers investigation/re-verification. It does not automatically diagnose the cause or authorize a correction to the survey or as-built model.

## 6. Standards boundary

ISO 17123-9 field procedures are useful for repeatability checks for an immediate task, but they are not a comprehensive instrument acceptance or performance evaluation. This monitor is a software-based longitudinal sentinel, not a substitute for the standard's field procedures, calibration, independent metrology or competent survey review.
