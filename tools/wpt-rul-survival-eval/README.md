# WPT competing-risk survival evaluator v0

**Status:** synthetic known-truth evaluator; fixed-horizon multicause IPCW Brier score  
**Parent:** [#3763 WPT-RUL-SURVIVAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/3763)  
**Related:** [#3758 asset lifecycle evidence](https://github.com/Luminous-Dynamics/symtropy/pull/3758); #3692 trajectory-dependent calibration; #3641 lifecycle economics.

## What v0 evaluates

One TEST cohort, one fixed horizon and a declared competing-risk cumulative-incidence estimand. Each subject supplies:
- observed time and outcome kind: target failure, competing event or right censor;
- one cluster identity;
- predicted event-free survival probability at the horizon;
- one cumulative-incidence probability for every declared event cause at the same horizon.

The probabilities must be in [0,1] and sum to one. Target failures and competing events remain distinct cause categories. A right-censored subject is never silently converted into a failure.

## Censoring-aware score

The evaluator estimates the censoring survival function from the evaluation cohort with a reverse Kaplan–Meier product limit. This v0 admits only the declared **unconditional independent right-censoring** assumption.

For an event observed by the horizon, it uses the left-limit inverse censoring survival weight at that event time. For an event-free subject observed through the horizon, it uses the left-limit inverse censoring survival at the horizon. A subject right-censored before the horizon has unknown horizon status and is excluded from that horizon's score contribution.

The competing-risk multiclass Brier score is:

`BS(t) = (1/n) * sum_i w_i(t) * sum_{k in {event-free, causes}} (p_ik(t) - y_ik(t))^2`

where the cause-specific target category is retained and `n` is the full test-cohort size. This is a fixed-horizon IPCW score, not an integrated score. The evaluator reports minimum censoring survival and maximum IPCW weight and returns NOT_EVALUABLE when the configured positivity floor is breached.

A secondary all-cause Harrell C-index is reported for ranking any first terminal event against the horizon-specific total event-risk score. It is not a cause-specific concordance measure and is not IPCW-adjusted; it must not be used as the primary competing-risk metric.

## Frozen fixture and hand-checkable results

The 8-subject synthetic cohort has:
- target causes: structural cracking and insulation-resistance failure;
- competing causes: road reconstruction and planned replacement;
- right censoring at 2, 6 and 12 hours;
- prediction horizon: 10 hours;
- 7 distinct clusters.

At 10 hours:
- 2 subjects are censored before the horizon and have unknown event status;
- 6 horizon outcomes are evaluable;
- reverse-KM censoring survival at the event/horizon queries is 0.875 or 0.7;
- minimum used censoring survival: 0.7;
- maximum IPCW weight: 1 / 0.7 = 1.4285714;
- multicause IPCW Brier score: 0.23453571428571426;
- all-cause Harrell C-index: 0.9 from 15 comparable pairs.

All numbers are synthetic known-truth controls. The score is hand-recomputable from the checked-in fixture; no WPT field-lifetime result is implied.

## Research positioning and limits

- The 2025 A-calibration method adds a formal goodness-of-fit calibration test for censored survival forecasts and reported greater power than D-calibration under its studied censoring regimes. Neither test is implemented here.
- SurvivalBoost (AISTATS 2025) reports a strictly proper censoring-adjusted scoring rule for survival and competing risks and outperforms 12 SOTA comparators across four real datasets in its study. It is an important comparator; this v0 does not claim to reproduce its score or surpass it.
- A 2026 arXiv preprint by Jonkers et al. constructs proper scores for right-censored observed-data laws (including Brier, CRPS and pinball variants) and documents forecast-ranking reversals for some forecast-dependent plug-in weighted scores. It is a preprint and not yet treated as settled field consensus. This v0 uses the simpler, established IPCW Brier estimator under an explicit unconditional independent-censoring assumption.
- Prince et al. (PLOS ONE 2025) show censoring-distribution estimation choices can matter in covariate-dependent censoring, small samples or noisy settings. v0 therefore records the evaluation-cohort KM policy and refuses to score declared covariate-dependent censoring.
- Aalen–Johansen cumulative-incidence reference curves, time-dependent AUC, Uno C-index, integrated Brier score, A-/D-calibration, cluster bootstrap intervals and conditional censoring models are not implemented.

The benchmark must next add dependent-censoring synthetic controls with a separately declared conditional censoring estimator, Aalen–Johansen reference calculations, multi-horizon curves and calibration/uncertainty diagnostics before it can support maintenance decisions.

## Run

`cargo test --locked -p wpt-rul-survival-eval`

## Claim ceiling

This crate validates a synthetic evaluation contract and computes point metrics under one explicit censoring assumption. It does not train an RUL model, prove field calibration, establish predictive-maintenance readiness, certify infrastructure lifetime, or demonstrate SOTA superiority.