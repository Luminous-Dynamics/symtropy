# Maritime prognostics and maintenance-decision benchmark v0.1

**Owner:** MARITIME-PROGNOSTICS-001 #4545  
**Related:** engineering-health PR #4444; transfer #4510; data trust #4535  
**Status:** benchmark contract and research protocol; no validated RUL model or maintenance policy

## Objective

Build a benchmark for remaining useful life (RUL), degradation forecasting and maintenance-decision support that does not collapse three distinct questions:

1. What abnormality or degradation is present now?
2. What is the calibrated probability distribution of a precisely defined future event under a declared duty profile?
3. Which inspection or maintenance options warrant human review, given uncertainty, logistics and consequence?

Classification accuracy is not an RUL metric. A point estimate is not a calibrated forecast. A low prediction error is not a safe-life limit. A maintenance-cost optimizer is not an authorization system.

## Paradigm: event-time distributions under explicit duty profiles

An RUL forecast must bind a specific asset/component/configuration, a declared event definition, forecast origin, duty-profile assumption, data/model/code provenance, uncertainty method and censoring semantics. Prefer predictive quantiles or a survival distribution over one unqualified number.

The contract is MARITIME_RUL_FORECAST_V0_1.schema.json. It explicitly represents:
- functional failure, physical failure, threshold crossing, maintenance intervention, retirement or another declared event;
- time units such as operating hours, calendar time, starts or load cycles;
- forecast horizon and p05/p50/p95 RUL quantiles;
- right-, interval- and intervention-censored observation;
- competing event definitions;
- input evidence digest, model artifact digest, code commit, evaluator and generation time;
- a non-authority declaration: no maintenance authorization, restriction clearing or ODD/mode change.

## Evidence and dataset limitations

The Aalto MU323DGSC dataset supports binary anomaly detection and some load-transfer work. It does **not** provide a validated component run-to-failure history or an independently adjudicated RUL target. Never manufacture RUL ground truth by treating the end of a CSV, the first anomaly label, or a file transition as a component failure time.

The June 2026 two-stroke multi-fault study reports a representative RUL RMSE of 0.215 and R² 0.413; its target scaling, horizon and health-index construction require full-paper audit. It also highlights false positives below 1%, but its engine-hour denominator and confidence interval are not available in the abstract. These are reproduction leads, not comparable Aalto benchmark results.

A suitable physical prognostics dataset needs asset identity, engine/configuration, duty/load exposure, maintenance history, inspection outcomes, intervention times, replacement reason, observed failure time if any, observation end and explicit right/interval/competing-risk censoring. Data rights and an independent marine-engineering label review are prerequisites.

## Track A — RUL forecast quality

Evaluate per-event definition and per-domain, not with a pooled scalar:
- MAE, RMSE and median absolute error where the scale and observation process support them;
- empirical coverage and sharpness of 50%, 80%, 90% and 95% prediction intervals;
- interval score / proper scoring rules for predictive distributions;
- concordance or time-dependent discrimination where appropriate;
- prognostic horizon and alpha-lambda accuracy for predeclared event thresholds;
- early/late prediction asymmetry and worst-case error;
- calibration drift under changing load, ambient conditions, maintenance state and engine configuration.

Report performance by engine, component, fault family, load regime, forecast horizon and observation/censoring type. Confidence intervals must cluster by asset/recording/run, never by overlapping time windows.

## Track B — censoring, intervention and competing risks

- A component still operating at observation end is right-censored, not a negative failure forever.
- Inspection that narrows the failure interval is interval-censored, not an exact event timestamp.
- Preventive replacement and scheduled overhaul are intervention-censored; they do not prove that a predicted failure would have happened.
- A different fault, vessel retirement, data-source loss or end of contract may be a competing event or censoring reason, depending on the declared estimand.
- Report both cause-specific and all-cause results where competing events matter.
- Never use future maintenance/work-order notes, post-event measurements, final labels or later calibration records in a forecast generated at an earlier timestamp.

