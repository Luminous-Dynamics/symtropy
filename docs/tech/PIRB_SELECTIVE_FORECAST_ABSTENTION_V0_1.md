---
title: PIRB Selective Forecasting and Abstention Audit
version: 0.1
status: research-stage, synthetic-only
authority: MeasurementOnly
---

# Selective forecasting and abstention audit v0.1

## Purpose

A model should not manufacture a probability when its evidence is stale, insufficient, outside its supported input domain, or mapped to the wrong target. PIRB records a reason-coded disposition and measures the cost of selectivity through both system coverage and forecast risk. Abstention is not a prediction of no event, a recommendation to suppress warning, or a gate on aid.

This follows the selective-prediction principle of reporting risk and coverage together rather than celebrating low error after silently dropping hard cases. It also extends the shift-aware, cost-aware deferral pattern studied in recent selective-prediction work. The Scientific Reports 2026 study on conformal selective prediction reports temporal in-distribution and out-of-distribution evaluation; its clinical numbers are not transferable to conflict forecasting, but the evaluation design is a useful methodological reference: https://www.nature.com/articles/s41598-026-40637-w.

PIRB's proper-scoring-rule framework remains authoritative for forecast target and formula details. Waghmare and Ziegel's 2026 review discusses proper scoring rules for estimation and forecast evaluation: https://doi.org/10.1146/annurev-statistics-042424-050626.

## Input contract

Schema: `docs/schemas/pirb-selective-forecast-v0.1.schema.json`.

Synthetic public fixture: `docs/fixtures/pirb-selective-forecast-synthetic-v0.1.json`.

The v0.1 schema deliberately admits only invented synthetic rows. A real historical study needs a separately reviewed extension to source permissions, frozen vintages, target-specific horizons, disclosure policy and independent provenance.

Every row carries:
- an opaque record ID and an opaque aggregate stratum ID;
- an explicit disposition;
- observed outcome status (`event`, `no_event`, or `unknown`);
- evidence sufficiency, input freshness, supported-domain status and target-match status;
- a probability and ranking score only if the row actually passed all evidence gates.

Disposition values are `scored`, `abstain_insufficient_evidence`, `abstain_stale_inputs`, `abstain_out_of_support`, `abstain_target_mismatch`, and `not_scored_unknown_outcome`. Reason codes are semantic assertions: a stale-input disposition requires stale inputs; out-of-support requires the corresponding support status; target-mismatch requires target mismatch. A scored row requires fresh inputs, sufficient evidence, an in-support input, a compatible target, known outcome and valid probability. Any non-scored row must not publish a forecast probability or selection score.

Unknown labels are not no-events. When evidence or target mismatch is itself the reason a record cannot be scored, the specific failure reason remains the disposition and the unknown outcome is reported separately. `not_scored_unknown_outcome` is reserved for records whose other evidence gates pass but whose outcome is still unknown.

## Two different coverage denominators

### Actual system score coverage

`actual_score_coverage = scored_record_count / total_record_count`

This exposes the number of cases the registered policy actually scored. All reason-coded non-scored records remain in its denominator. A system that abstains on every hard case cannot advertise a full-population score using only its easy records.

### Conditional risk–coverage among scored cases

For this v0.1 diagnostic, rows that passed the evidence gates are ranked by a preregistered `selection_score` (descending, then `record_id` ascending). At each preregistered coverage level, PIRB retains `ceil(level × scored_count)` records and reports Brier score, clipped log loss, observed event prevalence, realized coverage and boundary score.

This risk–coverage curve is explicitly **conditional on already-scored rows**. It measures whether the registered rank score orders model errors among eligible forecasts; it is not a counterfactual estimate of the risk that stale or unsupported rows would have had if forced to produce predictions. The actual system coverage and reason-specific outcome summaries are reported separately.

Coverage levels must be strictly increasing, include full coverage (1.0), and be fixed before evaluation. Exact ranking ties use a deterministic record-ID tie-break. The fixed fixture makes this replay deterministic; no threshold is selected on held-out outcomes.

## Reported metrics

The Rust binary `pirb-selective-forecast-audit` emits a receipt that binds the input/schema digests, source commit, forecast archive and outcome snapshot references.

It reports:
- total records, scored count and actual score coverage;
- reason-coded disposition counts;
- event, no-event and unknown outcome counts for each non-scored disposition;
- event prevalence among known outcomes and unknown-label rate among each non-scored disposition;
- a preregistered conditional risk–coverage curve with Brier and log loss;
- aggregate stratum metrics only when the group count meets the registered minimum; suppressed group counts are reported separately;
- explicit non-claims for distribution-shift detection quality, fairness, operational utility, causal impact and live decision authority.

Brier and log loss are evaluated only on known outcomes among scored records. Log loss uses the predeclared clipping epsilon of (10^{-15}) for numerical stability. This does not authorize clipping or calibration thresholds to be tuned on test outcomes.

## Synthetic benchmark gates

- 100% of declared synthetic negative controls reject mismatched reasons, stale or unsupported scored rows, target mismatch, unknown labels scored as no-event, duplicate row IDs and invalid coverage levels.
- Repeated replay with the same input, schema, source commit and evaluator produces identical receipt content and commitment.
- Non-scored rows never contribute a zero-valued prediction to Brier/log loss, and unknown outcomes never silently enter the no-event denominator.
- Every preregistered coverage point is reported; no single risk–coverage point is picked after looking at test results.
- Group metrics are suppressed below the registered minimum denominator.
- Stale source inputs, unknown labels, insufficient evidence, unsupported domains and target mismatch remain separate dispositions.

## What this does not prove

The synthetic suite does not establish that a real shift detector recognizes a new regime, that abstention is fair, or that any policy improves humanitarian outcomes. Because the selection curve is conditional on passed evidence gates, it cannot by itself prove that abstention is safer than forced prediction for all excluded records. A future authorized historical evaluation should add rolling-origin temporal splits, frozen outcome vintages, predeclared shift perturbations, group-wise coverage and unknown rates, and cost-aware decision-policy evaluation on common eligible rows.

The benchmark is not allowed to select operational thresholds on test data. No live use may convert abstention into denial of aid or suppress warning solely because a model refuses to score a case.

## References

- Kwon & Kim (2026), *Conformal selective prediction with cost aware deferral for safe clinical triage under distribution shift*, Scientific Reports: https://www.nature.com/articles/s41598-026-40637-w
- Waghmare & Ziegel (2026), *Proper Scoring Rules for Estimation and Forecast Evaluation*, Annual Review of Statistics and Its Application: https://doi.org/10.1146/annurev-statistics-042424-050626
- PIRB preregistered benchmark ladder: issue #2159
- PIRB selective forecasting and shift benchmark: issue #2160

## Dependencies

Follows the probability scoring, frozen-outcome, VIEWS import, target comparability and hierarchy coherence contracts (#2104, #2108, #2109, #2118, #2160, #2170, #2190).