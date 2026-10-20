---
title: PIRB Hierarchical Forecast Coherence Audit
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB hierarchical forecast coherence audit v0.1

## Purpose

Forecasts from a hierarchical system are related quantities, not independent rows. Subnational counts should aggregate consistently to country counts when coverage is complete; disaggregated event-type counts should reconcile to any declared composite total. PIRB measures this structural property separately from predictive accuracy.

This design is informed by the public [ACLED CAST methodology](https://acleddata.com/methodology/cast-methodology), which documents forecasts at administrative-1, country and global levels, event-type totals, MinTraceSparse reconciliation, and high/low scenarios estimated from historical forecast errors. That is a relevant public design baseline, not an assumed performance winner.

## Paradigm shift

Add an auditable constraint system around forecasts. For each reconciliation constraint, preserve the unreconciled parent and child predictions and the post-reconciliation predictions. Emit both sets of residuals, plus the sizes of adjustments. Never call improved coherence an accuracy improvement without evaluating against mature outcomes.

A missing child is not a zero. Only a constraint declared complete can be assessed for exact sum coherence. Incomplete or unknown child coverage is reported as `not_assessable`, including the missing forecast IDs, and is excluded from the coherence-rate denominator. A registration that claims complete coverage but omits a declared child is rejected.

## Contract

The schema is `docs/schemas/pirb-hierarchy-forecast-v0.1.schema.json`. The checked-in fixture is `docs/fixtures/pirb-hierarchy-forecast-synthetic-v0.1.json`; it contains invented point counts and two invented finite joint predictive draws, with placeholder artifact/method digests. It does not contain CAST outputs or any real provider data.

A registration pins the provider/release, target definition, analysis-unit scheme, temporal aggregation and index policy, forecast artifact and hierarchy digests, and reconciliation method digest. The fit-cutoff index must be strictly earlier than the forecast-origin index; the target period must be later than forecast origin. The schema requires that test outcomes and future data were not used and that pre-reconciliation predictions are preserved. These are contract declarations, not external authentication; real research still needs independently verified source-capture provenance.

Forecast rows are unique by `forecast_id`, share the registered target period and carry both `raw_count` and `reconciled_count`. Each aggregation constraint states:
- kind: `spatial` (same event type across units) or `event_type` (same unit across event types);
- one parent forecast and a unique list of child forecast IDs;
- declared completeness: `complete`, `incomplete`, or `unknown`;
- an explicit `expected_child_keys` set (unit IDs for spatial constraints, event-type IDs for event-type constraints), taken from the pinned hierarchy/ontology;
- a predeclared absolute residual tolerance.

For a complete constraint, the listed child rows must match the declared expected-child keys exactly. A missing forecast reference *or* an existing child forecast silently omitted from the listed set causes rejection. For incomplete/unknown constraints, the receipt reports missing forecast IDs and missing/unexpected expected-child keys instead of making them disappear. Spatial constraints cannot mix event types; event-type constraints cannot mix spatial units. Duplicate forecast and constraint IDs, dangling parent references, cycles in the aggregation graph, mismatched periods, invalid fit cutoff, negative count values and invalid complete-child claims fail closed.

## Receipt and metrics

The Rust command `pirb-hierarchy-coherence-audit` emits a deterministic JSON receipt binding input/schema digests, hierarchy and forecast commitments, reconciliation method digest, source commit, per-constraint point residuals, per-draw predictive-sample residuals, completeness and summary metrics. The `predictive_samples` representation is `finite_joint_count_samples`; each draw must contain exactly one nonnegative integer count row for every declared forecast ID. Duplicate draw/forecast pairs, unknown forecast IDs, and incomplete draw matrices are rejected.

Per complete constraint, signed residual is parent count minus the sum of the listed children. For raw and reconciled forecasts, the receipt records the parent value, child sum, residual and status against the registered tolerance. The summary reports:
- assessable, coherent, incoherent and not-assessable constraint counts;
- raw and reconciled coherence rates, with a null rate if the denominator is zero;
- mean absolute residual before and after reconciliation;
- total and maximum absolute per-row point-forecast reconciliation adjustment;
- predictive draw count, per-draw assessed/not-assessable constraint counts, raw and reconciled coherent/incoherent sample-constraint counts, and sample coherence rates.

Each complete constraint is assessed separately for each draw by summing that draw's child values and comparing them with the same draw's parent. The fixture's raw sample draws intentionally violate two spatial constraints; its reconciled sample draws satisfy every assessable synthetic constraint. The declared incomplete spatial constraint remains not assessable in both draws and is excluded from the sample coherence-rate denominator. This checks joint-sample consistency, not calibration or distribution quality.

The adjustment statistics measure how much the point values moved; they do not by themselves show whether the adjustments improved forecast accuracy. A system can be perfectly coherent and still predict badly. The audit does not calculate Brier score, log loss, count forecast skill, interval coverage, energy/variogram score, operational utility or causal impact.

## Suggested benchmark ladder

1. **Contract conformance:** 100% of synthetic negative controls reject invalid graph, target, period, completeness, or leakage declarations as expected.
2. **Identity control:** for exact synthetic integer sums and zero tolerance, post-reconciliation residual is exactly zero for every complete constraint.
3. **Honest coverage:** missing children never become implicit zeros; incomplete groups do not enter the coherence-rate denominator.
4. **Transparent reconciliation:** publish pre/post residuals and absolute adjustments together; the benchmark never collapses coherence and accuracy into a single score.
5. **No leakage:** the declared parameter-fit cutoff precedes forecast origin, with no access to held-out outcomes or future inputs.
6. **Joint sample integrity:** 100% of synthetic sample-matrix negative controls reject duplicate, unknown, or missing draw/forecast rows; coherent sample draws satisfy every complete aggregation constraint within its registered tolerance.
7. **Predictive evaluation later:** on authorized mature historical data, evaluate target-compatible count scores and uncertainty coverage on aligned rows, by spatial level and horizon, with dependence-aware uncertainty and predeclared outcome-vintage policy.

## Limitations and claim ceiling

This version checks point rows and already-reconciled finite predictive count samples against the registered parent/child constraint list. It does not independently authenticate that list against an external geography crosswalk, implement MinTraceSparse, fit reconciliation parameters, transform unreconciled predictive distributions into reconciled joint samples, assess distributional calibration, or prove nominal interval coverage. Those require separate method implementations and data governance. A synthetic coherence pass proves only contract behavior, not real forecast coherence, forecast skill, successful warning response, reduced violence, or causal peace impact.

Do not compare systems unless target, event ontology, analysis unit, temporal aggregation and horizon are aligned. Public methods and metric names do not imply comparable estimands. Review provider data permissions and disclosure risk before ingesting real forecasts.

## References

- ACLED CAST methodology: https://acleddata.com/methodology/cast-methodology
- Public CAST implementation: https://github.com/ACLED/cast-public
- Conflict early warning system comparison (2024): https://doi.org/10.1016/j.ijforecast.2023.01.001
- PIRB cross-provider comparability contract: `docs/tech/PIRB_CROSS_PROVIDER_BENCHMARK_V0_1.md`

## Dependencies

Follow-on to #2188 and the cross-provider registration contract in PR #2170. Builds on frozen forecasts and outcome vintages from #2109 and #2118.
