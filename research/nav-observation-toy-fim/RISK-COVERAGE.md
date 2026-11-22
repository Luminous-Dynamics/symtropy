# Navigation risk–coverage evaluator v0.1

**Issue:** [NAV-RISK-COVERAGE-001 #6687](https://github.com/Luminous-Dynamics/symtropy/issues/6687)  
**Status:** offline evaluator prototype; no physical validation.

## Why

A navigation service needs the right to abstain, degrade or declare unavailable when evidence is weak. But an accuracy-only score can reward a system that avoids difficult cases by abstaining. Conversely, a coverage-only score can reward unsafe claims. Report the complete risk–coverage frontier.

## Metrics

For each confidence threshold, the evaluator reports:
- record coverage and exposure-time coverage;
- abstained records;
- false-safe and false-degraded record counts;
- false-safe and false-degraded exposure seconds;
- selective error rate;
- false-safe record rate and one-sided 95% Wilson upper bound;
- asymmetric, duration-weighted service loss per covered hour.

The false-safe loss and false-degraded loss are supplied by an application-specific service profile. The implementation does not select those weights.

A threshold of 0 includes all records. The final threshold, 1,000,001 ppm, explicitly abstains on all records. Zero coverage returns null risk metrics, not a misleading zero-risk claim.

## Statistical caveat

The Wilson upper confidence bound treats covered records as independent Bernoulli trials. Navigation episodes often share physical events, site, hardware, firmware, clocks, power, anchors and data pipelines. For published results, report cluster-aware bootstrap or a justified hierarchical model at the event/site/device/failure-domain level. Do not interpret a per-record Wilson bound as an integrity certificate.

## Evaluation protocol

Select a threshold on a frozen validation split only. Lock it, hash the policy/configuration, then evaluate once on a separate held-out test split. Plot the full risk–coverage curve for analysis but do not choose the final threshold using test truth.

The evaluator consumes ground truth and must run outside the controller. See the controller/evaluator boundary prototype [PR #6684](https://github.com/Luminous-Dynamics/symtropy/pull/6684).

## Limits

No sensor estimator, integrity monitor or calibrated confidence model is implemented here. This prototype measures the consequences of supplied claims. It does not prove physical independence, label correctness, protection levels, or acceptable service risk.