## Track C — transfer and continual prognostics

Compare source-only, target healthy-reference calibration, few-shot, multi-source adaptation, digital-twin-assisted and continual-learning approaches. For every update:
- re-evaluate all prior engine/load/fault domains;
- report RUL error, interval coverage, sharpness and calibration drift;
- report source/target label budget, target-data access and model capacity;
- record whether historical raw data are retained or replayed;
- test false confidence under unsupported duty profiles and configuration changes.

A model that produces a narrow interval outside its validated domain should fail harder than a model that correctly widens uncertainty or abstains.

## Track D — maintenance-decision utility

Replay policies on identical time-ordered evidence:
- calendar/interval maintenance;
- fixed condition threshold;
- alarm/event-triggered inspection;
- RUL-quantile-triggered inspection;
- risk/cost-aware policy with human review;
- documented engineering practice baseline, where permissioned.

Measure expected downtime, unplanned interventions, inspection burden, spares use, missed-fault exposure, delay to inspection, service loss and lifecycle cost. Publish uncertainty and sensitivity to assumptions (port access, spares, crew, operating schedule and intervention lead times).

A policy may prioritize review; it must not create a work order, authorize maintenance, alter an ODD/mode or override the master/operator/Chief Engineer. Hard safety and authority constraints are non-compensatory; lower expected cost cannot buy permission to defer a required inspection.

## Frozen falsification cases

1. Future maintenance and post-event telemetry leak into an earlier forecast.
2. Preventive replacement is treated as an observed natural failure.
3. A right-censored engine is treated as a permanent non-failure.
4. A pseudo-health index is used both to construct RUL labels and score the model.
5. Target fault labels leak into digital-twin calibration declared zero-shot.
6. High-load duty is absent from training and the model still emits a narrow optimistic interval.
7. Domain adaptation improves mean error while worsening the rare severe-fault regime.
8. Continual updates improve the newest task but destroy prior-domain interval calibration.
9. Missing or reconstructed values suppress anomaly evidence and increase predicted RUL.
10. One model emits operating-hour RUL while another emits calendar-hour RUL; the leaderboard must reject the unaligned comparison.
11. A correct forecast is promoted to maintenance authorization or restriction clearance.
12. A changed component/configuration inherits a prior RUL qualification without revalidation.

## Proposed internal targets — aspirations only

These targets are meaningful only after an independently reviewed multi-asset prognostics dataset exists:
- 90% empirical coverage for nominal 90% forecast intervals in each adequately sampled held-out domain, with grouped confidence intervals;
- at least 15% lower normalized RUL error than the strongest reproduced baseline without worsening worst-domain interval coverage;
- demonstrable decision-utility improvement over matched threshold/calendar baselines under transparent cost and logistics assumptions;
- zero future-data, censoring, missingness, configuration and authority-gate violations across the frozen adversarial suite;
- prior-domain RUL calibration within a predeclared tolerance after each continual update.

Do not optimize these targets on a test set or present them as safe-life limits.

## Validation ladder

- P0: event, duty, censoring, asset and forecast contract schema.
- P1: synthetic leakage/censoring/calibration/authority mutation suite.
- P2: independent paper reproduction with exact task and target definitions.
- P3: permissioned physical degradation/intervention records with run-level splits and independent label review.
- P4: matched maintenance-policy replay with explicit operational/logistics assumptions.
- P5: supervised testbed and vessel-specific validation, independent engineering and class/operator review as applicable.

A passing synthetic or paper-reproduction benchmark does not establish safe remaining life or permission to operate.

## Claim ceiling

A PASS establishes only the reported prognostic and decision-support behavior for the exact frozen data, event definition, censoring process, duty profile, model, split, code and evaluator. It does not establish safe life, reliability of another engine/vessel, seaworthiness, class/flag approval, maintenance authorization, ODD clearance or permission to operate.
