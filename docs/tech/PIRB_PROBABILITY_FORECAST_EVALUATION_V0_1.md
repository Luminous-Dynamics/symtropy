---
title: PIRB Probability Forecast Evaluation Contract
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB probability forecast evaluation contract v0.1

## Paradigm shift

Do not ask only, "How many events did the forecast identify?" Evaluate the full probability distribution, its calibration, its value relative to information actually available at forecast time, and where it is unreliable. Keep this distinct from the operational warning-to-response chain.

A high ROC-AUC does not establish well-calibrated probabilities. A good Brier score can still be weak relative to a trivial historical-rate baseline. Average precision is informative for rare outcomes but cannot replace proper scoring rules. A global metric can also hide weak performance in lower-volume units; group diagnostics must be reported with declared privacy rules and uncertainty.

The core scoring principles use proper scoring rules rather than treating any one threshold as the forecast itself. See Waghmare and Ziegel, *Proper Scoring Rules for Estimation and Forecast Evaluation* (Annual Review of Statistics and Its Application, 2026): https://doi.org/10.1146/annurev-statistics-042424-050626.

## Input and source contract

A dataset is one fixed prediction target and one spatial analysis unit, with a frozen and named outcome-data release. Metadata must preserve:

- Provider, exact forecast release/run ID, model/version, probability variable, target threshold and definition.
- Analysis unit and geography reference; country-month and PRIO-GRID-month data are never mixed in one dataset.
- Forecast release digest, outcome release ID and digest, target/outcome definition, source references, license reference and required attribution.
- For each forecast, the issue month, last month of input data, target month, lead month, probability, outcome state and outcome-availability month.
- The evaluation-as-of month and train/calibration/test issue-month boundaries.
- A declared log-loss clipping epsilon, reliability bin count, minimum climatology history and minimum safe subgroup denominator.

The normalized timestamp is a monotonic integer month index, `year * 12 + (month - 1)`. Forecast lead must equal target month minus input cutoff month. The target month must be after both the cutoff and forecast issue month. This makes release vintage and horizon explicit rather than trying to infer them from row order.

For the currently published VIEWS system, the official FAQ states that country-level dichotomous forecasts predict the probability of at least 25 battle-related deaths per country-month, whereas PRIO-GRID-month forecasts predict the probability of at least one battle-related death per grid cell and month. The system also publishes fatality-count forecasts and these must not be interpreted as probabilities. Its current forecasts span 1–36 months from the last month of input data, and its evaluation partition has separate training, calibration and test periods. Source: https://viewsforecasting.org/faq/. The latest forecast release advertised on the data page when this contract was drafted is `fatalities003_2026_08_t01`, informed by inputs through August 2026 and forecasting September 2026–August 2029: https://viewsforecasting.org/data/.

The MVP is a normalized-input evaluator, not a data downloader or an endorsement of any source's use terms. Source/target normalization must be separately reviewed before actual provider data are admitted.

## Temporal validity and label maturity

For each record:

- Forecast issue month must be inside the declared split range.
- Input cutoff cannot be later than issuance.
- Target must be after issue/cutoff, and the stated lead must match the cutoff-target difference.
- Probability must be a finite number in [0, 1].
- A known event/no-event label must have an availability month strictly after the target month and no later than the evaluation-as-of month.
- Unknown labels must remain unknown and have no claimed availability month.
- For the same unit/target month, the frozen outcome version and availability month must agree across repeated forecast vintages.

To prevent label-maturity leakage at monthly granularity, every known training outcome must have an availability month strictly earlier than the calibration boundary, and every known calibration outcome must have an availability month strictly earlier than the held-out test boundary. A label reported in the boundary month is ambiguous at monthly resolution and is conservatively rejected. For a 36-month lead, the split gap may therefore need to include the full target horizon and outcome-publication delay. That restriction is intentional; otherwise later outcomes can determine earlier model selection.

