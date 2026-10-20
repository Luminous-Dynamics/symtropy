---
title: PIRB Scoring-Rule Admissibility Registry
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB scoring-rule admissibility registry v0.1

## Why metric admissibility is a separate gate

A metric can be correctly implemented yet invalid for a particular target, prediction representation, or comparison. PIRB must distinguish:
1. schema/registration validity;
2. score/representation/comparison admissibility;
3. score computation;
4. comparison superiority;
5. operational decision value;
6. causal impact.

The [2023 probabilistic forecast reconciliation study by Panagiotelis et al.](https://doi.org/10.1016/j.ejor.2022.07.040) shows why this matters: in the relevant comparison, logarithmic score is not proper for evaluating unreconciled forecasts against forecasts reconciled to a coherent distribution. Energy and variogram scores are studied as alternatives for probabilistic reconciliation, but PIRB must pin formula, representation, sampling assumptions and exact comparison before using them.

This validator never computes scores. It decides whether a registration is admissible for a future scoring step and emits reason codes when it is not. A result marked `admissible_for_score_computation` is not a ranking, score, or proof of superiority.

## Contract and tool

Schema: `docs/schemas/pirb-score-admissibility-v0.1.schema.json`

Negative-control fixture: `docs/fixtures/pirb-score-admissibility-log-score-reconciliation-synthetic-v0.1.json`

Rust command: `pirb-score-admissibility`

The registration pins the target estimand, prediction representation, outcome type, comparison type, common-row policy, held-out/mature outcome vintage, score orientation, formula reference and digest, implementation reference/digest, independent oracle reference, and governance. Historical evaluation can be admissible only when common rows are verified, outcomes are held out and mature/frozen, digests are non-placeholder, and data authority/disclosure gates are satisfied.

## v0.1 compatibility table

| Prediction representation | Outcome | Scores admissible for consideration |
|---|---|---|
| Binary probability | Binary outcome | Brier score; log score |
| Quantile forecast | Scalar outcome | Pinball loss; interval score |
| Multivariate joint samples | Multivariate outcome | Energy score; variogram score |

All combinations outside this closed list are returned as `not_admissible`. Score orientation is pinned: Brier, pinball, interval, energy and variogram are lower-is-better; log score is higher-is-better.

These combinations are a conservative v0.1 registry, not a claim that every score is appropriate under every data-generating process. Formula-level reference tests and assumptions still need independent verification before a real evaluation.

### Special reconciliation rule

For `reconciled_vs_unreconciled_distribution`, v0.1 explicitly marks `log_score` not admissible. This rule encodes the relevant comparison-specific warning in Panagiotelis et al. (2023); it does not claim that logarithmic score is unusable for ordinary probability-forecast verification or all model comparisons.

For joint predictive samples in that comparison, energy and variogram scores may be registered for review. The validator's acceptance only says the score name, representation, outcome type and comparison type are compatible with this registry. It does not prove the model assumptions or execute the score.

## Synthetic negative control

The public fixture attempts to use `log_score` for reconciled-vs-unreconciled multivariate joint samples. The validator must return:
- `comparison_status = not_admissible`;
- reason code `LOG_SCORE_NOT_ADMISSIBLE_FOR_RECONCILED_VS_UNRECONCILED_DISTRIBUTIONS`;
- `score_calculated = false`;
- `ranking_permitted = false`;
- superiority, decision value, and causal impact unestablished.

A valid alternative such as an energy score can be marked `admissible_for_score_computation` on the synthetic contract test, but no score is calculated and no ranking becomes permissible.

## Receipt and negative controls

The Rust receipt binds the whole registration input and schema digests, source commit, score name, target and comparison, admissibility status, reasons, and explicit claim ceiling. It never accepts prefilled score results. Tests cover:
- log score rejected for reconciled-vs-unreconciled distributions;
- Brier score accepted for a compatible binary probability registration but not executed;
- energy score accepted for a compatible joint-sample registration but not executed;
- Brier score rejected for multivariate joint samples;
- incompatible score orientation;
- historical scoring blocked when maturity, authority, disclosure or pins are missing;
- caller-supplied score/superiority claims rejected.

## Benchmarks

1. 100% of unsupported score/representation/comparison combinations fail closed.
2. Formula-level golden examples match an independent oracle within a pinned tolerance before any score implementation is released.
3. Computation, when separately implemented, is deterministic for a pinned dataset, formula version, row policy, outcome vintage and dependency lock.
4. No score can be selected or tuned on held-out outcomes. No pair ranks when estimands, outcomes or eligible rows differ.
5. A state-of-the-art claim requires preregistered paired differences, a practical margin, uncertainty appropriate to temporal/spatial dependence, and independent receipt verification.

## Boundaries

This validator defines admissibility; it does not compute Brier, log, pinball, interval, energy or variogram score. It does not calculate calibration, forecast superiority, decision value or impact. A green synthetic contract test proves only registry behavior.

## References

- [Panagiotelis et al. (2023), probabilistic forecast reconciliation](https://doi.org/10.1016/j.ejor.2022.07.040)
- [VIEWS evaluation repository](https://github.com/views-platform/views-evaluation)
- [ACLED CAST methodology](https://acleddata.com/methodology/cast-methodology)
- Tracking issue: #2254
