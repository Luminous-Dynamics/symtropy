---
title: PIRB Cross-Provider Benchmark Registration
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB Cross-Provider Benchmark Registration v0.1

## Purpose

The registration contract prevents a false leaderboard from being created by comparing systems that predict different things. It complements the PIRB probability evaluator, historical replay, outcome-vintage sensitivity, and offline VIEWS adapter; it does not replace their target-specific input contracts.

The schema is at `docs/schemas/pirb-cross-provider-benchmark-v0.1.schema.json`. The metadata-only example at `docs/fixtures/pirb-cross-provider-benchmark-incompatible-v0.1.json` deliberately marks VIEWS versus ACLED CAST as **not comparable**. It includes no provider forecast rows, no outcome rows, and no measured scores; its digest strings are placeholders, not captured-artifact evidence.

## Evidence from the field

The primary provider sources document different output contracts:

- [VIEWS FAQ](https://viewsforecasting.org/faq/) distinguishes probability forecasts by target and spatial analysis unit, including country-month probability of at least 25 battle-related deaths versus a distinct PRIO-GRID-month target.
- [ACLED CAST methodology](https://acleddata.com/methodology/cast-methodology) describes forecasts of selected political-violence event counts over rolling four-week periods, hierarchical geographic units, and high/low scenarios informed by historical errors.
- A review of conflict early-warning systems reports substantial variation in parameters and gaps in transparency and accessibility, and argues for standards and cooperation: [Rød, Gåsste and Hegre, International Journal of Forecasting (2024)](https://doi.org/10.1016/j.ijforecast.2023.01.001).
- Forecast-to-action valuation work stresses that value-of-information estimates depend on contextual factors relevant to humanitarian decisions: [Weather and Climate Extremes (2020)](https://doi.org/10.1016/j.wace.2018.03.006).

Therefore target, unit, or time-window mismatch is a scientific incompatibility—not a cosmetic metadata difference. A count forecast is not a Bernoulli probability; a scenario range is not automatically a calibrated predictive distribution; a ranking of places is not a probability. A model's published score is not comparable just because the metric name is familiar.

## Paradigm shift: two evaluation lanes

### Lane A — predictive skill

Register a shared estimand before scoring. Freeze event ontology, analysis unit, temporal aggregation, target measure, forecast horizon, model/release, source/evaluator versions, common-row policy, and mature outcome-data vintage.

A pairwise comparison must be one of:

- **directly comparable**: original outputs answer the same target question on matched rows with the same event definition, unit, time aggregation, horizon, and outcome vintage;
- **comparable after predeclared mapping**: a versioned mapping has been independently checked and approved before held-out evaluation, with the mapping code/digest and coverage loss in the receipt;
- **not comparable**: a mismatch remains unresolved. The schema requires both forecast-skill and decision-value comparisons to be disabled in this state.

The Rust binary `pirb-cross-provider-benchmark-validate` enforces cross-field invariants beyond the schema: duplicate provider/comparison IDs, dangling references, self-comparisons, direct-comparison field mismatches, mappings without an independent review and pinned policy, probability-only metric families attached to non-probability outputs, and governance gates for authorized historical evidence. JSON Schema validation alone is not sufficient to certify semantic comparability.

Report the excluded-row count and reasons next to scores. Do not hide selection bias by reporting only the subset where one system is available. Never mix forecasts across target horizons or revised outcomes silently. Label-maturity embargoes and the uncertainty design must be registered before the held-out outcomes are examined.

### Lane B — constrained decision value

Decision value is a separate question: given a locally legitimate action, fixed staff/budget/capacity, and an agreed cost range, what policy produces the best expected service under the same constraints? Evaluate at equal resource capacity and report the trade-off surface, including true/false alerts, missed events, lead time, alert volume, workload, latency, coverage, and group-wise burdens.

This lane cannot auto-select an intervention threshold, direct coercive measures, or deny aid. Stakeholders must define which actions and costs are legitimate; essential services must not be gated by a model's abstention. Keep assumptions visible and run sensitivity analysis over plausible costs instead of optimizing on test outcomes.

A response chain needs separate evidence for forecast issued, warning delivered, acknowledged, decision made, resource dispatched, service delivered, and completion independently verified. A forecast or notification receipt cannot prove the service happened. Neither operational metrics nor forecast scores alone establish causal impact.

## Minimum report card

Every published comparison should include:

| Dimension | Required disclosure |
|---|---|
| Original claim | Provider target, event ontology, geography, temporal aggregation, horizon, output type |
| Provenance | Release/run, model and method version, archived source bytes or approved protected-data commitment, evaluator version, license and attribution |
| Comparability | Status, reason, shared estimand, mapping version/reviewer if applicable |
| Coverage | Eligible common rows, excluded rows by reason, missingness/unknowns, outcome maturity |
| Predictive quality | Target-appropriate primary and secondary metrics, baseline, paired difference and uncertainty |
| Robustness | Rolling-origin windows, alternative mature outcome vintages, predeclared regime-shift tests, group-wise coverage |
| Decision utility | Agreed action policy, resource capacity, false-alert/missed-event burden, lead time, cost sensitivity and service coverage |
| Impact | Separately governed causal design, counterfactual, spillover/displacement checks and independently verified outcomes—or explicit statement that impact is not estimated |
| Claim ceiling | What the evidence supports and what it does not establish |

For probability forecasts, the PIRB target-specific evaluator uses Brier score, log loss, and a rolling past-only climatology comparison, with rare-event metrics and calibration diagnostics secondary. Count forecasts and predictive intervals need appropriate count/distribution/interval scoring and calibration checks; they must not be pushed through the probability evaluator. Use a pinned upstream evaluator as a formula-level comparator, not as an assumed ground truth: metric names do not guarantee identical estimands or definitions.

## Proposed acceptance gates

1. **Contract correctness:** 100% of public synthetic semantic negative controls are rejected with the expected reason; no mismatched output is automatically recoded as a probability.
2. **Replay integrity:** a fixed registration, provider-artifact commitments, normalized row inventory, evaluator build, and lockfile reproduce the same receipt.
3. **Comparable-row integrity:** every score is computed only on the predeclared shared eligible rows; coverage loss and exclusions are visible; no outcome label available after the forecast information cutoff enters a baseline or selection decision.
4. **Predictive advantage:** use paired, dependence-aware uncertainty against at least rolling past-only climatology and a justified operational baseline. Require an interval excluding zero *and* a practically meaningful margin declared before evaluation; do not pick a universal numeric score threshold before the event prevalence, unit and horizon are specified.
5. **Decision utility:** compare policies at the same capacity and predeclared stakeholder costs; report the value-sensitivity range rather than one cherry-picked cost.
6. **Safety and governance:** no protected raw data or guessable small-dataset hashes in public CI; no unsupported causal or deployment-safety claims.

## What would count as “better than state of the art”?

A valid claim must say what target and operating point it improves on, and show paired results on common mature outcomes at the same unit, horizon, release vintage, and test design. Predictive superiority requires practical paired skill improvement and sound calibration/uncertainty, not a higher headline AUC alone. Operational superiority requires better service under equal resource constraints and honest group-wise coverage. Impact superiority requires a separate credible impact design. A result on a mismatched target, a synthetic fixture, a single favorable horizon, or a selected row subset is not a SOTA claim.

## Implementation boundary

This increment establishes a registration schema, a negative-control fixture and a Rust validation CLI; it is not a full comparison engine. It has not been validated against actual provider forecasts and does not implement count scoring, target-mapping execution, common-row alignment, decision-policy optimization, or causal analysis. The validator emits validation and comparability receipts only. A later Rust increment should add versioned target-mapping execution, explicit row-eligibility and exclusion receipts, and separate forecast-skill / capacity-constrained decision-value / impact-evidence outputs.

No real provider records should be admitted until access terms, allowed use, target mapping, outcome maturity, disclosure risk and local governance are reviewed.

## References

- VIEWS FAQ: https://viewsforecasting.org/faq/
- ACLED CAST methodology: https://acleddata.com/methodology/cast-methodology
- Rød, Gåsste and Hegre (2024), *A review and comparison of conflict early warning systems*: https://doi.org/10.1016/j.ijforecast.2023.01.001
- *Bridging forecast verification and humanitarian decisions* (2020): https://doi.org/10.1016/j.wace.2018.03.006
- Waghmare and Ziegel (2026), *Proper Scoring Rules for Estimation and Forecast Evaluation*: https://doi.org/10.1146/annurev-statistics-042424-050626

## Dependencies

Builds on #2092, #2104, #2108, #2109, #2118, #2159, #2160 and #2168.