A dataset that mixes revised outcome vintages must be separated into explicit evaluation datasets or expanded in a reviewed future contract. It must not silently overwrite the original target labels.

## Scoring

### Primary proper scores

- **Brier score:** mean squared probability error.
- **Log loss:** mean Bernoulli negative log likelihood. The contract fixes an epsilon solely for numerically stable clipping at exact 0/1.
- **Brier skill score versus rolling past-only climatology:** `1 - model_brier / baseline_brier`, computed over the identical subset of records with a valid historical-rate baseline.

The baseline includes each unique known unit-month outcome only after its declared availability month. It is queried at each forecast's input cutoff. A forecast cannot borrow outcomes that were not yet released. History below the declared minimum yields no baseline for that record; the receipt gives the exact coverage and does not convert missing baselines into zero skill.

### Secondary diagnostics

- ROC-AUC using tie-aware average ranks; null if either outcome class is absent.
- Average precision with tied probability values evaluated as a group; null if either class is absent.
- Fixed equal-width reliability bins with row count, average probability and observed event rate.
- Expected calibration error as a descriptive weighted absolute reliability gap; always disclose bin count and scored denominator.
- By-split event/no-event/unknown counts, score coverage and score nullability.
- Held-out unit Brier scores only when the unit has at least the declared count of known labels; publish the number of suppressed units.

Calibration intercept/slope, confidence intervals, block bootstrap, cost-sensitive alert selection, and decision-curve/net-benefit analysis are **not implemented in v0.1**. They are next-stage work requiring appropriate sample-size, spatial/temporal dependence, decision-cost and governance choices. The tool never automatically selects an alert threshold.

## Receipt

The deterministic JSON receipt binds:

- The exact source commit argument and format check (the CI workflow separately requires that commit to equal the checked-out event SHA).
- The complete normalized-input, checked-in schema and generated Cargo lock digests.
- The forecast and outcome release digests already supplied by the adapter.
- Provider/model/version, variable/target definition, unit, outcome vintage, thresholds, source attribution and privacy policy.
- Split boundaries, log-loss epsilon, reliability bins, metrics, null metrics, baseline coverage and suppressed-unit count.
- Explicit false values for probability-calibration fitting, causal-effect estimation and live decision authority.

The source digests supplied by an adapter are not independently fetched or authenticated by the evaluator. For protected data, do not publish raw inputs or their raw SHA-256 hashes in public CI; keep the audit in the authorized environment and release only approved aggregates.

## Acceptance targets

Software conformance:
- 100% pass of schema, chronology, maturity and negative-control tests.
- Zero accepted probabilities outside [0, 1]; zero accepted lead/cutoff mismatches.
- Zero outcome labels made available after the forecast cutoff used in the past-only baseline.
- Zero calibration labels admitted to threshold/model-selection evaluation unless mature at the test boundary.
- Byte-for-byte input/schema/lock digests independently checked in a passing exact-head receipt.
- All scores independently recomputable; undefined AUC/AP remain null instead of receiving fabricated values.
- Exact suppression behavior for units below the declared denominator.
- No causal, consent, source-truth, or deployment-safety claim emitted by the evaluator.

Empirical performance:
- No universal probability-score threshold is asserted in a synthetic fixture. On actual authorized data, compare against the rolling past-only climatology and any justified persistence/reference forecasts, report uncertainty that respects temporal and spatial dependence, and publish base rates and coverage by lead and analysis unit.
- Report performance separately by lead-month band, target definition, and relevant governed groups. Do not select one favorable horizon or one favorable outcome vintage after inspecting test scores.
- No outcome source is admitted until its definition, release vintage, maturity, licensing, disclosure risk and mapping have been reviewed.

## Explicit non-claims

This contract and its synthetic tests do not prove conflict can be predicted in a useful way, that any warning will be acted upon, that a response reduces harm, or that any intervention causes peace. Statistical forecast performance is one necessary evidence class; governed human response, service delivery, distributional effects and causal evaluation remain separate gates.