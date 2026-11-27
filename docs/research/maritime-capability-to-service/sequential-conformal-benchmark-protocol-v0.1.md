# Sequential conformal calibration under maritime drift v0.1

Tracked by [MCSC-SEQUENTIAL-CONFORMAL-001 #7866](https://github.com/Luminous-Dynamics/symtropy/issues/7866), as a follow-on to the fixed split-conformal contract in [MCSC-CONFORMAL-ACTION-001 #7855](https://github.com/Luminous-Dynamics/symtropy/issues/7855).

## Why fixed split conformal is only the reference arm

The current conformal module implements exact finite-sample absolute-residual split-conformal rank arithmetic under a declared exchangeability applicability state. That gives a useful reproducible null/reference method. Maritime event streams, however, are sequential, temporally dependent, delayed and subject to port-/vessel-/regime-specific distribution shifts. A fixed calibration set can become stale, while reusing calibration data after adaptive selection can change the validity argument.

Keep the fixed split method unchanged as the baseline. Add adaptive/online methods as separately named challengers with their own assumptions and claim ceiling.

## Research comparators

### 1. Fixed chronological split conformal

Current reference arm. Freeze model, feature schema, calibration window and test chronology. It carries finite-sample marginal coverage only when its declared exchangeability assumptions are justified. It is not a default guarantee under serial dependence or drift.

### 2. Rolling-window split conformal

Recompute residual quantiles on a declared rolling window. Test window-size sensitivity, lag, coverage recovery and interval sharpness. Do not assume the rolling window restores exchangeability by itself.

### 3. Adaptive Conformal Inference (ACI)

Use an online miscoverage-level controller driven by revealed prequential errors. Evaluate long-run miscoverage control and interval width under abrupt and gradual drift. Record the controller's update step, clipping and reset rules; do not present a long-run average result as finite-sample conditional coverage.

### 4. Error-quantified Conformal Inference (ECI)

ICLR 2025, Wu et al. replace binary miss-only feedback with a smoothed/continuous quantile-loss feedback signal. The paper claims long-term coverage under arbitrary dependence and reports tighter sets than compared online baselines. Reproduce the exact algorithm, loss smoothing and assumptions before relying on those statements: https://proceedings.iclr.cc/paper_files/paper/2025/hash/cab5ae2704d3e01f06a92512a5376b87-Abstract-Conference.html

### 5. Strongly Adaptive Online Conformal Prediction (SAOCP)

Use as a strong adaptive calibration comparator across multiple time horizons. Preserve its interval-expert and regret assumptions. Compare interval score and coverage against simple ACI/rolling baselines under multiple shift durations.

### 6. Heterogeneous-horizon ensembles (PACE-like)

Forecasting 2026 (Amiri Shahbazi and Baheri) proposes an ensemble of online quantile calibrators with different window lengths, decay rates and quantile scales. The paper reports higher empirical coverage during persistent extreme regimes, often with wider intervals, and contrasts it with SAOCP. It explicitly describes PACE as heuristic without an analogous formal guarantee. Treat it as an empirical challenger, not a formally guaranteed baseline: https://doi.org/10.3390/forecast8050077

### 7. Fresh-fold sequential conformal under recursive selection

Yang et al. (arXiv, 9 October 2026) study multi-stage decisions where earlier selections determine which observations remain eligible for later calibration. Their method uses fresh folds at each stage to preserve finite-sample prefix coverage conditional on earlier folds, and analyses freshness versus data reuse. This is directly relevant to multi-epoch MCSC/R1: https://arxiv.org/abs/2610.12937

Do not reuse selected calibration cases after adaptive refitting unless the selected method's theorem permits it. Explicitly label fresh, blocked, reused and adaptively selected folds.

## Prequential evaluation contract

Every prediction must be frozen before its label is available. Each record must bind:
- prediction ID and immutable feature/payload digest;
- model and feature-schema digest;
- prediction-issued time;
- decision cutoff and observable-state digest;
- calibration method/version and calibration fold/window IDs;
- declared alpha or resource alpha allocation;
- interval or prediction set, frozen before label arrival;
- label first-available time and value digest;
- miss indicator and miss magnitude;
- update event/receipt and resulting calibration state.

Labels are admitted to the online updater only after their first-available time and only for predictions whose frozen interval predates the label. A late or corrected label creates a new vintage and must not rewrite the prior prediction receipt.

## Frozen stress-regime benchmark

Use a deterministic synthetic null and configurable stress generator before any operational claims:

1. stationary/exchangeable null;
2. serial correlation and clustered vessel arrivals;
3. gradual residual drift;
4. abrupt location shift;
5. variance and heavy-tail shift;
6. seasonal/trade-lane shift;
7. terminal- and vessel-class-specific covariate shift;
8. persistent extreme regime and recovery after event end;
9. missing/censored AIS, delayed labels and corrected event times;
10. recursive subpopulation selection after earlier no-backlog/service decisions;
11. common-cause infrastructure shocks that simultaneously affect multiple resource predictions.

Freeze generator parameters, seeds, model predictions, calibration roles and decision cutoffs. Keep a separate final evaluation stream whose labels cannot affect predictions or calibration objects reported for that same cutoff.

## Metrics

Report by resource, terminal, vessel class, regime and forecast horizon:
- empirical marginal miscoverage and rolling/cumulative miscoverage;
- exact finite-sample coverage only where the method's assumptions and design justify it;
- interval width, interval score and tail miss severity;
- worst-group coverage and subgroup gaps;
- time steps to recover target miscoverage after a shift;
- false shift alarms and detection delay;
- calibration-fold freshness, reuse and label delay;
- update CPU/memory/latency;
- unbounded/indeterminate intervals and abstention rate;
- downstream schedule feasibility and service metrics under a matched joint-risk budget.

Coverage alone is insufficient. A method that restores coverage by issuing unusably wide intervals may be statistically valid but operationally poor. A narrow method with undercoverage is also unacceptable. Publish coverage/width/adaptation/runtime Pareto frontiers.

## Method guarantee registry

| Method | Evidence class | Guarantee to report |
|---|---|---|
| Fixed split conformal | Finite-sample reference | Marginal coverage under exchangeability and exact split design |
| Rolling split conformal | Empirical / assumption-dependent | Windowed coverage; do not infer exchangeability from rolling |
| ACI | Online adaptive | State exact long-run error-control result and its update conditions |
| ECI | Online adaptive | State paper's arbitrary-dependence long-term result and implementation assumptions |
| SAOCP | Strongly adaptive online | State regret/coverage result and horizon/expert assumptions |
| PACE-like ensemble | Heuristic empirical comparator | No formal guarantee unless independently established |
| Fresh-fold sequential conformal | Conditional prefix | State the fold freshness and conditional exchangeability design |

Do not collapse these guarantee classes into one '90% conformal' badge.

## Joint decision use

Any multi-resource decision still requires a predeclared family-wise risk allocation. Marginal arrival, handling, crane, shore-power and yard intervals cannot be presumed independent. Apply the union-bound joint budget or a separately justified joint predictive method. Shifted/unknown applicability must result in Indeterminate or an explicitly justified alternative policy, not an automatic pass.

## Claim ceiling

This protocol establishes a benchmark plan only. It does not establish 90% coverage on Durban vessel calls, conditional coverage, causal scheduling benefits or robust feasibility of live port operations. The Durban study's 241 vessel-visit data remain unacquired; no paper reproduction or SOTA superiority claim is made.