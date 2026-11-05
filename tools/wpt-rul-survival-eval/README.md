# WPT competing-risk survival evaluator v0

**Status:** synthetic known-truth evaluator; fixed-horizon IPCW Brier and cause-specific hazard log scores  
**Parent:** [#3763 WPT-RUL-SURVIVAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/3763)  
**Related:** [#3758 asset lifecycle evidence](https://github.com/Luminous-Dynamics/symtropy/pull/3758); #3692 trajectory-dependent calibration; #3641 lifecycle economics.

## What v0 evaluates

One TEST cohort, one fixed horizon and a declared competing-risk cumulative-incidence estimand. Each subject supplies:
- observed time and outcome kind: target failure, competing event or right censor;
- one cluster identity;
- predicted event-free survival probability at the horizon;
- one cumulative-incidence probability for every declared event cause at the same horizon.

The probabilities must be in [0,1] and sum to one. Target failures and competing events remain distinct cause categories. A right-censored subject is never silently converted into a failure.

## Two complementary survival scoreboards

The evaluator computes two complementary proper-score views, plus a secondary all-cause Harrell C-index. The scores answer different questions and are never collapsed into a single leaderboard number.

### Fixed-horizon multicause IPCW Brier score

The evaluator estimates the censoring survival function from the evaluation cohort with a reverse Kaplan–Meier product limit. This v0 admits only the declared **unconditional independent right-censoring** assumption.

For an event observed by the horizon, it uses the left-limit inverse censoring survival weight at that event time. For an event-free subject observed through the horizon, it uses the left-limit inverse censoring survival at the horizon. A subject right-censored before the horizon has unknown horizon status and is excluded from that horizon's score contribution.

The competing-risk multiclass Brier score is:

`BS(t) = (1/n) * sum_i w_i(t) * sum_{k in {event-free, causes}} (p_ik(t) - y_ik(t))^2`

where the cause-specific target category is retained and `n` is the full test-cohort size. This is a fixed-horizon IPCW score, not an integrated score. The evaluator reports minimum censoring survival and maximum IPCW weight. If the configured positivity floor is breached, only the horizon-specific IPCW Brier field is marked NOT_EVALUABLE; the hazard log score and C-index remain independently available under their own assumptions.



### Cause-specific censored hazard log score

Each subject also supplies a piecewise-constant cause-specific hazard rate for every declared event cause and every interval in `hazard_time_edges_hours`. Rates are explicitly in h^-1.

For subject i observed until m_i, the score is:

`L_i = integral_0^m_i sum_k lambda_ik(u) du - delta_i * log(lambda_i,J_i(m_i))`

The integral charges total terminal-event hazard over observed exposure. If a target or competing event is observed, the event term scores the forecast hazard for that specific cause. For right-censoring, no event term is added; the observed follow-up still contributes the integrated hazard.

This is the cause-specific competing-risk extension of the hazard-form log score. It does not estimate or plug in the censoring distribution G(t), which avoids inverse-weight instability for this metric. It still relies on independent random censoring for propriety, and its expected value depends on the censoring regime actually operating. Compare hazard log scores only on the same censoring regime and time unit; never compare raw log-score magnitudes to Brier scores.

Philip Dawid's September 2026 note derives a general proper score for right-censored survival times and shows that the log-type score can be computed without modelling the censoring distribution under independent random censoring. The note also warns that score values depend on the active censoring process. The multi-cause cause-specific hazard form here is a likelihood extension for competing risks; it is not claimed as a verbatim theorem from the single-event note.

The fixture's hand-computed mean cause-specific hazard log score is 2.5669264097. This is reported in addition to, not instead of, the IPCW Brier score.

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
- cause-specific hazard log score: 2.5669264097;
- all-cause Harrell C-index: 0.9 from 15 comparable pairs.

All numbers are synthetic known-truth controls. The score is hand-recomputable from the checked-in fixture; no WPT field-lifetime result is implied.



## Forecast representation coherence

The same TEST record carries survival/CIF probabilities and piecewise-constant cause-specific hazards. These are not allowed to be unrelated prediction heads: the evaluator integrates the cause-specific hazards to the declared forecast horizon and checks that the implied event-free survival and each cause-specific cumulative incidence agree with the published probabilities to an absolute tolerance of 1e-8.

For a constant hazard vector over an interval of length \\(\\Delta t\\), total hazard \\(\\lambda=\\sum_k\\lambda_k\\), and survival at interval start \\(S\\), the cause-k incidence increment is:

`Delta F_k = S * (lambda_k / lambda) * (1 - exp(-lambda * Delta t))`

This makes the two score representations coherent before scoring. It is a stronger admission check than merely requiring the probability vector to sum to one.

## Aalen–Johansen descriptive reference

The evaluator also reports an empirical Aalen–Johansen event-free survival and cause-specific cumulative-incidence vector at the declared horizon. The estimator retains competing events as distinct causes and uses the pre-time risk set for tied terminal/censor observations.

This curve is an **in-cohort descriptive reference**, not an independent known-truth oracle and not a held-out calibration result. It is useful for checking event accounting and illustrating the cohort's observed cause distribution. It must not be compared to the model's TEST forecasts as though it were independent evidence or a causal ground truth.

For the frozen 10-hour fixture:
- Event-free survival: 15/28 = 0.5357143.
- Structural-crack cumulative incidence: 1/7 = 0.1428571.
- Insulation-resistance cumulative incidence: 5/28 = 0.1785714.
- Planned-replacement cumulative incidence: 1/7 = 0.1428571.
- Road-reconstruction cumulative incidence: 0.
- Three terminal events are observed by 10 hours.

## Research positioning and limits

- The 2025 A-calibration method adds a formal goodness-of-fit calibration test for censored survival forecasts and reported greater power than D-calibration under its studied censoring regimes. Neither test is implemented here.
- SurvivalBoost (AISTATS 2025) reports a strictly proper censoring-adjusted scoring rule for survival and competing risks and outperforms 12 SOTA comparators across four real datasets in its study. It is an important comparator; this v0 does not claim to reproduce its score or surpass it.
- A 2026 arXiv preprint by Jonkers et al. constructs proper scores for right-censored observed-data laws (including Brier, CRPS and pinball variants) and documents forecast-ranking reversals for some forecast-dependent plug-in weighted scores. It is a preprint and not yet treated as settled field consensus.
- Dawid's September 2026 arXiv note derives hazard-form proper scores for right-censored survival data without estimating G(t), under independent random censoring. The reported expected score depends on the active censoring regime; this evaluator therefore keeps the hazard log score separate from IPCW Brier and does not compare them numerically.
- Prince et al. (PLOS ONE 2025) show censoring-distribution estimation choices can matter in covariate-dependent censoring, small samples or noisy settings. v0 therefore records the evaluation-cohort KM policy and refuses to score declared covariate-dependent censoring.
- Aalen–Johansen cumulative incidence is now reported as an in-cohort descriptive reference only. Time-dependent AUC, Uno C-index, integrated Brier score, A-/D-calibration, cluster bootstrap intervals and conditional censoring models are not implemented.

The benchmark must next add dependent-censoring synthetic controls with a separately declared conditional censoring estimator, Aalen–Johansen reference calculations, multi-horizon curves and calibration/uncertainty diagnostics before it can support maintenance decisions.

## Validation

The Rust crate contains 17 unit tests covering hand-computed IPCW and hazard scores, censor-before-horizon treatment, competing-event cause scoring, probability and cause completeness, censoring positivity, independent-censoring assumptions, event/censor semantics, hazard interval boundaries and unsupported forecasts.

## Run

`cargo test --locked -p wpt-rul-survival-eval`

## Claim ceiling

This crate validates a synthetic evaluation contract and computes point metrics under one explicit censoring assumption. It does not train an RUL model, prove field calibration, establish predictive-maintenance readiness, certify infrastructure lifetime, or demonstrate SOTA superiority.