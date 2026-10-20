---
title: PIRB Joint Predictive Sample Coherence Audit
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB joint predictive sample coherence audit v0.1

## Research motivation

Public conflict forecasting already treats hierarchy as a first-class engineering concern. ACLED CAST describes forecasts over rolling four-week periods at ADMIN1, country and global levels, with distinct event types and an organized-violence total. It uses MinTraceSparse for point-forecast reconciliation and constructs high/low scenarios from rolling historical forecast residuals. See the [CAST methodology](https://acleddata.com/methodology/cast-methodology).

That is a useful industry baseline, but point coherence does not prove that a predictive distribution is coherent. Independently generated marginal samples can each look plausible while failing to sum together sample by sample. Conversely, coherent samples can still be badly calibrated or inaccurate.

Research on probabilistic forecast reconciliation formalizes distribution/sample reconciliation and investigates energy- and variogram-score optimisation. It also gives a crucial evaluation warning: the logarithmic score is not proper for comparing reconciled with unreconciled distributions in the relevant setting. PIRB must choose scoring rules based on the exact comparison, not apply a generic metric list blindly. See Panagiotelis et al., *Probabilistic forecast reconciliation: Properties, evaluation and score optimisation*, European Journal of Operational Research 306(2), 693–706 (2023), [doi:10.1016/j.ejor.2022.07.040](https://doi.org/10.1016/j.ejor.2022.07.040).

## Paradigm shift

Move from independently reported point/interval rows to **joint samples whose shared draw identity preserves dependence across geography, event types and forecast horizons**. For every registered draw, the same draw ID should represent the corresponding joint realization across nodes. A tool cannot establish that a provider has supplied a statistically valid coupling simply because IDs match; original artifact provenance and sample-generation semantics must be pinned.

PIRB audits both the base (unreconciled) sample and the submitted reconciled sample. It reports if each sample violates a registered hierarchy identity, instead of looking only at means or checking that marginal intervals look plausible.

## Contract

Schema: `docs/schemas/pirb-joint-sample-forecast-v0.1.schema.json`

Synthetic fixture: `docs/fixtures/pirb-joint-sample-forecast-synthetic-v0.1.json`

Rust command: `pirb-joint-sample-coherence-audit`

The registration pins a provider/release, target definition, temporal and spatial units, hierarchy digest, reconciliation method digest, fit cutoff, forecast origin and target period. The fit cutoff must precede the forecast origin, and the target period must follow it. The contract declares that test outcomes/future data were not used and that base samples are preserved; those declarations are checked structurally, not independently authenticated.

`sample_policy.draw_ids` defines the common predictive draws. Every declared forecast node must have exactly one raw/reconciled row for every registered draw. A missing cell, duplicate draw/node pair, unknown draw ID, unknown forecast ID, negative count, future fit cutoff, or cycle in the aggregation graph fails closed. PIRB does not invent missing values or draw IDs.

Each constraint declares one parent forecast, child forecast IDs, an expected child-key set, spatial or event-type aggregation semantics, completeness status, and numeric tolerance. For complete constraints, the listed children must match the expected child keys exactly. Incomplete or unknown constraints are kept in the receipt as `not_assessable`; missing children are not treated as zero and are excluded from coherence-rate denominators.

For every complete constraint and every draw, the receipt computes:

```
signed_residual = parent_count - sum(child_counts)
```

It reports the raw and reconciled residual and status separately. Summary metrics include raw/reconciled constraint-draw coherence rates, the fraction of draws where every complete constraint is coherent, mean absolute residuals, and total/maximum per-row adjustment magnitude. Metrics use registered tolerances; exact synthetic fixtures use zero tolerance. The receipt binds the full input digest, schema digest, deterministic serialization of sample rows, source commit, hierarchy digest and method digest.

## Fixture expectations

The synthetic fixture has four joint draws, two complete aggregation constraints, and one incomplete spatial constraint with a missing province. In the complete constraints it contains six coherent base constraint-draw pairs out of eight, while all eight submitted reconciled pairs are coherent. Only two of the four base draws satisfy both complete constraints simultaneously; all four reconciled draws do. One spatial constraint remains unassessable because the declared child set is incomplete.

These numbers are test-oracle expectations for invented values only. They are not measurements of CAST, VIEWS, or any real forecasting system, and they do not demonstrate that a reconciliation algorithm improved accuracy.

## Calibration and scoring remain separate

This binary does not ingest outcomes and therefore **does not calculate marginal interval coverage, joint coverage, sharpness, CRPS, energy score, variogram score, Brier score, log score, count accuracy, or causal impact**. Its coherence metrics are structural diagnostics only.

Next, a separately reviewed evaluator should compare base versus reconciled forecast distributions on identical authorized, mature, rolling-origin outcome rows. It must pin the scoring-rule formula and justify its propriety for that particular comparison. The probabilistic-reconciliation literature above means the log score must not be assumed to validly compare a coherent reconciled distribution against an unreconciled distribution. Energy/variogram scores are candidates to investigate, not automatic proof of superiority.

Report marginal calibration by target/unit/horizon with sample size and uncertainty; assess joint coverage only when its event is defined in advance. Do not sum marginal quantiles and call the result a joint interval. Quantiles of sums are not generally sums of marginal quantiles.

## Proposed acceptance gates

1. 100% of registered synthetic negative controls produce their expected outcomes: duplicate/missing rows, unknown draws, hierarchy/ontology mismatch, missing expected children, cycles and fit-cutoff leakage.
2. Each coherent synthetic draw satisfies every complete registered aggregation constraint at the declared tolerance.
3. No missing draw/node is imputed; incomplete constraints are visible and excluded from the assessable denominator.
4. Publish base and reconciled coherence, all-draw coherence, residual magnitudes and adjustments separately.
5. Never infer accuracy or calibration from coherence. A future superiority claim requires target-compatible score improvements, pre-registered practical margins, common mature outcome rows and dependence-aware uncertainty.
6. No actual protected/provider data in public CI; source-use authority and disclosure review are prerequisites for historical evaluation.

## Limits

This is an auditor, not a reconciliation engine. It does not implement MinTraceSparse or another distributional reconciliation method, fit weights, repair samples, authenticate source truth, infer dependence from marginal-only distributions, or evaluate calibration. It checks the supplied sample matrix against the declared constraints and leaves the original rows unchanged.

A successful run establishes contract/coherence observations for the submitted data. It does not establish state-of-the-art forecasting, effective warnings, fewer violent events or causal peace impact.

## References

- [ACLED CAST methodology](https://acleddata.com/methodology/cast-methodology)
- [ACLED CAST public methodology repository](https://github.com/ACLED/cast-public)
- [Panagiotelis et al. (2023), probabilistic forecast reconciliation](https://doi.org/10.1016/j.ejor.2022.07.040)
- [VIEWS Evaluation repository](https://github.com/views-platform/views-evaluation)
- Research tracker: issue #2191
